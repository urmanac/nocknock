//! The ladder simulator (issue #166): a read-only what-if for a USDC/NOCK range that is
//! mostly USDC and sits below spot, so it buys NOCK a little at a time as the price
//! falls. It works only from the latest stored snapshot: it makes no RPC read, signs
//! nothing and sends nothing, and it never chooses an amount. Every figure it states is
//! measured from the snapshot plus the caller's own inputs; an unknown is null with a
//! stated reason.
//!
//! Units: prices are USD (USDC) per NOCK. The range is worked in the pool's raw sqrt-price
//! space with `concentrated_amounts_at`, the same body the adapters size positions with,
//! so token order and decimals are read from the pool and never assumed.
//!
//! Every pool moves by the same ratio `r` from its own current price (the same rule as
//! the floor-crossing price, #165). Row 0 of the grid (r = 1) is therefore the snapshot.
use crate::*;
use serde_json::Value;

/// A provided NOCK amount matches the range's need when within the larger of 1 NOCK and
/// 10 bps of it: the Uniswap app displays rounded amounts.
pub(crate) const LADDER_NOCK_MATCH_TOLERANCE_NOCK: f64 = 1.0;
pub(crate) const LADDER_NOCK_MATCH_TOLERANCE_BPS: f64 = 10.0;
/// How deep the grid goes and its step, in % of spot, when the caller does not say. A
/// display depth, not an operator number; both are echoed in `inputs`.
pub(crate) const LADDER_GRID_DEFAULT_TO_PCT: f64 = -40.0;
pub(crate) const LADDER_GRID_DEFAULT_STEP_PCT: f64 = 5.0;
pub(crate) const LADDER_GRID_MAX_ROWS: usize = 60;
/// NOCK below this at spot is float noise from a sqrt round trip, not a holding: it is
/// treated as exactly 0 wherever the NOCK a ladder needs decides an outcome (#166 review).
pub(crate) const LADDER_NOCK_DUST: f64 = 1e-9;
/// The id the hypothetical range carries inside the floor-crossing figure.
pub(crate) const LADDER_POSITION_ID: &str = "hypothetical:ladder";
/// What the result is, stated in every body.
pub(crate) const LADDER_STATEMENT: &str = "A what-if from the stored snapshot. NockNock reads nothing new, signs nothing, sends nothing, and does not choose the amounts.";
/// The assumptions the figures rest on.
pub(crate) const LADDER_ASSUMPTIONS: [&str; 5] = [
    "Every NOCK pool moves by the same percentage as spot, each from its own current price; row 0 of the grid is the snapshot.",
    "Fees and gas are not counted.",
    "WETH/NOCK ranges hold ETH/USD constant and add no USDC.",
    "Gas-side ranges with no NOCK leg (ETH/USDC, #151) are left out of the stable balance and the NOCK totals.",
    "The ladder's USDC comes from the wallet and its NOCK from wallet NOCK.",
];
const RESERVE_GATE_NOTE: &str = "The gate is the stable balance at the ladder's own bottom (operator decision, 2026-11-20).";
const BELOW_BOTTOM_NOTE: &str = "The existing NOCK ranges keep converting below the ladder's bottom: a measurement, not the gate.";
const NOT_A_GATE: &str = "stated, not a gate";
const POOL_NEW_NFT_NOTE: &str = "a ladder in this pool is a NEW position (a new NFT); add it to config.yaml to be tracked";

/// One bound as the caller gave it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Bound {
    /// USD per NOCK.
    Usd(f64),
    /// Percent of the ladder pool's current price (spot, unless `position_id` names
    /// another pool), e.g. -30 for 30% below.
    Pct(f64),
    /// The ladder pool's current price (`high=spot`): on a snapped grid, the usable tick
    /// at or below it, so the range holds no NOCK.
    Spot,
}

/// The caller's inputs, validated.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LadderRequest {
    pub(crate) usdc: f64,
    pub(crate) nock: Option<f64>,
    pub(crate) low: Bound,
    pub(crate) high: Bound,
    pub(crate) position_id: Option<String>,
    pub(crate) tick_spacing: Option<i32>,
    pub(crate) grid_to_pct: f64,
    pub(crate) grid_step_pct: f64,
}

/// Why no simulation is returned: 400 (an input is wrong), 422 (the inputs are valid but
/// this range cannot be simulated as given, `code` says why) or 503 (no snapshot).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LadderError {
    pub(crate) status: u16,
    pub(crate) error: &'static str,
    pub(crate) code: Option<&'static str>,
    pub(crate) detail: String,
    pub(crate) nock_needed: Option<f64>,
    pub(crate) bounds: Option<Value>,
}

impl LadderError {
    pub(crate) fn invalid(detail: impl Into<String>) -> Self {
        LadderError { status: 400, error: "INVALID_PARAMETER", code: None, detail: detail.into(), nock_needed: None, bounds: None }
    }
    fn not_simulated(code: &'static str, detail: impl Into<String>) -> Self {
        LadderError { status: 422, error: "LADDER_NOT_SIMULATED", code: Some(code), detail: detail.into(), nock_needed: None, bounds: None }
    }
    pub(crate) fn body(&self) -> Value {
        let mut b = json!({ "error": self.error, "detail": self.detail });
        if let Some(c) = self.code {
            b["code"] = json!(c);
        }
        if let Some(n) = self.nock_needed {
            b["nock_needed"] = json!(fx(n, 4));
        }
        if let Some(bounds) = &self.bounds {
            b["bounds"] = bounds.clone();
        }
        b
    }
}

/// Fixed decimals, rounded, and never "-0.00". Pure.
fn fx(v: f64, dp: usize) -> String {
    let m = 10f64.powi(dp as i32);
    let r = (v * m).round() / m + 0.0;
    format!("{:.*}", dp, r)
}

/// A signed percentage with its sign: "+1.44", "-22.56", "0.00". Pure.
fn pct(v: f64) -> String {
    let s = fx(v, 2);
    if s.starts_with('-') || s == "0.00" { s } else { format!("+{s}") }
}

/// Whole cents, rounded down: the most that still fits. Pure.
fn floor_cents(v: f64) -> f64 {
    ((v * 100.0 + 1e-7).floor() / 100.0).max(0.0) + 0.0
}

const LADDER_PARAMS: [&str; 10] =
    ["usdc", "nock", "low", "low_pct", "high", "high_pct", "position_id", "tick_spacing", "grid_to_pct", "grid_step_pct"];

