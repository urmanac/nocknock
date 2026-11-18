use crate::*;

/// keccak256("Transfer(address,address,uint256)") — verified against the repo keccak tooling.
pub(crate) const TRANSFER_EVENT_TOPIC: &str =
    "0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef";
/// WETH9 `Withdrawal(address indexed src, uint256 wad)` — checked against live Base WETH
/// logs on 2026-11-15 (one indexed topic, amount in data).
pub(crate) const WETH_WITHDRAWAL_TOPIC: &str =
    "0x7fcf532c15f0a6db0bd6d0e038bea71d30d808c7d98cb3bf7268a95bf5081b65";
/// Uniswap v3 NonfungiblePositionManager on Base.
pub(crate) const UNISWAP_V3_NPM: &str = "0x03a520b32C04BF3bEEf7BEb72E919cf822Ed34f1";
/// Ledger token address for fees paid out as native ETH (unwrapped WETH). Native ETH has no
/// contract, so this conventional sentinel keeps (tx_hash, token_address) unique per token.
pub(crate) const NATIVE_ETH_LEDGER_ADDRESS: &str = "0xEeeeeEeeeEeEeEeEeEeEEEeeeeEeeeeeeeEEeE";

/// Extract what a fee claim paid the sovereign wallet from a transaction receipt:
/// - ERC-20 Transfer logs of USDC, NOCK or WETH to the wallet;
/// - an AERO Transfer to the wallet (issue #93): what a gauge pays when the operator
///   claims emissions. On the Aero gauges it is a mint, so its `from` is the zero
///   address (verified on two real claims, ONCHAIN_INTEGRATION.md §9);
/// - WETH unwrapped by a Uniswap position manager (a WETH `Withdrawal` whose `src` is the
///   v3 NonfungiblePositionManager or the v4 PositionManager) in a transaction the wallet
///   sent. The Uniswap app collects WETH fees this way and forwards native ETH, which
///   leaves no Transfer log; it is booked as "ETH".
/// Logs for tokens we do not track are ignored with a warning rather than guessed at.
pub(crate) fn parse_harvest_transfers(receipt: &serde_json::Value, wallet: &str) -> Vec<ParsedHarvestTransfer> {
    let mut out = Vec::new();
    let wallet_padded = format!(
        "000000000000000000000000{}",
        wallet.trim_start_matches("0x").to_lowercase()
    );
    let Some(logs) = receipt.get("logs").and_then(|l| l.as_array()) else {
        return out;
    };
    let sent_by_wallet = receipt
        .get("from")
        .and_then(|f| f.as_str())
        .is_some_and(|f| f.eq_ignore_ascii_case(wallet));
    for log in logs {
        let Some(topics) = log.get("topics").and_then(|t| t.as_array()) else {
            continue;
        };
        // Native ETH paid out by a position manager unwrapping WETH.
        if sent_by_wallet && topics.len() == 2 {
            let is_weth = log
                .get("address")
                .and_then(|a| a.as_str())
                .is_some_and(|a| a.eq_ignore_ascii_case(WETH_BASE));
            let is_withdrawal = topics[0]
                .as_str()
                .is_some_and(|t| t.eq_ignore_ascii_case(WETH_WITHDRAWAL_TOPIC));
            if is_weth && is_withdrawal {
                let src = topics[1].as_str().unwrap_or("").trim_start_matches("0x");
                let src_addr = if src.len() == 64 { format!("0x{}", src[24..].to_lowercase()) } else { String::new() };
                let by_position_manager = src_addr.eq_ignore_ascii_case(UNISWAP_V3_NPM)
                    || src_addr.eq_ignore_ascii_case(UNISWAP_V4_POSM_BASE);
                let amount = log
                    .get("data")
                    .and_then(|d| d.as_str())
                    .and_then(|d| u128::from_str_radix(d.trim_start_matches("0x"), 16).ok());
                if let (true, Some(amount)) = (by_position_manager, amount) {
                    out.push(ParsedHarvestTransfer {
                        token_address: NATIVE_ETH_LEDGER_ADDRESS.to_string(),
                        token_symbol: "ETH".to_string(),
                        from_address: src_addr,
                        recipient: wallet.to_string(),
                        amount_raw: amount.to_string(),
                        amount_human: format_units(amount, 18),
                        decimals: 18,
                    });
                }
                continue;
            }
        }
        if topics.len() < 3 {
            continue;
        }
        let (Some(t0), Some(from_word), Some(to_word)) = (
            topics[0].as_str(),
            topics[1].as_str(),
            topics[2].as_str(),
        ) else {
            continue;
        };
        if !t0.eq_ignore_ascii_case(TRANSFER_EVENT_TOPIC) {
            continue;
        }
        if !to_word
            .trim_start_matches("0x")
            .eq_ignore_ascii_case(&wallet_padded)
        {
            continue;
        }
        let Some(token_addr) = log.get("address").and_then(|a| a.as_str()) else {
            continue;
        };
        let (symbol, decimals): (&str, i32) = if token_addr.eq_ignore_ascii_case(USDC_BASE) {
            ("USDC", 6)
        } else if token_addr.eq_ignore_ascii_case(NOCK_BASE) {
            ("NOCK", 16)
        } else if token_addr.eq_ignore_ascii_case(WETH_BASE) {
            ("WETH", 18)
        } else if token_addr.eq_ignore_ascii_case(AERO_BASE) {
            ("AERO", AERO_DECIMALS)
        } else {
            tracing::warn!("Harvest receipt contains a Transfer of an untracked token {}; skipping", token_addr);
            continue;
        };
        let Some(data) = log.get("data").and_then(|d| d.as_str()) else {
            continue;
        };
        let Ok(amount) = u128::from_str_radix(data.trim_start_matches("0x"), 16) else {
            continue;
        };
        out.push(ParsedHarvestTransfer {
            token_address: token_addr.to_string(),
            token_symbol: symbol.to_string(),
            from_address: format!("0x{}", from_word.trim_start_matches("0x")[24..].to_lowercase()),
            recipient: wallet.to_string(),
            amount_raw: amount.to_string(),
            amount_human: format_units(amount, decimals),
            decimals,
        });
    }
    out
}

