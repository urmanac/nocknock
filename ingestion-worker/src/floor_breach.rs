//! The floor-crossing price (issue #165): how far NOCK can rise before the deployed
//! ranges sell the stack below the floor. A measurement of the ranges already deployed,
//! never advice. Pure: no clock, no network, no database.
//!
//! Tracked NOCK at a multiple `m >= 1` of spot is
//! `T(m) = W + F + Σ NOCK_i(m)`: wallet NOCK, uncollected NOCK fees and every NOCK-pair
//! position's NOCK at that price. At `m = 1` it is `global_status.total_nock_amount`
//! (advice.rs), the Floor Health reading. Every term is continuous and non-increasing in
//! `m`, so the crossing is found by deterministic bisection on `ln m`.
use crate::*;

/// What the figure assumes, stated in the snapshot beside it.
pub(crate) const FLOOR_BREACH_ASSUMPTION: &str = "Each pool is moved by the same multiple from its own current price. WETH/NOCK ranges hold ETH/USD constant (a NOCK/USD multiple is the same NOCK/ETH multiple). Full-range pools follow x*y=k on the wallet's share and never sell out. Uncollected NOCK fees count as held. Gas-side ranges with no NOCK leg hold no NOCK.";

/// Bisection steps on ln m: far below display precision, and deterministic.
const BISECTION_STEPS: usize = 200;

/// Where one level is crossed.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum Crossing {
    /// Tracked NOCK is already below the level at spot.
    AlreadyBelow,
    /// No rise takes it below: wallet NOCK and fees alone are at or above the level.
    Never,
    /// The first multiple of spot at which tracked NOCK is below the level.
    Crosses(f64),
}

/// How one NOCK-pair position's NOCK moves with the multiple.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Shape {
    /// No liquidity, or no NOCK leg: always 0.
    Empty,
    /// Full range (Aero basic): x*y=k on the wallet's share, `a / sqrt(m)`.
    Full { a: f64 },
    /// A concentrated range: `concentrated_amounts_at` at the moved sqrt price.
    Range { l: f64, s0: f64, sa: f64, sb: f64, nock_is_token1: bool, scale: f64 },
}

impl Shape {
    fn nock_at(&self, m: f64) -> f64 {
        match *self {
            Shape::Empty => 0.0,
            Shape::Full { a } => a / m.sqrt(),
            Shape::Range { l, s0, sa, sb, nock_is_token1, scale } => {
                // NOCK as token1: the raw price is NOCK per other token, so NOCK getting
                // dearer moves s down; as token0 it moves s up.
                if nock_is_token1 {
                    concentrated_amounts_at(l, s0 / m.sqrt(), sa, sb).1 / scale
                } else {
                    concentrated_amounts_at(l, s0 * m.sqrt(), sa, sb).0 / scale
                }
            }
        }
    }

    /// The exact multiple at which the range holds no NOCK, for bracketing the solve.
    fn edge(&self) -> Option<f64> {
        match *self {
            Shape::Range { s0, sa, sb, nock_is_token1, .. } => {
                if nock_is_token1 {
                    (s0 > sa).then(|| (s0 / sa).powi(2))
                } else {
                    (s0 < sb).then(|| (sb / s0).powi(2))
                }
            }
            _ => None,
        }
    }

    fn full_range_nock(&self) -> f64 {
        match *self {
            Shape::Full { a } => a,
            _ => 0.0,
        }
    }
}

/// The NOCK leg's amount in human units, or None for a position with no NOCK leg.
fn nock_amount(p: &RawPosition) -> Option<f64> {
    if is_nock_token(&p.token1) {
        Some(p.amount1)
    } else if is_nock_token(&p.token0) {
        Some(p.amount0)
    } else {
        None
    }
}