impl LadderRequest {
    /// The GET /v1/simulate/ladder query (and the MCP arguments, normalised to the same
    /// names by `ladder_text::ladder_query_from_args`). Pure.
    pub(crate) fn from_params(params: &HashMap<String, String>) -> Result<LadderRequest, LadderError> {
        let mut keys: Vec<&String> = params.keys().collect();
        keys.sort();
        for k in keys {
            if !LADDER_PARAMS.contains(&k.as_str()) {
                return Err(LadderError::invalid(format!(
                    "unknown parameter '{k}'; the parameters are {}",
                    LADDER_PARAMS.join(", ")
                )));
            }
        }
        let get = |k: &str| params.get(k).map(|v| v.trim()).filter(|v| !v.is_empty());
        let num = |k: &str| -> Result<Option<f64>, LadderError> {
            match get(k) {
                None => Ok(None),
                Some(v) => match v.parse::<f64>() {
                    Ok(x) if x.is_finite() => Ok(Some(x)),
                    _ => Err(LadderError::invalid(format!("{k} must be a finite number, not '{v}'"))),
                },
            }
        };
        let usdc = num("usdc")?.ok_or_else(|| LadderError::invalid("usdc is required: the USDC the ladder would hold at spot"))?;
        if !(usdc > 0.0) {
            return Err(LadderError::invalid(format!("usdc must be above 0 (nothing to size): {usdc}")));
        }
        let nock = num("nock")?;
        if let Some(n) = nock {
            if n < 0.0 {
                return Err(LadderError::invalid(format!("nock must be 0 or more: {n}")));
            }
        }
        let low = match (num("low")?, num("low_pct")?) {
            (Some(_), Some(_)) => return Err(LadderError::invalid("give low or low_pct, not both")),
            (None, None) => return Err(LadderError::invalid("low or low_pct is required: the bottom of the range")),
            (Some(v), None) if v > 0.0 => Bound::Usd(v),
            (Some(v), None) => return Err(LadderError::invalid(format!("low must be above 0 USD per NOCK: {v}"))),
            (None, Some(p)) if p > -100.0 => Bound::Pct(p),
            (None, Some(p)) => return Err(LadderError::invalid(format!("low_pct must be above -100: {p}"))),
        };
        let high_spot = get("high").map(|v| v.eq_ignore_ascii_case("spot")).unwrap_or(false);
        let high_num = if high_spot { None } else { num("high")? };
        let high = match (high_spot, high_num, num("high_pct")?) {
            (true, _, Some(_)) | (false, Some(_), Some(_)) => return Err(LadderError::invalid("give high or high_pct, not both")),
            (true, _, None) => Bound::Spot,
            (false, Some(v), None) if v > 0.0 => Bound::Usd(v),
            (false, Some(v), None) => return Err(LadderError::invalid(format!("high must be above 0 USD per NOCK: {v}"))),
            (false, None, Some(p)) if p > -100.0 => Bound::Pct(p),
            (false, None, Some(p)) => return Err(LadderError::invalid(format!("high_pct must be above -100: {p}"))),
            (false, None, None) => {
                return Err(LadderError::invalid("high, high=spot or high_pct is required: the top of the range"))
            }
        };
        match (&low, &high) {
            (Bound::Usd(l), Bound::Usd(h)) if l >= h => {
                return Err(LadderError::invalid(format!("low ({l}) must be below high ({h})")))
            }
            (Bound::Pct(l), Bound::Pct(h)) if l >= h => {
                return Err(LadderError::invalid(format!("low_pct ({l}) must be below high_pct ({h})")))
            }
            _ => {}
        }
        let position_id = get("position_id").map(|s| s.to_string());
        let tick_spacing = match get("tick_spacing") {
            None => None,
            Some(v) => match v.parse::<i32>() {
                Ok(s) if (1..=32767).contains(&s) => Some(s),
                _ => return Err(LadderError::invalid(format!("tick_spacing must be a whole number from 1 to 32767, not '{v}'"))),
            },
        };
        if position_id.is_some() && tick_spacing.is_some() {
            return Err(LadderError::invalid(
                "give position_id (snap to that tracked pool) or tick_spacing (snap to a spacing you state), not both",
            ));
        }
        let grid_to_pct = num("grid_to_pct")?.unwrap_or(LADDER_GRID_DEFAULT_TO_PCT);
        if !(-95.0..=-1.0).contains(&grid_to_pct) {
            return Err(LadderError::invalid(format!("grid_to_pct must be from -95 to -1: {grid_to_pct}")));
        }
        let grid_step_pct = num("grid_step_pct")?.unwrap_or(LADDER_GRID_DEFAULT_STEP_PCT);
        if !(0.5..=20.0).contains(&grid_step_pct) {
            return Err(LadderError::invalid(format!("grid_step_pct must be from 0.5 to 20: {grid_step_pct}")));
        }
        let rows = (-grid_to_pct / grid_step_pct + 1e-9).floor() as usize + 1;
        if rows > LADDER_GRID_MAX_ROWS {
            return Err(LadderError::invalid(format!(
                "grid_to_pct {grid_to_pct} at grid_step_pct {grid_step_pct} makes {rows} rows; at most {LADDER_GRID_MAX_ROWS}"
            )));
        }
        Ok(LadderRequest { usdc, nock, low, high, position_id, tick_spacing, grid_to_pct, grid_step_pct })
    }
}

// ------------------------------------------------------------------
// The stored snapshot as reads
// ------------------------------------------------------------------

/// The snapshot's NOCK-pair positions rebuilt as reads, so the same pure code that made the
/// snapshot (`concentrated_amounts_at`, `floor_breach_with`) runs on them. Only what the
/// snapshot states is used: amounts and fees as printed, liquidity, ticks and the pool's
/// exact sqrtPriceX96 from `pool_mechanics`.
pub(crate) struct StoredBook {
    pub(crate) raw: RawOnChainData,
    /// NOCK-pair positions that could not be rebuilt, with the reason.
    pub(crate) problems: Vec<String>,
}

/// The pool row of position `i`: the same index when it is the same pool (both arrays are
/// built from one list), otherwise the first row of that pool.
fn pool_row<'a>(pool_mechanics: &'a [PoolMechanic], i: usize, pool_id: &str) -> Option<&'a PoolMechanic> {
    pool_mechanics
        .get(i)
        .filter(|m| m.pool_id == pool_id)
        .or_else(|| pool_mechanics.iter().find(|m| m.pool_id == pool_id))
}

fn position_from_snapshot(pool_mechanics: &[PoolMechanic], i: usize, p: &Position) -> Result<RawPosition, String> {
    let label = format!("{} #{}", p.protocol, p.nft_id);
    let unread = || format!("position {} ({label}): its pool price is not in this snapshot", p.position_id);
    let m = pool_row(pool_mechanics, i, &p.pool_id).ok_or_else(unread)?;
    let sqrt_price_x96 = m.sqrt_price_x96.as_deref().and_then(|h| U256::from_hex_be(h).ok()).ok_or_else(unread)?;
    let tick = m.tick_current.ok_or_else(unread)?;
    let liquidity = p
        .liquidity_amount
        .parse::<u128>()
        .map_err(|_| format!("position {}: its liquidity '{}' could not be read", p.position_id, p.liquidity_amount))?;
    let amt = |s: &str, what: &str| -> Result<f64, String> {
        s.trim().parse::<f64>().ok().filter(|v| v.is_finite()).ok_or_else(|| format!("position {}: its {what} '{s}' could not be read", p.position_id))
    };
    Ok(RawPosition {
        position_id: p.position_id.clone(),
        network: p.network.clone(),
        protocol: p.protocol.clone(),
        wallet: p.wallet.clone(),
        nft_id: p.nft_id,
        pool_id: p.pool_id.clone(),
        npm_address: p.npm_address.clone(),
        fee_tier_bps: p.fee_tier_bps,
        token0: p.token0.clone(),
        token1: p.token1.clone(),
        liquidity,
        sqrt_price_x96,
        tick,
        tick_lower: p.lower_tick,
        tick_upper: p.upper_tick,
        amount0: amt(&p.amount0, "amount0")?,
        amount1: amt(&p.amount1, "amount1")?,
        fee0: amt(&p.tokens_owed_0, "tokens_owed_0")?,
        fee1: amt(&p.tokens_owed_1, "tokens_owed_1")?,
        in_range: p.in_range,
        is_price_source: p.is_price_source,
    })
}

/// Rebuild the stored book. Gas-side ranges with no NOCK leg (#151) are left out: they
/// hold no NOCK and their USDC is not the stable balance the reserve is measured on. Pure.
pub(crate) fn stored_book(data: &SnapshotData) -> StoredBook {
    stored_book_from(&data.positions, &data.pool_mechanics, &data.wallet_balances)
}

