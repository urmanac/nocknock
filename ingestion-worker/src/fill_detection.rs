use crate::*;
use std::collections::HashSet;

// ============================================================
// Fill detection for the accumulation plan (issue #118)
// ============================================================
// Each ingestion cycle, while a plan is active, the worker reads the tracked wallet's
// incoming NOCK transfers back to the plan anchor, checks every transaction it has not
// seen before against its receipt with the record endpoint's own rules
// (fill_verify.rs), and remembers the verdict in `accumulation_candidates`.
//
// What this never does:
//   - record a buy. A detected swap is OFFERED. Only POST /v1/execute/record-accumulation
//     writes the ledger, and it verifies the receipt again.
//   - fail the cycle. When it cannot run it says so in the plan status, and recording
//     by transaction hash works as before.
//   - classify a receipt it could not read. That hash is left out and tried again.
//   - filter by size or price. It has no thresholds: it shows what the chain shows.
//
// Verified against live Base on 2026-11-15/16 (ONCHAIN_INTEGRATION.md §8).

/// Transfers asked for per `alchemy_getAssetTransfers` page.
pub(crate) const FILL_SCAN_PAGE_SIZE: u32 = 100;
/// Pages read per cycle before the check reports itself incomplete. One page is the
/// normal case: paging only happens when more than a page of NOCK transfers reached
/// the wallet since the plan anchor.
pub(crate) const FILL_SCAN_MAX_PAGES: usize = 10;
/// New transactions examined per cycle (one receipt each, plus one block header for a
/// purchase). The rest wait for the next cycle and the status says so.
pub(crate) const FILL_CLASSIFY_MAX_PER_CYCLE: usize = 25;

// ---------------- Transfer history (a hint, never trusted) ----------------

/// One incoming NOCK transfer as the transfer-history call reports it. Only the hash
/// is acted on, and only to fetch the receipt that is then verified.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TransferHint {
    /// Lowercase, 0x-prefixed.
    pub(crate) tx_hash: String,
    pub(crate) block_number: i64,
    pub(crate) block_time: DateTime<Utc>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TransferPage {
    pub(crate) transfers: Vec<TransferHint>,
    pub(crate) page_key: Option<String>,
}

/// `alchemy_getAssetTransfers` request for NOCK transfers into `wallet`, newest first,
/// with block timestamps. Pure.
pub(crate) fn asset_transfers_request(wallet: &str, page_key: Option<&str>) -> serde_json::Value {
    let mut params = json!({
        "fromBlock": "0x0",
        "toBlock": "latest",
        "toAddress": wallet,
        "contractAddresses": [NOCK_BASE],
        "category": ["erc20"],
        "withMetadata": true,
        "excludeZeroValue": true,
        "order": "desc",
        "maxCount": format!("0x{:x}", FILL_SCAN_PAGE_SIZE),
    });
    if let Some(key) = page_key {
        params["pageKey"] = json!(key);
    }
    json!({ "jsonrpc": "2.0", "id": 44, "method": "alchemy_getAssetTransfers", "params": [params] })
}

/// Parse one `alchemy_getAssetTransfers` response. A response that is not the expected
/// shape is an error, never an empty list: "nothing found" must mean the list was read.
/// Pure.
pub(crate) fn parse_asset_transfers(resp: &serde_json::Value) -> std::result::Result<TransferPage, String> {
    let result = resp.get("result").ok_or("response has no result")?;
    let transfers = result
        .get("transfers")
        .and_then(|t| t.as_array())
        .ok_or("result has no transfers array")?;
    let mut out = Vec::with_capacity(transfers.len());
    for (i, t) in transfers.iter().enumerate() {
        let hash = t
            .get("hash")
            .and_then(|h| h.as_str())
            .filter(|h| is_valid_tx_hash(h))
            .ok_or_else(|| format!("transfer {i} has no 32-byte transaction hash"))?;
        let block_number = t
            .get("blockNum")
            .and_then(|b| b.as_str())
            .and_then(|b| i64::from_str_radix(b.trim_start_matches("0x"), 16).ok())
            .ok_or_else(|| format!("transfer {i} has no block number"))?;
        let block_time = t
            .pointer("/metadata/blockTimestamp")
            .and_then(|ts| ts.as_str())
            .and_then(|ts| DateTime::parse_from_rfc3339(ts).ok())
            .map(|ts| ts.with_timezone(&Utc))
            .ok_or_else(|| format!("transfer {i} has no block timestamp"))?;
        let hash = hash.to_lowercase();
        out.push(TransferHint {
            tx_hash: if hash.starts_with("0x") { hash } else { format!("0x{hash}") },
            block_number,
            block_time,
        });
    }
    let page_key = result
        .get("pageKey")
        .and_then(|k| k.as_str())
        .filter(|k| !k.is_empty())
        .map(|k| k.to_string());
    Ok(TransferPage { transfers: out, page_key })
}

