use crate::*;

/// Wilder's RSI over a close-price series. Returns one entry per input index;
/// `None` until `period` samples exist. Pure math (no I/O) — the anti-top-buying
/// tripwire per issue #29 ("check the RSI and buy when oversold").
pub(crate) fn compute_rsi_series(closes: &[f64], period: usize) -> Vec<Option<f64>> {
    let mut out = vec![None; closes.len()];
    if period == 0 || closes.len() <= period {
        return out;
    }
    let mut gain_sum = 0.0f64;
    let mut loss_sum = 0.0f64;
    for w in 1..=period {
        let diff = closes[w] - closes[w - 1];
        if diff > 0.0 { gain_sum += diff; } else { loss_sum -= diff; }
    }
    let mut avg_gain = gain_sum / period as f64;
    let mut avg_loss = loss_sum / period as f64;
    // No movement at all (g = l = 0) leaves the RSI undefined, not overbought.
    let rsi = |g: f64, l: f64| {
        if g == 0.0 && l == 0.0 {
            None
        } else if l == 0.0 {
            Some(100.0)
        } else {
            Some(100.0 - 100.0 / (1.0 + g / l))
        }
    };
    out[period] = rsi(avg_gain, avg_loss);
    for i in period + 1..closes.len() {
        let diff = closes[i] - closes[i - 1];
        let (g, l) = if diff > 0.0 { (diff, 0.0) } else { (0.0, -diff) };
        avg_gain = (avg_gain * (period as f64 - 1.0) + g) / period as f64;
        avg_loss = (avg_loss * (period as f64 - 1.0) + l) / period as f64;
        out[i] = rsi(avg_gain, avg_loss);
    }
    out
}

/// Snapshot `regime`: "unknown" when no daily RSI was computed (fresh database, fewer
/// than 15 daily closes, or a flat series), never a "normal" read from no data.
pub(crate) fn regime_label(regime: &MarketRegime) -> &'static str {
    if regime.overextended {
        "overextended"
    } else if regime.rsi_1d.is_none() {
        "unknown"
    } else {
        "normal"
    }
}

pub(crate) const RSI_OVERBOUGHT: f64 = 70.0;
pub(crate) const RSI_PERIOD: usize = 14;

/// Provenance tags for market_price_history rows. Historical rows keep
/// "aerodrome_rpc" (the retired feed); live spots now come from the tracked
/// Uniswap pools.
pub(crate) const PRICE_SOURCE_RPC: &str = "uniswap_rpc";
pub(crate) const PRICE_SOURCE_GECKOTERMINAL: &str = "geckoterminal";
pub(crate) const PRICE_SOURCE_DEXSCREENER: &str = "dexscreener";

/// Parse a GeckoTerminal OHLCV response. `data.attributes.ohlcv_list` entries are
/// [open_time_unix_secs, open, high, low, close, volume], newest-first; malformed
/// entries are skipped and the result is normalized to ascending time. Pure —
/// unit-testable against recorded fixtures.
pub(crate) fn parse_geckoterminal_ohlcv(body: &serde_json::Value) -> Vec<PriceCandle> {
    let mut out = Vec::new();
    if let Some(list) = body
        .get("data")
        .and_then(|d| d.get("attributes"))
        .and_then(|a| a.get("ohlcv_list"))
        .and_then(|l| l.as_array())
    {
        for entry in list {
            let Some(row) = entry.as_array() else { continue };
            if row.len() < 5 {
                continue;
            }
            let (Some(secs), Some(open), Some(high), Some(low), Some(close)) = (
                row[0].as_f64(),
                row[1].as_f64(),
                row[2].as_f64(),
                row[3].as_f64(),
                row[4].as_f64(),
            ) else {
                continue;
            };
            let Some(ts) = DateTime::<Utc>::from_timestamp(secs as i64, 0) else {
                continue;
            };
            if !(open.is_finite() && high.is_finite() && low.is_finite() && close.is_finite())
                || open <= 0.0
            {
                continue;
            }
            out.push(PriceCandle {
                timestamp: ts,
                open,
                high,
                low,
                close,
                volume: row.get(5).and_then(|v| v.as_f64()),
            });
        }
    }
    out.sort_by_key(|c| c.timestamp);
    out.dedup_by_key(|c| c.timestamp);
    out
}

