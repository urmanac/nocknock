use crate::*;

/// The high-water mark's date (issue #94, N2): the time of the earliest stored row at the
/// peak, unless the current total sets a new mark, in which case it is now. `prev` is
/// the peak row (value, generated_at) or None on an empty table. Pure.
pub(crate) fn resolve_high_water_mark_date(prev: Option<(f64, DateTime<Utc>)>, current_total_usd: f64, now: &str) -> String {
    match prev {
        Some((v, at)) if v >= current_total_usd => at.to_rfc3339(),
        _ => now.to_string(),
    }
}

/// Run one ingestion cycle and remember whether it failed, for /health.
pub(crate) async fn run_cycle_tracked(state: &Arc<AppState>) -> Result<()> {
    let result = run_ingestion_cycle(state).await;
    if let Ok(mut slot) = state.last_ingest_error.lock() {
        *slot = result.as_ref().err().map(|e| (Utc::now(), e.to_string()));
    }
    result
}

// ============================================================
// Ingestion Logic
// ============================================================

pub(crate) async fn run_ingestion_cycle(state: &Arc<AppState>) -> Result<()> {
    let start_time = std::time::Instant::now();
    let now = Utc::now().to_rfc3339();
    tracing::info!("Starting ingestion cycle at {now}");

    // Heal price-history gaps first (fail-open) so the daily RSI below sees
    // backfilled candles for any period the worker was offline.
    ensure_price_history(state).await;

    // Fetch raw data from Alchemy (parallelized on-chain calls)
    let raw_data = fetch_raw_data(state).await?;

    // Persistent high-water mark: the peak stored total and the earliest row that reached
    // it (issue #94, N2). An empty table (a fresh start) has no mark yet. A database error
    // fails the cycle: it must never silently reset the mark to the current value.
    let hwm_row = sqlx::query(
        r#"SELECT generated_at, (data->'derived_metrics'->'global_status'->>'total_portfolio_usd')::float8 AS v
           FROM snapshots
           WHERE (data->'derived_metrics'->'global_status'->>'total_portfolio_usd') IS NOT NULL
           ORDER BY v DESC, generated_at ASC
           LIMIT 1"#,
    )
    .fetch_optional(&state.db)
    .await
    .map_err(|e| anyhow::anyhow!("High-water mark query failed: {e}"))?;
    let prev_peak: Option<(f64, DateTime<Utc>)> = match hwm_row {
        None => None,
        Some(r) => Some((
            r.try_get::<f64, _>("v").map_err(|e| anyhow::anyhow!("High-water mark value unreadable: {e}"))?,
            r.try_get::<DateTime<Utc>, _>("generated_at")
                .map_err(|e| anyhow::anyhow!("High-water mark date unreadable: {e}"))?,
        )),
    };
    let prev_hwm: f64 = prev_peak.map(|(v, _)| v).unwrap_or(0.0);

    // Compute structural channels from historical data — most recent 50 snapshots,
    // oldest first, with their real snapshot timestamps
    let historical_rows = sqlx::query(
        r#"SELECT generated_at, (data->'derived_metrics'->'global_status'->>'total_portfolio_usd')::float8 as portfolio_usd FROM snapshots ORDER BY generated_at DESC LIMIT 50"#
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let historical_prices: Vec<(DateTime<Utc>, f64)> = historical_rows
        .iter()
        .rev()
        .filter_map(|r| {
            let ts: DateTime<Utc> = r.try_get("generated_at").ok()?;
            let v: f64 = r.try_get("portfolio_usd").ok()?;
            Some((ts, v))
        })
        .collect();

    let structural_channels = compute_structural_channels_sync(&historical_prices);

    // Sovereign ledger aggregate: receipt-verified harvests feed the virtual 60/40 sub-ledger
    let ledger_agg = match sqlx::query(
        "SELECT COALESCE(SUM(usd_value::numeric)::float8, 0) AS total_usd, \
                COUNT(DISTINCT tx_hash) AS harvests, \
                MAX(recorded_at) AS last_at, \
                COUNT(*) FILTER (WHERE usd_value IS NULL) AS unpriced \
         FROM harvest_events",
    )
    .fetch_one(&state.db)
    .await
    {
        Ok(r) => LedgerAggregate {
            total_usd: r.try_get::<f64, _>("total_usd").unwrap_or(0.0),
            harvest_count: r.try_get::<i64, _>("harvests").unwrap_or(0),
            last_at: r.try_get::<Option<DateTime<Utc>>, _>("last_at").ok().flatten(),
            unpriced_count: r.try_get::<i64, _>("unpriced").unwrap_or(0),
        },
        Err(e) => {
            tracing::warn!("Harvest ledger aggregate query failed: {e}");
            LedgerAggregate::default()
        }
    };

    // Fetch counterfactual historical data: the MOST RECENT 100 snapshots, collected as
    // paired (timestamp, portfolio, native) tuples so the two series stay index-aligned,
    // oldest first, carrying real snapshot timestamps (no fabricated spacing).
    let counterfactual_rows = sqlx::query(
        r#"
        SELECT generated_at,
               (data->'derived_metrics'->'global_status'->>'total_portfolio_usd')::float8 as portfolio_usd,
               (data->'derived_metrics'->'global_status'->>'total_native_usd')::float8 as native_usd
        FROM (
            SELECT generated_at, data FROM snapshots ORDER BY generated_at DESC LIMIT 100
        ) recent
        ORDER BY generated_at ASC
        "#
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();

    let mut counterfactual_history: Vec<(DateTime<Utc>, f64, f64)> = Vec::new();
    for row in counterfactual_rows {
        if let (Ok(ts), Ok(Some(p)), Ok(Some(n))) = (
            row.try_get::<DateTime<Utc>, _>("generated_at"),
            row.try_get::<Option<f64>, _>("portfolio_usd"),
            row.try_get::<Option<f64>, _>("native_usd"),
        ) {
            counterfactual_history.push((ts, p, n));
        }
    }

    // Compute derived metrics
    let profile = state.config.profile();
    // Market regime — two RSI(14) series per industry convention (the period is
    // 14 bars of the underlying timeframe, not 14 days):
    //   • Micro: RSI(14) over the 5-minute snapshot series (short-term momentum pulse).
    //   • Macro: daily RSI(14) over one close per UTC day spanning the multi-day
    //     lookback, backfilled from third-party OHLCV where snapshots are missing.
    //     The Overextension Cooldown keys on the daily series so a 4-hour bounce
    //     cannot fake a multi-day top (issue #29).
    // Historical micro closes: the NOCK spot stored in each past snapshot
    // (total_value_vs_baseline.token_price_series[0] — every row since v0.1.0 carries
    // it, so the series stays continuous across the v2.0.0 pool-shape change).
    let regime_rows = sqlx::query(
        r#"SELECT (data->'derived_metrics'->'total_value_vs_baseline'->'token_price_series'->>0)::float8 AS spot
           FROM snapshots ORDER BY generated_at DESC LIMIT 50"#,
    )
    .fetch_all(&state.db)
    .await
    .unwrap_or_default();
    let mut micro_closes: Vec<f64> = regime_rows
        .iter()
        .rev()
        .filter_map(|r| {
            let spot: f64 = r.try_get("spot").ok()?;
            if spot > 0.0 { Some(spot) } else { None }
        })
        .collect();
    // Live NOCK/USD spot: computed upstream from the price-source position pool.
    let live_spot = raw_data.spot_price_nock_usd;
    if live_spot > 0.0 {
        micro_closes.push(live_spot);
        // Contemporaneous price record: the RPC-observed spot is stored before any
        // snapshot dedup so the daily series always includes the freshest close.
        if let Err(e) =
            insert_price_point(&state.db, Utc::now(), live_spot, PRICE_SOURCE_RPC, false).await
        {
            tracing::warn!("Contemporaneous price insert failed (non-fatal): {e}");
        }
    }
    let rsi_5m = compute_rsi_series(&micro_closes, RSI_PERIOD)
        .iter()
        .rev()
        .find_map(|x| *x);

    // Macro (daily) series: one close per UTC day from market_price_history —
    // daily candle closes where backfilled, else the day's last spot.
    let macro_points: Vec<(DateTime<Utc>, Option<f64>, f64)> = sqlx::query(
        "SELECT timestamp, close_usd, price_usd FROM market_price_history \
         WHERE timestamp >= NOW() - make_interval(days => $1) ORDER BY timestamp ASC",
    )
    .bind(state.config.market_data.daily_lookback as i32)
    .fetch_all(&state.db)
    .await
    .unwrap_or_default()
    .iter()
    .filter_map(|r| {
        let ts: DateTime<Utc> = r.try_get("timestamp").ok()?;
        let close_usd: Option<f64> = r.try_get("close_usd").ok()?;
        let price_usd: f64 = r.try_get("price_usd").ok()?;
        Some((ts, close_usd, price_usd))
    })
    .collect();
    let mut macro_closes = daily_closes_from_points(&macro_points);
    if live_spot > 0.0 {
        macro_closes.push(live_spot); // the live spot closes today's partial day
    }
    let rsi_1d = compute_rsi_series(&macro_closes, RSI_PERIOD)
        .iter()
        .rev()
        .find_map(|x| *x);
    let regime = MarketRegime {
        rsi_5m,
        rsi_1d,
        overextended: rsi_1d.map_or(false, |r| r >= RSI_OVERBOUGHT),
    };

    let plan_ctx = load_plan_context(state).await;
    // Fill detection (issue #118): look for plan buys on-chain and offer them. It never
    // fails the cycle; when it cannot run, the plan status says so.
    let fill_detection = detect_fills(state, &plan_ctx).await;
    let mut derived_metrics = compute_derived_metrics_with_fills(&raw_data, prev_hwm, &profile, structural_channels,
        &counterfactual_history, &ledger_agg, &regime, &plan_ctx, fill_detection.as_ref());
    {
        let gs = &mut derived_metrics.global_status;
        let current_total: f64 = gs.total_portfolio_usd.parse().unwrap_or(f64::INFINITY);
        gs.high_water_mark_date = resolve_high_water_mark_date(prev_peak, current_total, &gs.high_water_mark_date);
    }
    // Route quotes (issue #115): only when the plan's own status says a tranche is due,
    // and only for that tranche's amount. Read-only quoter calls; they never fail the
    // cycle, and a venue that cannot be quoted says why.
    if let Some(ps) = derived_metrics.accumulation_plan.as_mut() {
        ps.route_quotes = quote_due_tranche(state, ps, Some(raw_data.spot_price_nock_usd)).await;
        let gs = &mut derived_metrics.global_status;
        gs.shot_advice_details =
            with_route_quote_pointer(&gs.shot_advice_action, std::mem::take(&mut gs.shot_advice_details), ps.route_quotes.as_ref());
    }
    // Cost basis (issue #98): plan buys from the accumulation ledger, and the whole stack
    // only when the operator configured an opening basis. A failed ledger read is stated
    // in the block (`unavailable`); it does not fail the cycle. The NOCK held is this
    // snapshot's own total, which is a whole number of NOCK, and is reported as one.
    let held_nock = derived_metrics.global_status.total_nock_amount.trim().parse::<f64>().ok();
    derived_metrics.cost_basis =
        Some(cost_basis_for_cycle(state, raw_data.spot_price_nock_usd, held_nock).await);
    // Capture directive strings before the metrics move into the snapshot envelope
    let next_directive_action = derived_metrics.global_status.shot_advice_action.clone();
    let next_directive_details = derived_metrics.global_status.shot_advice_details.clone();
    let next_plan_status = derived_metrics.accumulation_plan.clone();
    let next_range_top = range_top_reading(&raw_data);

    // Build snapshot data. The positions carry every stamp the cycle's fee rows were
    // built from (pool activity and, since #91, staking and the fees note).
    let positions = snapshot_positions(&raw_data);
    let snapshot_data = SnapshotData {
        wallet_balances: raw_data.wallet_balances,
        positions,
        pool_mechanics: raw_data.pool_mechanics,
        concentrated_liquidity: raw_data.concentrated_liquidity,
        contract_fees: raw_data.contract_fees,
        derived_metrics,
    };

    // Compute deterministic state hash
    let state_hash = compute_state_hash(&snapshot_data);

    // Build envelope
    let now = Utc::now();
    // Notifications (issues #28, #89): decided here against the previous snapshot, sent
    // only after this one is stored. A directive change notifies, and so do a plan
    // completing and the price-source range being exited. A first cycle with no previous
    // snapshot stays silent.
    let (prev_directive, prev_plan_status) = match state.latest_snapshot.lock().await.as_ref() {
        Some(s) => (
            Some(s.data.derived_metrics.global_status.shot_advice_action.clone()),
            s.data.derived_metrics.accumulation_plan.clone(),
        ),
        None => (None, None),
    };
    let range_exit = range_exit_note(last_range_top_reading().as_ref(), next_range_top.as_ref());
    let notification = cycle_notification(
        prev_directive.as_deref(),
        &next_directive_action,
        &next_directive_details,
        prev_plan_status.as_ref(),
        next_plan_status.as_ref(),
        range_exit.as_deref(),
        state.config.notifications.cockpit_url.as_deref(),
    );

    let envelope = SnapshotEnvelope {
        meta: SnapshotMeta {
            generated_at: now,
            state_hash: state_hash.clone(),
            version: SNAPSHOT_SPEC_VERSION.to_string(),
            snapshot_id: Uuid::new_v4(),
        },
        data: snapshot_data,
    };

    // Extract values for logging before moving
    let snapshot_id = envelope.meta.snapshot_id;
    let state_hash = envelope.meta.state_hash.clone();

    // Store in database with deduplication
    store_snapshot(&state.db, &envelope).await?;

    // Update in-memory cache
    *state.latest_snapshot.lock().await = Some(envelope);
    remember_range_top_reading(next_range_top);

    // Sent after the store: a cycle whose store fails leaves the previous snapshot in
    // place, so the same transition is decided again next cycle and is pushed once, when
    // it is on record, instead of once per failed attempt. Delivery failures are logged,
    // never fatal.
    if let Some(text) = notification {
        dispatch_notifications(state, &text).await;
    }

    let duration_ms = start_time.elapsed().as_millis() as u64;
    state.last_sync_duration_ms.store(duration_ms, std::sync::atomic::Ordering::Relaxed);

    tracing::info!(
        "Ingestion complete in {duration_ms}ms: snapshot_id={}, hash={}",
        snapshot_id,
        &state_hash[..16]
    );

    Ok(())
}

pub(crate) fn compute_state_hash(data: &SnapshotData) -> String {
    // Canonical JSON: sorted keys, no whitespace, wall-clock fields stripped
    let canonical = canonical_state_json(data);
    let mut hasher = Sha256::new();
    hasher.update(serde_json::to_vec(&canonical).unwrap());
    format!("{:x}", hasher.finalize())
}

/// Canonical hash input: the snapshot payload with every wall-clock field removed.
///
/// Timestamps (`updated_at`, `high_water_mark_date`, series `time_points`, channel
/// `start_time`/`end_time`/`break_point`, `LinePoint.time`) are produced fresh on every
/// ingestion cycle and carry no state information. Including them made the hash change
/// every cycle, defeating snapshot deduplication (`store_snapshot` would insert a new
/// row every cycle even when on-chain state was unchanged, growing the table unboundedly
/// and filling history charts with duplicate points).
pub(crate) fn canonical_state_json(data: &SnapshotData) -> serde_json::Value {
    let mut value = serde_json::to_value(data).expect("SnapshotData is JSON-serializable");
    strip_volatile_time_fields(&mut value);
    value
}

pub(crate) fn strip_volatile_time_fields(value: &mut serde_json::Value) {
    const VOLATILE_KEYS: [&str; 9] = [
        // Route quotes for a due tranche (issue #115): contract reads at a stated
        // block, fresh every cycle. They say nothing about the wallet's state.
        "route_quotes",
        // Fill detection's check time (issue #118): fresh every cycle, no state.
        "last_checked_at",
        "updated_at",
        "high_water_mark_date",
        "time_points",
        "time",
        "start_time",
        "end_time",
        "break_point",
    ];
    match value {
        serde_json::Value::Object(map) => {
            for key in VOLATILE_KEYS {
                map.remove(key);
            }
            for (_, v) in map.iter_mut() {
                strip_volatile_time_fields(v);
            }
        }
        serde_json::Value::Array(items) => {
            for item in items {
                strip_volatile_time_fields(item);
            }
        }
        _ => {}
    }
}

pub(crate) async fn store_snapshot(db: &PgPool, envelope: &SnapshotEnvelope) -> Result<()> {
    // Deduplicate: check if most recent snapshot has identical state hash
    let latest = sqlx::query!(
        "SELECT snapshot_id, state_hash FROM snapshots ORDER BY generated_at DESC LIMIT 1"
    )
    .fetch_optional(db)
    .await?;

    if let Some(ref r) = latest {
        if r.state_hash.trim() == envelope.meta.state_hash.trim() {
            tracing::info!(
                "State hash unchanged ({}), preserving historical snapshot timestamp",
                &r.state_hash[..16]
            );
            return Ok(());
        }
    }

    let data_json = serde_json::to_value(&envelope.data)?;
    sqlx::query!(
        r#"
        INSERT INTO snapshots (snapshot_id, generated_at, state_hash, version, data)
        VALUES ($1, $2, $3, $4, $5)
        ON CONFLICT (snapshot_id) DO UPDATE SET
            generated_at = EXCLUDED.generated_at,
            state_hash = EXCLUDED.state_hash,
            version = EXCLUDED.version,
            data = EXCLUDED.data
        "#,
        envelope.meta.snapshot_id,
        envelope.meta.generated_at,
        envelope.meta.state_hash,
        envelope.meta.version,
        data_json
    )
    .execute(db)
    .await?;
    Ok(())
}