/// Folds transfer pages (newest first) into distinct transaction hashes back to the
/// plan anchor. Pure state: the I/O loop only feeds it pages.
#[derive(Debug, Clone)]
pub(crate) struct TransferScan {
    anchor: DateTime<Utc>,
    seen: HashSet<String>,
    /// Distinct transactions at or after the anchor, newest first.
    pub(crate) hints: Vec<TransferHint>,
    /// True once the history was read as far back as the anchor (a transfer older than
    /// the anchor was reached, or the history ended).
    pub(crate) complete: bool,
    /// Oldest transfer time read so far.
    pub(crate) oldest_seen: Option<DateTime<Utc>>,
    pub(crate) pages: usize,
}

impl TransferScan {
    pub(crate) fn new(anchor: DateTime<Utc>) -> Self {
        Self { anchor, seen: HashSet::new(), hints: Vec::new(), complete: false, oldest_seen: None, pages: 0 }
    }

    /// Take one page. Returns the key of the next page to read, or None when the scan
    /// is finished (the anchor was reached or the history ended).
    pub(crate) fn absorb(&mut self, page: TransferPage) -> Option<String> {
        self.pages += 1;
        let mut reached_anchor = false;
        for t in page.transfers {
            self.oldest_seen = Some(self.oldest_seen.map_or(t.block_time, |o| o.min(t.block_time)));
            if fill_is_before_anchor(t.block_time, self.anchor) {
                // Swaps from before the anchor are not offered. Keep reading this page
                // rather than trust its order, but do not ask for an older one.
                reached_anchor = true;
                continue;
            }
            if self.seen.insert(t.tx_hash.clone()) {
                self.hints.push(t);
            }
        }
        if reached_anchor || page.page_key.is_none() {
            self.complete = true;
            return None;
        }
        page.page_key
    }

    /// How far back the history was read: the anchor when complete, otherwise the
    /// oldest transfer seen (or the anchor's far side is simply unknown: None).
    pub(crate) fn checked_back_to(&self) -> Option<DateTime<Utc>> {
        if self.complete {
            Some(self.anchor)
        } else {
            self.oldest_seen
        }
    }
}

// ---------------- Classification ----------------

/// What one detected transaction turned out to be.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CandidateVerdict {
    /// The receipt or its block header could not be read. Nothing is stored: the hash
    /// is examined again next cycle, never marked "not a purchase".
    Unread,
    /// Read, and not a USDC→NOCK purchase by the wallet. `reason` is the record
    /// endpoint's rejection code.
    NotAPurchase { reason: &'static str },
    /// A purchase by the record endpoint's rules. Whether it is at or after the plan
    /// anchor is decided when offers are listed, because the anchor can move.
    Purchase(VerifiedFill),
}

/// Classify one transaction from what could be read. `receipt` None means the node did
/// not answer; `block_time` None means the header was not read. Pure, and built only on
/// `verify_fill_receipt` / `verify_fill`, the record endpoint's checks.
pub(crate) fn classify_candidate(
    receipt: Option<&serde_json::Value>,
    wallet: &str,
    block_time: Option<DateTime<Utc>>,
    anchor: DateTime<Utc>,
) -> CandidateVerdict {
    let Some(receipt) = receipt else {
        return CandidateVerdict::Unread;
    };
    match verify_fill_receipt(receipt, wallet) {
        Err(FillRejection::ReceiptNotFound) => return CandidateVerdict::Unread,
        Err(r) => return CandidateVerdict::NotAPurchase { reason: r.code() },
        Ok(_) => {}
    }
    let Some(block_time) = block_time else {
        return CandidateVerdict::Unread;
    };
    match verify_fill(receipt, wallet, block_time, anchor) {
        Ok(fill) | Err(FillRejection::BeforePlanStart { fill, .. }) => CandidateVerdict::Purchase(fill),
        Err(FillRejection::ReceiptNotFound) => CandidateVerdict::Unread,
        Err(r) => CandidateVerdict::NotAPurchase { reason: r.code() },
    }
}

// ---------------- Offers (pure) ----------------

/// A stored candidate row, as far as the offer list needs it.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct FillCandidate {
    pub(crate) tx_hash: String,
    /// offered | dismissed | not_a_purchase
    pub(crate) state: String,
    pub(crate) block_number: Option<i64>,
    pub(crate) confirmed_at: Option<DateTime<Utc>>,
    pub(crate) usdc_spent: Option<String>,
    pub(crate) nock_received: Option<String>,
    pub(crate) effective_price_usd: Option<String>,
    pub(crate) reference_spot_usd: Option<String>,
    pub(crate) reference_spot_source: Option<String>,
    pub(crate) execution_cost_bps: Option<i32>,
}

/// "early" when the swap landed before the open tranche came due (last recorded fill
/// plus the plan interval), otherwise "on_schedule". Before the first fill nothing is
/// early. Pure.
pub(crate) fn fill_timing(confirmed_at: DateTime<Utc>, tranche_due_at: Option<DateTime<Utc>>) -> &'static str {
    match tranche_due_at {
        Some(due) if confirmed_at < due => "early",
        _ => "on_schedule",
    }
}

