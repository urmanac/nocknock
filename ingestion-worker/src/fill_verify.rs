use crate::*;

// ============================================================
// Plan-buy verification (issue #118)
// ============================================================
// The checks that decide whether a transaction counts as a plan buy. They are pure and
// live here once: `record_accumulation_handler` (the only write path into the ledger)
// and fill detection both call them, so what is offered and what is recorded cannot
// drift apart.

/// The amounts a receipt proves, before the block time is known.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ReceiptSwap {
    pub(crate) usdc_raw: u128,
    pub(crate) nock_raw: u128,
    pub(crate) usdc_spent: String,
    pub(crate) nock_received: String,
    /// USDC per NOCK, eight decimals.
    pub(crate) effective_price_usd: String,
    /// 0 when the receipt carries no readable block number.
    pub(crate) block_number: i64,
}

/// A swap that passed every check: receipt, amounts, block time and plan anchor.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct VerifiedFill {
    pub(crate) swap: ReceiptSwap,
    /// Block time. It drives the plan's cadence, so it is never a wall-clock time.
    pub(crate) confirmed_at: DateTime<Utc>,
}

/// Why a transaction does not count as a plan buy.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum FillRejection {
    /// The node returned no receipt (not mined, or still unbroadcast in the wallet).
    ReceiptNotFound,
    TxReverted,
    /// The receipt does not show net USDC out of, and net NOCK into, the wallet.
    NotANockPurchase,
    AmountsNotPositive,
    /// A real purchase, but confirmed before the plan's anchor. It carries the verified
    /// fill so a caller can remember the purchase without counting it.
    BeforePlanStart { fill: VerifiedFill, anchor: DateTime<Utc> },
}

impl FillRejection {
    /// Machine code returned by POST /v1/execute/record-accumulation.
    pub(crate) fn code(&self) -> &'static str {
        match self {
            FillRejection::ReceiptNotFound => "RECEIPT_NOT_FOUND",
            FillRejection::TxReverted => "TX_REVERTED",
            FillRejection::NotANockPurchase | FillRejection::AmountsNotPositive => "NOT_A_NOCK_PURCHASE",
            FillRejection::BeforePlanStart { .. } => "BEFORE_PLAN_START",
        }
    }

    pub(crate) fn http_status(&self) -> StatusCode {
        match self {
            FillRejection::ReceiptNotFound => StatusCode::NOT_FOUND,
            _ => StatusCode::UNPROCESSABLE_ENTITY,
        }
    }

    /// Human-readable detail returned with the code.
    pub(crate) fn detail(&self, plan_id: &str) -> String {
        match self {
            FillRejection::ReceiptNotFound => {
                "Transaction not found on-chain. If the wallet shows it approved, it may still be pending broadcast; retry once it lands.".to_string()
            }
            FillRejection::TxReverted => "Receipt status is not success; nothing recorded".to_string(),
            FillRejection::NotANockPurchase => {
                "Receipt does not show USDC leaving and NOCK entering the sovereign wallet".to_string()
            }
            FillRejection::AmountsNotPositive => "Parsed amounts are not positive".to_string(),
            FillRejection::BeforePlanStart { fill, anchor } => format!(
                "This purchase was confirmed at {}, before plan \"{}\" began ({}). To count earlier buys, set accumulation_plan.starts_at to an earlier time.",
                fill.confirmed_at.to_rfc3339(),
                plan_id,
                anchor.to_rfc3339()
            ),
        }
    }
}