/// The position's NOCK as a function of the multiple, or the reason it cannot be
/// evaluated. Pool activity is not consulted here (see `floor_breach_calc`).
fn shape(p: &RawPosition) -> Result<Shape, String> {
    let label = position_label(p);
    let nock_is_token1 = is_nock_token(&p.token1);
    if !nock_is_token1 && !is_nock_token(&p.token0) {
        return Ok(Shape::Empty);
    }
    if is_full_range(p.tick_lower, p.tick_upper) {
        let a = nock_amount(p).unwrap_or(0.0);
        if !a.is_finite() || a < 0.0 {
            return Err(format!("{label}: its NOCK amount could not be read ({a})"));
        }
        return Ok(Shape::Full { a });
    }
    if p.liquidity == 0 {
        return Ok(Shape::Empty);
    }
    let s0 = sqrt_price_x96_to_raw(p.sqrt_price_x96).sqrt();
    if !s0.is_finite() || s0 <= 0.0 {
        return Err(format!("{label}: pool price could not be read (sqrtPrice is zero)"));
    }
    let decimals = if nock_is_token1 { p.token1.decimals } else { p.token0.decimals };
    Ok(Shape::Range {
        l: p.liquidity as f64,
        s0,
        sa: tick_sqrt_price(p.tick_lower),
        sb: tick_sqrt_price(p.tick_upper),
        nock_is_token1,
        scale: 10f64.powi(decimals),
    })
}

/// One position's NOCK, in human units, when NOCK is `m` times its price now. Pure.
/// Err(reason) when the position cannot be evaluated. At `m = 1` this is the
/// adapter's own amount (both come from `concentrated_amounts_at` or the reserve share).
#[allow(dead_code)] // tests and #166
pub(crate) fn nock_at_multiple(p: &RawPosition, m: f64) -> Result<f64, String> {
    shape(p).map(|s| s.nock_at(m))
}

/// The exact multiple at which a concentrated range holds no NOCK: (s0/sa)^2 for NOCK as
/// token1, (sb/s0)^2 as token0. None for full range, an empty range, or a range already
/// past its edge. It brackets the solve; the reported sell-out multiple is the
/// tick-based `nock_sell_out_multiple`, which agrees to within one tick.
#[allow(dead_code)] // tests and #166
pub(crate) fn sell_out_edge_multiple(p: &RawPosition) -> Option<f64> {
    shape(p).ok().and_then(|s| s.edge())
}

/// Where `fixed + Σ shapes(m)` first drops below `level`. `now` is the total at spot.
fn solve(fixed: f64, now: f64, shapes: &[Shape], level: f64) -> Crossing {
    if now < level {
        return Crossing::AlreadyBelow;
    }
    // Every range sold out leaves `fixed`: at or above the level, no rise crosses it.
    if fixed >= level - 1e-9 {
        return Crossing::Never;
    }
    let total = |m: f64| fixed + shapes.iter().map(|s| s.nock_at(m)).sum::<f64>();
    // Bracket: past every range's edge, and past the full-range tail's own closed form
    // (A / (level - fixed))^2, the total is below the level.
    let a_sum: f64 = shapes.iter().map(|s| s.full_range_nock()).sum();
    let mut top = 1.0f64;
    for e in shapes.iter().filter_map(|s| s.edge()) {
        top = top.max(e);
    }
    if a_sum > 0.0 {
        top = top.max((a_sum / (level - fixed)).powi(2));
    }
    let (mut lo, mut hi) = (0.0f64, (2.0 * top).ln());
    for _ in 0..BISECTION_STEPS {
        let mid = (lo + hi) / 2.0;
        if total(mid.exp()) < level {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    Crossing::Crosses(hi.exp())
}

/// `crossing_multiple` over plain positions: `fixed_nock` is W + F and the positions are
/// NOCK pairs. Err when one cannot be evaluated. For tests and #166's simulations.
#[allow(dead_code)] // tests and #166
pub(crate) fn crossing_multiple(fixed_nock: f64, positions: &[&RawPosition], level: f64) -> Result<Crossing, String> {
    let shapes: Vec<Shape> = positions.iter().map(|p| shape(p)).collect::<Result<_, _>>()?;
    let now = fixed_nock + positions.iter().map(|p| nock_amount(p).unwrap_or(0.0)).sum::<f64>();
    Ok(solve(fixed_nock, now, &shapes, level))
}

/// One position's figures, unformatted.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PositionCalc {
    pub(crate) position_id: String,
    pub(crate) label: String,
    pub(crate) full_range: bool,
    pub(crate) nock_now: f64,
    pub(crate) nock_at_crossing: Option<f64>,
    pub(crate) sold_out_at_multiple: Option<f64>,
    pub(crate) note: Option<String>,
}

/// The whole figure, unformatted (tests read these to full precision).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BreachCalc {
    pub(crate) spot: f64,
    pub(crate) floor: f64,
    pub(crate) wallet: f64,
    pub(crate) fees: f64,
    pub(crate) nock_now: f64,
    pub(crate) hard: Crossing,
    /// (level, crossing) when a tolerance is set.
    pub(crate) tolerated: Option<(f64, Crossing)>,
    pub(crate) positions: Vec<PositionCalc>,
    pub(crate) largest_seller: Option<String>,
    pub(crate) first_to_sell_out: Option<String>,
}