/// The offer list and the dismissed list: a pure function of the candidate rows, the
/// recorded fills and the plan anchor.
///
/// Left out: a hash already in the ledger (any plan id, compared without case), rows
/// that are not purchases, anything before the anchor, incomplete rows, and a second
/// row for a hash already listed. Both lists are oldest first.
pub(crate) fn build_fill_offers(
    candidates: &[FillCandidate],
    recorded_hashes: &HashSet<String>,
    anchor: DateTime<Utc>,
    tranche_due_at: Option<DateTime<Utc>>,
) -> (Vec<FillOffer>, Vec<FillOffer>) {
    let recorded: HashSet<String> = recorded_hashes.iter().map(|h| h.to_lowercase()).collect();
    let mut listed: HashSet<String> = HashSet::new();
    let mut rows: Vec<(DateTime<Utc>, bool, FillOffer)> = Vec::new();
    for c in candidates {
        let dismissed = match c.state.as_str() {
            "offered" => false,
            "dismissed" => true,
            _ => continue,
        };
        let hash = c.tx_hash.to_lowercase();
        if recorded.contains(&hash) {
            continue;
        }
        let (Some(confirmed_at), Some(block_number), Some(usdc), Some(nock), Some(price)) = (
            c.confirmed_at,
            c.block_number,
            c.usdc_spent.as_ref(),
            c.nock_received.as_ref(),
            c.effective_price_usd.as_ref(),
        ) else {
            continue;
        };
        if fill_is_before_anchor(confirmed_at, anchor) {
            continue;
        }
        if !listed.insert(hash.clone()) {
            continue;
        }
        // A cost figure needs its reference: both or neither.
        let (reference_spot_usd, reference_spot_source, execution_cost_bps) =
            match (c.reference_spot_usd.as_ref(), c.execution_cost_bps) {
                (Some(spot), Some(bps)) => (Some(spot.clone()), c.reference_spot_source.clone(), Some(bps)),
                _ => (None, None, None),
            };
        rows.push((
            confirmed_at,
            dismissed,
            FillOffer {
                tx_hash: hash,
                confirmed_at: confirmed_at.to_rfc3339(),
                block_number,
                usdc_spent: usdc.clone(),
                nock_received: nock.clone(),
                effective_price_usd: price.clone(),
                timing: fill_timing(confirmed_at, tranche_due_at).to_string(),
                reference_spot_usd,
                reference_spot_source,
                execution_cost_bps,
            },
        ));
    }
    rows.sort_by(|a, b| a.0.cmp(&b.0).then_with(|| a.2.tx_hash.cmp(&b.2.tx_hash)));
    let (mut offers, mut dismissed_out) = (Vec::new(), Vec::new());
    for (_, dismissed, offer) in rows {
        if dismissed {
            dismissed_out.push(offer);
        } else {
            offers.push(offer);
        }
    }
    (offers, dismissed_out)
}

// ---------------- Status (pure) ----------------

impl FillDetection {
    pub(crate) fn not_run(detail: &str) -> Self {
        FillDetection { status: "not_run".to_string(), detail: Some(detail.to_string()), ..Default::default() }
    }

    pub(crate) fn unavailable(detail: &str) -> Self {
        FillDetection { status: "unavailable".to_string(), detail: Some(detail.to_string()), ..Default::default() }
    }
}

/// Whether this cycle looks for new swaps.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum DetectionGate {
    Run,
    /// Not run, with the reason. `list_known` keeps already-detected swaps listed
    /// (a finished plan may still have one waiting for an answer).
    Skip { reason: String, list_known: bool },
}

/// Decided from the config and the ledger, because the plan's status string does not
/// exist yet when detection runs. Uses `plan_directive` itself for the three states
/// that do not depend on live balances, so the two cannot disagree. Pure.
pub(crate) fn detection_gate(
    cfg: &AccumulationPlanConfig,
    fills: &PlanFills,
    ledger_unavailable: bool,
    now: DateTime<Utc>,
) -> DetectionGate {
    if ledger_unavailable {
        return DetectionGate::Skip {
            reason: "the accumulation ledger could not be read this cycle, so recorded buys cannot be told from new ones".to_string(),
            list_known: false,
        };
    }
    match plan_directive(cfg, fills, &PlanGuards::default(), false, now) {
        PlanDirective::Invalid { .. } => DetectionGate::Skip {
            reason: "the plan configuration is invalid".to_string(),
            list_known: false,
        },
        PlanDirective::Pending { starts_at } => DetectionGate::Skip {
            reason: format!("the plan has not started (it starts at {})", starts_at.to_rfc3339()),
            list_known: false,
        },
        PlanDirective::Complete => DetectionGate::Skip {
            reason: "the plan's budget is spent, so no further buys are looked for".to_string(),
            list_known: true,
        },
        _ => DetectionGate::Run,
    }
}

/// What one cycle's scan managed to do.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ScanSummary {
    /// The history was read back to the plan anchor.
    pub(crate) complete: bool,
    pub(crate) checked_back_to: Option<DateTime<Utc>>,
    /// Transactions whose receipt or block header could not be read this cycle.
    pub(crate) unread: usize,
    /// New transactions left for a later cycle by the per-cycle limit.
    pub(crate) deferred: usize,
}