/// The receipt checks: it exists, it succeeded, and it shows USDC leaving and NOCK
/// entering `wallet`. Amounts come from the Transfer logs. Pure.
///
/// This is the part of `verify_fill` that needs no block time. Callers run it first so
/// they do not fetch a block header for a transaction that is not a purchase.
pub(crate) fn verify_fill_receipt(receipt: &serde_json::Value, wallet: &str) -> std::result::Result<ReceiptSwap, FillRejection> {
    if receipt.is_null() {
        return Err(FillRejection::ReceiptNotFound);
    }
    if receipt.get("status").and_then(|s| s.as_str()) != Some("0x1") {
        return Err(FillRejection::TxReverted);
    }
    let Some((usdc_raw, nock_raw)) = parse_accumulation_swap(receipt, wallet) else {
        return Err(FillRejection::NotANockPurchase);
    };
    let usdc_spent = format_units(usdc_raw, 6);
    let nock_received = format_units(nock_raw, 16);
    let (usdc_f, nock_f) = (
        usdc_spent.parse::<f64>().unwrap_or(0.0),
        nock_received.parse::<f64>().unwrap_or(0.0),
    );
    if !(usdc_f > 0.0 && nock_f > 0.0) {
        return Err(FillRejection::AmountsNotPositive);
    }
    let block_number: i64 = receipt
        .get("blockNumber")
        .and_then(|b| b.as_str())
        .and_then(|s| i64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        .unwrap_or(0);
    Ok(ReceiptSwap {
        usdc_raw,
        nock_raw,
        usdc_spent,
        nock_received,
        effective_price_usd: format!("{:.8}", usdc_f / nock_f),
        block_number,
    })
}

/// A buy confirmed before the plan's anchor is not part of the plan: without this an
/// old transaction would advance, or even complete, a brand-new plan. Pure.
pub(crate) fn fill_is_before_anchor(confirmed_at: DateTime<Utc>, anchor: DateTime<Utc>) -> bool {
    confirmed_at < anchor
}

/// Every check a plan buy must pass, over (receipt, wallet, block time, plan anchor).
/// Pure. `record_accumulation_handler` records only what this returns; fill detection
/// offers only what this accepts.
pub(crate) fn verify_fill(
    receipt: &serde_json::Value,
    wallet: &str,
    block_time: DateTime<Utc>,
    anchor: DateTime<Utc>,
) -> std::result::Result<VerifiedFill, FillRejection> {
    let swap = verify_fill_receipt(receipt, wallet)?;
    let fill = VerifiedFill { swap, confirmed_at: block_time };
    if fill_is_before_anchor(block_time, anchor) {
        return Err(FillRejection::BeforePlanStart { fill, anchor });
    }
    Ok(fill)
}

/// Block time from an `eth_getBlockByNumber` response. None when the block or its
/// timestamp is missing or unreadable: never a substitute time. Pure.
pub(crate) fn block_time_from_response(resp: &serde_json::Value) -> Option<DateTime<Utc>> {
    resp.get("result")
        .and_then(|blk| blk.get("timestamp"))
        .and_then(|t| t.as_str())
        .and_then(|s| u64::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        .and_then(|ts| Utc.timestamp_opt(ts as i64, 0).single())
}

/// Read a block's time. None when the header could not be read (or the receipt carried
/// no block number); callers treat that as "not verified", never as "now".
pub(crate) async fn fetch_block_time(client: &Client, rpc_url: &str, block_number: i64) -> Option<DateTime<Utc>> {
    if block_number <= 0 {
        return None;
    }
    let payload = json!({
        "jsonrpc": "2.0", "id": 43, "method": "eth_getBlockByNumber",
        "params": [format!("0x{block_number:x}"), false]
    });
    post_rpc_with_retry(client, rpc_url, &payload, 2)
        .await
        .and_then(|res| block_time_from_response(&res))
}

/// Execution cost of a fill over a reference spot, in whole basis points:
/// (effective price / reference spot − 1) × 10,000. Positive means the fill paid more
/// than the reference. None when either price is missing, zero or not finite: an
/// unknown reference gives no cost figure, never a substitute. Pure.
pub(crate) fn execution_cost_bps(effective_price_usd: f64, reference_spot_usd: Option<f64>) -> Option<i32> {
    let reference = reference_spot_usd.filter(|p| p.is_finite() && *p > 0.0)?;
    if !(effective_price_usd.is_finite() && effective_price_usd > 0.0) {
        return None;
    }
    let bps = ((effective_price_usd / reference - 1.0) * 10_000.0).round();
    (bps.is_finite() && bps.abs() < i32::MAX as f64).then_some(bps as i32)
}

/// The reference spot for a fill and the cost it implies.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct FillReference {
    pub(crate) spot_usd: Option<String>,
    /// Where the reference came from, including its own timestamp.
    pub(crate) source: Option<String>,
    pub(crate) cost_bps: Option<i32>,
}

/// Pair an effective price with a looked-up reference. All three fields are None
/// together when there is no usable reference. Pure.
pub(crate) fn fill_reference(effective_price_usd: &str, reference: Option<(f64, String)>) -> FillReference {
    let effective = effective_price_usd.parse::<f64>().unwrap_or(f64::NAN);
    match reference {
        Some((spot, source)) => match execution_cost_bps(effective, Some(spot)) {
            Some(bps) => FillReference {
                spot_usd: Some(format!("{:.8}", spot)),
                source: Some(source),
                cost_bps: Some(bps),
            },
            None => FillReference::default(),
        },
        None => FillReference::default(),
    }
}

/// Reference spot at a fill's block time: the stored NOCK/USD price that stands for it
/// (the lookup the harvest ledger uses, within 60 minutes: the worker's own read first, a
/// backfilled candle as its open, #129). No stored price in that window means no
/// reference and no cost figure.
pub(crate) async fn reference_for_fill(db: &PgPool, effective_price_usd: &str, confirmed_at: DateTime<Utc>) -> FillReference {
    fill_reference(effective_price_usd, nock_price_near(db, confirmed_at).await)
}