/// One ledger row per (transaction, token): the ledger's unique key is
/// `(tx_hash, token_address)`, so several Transfers of one token to the wallet in one
/// transaction are summed into one row before they are inserted. Without this only the
/// first was booked and the rest were dropped as "already present" (#93: a claim from
/// two gauges in one transaction mints AERO to the wallet twice). Pure.
/// - Amounts are added as integers and `amount_human` is re-derived from the sum.
/// - `from_address` is the first sender. Gauge mints all come from the zero address;
///   when senders differ the row keeps the first one.
/// - Rows keep the order in which each token first appears in the receipt.
/// An amount that cannot be summed is an error, never a truncated row.
pub(crate) fn merge_harvest_transfers(transfers: Vec<ParsedHarvestTransfer>) -> Result<Vec<ParsedHarvestTransfer>> {
    let mut merged: Vec<(ParsedHarvestTransfer, u128)> = Vec::new();
    for t in transfers {
        let amount: u128 = t
            .amount_raw
            .parse()
            .map_err(|e| anyhow::anyhow!("{} transfer amount '{}' is not an integer: {e}", t.token_symbol, t.amount_raw))?;
        match merged.iter_mut().find(|(m, _)| m.token_address.eq_ignore_ascii_case(&t.token_address)) {
            Some((m, sum)) => {
                *sum = sum
                    .checked_add(amount)
                    .ok_or_else(|| anyhow::anyhow!("{} transfers in one transaction overflow 128 bits", m.token_symbol))?;
            }
            None => merged.push((t, amount)),
        }
    }
    Ok(merged
        .into_iter()
        .map(|(mut t, sum)| {
            t.amount_raw = sum.to_string();
            t.amount_human = format_units(sum, t.decimals);
            t
        })
        .collect())
}

/// Fills the USD value of a ledger row that was recorded without one (the price read at
/// the claim's block failed at the time), when the same transaction is recorded again
/// and the price is now readable. It never touches a row that already has a value, and
/// only a row booking the same amount, so a retry cannot overwrite or mismatch a price.
pub(crate) const FILL_HARVEST_USD_SQL: &str = "UPDATE harvest_events \
     SET usd_value = $1, price_usd = $2, price_source = $3 \
     WHERE tx_hash = $4 AND token_address = $5 AND amount_raw = $6 AND usd_value IS NULL";

/// Net effect of a transaction on the sovereign wallet for a USDC→NOCK purchase:
/// `(usdc_spent_raw, nock_received_raw)`. Sums every USDC/NOCK Transfer log touching
/// the wallet (multi-hop routes and refunds net out) and returns `None` unless the
/// wallet ended with strictly less USDC and strictly more NOCK. Pure — unit-testable.
pub(crate) fn parse_accumulation_swap(receipt: &serde_json::Value, wallet: &str) -> Option<(u128, u128)> {
    let wallet_padded = format!(
        "000000000000000000000000{}",
        wallet.trim_start_matches("0x").to_lowercase()
    );
    let logs = receipt.get("logs").and_then(|l| l.as_array())?;
    let (mut usdc_out, mut usdc_in, mut nock_out, mut nock_in) = (0u128, 0u128, 0u128, 0u128);
    for log in logs {
        let Some(topics) = log.get("topics").and_then(|t| t.as_array()) else { continue };
        if topics.len() < 3 {
            continue;
        }
        let (Some(t0), Some(from_word), Some(to_word)) =
            (topics[0].as_str(), topics[1].as_str(), topics[2].as_str())
        else {
            continue;
        };
        if !t0.eq_ignore_ascii_case(TRANSFER_EVENT_TOPIC) {
            continue;
        }
        let Some(token) = log.get("address").and_then(|a| a.as_str()) else { continue };
        let is_usdc = token.eq_ignore_ascii_case(USDC_BASE);
        let is_nock = token.eq_ignore_ascii_case(NOCK_BASE);
        if !is_usdc && !is_nock {
            continue;
        }
        let Some(amount) = log
            .get("data")
            .and_then(|d| d.as_str())
            .and_then(|d| u128::from_str_radix(d.trim_start_matches("0x"), 16).ok())
        else {
            continue;
        };
        let from_wallet = from_word.trim_start_matches("0x").eq_ignore_ascii_case(&wallet_padded);
        let to_wallet = to_word.trim_start_matches("0x").eq_ignore_ascii_case(&wallet_padded);
        match (is_usdc, from_wallet, to_wallet) {
            (true, true, false) => usdc_out = usdc_out.saturating_add(amount),
            (true, false, true) => usdc_in = usdc_in.saturating_add(amount),
            (false, true, false) => nock_out = nock_out.saturating_add(amount),
            (false, false, true) => nock_in = nock_in.saturating_add(amount),
            _ => {}
        }
    }
    if usdc_out > usdc_in && nock_in > nock_out {
        Some((usdc_out - usdc_in, nock_in - nock_out))
    } else {
        None
    }
}