/// Status and detail for a scan that read the transfer history. `ok` only when the
/// history was read back to the anchor and every new transaction was examined;
/// otherwise `partial`, saying exactly what is missing. Pure.
pub(crate) fn scan_status(summary: &ScanSummary, anchor: DateTime<Utc>) -> (&'static str, Option<String>) {
    let mut gaps: Vec<String> = Vec::new();
    if summary.unread > 0 {
        gaps.push(format!(
            "{} transaction(s) could not be read and will be tried again next cycle",
            summary.unread
        ));
    }
    if summary.deferred > 0 {
        gaps.push(format!(
            "{} more transfer(s) are still to be examined (at most {} per cycle)",
            summary.deferred, FILL_CLASSIFY_MAX_PER_CYCLE
        ));
    }
    if !summary.complete {
        gaps.push(match summary.checked_back_to {
            Some(t) => format!(
                "the transfer history was read back to {}, not as far as the plan anchor {}",
                t.to_rfc3339(),
                anchor.to_rfc3339()
            ),
            None => format!("the transfer history was not read as far as the plan anchor {}", anchor.to_rfc3339()),
        });
    }
    if gaps.is_empty() {
        ("ok", None)
    } else {
        ("partial", Some(gaps.join("; ")))
    }
}

// ---------------- Advice text (pure) ----------------

fn short_hash(h: &str) -> String {
    if h.len() > 18 {
        format!("{}…{}", &h[..10], &h[h.len() - 6..])
    } else {
        h.to_string()
    }
}

/// The sentence that leads the plan's advice while a detected swap waits for an
/// answer. It changes the details only: the action title stays as it is, so the
/// directive-transition notifier (which compares titles) does not fire for it. Pure.
pub(crate) fn open_offer_notice(ps: &AccumulationPlanStatus) -> Option<String> {
    let offers = &ps.fill_detection.as_ref()?.offers;
    let first = offers.first()?;
    let before_buying = if ps.status == "tranche_due" { " before buying again" } else { "" };
    Some(if offers.len() == 1 {
        // Rounded for reading; the snapshot's offer carries the exact amounts.
        let two_dp = |v: &str| v.parse::<f64>().map(|n| format!("{n:.2}")).unwrap_or_else(|_| v.to_string());
        format!(
            "A swap that looks like a plan buy was found on-chain and is not counted yet: ${} USDC for {} NOCK at {} ({}). In the dashboard, confirm it or mark it as not part of the plan{}.",
            two_dp(&first.usdc_spent),
            two_dp(&first.nock_received),
            first.confirmed_at,
            short_hash(&first.tx_hash),
            before_buying
        )
    } else {
        let total: Option<f64> = offers.iter().map(|o| o.usdc_spent.parse::<f64>().ok()).sum();
        format!(
            "{} swaps that look like plan buys were found on-chain and are not counted yet{}. In the dashboard, confirm each one or mark it as not part of the plan{}.",
            offers.len(),
            total.map(|t| format!(" (${t:.2} USDC in total)")).unwrap_or_default(),
            before_buying
        )
    })
}

/// What to do after buying, given whether detection is working this cycle. Without a
/// detection result (or with one that did not run) it is the pre-v0.6.0 sentence. Pure.
pub(crate) fn after_buying_sentence(ps: &AccumulationPlanStatus) -> &'static str {
    match ps.fill_detection.as_ref().map(|d| d.status.as_str()) {
        Some("ok") | Some("partial") => {
            "After buying, NockNock looks for the swap on-chain and offers it for confirmation; the plan advances once you confirm it (or record the transaction hash yourself)."
        }
        Some("unavailable") => {
            "Swap detection could not check this cycle, so after buying record the transaction hash so the plan advances."
        }
        _ => "After buying, record the transaction hash so the plan advances.",
    }
}

// ---------------- Database ----------------

fn row_to_candidate(r: &sqlx::postgres::PgRow) -> FillCandidate {
    FillCandidate {
        tx_hash: r.try_get::<String, _>("tx_hash").unwrap_or_default(),
        state: r.try_get::<String, _>("state").unwrap_or_default(),
        block_number: r.try_get::<Option<i64>, _>("block_number").ok().flatten(),
        confirmed_at: r.try_get::<Option<DateTime<Utc>>, _>("confirmed_at").ok().flatten(),
        usdc_spent: r.try_get::<Option<String>, _>("usdc_spent").ok().flatten(),
        nock_received: r.try_get::<Option<String>, _>("nock_received").ok().flatten(),
        effective_price_usd: r.try_get::<Option<String>, _>("effective_price_usd").ok().flatten(),
        reference_spot_usd: r.try_get::<Option<String>, _>("reference_spot_usd").ok().flatten(),
        reference_spot_source: r.try_get::<Option<String>, _>("reference_spot_source").ok().flatten(),
        execution_cost_bps: r.try_get::<Option<i32>, _>("execution_cost_bps").ok().flatten(),
    }
}

/// Every hash the wallet's candidates table already holds (any state).
pub(crate) async fn known_candidate_hashes(db: &PgPool, wallet: &str) -> std::result::Result<HashSet<String>, sqlx::Error> {
    let rows = sqlx::query("SELECT tx_hash FROM accumulation_candidates WHERE wallet = $1")
        .bind(wallet)
        .fetch_all(db)
        .await?;
    Ok(rows.iter().filter_map(|r| r.try_get::<String, _>("tx_hash").ok()).map(|h| h.to_lowercase()).collect())
}