/// Parse DEXScreener's current pair price (fallback source; exposes no history).
pub(crate) fn parse_dexscreener_price(body: &serde_json::Value) -> Option<f64> {
    body.get("pairs")?
        .get(0)?
        .get("priceUsd")?
        .as_str()?
        .parse::<f64>()
        .ok()
        .filter(|p| p.is_finite() && *p > 0.0)
}

/// Decide the backfill actions for this cycle. A gap exists when the table is
/// empty or the newest point is older than `gap_threshold_minutes`; depth is
/// insufficient when the earliest point is newer than `now - min_history_days`.
pub(crate) fn plan_backfill(
    last_point: Option<DateTime<Utc>>,
    earliest_point: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    gap_threshold_minutes: i64,
    min_history_days: i64,
) -> BackfillPlan {
    let gap = match last_point {
        None => true,
        Some(last) => {
            now.signed_duration_since(last) > chrono::Duration::minutes(gap_threshold_minutes)
        }
    };
    let depth_insufficient = match earliest_point {
        None => true,
        Some(earliest) => earliest > now - chrono::Duration::days(min_history_days),
    };
    BackfillPlan {
        backfill_hourly: gap,
        backfill_daily: depth_insufficient,
        seed_fallback_point: gap,
    }
}

/// Collapse raw price points (timestamp, candle_close, spot_price) into one close
/// per UTC day. Daily candle closes are authoritative; days without a candle take
/// the last arriving spot (points are ordered ascending). Pure — unit-testable.
pub(crate) fn daily_closes_from_points(points: &[(DateTime<Utc>, Option<f64>, f64)]) -> Vec<f64> {
    use std::collections::BTreeMap;
    // day -> (has_candle_close, close)
    let mut days: BTreeMap<chrono::NaiveDate, (bool, f64)> = BTreeMap::new();
    for &(ts, close_usd, price_usd) in points {
        let day = ts.date_naive();
        match close_usd {
            Some(c) => {
                days.insert(day, (true, c));
            }
            None => {
                let entry = days.entry(day).or_insert((false, price_usd));
                if !entry.0 {
                    entry.1 = price_usd;
                }
            }
        }
    }
    days.into_values().map(|(_, close)| close).collect()
}

/// Latest and earliest stored price points (both None when the table is empty).
pub(crate) async fn price_history_bounds(
    db: &PgPool,
) -> Result<(Option<DateTime<Utc>>, Option<DateTime<Utc>>)> {
    let row = sqlx::query(
        "SELECT MAX(timestamp) AS last_ts, MIN(timestamp) AS earliest_ts FROM market_price_history",
    )
    .fetch_one(db)
    .await?;
    let last: Option<DateTime<Utc>> = row.try_get("last_ts").unwrap_or(None);
    let earliest: Option<DateTime<Utc>> = row.try_get("earliest_ts").unwrap_or(None);
    Ok((last, earliest))
}

/// Insert a single price point with explicit provenance. `ON CONFLICT DO NOTHING`
/// keeps every write idempotent: the first row at a timestamp wins and is never
/// rewritten by a later backfill.
pub(crate) async fn insert_price_point(
    db: &PgPool,
    ts: DateTime<Utc>,
    price_usd: f64,
    source: &str,
    is_backfilled: bool,
) -> Result<()> {
    sqlx::query(
        "INSERT INTO market_price_history (timestamp, price_usd, source, is_backfilled) \
         VALUES ($1, $2, $3, $4) ON CONFLICT (timestamp) DO NOTHING",
    )
    .bind(ts)
    .bind(price_usd)
    .bind(source)
    .bind(is_backfilled)
    .execute(db)
    .await?;
    Ok(())
}