/// POST /v1/execute/record-accumulation — attach a confirmed USDC→NOCK purchase to
/// the configured accumulation plan (issue #79). The swap may have been made anywhere
/// (aero.xyz, a wallet's swap screen): acceptance depends only on the on-chain
/// receipt showing USDC leaving, and NOCK entering, the tracked wallet. Amounts come
/// from the Transfer logs. Idempotent on tx_hash.
pub(crate) async fn record_accumulation_handler(
    Extension(state): Extension<Arc<AppState>>,
    Json(req): Json<RecordAccumulationRequest>,
) -> impl IntoResponse {
    let err = |code: StatusCode, error: &str, detail: &str| {
        (code, Json(json!({ "error": error, "detail": detail }))).into_response()
    };
    let Some(plan) = state.config.accumulation_plan.clone() else {
        return err(StatusCode::CONFLICT, "NO_PLAN", "No accumulation_plan is configured; nothing to record against");
    };
    if !is_valid_tx_hash(&req.tx_hash) {
        return err(StatusCode::UNPROCESSABLE_ENTITY, "INVALID_TX_HASH", "tx_hash must be a 32-byte hex string");
    }
    let tx_hash = if req.tx_hash.starts_with("0x") {
        req.tx_hash.to_lowercase()
    } else {
        format!("0x{}", req.tx_hash.to_lowercase())
    };
    let alchemy_key = state.config.alchemy_api_keys.first().cloned().unwrap_or_default();
    if alchemy_key.is_empty() || alchemy_key == "YOUR_ALCHEMY_API_KEY" {
        return err(StatusCode::SERVICE_UNAVAILABLE, "NO_RPC", "No Alchemy RPC key configured; cannot verify receipts");
    }
    let alchemy_url = format!("https://base-mainnet.g.alchemy.com/v2/{}", alchemy_key);
    let Some(wallet) = state.config.tracked_wallets.first().cloned() else {
        return err(StatusCode::SERVICE_UNAVAILABLE, "NO_TRACKED_WALLET", "No tracked wallet configured; cannot verify receipts");
    };

    let receipt_payload = json!({
        "jsonrpc": "2.0", "method": "eth_getTransactionReceipt", "params": [tx_hash], "id": 42
    });
    let receipt = match post_rpc_with_retry(&state.http_client, &alchemy_url, &receipt_payload, 2).await {
        Some(res) => res.get("result").cloned().unwrap_or(serde_json::Value::Null),
        None => return err(StatusCode::SERVICE_UNAVAILABLE, "RPC_UNAVAILABLE", "Could not query receipt; not recorded"),
    };
    // Every acceptance check lives in fill_verify.rs, shared with fill detection
    // (issue #118). The receipt checks run first so a transaction that is not a
    // purchase is rejected without a block read, exactly as before.
    let reject = |r: &FillRejection| err(r.http_status(), r.code(), &r.detail(&plan.id));
    let swap = match verify_fill_receipt(&receipt, &wallet) {
        Ok(swap) => swap,
        Err(r) => return reject(&r),
    };
    // The fill time drives the plan's cadence, so it must be the block time. Unlike
    // the harvest ledger there is no wall-clock fallback: a wrong time would shift
    // when the next tranche comes due.
    let Some(confirmed_at) = fetch_block_time(&state.http_client, &alchemy_url, swap.block_number).await else {
        return err(StatusCode::SERVICE_UNAVAILABLE, "RPC_UNAVAILABLE", "Could not read the block timestamp; not recorded");
    };

    // A buy made before the plan existed is not part of the plan: without this an old
    // hash pasted by mistake would advance, or even complete, a brand-new plan.
    let registered_at = match plan_registered_at(&state.db, &plan.id).await {
        Ok(t) => t,
        Err(e) => {
            tracing::error!("Accumulation plan registry unavailable: {e}");
            return err(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", "Could not read the plan registry; nothing recorded");
        }
    };
    let anchor = plan_anchor(plan.starts_at, registered_at);
    let fill = match verify_fill(&receipt, &wallet, confirmed_at, anchor) {
        Ok(fill) => fill,
        Err(r) => return reject(&r),
    };
    let (usdc_raw, nock_raw, block_number) = (fill.swap.usdc_raw, fill.swap.nock_raw, fill.swap.block_number);
    let (usdc_spent, nock_received, effective_price) =
        (fill.swap.usdc_spent.clone(), fill.swap.nock_received.clone(), fill.swap.effective_price_usd.clone());
    // Execution cost over the reference spot at the fill's block time (issue #118). No
    // stored price near that time means no cost figure: all three columns stay null.
    // A swap confirmed from an offer keeps the figure the offer showed.
    let reference = reference_for_recorded_fill(&state.db, &wallet, &tx_hash, &effective_price, confirmed_at).await;
    // Cost over the best quote that was current for this tranche (issue #115): the
    // latest stored quote read before the fill's block, for this tranche and within the
    // hour before the fill. None on record means all eight columns stay null; a later
    // quote, or one read for the tranche before, is never used.
    let quote = fill_quote_reference(
        &effective_price,
        quote_for_fill(&state.db, &plan.id, &wallet, block_number, confirmed_at).await,
    );

    let res = sqlx::query(
        "INSERT INTO accumulation_events \
            (plan_id, tx_hash, venue, usdc_spent_raw, nock_received_raw, usdc_spent, nock_received, effective_price_usd, block_number, confirmed_at, \
             reference_spot_usd, reference_spot_source, execution_cost_bps, \
             quote_venue, quote_route, quote_usdc, quote_nock_out, quote_effective_price_usd, quote_block_number, \
             quoted_at, quote_cost_bps) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21) \
         ON CONFLICT (tx_hash) DO NOTHING",
    )
    .bind(&plan.id)
    .bind(&tx_hash)
    // The column is NOT NULL; a plan without a venue label stores "" and the API
    // reads it back as null (issue #132).
    .bind(plan_venue_label(&plan).unwrap_or_default())
    .bind(usdc_raw.to_string())
    .bind(nock_raw.to_string())
    .bind(&usdc_spent)
    .bind(&nock_received)
    .bind(&effective_price)
    .bind(block_number)
    .bind(confirmed_at)
    .bind(&reference.spot_usd)
    .bind(&reference.source)
    .bind(reference.cost_bps)
    .bind(&quote.venue)
    .bind(&quote.route)
    .bind(&quote.usdc)
    .bind(&quote.nock_out)
    .bind(&quote.effective_price_usd)
    .bind(quote.block_number)
    .bind(quote.quoted_at)
    .bind(quote.cost_bps)
    .execute(&state.db)
    .await;
    match res {
        Ok(r) => Json(json!({
            "recorded": r.rows_affected() > 0,
            "duplicate": r.rows_affected() == 0,
            "plan_id": plan.id,
            "tx_hash": tx_hash,
            "usdc_spent": usdc_spent,
            "nock_received": nock_received,
            "effective_price_usd": effective_price,
            "block_number": block_number,
            "confirmed_at": confirmed_at.to_rfc3339(),
            "reference_spot_usd": reference.spot_usd,
            "reference_spot_source": reference.source,
            "execution_cost_bps": reference.cost_bps,
            // Null together when no quote was on record for this tranche (#115). On a
            // duplicate these describe this request, not the stored row.
            "quote_venue": quote.venue,
            "quote_route": quote.route,
            "quote_usdc": quote.usdc,
            "quote_nock_out": quote.nock_out,
            "quote_effective_price_usd": quote.effective_price_usd,
            "quote_block_number": quote.block_number,
            "quoted_at": quote.quoted_at.map(|t| t.to_rfc3339()),
            "quote_cost_bps": quote.cost_bps,
            "note": "Plan status updates on the next ingestion cycle (POST /v1/ingest to refresh now)."
        }))
        .into_response(),
        Err(e) => {
            tracing::error!("Failed to insert accumulation event: {e}");
            err(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", "Ledger insert failed; nothing recorded")
        }
    }
}

/// GET /v1/accumulation/events — receipt-verified fills, oldest first.
pub(crate) async fn accumulation_events_handler(Extension(state): Extension<Arc<AppState>>) -> impl IntoResponse {
    let rows = sqlx::query(
        "SELECT plan_id, tx_hash, venue, usdc_spent, nock_received, effective_price_usd, block_number, confirmed_at, \
                reference_spot_usd, reference_spot_source, execution_cost_bps, \
                quote_venue, quote_route, quote_usdc, quote_nock_out, quote_effective_price_usd, quote_block_number, \
                quoted_at, quote_cost_bps \
         FROM accumulation_events ORDER BY confirmed_at ASC, id ASC",
    )
    .fetch_all(&state.db)
    .await;
    match rows {
        Ok(rows) => {
            let events: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    json!({
                        "plan_id": r.try_get::<String, _>("plan_id").unwrap_or_default(),
                        "tx_hash": r.try_get::<String, _>("tx_hash").unwrap_or_default(),
                        "venue": r.try_get::<String, _>("venue").ok().filter(|v| !v.is_empty()),
                        "usdc_spent": r.try_get::<String, _>("usdc_spent").unwrap_or_default(),
                        "nock_received": r.try_get::<String, _>("nock_received").unwrap_or_default(),
                        "effective_price_usd": r.try_get::<String, _>("effective_price_usd").unwrap_or_default(),
                        "block_number": r.try_get::<i64, _>("block_number").unwrap_or(0),
                        "confirmed_at": r
                            .try_get::<DateTime<Utc>, _>("confirmed_at")
                            .map(|t| t.to_rfc3339())
                            .unwrap_or_default(),
                        // Null when no stored price was near the fill's block time (#118).
                        "reference_spot_usd": r.try_get::<Option<String>, _>("reference_spot_usd").ok().flatten(),
                        "reference_spot_source": r.try_get::<Option<String>, _>("reference_spot_source").ok().flatten(),
                        "execution_cost_bps": r.try_get::<Option<i32>, _>("execution_cost_bps").ok().flatten(),
                        // Null together: no quote was on record for this fill's tranche (#115).
                        "quote_venue": r.try_get::<Option<String>, _>("quote_venue").ok().flatten(),
                        "quote_route": r.try_get::<Option<String>, _>("quote_route").ok().flatten(),
                        "quote_usdc": r.try_get::<Option<String>, _>("quote_usdc").ok().flatten(),
                        "quote_nock_out": r.try_get::<Option<String>, _>("quote_nock_out").ok().flatten(),
                        "quote_effective_price_usd": r.try_get::<Option<String>, _>("quote_effective_price_usd").ok().flatten(),
                        "quote_block_number": r.try_get::<Option<i64>, _>("quote_block_number").ok().flatten(),
                        "quoted_at": r
                            .try_get::<Option<DateTime<Utc>>, _>("quoted_at")
                            .ok()
                            .flatten()
                            .map(|t| t.to_rfc3339()),
                        "quote_cost_bps": r.try_get::<Option<i32>, _>("quote_cost_bps").ok().flatten(),
                    })
                })
                .collect();
            Json(json!({ "events": events })).into_response()
        }
        Err(e) => {
            tracing::error!("Accumulation events query failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "DB_ERROR", "detail": "Could not read the accumulation ledger" })),
            )
                .into_response()
        }
    }
}

