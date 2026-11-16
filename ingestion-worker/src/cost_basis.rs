//! NOCK cost basis (issue #98): what the NOCK cost, against what it is worth at spot.
//!
//! Two layers, kept apart on purpose:
//!
//! 1. **Plan buys.** The average cost of the receipt-verified buys in the accumulation
//!    ledger (`accumulation_events`). Exact, and it needs nothing from the operator.
//! 2. **Whole stack.** Only when the operator enters an opening basis in config
//!    (`cost_basis.opening`): the blended average of that opening basis and the verified
//!    buys confirmed after it. NockNock cannot derive the stack's history from the chain
//!    (it includes sells, a liquidation and LP activity) and never invents it: unset is
//!    "not configured", a half-filled block is a configuration error.
//!
//! Everything here reads. Nothing sizes a buy, and no text urges one.
use crate::*;

/// A stored spot may stand for the price "at" a fill when it is at most this far from the
/// fill's block time (the harvest ledger's rule). Further than that there is no match.
pub(crate) const COST_BASIS_SPOT_MATCH_WINDOW_MINUTES: i64 = 60;
/// Upper bound on spot points in one series response.
pub(crate) const COST_BASIS_SERIES_MAX_POINTS: i64 = 600;
/// Longest range the series endpoint serves.
pub(crate) const COST_BASIS_SERIES_MAX_DAYS: i64 = 365;
/// Range when there is nothing to anchor it to (no fill, no opening basis).
pub(crate) const COST_BASIS_SERIES_DEFAULT_DAYS: i64 = 30;

/// One receipt-verified buy as the cost basis reads it. The amounts are the ledger's own
/// decimal strings (kept for display) and their parsed values.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BasisFill {
    pub(crate) plan_id: String,
    pub(crate) tx_hash: String,
    pub(crate) block_number: i64,
    pub(crate) confirmed_at: DateTime<Utc>,
    pub(crate) usdc_spent: String,
    pub(crate) nock_received: String,
    pub(crate) usdc: f64,
    pub(crate) nock: f64,
}

impl BasisFill {
    /// Build a fill from ledger text. A row whose amounts do not parse or are not
    /// positive is an error for the whole read: skipping it would understate the cost.
    pub(crate) fn parse(
        plan_id: &str,
        tx_hash: &str,
        block_number: i64,
        confirmed_at: DateTime<Utc>,
        usdc_spent: &str,
        nock_received: &str,
    ) -> std::result::Result<Self, String> {
        let amount = |label: &str, text: &str| -> std::result::Result<f64, String> {
            match text.trim().parse::<f64>() {
                Ok(v) if v.is_finite() && v > 0.0 => Ok(v),
                _ => Err(format!("accumulation ledger row {tx_hash} has an unreadable {label} (\"{text}\")")),
            }
        };
        Ok(Self {
            plan_id: plan_id.to_string(),
            tx_hash: tx_hash.to_string(),
            block_number,
            confirmed_at,
            usdc: amount("usdc_spent", usdc_spent)?,
            nock: amount("nock_received", nock_received)?,
            usdc_spent: usdc_spent.trim().to_string(),
            nock_received: nock_received.trim().to_string(),
        })
    }
}

/// A validated opening basis: the operator's own figures, never NockNock's.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct OpeningBasis {
    pub(crate) nock: f64,
    pub(crate) cost_usd: f64,
    pub(crate) as_of: DateTime<Utc>,
    pub(crate) source: String,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum OpeningBasisState {
    /// No `cost_basis.opening` in config: there is no whole-stack basis.
    NotConfigured,
    /// The block is present but cannot be used; the text says what to fix.
    Invalid(String),
    Set(OpeningBasis),
}

/// A config scalar as a number. Numeric strings count (YAML quoting is not an error);
/// anything else, including a placeholder word, does not.
fn config_number(v: &serde_json::Value) -> Option<f64> {
    match v {
        serde_json::Value::Number(n) => n.as_f64(),
        serde_json::Value::String(s) => s.trim().parse::<f64>().ok(),
        _ => None,
    }
    .filter(|x| x.is_finite())
}