/// Every recorded fill's hash, lowercase, across all plan ids: a buy counted once for
/// any plan is never offered again.
pub(crate) async fn recorded_fill_hashes(db: &PgPool) -> std::result::Result<HashSet<String>, sqlx::Error> {
    let rows = sqlx::query("SELECT lower(tx_hash) AS tx_hash FROM accumulation_events").fetch_all(db).await?;
    Ok(rows.iter().filter_map(|r| r.try_get::<String, _>("tx_hash").ok()).collect())
}

pub(crate) async fn offered_and_dismissed(db: &PgPool, wallet: &str) -> std::result::Result<Vec<FillCandidate>, sqlx::Error> {
    let rows = sqlx::query(
        "SELECT tx_hash, state, block_number, confirmed_at, usdc_spent, nock_received, effective_price_usd, \
                reference_spot_usd, reference_spot_source, execution_cost_bps \
         FROM accumulation_candidates WHERE wallet = $1 AND state IN ('offered', 'dismissed') \
         ORDER BY confirmed_at ASC, tx_hash ASC",
    )
    .bind(wallet)
    .fetch_all(db)
    .await?;
    Ok(rows.iter().map(row_to_candidate).collect())
}

pub(crate) async fn store_verdict(
    db: &PgPool,
    wallet: &str,
    tx_hash: &str,
    verdict: &CandidateVerdict,
) -> std::result::Result<(), sqlx::Error> {
    match verdict {
        CandidateVerdict::Unread => Ok(()),
        CandidateVerdict::NotAPurchase { reason } => sqlx::query(
            "INSERT INTO accumulation_candidates (wallet, tx_hash, state, reason) VALUES ($1, $2, 'not_a_purchase', $3) \
             ON CONFLICT (wallet, tx_hash) DO NOTHING",
        )
        .bind(wallet)
        .bind(tx_hash)
        .bind(reason)
        .execute(db)
        .await
        .map(|_| ()),
        CandidateVerdict::Purchase(fill) => {
            let reference = reference_for_fill(db, &fill.swap.effective_price_usd, fill.confirmed_at).await;
            sqlx::query(
                "INSERT INTO accumulation_candidates \
                    (wallet, tx_hash, state, block_number, confirmed_at, usdc_spent_raw, nock_received_raw, usdc_spent, \
                     nock_received, effective_price_usd, reference_spot_usd, reference_spot_source, execution_cost_bps) \
                 VALUES ($1, $2, 'offered', $3, $4, $5, $6, $7, $8, $9, $10, $11, $12) \
                 ON CONFLICT (wallet, tx_hash) DO NOTHING",
            )
            .bind(wallet)
            .bind(tx_hash)
            .bind(fill.swap.block_number)
            .bind(fill.confirmed_at)
            .bind(fill.swap.usdc_raw.to_string())
            .bind(fill.swap.nock_raw.to_string())
            .bind(&fill.swap.usdc_spent)
            .bind(&fill.swap.nock_received)
            .bind(&fill.swap.effective_price_usd)
            .bind(&reference.spot_usd)
            .bind(&reference.source)
            .bind(reference.cost_bps)
            .execute(db)
            .await
            .map(|_| ())
        }
    }
}

/// Fill in the reference spot and cost for rows that have none yet: fills recorded
/// before v0.6.0, and swaps whose block time had no stored price near it when they
/// were first seen (a backfilled candle can supply one later). A row is written once
/// and never revised. Failures are logged and leave the row "unavailable".
pub(crate) async fn backfill_reference_spots(db: &PgPool, wallet: &str) {
    let fills = sqlx::query(
        "SELECT tx_hash, effective_price_usd, confirmed_at FROM accumulation_events \
         WHERE reference_spot_usd IS NULL ORDER BY confirmed_at DESC LIMIT 20",
    )
    .fetch_all(db)
    .await
    .unwrap_or_default();
    let candidates = sqlx::query(
        "SELECT tx_hash, effective_price_usd, confirmed_at FROM accumulation_candidates \
         WHERE wallet = $1 AND state IN ('offered', 'dismissed') AND reference_spot_usd IS NULL \
           AND confirmed_at IS NOT NULL AND effective_price_usd IS NOT NULL \
         ORDER BY confirmed_at DESC LIMIT 20",
    )
    .bind(wallet)
    .fetch_all(db)
    .await
    .unwrap_or_default();
    for (rows, table) in [(&fills, "accumulation_events"), (&candidates, "accumulation_candidates")] {
        for r in rows.iter() {
            let (Ok(tx_hash), Ok(price), Ok(at)) = (
                r.try_get::<String, _>("tx_hash"),
                r.try_get::<String, _>("effective_price_usd"),
                r.try_get::<DateTime<Utc>, _>("confirmed_at"),
            ) else {
                continue;
            };
            let reference = reference_for_fill(db, &price, at).await;
            if reference.spot_usd.is_none() {
                continue;
            }
            // `table` is one of two literals above, never input.
            let sql = format!(
                "UPDATE {table} SET reference_spot_usd = $1, reference_spot_source = $2, execution_cost_bps = $3 \
                 WHERE tx_hash = $4 AND reference_spot_usd IS NULL"
            );
            if let Err(e) = sqlx::query(&sql)
                .bind(&reference.spot_usd)
                .bind(&reference.source)
                .bind(reference.cost_bps)
                .bind(&tx_hash)
                .execute(db)
                .await
            {
                tracing::warn!("Reference spot backfill for {tx_hash} failed: {e}");
            }
        }
    }
}