/// Ok(None) when no floor is set; Err(reason) when the figure cannot be stated.
/// `extra` are hypothetical ranges added to the tracked ones (#166).
pub(crate) fn floor_breach_calc(raw: &RawOnChainData, profile: &OperatorProfile, extra: &[RawPosition]) -> Result<Option<BreachCalc>, String> {
    let floor = profile.treasury_floor_nock;
    if !floor.is_finite() || floor <= 0.0 {
        return Ok(None);
    }
    let spot = raw.spot_price_nock_usd;
    if !spot.is_finite() || spot <= 0.0 {
        return Err("no NOCK/USD spot this cycle".to_string());
    }
    // The pool's in-range liquidity, keyed by position_id. A hypothetical range has no
    // entry of its own; it reads the entry of a tracked position on the same pool.
    let activity = |p: &RawPosition| -> (Option<u128>, bool) {
        let parse = |id: &str| raw.pool_liquidity.get(id).and_then(|v| v.parse::<u128>().ok());
        if raw.pool_liquidity.contains_key(&p.position_id) {
            return (parse(&p.position_id), true);
        }
        let sibling = raw
            .positions
            .iter()
            .find(|t| t.pool_id == p.pool_id && t.protocol == p.protocol && raw.pool_liquidity.contains_key(&t.position_id));
        match sibling {
            Some(t) => (parse(&t.position_id), true),
            None => (None, raw.positions.iter().any(|t| t.position_id == p.position_id)),
        }
    };
    let mut shapes: Vec<Shape> = Vec::new();
    let mut positions: Vec<PositionCalc> = Vec::new();
    for p in raw.positions.iter().chain(extra.iter()).filter(|p| is_nock_pair(p)) {
        let label = position_label(p);
        let sh = shape(p)?;
        let full_range = matches!(sh, Shape::Full { .. });
        let nock_now = match sh {
            Shape::Empty => 0.0,
            _ => nock_amount(p).unwrap_or(0.0),
        };
        if !nock_now.is_finite() {
            return Err(format!("{label}: its NOCK amount could not be read ({nock_now})"));
        }
        let mut note: Option<String> = None;
        if let Shape::Range { .. } = sh {
            let (liq, tracked) = activity(p);
            if nock_now > 0.0 && liq == Some(0) {
                return Err(format!(
                    "{label}: its pool has no in-range liquidity, so its price is frozen and does not say where this range's NOCK sells"
                ));
            }
            if nock_now <= 0.0 {
                note = Some("holds no NOCK at this price".to_string());
            } else if liq.is_none() {
                note = Some(if tracked {
                    "pool activity not read this cycle".to_string()
                } else {
                    "hypothetical range: no pool activity read for its pool".to_string()
                });
            }
        } else if let Shape::Empty = sh {
            note = Some("empty".to_string());
        } else {
            note = Some("full range: sells gradually as price rises and never empties".to_string());
        }
        let sold_out_at_multiple = if nock_now > 0.0 { nock_sell_out_multiple(p) } else { None };
        shapes.push(sh);
        positions.push(PositionCalc {
            position_id: p.position_id.clone(),
            label,
            full_range,
            nock_now,
            nock_at_crossing: None,
            sold_out_at_multiple,
            note,
        });
    }
    let wallet = wallet_nock(raw);
    let fees: f64 = raw
        .positions
        .iter()
        .map(|p| {
            (if is_nock_token(&p.token0) { p.fee0 } else { 0.0 }) + (if is_nock_token(&p.token1) { p.fee1 } else { 0.0 })
        })
        .sum();
    let fixed = wallet + fees;
    // The same sum as total_nock (advice.rs): LP NOCK + wallet NOCK + NOCK fees.
    let lp_now: f64 = positions.iter().map(|p| p.nock_now).sum();
    let nock_now = lp_now + wallet + fees;
    let hard = solve(fixed, nock_now, &shapes, floor);
    let tolerance = floor * (profile.lp_floor_tolerance_bps as f64 / 10000.0);
    let tolerated = (profile.lp_floor_tolerance_bps > 0).then(|| {
        let level = floor - tolerance;
        (level, solve(fixed, nock_now, &shapes, level))
    });
    let mut largest_seller: Option<(f64, String)> = None;
    if let Crossing::Crosses(m) = hard {
        for (pc, sh) in positions.iter_mut().zip(shapes.iter()) {
            let at = sh.nock_at(m);
            pc.nock_at_crossing = Some(at);
            let sold = pc.nock_now - at;
            if sold > 0.0 && largest_seller.as_ref().map_or(true, |(best, _)| sold > *best) {
                largest_seller = Some((sold, pc.label.clone()));
            }
        }
    }
    let mut first: Option<(f64, String)> = None;
    for pc in &positions {
        if let Some(x) = pc.sold_out_at_multiple {
            if first.as_ref().map_or(true, |(best, _)| x < *best) {
                first = Some((x, pc.label.clone()));
            }
        }
    }
    Ok(Some(BreachCalc {
        spot,
        floor,
        wallet,
        fees,
        nock_now,
        hard,
        tolerated,
        positions,
        largest_seller: largest_seller.map(|(_, l)| l),
        first_to_sell_out: first.map(|(_, l)| l),
    }))
}