/// `stored_book` over the three arrays it reads. Pure.
pub(crate) fn stored_book_from(snapshot_positions: &[Position], pool_mechanics: &[PoolMechanic], wallet_balances: &[WalletBalance]) -> StoredBook {
    let mut positions = Vec::new();
    let mut problems = Vec::new();
    let mut pool_liquidity = HashMap::new();
    for (i, p) in snapshot_positions.iter().enumerate() {
        if !(is_nock_token(&p.token0) || is_nock_token(&p.token1)) {
            continue;
        }
        match position_from_snapshot(pool_mechanics, i, p) {
            Ok(raw) => {
                // The snapshot states only whether the pool had in-range liquidity
                // (`pool_active`); the floor-crossing figure reads only zero vs not zero,
                // so a live pool is entered as 1 and a frozen one as 0. Unknown stays absent.
                match p.pool_active {
                    Some(true) => {
                        pool_liquidity.insert(p.position_id.clone(), "1".to_string());
                    }
                    Some(false) => {
                        pool_liquidity.insert(p.position_id.clone(), "0".to_string());
                    }
                    None => {}
                }
                positions.push(raw);
            }
            Err(e) => problems.push(e),
        }
    }
    let spot = nock_spot_from_positions(&positions).unwrap_or(0.0);
    StoredBook {
        raw: RawOnChainData {
            wallet_balances: wallet_balances.to_vec(),
            positions,
            spot_price_nock_usd: spot,
            pool_liquidity,
            // A tracked ladder (#167) keeps its role: it is an existing NOCK range here,
            // its USDC in the stable balance and its NOCK in the floor-crossing figure.
            ladders: ladders_from_snapshot(snapshot_positions),
            ..Default::default()
        },
        problems,
    }
}

/// USD per NOCK in this USDC/NOCK position's pool, from its exact sqrtPriceX96. Pure.
fn pool_usd_price(p: &RawPosition) -> Option<f64> {
    let mut q = p.clone();
    q.is_price_source = true;
    nock_spot_from_positions(std::slice::from_ref(&q))
}

/// One NOCK-pair position's (USDC, NOCK) in human units when every pool has moved by the
/// ratio `r` from its own price. Full range: x*y=k on the wallet's share. Pure.
pub(crate) fn existing_at_ratio(p: &RawPosition, r: f64) -> (f64, f64) {
    let usdc_is_0 = is_usdc_token(&p.token0);
    let usdc_is_1 = is_usdc_token(&p.token1);
    let nock_is_1 = is_nock_token(&p.token1);
    if is_full_range(p.tick_lower, p.tick_upper) {
        let usdc_now = if usdc_is_0 { p.amount0 } else if usdc_is_1 { p.amount1 } else { 0.0 };
        let nock_now = if nock_is_1 { p.amount1 } else { p.amount0 };
        return (usdc_now * r.sqrt(), nock_now / r.sqrt());
    }
    if p.liquidity == 0 {
        return (0.0, 0.0);
    }
    let s0 = sqrt_price_x96_to_raw(p.sqrt_price_x96).sqrt();
    // NOCK as token1: the raw price is NOCK per other token, so NOCK getting dearer moves
    // s down; as token0 it moves s up.
    let s = if nock_is_1 { s0 / r.sqrt() } else { s0 * r.sqrt() };
    let (a0, a1) = concentrated_amounts_at(p.liquidity as f64, s, tick_sqrt_price(p.tick_lower), tick_sqrt_price(p.tick_upper));
    let (h0, h1) = (a0 / 10f64.powi(p.token0.decimals), a1 / 10f64.powi(p.token1.decimals));
    let usdc = if usdc_is_0 { h0 } else if usdc_is_1 { h1 } else { 0.0 };
    let nock = if nock_is_1 { h1 } else { h0 };
    (usdc, nock)
}

// ------------------------------------------------------------------
// The ladder's own range
// ------------------------------------------------------------------

/// The ladder's pool: its orientation and its price now.
struct LadderPool {
    template: RawPosition,
    nock_is_token1: bool,
    usdc_dec: i32,
    nock_dec: i32,
    /// Raw sqrt price now (sqrt(token1_raw / token0_raw)).
    s_now: f64,
    /// USD per NOCK in this pool now.
    price: f64,
}

impl LadderPool {
    /// raw = token1_raw per token0_raw at USD price p.
    fn usd_to_raw(&self, p: f64) -> f64 {
        let d = (self.template.token1.decimals - self.template.token0.decimals) as i32;
        if self.nock_is_token1 { 10f64.powi(d) / p } else { p * 10f64.powi(d) }
    }
    fn raw_to_usd(&self, raw: f64) -> f64 {
        let d = (self.template.token1.decimals - self.template.token0.decimals) as i32;
        if self.nock_is_token1 { 10f64.powi(d) / raw } else { raw / 10f64.powi(d) }
    }
    fn tick_f(&self, p: f64) -> f64 {
        self.usd_to_raw(p).ln() / 1.0001f64.ln()
    }
    fn usd_at_tick(&self, t: i32) -> f64 {
        self.raw_to_usd(1.0001f64.powi(t))
    }
    /// The raw sqrt price when this pool has moved by the ratio r (USD terms).
    fn s_at(&self, r: f64) -> f64 {
        if self.nock_is_token1 { self.s_now / r.sqrt() } else { self.s_now * r.sqrt() }
    }
    /// (USDC, NOCK) in human units for liquidity l at raw sqrt price s.
    fn amounts(&self, l: f64, s: f64, sa: f64, sb: f64) -> (f64, f64) {
        let (a0, a1) = concentrated_amounts_at(l, s, sa, sb);
        if self.nock_is_token1 {
            (a0 / 10f64.powi(self.usdc_dec), a1 / 10f64.powi(self.nock_dec))
        } else {
            (a1 / 10f64.powi(self.usdc_dec), a0 / 10f64.powi(self.nock_dec))
        }
    }
}

/// The range in raw sqrt space plus its USD bounds.
#[derive(Debug, Clone, Copy)]
struct Range {
    low_usd: f64,
    high_usd: f64,
    /// Raw sqrt edges, sa < sb.
    sa: f64,
    sb: f64,
    /// The raw sqrt edge at the USD bottom and at the USD top.
    s_bottom: f64,
    s_top: f64,
    /// Snapped ticks (tick_lower < tick_upper), None when unsnapped.
    ticks: Option<(i32, i32)>,
}

fn usable_extremes(spacing: i32) -> (i32, i32) {
    let lo = (MIN_TICK as f64 / spacing as f64).ceil() as i32 * spacing;
    let hi = (MAX_TICK as f64 / spacing as f64).floor() as i32 * spacing;
    (lo, hi)
}

/// round(t / s) * s, half away from zero: the Uniswap SDK's nearestUsableTick. Pure.
pub(crate) fn nearest_usable_tick(t: f64, spacing: i32) -> i32 {
    ((t / spacing as f64).round() as i64 * spacing as i64) as i32
}

/// A multiple of `spacing` next to `t` on one side; a tick that already is a multiple
/// (to 1e-9 of a spacing) is itself. Pure.
fn usable_tick_toward(t: f64, spacing: i32, up: bool) -> i32 {
    let q = t / spacing as f64;
    let n = if (q - q.round()).abs() < 1e-9 { q.round() } else if up { q.ceil() } else { q.floor() };
    (n as i64 * spacing as i64) as i32
}