pub(crate) async fn last_successful_check(db: &PgPool, plan_id: &str) -> Option<DateTime<Utc>> {
    sqlx::query("SELECT fills_checked_at FROM accumulation_plans WHERE plan_id = $1")
        .bind(plan_id)
        .fetch_optional(db)
        .await
        .ok()
        .flatten()
        .and_then(|r| r.try_get::<Option<DateTime<Utc>>, _>("fills_checked_at").ok().flatten())
}

// ---------------- One cycle's detection ----------------

/// Read the transfer history back to the anchor and classify every transaction not
/// seen before. Err is "could not check": the message says why.
async fn scan_for_fills(
    state: &Arc<AppState>,
    wallet: &str,
    anchor: DateTime<Utc>,
) -> std::result::Result<ScanSummary, String> {
    let alchemy_key = state.config.alchemy_api_keys.first().cloned().unwrap_or_default();
    if alchemy_key.is_empty() || alchemy_key == "YOUR_ALCHEMY_API_KEY" {
        return Err("no Alchemy RPC key is configured".to_string());
    }
    let alchemy_url = format!("https://base-mainnet.g.alchemy.com/v2/{}", alchemy_key);

    let mut scan = TransferScan::new(anchor);
    let mut page_key: Option<String> = None;
    loop {
        let payload = asset_transfers_request(wallet, page_key.as_deref());
        let resp = post_rpc_with_retry(&state.http_client, &alchemy_url, &payload, 2)
            .await
            .ok_or("the wallet's transfer history could not be read (alchemy_getAssetTransfers gave no answer)")?;
        let page = parse_asset_transfers(&resp)
            .map_err(|e| format!("the transfer history response was not understood ({e})"))?;
        page_key = scan.absorb(page);
        if page_key.is_none() || scan.pages >= FILL_SCAN_MAX_PAGES {
            break;
        }
    }

    let known = known_candidate_hashes(&state.db, wallet)
        .await
        .map_err(|e| format!("the candidates table could not be read ({e})"))?;
    let recorded = recorded_fill_hashes(&state.db)
        .await
        .map_err(|e| format!("the accumulation ledger could not be read ({e})"))?;
    let fresh: Vec<&TransferHint> = scan
        .hints
        .iter()
        .filter(|h| !known.contains(&h.tx_hash) && !recorded.contains(&h.tx_hash))
        .collect();
    let deferred = fresh.len().saturating_sub(FILL_CLASSIFY_MAX_PER_CYCLE);
    let mut unread = 0usize;
    for hint in fresh.into_iter().take(FILL_CLASSIFY_MAX_PER_CYCLE) {
        let receipt_payload = json!({
            "jsonrpc": "2.0", "method": "eth_getTransactionReceipt", "params": [hint.tx_hash], "id": 45
        });
        let receipt: Option<serde_json::Value> =
            post_rpc_with_retry(&state.http_client, &alchemy_url, &receipt_payload, 2)
                .await
                .map(|res| res.get("result").cloned().unwrap_or(serde_json::Value::Null));
        // A block header is read only for a transaction whose receipt is a purchase.
        let block_time = match receipt.as_ref().map(|r| verify_fill_receipt(r, wallet)) {
            Some(Ok(swap)) => fetch_block_time(&state.http_client, &alchemy_url, swap.block_number).await,
            _ => None,
        };
        let verdict = classify_candidate(receipt.as_ref(), wallet, block_time, anchor);
        if verdict == CandidateVerdict::Unread {
            unread += 1;
            continue;
        }
        if let Err(e) = store_verdict(&state.db, wallet, &hint.tx_hash, &verdict).await {
            // Not stored means not classified: it is examined again next cycle.
            tracing::warn!("Could not store the verdict for candidate {}: {e}", hint.tx_hash);
            unread += 1;
        }
    }
    Ok(ScanSummary { complete: scan.complete, checked_back_to: scan.checked_back_to(), unread, deferred })
}