/// A config scalar as text; None for null, a list or a mapping.
fn config_text(v: &serde_json::Value) -> Option<String> {
    match v {
        serde_json::Value::String(s) => Some(s.trim().to_string()),
        serde_json::Value::Number(n) => Some(n.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        _ => None,
    }
}

/// Check the operator's opening basis. Pure: the clock is an argument.
///
/// Every field is required once any is present, and every problem is listed, so a
/// half-filled block is reported whole instead of being completed with a default:
/// - `nock_amount` and `cost_usd`: numbers greater than zero;
/// - `as_of`: a full timestamp with a UTC offset, not in the future. A date alone is
///   refused, because a buy made that day might or might not be inside the figures;
/// - `source`: non-empty, because no figure is shown without its stated source.
pub(crate) fn resolve_opening_basis(cfg: Option<&OpeningBasisConfig>, now: DateTime<Utc>) -> OpeningBasisState {
    let Some(cfg) = cfg else { return OpeningBasisState::NotConfigured };
    let present = |v: &Option<serde_json::Value>| v.as_ref().filter(|x| !x.is_null()).cloned();
    let (nock_v, cost_v, as_of_v, source_v) =
        (present(&cfg.nock_amount), present(&cfg.cost_usd), present(&cfg.as_of), present(&cfg.source));
    if nock_v.is_none() && cost_v.is_none() && as_of_v.is_none() && source_v.is_none() {
        return OpeningBasisState::NotConfigured;
    }
    let mut problems: Vec<String> = Vec::new();
    let mut positive = |key: &str, what: &str, v: &Option<serde_json::Value>| -> Option<f64> {
        match v {
            None => {
                problems.push(format!("{key} is missing ({what})"));
                None
            }
            Some(raw) => match config_number(raw) {
                Some(x) if x > 0.0 => Some(x),
                Some(x) => {
                    problems.push(format!("{key} must be greater than zero (it is {x})"));
                    None
                }
                None => {
                    problems.push(format!("{key} must be a number (it is {raw})"));
                    None
                }
            },
        }
    };
    let nock = positive("nock_amount", "the NOCK you held at as_of", &nock_v);
    let cost = positive("cost_usd", "what that NOCK cost you in total, in USD", &cost_v);
    let as_of = match as_of_v.as_ref().and_then(config_text).filter(|s| !s.is_empty()) {
        None => {
            problems.push("as_of is missing (the instant you read the figures)".to_string());
            None
        }
        Some(text) => match DateTime::parse_from_rfc3339(&text) {
            Ok(t) if t.with_timezone(&Utc) > now => {
                problems.push(format!("as_of {text} is in the future"));
                None
            }
            Ok(t) => Some(t.with_timezone(&Utc)),
            Err(_) => {
                problems.push(format!(
                    "as_of \"{text}\" must be a full timestamp with a UTC offset, such as 2026-01-31T18:00:00Z \
                     (a date alone does not say whether a buy made that day is already inside the figures)"
                ));
                None
            }
        },
    };
    let source = match source_v.as_ref().and_then(config_text).filter(|s| !s.is_empty()) {
        None => {
            problems.push("source is missing (say where the figures came from)".to_string());
            None
        }
        some => some,
    };
    match (nock, cost, as_of, source) {
        (Some(nock), Some(cost_usd), Some(as_of), Some(source)) if problems.is_empty() => {
            OpeningBasisState::Set(OpeningBasis { nock, cost_usd, as_of, source })
        }
        _ => OpeningBasisState::Invalid(format!(
            "cost_basis.opening cannot be used: {}. Fix it or remove the block; until then only plan buys are reported.",
            problems.join("; ")
        )),
    }
}

/// Running totals at one point of the ledger.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(crate) struct Totals {
    pub(crate) usdc: f64,
    pub(crate) nock: f64,
}