impl LadderPool {
    fn range_from_ticks(&self, t_low_usd: i32, t_high_usd: i32) -> Range {
        let (tl, tu) = (t_low_usd.min(t_high_usd), t_low_usd.max(t_high_usd));
        let (sa, sb) = (tick_sqrt_price(tl), tick_sqrt_price(tu));
        let (s_bottom, s_top) = if self.nock_is_token1 { (sb, sa) } else { (sa, sb) };
        Range {
            low_usd: self.usd_at_tick(t_low_usd),
            high_usd: self.usd_at_tick(t_high_usd),
            sa,
            sb,
            s_bottom,
            s_top,
            ticks: Some((tl, tu)),
        }
    }
    fn range_unsnapped(&self, low: f64, high: f64) -> Range {
        let (s_low, s_high) = (self.usd_to_raw(low).sqrt(), self.usd_to_raw(high).sqrt());
        Range {
            low_usd: low,
            high_usd: high,
            sa: s_low.min(s_high),
            sb: s_low.max(s_high),
            s_bottom: s_low,
            s_top: s_high,
            ticks: None,
        }
    }
    /// The usable tick at or below the pool price in USD terms (`high=spot`).
    fn tick_at_or_below_price(&self, spacing: i32) -> i32 {
        // A higher USD price is a lower tick when NOCK is token1.
        usable_tick_toward(self.tick_f(self.price), spacing, self.nock_is_token1)
    }
    /// The next usable tick above `t` in USD terms.
    fn tick_above(&self, t: i32, spacing: i32) -> i32 {
        if self.nock_is_token1 { t - spacing } else { t + spacing }
    }
    /// Liquidity for `usdc` at the pool's price, the NOCK it then needs, or None when
    /// the range holds no USDC at this price (it is entirely above).
    fn size(&self, range: &Range, usdc: f64) -> Option<(f64, f64)> {
        let (u1, n1) = self.amounts(1.0, self.s_now, range.sa, range.sb);
        if !(u1 > 0.0) || !u1.is_finite() {
            return None;
        }
        let l = usdc / u1;
        Some((l, l * n1))
    }
}

fn bounds_json(range: &Range, price: f64) -> Value {
    json!({
        "low_usd": fx(range.low_usd, 8),
        "high_usd": fx(range.high_usd, 8),
        "low_pct": pct((range.low_usd / price - 1.0) * 100.0),
        "high_pct": pct((range.high_usd / price - 1.0) * 100.0),
        "tick_lower": range.ticks.map(|t| t.0),
        "tick_upper": range.ticks.map(|t| t.1),
    })
}

fn bound_text(b: &Bound) -> Value {
    match b {
        Bound::Usd(v) => json!(format!("{v}")),
        Bound::Pct(p) => json!(format!("{p}%")),
        Bound::Spot => json!("spot"),
    }
}

/// The reserve reading of one price: standing_reserve over (wallet USDC less the ladder's,
/// the ladder's USDC there plus the existing NOCK ranges' USDC there).
fn reserve_reading(configured: Option<f64>, wallet_after: f64, lp_usdc: f64) -> (f64, Option<f64>) {
    match standing_reserve(configured, wallet_after, lp_usdc) {
        StandingReserve::Set { stable, headroom, .. } => (stable, Some(headroom)),
        _ => (wallet_after + lp_usdc, None),
    }
}