/// One cycle's fill detection. None when no plan is configured. It never returns an
/// error and never fails the cycle: a check that could not run is reported in the
/// result (`unavailable` / `not_run`) with its reason.
pub(crate) async fn detect_fills(state: &Arc<AppState>, plan: &PlanContext) -> Option<FillDetection> {
    let cfg = plan.cfg.as_ref()?;
    let wallet = state.config.tracked_wallets.first().map(|w| w.to_lowercase());

    // Cost over spot for fills recorded before this field existed. Database only, and
    // independent of whether the transfer history is read.
    if let (Some(w), false) = (wallet.as_ref(), plan.ledger_unavailable) {
        backfill_reference_spots(&state.db, w).await;
    }

    if !state.config.fill_detection.enabled {
        return Some(FillDetection::not_run("swap detection is turned off (fill_detection.enabled is false)"));
    }
    let gate = detection_gate(cfg, &plan.fills, plan.ledger_unavailable, plan.now);
    if let DetectionGate::Skip { reason, list_known: false } = &gate {
        return Some(FillDetection::not_run(reason));
    }
    let Some(wallet) = wallet else {
        return Some(FillDetection::unavailable("no tracked wallet is configured"));
    };
    let anchor = match plan_registered_at(&state.db, &cfg.id).await {
        Ok(registered_at) => plan_anchor(cfg.starts_at, registered_at),
        Err(e) => {
            tracing::warn!("Fill detection: plan registry unavailable: {e}");
            return Some(FillDetection::unavailable("the plan registry could not be read, so the plan anchor is unknown"));
        }
    };

    let scan = match &gate {
        DetectionGate::Run => Some(scan_for_fills(state, &wallet, anchor).await),
        DetectionGate::Skip { .. } => None,
    };
    if let Some(Ok(_)) = &scan {
        if let Err(e) = sqlx::query("UPDATE accumulation_plans SET fills_checked_at = NOW() WHERE plan_id = $1")
            .bind(&cfg.id)
            .execute(&state.db)
            .await
        {
            tracing::warn!("Fill detection: could not store the check time: {e}");
        }
    }
    if let Some(Err(e)) = &scan {
        tracing::warn!("Fill detection could not check this cycle: {e}");
    }

    // The lists come from the table, so swaps found by an earlier check stay listed
    // when this cycle's check could not run.
    let lists = match (offered_and_dismissed(&state.db, &wallet).await, recorded_fill_hashes(&state.db).await) {
        (Ok(candidates), Ok(recorded)) => {
            Ok(build_fill_offers(&candidates, &recorded, anchor, next_tranche_due_at(cfg, &plan.fills)))
        }
        (Err(e), _) | (_, Err(e)) => Err(e),
    };
    let (offers, dismissed) = match lists {
        Ok(l) => l,
        Err(e) => {
            tracing::warn!("Fill detection: offers could not be listed: {e}");
            return Some(FillDetection::unavailable(
                "detected swaps could not be listed (the candidates table or the ledger was unreadable)",
            ));
        }
    };
    let last_checked_at = last_successful_check(&state.db, &cfg.id).await.map(|t| t.to_rfc3339());

    let (status, detail, checked_back_to) = match (&gate, &scan) {
        (DetectionGate::Skip { reason, .. }, _) => ("not_run", Some(reason.clone()), None),
        (_, Some(Ok(summary))) => {
            let (status, detail) = scan_status(summary, anchor);
            (status, detail, summary.checked_back_to.map(|t| t.to_rfc3339()))
        }
        (_, Some(Err(e))) => ("unavailable", Some(e.clone()), None),
        (DetectionGate::Run, None) => ("unavailable", Some("the check did not run".to_string()), None),
    };
    Some(FillDetection {
        status: status.to_string(),
        detail,
        checked_back_to,
        last_checked_at,
        offers,
        dismissed,
    })
}

// ---------------- REST: dismiss / restore / list ----------------