impl Totals {
    fn add(&mut self, f: &BasisFill) {
        self.usdc += f.usdc;
        self.nock += f.nock;
    }
    /// Σ USDC / Σ NOCK; None before anything was bought.
    pub(crate) fn avg(&self) -> Option<f64> {
        (self.nock > 0.0).then(|| self.usdc / self.nock)
    }
}

/// Is this fill added on top of the opening basis? Only when it was confirmed strictly
/// after the as-of instant; anything at or before it is taken to be inside the figures.
pub(crate) fn counts_after_opening(fill: &BasisFill, opening: &OpeningBasis) -> bool {
    fill.confirmed_at > opening.as_of
}

fn usd(v: f64) -> String {
    format!("{:.2}", v)
}
fn nock_units(v: f64) -> String {
    format!("{:.4}", v)
}
fn price(v: f64) -> String {
    format!("{:.6}", v)
}
/// A NOCK amount known only to the nearest whole NOCK.
fn whole_nock(v: f64) -> String {
    format!("{:.0}", v)
}

/// What `nock` is worth at `spot`, its signed difference from `cost`, and that difference
/// as a percentage of the cost. All None without a price.
fn at_spot(nock: f64, cost: f64, spot: Option<f64>) -> (Option<String>, Option<String>, Option<String>) {
    let Some(spot) = spot.filter(|s| s.is_finite() && *s > 0.0) else { return (None, None, None) };
    let value = nock * spot;
    let pct = (cost > 0.0).then(|| format!("{:.2}", (value / cost - 1.0) * 100.0));
    (Some(usd(value)), Some(usd(value - cost)), pct)
}

pub(crate) const PLAN_BUYS_INCLUDES: &str = "Every receipt-verified buy in the accumulation ledger, across all plans: the USDC that left the wallet and the NOCK that entered it in each recorded transaction. Swap fees and price impact are inside those amounts.";
pub(crate) const PLAN_BUYS_EXCLUDES: [&str; 3] = [
    "NOCK held before the first recorded buy, and any buy that was not recorded.",
    "Everything that is not a buy: sells, NOCK that LP ranges sell or buy back, fees earned.",
    "Gas paid in ETH.",
];