fn crossing_fields(level: f64, c: Crossing, spot: f64) -> FloorCrossing {
    let (status, m) = match c {
        Crossing::AlreadyBelow => ("already_below", Some(1.0)),
        Crossing::Never => ("never", None),
        Crossing::Crosses(m) => ("crosses", Some(m)),
    };
    FloorCrossing {
        level_nock: format!("{:.0}", level),
        status: status.to_string(),
        multiple: m.map(|m| format!("{:.4}", m)),
        price_usd: m.map(|m| format!("{:.6}", spot * m)),
        pct_above_spot: m.map(|m| format!("{:.1}", (m - 1.0) * 100.0)),
    }
}

/// The snapshot's `derived_metrics.floor_breach`. Pure.
pub(crate) fn floor_breach(raw: &RawOnChainData, profile: &OperatorProfile) -> FloorBreach {
    floor_breach_with(raw, profile, &[])
}

/// The same figure with hypothetical ranges added to the tracked ones: for #166's
/// "effect on the floor-crossing price" of a range not yet deployed. Pure.
pub(crate) fn floor_breach_with(raw: &RawOnChainData, profile: &OperatorProfile, extra: &[RawPosition]) -> FloorBreach {
    let floor_nock = format!("{:.0}", profile.treasury_floor_nock);
    let base = FloorBreach { floor_nock, assumption: FLOOR_BREACH_ASSUMPTION.to_string(), ..Default::default() };
    let c = match floor_breach_calc(raw, profile, extra) {
        Ok(Some(c)) => c,
        Ok(None) => return FloorBreach { status: "no_floor".to_string(), ..base },
        Err(reason) => return FloorBreach { status: "unavailable".to_string(), reason: Some(reason), ..base },
    };
    let crossing = crossing_fields(c.floor, c.hard, c.spot);
    let fmt_m = |m: f64| format!("{:.4}", m);
    FloorBreach {
        status: crossing.status.clone(),
        spot_price_usd: Some(format!("{:.6}", c.spot)),
        crossing: Some(crossing),
        tolerated: c.tolerated.map(|(level, t)| crossing_fields(level, t, c.spot)),
        nock_now: Some(format!("{:.0}", c.nock_now)),
        wallet_nock: Some(format!("{:.0}", c.wallet)),
        fees_nock: Some(format!("{:.0}", c.fees)),
        nock_if_all_sold: Some(format!("{:.0}", c.wallet + c.fees)),
        largest_seller: c.largest_seller.clone(),
        first_to_sell_out: c.first_to_sell_out.clone(),
        positions: c
            .positions
            .iter()
            .map(|p| FloorBreachPosition {
                position_id: p.position_id.clone(),
                label: p.label.clone(),
                full_range: p.full_range,
                nock_now: format!("{:.0}", p.nock_now),
                nock_at_crossing: p.nock_at_crossing.map(|n| format!("{:.0}", n)),
                sold_out_at_multiple: p.sold_out_at_multiple.map(fmt_m),
                sold_out_at_price_usd: p.sold_out_at_multiple.map(|m| format!("{:.6}", c.spot * m)),
                note: p.note.clone(),
            })
            .collect(),
        ..base
    }
}