/// The simulation. Pure: the stored snapshot, the caller's inputs, the time now and the
/// worker's health verdict (both only stated). No I/O and no clock.
pub(crate) fn simulate_ladder(env: &SnapshotEnvelope, req: &LadderRequest, now: DateTime<Utc>, health: &str) -> Result<Value, LadderError> {
    let data = &env.data;
    let gs = &data.derived_metrics.global_status;
    let book = stored_book(data);
    let raw = &book.raw;

    // Spot: the price-source pool's exact sqrtPriceX96, never the 5-dp price series.
    let source = data.positions.iter().find(|p| p.is_price_source);
    let source_raw = raw.positions.iter().find(|p| p.is_price_source);
    let spot = match (source, source_raw) {
        (Some(_), Some(r)) => pool_usd_price(r)
            .ok_or_else(|| LadderError::not_simulated("SPOT_UNAVAILABLE", "the price-source pool's price could not be read from this snapshot"))?,
        (Some(p), None) => {
            return Err(LadderError::not_simulated(
                "SPOT_UNAVAILABLE",
                book.problems.iter().find(|e| e.contains(&p.position_id)).cloned().unwrap_or_else(|| "the price-source pool is not in this snapshot".to_string()),
            ))
        }
        _ => {
            return Err(LadderError::not_simulated(
                "SPOT_UNAVAILABLE",
                "no live price-source position is in this snapshot, so there is no exact spot to simulate from",
            ))
        }
    };
    let source_raw = source_raw.expect("checked above");

    // The pool the ladder would sit in, and where its tick spacing comes from.
    let (template, spacing, spacing_source, named): (RawPosition, Option<i32>, Option<&str>, bool) = match (&req.position_id, req.tick_spacing) {
        (Some(id), _) => {
            let Some(p) = data.positions.iter().find(|p| &p.position_id == id) else {
                if data.closed_positions.iter().any(|c| &c.position_id == id || &c.config_id == id) {
                    return Err(LadderError::invalid(format!("position {id} is closed (#144): it has no pool price in this snapshot")));
                }
                return Err(LadderError::invalid(format!("position_id {id} is not a tracked position in this snapshot")));
            };
            let usdc_nock = (is_usdc_token(&p.token0) && is_nock_token(&p.token1)) || (is_nock_token(&p.token0) && is_usdc_token(&p.token1));
            if !usdc_nock {
                return Err(LadderError::not_simulated(
                    "POOL_NOT_SNAPPABLE",
                    format!("{id} is a {}/{} pool; a ladder is a USDC/NOCK range", p.token0.symbol, p.token1.symbol),
                ));
            }
            if p.full_range {
                return Err(LadderError::not_simulated(
                    "POOL_NOT_SNAPPABLE",
                    format!("{id} is a basic Aero pool: it has no ranges, its deposits are full range"),
                ));
            }
            let Some(s) = p.tick_spacing else {
                return Err(LadderError::not_simulated(
                    "POOL_NOT_SNAPPABLE",
                    format!("{id}: tick spacing not read for this pool in this snapshot; give tick_spacing instead, or leave the range unsnapped"),
                ));
            };
            let Some(t) = raw.positions.iter().find(|r| &r.position_id == id) else {
                return Err(LadderError::not_simulated(
                    "SPOT_UNAVAILABLE",
                    book.problems.iter().find(|e| e.contains(id.as_str())).cloned().unwrap_or_else(|| format!("{id}: its pool price is not in this snapshot")),
                ));
            };
            (t.clone(), Some(s), Some("position"), true)
        }
        (None, Some(s)) => (source_raw.clone(), Some(s), Some("caller"), false),
        (None, None) => (source_raw.clone(), None, None, false),
    };
    let nock_is_token1 = is_nock_token(&template.token1);
    let price = if named {
        pool_usd_price(&template).ok_or_else(|| LadderError::not_simulated("SPOT_UNAVAILABLE", "the named pool's price could not be read"))?
    } else {
        spot
    };
    let pool = LadderPool {
        nock_is_token1,
        usdc_dec: if nock_is_token1 { template.token0.decimals } else { template.token1.decimals },
        nock_dec: if nock_is_token1 { template.token1.decimals } else { template.token0.decimals },
        s_now: sqrt_price_x96_to_raw(template.sqrt_price_x96).sqrt(),
        price,
        template,
    };

    // Bounds in USD, then snapped.
    let low = match req.low {
        Bound::Usd(v) => v,
        Bound::Pct(p) => price * (1.0 + p / 100.0),
        Bound::Spot => price,
    };
    let high = match req.high {
        Bound::Usd(v) => v,
        Bound::Pct(p) => price * (1.0 + p / 100.0),
        Bound::Spot => price,
    };
    if low >= high {
        return Err(LadderError::invalid(format!("the bottom ({}) must be below the top ({})", fx(low, 8), fx(high, 8))));
    }
    let above_spot = |l: f64| {
        LadderError::not_simulated(
            "RANGE_ABOVE_SPOT",
            format!(
                "the bottom ${} is at or above the pool price ${}: a range above spot holds only NOCK and buys nothing on the way down (see the floor-crossing price, #165)",
                fx(l, 8),
                fx(price, 8)
            ),
        )
    };
    if low >= price {
        return Err(above_spot(low));
    }
    let (range, high_rule) = match spacing {
        Some(s) => {
            let (lo_ok, hi_ok) = usable_extremes(s);
            let clamp = |t: i32, what: &str| -> Result<i32, LadderError> {
                let c = t.clamp(lo_ok, hi_ok);
                if (c - t).abs() > s {
                    return Err(LadderError::invalid(format!("the {what} is outside the pool's tick range")));
                }
                Ok(c)
            };
            let t_low = clamp(nearest_usable_tick(pool.tick_f(low), s), "bottom")?;
            let (t_high, rule) = match req.high {
                Bound::Spot => (clamp(pool.tick_at_or_below_price(s), "top")?, "the usable tick at or below the pool price (high = spot): it holds no NOCK"),
                _ => (clamp(nearest_usable_tick(pool.tick_f(high), s), "top")?, "the nearest usable tick"),
            };
            if t_low == t_high {
                return Err(LadderError::not_simulated(
                    "RANGE_TOO_NARROW",
                    format!(
                        "both bounds snap to tick {t_low}: one spacing of {s} ticks is {}% of price, wider than this range",
                        fx((1.0001f64.powi(s) - 1.0) * 100.0, 2)
                    ),
                ));
            }
            (pool.range_from_ticks(t_low, t_high), rule)
        }
        None => {
            // A bound past the protocol's tick range is a range that cannot exist.
            for (what, p) in [("bottom", low), ("top", high)] {
                let t = pool.tick_f(p);
                if !t.is_finite() || t < MIN_TICK as f64 || t > MAX_TICK as f64 {
                    return Err(LadderError::not_simulated(
                        "RANGE_OUTSIDE_TICKS",
                        format!(
                            "the {what} ${} is at tick {}, outside the pool's tick range [{MIN_TICK}, {MAX_TICK}]: no such range can exist",
                            fx(p, 8),
                            fx(t, 2)
                        ),
                    ));
                }
            }
            let mut r = pool.range_unsnapped(low, high);
            if req.high == Bound::Spot {
                // The top is the pool's own price: its own sqrt, not a USD round trip
                // that can land one ulp off and leave dust NOCK in the range.
                r.s_top = pool.s_now;
                if pool.nock_is_token1 { r.sa = pool.s_now } else { r.sb = pool.s_now }
            }
            (r, "unsnapped")
        }
    };
    if range.low_usd >= price {
        return Err(above_spot(range.low_usd));
    }
    let bounds = bounds_json(&range, price);

    // The wallet.
    let wallet_usdc: f64 = data
        .wallet_balances
        .iter()
        .find(|w| w.token_symbol == "USDC")
        .and_then(|w| w.amount.parse().ok())
        .unwrap_or(0.0);
    let wallet_nock_now = wallet_nock(raw);
    if req.usdc > wallet_usdc + 1e-9 {
        let mut e = LadderError::not_simulated(
            "USDC_EXCEEDS_WALLET",
            format!("${} is more than the ${} of USDC in the wallet", fx(req.usdc, 2), fx(wallet_usdc, 2)),
        );
        e.bounds = Some(bounds);
        return Err(e);
    }

    // Sizing.
    let Some((l, n0)) = pool.size(&range, req.usdc) else {
        return Err(above_spot(range.low_usd));
    };
    let n0 = if n0 < LADDER_NOCK_DUST { 0.0 } else { n0 };
    let nock_check = match req.nock {
        None => "not_given",
        Some(n) => {
            let tol = LADDER_NOCK_MATCH_TOLERANCE_NOCK.max(n0 * LADDER_NOCK_MATCH_TOLERANCE_BPS / 10000.0);
            let wrong = if n < n0 - tol { Some("NOCK_TOO_LITTLE") } else if n > n0 + tol { Some("NOCK_TOO_MUCH") } else { None };
            if let Some(code) = wrong {
                let mut e = LadderError::not_simulated(
                    code,
                    format!("this range at the pool price takes {} NOCK with ${} USDC; {} was given", fx(n0, 4), fx(req.usdc, 2), n),
                );
                e.nock_needed = Some(n0);
                e.bounds = Some(bounds);
                return Err(e);
            }
            "matches"
        }
    };
    let ladder_at = |r: f64| pool.amounts(l, pool.s_at(r), range.sa, range.sb);
    let (_, bottom_nock) = pool.amounts(l, range.s_bottom, range.sa, range.sb);
    let (top_usdc, top_nock) = pool.amounts(l, range.s_top, range.sa, range.sb);
    let bought = bottom_nock - n0;
    let r_bottom = range.low_usd / price;
    let r_top = range.high_usd / price;

    // The existing NOCK ranges at a ratio; None when one of them could not be rebuilt.
    let existing_ok = book.problems.is_empty();
    let existing = |r: f64| -> Option<(f64, f64)> {
        existing_ok.then(|| {
            raw.positions.iter().map(|p| existing_at_ratio(p, r)).fold((0.0, 0.0), |a, b| (a.0 + b.0, a.1 + b.1))
        })
    };
    let nock_fees: f64 = raw
        .positions
        .iter()
        .map(|p| (if is_nock_token(&p.token0) { p.fee0 } else { 0.0 }) + (if is_nock_token(&p.token1) { p.fee1 } else { 0.0 }))
        .sum();

    // The standing reserve, from the snapshot's own statement of it.
    let configured: Option<f64> = match (&gs.cash_reserve_usdc, &gs.reserve_note) {
        (Some(v), _) => v.trim().parse::<f64>().ok(),
        (None, Some(_)) => Some(f64::NAN), // standing_reserve reads it as Invalid
        (None, None) => None,
    };
    let wallet_after = wallet_usdc - req.usdc;

    // The grid.
    let mut ratios: Vec<f64> = Vec::new();
    let mut k = 0usize;
    loop {
        let p = -(k as f64) * req.grid_step_pct;
        if p < req.grid_to_pct - 1e-9 {
            break;
        }
        ratios.push(1.0 + p / 100.0);
        k += 1;
    }
    let r_min = 1.0 + req.grid_to_pct / 100.0;
    for r in [r_bottom, r_top] {
        if r >= r_min - 1e-12 {
            ratios.push(r);
        }
    }
    ratios.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
    ratios.dedup_by(|a, b| (*a - *b).abs() <= 1e-9 * b.abs().max(1.0));
    let same = |a: f64, b: f64| (a - b).abs() <= 1e-9 * b.abs().max(1.0);
    let grid: Vec<Value> = ratios
        .iter()
        .map(|&r| {
            let (lu, ln) = if same(r, r_bottom) {
                (0.0, bottom_nock)
            } else if same(r, r_top) {
                (top_usdc, top_nock)
            } else {
                ladder_at(r)
            };
            let ex = existing(r);
            let stable = ex.map(|(eu, _)| reserve_reading(configured, wallet_after, lu + eu));
            let mut markers: Vec<&str> = Vec::new();
            if same(r, 1.0) {
                markers.push("spot");
            }
            if same(r, r_top) {
                markers.push("top");
            }
            if same(r, r_bottom) {
                markers.push("bottom");
            }
            json!({
                "price_usd": fx(spot * r, 8),
                "pool_price_usd": fx(price * r, 8),
                "pct_of_spot": pct((r - 1.0) * 100.0),
                "ladder_nock": fx(ln, 4),
                "ladder_nock_acquired": fx(ln - n0, 4),
                "ladder_usdc": fx(lu, 2),
                "existing_nock_lp_usdc": ex.map(|e| fx(e.0, 2)),
                "existing_nock_lp_nock": ex.map(|e| fx(e.1, 4)),
                "stable_usdc": stable.map(|s| fx(s.0, 2)),
                "reserve_headroom_usdc": stable.and_then(|s| s.1).map(|h| fx(h, 2)),
                "total_tracked_nock": ex.map(|e| fx(wallet_nock_now - n0 + ln + e.1 + nock_fees, 2)),
                "markers": markers,
            })
        })
        .collect();

    // The reserve check: gated at the ladder's own bottom.
    let at_spot = existing(1.0).map(|(eu, _)| reserve_reading(configured, wallet_usdc, eu));
    let at_bottom = existing(r_bottom).map(|(eu, _)| (eu, reserve_reading(configured, wallet_after, eu)));
    // Out of range above, the ladder holds only USDC. A top at or below the pool price is
    // that state now: the reading is at spot, where the ladder leaves the stable balance
    // unchanged. A top above it is read at the top.
    let r_at_top = r_top.max(1.0);
    let at_top = existing(r_at_top).map(|(eu, _)| reserve_reading(configured, wallet_after, top_usdc + eu));
    let below = (r_min < r_bottom - 1e-12)
        .then(|| existing(r_min).map(|(eu, _)| reserve_reading(configured, wallet_after, eu)))
        .flatten();
    let (status, reason): (&str, Option<String>) = if !existing_ok {
        ("unknown", Some(book.problems.join("; ")))
    } else {
        match &configured {
            None => ("not_configured", Some("operator_profile.cash_reserve_usdc is not set: no reserve rule".to_string())),
            Some(v) if !v.is_finite() || *v < 0.0 => ("invalid", Some(RESERVE_INVALID_NOTE.to_string())),
            Some(_) => match at_bottom.and_then(|(_, (_, h))| h) {
                Some(h) if h >= 0.0 => ("holds", None),
                Some(_) => ("breached", None),
                None => ("unknown", Some("the reserve could not be measured".to_string())),
            },
        }
    };
    let reserve_amount = configured.filter(|v| v.is_finite() && *v >= 0.0);
    let max_usdc = match (reserve_amount, at_bottom) {
        (Some(res), Some((eu, _))) => Some(floor_cents(wallet_usdc + eu - res).min(floor_cents(wallet_usdc))),
        _ => None,
    };
    let plan = data.derived_metrics.accumulation_plan.as_ref();
    let plan_unspent = plan.map(|p| {
        if p.progress_unknown {
            json!({ "usdc": p.budget_usdc, "note": "the ledger was not read this cycle: the whole budget is the most it can be" })
        } else {
            json!({ "usdc": p.remaining_usdc, "note": Value::Null })
        }
    });
    let mut reserve_check = json!({
        "status": status,
        "gate": RESERVE_GATE_NOTE,
        "cash_reserve_usdc": reserve_amount.map(|v| fx(v, 2)),
        "stable_at_spot_usdc": at_spot.map(|s| fx(s.0, 2)),
        "headroom_at_spot_usdc": at_spot.and_then(|s| s.1).map(|h| fx(h, 2)),
        "existing_ranges_usdc_at_bottom": at_bottom.map(|(eu, _)| fx(eu, 2)),
        "stable_at_bottom_usdc": at_bottom.map(|(_, s)| fx(s.0, 2)),
        "margin_usdc": at_bottom.and_then(|(_, s)| s.1).map(|h| fx(h, 2)),
        "max_usdc_for_this_range": max_usdc.map(|v| fx(v, 2)),
        "at_top": {
            "price_usd": fx(spot * r_at_top, 8),
            "out_of_range_above_now": r_top <= 1.0 + 1e-12,
            "ladder_usdc": fx(top_usdc, 2),
            "stable_usdc": at_top.map(|s| fx(s.0, 2)),
            "headroom_usdc": at_top.and_then(|s| s.1).map(|h| fx(h, 2)),
            "note": if r_top <= 1.0 + 1e-12 {
                "The ladder is out of range above now and holds only USDC: the stable balance at spot is unchanged by it, and a rise takes none of it."
            } else {
                "Out of range above (read at the top), the ladder holds only USDC: its NOCK has sold on the way up."
            },
        },
        "existing_ranges_below_bottom": below.map(|s| json!({
            "price_usd": fx(spot * r_min, 8),
            "pct_of_spot": pct((r_min - 1.0) * 100.0),
            "stable_usdc": fx(s.0, 2),
            "headroom_usdc": s.1.map(|h| fx(h, 2)),
            "note": BELOW_BOTTOM_NOTE,
        })),
        "existing_ranges_below_bottom_reason": if below.is_some() {
            Value::Null
        } else if !existing_ok {
            json!("the existing ranges could not be evaluated")
        } else {
            json!("the grid ends at or above the ladder's bottom")
        },
        "accumulation_plan_unspent": plan_unspent.map(|p| json!({ "usdc": p["usdc"], "note": p["note"], "is": NOT_A_GATE })),
        "accumulation_plan_unspent_reason": if plan.is_none() { json!("no accumulation_plan is configured") } else { Value::Null },
        "tax_reserve_usd": data.derived_metrics.sovereign_ledger.tax_reserve_usd.trim().parse::<f64>().ok().map(|v| json!({ "usd": fx(v, 2), "is": NOT_A_GATE })),
        "reason": reason,
    });

    // The floor check: the ladder's NOCK against the floor-plus-tolerance budget itself.
    let budget = gs.lp_nock_budget.as_deref().and_then(|b| b.trim().parse::<f64>().ok());
    let rebuild_nock: f64 = data
        .derived_metrics
        .lp_rebuild
        .suggestions
        .iter()
        .map(|sg| {
            (if sg.token0_symbol == "NOCK" { sg.amount0.parse::<f64>().unwrap_or(0.0) } else { 0.0 })
                + (if sg.token1_symbol == "NOCK" { sg.amount1.parse::<f64>().unwrap_or(0.0) } else { 0.0 })
        })
        .sum();
    let fits = budget.map(|b| n0 <= b.max(0.0) + 1e-9);
    let floor_status = match fits {
        None => "unknown",
        Some(true) => "fits",
        Some(false) => "exceeds",
    };

    // The hypothetical range as one more read, for the floor-crossing price.
    // Unsnapped bounds are rounded to whole ticks; a top at spot is rounded toward the
    // side that holds no NOCK, so the rounded range holds none at spot either.
    let (tl, tu) = range.ticks.unwrap_or_else(|| {
        let tick_of = |s: f64| (s * s).ln() / 1.0001f64.ln();
        let t_top = tick_of(range.s_top);
        let t_top = if req.high == Bound::Spot {
            usable_tick_toward(t_top, 1, pool.nock_is_token1)
        } else {
            t_top.round() as i32
        };
        let t_bottom = tick_of(range.s_bottom).round() as i32;
        (t_top.min(t_bottom), t_top.max(t_bottom))
    });
    // On rounded ticks the same liquidity would hold slightly different NOCK at spot; it
    // is rescaled so the hypothetical range holds exactly the NOCK the ladder needs (and
    // floor_breach_effect takes exactly that out of the wallet).
    let nock_of = |liq: u128| {
        let (a0, a1) = concentrated_amounts_raw(liq, pool.template.sqrt_price_x96, tl, tu);
        if pool.nock_is_token1 { a1 / 10f64.powi(pool.nock_dec) } else { a0 / 10f64.powi(pool.nock_dec) }
    };
    let l_u128 = l.round().max(0.0) as u128;
    let mut l_hyp = l_u128;
    if range.ticks.is_none() && n0 > 0.0 {
        let held = nock_of(l_hyp);
        if held > 0.0 {
            l_hyp = (l_hyp as f64 * n0 / held).round() as u128;
        }
    }
    let (ra0, ra1) = concentrated_amounts_raw(l_hyp, pool.template.sqrt_price_x96, tl, tu);
    let ladder_raw = RawPosition {
        position_id: LADDER_POSITION_ID.to_string(),
        nft_id: 0,
        liquidity: l_hyp,
        tick_lower: tl,
        tick_upper: tu,
        amount0: ra0 / 10f64.powi(pool.template.token0.decimals),
        amount1: ra1 / 10f64.powi(pool.template.token1.decimals),
        fee0: 0.0,
        fee1: 0.0,
        in_range: tl <= pool.template.tick && pool.template.tick <= tu,
        is_price_source: false,
        ..pool.template.clone()
    };
    let effect = floor_breach_effect(data, &book, &ladder_raw, n0, req.usdc, range.ticks.is_none());

    let floor_check = json!({
        "status": floor_status,
        "lp_nock_budget": budget.map(|b| fx(b, 4)),
        "budget_note": gs.lp_nock_budget_note,
        "nock_needed": fx(n0, 4),
        "rebuild_suggested_nock": fx(rebuild_nock, 4),
        "rebuild_note": format!(
            "the rebuild's suggestions this cycle also draw {} NOCK of the same budget: an alternative use of it, not subtracted",
            fx(rebuild_nock, 0)
        ),
        "max_usdc_within_budget": match (fits, budget) {
            (Some(false), Some(b)) if n0 > 0.0 => Some(fx(floor_cents(req.usdc * b.max(0.0) / n0), 2)),
            _ => None,
        },
        "floor_breach_effect": effect,
        "reason": if budget.is_none() { json!("this snapshot predates lp_nock_budget (2.9.0)") } else { Value::Null },
    });

    // The tops next to spot on a snapped grid, each with its NOCK need.
    let (top_neighbours, top_neighbours_reason) = match (spacing, range.ticks) {
        (Some(s), Some((tl, tu))) => {
            // The bottom's tick: the higher tick when NOCK is token1.
            let t_low = if pool.nock_is_token1 { tu } else { tl };
            let t_at = pool.tick_at_or_below_price(s);
            let rows: Vec<Value> = [t_at, pool.tick_above(t_at, s)]
                .iter()
                .map(|&t| {
                    let p_usd = pool.usd_at_tick(t);
                    let need = (t != t_low).then(|| pool.range_from_ticks(t_low, t)).and_then(|rg| pool.size(&rg, req.usdc)).map(|(_, n)| n);
                    json!({
                        "tick": t,
                        "price_usd": fx(p_usd, 8),
                        "pct_of_spot": pct((p_usd / price - 1.0) * 100.0),
                        "nock_needed": need.map(|n| fx(n, 4)),
                        "fits_budget": need.and_then(|n| budget.map(|b| n <= b.max(0.0) + 1e-9)),
                        "selected": range.ticks.map(|(a, b)| a == t || b == t).unwrap_or(false) && t != t_low,
                    })
                })
                .collect();
            (json!(rows), Value::Null)
        }
        _ => (Value::Null, json!("unsnapped: there is no tick grid, so there are no neighbouring tops")),
    };

    let lag = now.signed_duration_since(env.meta.generated_at).num_seconds().max(0);
    let source_label = position_label(source_raw);
    let pool_label = position_label(&pool.template);
    let pool_note = match spacing_source {
        Some("position") => POOL_NEW_NFT_NOTE.to_string(),
        Some(_) => format!(
            "snapped to the tick spacing you gave ({}); sized at the price-source pool's price ({source_label})",
            spacing.unwrap_or(0)
        ),
        None => "unsnapped: no position_id or tick_spacing was given; the app rounds each bound to a usable tick of the pool you choose".to_string(),
    };
    // Tracked ladders (#167) among the existing NOCK ranges; absent with none.
    if !raw.ladders.is_empty() {
        let mut ids: Vec<&String> = raw.ladders.keys().collect();
        ids.sort();
        reserve_check["existing_ladders"] = json!(ids);
    }
    Ok(json!({
        "snapshot": {
            "snapshot_id": env.meta.snapshot_id.to_string(),
            "generated_at": env.meta.generated_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            "version": env.meta.version,
            "age_seconds": lag,
            "age_minutes": lag / 60,
            "worker_health": health,
            "spot_usd": fx(spot, 8),
            "spot_source": format!("price-source pool sqrtPriceX96 ({source_label})"),
        },
        "inputs": {
            "usdc": fx(req.usdc, 2),
            "nock": req.nock.map(|n| format!("{n}")),
            "low_requested": bound_text(&req.low),
            "high_requested": bound_text(&req.high),
            "position_id": req.position_id,
            "tick_spacing": req.tick_spacing,
            "grid_to_pct": req.grid_to_pct,
            "grid_step_pct": req.grid_step_pct,
        },
        "pool": {
            "position_id": if named { json!(pool.template.position_id) } else { Value::Null },
            "label": pool_label,
            "pool_price_usd": fx(price, 8),
            "pool_price_vs_spot_pct": pct((price / spot - 1.0) * 100.0),
            "tick_spacing": spacing,
            "spacing_source": spacing_source,
            "snapped": spacing.is_some(),
            "note": pool_note,
        },
        "bounds": {
            "low_usd": bounds["low_usd"],
            "high_usd": bounds["high_usd"],
            "low_pct": bounds["low_pct"],
            "high_pct": bounds["high_pct"],
            "tick_lower": bounds["tick_lower"],
            "tick_upper": bounds["tick_upper"],
            "high_rule": high_rule,
        },
        "top_neighbours": top_neighbours,
        "top_neighbours_reason": top_neighbours_reason,
        "sizing": {
            "liquidity_raw": format!("{}", l_u128),
            "liquidity_human": fx(l / 10f64.powf((pool.usdc_dec + pool.nock_dec) as f64 / 2.0), 4),
            "usdc_at_spot": fx(req.usdc, 2),
            "nock_needed_at_spot": fx(n0, 4),
            "nock_given": req.nock.map(|n| format!("{n}")),
            "nock_check": nock_check,
        },
        "grid": grid,
        "bottom": {
            "price_usd": fx(range.low_usd, 8),
            "pct_of_spot": pct((r_bottom - 1.0) * 100.0),
            "nock_held": fx(bottom_nock, 4),
            "nock_bought": fx(bought, 4),
            "avg_buy_price_usd": (bought > 0.0).then(|| fx(req.usdc / bought, 8)),
        },
        "top": {
            "price_usd": fx(range.high_usd, 8),
            "pct_of_spot": pct((r_top - 1.0) * 100.0),
            "multiple_of_spot": fx(r_top, 4),
            "usdc_held": fx(top_usdc, 2),
            "out_of_range_above_now": r_top <= 1.0 + 1e-12,
            "avg_sell_price_usd": (n0 > 0.0).then(|| fx((top_usdc - req.usdc) / n0, 8)),
        },
        "reserve_check": reserve_check,
        "floor_check": floor_check,
        "assumptions": LADDER_ASSUMPTIONS,
        "statement": LADDER_STATEMENT,
    }))
}