/// The snapshot's cost-basis block. Pure.
///
/// `fills` is the whole accumulation ledger, oldest first, or the reason it could not be
/// read (then both layers that need it are `unavailable`, never "no buys"). `spot` and
/// `held_nock` are this cycle's reads; None is reported as null. `held_nock` arrives
/// rounded to a whole NOCK (it is the snapshot's `total_nock_amount`) and is reported
/// that way: no decimals that were never read.
pub(crate) fn compute_cost_basis(
    fills: std::result::Result<&[BasisFill], &str>,
    opening: &OpeningBasisState,
    spot: Option<f64>,
    held_nock: Option<f64>,
) -> CostBasis {
    let spot = spot.filter(|s| s.is_finite() && *s > 0.0);
    let plan_notes = (
        vec![PLAN_BUYS_INCLUDES.to_string()],
        PLAN_BUYS_EXCLUDES.iter().map(|s| s.to_string()).collect::<Vec<_>>(),
    );

    let plan_buys = match fills {
        // The ledger was not read, so the count is unknown: `fills` stays null, never 0.
        Err(reason) => PlanBuysBasis {
            status: "unavailable".to_string(),
            reason: Some(reason.to_string()),
            includes: plan_notes.0.clone(),
            excludes: plan_notes.1.clone(),
            ..Default::default()
        },
        Ok([]) => PlanBuysBasis {
            status: "no_fills".to_string(),
            // The ledger was read and holds no buy: a real zero.
            fills: Some(0),
            includes: plan_notes.0.clone(),
            excludes: plan_notes.1.clone(),
            ..Default::default()
        },
        Ok(fills) => {
            struct PlanRun {
                plan_id: String,
                totals: Totals,
                fills: i64,
                first: DateTime<Utc>,
                last: DateTime<Utc>,
            }
            let mut all = Totals::default();
            // Per plan, in order of each plan's first fill.
            let mut plans: Vec<PlanRun> = Vec::new();
            for f in fills {
                all.add(f);
                match plans.iter_mut().find(|p| p.plan_id == f.plan_id) {
                    Some(p) => {
                        p.totals.add(f);
                        p.fills += 1;
                        p.first = p.first.min(f.confirmed_at);
                        p.last = p.last.max(f.confirmed_at);
                    }
                    None => {
                        let mut totals = Totals::default();
                        totals.add(f);
                        plans.push(PlanRun {
                            plan_id: f.plan_id.clone(),
                            totals,
                            fills: 1,
                            first: f.confirmed_at,
                            last: f.confirmed_at,
                        });
                    }
                }
            }
            let (value, diff, pct) = at_spot(all.nock, all.usdc, spot);
            PlanBuysBasis {
                status: "ok".to_string(),
                reason: None,
                fills: Some(fills.len() as i64),
                spent_usdc: Some(usd(all.usdc)),
                acquired_nock: Some(nock_units(all.nock)),
                avg_cost_usd: all.avg().map(price),
                first_fill_at: fills.iter().map(|f| f.confirmed_at).min().map(|t| t.to_rfc3339()),
                last_fill_at: fills.iter().map(|f| f.confirmed_at).max().map(|t| t.to_rfc3339()),
                value_at_spot_usd: value,
                unrealized_usd: diff,
                unrealized_pct: pct,
                plans: plans
                    .into_iter()
                    .map(|p| PlanBasis {
                        plan_id: p.plan_id,
                        fills: p.fills,
                        spent_usdc: usd(p.totals.usdc),
                        acquired_nock: nock_units(p.totals.nock),
                        avg_cost_usd: p.totals.avg().map(price).unwrap_or_default(),
                        first_fill_at: p.first.to_rfc3339(),
                        last_fill_at: p.last.to_rfc3339(),
                    })
                    .collect(),
                includes: plan_notes.0.clone(),
                excludes: plan_notes.1.clone(),
            }
        }
    };

    let whole_stack = match opening {
        OpeningBasisState::NotConfigured => WholeStackBasis {
            status: "not_configured".to_string(),
            reason: Some(
                "cost_basis.opening is not set in config. NockNock cannot derive the whole stack's cost from the chain, so only plan buys are reported."
                    .to_string(),
            ),
            ..Default::default()
        },
        OpeningBasisState::Invalid(reason) => WholeStackBasis {
            status: "invalid".to_string(),
            reason: Some(reason.clone()),
            ..Default::default()
        },
        OpeningBasisState::Set(o) => {
            let opening_fields = WholeStackBasis {
                source: Some(o.source.clone()),
                as_of: Some(o.as_of.to_rfc3339()),
                opening_nock: Some(nock_units(o.nock)),
                opening_cost_usd: Some(usd(o.cost_usd)),
                opening_avg_cost_usd: Some(price(o.cost_usd / o.nock)),
                held_nock: held_nock.filter(|h| h.is_finite()).map(whole_nock),
                ..Default::default()
            };
            match fills {
                // The opening figures are still the operator's; the blend needs the ledger.
                Err(reason) => WholeStackBasis {
                    status: "unavailable".to_string(),
                    reason: Some(reason.to_string()),
                    ..opening_fields
                },
                Ok(fills) => {
                    let mut since = Totals::default();
                    let mut n = 0i64;
                    for f in fills.iter().filter(|f| counts_after_opening(f, o)) {
                        since.add(f);
                        n += 1;
                    }
                    let (basis_nock, basis_cost) = (o.nock + since.nock, o.cost_usd + since.usdc);
                    let (value, diff, pct) = at_spot(basis_nock, basis_cost, spot);
                    let as_of = o.as_of.to_rfc3339();
                    WholeStackBasis {
                        status: "ok".to_string(),
                        fills_since: Some(n),
                        bought_since_usdc: Some(usd(since.usdc)),
                        bought_since_nock: Some(nock_units(since.nock)),
                        basis_nock: Some(nock_units(basis_nock)),
                        basis_cost_usd: Some(usd(basis_cost)),
                        avg_cost_usd: Some(price(basis_cost / basis_nock)),
                        value_at_spot_usd: value,
                        unrealized_usd: diff,
                        unrealized_pct: pct,
                        includes: vec![
                            format!(
                                "The opening figures entered in config: {} NOCK for ${} as of {as_of}, source: {}. NockNock did not read or check them.",
                                nock_units(o.nock),
                                usd(o.cost_usd),
                                o.source
                            ),
                            format!("Every receipt-verified buy confirmed after {as_of}: {n} so far."),
                        ],
                        excludes: vec![
                            format!("Buys confirmed at or before {as_of}: they are taken to be inside the opening figures and are not added again."),
                            format!("Everything after {as_of} that is not a recorded buy: sells, NOCK that LP ranges sell or buy back, fees earned, transfers in or out, unrecorded buys."),
                            "Realized gains and losses, and gas paid in ETH.".to_string(),
                        ],
                        ..opening_fields
                    }
                }
            }
        }
    };

    CostBasis { spot_price_usd: spot.map(price), plan_buys, whole_stack }
}