pub(crate) fn is_valid_tx_hash(s: &str) -> bool {
    let clean = s.trim_start_matches("0x");
    clean.len() == 64 && clean.chars().all(|c| c.is_ascii_hexdigit())
}

/// How far from a claim's block time a stored NOCK price may be and still value it. Live
/// cycles store one about every 5 minutes; backfilled candles are coarser.
pub(crate) const HARVEST_NOCK_PRICE_WINDOW_MINUTES: i64 = 60;

/// Unit price for a harvested token: USDC is $1 by definition of the ledger, NOCK and
/// ETH/WETH use the prices found for the claim's block. Pure.
pub(crate) fn harvest_unit_price<'a>(
    symbol: &str,
    nock: Option<&'a (f64, String)>,
    eth: Option<&'a (f64, String)>,
) -> Option<(f64, &'a str)> {
    match symbol {
        "USDC" => Some((1.0, "USDC at $1")),
        "NOCK" => nock.map(|(p, s)| (*p, s.as_str())),
        "ETH" | "WETH" => eth.map(|(p, s)| (*p, s.as_str())),
        _ => None,
    }
}

/// USD value of a harvested amount at a unit price; None when the price is unknown or
/// the amount does not parse. Pure.
pub(crate) fn harvest_usd_value(amount_human: &str, unit_price: Option<f64>) -> Option<String> {
    let amount = amount_human.parse::<f64>().ok()?;
    let price = unit_price.filter(|p| p.is_finite() && *p > 0.0)?;
    Some(format!("{:.4}", amount * price))
}