/// Idempotent candle upsert. Only completed candles are accepted: an open time
/// less than one candle period old is the provider's in-progress candle and is
/// skipped so a partial close never gets frozen into history. Returns rows added.
pub(crate) async fn upsert_price_candles(
    db: &PgPool,
    candles: &[PriceCandle],
    source: &str,
    candle_period: chrono::Duration,
    now: DateTime<Utc>,
) -> Result<u64> {
    let mut added = 0u64;
    for c in candles {
        if now.signed_duration_since(c.timestamp) < candle_period {
            continue;
        }
        let result = sqlx::query(
            "INSERT INTO market_price_history \
             (timestamp, price_usd, open_usd, high_usd, low_usd, close_usd, volume_usd, source, is_backfilled, backfilled_at) \
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, true, $9) ON CONFLICT (timestamp) DO NOTHING",
        )
        .bind(c.timestamp)
        .bind(c.close)
        .bind(c.open)
        .bind(c.high)
        .bind(c.low)
        .bind(c.close)
        .bind(c.volume)
        .bind(source)
        .bind(now)
        .execute(db)
        .await?;
        added += result.rows_affected();
    }
    Ok(added)
}

/// GET a GeckoTerminal OHLCV series (unauthenticated public endpoint). Returns
/// None on any network/HTTP/parse failure — the caller degrades instead of failing.
pub(crate) async fn fetch_geckoterminal_ohlcv(
    state: &Arc<AppState>,
    timeframe: &str,
    limit: u32,
) -> Option<Vec<PriceCandle>> {
    let url = format!(
        "{}/networks/base/pools/{}/ohlcv/{}?limit={}",
        state.config.market_data.geckoterminal_base_url.trim_end_matches('/'),
        state.config.market_data.pool_address,
        timeframe,
        limit
    );
    let resp = match state
        .http_client
        .get(&url)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("GeckoTerminal {timeframe} OHLCV request failed: {e}");
            return None;
        }
    };
    if !resp.status().is_success() {
        tracing::warn!("GeckoTerminal {timeframe} OHLCV returned HTTP {}", resp.status());
        return None;
    }
    match resp.json::<serde_json::Value>().await {
        Ok(body) => Some(parse_geckoterminal_ohlcv(&body)),
        Err(e) => {
            tracing::warn!("GeckoTerminal {timeframe} OHLCV body parse failed: {e}");
            None
        }
    }
}

/// GET DEXScreener's current pair price (fallback; no historical candles).
pub(crate) async fn fetch_dexscreener_price(state: &Arc<AppState>) -> Option<f64> {
    let url = format!(
        "{}/dex/pairs/base/{}",
        state.config.market_data.dexscreener_base_url.trim_end_matches('/'),
        state.config.market_data.pool_address
    );
    let resp = match state
        .http_client
        .get(&url)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
    {
        Ok(r) => r,
        Err(e) => {
            tracing::warn!("DEXScreener request failed: {e}");
            return None;
        }
    };
    if !resp.status().is_success() {
        tracing::warn!("DEXScreener returned HTTP {}", resp.status());
        return None;
    }
    match resp.json::<serde_json::Value>().await {
        Ok(body) => parse_dexscreener_price(&body),
        Err(e) => {
            tracing::warn!("DEXScreener body parse failed: {e}");
            None
        }
    }
}