// ============================================================
// Time series: fills with their running averages, and stored spot
// ============================================================

/// One stored NOCK/USD observation from `market_price_history`: the price AT `at`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SpotPoint {
    pub(crate) at: DateTime<Utc>,
    pub(crate) price_usd: f64,
    pub(crate) source: String,
    pub(crate) is_backfilled: bool,
}

impl SpotPoint {
    /// The price a stored row gives for its own `timestamp`. None when it gives none.
    ///
    /// The table holds two kinds of row. An observation (the worker's own read, or the
    /// single DEXScreener seed) is stamped when it was taken and has no OHLC columns:
    /// its `price_usd` is the price at `timestamp`. A third-party candle is stored at its
    /// OPEN time with `price_usd` set to its CLOSE, which is the price one candle period
    /// later (an hour, or a day). For a candle the price at `timestamp` is its open, so
    /// that is what is used here; the close is never placed at the open time. A candle
    /// whose open is not a positive number yields no point rather than its close.
    pub(crate) fn from_stored(
        timestamp: DateTime<Utc>,
        price_usd: f64,
        open_usd: Option<f64>,
        source: &str,
        is_backfilled: bool,
    ) -> Option<Self> {
        let at_timestamp = match open_usd {
            Some(open) => open,
            None => price_usd,
        };
        (at_timestamp.is_finite() && at_timestamp > 0.0).then(|| Self {
            at: timestamp,
            price_usd: at_timestamp,
            source: source.to_string(),
            is_backfilled,
        })
    }

    fn to_json(&self) -> serde_json::Value {
        json!({
            "at": self.at.to_rfc3339(),
            "price_usd": format!("{:.8}", self.price_usd),
            "source": self.source,
            "is_backfilled": self.is_backfilled,
        })
    }
}

/// The stored price that stands for `at`, if one lies within `window` of it. `points`
/// must be sorted oldest first. Among the points inside the window, the worker's own
/// contemporaneous read nearest `at` is taken; only when there is none is the nearest
/// backfilled (third-party) point used. None means there is no price for that time: the
/// caller reports null and never reaches for a farther point.
pub(crate) fn nearest_spot(points: &[SpotPoint], at: DateTime<Utc>, window: chrono::Duration) -> Option<&SpotPoint> {
    let i = points.partition_point(|p| p.at < at);
    let distance = |p: &SpotPoint| (p.at - at).num_milliseconds().abs();
    let within = |p: &&SpotPoint| distance(p) <= window.num_milliseconds();
    let before = points[..i].iter().rev().take_while(within);
    let after = points[i..].iter().take_while(within);
    before.chain(after).min_by_key(|p| (p.is_backfilled, distance(p)))
}