/// The floor-crossing price before and after adding the ladder (#165's
/// `floor_breach_with`): the ladder's NOCK moves out of the wallet into the range. Pure.
fn floor_breach_effect(data: &SnapshotData, book: &StoredBook, ladder: &RawPosition, n0: f64, usdc: f64, rounded_ticks: bool) -> Value {
    let unknown = |reason: String| json!({ "status": "unknown", "before": Value::Null, "after": Value::Null, "reason": reason, "assumption": Value::Null });
    let Some(stored) = data.derived_metrics.floor_breach.as_ref() else {
        return unknown("this snapshot predates floor_breach (2.9.0)".to_string());
    };
    let Some(bps) = data.derived_metrics.global_status.lp_floor_tolerance_bps else {
        return unknown("this snapshot predates lp_floor_tolerance_bps (2.9.0)".to_string());
    };
    if !book.problems.is_empty() {
        return unknown(book.problems.join("; "));
    }
    let Ok(floor) = stored.floor_nock.trim().parse::<f64>() else {
        return unknown(format!("floor_breach.floor_nock '{}' could not be read", stored.floor_nock));
    };
    let profile = OperatorProfile { treasury_floor_nock: floor, lp_floor_tolerance_bps: bps, ..Default::default() };
    let rebuild = |nock_out: f64, usdc_out: f64| RawOnChainData {
        wallet_balances: book
            .raw
            .wallet_balances
            .iter()
            .map(|w| {
                let out = match w.token_symbol.as_str() {
                    "NOCK" => nock_out,
                    "USDC" => usdc_out,
                    _ => 0.0,
                };
                if out == 0.0 {
                    return w.clone();
                }
                let amt: f64 = w.amount.parse().unwrap_or(0.0);
                WalletBalance { amount: format!("{:.10}", amt - out), ..w.clone() }
            })
            .collect(),
        positions: book.raw.positions.clone(),
        spot_price_nock_usd: book.raw.spot_price_nock_usd,
        pool_liquidity: book.raw.pool_liquidity.clone(),
        ..Default::default()
    };
    let before = floor_breach_with(&rebuild(0.0, 0.0), &profile, &[]);
    // NOCK is conserved at spot: what leaves the wallet is the hypothetical range's own
    // NOCK on its (possibly rounded) ticks, not the unrounded figure.
    let ladder_nock = if is_nock_token(&ladder.token1) { ladder.amount1 } else { ladder.amount0 };
    let ladder_nock = if ladder_nock < LADDER_NOCK_DUST { 0.0 } else { ladder_nock };
    let after = floor_breach_with(&rebuild(ladder_nock, usdc), &profile, std::slice::from_ref(ladder));
    let view = |f: &FloorBreach| json!({ "status": f.status, "crossing": f.crossing, "tolerated": f.tolerated, "reason": f.reason, "nock_now": f.nock_now });
    let multiple = |f: &FloorBreach| f.crossing.as_ref().and_then(|c| c.multiple.as_deref()).and_then(|m| m.parse::<f64>().ok());
    let (status, reason): (&str, String) = if before.status == "no_floor" {
        ("no_floor", "no treasury_floor_nock is set".to_string())
    } else if before.status == "unavailable" || after.status == "unavailable" {
        ("unknown", after.reason.clone().or(before.reason.clone()).unwrap_or_default())
    } else if n0 <= 0.0 {
        ("none", "the ladder holds no NOCK at or above spot, so a rise sells none of it".to_string())
    } else {
        match (multiple(&before), multiple(&after), before.status.as_str(), after.status.as_str()) {
            (Some(b), Some(a), _, _) if a < b => ("lowers", "the ladder's NOCK leaves the wallet and sells as NOCK rises to the top".to_string()),
            (Some(b), Some(a), _, _) if a > b => ("raises", "the ladder raises the floor-crossing price".to_string()),
            (_, _, bs, as_) if bs == as_ && before.crossing == after.crossing => ("unchanged", "the floor-crossing price is unchanged at this precision".to_string()),
            (None, Some(_), _, _) => ("lowers", "with the ladder a rise crosses the floor; without it none does".to_string()),
            _ => ("changes", format!("{} before, {} with the ladder", before.status, after.status)),
        }
    };
    let mut assumption = FLOOR_BREACH_ASSUMPTION.to_string();
    if rounded_ticks {
        assumption.push_str(" The unsnapped ladder's bounds are rounded to whole ticks for this figure.");
    }
    json!({
        "status": status,
        "before": view(&before),
        "after": view(&after),
        "reason": reason,
        "assumption": assumption,
    })
}