#[derive(Deserialize)]
pub(crate) struct CandidateActionRequest {
    tx_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum CandidateAction {
    Dismiss,
    Restore,
}

/// The state a dismiss or restore leads to, and whether anything changes. Both actions
/// are idempotent. They move a detected purchase between "offered" and "dismissed" and
/// nothing else: they cannot create a candidate, count a buy, or touch the ledger. Pure.
pub(crate) fn candidate_transition(
    action: CandidateAction,
    current_state: Option<&str>,
    recorded: bool,
) -> std::result::Result<(&'static str, bool), (StatusCode, &'static str, &'static str)> {
    let Some(current) = current_state else {
        return Err((
            StatusCode::NOT_FOUND,
            "CANDIDATE_NOT_FOUND",
            "NockNock has not detected this transaction as a swap, so there is nothing to dismiss or restore. To count a buy, record its transaction hash.",
        ));
    };
    if recorded {
        return Err((
            StatusCode::CONFLICT,
            "ALREADY_RECORDED",
            "This buy is already in the plan ledger. A recorded buy is not an offer and cannot be dismissed or restored.",
        ));
    }
    match (action, current) {
        (CandidateAction::Dismiss, "offered") => Ok(("dismissed", true)),
        (CandidateAction::Dismiss, "dismissed") => Ok(("dismissed", false)),
        (CandidateAction::Restore, "dismissed") => Ok(("offered", true)),
        (CandidateAction::Restore, "offered") => Ok(("offered", false)),
        _ => Err((
            StatusCode::CONFLICT,
            "NOT_AN_OFFER",
            "This transaction was checked and is not a USDC to NOCK purchase by the tracked wallet. It was never offered.",
        )),
    }
}

async fn candidate_action(state: Arc<AppState>, req: CandidateActionRequest, action: CandidateAction) -> Response {
    let err = |code: StatusCode, error: &str, detail: &str| {
        (code, Json(json!({ "error": error, "detail": detail }))).into_response()
    };
    if !is_valid_tx_hash(&req.tx_hash) {
        return err(StatusCode::UNPROCESSABLE_ENTITY, "INVALID_TX_HASH", "tx_hash must be a 32-byte hex string");
    }
    let tx_hash = if req.tx_hash.starts_with("0x") {
        req.tx_hash.to_lowercase()
    } else {
        format!("0x{}", req.tx_hash.to_lowercase())
    };
    let Some(wallet) = state.config.tracked_wallets.first().map(|w| w.to_lowercase()) else {
        return err(StatusCode::SERVICE_UNAVAILABLE, "NO_TRACKED_WALLET", "No tracked wallet configured");
    };
    let db_err = |e: sqlx::Error| {
        tracing::error!("Candidate {action:?} failed: {e}");
        err(StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", "Could not read or update the candidates table; nothing changed")
    };
    let current: Option<String> = match sqlx::query("SELECT state FROM accumulation_candidates WHERE wallet = $1 AND tx_hash = $2")
        .bind(&wallet)
        .bind(&tx_hash)
        .fetch_optional(&state.db)
        .await
    {
        Ok(row) => row.and_then(|r| r.try_get::<String, _>("state").ok()),
        Err(e) => return db_err(e),
    };
    let recorded = match sqlx::query("SELECT 1 AS one FROM accumulation_events WHERE lower(tx_hash) = $1 LIMIT 1")
        .bind(&tx_hash)
        .fetch_optional(&state.db)
        .await
    {
        Ok(row) => row.is_some(),
        Err(e) => return db_err(e),
    };
    let (new_state, changed) = match candidate_transition(action, current.as_deref(), recorded) {
        Ok(t) => t,
        Err((code, error, detail)) => return err(code, error, detail),
    };
    if changed {
        if let Err(e) = sqlx::query(
            "UPDATE accumulation_candidates SET state = $3, updated_at = NOW() WHERE wallet = $1 AND tx_hash = $2",
        )
        .bind(&wallet)
        .bind(&tx_hash)
        .bind(new_state)
        .execute(&state.db)
        .await
        {
            return db_err(e);
        }
    }
    Json(json!({
        "tx_hash": tx_hash,
        "state": new_state,
        "changed": changed,
        "note": "The plan ledger is untouched. The snapshot's offer list updates on the next ingestion cycle (POST /v1/ingest to refresh now)."
    }))
    .into_response()
}

/// POST /v1/accumulation/candidates/dismiss — mark a detected swap "not part of the
/// plan". It stays listed and can be restored; nothing is written to the ledger.
pub(crate) async fn dismiss_candidate_handler(
    Extension(state): Extension<Arc<AppState>>,
    Json(req): Json<CandidateActionRequest>,
) -> impl IntoResponse {
    candidate_action(state, req, CandidateAction::Dismiss).await
}

/// POST /v1/accumulation/candidates/restore — put a dismissed swap back on offer.
pub(crate) async fn restore_candidate_handler(
    Extension(state): Extension<Arc<AppState>>,
    Json(req): Json<CandidateActionRequest>,
) -> impl IntoResponse {
    candidate_action(state, req, CandidateAction::Restore).await
}

/// GET /v1/accumulation/candidates — every transaction fill detection has examined for
/// the tracked wallet, newest first, with its verdict and whether it is in the ledger.
pub(crate) async fn accumulation_candidates_handler(Extension(state): Extension<Arc<AppState>>) -> impl IntoResponse {
    let wallet = state.config.tracked_wallets.first().map(|w| w.to_lowercase()).unwrap_or_default();
    let rows = sqlx::query(
        "SELECT c.tx_hash, c.state, c.reason, c.block_number, c.confirmed_at, c.usdc_spent, c.nock_received, \
                c.effective_price_usd, c.reference_spot_usd, c.reference_spot_source, c.execution_cost_bps, \
                c.first_seen_at, \
                EXISTS (SELECT 1 FROM accumulation_events e WHERE lower(e.tx_hash) = c.tx_hash) AS recorded \
         FROM accumulation_candidates c WHERE c.wallet = $1 \
         ORDER BY c.confirmed_at DESC NULLS LAST, c.first_seen_at DESC",
    )
    .bind(&wallet)
    .fetch_all(&state.db)
    .await;
    match rows {
        Ok(rows) => {
            let candidates: Vec<serde_json::Value> = rows
                .iter()
                .map(|r| {
                    let c = row_to_candidate(r);
                    json!({
                        "tx_hash": c.tx_hash,
                        "state": c.state,
                        "reason": r.try_get::<Option<String>, _>("reason").ok().flatten(),
                        "recorded": r.try_get::<bool, _>("recorded").unwrap_or(false),
                        "block_number": c.block_number,
                        "confirmed_at": c.confirmed_at.map(|t| t.to_rfc3339()),
                        "usdc_spent": c.usdc_spent,
                        "nock_received": c.nock_received,
                        "effective_price_usd": c.effective_price_usd,
                        "reference_spot_usd": c.reference_spot_usd,
                        "reference_spot_source": c.reference_spot_source,
                        "execution_cost_bps": c.execution_cost_bps,
                        "first_seen_at": r
                            .try_get::<DateTime<Utc>, _>("first_seen_at")
                            .map(|t| t.to_rfc3339())
                            .ok(),
                    })
                })
                .collect();
            Json(json!({ "candidates": candidates })).into_response()
        }
        Err(e) => {
            tracing::error!("Accumulation candidates query failed: {e}");
            (
                StatusCode::INTERNAL_SERVER_ERROR,
                Json(json!({ "error": "DB_ERROR", "detail": "Could not read the candidates table" })),
            )
                .into_response()
        }
    }
}