/// The statement behind `nock_price_near` (#129). The table holds two kinds of row. An
/// observation (the worker's own read, or a seeded point) is stamped when it was taken
/// and has no OHLC columns: its `price_usd` is the price at `timestamp`. A third-party
/// candle is stored at its OPEN time with `price_usd` set to its CLOSE, which is the
/// price one candle period later (an hour, or a day). So a candle counts as its open,
/// never its close, and a candle with no positive open gives no price. Inside the window
/// the worker's own read nearest `at` wins; a backfilled point is used only when there
/// is none. This is the same rule the cost-basis series applies (`nearest_spot`).
pub(crate) const NOCK_PRICE_NEAR_SQL: &str =
    "SELECT timestamp, COALESCE(open_usd, price_usd) AS price_usd, source FROM market_price_history \
     WHERE timestamp BETWEEN $2 AND $3 AND COALESCE(open_usd, price_usd) > 0 \
     ORDER BY is_backfilled ASC, ABS(EXTRACT(EPOCH FROM (timestamp - $1))) ASC LIMIT 1";

/// The stored NOCK/USD price that stands for `at`, within
/// HARVEST_NOCK_PRICE_WINDOW_MINUTES, with its provenance: the price at the row's own
/// timestamp (see NOCK_PRICE_NEAR_SQL). None when no stored price is close enough or the
/// query fails.
pub(crate) async fn nock_price_near(db: &PgPool, at: DateTime<Utc>) -> Option<(f64, String)> {
    let window = chrono::Duration::minutes(HARVEST_NOCK_PRICE_WINDOW_MINUTES);
    let row = sqlx::query(NOCK_PRICE_NEAR_SQL)
    .bind(at)
    .bind(at - window)
    .bind(at + window)
    .fetch_optional(db)
    .await
    .map_err(|e| tracing::warn!("NOCK price lookup for a harvest failed: {e}"))
    .ok()??;
    let ts: DateTime<Utc> = row.try_get("timestamp").ok()?;
    let price: f64 = row.try_get("price_usd").ok()?;
    let source: String = row.try_get("source").unwrap_or_default();
    (price.is_finite() && price > 0.0).then(|| (price, format!("market_price_history {source} at {}", ts.to_rfc3339())))
}

