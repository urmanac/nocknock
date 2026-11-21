//! Fee yield (snapshot 2.10.0, extending the ladder simulator #166): the swap fees our own
//! tracked ranges earned per unit of liquidity while in range, measured from the stored
//! snapshot history. The ladder simulator multiplies it by a rung's own liquidity to
//! estimate the rung's fees per day while in range.
//!
//! What is measured. Each stored snapshot holds every tracked position's liquidity and its
//! uncollected fees (`tokens_owed_0/1`). Between two consecutive rows of one position whose
//! liquidity did not change, the growth of its uncollected fees divided by its liquidity
//! is the fee per unit of liquidity that any range containing the price earned over that
//! interval. Nothing else is read: not `harvest_events` (no position id, and a v4 add
//! collects fees that never reach the ledger), not `market_price_history` (its volume is
//! another pool's and is never used here).
//!
//! Which positions measure. Per USDC/NOCK pool, one tracked concentrated range whose fees
//! accrue to the LP: not full range (a basic Aero pool), not staked in a gauge (swap fees on
//! staked liquidity go to the gauge), and not a tracked ladder (#167). With several in one
//! pool, the widest measures (then the lowest position id) and the others are listed as
//! excluded with that reason, so measured seconds stay wall-clock seconds.
//!
//! When. At most once an hour (`FEE_YIELD_REFRESH_SECONDS`): one bounded query over the
//! trailing window, its result carried forward between and re-priced every cycle at that
//! cycle's pool price. The block is left out of the state hash, so it never makes the
//! worker store a row it would not have stored.
use crate::*;

/// The trailing window measured, in days. A measurement window, echoed; not an operator
/// number.
pub(crate) const FEE_YIELD_WINDOW_DAYS: i64 = 30;
/// Two rows further apart than this are not one interval (a failed cycle or a restart),
/// unless both rows hold the same fees, liquidity and tick: the worker stores no new row
/// while nothing changes, so such a gap is zero-fee time.
pub(crate) const FEE_YIELD_MAX_GAP_SECONDS: i64 = 7200;
/// A day enters the daily spread with at least this many measured seconds. Display.
pub(crate) const FEE_YIELD_DAY_MIN_SECONDS: i64 = 72000;
/// The history is measured at most this often; the last result is carried forward.
pub(crate) const FEE_YIELD_REFRESH_SECONDS: i64 = 3600;
pub(crate) const FEE_YIELD_CONSTANTS_NOTE: &str =
    "window_days, max_gap_seconds, day_min_seconds and refresh_seconds are measurement and display constants, not settings";
pub(crate) const FEE_YIELD_STATEMENT: &str = "Measured from stored snapshots of our own tracked ranges: swap fees per unit of liquidity while in range. A past measurement, not a forecast. The stored market volume of another pool is never used.";
const SECONDS_PER_DAY: f64 = 86400.0;

/// One stored snapshot's reading of one position (and its pool's tick).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FeeYieldRow {
    pub(crate) at: DateTime<Utc>,
    pub(crate) position_id: String,
    /// Raw liquidity; None when the stored string could not be read.
    pub(crate) liquidity: Option<u128>,
    pub(crate) owed0: Option<f64>,
    pub(crate) owed1: Option<f64>,
    pub(crate) in_range: Option<bool>,
    pub(crate) pool_active: Option<bool>,
    pub(crate) tick: Option<i32>,
    /// The row says its fees were not LP fees then (staked in a gauge, or a fees note):
    /// the stored owed amounts are carried as 0, so such time is never measured.
    pub(crate) staked: bool,
}

/// e-notation with 7 significant digits, the way per-liquidity figures are stored. Pure.
pub(crate) fn sci(v: f64) -> String {
    format!("{:.6e}", v + 0.0)
}

fn usdc_nock_pair(p: &Position) -> bool {
    (is_usdc_token(&p.token0) && is_nock_token(&p.token1)) || (is_nock_token(&p.token0) && is_usdc_token(&p.token1))
}