// ------------------------------------------------------------------
// HTTP
// ------------------------------------------------------------------

/// GET /v1/simulate/ladder (#166). Read-only: the latest stored snapshot (memory, then
/// the database, as /v1/snapshot/latest), no RPC read, no CycleGate. 200 the simulation,
/// 400 INVALID_PARAMETER, 422 LADDER_NOT_SIMULATED with a `code`, 503 NO_SNAPSHOT.
pub(crate) async fn simulate_ladder_handler(
    Extension(state): Extension<Arc<AppState>>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Response {
    let req = match LadderRequest::from_params(&params) {
        Ok(r) => r,
        Err(e) => return ladder_error_response(&e),
    };
    let mut latest = state.latest_snapshot.lock().await;
    load_latest_snapshot_if_empty(&mut latest, &state.db).await;
    let last_error: Option<String> = state.last_ingest_error.lock().ok().and_then(|slot| slot.clone()).map(|(_, e)| e);
    let Some(env) = latest.as_ref() else {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(no_snapshot_body(last_error.as_deref()))).into_response();
    };
    let now = Utc::now();
    let lag = now.signed_duration_since(env.meta.generated_at).num_seconds().max(0);
    let health = health_status(true, lag, state.config.sync_window_seconds as i64, last_error.is_some());
    match simulate_ladder(env, &req, now, health) {
        Ok(body) => (StatusCode::OK, Json(body)).into_response(),
        Err(e) => ladder_error_response(&e),
    }
}

pub(crate) fn ladder_error_response(e: &LadderError) -> Response {
    let status = StatusCode::from_u16(e.status).unwrap_or(StatusCode::BAD_REQUEST);
    (status, Json(e.body())).into_response()
}

/// The MCP tool (both transports run the same text): the arguments, the snapshot held in
/// memory, the time and health. Returns (text, is_error). Pure.
pub(crate) fn simulate_ladder_mcp(args: Option<&Value>, env: &SnapshotEnvelope, now: DateTime<Utc>, health: &str) -> (String, bool) {
    let params: HashMap<String, String> = match ladder_text::ladder_query_from_args(args) {
        Ok(pairs) => pairs.into_iter().collect(),
        Err(body) => return (ladder_text::ladder_error_text(&body), true),
    };
    match LadderRequest::from_params(&params).and_then(|r| simulate_ladder(env, &r, now, health)) {
        Ok(body) => (ladder_text::ladder_text(&body), false),
        Err(e) => (ladder_text::ladder_error_text(&e.body()), true),
    }
}