/// POST /v1/execute/record-harvest — receipt-verified harvest ledger entry.
/// Clients call this after a claim transaction is confirmed by the wallet.
/// "Receipt not found" also covers the Uniswap-app pending-broadcast quirk
/// (approved but not yet broadcast): the client should surface that state and retry.
pub(crate) async fn record_harvest_handler(
    Extension(state): Extension<Arc<AppState>>,
    Json(req): Json<RecordHarvestRequest>,
) -> impl IntoResponse {
    // The same four values a position's `protocol` may carry (adapters::Protocol).
    if Protocol::parse(&req.platform).is_none() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "error": "INVALID_PLATFORM", "detail": "platform must be 'aerodrome', 'aerodrome-cl', 'uniswap-v4' or 'uniswap-v3'" })),
        )
            .into_response();
    }
    if !is_valid_tx_hash(&req.tx_hash) {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "error": "INVALID_TX_HASH", "detail": "tx_hash must be a 32-byte hex string" })),
        )
            .into_response();
    }
    let alchemy_key = state
        .config
        .alchemy_api_keys
        .first()
        .cloned()
        .unwrap_or_default();
    if alchemy_key.is_empty() || alchemy_key == "YOUR_ALCHEMY_API_KEY" {
        return (
            StatusCode::SERVICE_UNAVAILABLE,
            Json(json!({ "error": "NO_RPC", "detail": "No Alchemy RPC key configured; cannot verify receipts" })),
        )
            .into_response();
    }
    let alchemy_url = format!("https://base-mainnet.g.alchemy.com/v2/{}", alchemy_key);
    let wallet = match state.config.tracked_wallets.first().cloned() {
        Some(w) => w,
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "NO_TRACKED_WALLET", "detail": "No tracked wallet configured; cannot verify receipts" })),
            )
                .into_response();
        }
    };

    let receipt_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_getTransactionReceipt",
        "params": [if req.tx_hash.starts_with("0x") { req.tx_hash.clone() } else { format!("0x{}", req.tx_hash) }],
        "id": 40
    });
    let receipt = match post_rpc_with_retry(&state.http_client, &alchemy_url, &receipt_payload, 2).await {
        Some(res) => res.get("result").cloned().unwrap_or(serde_json::Value::Null),
        None => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "RPC_UNAVAILABLE", "detail": "Could not query receipt; not recorded" })),
            )
                .into_response()
        }
    };
    if receipt.is_null() {
        return (
            StatusCode::NOT_FOUND,
            Json(json!({
                "error": "RECEIPT_NOT_FOUND",
                "detail": "Transaction not found on-chain. If the wallet shows it approved, it may still be pending broadcast in the wallet (Uniswap app quirk) - surface that state and retry once it broadcasts."
            })),
        )
            .into_response();
    }
    if receipt.get("status").and_then(|s| s.as_str()) != Some("0x1") {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "error": "TX_REVERTED", "detail": "Receipt status is not success; nothing recorded" })),
        )
            .into_response();
    }
    // One row per token: several payments of one token in the transaction are summed.
    let transfers = match merge_harvest_transfers(parse_harvest_transfers(&receipt, &wallet)) {
        Ok(t) => t,
        Err(e) => {
            return (
                StatusCode::UNPROCESSABLE_ENTITY,
                Json(json!({ "error": "UNREADABLE_TRANSFERS", "detail": format!("Receipt transfers could not be summed; nothing recorded: {e}") })),
            )
                .into_response();
        }
    };
    if transfers.is_empty() {
        return (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "error": "NO_TRACKED_TRANSFERS", "detail": "Receipt pays the sovereign wallet no tracked token (USDC, NOCK, WETH, AERO, or ETH unwrapped by a Uniswap position manager)" })),
        )
            .into_response();
    }

    let block_number: i64 = receipt
        .get("blockNumber")
        .and_then(|b| b.as_str())
        .and_then(|s| i64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        .unwrap_or(0);
    // Tax-honest confirmed_at: the block timestamp (when the tx actually landed), not
    // the wall clock at recording time — retries after the pending-broadcast quirk can
    // record long after confirmation, which would misdate the Koinly export. Falls back
    // to now only if the block query fails.
    let confirmed_at: DateTime<Utc> = if block_number > 0 {
        let block_payload = json!({
            "jsonrpc": "2.0",
            "id": 41,
            "method": "eth_getBlockByNumber",
            "params": [format!("0x{block_number:x}"), false]
        });
        post_rpc_with_retry(&state.http_client, &alchemy_url, &block_payload, 2)
            .await
            .and_then(|res| res.get("result").cloned())
            .and_then(|blk| {
                blk.get("timestamp")
                    .and_then(|t| t.as_str())
                    .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
            })
            .and_then(|ts| Utc.timestamp_opt(ts as i64, 0).single())
            .unwrap_or_else(Utc::now)
    } else {
        Utc::now()
    };

    // USD valuation at the claim's block, never at recording time: NOCK from the stored
    // price nearest the block time, ETH/WETH from the ETH price pool read at that block,
    // AERO from the AERO price pool read at that block (#93; the pool must be AERO/USDC
    // and have in-range liquidity at that block).
    // A price we cannot find is an explicit null, never a substitute.
    let needs_nock = transfers.iter().any(|t| t.token_symbol == "NOCK");
    let needs_eth = transfers.iter().any(|t| matches!(t.token_symbol.as_str(), "ETH" | "WETH"));
    let nock_price: Option<(f64, String)> = if needs_nock {
        nock_price_near(&state.db, confirmed_at).await
    } else {
        None
    };
    let eth_price: Option<(f64, String)> = if needs_eth && block_number > 0 {
        let pool = state.config.eth_price_pool.clone().unwrap_or_else(|| DEFAULT_ETH_PRICE_POOL.to_string());
        fetch_eth_price_usd_at(&state.http_client, &alchemy_url, &pool, &format!("0x{block_number:x}"))
            .await
            .map(|p| (p, format!("eth_price_pool {pool} slot0 at block {block_number}")))
    } else {
        None
    };

    let needs_aero = transfers.iter().any(|t| t.token_symbol == "AERO");
    let aero_price: Option<(f64, String)> = if needs_aero && block_number > 0 {
        let pool = state.config.aero_price_pool.clone().unwrap_or_else(|| DEFAULT_AERO_PRICE_POOL.to_string());
        match fetch_aero_price_usd_at(&state.http_client, &alchemy_url, &pool, &format!("0x{block_number:x}")).await {
            Ok(p) => Some((p, format!("aero_price_pool {pool} slot0 at block {block_number}"))),
            Err(e) => {
                tracing::warn!("AERO price at block {block_number} unavailable; the AERO row gets no USD value: {e}");
                None
            }
        }
    } else {
        None
    };

    let mut recorded = Vec::new();
    let mut priced = Vec::new();
    let mut skipped = 0usize;
    for t in &transfers {
        let price = harvest_unit_price_with_aero(&t.token_symbol, nock_price.as_ref(), eth_price.as_ref(), aero_price.as_ref());
        let usd_value = harvest_usd_value(&t.amount_human, price.map(|(p, _)| p));
        let price_usd = price.map(|(p, _)| format!("{p}"));
        let price_source = price.map(|(_, s)| s.to_string());
        let res = sqlx::query(
            "INSERT INTO harvest_events (platform, tx_hash, token_address, token_symbol, from_address, recipient, amount_raw, amount_human, decimals, usd_value, block_number, confirmed_at, price_usd, price_source)              VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)              ON CONFLICT (tx_hash, token_address) DO NOTHING",
        )
        .bind(&req.platform)
        .bind(&req.tx_hash)
        .bind(&t.token_address)
        .bind(&t.token_symbol)
        .bind(&t.from_address)
        .bind(&t.recipient)
        .bind(&t.amount_raw)
        .bind(&t.amount_human)
        .bind(t.decimals)
        .bind(&usd_value)
        .bind(block_number)
        .bind(confirmed_at)
        .bind(&price_usd)
        .bind(&price_source)
        .execute(&state.db)
        .await;
        match res {
            Ok(r) if r.rows_affected() > 0 => {
                recorded.push(json!({
                    "platform": req.platform,
                    "tx_hash": req.tx_hash,
                    "token_symbol": t.token_symbol,
                    "amount_human": t.amount_human,
                    "usd_value": usd_value,
                    "price_usd": price_usd,
                    "price_source": price_source,
                    "block_number": block_number,
                    "confirmed_at": confirmed_at.to_rfc3339(),
                }));
            }
            // Already in the ledger. If that row has no USD value (the price read at the
            // claim's block failed when it was recorded) and this attempt has one, fill
            // it: the price is the same block's, so nothing already valued can change.
            Ok(_) => {
                let filled = match usd_value.as_ref() {
                    Some(usd) => sqlx::query(FILL_HARVEST_USD_SQL)
                        .bind(usd)
                        .bind(&price_usd)
                        .bind(&price_source)
                        .bind(&req.tx_hash)
                        .bind(&t.token_address)
                        .bind(&t.amount_raw)
                        .execute(&state.db)
                        .await
                        .map(|r| r.rows_affected() > 0),
                    None => Ok(false),
                };
                match filled {
                    Ok(true) => priced.push(json!({
                        "platform": req.platform,
                        "tx_hash": req.tx_hash,
                        "token_symbol": t.token_symbol,
                        "amount_human": t.amount_human,
                        "usd_value": usd_value,
                        "price_usd": price_usd,
                        "price_source": price_source,
                        "block_number": block_number,
                    })),
                    Ok(false) => skipped += 1,
                    Err(e) => {
                        tracing::error!("Failed to fill the USD value of a harvest event: {e}");
                        return (
                            StatusCode::INTERNAL_SERVER_ERROR,
                            Json(json!({ "error": "DB_ERROR", "detail": "Ledger update failed; the row keeps no USD value" })),
                        )
                            .into_response();
                    }
                }
            }
            Err(e) => {
                tracing::error!("Failed to insert harvest event: {e}");
                return (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    Json(json!({ "error": "DB_ERROR", "detail": "Ledger insert failed; nothing partially recorded beyond this row" })),
                )
                    .into_response();
            }
        }
    }
    if recorded.is_empty() && priced.is_empty() {
        return (
            StatusCode::CONFLICT,
            Json(json!({ "error": "ALREADY_RECORDED", "detail": "All transfers from this transaction are already in the ledger", "skipped": skipped })),
        )
            .into_response();
    }
    // `priced`: rows that were already in the ledger without a USD value and now have one.
    Json(json!({ "recorded": recorded, "already_present": skipped, "priced": priced })).into_response()
}