/// The bucket width that keeps a range within `max_points`: the smallest of a fixed
/// ladder from 5 minutes (the default sync window) to one day.
pub(crate) fn series_bucket_seconds(range_seconds: i64, max_points: i64) -> i64 {
    const LADDER: [i64; 9] = [300, 900, 1800, 3600, 7200, 14400, 21600, 43200, 86400];
    let max_points = max_points.max(1);
    LADDER
        .into_iter()
        .find(|b| range_seconds.max(0) / b <= max_points)
        .unwrap_or(86400)
}

/// Thin a sorted series to at most one point per bucket: the LAST stored observation in
/// each. Every point returned is a stored row with its own timestamp and price; nothing
/// is averaged or interpolated, and a bucket with no observation yields no point.
pub(crate) fn downsample_spot(points: &[SpotPoint], bucket_seconds: i64) -> Vec<SpotPoint> {
    let bucket = bucket_seconds.max(1);
    let mut out: Vec<SpotPoint> = Vec::new();
    let mut current: Option<i64> = None;
    for p in points {
        let b = p.at.timestamp().div_euclid(bucket);
        if current == Some(b) {
            if let Some(last) = out.last_mut() {
                *last = p.clone();
            }
        } else {
            out.push(p.clone());
            current = Some(b);
        }
    }
    out
}

/// Each fill with the averages as they stood once it was confirmed. Pure.
///
/// - `plan_avg_cost_usd`: running average inside the fill's own plan;
/// - `avg_cost_usd`: running average across every plan in the ledger;
/// - `whole_stack_avg_cost_usd`: the blended average, only when an opening basis is set
///   and the fill was confirmed after it (null otherwise);
/// - `spot_at_fill`: the stored price that stands for the fill's block time (see
///   `nearest_spot`), or null when none lies within the match window.
pub(crate) fn fill_series(fills: &[BasisFill], opening: Option<&OpeningBasis>, spot: &[SpotPoint]) -> Vec<serde_json::Value> {
    let window = chrono::Duration::minutes(COST_BASIS_SPOT_MATCH_WINDOW_MINUTES);
    let mut all = Totals::default();
    let mut per_plan: HashMap<String, Totals> = HashMap::new();
    let mut blended = opening.map(|o| Totals { usdc: o.cost_usd, nock: o.nock });
    fills
        .iter()
        .map(|f| {
            all.add(f);
            let plan = per_plan.entry(f.plan_id.clone()).or_default();
            plan.add(f);
            let whole = match (opening, blended.as_mut()) {
                (Some(o), Some(b)) if counts_after_opening(f, o) => {
                    b.add(f);
                    b.avg().map(|p| format!("{:.8}", p))
                }
                _ => None,
            };
            json!({
                "plan_id": f.plan_id,
                "tx_hash": f.tx_hash,
                "block_number": f.block_number,
                "confirmed_at": f.confirmed_at.to_rfc3339(),
                "usdc_spent": f.usdc_spent,
                "nock_received": f.nock_received,
                "effective_price_usd": format!("{:.8}", f.usdc / f.nock),
                "plan_avg_cost_usd": plan.avg().map(|p| format!("{:.8}", p)),
                "avg_cost_usd": all.avg().map(|p| format!("{:.8}", p)),
                "whole_stack_avg_cost_usd": whole,
                "spot_at_fill": nearest_spot(spot, f.confirmed_at, window).map(|p| p.to_json()),
            })
        })
        .collect()
}

/// How many days the series covers when the caller does not say: back to the earlier of
/// the first fill and the opening basis, plus a day of margin, within 2 to 365 days.
pub(crate) fn default_series_days(earliest: Option<DateTime<Utc>>, now: DateTime<Utc>) -> i64 {
    match earliest {
        None => COST_BASIS_SERIES_DEFAULT_DAYS,
        Some(t) => {
            let span = (now - t).num_seconds().max(0);
            ((span + 86_399) / 86_400 + 1).clamp(2, COST_BASIS_SERIES_MAX_DAYS)
        }
    }
}