/// Gap detection + backfill (v0.3.0). Heals multi-hour/multi-day gaps in
/// `market_price_history` (laptop sleep, worker downtime) from free public OHLCV
/// feeds, and extends daily depth for the multi-day RSI. Strictly fail-open: any
/// provider or DB failure logs a warning and returns — the ingestion loop and
/// on-chain polling are never blocked. Idempotent: reruns upsert nothing new.
pub(crate) async fn ensure_price_history(state: &Arc<AppState>) {
    if !state.config.market_data.backfill_enabled {
        return;
    }
    let md = &state.config.market_data;
    let (last_point, earliest_point) = match price_history_bounds(&state.db).await {
        Ok(b) => b,
        Err(e) => {
            tracing::warn!("Price-history bounds query failed (backfill skipped): {e}");
            return;
        }
    };
    let now = Utc::now();
    let plan = plan_backfill(
        last_point,
        earliest_point,
        now,
        md.gap_threshold_minutes,
        md.min_history_days,
    );
    if plan == BackfillPlan::default() {
        return;
    }
    tracing::info!(
        "Price backfill: hourly={} daily={} (last={:?}, earliest={:?})",
        plan.backfill_hourly,
        plan.backfill_daily,
        last_point,
        earliest_point
    );

    let mut provider_ok = true;
    if plan.backfill_daily {
        let candles = fetch_geckoterminal_ohlcv(state, "day", md.daily_lookback).await;
        match candles {
            Some(c) => match upsert_price_candles(
                &state.db,
                &c,
                PRICE_SOURCE_GECKOTERMINAL,
                chrono::Duration::days(1),
                now,
            )
            .await
            {
                Ok(n) => tracing::info!("Backfilled {n} daily candle(s)"),
                Err(e) => tracing::warn!("Daily candle upsert failed (non-fatal): {e}"),
            },
            None => provider_ok = false,
        }
    }
    if plan.backfill_hourly {
        let candles = fetch_geckoterminal_ohlcv(state, "hour", md.hourly_lookback).await;
        match candles {
            Some(c) => match upsert_price_candles(
                &state.db,
                &c,
                PRICE_SOURCE_GECKOTERMINAL,
                chrono::Duration::hours(1),
                now,
            )
            .await
            {
                Ok(n) => tracing::info!("Backfilled {n} hourly candle(s)"),
                Err(e) => tracing::warn!("Hourly candle upsert failed (non-fatal): {e}"),
            },
            None => provider_ok = false,
        }
    }

    // DEXScreener fallback: it exposes only the current price, so it may seed a
    // single point stamped at observation time — never backdated into the gap.
    if !provider_ok && plan.seed_fallback_point {
        match fetch_dexscreener_price(state).await {
            Some(price) => {
                tracing::warn!(
                    "GeckoTerminal unavailable; seeding current DEXScreener price (degraded, single point)"
                );
                if let Err(e) =
                    insert_price_point(&state.db, now, price, PRICE_SOURCE_DEXSCREENER, true).await
                {
                    tracing::warn!("DEXScreener seed insert failed (non-fatal): {e}");
                }
            }
            None => {
                tracing::warn!(
                    "Both GeckoTerminal and DEXScreener unavailable; price backfill degraded this cycle"
                )
            }
        }
    }
}

/// Backfill telemetry for /v1/snapshot: how much of the stored price series came
/// from third-party gap fills and when the last one landed. Computed live per
/// request; never persisted into snapshots (keeps state_hash/ETag stable). Fail-open.
pub(crate) async fn data_provenance(db: &PgPool) -> serde_json::Value {
    match sqlx::query(
        "SELECT MAX(backfilled_at) AS last_backfill_at, COUNT(*) AS points \
         FROM market_price_history WHERE is_backfilled",
    )
    .fetch_one(db)
    .await
    {
        Ok(r) => {
            let last: Option<DateTime<Utc>> = r.try_get("last_backfill_at").unwrap_or(None);
            let points: i64 = r.try_get("points").unwrap_or(0);
            // Per-source breakdown so consumers can name the feed instead of
            // hardcoding one (geckoterminal is the norm; dexscreener is the fallback).
            let mut sources = serde_json::Map::new();
            if let Ok(rows) = sqlx::query(
                "SELECT source, COUNT(*) AS points FROM market_price_history \
                 WHERE is_backfilled GROUP BY source ORDER BY points DESC",
            )
            .fetch_all(db)
            .await
            {
                for row in rows {
                    let src: String = row.try_get("source").unwrap_or_default();
                    let n: i64 = row.try_get("points").unwrap_or(0);
                    sources.insert(src, json!(n));
                }
            }
            json!({
                "last_backfill_at": last.map(|t| t.to_rfc3339()),
                "backfilled_points_count": points,
                "sources": sources,
            })
        }
        Err(e) => {
            tracing::warn!("Provenance query failed (non-fatal): {e}");
            json!({
                "last_backfill_at": Option::<String>::None,
                "backfilled_points_count": 0,
                "sources": {},
            })
        }
    }
}