/// Koinly custom-CSV rendering of harvest ledger rows: fee claims are tagged
/// `income` (per CHANGELOG: Income / Staking Reward) and carry their Base tx hash.
/// An AERO row is described as a reward claim, not a fee harvest (#93); its label stays
/// `income`, since the tax category is the operator's and their CPA's to change.
/// Rebalances/liquidity transfers are not yet ledgered — when they are, they export
/// with the (non-taxable) `transfer` label; documented gap, not silently omitted.
pub(crate) fn format_koinly_csv(rows: &[HarvestExportRow]) -> String {
    let header = "Date, Sent Amount, Sent Currency, Received Amount, Received Currency, Fee Amount, Fee Currency, Net Worth Amount, Net Worth Currency, Label, Description, TxHash";
    let mut out = String::from(header);
    out.push('\n');
    for r in rows {
        let (net_amount, net_currency) = match &r.usd_value {
            Some(usd) => (usd.clone(), "USD".to_string()),
            None => (String::new(), String::new()),
        };
        let description = harvest_row_description(&r.token_symbol, &r.platform);
        out.push_str(&format!(
            "{}, , , {}, {}, , , {}, {}, income, \"{}\", {}\n",
            r.confirmed_at,
            r.amount_human,
            r.token_symbol,
            net_amount,
            net_currency,
            description,
            r.tx_hash
        ));
    }
    out
}