/// The series response body. Pure: every input is passed in.
///
/// `spot` is the stored history covering at least `from..=to`, oldest first, or the reason
/// it could not be read. A failed spot read leaves `spot` null with `spot_error` set; the
/// fills and their averages do not depend on it.
pub(crate) fn build_series_response(
    fills: &[BasisFill],
    opening: &OpeningBasisState,
    spot: std::result::Result<&[SpotPoint], &str>,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> serde_json::Value {
    let bucket = series_bucket_seconds((to - from).num_seconds(), COST_BASIS_SERIES_MAX_POINTS);
    let opening_set = match opening {
        OpeningBasisState::Set(o) => Some(o),
        _ => None,
    };
    let (spot_json, spot_error, spot_points): (serde_json::Value, Option<String>, &[SpotPoint]) = match spot {
        Ok(points) => {
            let in_range: Vec<SpotPoint> =
                points.iter().filter(|p| p.at >= from && p.at <= to).cloned().collect();
            let thinned = downsample_spot(&in_range, bucket);
            (json!(thinned.iter().map(|p| p.to_json()).collect::<Vec<_>>()), None, points)
        }
        Err(reason) => (serde_json::Value::Null, Some(reason.to_string()), &[]),
    };
    let whole_stack = match opening {
        OpeningBasisState::NotConfigured => json!({ "status": "not_configured", "reason": null, "opening": null }),
        OpeningBasisState::Invalid(reason) => json!({ "status": "invalid", "reason": reason, "opening": null }),
        OpeningBasisState::Set(o) => json!({
            "status": "ok",
            "reason": null,
            "opening": {
                "as_of": o.as_of.to_rfc3339(),
                "nock": nock_units(o.nock),
                "cost_usd": usd(o.cost_usd),
                "avg_cost_usd": format!("{:.8}", o.cost_usd / o.nock),
                "source": o.source,
            },
        }),
    };
    json!({
        "generated_at": to.to_rfc3339(),
        "from": from.to_rfc3339(),
        "to": to.to_rfc3339(),
        "bucket_seconds": bucket,
        "spot_match_window_minutes": COST_BASIS_SPOT_MATCH_WINDOW_MINUTES,
        "spot": spot_json,
        "spot_error": spot_error,
        "fills": fill_series(fills, opening_set, spot_points),
        "whole_stack": whole_stack,
    })
}

// ============================================================
// Reads
// ============================================================

/// The whole accumulation ledger, oldest first. Named columns only, so columns added by
/// later migrations are ignored. An error is the reason text for `unavailable`.
pub(crate) async fn load_basis_fills(db: &PgPool) -> std::result::Result<Vec<BasisFill>, String> {
    let rows = sqlx::query(
        "SELECT plan_id, tx_hash, block_number, confirmed_at, usdc_spent, nock_received \
         FROM accumulation_events ORDER BY confirmed_at ASC, id ASC",
    )
    .fetch_all(db)
    .await
    .map_err(|e| {
        tracing::error!("Cost basis: accumulation ledger query failed: {e}");
        "the accumulation ledger could not be read".to_string()
    })?;
    rows.iter()
        .map(|r| {
            let column = |e: sqlx::Error| {
                tracing::error!("Cost basis: accumulation ledger row unreadable: {e}");
                "an accumulation ledger row could not be read".to_string()
            };
            BasisFill::parse(
                &r.try_get::<String, _>("plan_id").map_err(column)?,
                &r.try_get::<String, _>("tx_hash").map_err(column)?,
                r.try_get::<i64, _>("block_number").map_err(column)?,
                r.try_get::<DateTime<Utc>, _>("confirmed_at").map_err(column)?,
                &r.try_get::<String, _>("usdc_spent").map_err(column)?,
                &r.try_get::<String, _>("nock_received").map_err(column)?,
            )
        })
        .collect()
}

/// Stored spot between two instants, oldest first. Each row becomes the price at its own
/// timestamp (`SpotPoint::from_stored`): a candle contributes its open, not its close.
pub(crate) async fn load_spot_points(
    db: &PgPool,
    from: DateTime<Utc>,
    to: DateTime<Utc>,
) -> std::result::Result<Vec<SpotPoint>, String> {
    let rows = sqlx::query(
        "SELECT timestamp, price_usd, open_usd, source, is_backfilled FROM market_price_history \
         WHERE timestamp >= $1 AND timestamp <= $2 ORDER BY timestamp ASC",
    )
    .bind(from)
    .bind(to)
    .fetch_all(db)
    .await
    .map_err(|e| {
        tracing::warn!("Cost basis: price history query failed: {e}");
        "the stored price history could not be read".to_string()
    })?;
    Ok(rows
        .iter()
        .filter_map(|r| {
            SpotPoint::from_stored(
                r.try_get("timestamp").ok()?,
                r.try_get("price_usd").ok()?,
                r.try_get::<Option<f64>, _>("open_usd").ok()?,
                &r.try_get::<String, _>("source").ok()?,
                r.try_get("is_backfilled").ok()?,
            )
        })
        .collect())
}

/// This cycle's cost-basis block. The ledger read is optional to the cycle: when it
/// fails the block says `unavailable` with the reason and the cycle carries on.
pub(crate) async fn cost_basis_for_cycle(state: &Arc<AppState>, spot: f64, held_nock: Option<f64>) -> CostBasis {
    let opening = resolve_opening_basis(state.config.opening_basis(), Utc::now());
    let fills = load_basis_fills(&state.db).await;
    compute_cost_basis(fills.as_deref().map_err(|e| e.as_str()), &opening, Some(spot), held_nock)
}

/// GET /v1/cost-basis/series — the chart's data: every verified fill with the running
/// averages after it and the stored spot nearest it, plus the stored spot history over
/// the range, thinned to real observations. Optional `days` (1 to 365) sets the range.
pub(crate) async fn cost_basis_series_handler(
    Extension(state): Extension<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Response {
    let now = Utc::now();
    let days_param = match params.get("days") {
        None => None,
        Some(raw) => match raw.trim().parse::<i64>() {
            Ok(d) if (1..=COST_BASIS_SERIES_MAX_DAYS).contains(&d) => Some(d),
            _ => {
                return (
                    StatusCode::BAD_REQUEST,
                    Json(json!({
                        "error": "INVALID_PARAMETER",
                        "detail": format!("days must be a whole number from 1 to {COST_BASIS_SERIES_MAX_DAYS}")
                    })),
                )
                    .into_response()
            }
        },
    };
    let fills = match load_basis_fills(&state.db).await {
        Ok(f) => f,
        Err(reason) => {
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "LEDGER_UNAVAILABLE", "detail": reason })),
            )
                .into_response()
        }
    };
    let opening = resolve_opening_basis(state.config.opening_basis(), now);
    let first_fill = fills.iter().map(|f| f.confirmed_at).min();
    let opening_at = match &opening {
        OpeningBasisState::Set(o) => Some(o.as_of),
        _ => None,
    };
    let earliest = [first_fill, opening_at].into_iter().flatten().min();
    let days = days_param.unwrap_or_else(|| default_series_days(earliest, now));
    let from = now - chrono::Duration::days(days);
    // Spot is also matched to fills older than the range, so read back to the first fill.
    let window = chrono::Duration::minutes(COST_BASIS_SPOT_MATCH_WINDOW_MINUTES);
    let read_from = first_fill.map_or(from, |t| from.min(t - window));
    let spot = load_spot_points(&state.db, read_from, now).await;
    Json(build_series_response(&fills, &opening, spot.as_deref().map_err(|e| e.as_str()), from, now)).into_response()
}