/// The label a pool's measuring range is named by, in the adapters' words. Pure.
fn measuring_label(p: &Position) -> String {
    let fee_pct = p.fee_tier_bps as f64 / 100.0;
    let proto = match p.protocol.as_str() {
        "uniswap-v4" => "v4",
        "uniswap-v3" => "v3",
        "aerodrome-cl" => "Aero CL",
        other => other,
    };
    format!("{proto} {}/{} {:.2}% #{}", p.token0.symbol, p.token1.symbol, fee_pct, p.nft_id)
}

/// The measuring range of each USDC/NOCK pool, and every other position with a NOCK leg
/// with the reason it does not measure. Pure.
pub(crate) fn measuring_positions(positions: &[Position]) -> (Vec<&Position>, Vec<FeeYieldExcluded>) {
    let mut excluded = Vec::new();
    let mut candidates: Vec<&Position> = Vec::new();
    for p in positions {
        if !(is_nock_token(&p.token0) || is_nock_token(&p.token1)) {
            continue;
        }
        let reason = if !usdc_nock_pair(p) {
            Some(format!("a {}/{} pool: fees are measured only in USDC/NOCK pools, where a ladder sits", p.token0.symbol, p.token1.symbol))
        } else if p.full_range || is_full_range(p.lower_tick, p.upper_tick) {
            Some("a full-range position (a basic Aero pool): it has no range to measure, and its claimFees reverts".to_string())
        } else if p.staked_in_gauge.is_some() {
            Some("staked: swap fees on staked liquidity go to the gauge, not to the LP".to_string())
        } else if let Some(note) = &p.fees_note {
            Some(format!("its fees are not LP-claimable: {note}"))
        } else if p.role.as_deref() == Some("ladder") {
            Some("a tracked ladder (#167): its rungs are not the measuring range".to_string())
        } else {
            None
        };
        match reason {
            Some(r) => excluded.push(FeeYieldExcluded { position_id: p.position_id.clone(), reason: r }),
            None => candidates.push(p),
        }
    }
    // One per pool: the widest, then the lowest position id.
    let mut chosen: Vec<&Position> = Vec::new();
    for p in &candidates {
        let best = candidates
            .iter()
            .filter(|q| q.pool_id == p.pool_id)
            .max_by(|a, b| {
                let wa = a.upper_tick as i64 - a.lower_tick as i64;
                let wb = b.upper_tick as i64 - b.lower_tick as i64;
                wa.cmp(&wb).then_with(|| b.position_id.cmp(&a.position_id))
            })
            .expect("p itself");
        if best.position_id == p.position_id {
            chosen.push(p);
        } else {
            excluded.push(FeeYieldExcluded {
                position_id: p.position_id.clone(),
                reason: format!("another tracked range measures this pool: {}, the widest", best.position_id),
            });
        }
    }
    excluded.sort_by(|a, b| a.position_id.cmp(&b.position_id));
    (chosen, excluded)
}

/// Why one interval is not counted.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Skip {
    LiquidityChanged,
    ZeroLiquidity,
    FeesFell,
    PoolInactive,
    OutOfRange,
    Gap,
    Unreadable,
    Staked,
}

fn add_skip(s: &mut FeeYieldSkips, secs: &mut FeeYieldSkipSeconds, why: Skip, dt: f64) {
    let (n, t) = match why {
        Skip::LiquidityChanged => (&mut s.liquidity_changed, &mut secs.liquidity_changed),
        Skip::ZeroLiquidity => (&mut s.zero_liquidity, &mut secs.zero_liquidity),
        Skip::FeesFell => (&mut s.fees_fell, &mut secs.fees_fell),
        Skip::PoolInactive => (&mut s.pool_inactive, &mut secs.pool_inactive),
        Skip::OutOfRange => (&mut s.out_of_range, &mut secs.out_of_range),
        Skip::Gap => (&mut s.gap, &mut secs.gap),
        Skip::Unreadable => (&mut s.unreadable, &mut secs.unreadable),
        Skip::Staked => (&mut s.staked, &mut secs.staked),
    };
    *n += 1;
    *t += dt;
}