/// GET /v1/tax/export?format=koinly — Koinly-compatible CSV (default) or JSON export
/// of the receipt-verified harvest ledger. Empty ledger exports a header-only CSV.
pub(crate) async fn tax_export_handler(
    Extension(state): Extension<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<std::collections::HashMap<String, String>>,
) -> impl IntoResponse {
    let format = params.get("format").cloned().unwrap_or_else(|| "koinly".to_string());
    let rows = match sqlx::query(
        "SELECT platform, tx_hash, token_symbol, amount_human, usd_value, confirmed_at \
         FROM harvest_events ORDER BY confirmed_at ASC, id ASC",
    )
    .fetch_all(&state.db)
    .await
    {
        Ok(rows) => {
            let mut out = Vec::new();
            for r in rows {
                let confirmed_at: DateTime<Utc> = r.try_get("confirmed_at").unwrap_or_else(|_| Utc::now());
                out.push(HarvestExportRow {
                    platform: r.try_get("platform").unwrap_or_default(),
                    tx_hash: r.try_get("tx_hash").unwrap_or_default(),
                    token_symbol: r.try_get("token_symbol").unwrap_or_default(),
                    amount_human: r.try_get("amount_human").unwrap_or_default(),
                    usd_value: r.try_get::<Option<String>, _>("usd_value").ok().flatten(),
                    confirmed_at: confirmed_at.to_rfc3339(),
                });
            }
            out
        }
        Err(e) => {
            tracing::error!("Tax export query failed: {e}");
            return (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "DB_ERROR", "detail": "Tax export query failed" })),
            )
                .into_response();
        }
    };
    match format.as_str() {
        "json" => Json(json!({ "rows": rows })).into_response(),
        "koinly" | "csv" => {
            (
                StatusCode::OK,
                [
                    ("content-type", "text/csv; charset=utf-8"),
                    (
                        "content-disposition",
                        "attachment; filename=\"nocknock-harvests-koinly.csv\"",
                    ),
                ],
                format_koinly_csv(&rows),
            )
                .into_response()
        }
        _ => (
            StatusCode::UNPROCESSABLE_ENTITY,
            Json(json!({ "error": "UNSUPPORTED_FORMAT", "detail": "format must be 'koinly' (default) or 'json'" })),
        )
            .into_response(),
    }
}