fn ms(v: f64) -> f64 {
    (v * 1000.0).round() / 1000.0 + 0.0
}

/// The measurement of one pool from its measuring range's stored rows (any order; other
/// positions' rows are ignored). Unpriced: `price_fee_yield` stamps the USD figures. Pure.
pub(crate) fn measure_pool(p: &Position, rows: &[FeeYieldRow]) -> FeeYieldPool {
    let mut mine: Vec<&FeeYieldRow> = rows.iter().filter(|r| r.position_id == p.position_id).collect();
    mine.sort_by_key(|r| r.at);
    let mut skipped = FeeYieldSkips::default();
    let mut skipped_seconds = FeeYieldSkipSeconds::default();
    let (mut secs, mut s0, mut s1) = (0.0f64, 0.0f64, 0.0f64);
    let mut intervals = 0u32;
    let mut zero_gaps = 0u32;
    // date -> (seconds, Σd0/L, Σd1/L); (bin_lo, bin_hi) -> the same.
    let mut days: std::collections::BTreeMap<String, (f64, f64, f64)> = Default::default();
    let mut spans: std::collections::BTreeMap<(i32, i32), (f64, f64, f64)> = Default::default();
    let spacing = p.tick_spacing.filter(|s| *s > 0);
    for w in mine.windows(2) {
        let (a, b) = (w[0], w[1]);
        let dt = (b.at - a.at).num_microseconds().map(|u| u as f64 / 1e6).unwrap_or(f64::INFINITY);
        if !(dt > 0.0) || !dt.is_finite() {
            add_skip(&mut skipped, &mut skipped_seconds, Skip::Unreadable, 0.0);
            continue;
        }
        let why = match (a.liquidity, b.liquidity, a.owed0, a.owed1, b.owed0, b.owed1) {
            // Staked at either end: the swap fees went to the gauge, not to the LP.
            _ if a.staked || b.staked => Some(Skip::Staked),
            (Some(la), Some(lb), Some(a0), Some(a1), Some(b0), Some(b1)) => {
                if la != lb {
                    Some(Skip::LiquidityChanged)
                } else if la == 0 {
                    Some(Skip::ZeroLiquidity)
                } else if b0 < a0 || b1 < a1 {
                    Some(Skip::FeesFell)
                } else if a.pool_active != Some(true) || b.pool_active != Some(true) {
                    Some(Skip::PoolInactive)
                } else if a.in_range != Some(true) || b.in_range != Some(true) {
                    Some(Skip::OutOfRange)
                } else if a.tick.is_none() || b.tick.is_none() {
                    Some(Skip::Unreadable)
                } else if dt > FEE_YIELD_MAX_GAP_SECONDS as f64 {
                    if a0 == b0 && a1 == b1 && a.tick == b.tick {
                        zero_gaps += 1;
                        None
                    } else {
                        Some(Skip::Gap)
                    }
                } else {
                    None
                }
            }
            _ => Some(Skip::Unreadable),
        };
        if let Some(why) = why {
            add_skip(&mut skipped, &mut skipped_seconds, why, dt);
            continue;
        }
        let l = a.liquidity.unwrap() as f64;
        let d0 = (b.owed0.unwrap() - a.owed0.unwrap()) / l;
        let d1 = (b.owed1.unwrap() - a.owed1.unwrap()) / l;
        intervals += 1;
        secs += dt;
        s0 += d0;
        s1 += d1;
        let day = days.entry(a.at.format("%Y-%m-%d").to_string()).or_default();
        day.0 += dt;
        day.1 += d0;
        day.2 += d1;
        if let Some(s) = spacing {
            let (ta, tb) = (a.tick.unwrap(), b.tick.unwrap());
            let bin = |t: i32| t.div_euclid(s) * s;
            let e = spans.entry((bin(ta.min(tb)), bin(ta.max(tb)))).or_default();
            e.0 += dt;
            e.1 += d0;
            e.2 += d1;
        }
    }
    let measured = intervals > 0 && secs > 0.0;
    let per_day = |sum: f64, s: f64| sum / (s / SECONDS_PER_DAY);
    let skipped_seconds = FeeYieldSkipSeconds {
        liquidity_changed: ms(skipped_seconds.liquidity_changed),
        zero_liquidity: ms(skipped_seconds.zero_liquidity),
        fees_fell: ms(skipped_seconds.fees_fell),
        pool_inactive: ms(skipped_seconds.pool_inactive),
        out_of_range: ms(skipped_seconds.out_of_range),
        gap: ms(skipped_seconds.gap),
        unreadable: ms(skipped_seconds.unreadable),
        staked: ms(skipped_seconds.staked),
    };
    FeeYieldPool {
        pool_id: p.pool_id.clone(),
        label: measuring_label(p),
        measured_by: p.position_id.clone(),
        fee_tier_bps: p.fee_tier_bps,
        tick_spacing: spacing,
        covered_tick_lower: p.lower_tick,
        covered_tick_upper: p.upper_tick,
        status: if measured { "measured" } else { "no_intervals" }.to_string(),
        reason: (!measured).then(|| {
            if mine.len() < 2 {
                format!("{} stored rows of {} in the window: too little history to measure", mine.len(), p.position_id)
            } else {
                format!("none of the {} intervals between stored rows of {} could be counted (see skipped)", mine.len() - 1, p.position_id)
            }
        }),
        from: mine.first().map(|r| r.at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
        to: mine.last().map(|r| r.at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
        rows: mine.len() as u32,
        intervals,
        zero_fee_gap_intervals: zero_gaps,
        measured_seconds: ms(secs),
        skipped,
        skipped_seconds,
        usdc_per_liquidity_day: measured.then(|| sci(per_day(s0, secs))),
        nock_per_liquidity_day: measured.then(|| sci(per_day(s1, secs))),
        pool_price_usd: None,
        usd_per_liquidity_day: None,
        usd_per_1e15_liquidity_day: None,
        by_day: days
            .into_iter()
            .map(|(date, (s, a0, a1))| FeeYieldDay {
                date,
                seconds: ms(s),
                usdc_per_liquidity_day: sci(per_day(a0, s)),
                nock_per_liquidity_day: sci(per_day(a1, s)),
                usd_per_liquidity_day: None,
            })
            .collect(),
        spans: spans
            .into_iter()
            .map(|((lo, hi), (s, a0, a1))| FeeYieldSpan { bin_lo: lo, bin_hi: hi, seconds: ms(s), usdc_per_liquidity: sci(a0), nock_per_liquidity: sci(a1) })
            .collect(),
        spans_reason: spacing.is_none().then(|| "the pool's tick spacing was not read, so intervals cannot be placed on its tick grid".to_string()),
        detail_note: None,
    }
}

/// The whole measurement from the snapshot positions (which choose the measuring ranges)
/// and the stored rows. Unpriced. Pure.
pub(crate) fn fee_yield_from_rows(positions: &[Position], rows: &[FeeYieldRow], computed_at: DateTime<Utc>) -> FeeYield {
    let (chosen, excluded) = measuring_positions(positions);
    let pools: Vec<FeeYieldPool> = chosen.iter().map(|p| measure_pool(p, rows)).collect();
    let mut fy = fee_yield_shell(Some(computed_at), excluded);
    if pools.is_empty() {
        fy.status = "unavailable".to_string();
        fy.reason = Some("no tracked USDC/NOCK concentrated range earns LP fees, so nothing measures them".to_string());
    }
    fy.pools = pools;
    fy
}

fn fee_yield_shell(computed_at: Option<DateTime<Utc>>, excluded: Vec<FeeYieldExcluded>) -> FeeYield {
    FeeYield {
        status: "measured".to_string(),
        reason: None,
        computed_at: computed_at.map(|t| t.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)),
        window_days: FEE_YIELD_WINDOW_DAYS,
        max_gap_seconds: FEE_YIELD_MAX_GAP_SECONDS,
        day_min_seconds: FEE_YIELD_DAY_MIN_SECONDS,
        refresh_seconds: FEE_YIELD_REFRESH_SECONDS,
        constants_note: FEE_YIELD_CONSTANTS_NOTE.to_string(),
        pools: Vec::new(),
        excluded,
        statement: FEE_YIELD_STATEMENT.to_string(),
    }
}

/// USD per NOCK in a USDC/NOCK pool from its stored sqrtPriceX96. Pure.
pub(crate) fn usd_per_nock_in_pool(p: &Position, pools: &[PoolMechanic]) -> Option<f64> {
    if !usdc_nock_pair(p) {
        return None;
    }
    let m = pools.iter().find(|m| m.pool_id == p.pool_id)?;
    let raw = sqrt_price_x96_to_raw(U256::from_hex_be(m.sqrt_price_x96.as_deref()?).ok()?);
    if !(raw > 0.0) || !raw.is_finite() {
        return None;
    }
    let d = (p.token1.decimals - p.token0.decimals) as i32;
    let usd = if is_nock_token(&p.token1) { 10f64.powi(d) / raw } else { raw / 10f64.powi(d) };
    (usd.is_finite() && usd > 0.0).then_some(usd)
}

/// Value the NOCK leg of every pool at this cycle's own pool price. Pure.
pub(crate) fn price_fee_yield(fy: &mut FeeYield, positions: &[Position], pool_mechanics: &[PoolMechanic]) {
    for pool in &mut fy.pools {
        let price = positions.iter().find(|p| p.position_id == pool.measured_by).and_then(|p| usd_per_nock_in_pool(p, pool_mechanics));
        let val = |u: &str, n: &str, px: f64| -> Option<f64> { Some(u.parse::<f64>().ok()? + n.parse::<f64>().ok()? * px) };
        pool.pool_price_usd = price.map(|v| format!("{:.8}", v));
        let usd = match (price, &pool.usdc_per_liquidity_day, &pool.nock_per_liquidity_day) {
            (Some(px), Some(u), Some(n)) => val(u, n, px),
            _ => None,
        };
        pool.usd_per_liquidity_day = usd.map(sci);
        pool.usd_per_1e15_liquidity_day = usd.map(|v| format!("{:.4}", v * 1e15 + 0.0));
        for d in &mut pool.by_day {
            d.usd_per_liquidity_day = price.and_then(|px| val(&d.usdc_per_liquidity_day, &d.nock_per_liquidity_day, px)).map(sci);
        }
    }
}

// ------------------------------------------------------------------
// The cycle: one bounded query at most once an hour
// ------------------------------------------------------------------

struct FeeYieldCache {
    computed_at: DateTime<Utc>,
    ids: Vec<String>,
    measured: FeeYield,
}

static FEE_YIELD_CACHE: std::sync::Mutex<Option<FeeYieldCache>> = std::sync::Mutex::new(None);

/// Whether a cached measurement can be carried forward. Pure.
pub(crate) fn fee_yield_is_fresh(computed_at: DateTime<Utc>, cached_ids: &[String], ids: &[String], now: DateTime<Utc>) -> bool {
    cached_ids == ids && (0..FEE_YIELD_REFRESH_SECONDS).contains(&now.signed_duration_since(computed_at).num_seconds())
}

/// One query: the measuring ranges' rows of the trailing window, only the fields used.
async fn fetch_fee_yield_rows(db: &PgPool, ids: &[String], since: DateTime<Utc>) -> Result<Vec<FeeYieldRow>> {
    let rows = sqlx::query(
        r#"
        SELECT s.generated_at,
               p->>'position_id' AS position_id,
               p->>'liquidity_amount' AS liquidity,
               p->>'tokens_owed_0' AS owed0,
               p->>'tokens_owed_1' AS owed1,
               (p->>'in_range')::boolean AS in_range,
               (p->>'pool_active')::boolean AS pool_active,
               (p ? 'staked_in_gauge' OR p ? 'fees_note') AS staked,
               (SELECT (m->>'tick_current')::int
                  FROM jsonb_array_elements(s.data->'pool_mechanics') m
                 WHERE m->>'pool_id' = p->>'pool_id' LIMIT 1) AS tick
        FROM snapshots s, jsonb_array_elements(s.data->'positions') p
        WHERE s.generated_at >= $1 AND p->>'position_id' = ANY($2)
        ORDER BY s.generated_at ASC
        "#,
    )
    .bind(since)
    .bind(ids)
    .fetch_all(db)
    .await?;
    Ok(rows
        .iter()
        .filter_map(|r| {
            let at: DateTime<Utc> = r.try_get("generated_at").ok()?;
            let position_id: String = r.try_get("position_id").ok()?;
            let txt = |k: &str| r.try_get::<Option<String>, _>(k).ok().flatten();
            Some(FeeYieldRow {
                at,
                position_id,
                liquidity: txt("liquidity").and_then(|v| v.trim().parse::<u128>().ok()),
                owed0: txt("owed0").and_then(|v| v.trim().parse::<f64>().ok()).filter(|v| v.is_finite()),
                owed1: txt("owed1").and_then(|v| v.trim().parse::<f64>().ok()).filter(|v| v.is_finite()),
                in_range: r.try_get::<Option<bool>, _>("in_range").ok().flatten(),
                pool_active: r.try_get::<Option<bool>, _>("pool_active").ok().flatten(),
                tick: r.try_get::<Option<i32>, _>("tick").ok().flatten(),
                // Unknown counts as staked: an unread flag is never measured as LP time.
                staked: r.try_get::<Option<bool>, _>("staked").ok().flatten().unwrap_or(true),
            })
        })
        .collect())
}

/// This cycle's fee_yield: the cached measurement when it is under an hour old and the
/// measuring ranges are the same, otherwise one query over the window. A failed query
/// carries the last measurement forward (its `computed_at` says how old it is); with none,
/// the block says why. Then priced at this cycle's pool prices. Never fails the cycle.
pub(crate) async fn fee_yield_for_cycle(db: &PgPool, data: &SnapshotData, now: DateTime<Utc>) -> FeeYield {
    let (chosen, excluded) = measuring_positions(&data.positions);
    let mut ids: Vec<String> = chosen.iter().map(|p| p.position_id.clone()).collect();
    ids.sort();
    let mut fy = if ids.is_empty() {
        fee_yield_from_rows(&data.positions, &[], now)
    } else {
        let cached = {
            let slot = FEE_YIELD_CACHE.lock().unwrap_or_else(|p| p.into_inner());
            slot.as_ref().map(|c| (c.computed_at, c.ids.clone(), c.measured.clone()))
        };
        match cached {
            Some((at, cids, m)) if fee_yield_is_fresh(at, &cids, &ids, now) => m,
            stale => match fetch_fee_yield_rows(db, &ids, now - chrono::Duration::days(FEE_YIELD_WINDOW_DAYS)).await {
                Ok(rows) => {
                    let m = fee_yield_from_rows(&data.positions, &rows, now);
                    *FEE_YIELD_CACHE.lock().unwrap_or_else(|p| p.into_inner()) =
                        Some(FeeYieldCache { computed_at: now, ids: ids.clone(), measured: m.clone() });
                    m
                }
                Err(e) => {
                    tracing::warn!("fee_yield history query failed (non-fatal): {e}");
                    match stale {
                        Some((_, cids, m)) if cids == ids => m,
                        _ => {
                            let mut f = fee_yield_shell(None, excluded);
                            f.status = "unavailable".to_string();
                            f.reason = Some("the stored history could not be read this cycle".to_string());
                            f
                        }
                    }
                }
            },
        }
    };
    price_fee_yield(&mut fy, &data.positions, &data.pool_mechanics);
    fy
}
