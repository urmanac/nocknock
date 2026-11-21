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

/// One rung as the caller gave it: the flat form is one rung with no index and no role.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct RungRequest {
    /// 1..=LADDER_MAX_RUNGS for an indexed rung (`r1_`..), 0 for the flat form.
    pub(crate) index: usize,
    /// `near` | `deep` for an indexed rung (required, never assumed); None for the flat form.
    pub(crate) rung: Option<String>,
    pub(crate) usdc: f64,
    pub(crate) nock: Option<f64>,
    pub(crate) low: Bound,
    pub(crate) high: Bound,
}

/// The caller's inputs, validated.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LadderRequest {
    /// One rung in the flat form; one per index in the indexed form.
    pub(crate) rungs: Vec<RungRequest>,
    /// True for the indexed form (`r1_`..`r4_`), even with one rung.
    pub(crate) indexed: bool,
    pub(crate) position_id: Option<String>,
    pub(crate) tick_spacing: Option<i32>,
    pub(crate) grid_to_pct: f64,
    pub(crate) grid_step_pct: f64,
    /// The trade-off table's split step, % of the total (indexed form only).
    pub(crate) split_step_pct: f64,
    /// `shape=configured`: the operator's own `ladder_rungs`, sized to `shape_usdc` or,
    /// when it is not given, to the shared reserve gate's most (`max_usdc_total`).
    pub(crate) shape: Option<ShapeRequest>,
}

/// `shape=configured` and its total.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ShapeRequest {
    pub(crate) total_usdc: Option<f64>,
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
    /// The same error, said of rung `rq` of a ladder of rungs: its detail names the rung,
    /// and a range that snaps to zero width is RUNG_TOO_NARROW.
    fn of_rung(mut self, rq: &RungRequest) -> Self {
        if rq.index == 0 {
            return self;
        }
        if self.code == Some("RANGE_TOO_NARROW") {
            self.code = Some("RUNG_TOO_NARROW");
        }
        self.detail = format!("r{} ({}): {}", rq.index, rq.rung.as_deref().unwrap_or("?"), self.detail);
        self
    }
}

/// Fixed decimals, rounded, and never "-0.00". Pure.
pub(crate) fn fx(v: f64, dp: usize) -> String {
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
/// The per-rung parameters of the indexed form (`r1_usdc`, ...).
pub(crate) const LADDER_RUNG_PARAMS: [&str; 7] = ["rung", "usdc", "nock", "low", "low_pct", "high", "high_pct"];
/// The flat form's rung parameters: never mixed with indexed ones.
const LADDER_FLAT_RUNG_PARAMS: [&str; 6] = ["usdc", "nock", "low", "low_pct", "high", "high_pct"];
/// The most rungs one call simulates. A display limit, echoed; not an operator number.
pub(crate) const LADDER_MAX_RUNGS: usize = 4;
/// The split table's step, % of the total, when the caller does not say (5 to 50). A
/// display step, echoed; not an operator number.
pub(crate) const LADDER_SPLIT_STEP_DEFAULT_PCT: f64 = 10.0;
/// The most rows a depth table draws. A display limit, echoed.
pub(crate) const LADDER_TRADEOFF_MAX_ROWS: usize = 12;
/// The fewest seconds of stored intervals wholly inside a rung for a per-day band figure;
/// below it only the replay dollars and seconds are printed. A display filter, echoed.
pub(crate) const LADDER_FEE_BAND_MIN_SECONDS: f64 = 72000.0;
pub(crate) const LADDER_DISPLAY_NOTE: &str = "display constants, not settings";
/// The trade-off tables' fixed sentence, the same in every surface.
pub(crate) const LADDER_TRADEOFF_STATEMENT: &str =
    "Rows are measurements at your depths; NockNock does not choose the split or the depths.";
pub(crate) const LADDER_TRADEOFF_ORDER: &str = "Rows are in the order of the quantity varied and are not ranked.";
pub(crate) const LADDER_RUNG_ROLES: [&str; 3] = ["cap", "near", "deep"];

/// `r3_usdc` -> (3, "usdc"); None when the key is not of the indexed form. Pure.
fn indexed_key(k: &str) -> Option<(usize, &str)> {
    let rest = k.strip_prefix('r')?;
    let (n, suffix) = rest.split_once('_')?;
    if n.is_empty() || !n.chars().all(|c| c.is_ascii_digit()) || n.len() > 3 {
        return None;
    }
    Some((n.parse().ok()?, suffix))
}

/// One rung's parameters, named `{prefix}usdc` and so on. Pure.
fn parse_rung(params: &HashMap<String, String>, prefix: &str, index: usize) -> Result<RungRequest, LadderError> {
    let key = |k: &str| format!("{prefix}{k}");
    let get = |k: &str| params.get(&key(k)).map(|v| v.trim()).filter(|v| !v.is_empty());
    let num = |k: &str| -> Result<Option<f64>, LadderError> {
        match get(k) {
            None => Ok(None),
            Some(v) => match v.parse::<f64>() {
                Ok(x) if x.is_finite() => Ok(Some(x)),
                _ => Err(LadderError::invalid(format!("{} must be a finite number, not '{v}'", key(k)))),
            },
        }
    };
    let rung = if index == 0 {
        None
    } else {
        match get("rung") {
            None => {
                return Err(LadderError::invalid(format!(
                    "{} is required (cap, near or deep): NockNock assumes none",
                    key("rung")
                )))
            }
            Some(r) if LADDER_RUNG_ROLES.contains(&r.to_ascii_lowercase().as_str()) => Some(r.to_ascii_lowercase()),
            Some(r) => return Err(LadderError::invalid(format!("{} must be cap, near or deep, not '{r}'", key("rung")))),
        }
    };
    let (u, n, lo, lp, hi, hp) = (key("usdc"), key("nock"), key("low"), key("low_pct"), key("high"), key("high_pct"));
    let usdc = num("usdc")?.ok_or_else(|| LadderError::invalid(format!("{u} is required: the USDC the ladder would hold at spot")))?;
    if !(usdc > 0.0) {
        return Err(LadderError::invalid(format!("{u} must be above 0 (nothing to size): {usdc}")));
    }
    let nock = num("nock")?;
    if let Some(x) = nock {
        if x < 0.0 {
            return Err(LadderError::invalid(format!("{n} must be 0 or more: {x}")));
        }
    }
    let low = match (num("low")?, num("low_pct")?) {
        (Some(_), Some(_)) => return Err(LadderError::invalid(format!("give {lo} or {lp}, not both"))),
        (None, None) => return Err(LadderError::invalid(format!("{lo} or {lp} is required: the bottom of the range"))),
        (Some(v), None) if v > 0.0 => Bound::Usd(v),
        (Some(v), None) => return Err(LadderError::invalid(format!("{lo} must be above 0 USD per NOCK: {v}"))),
        (None, Some(p)) if p > -100.0 => Bound::Pct(p),
        (None, Some(p)) => return Err(LadderError::invalid(format!("{lp} must be above -100: {p}"))),
    };
    let high_spot = get("high").map(|v| v.eq_ignore_ascii_case("spot")).unwrap_or(false);
    let high_num = if high_spot { None } else { num("high")? };
    let high = match (high_spot, high_num, num("high_pct")?) {
        (true, _, Some(_)) | (false, Some(_), Some(_)) => return Err(LadderError::invalid(format!("give {hi} or {hp}, not both"))),
        (true, _, None) => Bound::Spot,
        (false, Some(v), None) if v > 0.0 => Bound::Usd(v),
        (false, Some(v), None) => return Err(LadderError::invalid(format!("{hi} must be above 0 USD per NOCK: {v}"))),
        (false, None, Some(p)) if p > -100.0 => Bound::Pct(p),
        (false, None, Some(p)) => return Err(LadderError::invalid(format!("{hp} must be above -100: {p}"))),
        (false, None, None) => {
            return Err(LadderError::invalid(format!("{hi}, {hi}=spot or {hp} is required: the top of the range")))
        }
    };
    match (&low, &high) {
        (Bound::Usd(l), Bound::Usd(h)) if l >= h => {
            return Err(LadderError::invalid(format!("{lo} ({l}) must be below {hi} ({h})")))
        }
        (Bound::Pct(l), Bound::Pct(h)) if l >= h => {
            return Err(LadderError::invalid(format!("{lp} ({l}) must be below {hp} ({h})")))
        }
        _ => {}
    }
    Ok(RungRequest { index, rung, usdc, nock, low, high })
}

impl LadderRequest {
    /// The GET /v1/simulate/ladder query (and the MCP arguments, normalised to the same
    /// names by `ladder_text::ladder_query_from_args`). The flat form (`usdc`, `low`, ...)
    /// is one rung; the indexed form (`r1_rung`, `r1_usdc`, ...) is up to
    /// LADDER_MAX_RUNGS rungs in one pool. Pure.
    pub(crate) fn from_params(params: &HashMap<String, String>) -> Result<LadderRequest, LadderError> {
        let mut keys: Vec<&String> = params.keys().collect();
        keys.sort();
        let mut indices: Vec<usize> = Vec::new();
        for k in keys {
            if LADDER_PARAMS.contains(&k.as_str()) || ["split_step_pct", "shape", "shape_usdc"].contains(&k.as_str()) {
                continue;
            }
            match indexed_key(k) {
                Some((n, suffix)) if LADDER_RUNG_PARAMS.contains(&suffix) && n >= 1 => {
                    if n > LADDER_MAX_RUNGS {
                        return Err(LadderError::invalid(format!(
                            "'{k}': at most {LADDER_MAX_RUNGS} rungs (LADDER_MAX_RUNGS, a display limit) are simulated in one call"
                        )));
                    }
                    if !indices.contains(&n) {
                        indices.push(n);
                    }
                }
                Some((n, _)) if n >= 1 => {
                    return Err(LadderError::invalid(format!(
                        "unknown rung parameter '{k}'; a rung's parameters are r{n}_{}",
                        LADDER_RUNG_PARAMS.join(&format!(", r{n}_"))
                    )))
                }
                _ => {
                    return Err(LadderError::invalid(format!(
                        "unknown parameter '{k}'; the parameters are {}",
                        LADDER_PARAMS.join(", ")
                    )))
                }
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
        let shape = match (get("shape"), params.contains_key("shape_usdc")) {
            (None, false) => None,
            (None, true) => return Err(LadderError::invalid("shape_usdc applies only with shape=configured")),
            (Some(v), _) if v.eq_ignore_ascii_case("configured") => {
                if let Some(k) = LADDER_FLAT_RUNG_PARAMS.iter().find(|k| params.contains_key(**k)) {
                    return Err(LadderError::invalid(format!("'{k}' is the one-rung form; shape=configured draws the rungs from operator_profile.ladder_rungs, not both")));
                }
                if !indices.is_empty() {
                    return Err(LadderError::invalid("shape=configured draws the rungs from operator_profile.ladder_rungs; give it or indexed rungs (r1_..), not both"));
                }
                let total = match num("shape_usdc")? {
                    None => None,
                    Some(v) if v > 0.0 => Some(v),
                    Some(v) => return Err(LadderError::invalid(format!("shape_usdc must be above 0 (nothing to size): {v}"))),
                };
                Some(ShapeRequest { total_usdc: total })
            }
            (Some(v), _) => return Err(LadderError::invalid(format!("shape must be 'configured' (operator_profile.ladder_rungs), not '{v}'"))),
        };
        let indexed = !indices.is_empty() || shape.is_some();
        let rungs = if shape.is_some() {
            Vec::new()
        } else if indexed {
            if let Some(k) = LADDER_FLAT_RUNG_PARAMS.iter().find(|k| params.contains_key(**k)) {
                return Err(LadderError::invalid(format!(
                    "'{k}' is the one-rung form; with indexed rungs (r1_..) give every rung's parameters indexed, not both"
                )));
            }
            indices.sort();
            if let Some(gap) = (1..=indices.len()).find(|i| !indices.contains(i)) {
                return Err(LadderError::invalid(format!(
                    "rungs are numbered from r1 without a gap: r{gap} is missing (given: {})",
                    indices.iter().map(|i| format!("r{i}")).collect::<Vec<_>>().join(", ")
                )));
            }
            indices.iter().map(|&i| parse_rung(params, &format!("r{i}_"), i)).collect::<Result<Vec<_>, _>>()?
        } else {
            vec![parse_rung(params, "", 0)?]
        };
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
        let split_step_pct = match num("split_step_pct")? {
            Some(_) if !indexed => {
                return Err(LadderError::invalid("split_step_pct applies to a ladder of rungs (r1_..), not to the one-rung form"))
            }
            Some(v) if (5.0..=50.0).contains(&v) => v,
            Some(v) => return Err(LadderError::invalid(format!("split_step_pct must be from 5 to 50: {v}"))),
            None => LADDER_SPLIT_STEP_DEFAULT_PCT,
        };
        Ok(LadderRequest { rungs, indexed, position_id, tick_spacing, grid_to_pct, grid_step_pct, split_step_pct, shape })
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

impl LadderRequest {
    /// The one rung of the flat form (the first of the indexed form).
    pub(crate) fn first(&self) -> &RungRequest {
        &self.rungs[0]
    }
}

/// Everything a simulation reads from the snapshot before any range is drawn: spot, the
/// ladder's pool, the wallet, the reserve, the budget and the fee measurement.
struct LadderCtx<'a> {
    env: &'a SnapshotEnvelope,
    book: StoredBook,
    spot: f64,
    source_label: String,
    pool: LadderPool,
    spacing: Option<i32>,
    spacing_source: Option<&'static str>,
    named: bool,
    wallet_usdc: f64,
    wallet_nock_now: f64,
    /// The standing reserve as the snapshot states it (NaN = invalid).
    configured: Option<f64>,
    nock_fees: f64,
    budget: Option<f64>,
    /// `operator_profile.ladder_bottom_usd` from the snapshot, or why it is not applied.
    ladder_bottom: Option<f64>,
    ladder_bottom_note: Option<String>,
    fee: FeeCtx<'a>,
}

/// The fee measurement of the ladder's pool, or why there is none.
struct FeeCtx<'a> {
    fy: Option<&'a FeeYield>,
    pool: Option<&'a FeeYieldPool>,
    reason: Option<String>,
    /// USDC and NOCK per raw liquidity per day.
    rate: Option<(f64, f64)>,
    /// The pool's active liquidity at the current tick, or why it is not known.
    active: Option<f64>,
    /// The stored decimal string of the active liquidity, stated as read.
    active_raw: Option<String>,
    active_reason: Option<String>,
    age_seconds: Option<i64>,
}

impl<'a> LadderCtx<'a> {
    fn new(env: &'a SnapshotEnvelope, req: &LadderRequest, now: DateTime<Utc>) -> Result<LadderCtx<'a>, LadderError> {
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
        let (template, spacing, spacing_source, named): (RawPosition, Option<i32>, Option<&'static str>, bool) = match (&req.position_id, req.tick_spacing) {
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
        let source_label = position_label(source_raw);
        let pool = LadderPool {
            nock_is_token1,
            usdc_dec: if nock_is_token1 { template.token0.decimals } else { template.token1.decimals },
            nock_dec: if nock_is_token1 { template.token1.decimals } else { template.token0.decimals },
            s_now: sqrt_price_x96_to_raw(template.sqrt_price_x96).sqrt(),
            price,
            template,
        };
        let wallet_usdc: f64 = data
            .wallet_balances
            .iter()
            .find(|w| w.token_symbol == "USDC")
            .and_then(|w| w.amount.parse().ok())
            .unwrap_or(0.0);
        let wallet_nock_now = wallet_nock(raw);
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
        let budget = gs.lp_nock_budget.as_deref().and_then(|b| b.trim().parse::<f64>().ok());
        let ladder_bottom = gs.ladder_bottom_usd.as_deref().and_then(|v| v.trim().parse::<f64>().ok()).filter(|v| v.is_finite() && *v > 0.0);
        let fee = FeeCtx::new(data, &pool, spacing_source, now);
        Ok(LadderCtx {
            env,
            book,
            spot,
            source_label,
            pool,
            spacing,
            spacing_source,
            named,
            wallet_usdc,
            wallet_nock_now,
            configured,
            nock_fees,
            budget,
            ladder_bottom,
            ladder_bottom_note: gs.ladder_bottom_note.clone(),
            fee,
        })
    }

    fn price(&self) -> f64 {
        self.pool.price
    }

    /// The existing NOCK ranges at a ratio; None when one of them could not be rebuilt.
    fn existing(&self, r: f64) -> Option<(f64, f64)> {
        self.book.problems.is_empty().then(|| {
            self.book.raw.positions.iter().map(|p| existing_at_ratio(p, r)).fold((0.0, 0.0), |a, b| (a.0 + b.0, a.1 + b.1))
        })
    }

    fn reserve_amount(&self) -> Option<f64> {
        self.configured.filter(|v| v.is_finite() && *v >= 0.0)
    }

    /// The range a rung's bounds give, snapped or not, and the rule its top followed.
    fn resolve_range(&self, low_b: &Bound, high_b: &Bound) -> Result<(Range, &'static str), LadderError> {
        let pool = &self.pool;
        let price = pool.price;
        let low = match low_b {
            Bound::Usd(v) => *v,
            Bound::Pct(p) => price * (1.0 + p / 100.0),
            Bound::Spot => price,
        };
        let high = match high_b {
            Bound::Usd(v) => *v,
            Bound::Pct(p) => price * (1.0 + p / 100.0),
            Bound::Spot => price,
        };
        if low >= high {
            return Err(LadderError::invalid(format!("the bottom ({}) must be below the top ({})", fx(low, 8), fx(high, 8))));
        }
        if low >= price {
            return Err(above_spot(low, price));
        }
        let (range, high_rule) = match self.spacing {
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
                let (t_high, rule) = match high_b {
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
                if *high_b == Bound::Spot {
                    // The top is the pool's own price: its own sqrt, not a USD round trip
                    // that can land one ulp off and leave dust NOCK in the range.
                    r.s_top = pool.s_now;
                    if pool.nock_is_token1 { r.sa = pool.s_now } else { r.sb = pool.s_now }
                }
                (r, "unsnapped")
            }
        };
        if range.low_usd >= price {
            return Err(above_spot(range.low_usd, price));
        }
        Ok((range, high_rule))
    }

    /// The rung sized on its range: liquidity, the NOCK it needs at the pool price, and
    /// what it holds at its bottom and top. None when the range holds no USDC now.
    fn sized(&self, rq: &RungRequest, range: Range, high_rule: &'static str, usdc: f64) -> Option<SizedRung> {
        let (l, n0) = self.pool.size(&range, usdc)?;
        let n0 = if n0 < LADDER_NOCK_DUST { 0.0 } else { n0 };
        let (_, bottom_nock) = self.pool.amounts(l, range.s_bottom, range.sa, range.sb);
        let (top_usdc, top_nock) = self.pool.amounts(l, range.s_top, range.sa, range.sb);
        Some(SizedRung { req: rq.clone(), usdc, range, high_rule, l, n0, bottom_nock, top_usdc, top_nock })
    }

    /// One rung as the hypothetical position the floor-crossing figure adds. Unsnapped
    /// bounds are rounded to whole ticks; a top at spot is rounded toward the side that
    /// holds no NOCK, so the rounded range holds none at spot either.
    fn hypothetical(&self, r: &SizedRung, id: String) -> RawPosition {
        let pool = &self.pool;
        let range = &r.range;
        let (tl, tu) = range.ticks.unwrap_or_else(|| {
            let tick_of = |s: f64| (s * s).ln() / 1.0001f64.ln();
            let t_top = tick_of(range.s_top);
            let t_top = if r.req.high == Bound::Spot { usable_tick_toward(t_top, 1, pool.nock_is_token1) } else { t_top.round() as i32 };
            let t_bottom = tick_of(range.s_bottom).round() as i32;
            (t_top.min(t_bottom), t_top.max(t_bottom))
        });
        // On rounded ticks the same liquidity would hold slightly different NOCK at spot; it
        // is rescaled so the hypothetical range holds exactly the NOCK the rung needs (and
        // floor_breach_effect takes exactly that out of the wallet).
        let nock_of = |liq: u128| {
            let (a0, a1) = concentrated_amounts_raw(liq, pool.template.sqrt_price_x96, tl, tu);
            if pool.nock_is_token1 { a1 / 10f64.powi(pool.nock_dec) } else { a0 / 10f64.powi(pool.nock_dec) }
        };
        let mut l_hyp = r.l.round().max(0.0) as u128;
        if range.ticks.is_none() && r.n0 > 0.0 {
            let held = nock_of(l_hyp);
            if held > 0.0 {
                l_hyp = (l_hyp as f64 * r.n0 / held).round() as u128;
            }
        }
        let (ra0, ra1) = concentrated_amounts_raw(l_hyp, pool.template.sqrt_price_x96, tl, tu);
        RawPosition {
            position_id: id,
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
        }
    }

    /// Where the reserve gate is read: the rungs' deepest bottom, or the operator's
    /// ladder bottom when it is lower. (ratio, price, rule, configured-bottom stated?)
    fn gate(&self, deepest_low_usd: f64) -> (f64, f64, String) {
        let price = self.price();
        match (self.ladder_bottom, &self.ladder_bottom_note) {
            (Some(lb), _) if lb < deepest_low_usd => (
                lb / price,
                lb,
                format!("your ladder bottom (operator_profile.ladder_bottom_usd ${}), below the deepest rung's bottom", fx(lb, 8)),
            ),
            (Some(lb), _) => (
                deepest_low_usd / price,
                deepest_low_usd,
                format!("the deepest rung's bottom, at or below your ladder bottom (operator_profile.ladder_bottom_usd ${})", fx(lb, 8)),
            ),
            (None, Some(note)) => (deepest_low_usd / price, deepest_low_usd, format!("the deepest rung's bottom: {note}")),
            (None, None) => (
                deepest_low_usd / price,
                deepest_low_usd,
                "the deepest rung's bottom: operator_profile.ladder_bottom_usd is not set, so the gate falls back to it".to_string(),
            ),
        }
    }

    /// The `snapshot` block every body carries.
    fn snapshot_block(&self, now: DateTime<Utc>, health: &str) -> Value {
        let lag = now.signed_duration_since(self.env.meta.generated_at).num_seconds().max(0);
        json!({
            "snapshot_id": self.env.meta.snapshot_id.to_string(),
            "generated_at": self.env.meta.generated_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            "version": self.env.meta.version,
            "age_seconds": lag,
            "age_minutes": lag / 60,
            "worker_health": health,
            "spot_usd": fx(self.spot, 8),
            "spot_source": format!("price-source pool sqrtPriceX96 ({})", self.source_label),
        })
    }

    /// The `pool` block every body carries.
    fn pool_block(&self) -> Value {
        let pool_note = match self.spacing_source {
            Some("position") => POOL_NEW_NFT_NOTE.to_string(),
            Some(_) => format!(
                "snapped to the tick spacing you gave ({}); sized at the price-source pool's price ({})",
                self.spacing.unwrap_or(0),
                self.source_label
            ),
            None => "unsnapped: no position_id or tick_spacing was given; the app rounds each bound to a usable tick of the pool you choose".to_string(),
        };
        json!({
            "position_id": if self.named { json!(self.pool.template.position_id) } else { Value::Null },
            "label": position_label(&self.pool.template),
            "pool_price_usd": fx(self.price(), 8),
            "pool_price_vs_spot_pct": pct((self.price() / self.spot - 1.0) * 100.0),
            "tick_spacing": self.spacing,
            "spacing_source": self.spacing_source,
            "snapped": self.spacing.is_some(),
            "note": pool_note,
        })
    }

    /// The grid's ratios: every step down to grid_to_pct, plus the given extra ratios that
    /// fall inside it, highest first, without duplicates.
    fn grid_ratios(&self, req: &LadderRequest, extra: &[f64]) -> Vec<f64> {
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
        for &r in extra {
            if r >= r_min - 1e-12 {
                ratios.push(r);
            }
        }
        ratios.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        ratios.dedup_by(|a, b| (*a - *b).abs() <= 1e-9 * b.abs().max(1.0));
        ratios
    }

    /// The accumulation plan and tax reserve, stated, not gates.
    fn stated_not_gates(&self) -> (Value, Value, Value) {
        let data = &self.env.data;
        let plan = data.derived_metrics.accumulation_plan.as_ref();
        let plan_unspent = plan.map(|p| {
            if p.progress_unknown {
                json!({ "usdc": p.budget_usdc, "note": "the ledger was not read this cycle: the whole budget is the most it can be" })
            } else {
                json!({ "usdc": p.remaining_usdc, "note": Value::Null })
            }
        });
        (
            plan_unspent.map(|p| json!({ "usdc": p["usdc"], "note": p["note"], "is": NOT_A_GATE })).unwrap_or(Value::Null),
            if plan.is_none() { json!("no accumulation_plan is configured") } else { Value::Null },
            data.derived_metrics.sovereign_ledger.tax_reserve_usd.trim().parse::<f64>().ok().map(|v| json!({ "usd": fx(v, 2), "is": NOT_A_GATE })).unwrap_or(Value::Null),
        )
    }

    /// The exact mint of a snapped rung (review of #166 rungs, finding 4): at most its
    /// USDC and NOCK, truncated to raw units; the liquidity those buy, rounded down with
    /// Uniswap's integer formulas, and the amounts that liquidity pulls, rounded up as the
    /// pool rounds them. Never more than the amounts stated. Null when unsnapped.
    fn exact_mint_json(&self, r: &SizedRung) -> Value {
        use crate::mint_math::*;
        let Some((tl, tu)) = r.range.ticks else {
            return json!({ "status": "unavailable", "reason": "unsnapped: no ticks to mint on" });
        };
        let pool = &self.pool;
        let floor_raw = |v: f64, dec: i32, keep: i32| -> Big {
            // Truncate to `keep` decimals in f64 (exact for these sizes), then scale up.
            let units = (v.max(0.0) * 10f64.powi(keep)).floor() as u128;
            Big::from_u128(units).mul(&Big::from_u128(10u128.pow((dec - keep).max(0) as u32)))
        };
        let usdc_max = floor_raw(r.usdc, pool.usdc_dec, pool.usdc_dec.min(6));
        let nock_max = if r.n0 > 0.0 { floor_raw(r.n0, pool.nock_dec, pool.nock_dec.min(8)) } else { Big::zero() };
        let (max0, max1) = if pool.nock_is_token1 { (usdc_max.clone(), nock_max.clone()) } else { (nock_max.clone(), usdc_max.clone()) };
        let sqrt_p = Big::from_hex(&format!("{:032x}{:032x}", pool.template.sqrt_price_x96.0, pool.template.sqrt_price_x96.1)).unwrap_or(Big::zero());
        match exact_mint(pool.template.tick, &sqrt_p, tl, tu, &max0, &max1) {
            None => json!({ "status": "unavailable", "reason": "these amounts buy no liquidity on these ticks" }),
            Some((l, a0, a1)) => {
                let (u, n) = if pool.nock_is_token1 { (&a0, &a1) } else { (&a1, &a0) };
                json!({
                    "status": "exact",
                    "liquidity_raw": l.to_dec(),
                    "amount0_raw": a0.to_dec(),
                    "amount1_raw": a1.to_dec(),
                    "usdc_raw": u.to_dec(),
                    "nock_raw": n.to_dec(),
                    "usdc": raw_to_decimal(u, pool.usdc_dec as u32),
                    "nock": raw_to_decimal(n, pool.nock_dec as u32),
                    "usdc_max_raw": usdc_max.to_dec(),
                    "nock_max_raw": nock_max.to_dec(),
                    "tick": pool.template.tick,
                    "note": "liquidity_raw is what these amounts buy, rounded down (LiquidityAmounts); amount0_raw and amount1_raw are what a mint of it pulls, rounded up (SqrtPriceMath), never more than the maxima (the rung's USDC and NOCK truncated to raw units)",
                })
            }
        }
    }

    fn rebuild_nock(&self) -> f64 {
        self.env
            .data
            .derived_metrics
            .lp_rebuild
            .suggestions
            .iter()
            .map(|sg| {
                (if sg.token0_symbol == "NOCK" { sg.amount0.parse::<f64>().unwrap_or(0.0) } else { 0.0 })
                    + (if sg.token1_symbol == "NOCK" { sg.amount1.parse::<f64>().unwrap_or(0.0) } else { 0.0 })
            })
            .sum()
    }
}

fn above_spot(l: f64, price: f64) -> LadderError {
    LadderError::not_simulated(
        "RANGE_ABOVE_SPOT",
        format!(
            "the bottom ${} is at or above the pool price ${}: a range above spot holds only NOCK and buys nothing on the way down (see the floor-crossing price, #165)",
            fx(l, 8),
            fx(price, 8)
        ),
    )
}

/// One rung sized at the pool price.
#[derive(Debug, Clone)]
struct SizedRung {
    req: RungRequest,
    usdc: f64,
    range: Range,
    high_rule: &'static str,
    /// Raw liquidity.
    l: f64,
    /// NOCK it needs at the pool price.
    n0: f64,
    bottom_nock: f64,
    top_usdc: f64,
    top_nock: f64,
}

impl SizedRung {
    fn is_deep(&self) -> bool {
        self.req.rung.as_deref() == Some("deep")
    }
    fn is_near(&self) -> bool {
        self.req.rung.as_deref() == Some("near")
    }
    fn bought(&self) -> f64 {
        self.bottom_nock - self.n0
    }
    /// The same range with another amount of USDC: liquidity and NOCK scale with it.
    fn with_usdc(&self, usdc: f64) -> SizedRung {
        let k = usdc / self.usdc;
        SizedRung { usdc, l: self.l * k, n0: self.n0 * k, bottom_nock: self.bottom_nock * k, top_usdc: self.top_usdc * k, top_nock: self.top_nock * k, ..self.clone() }
    }
    /// Whether the pool price at ratio `r` is inside this rung (tickLower <= tick <
    /// tickUpper, in sqrt-price terms).
    fn in_range_at(&self, pool: &LadderPool, r: f64) -> bool {
        let s = pool.s_at(r);
        // Tick edges come from tick_sqrt_price and USD ratios from powi: they agree to
        // ~1e-10, so an edge is matched within 1e-9.
        let tol = 1e-9;
        s >= self.range.sa * (1.0 - tol) && s < self.range.sb * (1.0 - tol)
    }
}

// ------------------------------------------------------------------
// The fee estimate (fee_yield, snapshot 2.10.0)
// ------------------------------------------------------------------

/// The assumptions every fee estimate rests on, stated in its body.
pub(crate) const LADDER_FEE_ASSUMPTIONS: [&str; 8] = [
    "Fee intensity per unit of liquidity is taken as uniform across ticks within the measuring range: the estimate is the measuring range's average fee per unit of liquidity times this range's liquidity, not a measurement inside it (band_measured is the measurement inside it, where there is one).",
    "Fees per unit of liquidity are shared by every range that contains the price (the same pool, the same fee tier).",
    "The pool's swap volume is taken as unchanged by the new liquidity: only its share dilutes.",
    "Dilution uses the pool's active liquidity at the current tick, read at 'latest' in the snapshot's cycle (not at the snapshot's block); the liquidity at the range's own ticks is not read and is taken as the same.",
    "The NOCK fee leg is valued at the current pool price, not at the price when it accrued.",
    "Intervals where the measuring range's liquidity changed or its fees fell (a collect, or a v4 add that collects) are skipped, so the measurement slightly undercounts; the seconds skipped are stated.",
    "Gas is not counted. Past fees are not a forecast.",
    "The stored market volume of another pool (market_price_history) is never used.",
];
const REPLAY_NOTE: &str = "At most: from endpoint ticks sampled every ~5 minutes, an interval counts when both ends sat inside the range (inside) or one did (edge); the price can leave and return within an interval, and a part of a range never earns more per unit of liquidity than the range containing it. No lower bound is claimed (it can be $0).";

impl<'a> FeeCtx<'a> {
    fn new(data: &'a SnapshotData, pool: &LadderPool, spacing_source: Option<&str>, now: DateTime<Utc>) -> FeeCtx<'a> {
        let mut c = FeeCtx { fy: None, pool: None, reason: None, rate: None, active: None, active_raw: None, active_reason: None, age_seconds: None };
        let pool_id = &pool.template.pool_id;
        let am = data.pool_mechanics.iter().find(|m| &m.pool_id == pool_id).and_then(|m| m.active_liquidity_raw.as_deref());
        c.active = am.and_then(|v| v.parse::<u128>().ok()).map(|v| v as f64).filter(|v| *v > 0.0);
        c.active_raw = c.active.and(am.map(|v| v.to_string()));
        if c.active.is_none() {
            c.active_reason = Some(match am {
                Some(_) => "the pool reported no active liquidity at its current tick: no share can be stated".to_string(),
                None => "the pool's active liquidity is not in this snapshot (not read this cycle, or a snapshot before 2.10.0): only the undiluted figure is stated".to_string(),
            });
        }
        let Some(fy) = data.derived_metrics.fee_yield.as_ref() else {
            c.reason = Some("this snapshot has no fee_yield (2.10.0): fees are not measured in it".to_string());
            return c;
        };
        c.fy = Some(fy);
        c.age_seconds = fy
            .computed_at
            .as_deref()
            .and_then(|t| DateTime::parse_from_rfc3339(t).ok())
            .map(|t| now.signed_duration_since(t.with_timezone(&Utc)).num_seconds().max(0));
        if spacing_source == Some("caller") {
            c.reason = Some("fees are measured per pool: a tick spacing you state names no tracked pool (give position_id)".to_string());
            return c;
        }
        if fy.status != "measured" {
            c.reason = Some(fy.reason.clone().unwrap_or_else(|| "fees were not measured".to_string()));
            return c;
        }
        let Some(p) = fy.pools.iter().find(|p| &p.pool_id == pool_id) else {
            let why: Vec<String> = fy
                .excluded
                .iter()
                .filter(|e| data.positions.iter().any(|p| p.position_id == e.position_id && &p.pool_id == pool_id))
                .map(|e| format!("{}: {}", e.position_id, e.reason))
                .collect();
            c.reason = Some(if why.is_empty() {
                "fees are measured per pool; no tracked range in this pool earns LP fees".to_string()
            } else {
                format!("fees are measured per pool; no tracked range in this pool earns LP fees ({})", why.join("; "))
            });
            return c;
        };
        c.pool = Some(p);
        if p.status != "measured" {
            c.reason = Some(p.reason.clone().unwrap_or_else(|| "no interval could be counted".to_string()));
            return c;
        }
        match (p.usdc_per_liquidity_day.as_deref().and_then(|v| v.parse::<f64>().ok()), p.nock_per_liquidity_day.as_deref().and_then(|v| v.parse::<f64>().ok())) {
            (Some(u), Some(n)) => c.rate = Some((u, n)),
            _ => c.reason = Some("the measured rates could not be read".to_string()),
        }
        c
    }

    /// USD per raw liquidity per day at the pool price.
    fn usd_rate(&self, price: f64) -> Option<f64> {
        self.rate.map(|(u, n)| u + n * price)
    }

    /// The fee estimate of one range with liquidity `l` and `value_usd` at spot (its USDC
    /// plus its NOCK at the pool price). `l_new` is the
    /// hypothetical liquidity that would be in range with it (its own and any overlapping
    /// rung's), for the dilution factor. Pure.
    fn estimate(&self, pool: &LadderPool, range: &Range, l: f64, value_usd: f64, l_new: f64) -> Value {
        let price = pool.price;
        let constants = json!({
            "window_days": self.fy.map(|f| f.window_days),
            "max_gap_seconds": self.fy.map(|f| f.max_gap_seconds),
            "day_min_seconds": self.fy.map(|f| f.day_min_seconds),
            "band_min_seconds": LADDER_FEE_BAND_MIN_SECONDS,
            "note": LADDER_DISPLAY_NOTE,
        });
        let mut v = json!({
            "status": "unavailable",
            "reason": Value::Null,
            "measured_by": self.pool.map(|p| p.measured_by.clone()),
            "measured_label": self.pool.map(|p| p.label.clone()),
            "measured_at": self.fy.and_then(|f| f.computed_at.clone()),
            "measurement_age_seconds": self.age_seconds,
            "measured_seconds": self.pool.map(|p| p.measured_seconds),
            "measured_days": self.pool.map(|p| fx(p.measured_seconds / 86400.0, 2)),
            "intervals": self.pool.map(|p| p.intervals),
            "skipped": self.pool.map(|p| json!(p.skipped)),
            "skipped_seconds": self.pool.map(|p| json!(p.skipped_seconds)),
            "rate": Value::Null,
            "per_day_in_range_usd": Value::Null,
            "per_day_in_range_usdc_leg": Value::Null,
            "per_day_in_range_nock_leg": Value::Null,
            "per_100_usd_per_day_in_range_usd": Value::Null,
            "per_day_in_range_diluted_usd": Value::Null,
            "dilution": Value::Null,
            "dilution_reason": Value::Null,
            "spread": Value::Null,
            "spread_reason": Value::Null,
            "band_measured": Value::Null,
            "band_reason": Value::Null,
            "in_range_share_stored_history": Value::Null,
            "in_range_share_reason": Value::Null,
            "stored_history_replay": Value::Null,
            "replay_reason": Value::Null,
            "coverage": Value::Null,
            "constants": constants,
            "assumptions": LADDER_FEE_ASSUMPTIONS,
        });
        let (Some(p), Some((u, n)), Some(rate)) = (self.pool, self.rate, self.usd_rate(price)) else {
            v["reason"] = json!(self.reason.clone().unwrap_or_else(|| "fees are not measured for this pool".to_string()));
            return v;
        };
        // The range's ticks (rounded from its sqrt edges when unsnapped) against the
        // measuring range's own [lower, upper).
        let tick_of = |s: f64| (s * s).ln() / 1.0001f64.ln();
        let (rl, ru) = match range.ticks {
            Some((a, b)) => (a as f64, b as f64),
            None => (tick_of(range.sa), tick_of(range.sb)),
        };
        let (cl, cu) = (p.covered_tick_lower as f64, p.covered_tick_upper as f64);
        let covered_usd = |t: f64| pool.raw_to_usd(1.0001f64.powf(t));
        let (c_lo, c_hi) = (covered_usd(cl.max(rl)), covered_usd(cu.min(ru)));
        let (c_low_usd, c_high_usd) = (c_lo.min(c_hi), c_lo.max(c_hi));
        if ru <= cl || rl >= cu {
            let (m_lo, m_hi) = (covered_usd(cl), covered_usd(cu));
            v["reason"] = json!(format!(
                "this range lies wholly outside the measuring range's ${}..${} ({}): no fees are measured there",
                fx(m_lo.min(m_hi), 8),
                fx(m_lo.max(m_hi), 8),
                p.label
            ));
            return v;
        }
        let whole = rl >= cl && ru <= cu;
        v["coverage"] = json!({
            "low_usd": fx(c_low_usd, 8),
            "high_usd": fx(c_high_usd, 8),
            "whole": whole,
            "note": if whole {
                json!("the whole range lies inside the measuring range")
            } else {
                json!(format!(
                    "only ${}..${} of this range lies inside the measuring range ({}); outside it no fees are measured, and these figures apply only while the price is inside that part",
                    fx(c_low_usd, 8),
                    fx(c_high_usd, 8),
                    p.label
                ))
            },
        });
        v["status"] = json!("measured");
        v["rate"] = json!({
            "usdc_per_liquidity_day": sci(u),
            "nock_per_liquidity_day": sci(n),
            "usd_per_1e15_liquidity_day": fx(rate * 1e15, 4),
            "valued_at_pool_price_usd": fx(price, 8),
        });
        let per_day = l * rate;
        v["per_day_in_range_usd"] = json!(fx(per_day, 2));
        v["per_day_in_range_usdc_leg"] = json!(fx(l * u, 2));
        v["per_day_in_range_nock_leg"] = json!(fx(l * n, 4));
        v["per_100_usd_per_day_in_range_usd"] = json!(fx(per_day / value_usd * 100.0, 2));
        match self.active {
            Some(a) => {
                let factor = a / (a + l_new);
                v["per_day_in_range_diluted_usd"] = json!(fx(per_day * factor, 2));
                v["dilution"] = json!({
                    "active_liquidity_raw": self.active_raw.clone(),
                    "new_liquidity_raw": format!("{}", l_new.round().max(0.0) as u128),
                    "factor": fx(factor, 6),
                    "share_of_active_pct": fx(l_new / (a + l_new) * 100.0, 2),
                });
            }
            None => v["dilution_reason"] = json!(self.active_reason.clone()),
        }
        // A stored row read back from the database has no by-day or band detail: stated,
        // never read as zero.
        if let Some(note) = &p.detail_note {
            let why = format!("this snapshot was read back from the database, whose rows keep no by-day or band detail ({note})");
            v["band_reason"] = json!(why.clone());
            v["in_range_share_reason"] = json!(why.clone());
            v["replay_reason"] = json!(why.clone());
            v["spread_reason"] = json!(why);
            return v;
        }
        // Every UTC day of the measurement, at this range's liquidity; the spread is over
        // the days with at least day_min_seconds measured.
        let day_min = self.fy.map(|f| f.day_min_seconds as f64).unwrap_or(FEE_YIELD_DAY_MIN_SECONDS as f64);
        let mut counted: Vec<f64> = Vec::new();
        let days: Vec<Value> = p
            .by_day
            .iter()
            .filter_map(|d| {
                let du = d.usdc_per_liquidity_day.parse::<f64>().ok()?;
                let dn = d.nock_per_liquidity_day.parse::<f64>().ok()?;
                let usd = l * (du + dn * price);
                let ok = d.seconds >= day_min;
                if ok {
                    counted.push(usd);
                }
                Some(json!({ "date": d.date, "seconds": d.seconds, "usd_per_day": fx(usd, 2), "counted": ok }))
            })
            .collect();
        counted.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let median = match counted.len() {
            0 => None,
            k if k % 2 == 1 => Some(counted[k / 2]),
            k => Some((counted[k / 2 - 1] + counted[k / 2]) / 2.0),
        };
        v["spread"] = json!({
            "days_counted": counted.len(),
            "day_min_seconds": day_min,
            "min_usd_per_day": counted.first().map(|x| fx(*x, 2)),
            "median_usd_per_day": median.map(|x| fx(x, 2)),
            "max_usd_per_day": counted.last().map(|x| fx(*x, 2)),
            "days": days,
            "note": "UTC days; an interval counts on the date it starts. Every day is listed; the spread is over the days counted.",
        });
        // Inside the range's own ticks: from the spans on the pool's grid.
        let grid_ok = match (range.ticks, p.tick_spacing) {
            (None, _) => Err("unsnapped: the range's bounds are not on the pool's tick grid, so the stored spans cannot be split at them".to_string()),
            (Some(_), None) => Err(p.spans_reason.clone().unwrap_or_else(|| "the stored spans have no tick grid".to_string())),
            (Some((tl, tu)), Some(s)) if tl % s != 0 || tu % s != 0 => Err(format!("the range's ticks {tl}..{tu} are not on the measuring pool's {s}-tick grid")),
            (Some(t), Some(_)) => Ok(t),
        };
        match grid_ok {
            Err(why) => {
                v["band_reason"] = json!(why.clone());
                v["in_range_share_reason"] = json!(why.clone());
                v["replay_reason"] = json!(why);
            }
            Ok((tl, tu)) => {
                let (mut s_in, mut s_edge, mut f_in, mut f_edge) = (0.0, 0.0, 0.0, 0.0);
                for sp in &p.spans {
                    let (Ok(su), Ok(sn)) = (sp.usdc_per_liquidity.parse::<f64>(), sp.nock_per_liquidity.parse::<f64>()) else { continue };
                    let usd = su + sn * price;
                    // A bin starts at its tick: the range [tl, tu) holds bins tl..tu-spacing.
                    if sp.bin_lo >= tl && sp.bin_hi < tu {
                        s_in += sp.seconds;
                        f_in += usd;
                    } else if sp.bin_hi >= tl && sp.bin_lo < tu {
                        s_edge += sp.seconds;
                        f_edge += usd;
                    }
                }
                let of = p.measured_seconds;
                v["in_range_share_stored_history"] = json!({
                    "share_pct": fx(if of > 0.0 { s_in / of * 100.0 } else { 0.0 }, 2),
                    "seconds": fx(s_in, 0),
                    "of_seconds": fx(of, 0),
                    "edge_seconds": fx(s_edge, 0),
                    "note": "the share of the measured seconds whose two endpoint ticks were both inside this range: a past measurement, not a forecast",
                });
                v["stored_history_replay"] = json!({
                    "inside_upper_usd": fx(f_in * l, 2),
                    "edge_upper_usd": fx(f_edge * l, 2),
                    "inside_seconds": fx(s_in, 0),
                    "edge_seconds": fx(s_edge, 0),
                    "measured_days": fx(of / 86400.0, 2),
                    "undiluted": true,
                    "note": REPLAY_NOTE,
                });
                if s_in <= 0.0 {
                    v["band_reason"] = json!(format!("no stored interval sat wholly inside ${}..${}", fx(range.low_usd, 8), fx(range.high_usd, 8)));
                } else if s_in < LADDER_FEE_BAND_MIN_SECONDS {
                    v["band_measured"] = json!({
                        "usd_per_day": Value::Null,
                        "usd_total_upper": fx(f_in * l, 2),
                        "seconds": fx(s_in, 0),
                        "days": fx(s_in / 86400.0, 2),
                    });
                    v["band_reason"] = json!(format!(
                        "only {} s of stored intervals sat wholly inside: below the {} s a per-day figure needs, so only the dollars and seconds are stated",
                        fx(s_in, 0),
                        LADDER_FEE_BAND_MIN_SECONDS
                    ));
                } else {
                    v["band_measured"] = json!({
                        "usd_per_day": fx(f_in * l / (s_in / 86400.0), 2),
                        "usd_total_upper": fx(f_in * l, 2),
                        "seconds": fx(s_in, 0),
                        "days": fx(s_in / 86400.0, 2),
                    });
                }
            }
        }
        v
    }
}

// ------------------------------------------------------------------
// The simulation
// ------------------------------------------------------------------

/// The simulation. Pure: the stored snapshot, the caller's inputs, the time now and the
/// worker's health verdict (both only stated). No I/O and no clock.
pub(crate) fn simulate_ladder(env: &SnapshotEnvelope, req: &LadderRequest, now: DateTime<Utc>, health: &str) -> Result<Value, LadderError> {
    let ctx = LadderCtx::new(env, req, now)?;
    if let Some(shape) = &req.shape {
        return simulate_shape(&ctx, req, shape, now, health);
    }
    if req.indexed {
        simulate_rungs(&ctx, req, now, health, None)
    } else {
        simulate_flat(&ctx, req, now, health)
    }
}

/// The one-rung form (#166): its body is unchanged apart from the additive fee_estimate,
/// and, only when operator_profile.ladder_bottom_usd is set, the reserve gate's price.
fn simulate_flat(ctx: &LadderCtx, req: &LadderRequest, now: DateTime<Utc>, health: &str) -> Result<Value, LadderError> {
    let data = &ctx.env.data;
    let gs = &data.derived_metrics.global_status;
    let raw = &ctx.book.raw;
    let pool = &ctx.pool;
    let price = pool.price;
    let spot = ctx.spot;
    let rq = req.first();
    let (range, high_rule) = ctx.resolve_range(&rq.low, &rq.high)?;
    let bounds = bounds_json(&range, price);

    // The wallet.
    let wallet_usdc = ctx.wallet_usdc;
    let wallet_nock_now = ctx.wallet_nock_now;
    if rq.usdc > wallet_usdc + 1e-9 {
        let mut e = LadderError::not_simulated(
            "USDC_EXCEEDS_WALLET",
            format!("${} is more than the ${} of USDC in the wallet", fx(rq.usdc, 2), fx(wallet_usdc, 2)),
        );
        e.bounds = Some(bounds);
        return Err(e);
    }

    // Sizing.
    let Some(sr) = ctx.sized(rq, range, high_rule, rq.usdc) else {
        return Err(above_spot(range.low_usd, price));
    };
    let (l, n0, bottom_nock, top_usdc, top_nock) = (sr.l, sr.n0, sr.bottom_nock, sr.top_usdc, sr.top_nock);
    let nock_check = nock_check(&sr, &bounds)?;
    let ladder_at = |r: f64| pool.amounts(l, pool.s_at(r), range.sa, range.sb);
    let bought = bottom_nock - n0;
    let r_bottom = range.low_usd / price;
    let r_top = range.high_usd / price;

    let existing_ok = ctx.book.problems.is_empty();
    let existing = |r: f64| ctx.existing(r);
    let nock_fees = ctx.nock_fees;
    let configured = ctx.configured;
    let wallet_after = wallet_usdc - rq.usdc;

    // The grid.
    let ratios = ctx.grid_ratios(req, &[r_bottom, r_top]);
    let r_min = 1.0 + req.grid_to_pct / 100.0;
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

    // The reserve check: gated at the ladder's own bottom, or at the operator's ladder
    // bottom when that is set and lower.
    let (r_gate, gate_usd, gate_rule) = ctx.gate(range.low_usd);
    let gate_set = ctx.ladder_bottom.is_some() || ctx.ladder_bottom_note.is_some();
    let r_gate = if gate_set { r_gate } else { r_bottom };
    let at_spot = existing(1.0).map(|(eu, _)| reserve_reading(configured, wallet_usdc, eu));
    let at_bottom = existing(r_gate).map(|(eu, _)| (eu, reserve_reading(configured, wallet_after, eu)));
    // Out of range above, the ladder holds only USDC. A top at or below the pool price is
    // that state now: the reading is at spot, where the ladder leaves the stable balance
    // unchanged. A top above it is read at the top.
    let r_at_top = r_top.max(1.0);
    let at_top = existing(r_at_top).map(|(eu, _)| reserve_reading(configured, wallet_after, top_usdc + eu));
    let r_below = r_gate.min(r_bottom);
    let below = (r_min < r_below - 1e-12)
        .then(|| existing(r_min).map(|(eu, _)| reserve_reading(configured, wallet_after, eu)))
        .flatten();
    let (status, reason) = reserve_status(existing_ok, &ctx.book.problems, configured, at_bottom.and_then(|(_, (_, h))| h));
    let reserve_amount = ctx.reserve_amount();
    let max_usdc = match (reserve_amount, at_bottom) {
        (Some(res), Some((eu, _))) => Some(floor_cents(wallet_usdc + eu - res).min(floor_cents(wallet_usdc))),
        _ => None,
    };
    let (plan_unspent, plan_reason, tax) = ctx.stated_not_gates();
    let mut reserve_check = json!({
        "status": status,
        "gate": if ctx.ladder_bottom.is_some() { RESERVE_GATE_CONFIGURED_NOTE } else if gate_set { RESERVE_GATE_INVALID_BOTTOM_NOTE } else { RESERVE_GATE_NOTE },
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
        "accumulation_plan_unspent": plan_unspent,
        "accumulation_plan_unspent_reason": plan_reason,
        "tax_reserve_usd": tax,
        "reason": reason,
    });
    if gate_set {
        reserve_check["gate_price_usd"] = json!(fx(gate_usd, 8));
        reserve_check["gate_price_rule"] = json!(gate_rule);
    }

    // The floor check: the ladder's NOCK against the floor-plus-tolerance budget itself.
    let budget = ctx.budget;
    let rebuild_nock = ctx.rebuild_nock();
    let fits = budget.map(|b| n0 <= b.max(0.0) + 1e-9);
    let floor_status = match fits {
        None => "unknown",
        Some(true) => "fits",
        Some(false) => "exceeds",
    };

    // The hypothetical range as one more read, for the floor-crossing price.
    let ladder_raw = ctx.hypothetical(&sr, LADDER_POSITION_ID.to_string());
    let effect = floor_breach_effect(data, &ctx.book, std::slice::from_ref(&ladder_raw), n0, rq.usdc, range.ticks.is_none());

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
            (Some(false), Some(b)) if n0 > 0.0 => Some(fx(floor_cents(rq.usdc * b.max(0.0) / n0), 2)),
            _ => None,
        },
        "floor_breach_effect": effect,
        "reason": if budget.is_none() { json!("this snapshot predates lp_nock_budget (2.9.0)") } else { Value::Null },
    });

    // The tops next to spot on a snapped grid, each with its NOCK need.
    let (top_neighbours, top_neighbours_reason) = match (ctx.spacing, range.ticks) {
        (Some(s), Some((tl, tu))) => {
            // The bottom's tick: the higher tick when NOCK is token1.
            let t_low = if pool.nock_is_token1 { tu } else { tl };
            let t_at = pool.tick_at_or_below_price(s);
            let rows: Vec<Value> = [t_at, pool.tick_above(t_at, s)]
                .iter()
                .map(|&t| {
                    let p_usd = pool.usd_at_tick(t);
                    let need = (t != t_low).then(|| pool.range_from_ticks(t_low, t)).and_then(|rg| pool.size(&rg, rq.usdc)).map(|(_, n)| n);
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

    // Tracked ladders (#167) among the existing NOCK ranges; absent with none.
    if !raw.ladders.is_empty() {
        let mut ids: Vec<&String> = raw.ladders.keys().collect();
        ids.sort();
        reserve_check["existing_ladders"] = json!(ids);
    }
    let fee_estimate = ctx.fee.estimate(pool, &range, l, rq.usdc + n0 * price, l);
    Ok(json!({
        "snapshot": ctx.snapshot_block(now, health),
        "inputs": {
            "usdc": fx(rq.usdc, 2),
            "nock": rq.nock.map(|n| format!("{n}")),
            "low_requested": bound_text(&rq.low),
            "high_requested": bound_text(&rq.high),
            "position_id": req.position_id,
            "tick_spacing": req.tick_spacing,
            "grid_to_pct": req.grid_to_pct,
            "grid_step_pct": req.grid_step_pct,
        },
        "pool": ctx.pool_block(),
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
            "liquidity_raw": format!("{}", l.round().max(0.0) as u128),
            "liquidity_human": fx(l / 10f64.powf((pool.usdc_dec + pool.nock_dec) as f64 / 2.0), 4),
            "usdc_at_spot": fx(rq.usdc, 2),
            "nock_needed_at_spot": fx(n0, 4),
            "nock_given": rq.nock.map(|n| format!("{n}")),
            "nock_check": nock_check,
        },
        "grid": grid,
        "bottom": {
            "price_usd": fx(range.low_usd, 8),
            "pct_of_spot": pct((r_bottom - 1.0) * 100.0),
            "nock_held": fx(bottom_nock, 4),
            "nock_bought": fx(bought, 4),
            "avg_buy_price_usd": (bought > 0.0).then(|| fx(rq.usdc / bought, 8)),
        },
        "top": {
            "price_usd": fx(range.high_usd, 8),
            "pct_of_spot": pct((r_top - 1.0) * 100.0),
            "multiple_of_spot": fx(r_top, 4),
            "usdc_held": fx(top_usdc, 2),
            "out_of_range_above_now": r_top <= 1.0 + 1e-12,
            "avg_sell_price_usd": (n0 > 0.0).then(|| fx((top_usdc - rq.usdc) / n0, 8)),
        },
        "reserve_check": reserve_check,
        "floor_check": floor_check,
        "fee_estimate": fee_estimate,
        "assumptions": LADDER_ASSUMPTIONS,
        "statement": LADDER_STATEMENT,
    }))
}

/// The given NOCK checked against what the rung needs. Err is the 422.
fn nock_check(sr: &SizedRung, bounds: &Value) -> Result<&'static str, LadderError> {
    match sr.req.nock {
        None => Ok("not_given"),
        Some(n) => {
            let n0 = sr.n0;
            let tol = LADDER_NOCK_MATCH_TOLERANCE_NOCK.max(n0 * LADDER_NOCK_MATCH_TOLERANCE_BPS / 10000.0);
            let wrong = if n < n0 - tol { Some("NOCK_TOO_LITTLE") } else if n > n0 + tol { Some("NOCK_TOO_MUCH") } else { None };
            if let Some(code) = wrong {
                let mut e = LadderError::not_simulated(
                    code,
                    format!("this range at the pool price takes {} NOCK with ${} USDC; {} was given", fx(n0, 4), fx(sr.usdc, 2), n),
                );
                e.nock_needed = Some(n0);
                e.bounds = Some(bounds.clone());
                return Err(e.of_rung(&sr.req));
            }
            Ok("matches")
        }
    }
}

/// The reserve verdict at the gate. Pure.
fn reserve_status(existing_ok: bool, problems: &[String], configured: Option<f64>, margin: Option<f64>) -> (&'static str, Option<String>) {
    if !existing_ok {
        return ("unknown", Some(problems.join("; ")));
    }
    match &configured {
        None => ("not_configured", Some("operator_profile.cash_reserve_usdc is not set: no reserve rule".to_string())),
        Some(v) if !v.is_finite() || *v < 0.0 => ("invalid", Some(RESERVE_INVALID_NOTE.to_string())),
        Some(_) => match margin {
            Some(h) if h >= 0.0 => ("holds", None),
            Some(_) => ("breached", None),
            None => ("unknown", Some("the reserve could not be measured".to_string())),
        },
    }
}

const RESERVE_GATE_INVALID_BOTTOM_NOTE: &str = "operator_profile.ladder_bottom_usd is set but invalid and not applied: the gate falls back to the stable balance at the ladder's own bottom.";
const RESERVE_GATE_CONFIGURED_NOTE: &str = "The gate is the stable balance at the lower of the ladder's deepest bottom and your ladder bottom (operator_profile.ladder_bottom_usd); the *_at_bottom figures are read at gate_price_usd.";
const RESERVE_GATE_RUNGS_NOTE: &str = "The gate is the stable balance where every rung is fully NOCK: at the lower of the deepest rung's bottom and your ladder bottom (operator_profile.ladder_bottom_usd) when it is set, otherwise at the deepest rung's bottom; the *_at_bottom figures are read at gate_price_usd.";
/// The assumptions a ladder of rungs adds to LADDER_ASSUMPTIONS.
pub(crate) const LADDER_RUNG_ASSUMPTIONS: [&str; 3] = [
    "Each rung is its own position (its own NFT) in the same pool; overlapping rungs are allowed and their liquidity adds.",
    "The rungs share one reserve gate and one NOCK budget.",
    "A deep rung keeps its NOCK on a rebound only if it is pulled when it is fully NOCK; pulling is a withdrawal you sign, and its gas is not counted.",
];

/// The combined measurements of a set of rungs: what a trade-off row and the main body
/// share. Pure.
struct RungsMeasure {
    total_usdc: f64,
    total_n0: f64,
    deepest_low_usd: f64,
    r_gate: f64,
    gate_usd: f64,
    gate_rule: String,
    /// (existing ranges' USDC at the gate, stable there, headroom there)
    at_gate: Option<(f64, (f64, Option<f64>))>,
    nock_at_deepest: f64,
    bought_at_deepest: f64,
    kept: f64,
    kept_bought: f64,
    kept_usdc: f64,
    deep_rungs: usize,
    max_total: Option<f64>,
}

impl LadderCtx<'_> {
    /// `measure` with the gate read at `gate_low` (the whole ladder's deepest bottom) when
    /// given, so every trade-off row is gated at the same price.
    fn measure_at(&self, rungs: &[SizedRung], gate_low: Option<f64>) -> RungsMeasure {
        let total_usdc: f64 = rungs.iter().map(|r| r.usdc).sum();
        let total_n0: f64 = rungs.iter().map(|r| r.n0).sum();
        let deepest_low_usd = rungs.iter().map(|r| r.range.low_usd).fold(f64::INFINITY, f64::min);
        let (r_gate, gate_usd, gate_rule) = self.gate(gate_low.unwrap_or(deepest_low_usd).min(deepest_low_usd));
        let wallet_after = self.wallet_usdc - total_usdc;
        let at_gate = self.existing(r_gate).map(|(eu, _)| (eu, reserve_reading(self.configured, wallet_after, eu)));
        let deep: Vec<&SizedRung> = rungs.iter().filter(|r| r.is_deep()).collect();
        let max_total = match (self.reserve_amount(), at_gate) {
            (Some(res), Some((eu, _))) => Some(floor_cents(self.wallet_usdc + eu - res).min(floor_cents(self.wallet_usdc))),
            _ => None,
        };
        RungsMeasure {
            total_usdc,
            total_n0,
            deepest_low_usd,
            r_gate,
            gate_usd,
            gate_rule,
            at_gate,
            nock_at_deepest: rungs.iter().map(|r| r.bottom_nock).sum(),
            bought_at_deepest: rungs.iter().map(|r| r.bought()).sum(),
            kept: deep.iter().map(|r| r.bottom_nock).sum(),
            kept_bought: deep.iter().map(|r| r.bought()).sum(),
            kept_usdc: deep.iter().map(|r| r.usdc).sum(),
            deep_rungs: deep.len(),
            max_total,
        }
    }

    /// The liquidity of the other rungs that overlap rung `i`, plus its own.
    fn l_with_overlaps(rungs: &[SizedRung], i: usize) -> f64 {
        let a = &rungs[i];
        rungs
            .iter()
            .enumerate()
            .filter(|(j, b)| *j == i || (b.range.sa < a.range.sb && a.range.sa < b.range.sb))
            .map(|(_, b)| b.l)
            .sum()
    }

    /// One trade-off row: the rungs as varied, every column measured. No ranking.
    fn tradeoff_row(&self, rungs: &[SizedRung], all: &[SizedRung]) -> Value {
        let price = self.price();
        // Every row is gated where the whole ladder is (review: a row that drops a rung
        // would otherwise read its margin at another price).
        let gate_low = all.iter().map(|r| r.range.low_usd).fold(f64::INFINITY, f64::min);
        let m = self.measure_at(rungs, Some(gate_low));
        let per_rung: Vec<Value> = all
            .iter()
            .map(|a| {
                let Some(i) = rungs.iter().position(|r| r.req.index == a.req.index) else {
                    return json!({
                        "index": a.req.index, "rung": a.req.rung, "usdc": "0.00",
                        "low_usd": fx(a.range.low_usd, 8), "high_usd": fx(a.range.high_usd, 8),
                        "fee_per_day_in_range_usd": Value::Null, "fee_per_day_in_range_diluted_usd": Value::Null,
                        "fee_per_100_usd_per_day_in_range_usd": Value::Null, "in_range_share_pct": Value::Null,
                        "replay_inside_upper_usd": Value::Null, "replay_edge_upper_usd": Value::Null,
                        "band_usd_per_day": Value::Null, "coverage_whole": Value::Null, "note": "no USDC in this rung in this row",
                    });
                };
                let r = &rungs[i];
                let fe = self.fee.estimate(&self.pool, &r.range, r.l, r.usdc + r.n0 * self.pool.price, Self::l_with_overlaps(rungs, i));
                json!({
                    "index": r.req.index,
                    "rung": r.req.rung,
                    "usdc": fx(r.usdc, 2),
                    "low_usd": fx(r.range.low_usd, 8),
                    "high_usd": fx(r.range.high_usd, 8),
                    "fee_per_day_in_range_usd": fe["per_day_in_range_usd"],
                    "fee_per_day_in_range_diluted_usd": fe["per_day_in_range_diluted_usd"],
                    "fee_per_100_usd_per_day_in_range_usd": fe["per_100_usd_per_day_in_range_usd"],
                    "in_range_share_pct": fe.pointer("/in_range_share_stored_history/share_pct").cloned().unwrap_or(Value::Null),
                    "replay_inside_upper_usd": fe.pointer("/stored_history_replay/inside_upper_usd").cloned().unwrap_or(Value::Null),
                    "replay_edge_upper_usd": fe.pointer("/stored_history_replay/edge_upper_usd").cloned().unwrap_or(Value::Null),
                    "band_usd_per_day": fe.pointer("/band_measured/usd_per_day").cloned().unwrap_or(Value::Null),
                    "coverage_whole": fe.pointer("/coverage/whole").cloned().unwrap_or(Value::Null),
                    "note": fe.pointer("/coverage/note").filter(|_| fe.pointer("/coverage/whole") == Some(&json!(false))).cloned().unwrap_or(Value::Null),
                })
            })
            .collect();
        // NOCK the ladder holds at each rung's bottom: how low it buys, at the operator's
        // own depths.
        let mut bottoms: Vec<f64> = rungs.iter().map(|r| r.range.low_usd).collect();
        bottoms.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
        bottoms.dedup_by(|a, b| (*a - *b).abs() <= 1e-12 * b.abs());
        let nock_by_depth: Vec<Value> = bottoms
            .iter()
            .map(|&p| {
                let r = p / price;
                let held: f64 = rungs.iter().map(|x| self.pool.amounts(x.l, self.pool.s_at(r), x.range.sa, x.range.sb).1).sum();
                json!({ "price_usd": fx(p, 8), "pct_of_spot": pct((p / self.spot - 1.0) * 100.0), "nock_held": fx(held, 4) })
            })
            .collect();
        let margin = m.at_gate.and_then(|(_, (_, h))| h);
        let (rs, _) = reserve_status(self.book.problems.is_empty(), &self.book.problems, self.configured, margin);
        json!({
            "per_rung": per_rung,
            "total_usdc": fx(m.total_usdc, 2),
            "nock_at_deepest_bottom": fx(m.nock_at_deepest, 4),
            "avg_buy_price_usd": (m.bought_at_deepest > 0.0).then(|| fx(m.total_usdc / m.bought_at_deepest, 8)),
            "nock_kept_if_deep_rungs_pulled": fx(m.kept, 4),
            "avg_kept_price_usd": (m.kept_bought > 0.0).then(|| fx(m.kept_usdc / m.kept_bought, 8)),
            "nock_by_depth": nock_by_depth,
            "reserve_margin_usdc": margin.map(|h| fx(h, 2)),
            "gate_price_usd": fx(m.gate_usd, 8),
            "reserve_status": rs,
            "nock_needed": fx(m.total_n0, 4),
            "budget_status": match self.budget { None => "unknown", Some(b) if m.total_n0 <= b.max(0.0) + 1e-9 => "fits", Some(_) => "exceeds" },
        })
    }

    /// The trade-off tables of a ladder of rungs (two or more). Measurements only.
    fn tradeoff(&self, req: &LadderRequest, rungs: &[SizedRung]) -> Value {
        let total: f64 = rungs.iter().map(|r| r.usdc).sum();
        // (A) split: exactly two rungs, the same total re-divided.
        let (split, split_reason) = if rungs.len() == 2 {
            let step = req.split_step_pct;
            let mut shares: Vec<f64> = Vec::new();
            let mut k = 0.0;
            while k <= 100.0 + 1e-9 {
                shares.push(k);
                k += step;
            }
            let own = rungs[0].usdc / total * 100.0;
            if !shares.iter().any(|s| (s - own).abs() < 1e-6) {
                shares.push(own);
            }
            shares.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
            let rows: Vec<Value> = shares
                .iter()
                .map(|&s| {
                    let u0 = total * s / 100.0;
                    let set: Vec<SizedRung> = [(0usize, u0), (1usize, total - u0)]
                        .iter()
                        .filter(|(_, u)| *u > 1e-9)
                        .map(|(i, u)| rungs[*i].with_usdc(*u))
                        .collect();
                    let mut row = self.tradeoff_row(&set, rungs);
                    row["first_rung_share_pct"] = json!(fx(s, 2));
                    row
                })
                .collect();
            (json!({ "step_pct": step, "step_note": LADDER_DISPLAY_NOTE, "varies": format!("the share of the ${} total in r1; r2 holds the rest", fx(total, 2)), "rows": rows }), Value::Null)
        } else {
            (Value::Null, json!("the split table is drawn for two rungs"))
        };
        // (B) near depth and (C) deep top: one near and one deep rung, snapped.
        let near = rungs.iter().position(|r| r.is_near());
        let deep = rungs.iter().position(|r| r.is_deep());
        let depth_ok = match (rungs.len(), near, deep, self.spacing) {
            (2, Some(n), Some(d), Some(s)) if rungs[n].range.ticks.is_some() && rungs[d].range.ticks.is_some() => Ok((n, d, s)),
            (2, Some(_), Some(_), _) => Err("the depth tables need snapped rungs (give position_id or tick_spacing)".to_string()),
            _ => Err("the depth tables are drawn for one near and one deep rung".to_string()),
        };
        let (near_depth, near_reason, deep_top, deep_reason) = match depth_ok {
            Err(why) => (Value::Null, json!(why.clone()), Value::Null, json!(why)),
            Ok((n, d, s)) => {
                let pool = &self.pool;
                // Ticks in USD order: a step down in price.
                let down = if pool.nock_is_token1 { s } else { -s };
                let low_tick = |r: &SizedRung| { let (a, b) = r.range.ticks.unwrap(); if pool.nock_is_token1 { b } else { a } };
                let high_tick = |r: &SizedRung| { let (a, b) = r.range.ticks.unwrap(); if pool.nock_is_token1 { a } else { b } };
                let table = |vary: usize, edge_low: bool, from: i32, to: i32, what: &str| -> (Value, Value) {
                    let mut rows = Vec::new();
                    let mut skipped = Vec::new();
                    let mut t = from;
                    let mut count = 0usize;
                    let past = |t: i32| if down > 0 { t > to } else { t < to };
                    while !past(t) {
                        count += 1;
                        if rows.len() < LADDER_TRADEOFF_MAX_ROWS {
                            let r = &rungs[vary];
                            let (lo_t, hi_t) = if edge_low { (t, high_tick(r)) } else { (low_tick(r), t) };
                            let varied = if lo_t == hi_t {
                                None
                            } else {
                                let rg = pool.range_from_ticks(lo_t, hi_t);
                                (rg.low_usd < pool.price).then(|| self.sized(&r.req, rg, "the usable tick", r.usdc)).flatten()
                            };
                            match varied {
                                Some(v) => {
                                    let mut set: Vec<SizedRung> = rungs.to_vec();
                                    set[vary] = v;
                                    let mut row = self.tradeoff_row(&set, rungs);
                                    row[what] = json!(fx(pool.usd_at_tick(t), 8));
                                    rows.push(row);
                                }
                                None => skipped.push(json!({ what: fx(pool.usd_at_tick(t), 8), "reason": "the rung snaps to zero width or above the pool price here" })),
                            }
                        }
                        t += down;
                    }
                    if count == 0 {
                        return (Value::Null, json!("no usable tick lies between the two rungs"));
                    }
                    (
                        json!({
                            "rows": rows,
                            "skipped_rows": skipped,
                            "ticks_between": count,
                            "max_rows": LADDER_TRADEOFF_MAX_ROWS,
                            "max_rows_note": LADDER_DISPLAY_NOTE,
                            "truncated": count > LADDER_TRADEOFF_MAX_ROWS,
                        }),
                        Value::Null,
                    )
                };
                let (a, ar) = table(n, true, high_tick(&rungs[n]) + down, high_tick(&rungs[d]), "near_low_usd");
                let (b, br) = table(d, false, low_tick(&rungs[n]), low_tick(&rungs[d]) - down, "deep_high_usd");
                (a, ar, b, br)
            }
        };
        json!({
            "split": split,
            "split_reason": split_reason,
            "near_depth": near_depth,
            "near_depth_reason": near_reason,
            "deep_top": deep_top,
            "deep_top_reason": deep_reason,
            "statement": LADDER_TRADEOFF_STATEMENT,
            "order": LADDER_TRADEOFF_ORDER,
            "columns_note": "Fee per day in range is always shown with the in-range share of stored history and the replay upper bounds: a high rate while in range is not a rate per day.",
        })
    }
}

/// A ladder of rungs (indexed form): every rung sized in one pool, one shared reserve gate
/// and NOCK budget, each rung's fee estimate, and the trade-off tables.
fn simulate_rungs(ctx: &LadderCtx, req: &LadderRequest, now: DateTime<Utc>, health: &str, gate_low: Option<f64>) -> Result<Value, LadderError> {
    let data = &ctx.env.data;
    let gs = &data.derived_metrics.global_status;
    let raw = &ctx.book.raw;
    let pool = &ctx.pool;
    let price = pool.price;
    let spot = ctx.spot;

    let mut ranges: Vec<(Range, &'static str)> = Vec::new();
    for rq in &req.rungs {
        ranges.push(ctx.resolve_range(&rq.low, &rq.high).map_err(|e| e.of_rung(rq))?);
    }
    let total: f64 = req.rungs.iter().map(|r| r.usdc).sum();
    if total > ctx.wallet_usdc + 1e-9 {
        let list: Vec<String> = req.rungs.iter().map(|r| format!("r{} ${}", r.index, fx(r.usdc, 2))).collect();
        return Err(LadderError::not_simulated(
            "RUNGS_EXCEED_WALLET",
            format!("the rungs' ${} ({}) is more than the ${} of USDC in the wallet", fx(total, 2), list.join(", "), fx(ctx.wallet_usdc, 2)),
        ));
    }
    let mut rungs: Vec<SizedRung> = Vec::new();
    for (rq, (range, rule)) in req.rungs.iter().zip(ranges) {
        let Some(sr) = ctx.sized(rq, range, rule, rq.usdc) else {
            return Err(above_spot(range.low_usd, price).of_rung(rq));
        };
        nock_check(&sr, &bounds_json(&range, price))?;
        rungs.push(sr);
    }
    let m = ctx.measure_at(&rungs, gate_low);
    let wallet_after = ctx.wallet_usdc - m.total_usdc;
    let existing_ok = ctx.book.problems.is_empty();
    let configured = ctx.configured;

    // The grid: the steps plus every rung's top and bottom and the gate.
    let mut extra: Vec<f64> = vec![m.r_gate];
    for r in &rungs {
        extra.push(r.range.low_usd / price);
        extra.push(r.range.high_usd / price);
    }
    let ratios = ctx.grid_ratios(req, &extra);
    let r_min = 1.0 + req.grid_to_pct / 100.0;
    let same = |a: f64, b: f64| (a - b).abs() <= 1e-9 * b.abs().max(1.0);
    let rate = ctx.fee.usd_rate(price);
    let measured_rung: Vec<bool> = rungs
        .iter()
        .map(|r| ctx.fee.estimate(pool, &r.range, r.l, r.usdc + r.n0 * price, r.l)["status"] == "measured")
        .collect();
    let grid: Vec<Value> = ratios
        .iter()
        .map(|&r| {
            let per: Vec<(f64, f64, bool)> = rungs
                .iter()
                .map(|x| {
                    let (u, n) = if same(r, x.range.low_usd / price) {
                        (0.0, x.bottom_nock)
                    } else if same(r, x.range.high_usd / price) {
                        (x.top_usdc, x.top_nock)
                    } else {
                        pool.amounts(x.l, pool.s_at(r), x.range.sa, x.range.sb)
                    };
                    (u, n, x.in_range_at(pool, r))
                })
                .collect();
            let lu: f64 = per.iter().map(|p| p.0).sum();
            let ln: f64 = per.iter().map(|p| p.1).sum();
            let ex = ctx.existing(r);
            let stable = ex.map(|(eu, _)| reserve_reading(configured, wallet_after, lu + eu));
            let mut markers: Vec<String> = Vec::new();
            if same(r, 1.0) {
                markers.push("spot".to_string());
            }
            for x in &rungs {
                if same(r, x.range.high_usd / price) {
                    markers.push(format!("r{} top", x.req.index));
                }
                if same(r, x.range.low_usd / price) {
                    markers.push(format!("r{} bottom", x.req.index));
                }
            }
            if same(r, m.r_gate) {
                markers.push("gate".to_string());
            }
            let in_idx: Vec<usize> = (0..rungs.len()).filter(|i| per[*i].2).collect();
            let l_new: f64 = in_idx.iter().map(|i| rungs[*i].l).sum();
            // A row states a fee figure only where every rung in range there is measured at
            // that price; otherwise null with the reason, never a partial sum or a zero.
            let row_tick = { let s = pool.s_at(r); (s * s).ln() / 1.0001f64.ln() };
            let outside_measured = ctx.fee.pool.map_or(false, |p| row_tick < p.covered_tick_lower as f64 || row_tick >= p.covered_tick_upper as f64);
            let (est, est_dil, est_reason) = match rate {
                _ if in_idx.is_empty() => (Some(0.0), Some(0.0), Some("no rung is in range at this price, so none earns fees".to_string())),
                None => (None, None, ctx.fee.reason.clone()),
                Some(rt) => {
                    let mut why: Vec<String> = in_idx
                        .iter()
                        .filter(|i| !measured_rung[**i])
                        .map(|i| format!("r{} lies wholly outside the measuring range: its fees are not measured", rungs[*i].req.index))
                        .collect();
                    if outside_measured {
                        why.push(format!("at ${} the price is outside the measuring range: no fees are measured here", fx(price * r, 8)));
                    }
                    if !why.is_empty() {
                        (None, None, Some(why.join("; ")))
                    } else {
                        let f: f64 = in_idx.iter().map(|i| rungs[*i].l * rt).sum();
                        let dil = ctx.fee.active.map(|a| f * a / (a + l_new));
                        (Some(f), dil, dil.is_none().then(|| ctx.fee.active_reason.clone().unwrap_or_default()))
                    }
                }
            };
            json!({
                "price_usd": fx(spot * r, 8),
                "pool_price_usd": fx(price * r, 8),
                "pct_of_spot": pct((r - 1.0) * 100.0),
                "ladder_nock": fx(ln, 4),
                "ladder_nock_acquired": fx(ln - m.total_n0, 4),
                "ladder_usdc": fx(lu, 2),
                "existing_nock_lp_usdc": ex.map(|e| fx(e.0, 2)),
                "existing_nock_lp_nock": ex.map(|e| fx(e.1, 4)),
                "stable_usdc": stable.map(|s| fx(s.0, 2)),
                "reserve_headroom_usdc": stable.and_then(|s| s.1).map(|h| fx(h, 2)),
                "total_tracked_nock": ex.map(|e| fx(ctx.wallet_nock_now - m.total_n0 + ln + e.1 + ctx.nock_fees, 2)),
                "markers": markers,
                "rungs": rungs.iter().zip(&per).map(|(x, p)| json!({ "index": x.req.index, "ladder_usdc": fx(p.0, 2), "ladder_nock": fx(p.1, 4), "in_range": p.2 })).collect::<Vec<_>>(),
                "rungs_in_range": in_idx.iter().map(|i| rungs[*i].req.index).collect::<Vec<_>>(),
                "est_fee_usd_per_day": est.map(|f| fx(f, 2)),
                "est_fee_usd_per_day_diluted": est_dil.map(|f| fx(f, 2)),
                "est_fee_reason": est_reason,
            })
        })
        .collect();

    // The reserve, shared, at the gate.
    let at_spot = ctx.existing(1.0).map(|(eu, _)| reserve_reading(configured, ctx.wallet_usdc, eu));
    let r_top_max = rungs.iter().map(|r| r.range.high_usd / price).fold(0.0f64, f64::max);
    let r_at_top = r_top_max.max(1.0);
    let top_usdc_total: f64 = rungs.iter().map(|x| pool.amounts(x.l, pool.s_at(r_at_top), x.range.sa, x.range.sb).0).sum();
    let all_above = r_top_max <= 1.0 + 1e-12;
    let at_top = ctx.existing(r_at_top).map(|(eu, _)| reserve_reading(configured, wallet_after, top_usdc_total + eu));
    let below = (r_min < m.r_gate - 1e-12)
        .then(|| ctx.existing(r_min).map(|(eu, _)| reserve_reading(configured, wallet_after, eu)))
        .flatten();
    let margin = m.at_gate.and_then(|(_, (_, h))| h);
    let (status, reason) = reserve_status(existing_ok, &ctx.book.problems, configured, margin);
    let (plan_unspent, plan_reason, tax) = ctx.stated_not_gates();
    let mut reserve_check = json!({
        "status": status,
        "gate": RESERVE_GATE_RUNGS_NOTE,
        "gate_price_usd": fx(m.gate_usd, 8),
        "gate_price_rule": m.gate_rule,
        "cash_reserve_usdc": ctx.reserve_amount().map(|v| fx(v, 2)),
        "stable_at_spot_usdc": at_spot.map(|s| fx(s.0, 2)),
        "headroom_at_spot_usdc": at_spot.and_then(|s| s.1).map(|h| fx(h, 2)),
        "existing_ranges_usdc_at_bottom": m.at_gate.map(|(eu, _)| fx(eu, 2)),
        "stable_at_bottom_usdc": m.at_gate.map(|(_, s)| fx(s.0, 2)),
        "margin_usdc": margin.map(|h| fx(h, 2)),
        "max_usdc_for_this_range": m.max_total.map(|v| fx(v, 2)),
        "at_top": {
            "price_usd": fx(spot * r_at_top, 8),
            "out_of_range_above_now": all_above,
            "ladder_usdc": fx(top_usdc_total, 2),
            "stable_usdc": at_top.map(|s| fx(s.0, 2)),
            "headroom_usdc": at_top.and_then(|s| s.1).map(|h| fx(h, 2)),
            "note": if all_above {
                "Every rung is out of range above now and holds only USDC: the stable balance at spot is unchanged by them, and a rise takes none of it."
            } else {
                "Out of range above (read at the highest top), every rung holds only USDC: its NOCK has sold on the way up."
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
            json!("the grid ends at or above the gate")
        },
        "accumulation_plan_unspent": plan_unspent,
        "accumulation_plan_unspent_reason": plan_reason,
        "tax_reserve_usd": tax,
        "reason": reason,
    });
    if !raw.ladders.is_empty() {
        let mut ids: Vec<&String> = raw.ladders.keys().collect();
        ids.sort();
        reserve_check["existing_ladders"] = json!(ids);
    }

    // The NOCK budget, shared.
    let budget = ctx.budget;
    let rebuild_nock = ctx.rebuild_nock();
    let fits = budget.map(|b| m.total_n0 <= b.max(0.0) + 1e-9);
    let hyps: Vec<RawPosition> = rungs.iter().map(|r| ctx.hypothetical(r, format!("{LADDER_POSITION_ID}:r{}", r.req.index))).collect();
    let any_unsnapped = rungs.iter().any(|r| r.range.ticks.is_none());
    let effect = floor_breach_effect(data, &ctx.book, &hyps, m.total_n0, m.total_usdc, any_unsnapped);
    let floor_check = json!({
        "status": match fits { None => "unknown", Some(true) => "fits", Some(false) => "exceeds" },
        "lp_nock_budget": budget.map(|b| fx(b, 4)),
        "budget_note": gs.lp_nock_budget_note,
        "nock_needed": fx(m.total_n0, 4),
        "nock_needed_by_rung": rungs.iter().map(|r| json!({ "index": r.req.index, "nock_needed": fx(r.n0, 4) })).collect::<Vec<_>>(),
        "rebuild_suggested_nock": fx(rebuild_nock, 4),
        "rebuild_note": format!(
            "the rebuild's suggestions this cycle also draw {} NOCK of the same budget: an alternative use of it, not subtracted",
            fx(rebuild_nock, 0)
        ),
        "max_usdc_within_budget": match (fits, budget) {
            (Some(false), Some(b)) if m.total_n0 > 0.0 => Some(fx(floor_cents(m.total_usdc * b.max(0.0) / m.total_n0), 2)),
            _ => None,
        },
        "floor_breach_effect": effect,
        "reason": if budget.is_none() { json!("this snapshot predates lp_nock_budget (2.9.0)") } else { Value::Null },
    });

    // Per rung.
    let rung_bodies: Vec<Value> = rungs
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let b = bounds_json(&r.range, price);
            let r_bottom = r.range.low_usd / price;
            let r_top = r.range.high_usd / price;
            let mut v = json!({
                "index": r.req.index,
                "rung": r.req.rung,
                "inputs": {
                    "usdc": fx(r.usdc, 2),
                    "nock": r.req.nock.map(|n| format!("{n}")),
                    "low_requested": bound_text(&r.req.low),
                    "high_requested": bound_text(&r.req.high),
                },
                "bounds": {
                    "low_usd": b["low_usd"], "high_usd": b["high_usd"], "low_pct": b["low_pct"], "high_pct": b["high_pct"],
                    "tick_lower": b["tick_lower"], "tick_upper": b["tick_upper"], "high_rule": r.high_rule,
                },
                "sizing": {
                    "liquidity_raw": format!("{}", r.l.round().max(0.0) as u128),
                    "liquidity_human": fx(r.l / 10f64.powf((pool.usdc_dec + pool.nock_dec) as f64 / 2.0), 4),
                    "usdc_at_spot": fx(r.usdc, 2),
                    "nock_needed_at_spot": fx(r.n0, 4),
                    "nock_given": r.req.nock.map(|n| format!("{n}")),
                    "nock_check": if r.req.nock.is_some() { "matches" } else { "not_given" },
                },
                "mint": ctx.exact_mint_json(r),
                "bottom": {
                    "price_usd": fx(r.range.low_usd, 8),
                    "pct_of_spot": pct((r_bottom - 1.0) * 100.0),
                    "nock_held": fx(r.bottom_nock, 4),
                    "nock_bought": fx(r.bought(), 4),
                    "avg_buy_price_usd": (r.bought() > 0.0).then(|| fx(r.usdc / r.bought(), 8)),
                },
                "top": {
                    "price_usd": fx(r.range.high_usd, 8),
                    "pct_of_spot": pct((r_top - 1.0) * 100.0),
                    "usdc_held": fx(r.top_usdc, 2),
                    "out_of_range_above_now": r_top <= 1.0 + 1e-12,
                },
                "max_usdc_with_the_others_as_given": m.max_total.map(|t| fx((t - (m.total_usdc - r.usdc)).max(0.0), 2)),
                "fee_estimate": ctx.fee.estimate(pool, &r.range, r.l, r.usdc + r.n0 * price, LadderCtx::l_with_overlaps(&rungs, i)),
                "round_trip": Value::Null,
                "if_pulled_at_fill": Value::Null,
            });
            if r.is_near() || r.req.rung.as_deref() == Some("cap") {
                // Down through the rung to its bottom and back to the price now: the same
                // amounts it started with. Its gain is its fees.
                let (u_back, n_back) = pool.amounts(r.l, pool.s_now, r.range.sa, r.range.sb);
                v["round_trip"] = json!({
                    "down_to_usd": fx(r.range.low_usd, 8),
                    "back_to_usd": fx(price, 8),
                    "usdc_start": fx(r.usdc, 2),
                    "nock_start": fx(r.n0, 4),
                    "usdc_back": fx(u_back, 2),
                    "nock_back": fx(if n_back < LADDER_NOCK_DUST { 0.0 } else { n_back }, 4),
                    "text": format!(
                        "A fall to ${} and a return to ${} (the pool price now) hands back ${} USDC and {} NOCK, what this rung started with: the round trip's gain is the fees it earned in range (fee_estimate), and gas is not counted.",
                        fx(r.range.low_usd, 8),
                        fx(price, 8),
                        fx(u_back, 2),
                        fx(if n_back < LADDER_NOCK_DUST { 0.0 } else { n_back }, 4)
                    ),
                });
            }
            if r.is_deep() {
                v["if_pulled_at_fill"] = json!({
                    "at_usd": fx(r.range.low_usd, 8),
                    "nock_kept": fx(r.bottom_nock, 4),
                    "nock_bought": fx(r.bought(), 4),
                    "avg_price_usd": (r.bought() > 0.0).then(|| fx(r.usdc / r.bought(), 8)),
                    "note": "Pulled when fully NOCK (at its bottom), the rung keeps this NOCK on a rebound; left in place, a rebound sells it back. Pulling is a withdrawal you sign; gas is not counted.",
                });
            }
            v
        })
        .collect();

    // Overlapping rungs: allowed (separate NFTs), stated.
    let mut overlaps: Vec<Value> = Vec::new();
    for i in 0..rungs.len() {
        for j in (i + 1)..rungs.len() {
            let (a, b) = (&rungs[i].range, &rungs[j].range);
            let lo = a.low_usd.max(b.low_usd);
            let hi = a.high_usd.min(b.high_usd);
            if lo < hi - 1e-15 {
                overlaps.push(json!({ "a": rungs[i].req.index, "b": rungs[j].req.index, "low_usd": fx(lo, 8), "high_usd": fx(hi, 8) }));
            }
        }
    }
    let combined = json!({
        "rung_count": rungs.len(),
        "max_rungs": LADDER_MAX_RUNGS,
        "max_rungs_note": LADDER_DISPLAY_NOTE,
        "total_usdc": fx(m.total_usdc, 2),
        "total_nock_needed": fx(m.total_n0, 4),
        "deepest_bottom_usd": fx(m.deepest_low_usd, 8),
        "nock_held_at_deepest_bottom": fx(m.nock_at_deepest, 4),
        "nock_bought_at_deepest_bottom": fx(m.bought_at_deepest, 4),
        "avg_buy_price_usd": (m.bought_at_deepest > 0.0).then(|| fx(m.total_usdc / m.bought_at_deepest, 8)),
        "deep_rungs": m.deep_rungs,
        "nock_kept_if_deep_rungs_pulled": fx(m.kept, 4),
        "nock_bought_by_deep_rungs": fx(m.kept_bought, 4),
        "avg_kept_price_usd": (m.kept_bought > 0.0).then(|| fx(m.kept_usdc / m.kept_bought, 8)),
        "max_usdc_total": m.max_total.map(|v| fx(v, 2)),
        "overlaps": overlaps,
        "gate_price_usd": fx(m.gate_usd, 8),
        "gate_price_rule": m.gate_rule,
    });
    let tradeoff = if rungs.len() >= 2 { ctx.tradeoff(req, &rungs) } else { Value::Null };
    let r_deep = m.deepest_low_usd / price;
    let highest = rungs.iter().map(|r| r.range.high_usd).fold(0.0f64, f64::max);
    let mut assumptions: Vec<&str> = LADDER_ASSUMPTIONS.to_vec();
    assumptions.extend(LADDER_RUNG_ASSUMPTIONS);
    Ok(json!({
        "snapshot": ctx.snapshot_block(now, health),
        "inputs": {
            "usdc": fx(m.total_usdc, 2),
            "nock": Value::Null,
            "low_requested": Value::Null,
            "high_requested": Value::Null,
            "position_id": req.position_id,
            "tick_spacing": req.tick_spacing,
            "grid_to_pct": req.grid_to_pct,
            "grid_step_pct": req.grid_step_pct,
            "split_step_pct": req.split_step_pct,
            "split_step_note": LADDER_DISPLAY_NOTE,
            "rungs": req.rungs.iter().map(|r| json!({
                "index": r.index, "rung": r.rung, "usdc": fx(r.usdc, 2), "nock": r.nock.map(|n| format!("{n}")),
                "low_requested": bound_text(&r.low), "high_requested": bound_text(&r.high),
            })).collect::<Vec<_>>(),
        },
        "pool": ctx.pool_block(),
        "bounds": {
            "low_usd": fx(m.deepest_low_usd, 8),
            "high_usd": fx(highest, 8),
            "low_pct": pct((r_deep - 1.0) * 100.0),
            "high_pct": pct((highest / price - 1.0) * 100.0),
            "tick_lower": Value::Null,
            "tick_upper": Value::Null,
            "high_rule": "per rung: rungs[].bounds.high_rule",
            "describes": "the span from the deepest rung's bottom to the highest rung's top",
        },
        "top_neighbours": Value::Null,
        "top_neighbours_reason": "a ladder of rungs lists no neighbouring tops; each rung's bounds are in rungs[]",
        "sizing": {
            "liquidity_raw": Value::Null,
            "liquidity_human": Value::Null,
            "usdc_at_spot": fx(m.total_usdc, 2),
            "nock_needed_at_spot": fx(m.total_n0, 4),
            "nock_given": Value::Null,
            "nock_check": "per_rung",
            "describes": "the sum over the rungs; liquidity is per rung (rungs[].sizing)",
        },
        "grid": grid,
        "bottom": {
            "price_usd": fx(m.deepest_low_usd, 8),
            "pct_of_spot": pct((r_deep - 1.0) * 100.0),
            "nock_held": fx(m.nock_at_deepest, 4),
            "nock_bought": fx(m.bought_at_deepest, 4),
            "avg_buy_price_usd": (m.bought_at_deepest > 0.0).then(|| fx(m.total_usdc / m.bought_at_deepest, 8)),
            "describes": "the sum over the rungs at the deepest rung's bottom",
        },
        "top": {
            "price_usd": fx(highest, 8),
            "pct_of_spot": pct((highest / price - 1.0) * 100.0),
            "multiple_of_spot": fx(highest / price, 4),
            "usdc_held": fx(top_usdc_total, 2),
            "out_of_range_above_now": all_above,
            "avg_sell_price_usd": (m.total_n0 > 0.0).then(|| fx((top_usdc_total - m.total_usdc) / m.total_n0, 8)),
            "describes": "the sum over the rungs at the highest rung's top",
        },
        "reserve_check": reserve_check,
        "floor_check": floor_check,
        "fee_estimate": Value::Null,
        "fee_estimate_reason": "per rung: rungs[].fee_estimate; each grid row sums the rungs in range there",
        "rungs": rung_bodies,
        "combined": combined,
        "tradeoff": tradeoff,
        "tradeoff_reason": if rungs.len() >= 2 { Value::Null } else { json!("the trade-off tables are drawn for two or more rungs") },
        "assumptions": assumptions,
        "statement": LADDER_STATEMENT,
    }))
}

/// Check `operator_profile.ladder_rungs` (2.10.0). Err names the first problem; an
/// invalid shape is never completed with a default. Rules: at most one `cap`, first, with
/// no share and no low_usd; every near or deep rung has a share above 0 and a low_usd
/// above 0; their shares sum to 1 within 1e-9 (they divide the USDC left after the cap);
/// their bottoms strictly decrease, top down; and the last bottom is at or above
/// `ladder_bottom_usd` when that is set (a shape may stop above it; the reserve gate is
/// still read at the ladder bottom). Pure.
pub(crate) fn validate_ladder_rungs(rungs: &[LadderRungConfig], ladder_bottom_usd: Option<f64>) -> Result<(), String> {
    let bad = |why: String| Err(format!("operator_profile.ladder_rungs is a configuration error: {why}; it is not applied"));
    if rungs.is_empty() {
        return bad("it lists no rung".to_string());
    }
    if rungs.len() > LADDER_MAX_RUNGS {
        return bad(format!("it lists {} rungs; the simulator draws at most {LADDER_MAX_RUNGS}", rungs.len()));
    }
    let mut sum = 0.0;
    let mut last_low: Option<(usize, f64)> = None;
    for (i, r) in rungs.iter().enumerate() {
        let n = i + 1;
        if !LADDER_RUNG_ROLES.contains(&r.rung.as_str()) {
            return bad(format!("rung {n} has unknown rung '{}'. Supported: cap, near, deep", r.rung));
        }
        if r.rung == "cap" {
            if i > 0 {
                return bad(format!("rung {n} is a cap: a cap is the first rung, and there is at most one"));
            }
            if r.share.is_some() || r.low_usd.is_some() {
                return bad("the cap takes no share and no low_usd: it is the tick band containing spot, sized by the LP NOCK budget".to_string());
            }
            continue;
        }
        let Some(share) = r.share else {
            return bad(format!("rung {n} ({}) has no share", r.rung));
        };
        let Some(low) = r.low_usd else {
            return bad(format!("rung {n} ({}) has no low_usd", r.rung));
        };
        if !(share.is_finite() && share > 0.0 && share <= 1.0) {
            return bad(format!("rung {n}'s share {share} is not above 0 and at most 1"));
        }
        if !(low.is_finite() && low > 0.0) {
            return bad(format!("rung {n}'s low_usd {low} is not above 0 USD per NOCK"));
        }
        if let Some((m, prev)) = last_low {
            if !(low < prev) {
                return bad(format!("rung {n}'s low_usd {low} is not below rung {m}'s {prev}: the bottoms must strictly decrease, top down"));
            }
        }
        last_low = Some((n, low));
        sum += share;
    }
    let Some((_, last)) = last_low else {
        return bad("it lists only a cap: the near and deep rungs hold the ladder's USDC".to_string());
    };
    if (sum - 1.0).abs() > 1e-9 {
        return bad(format!("the near and deep shares sum to {sum}, not 1"));
    }
    if let Some(bottom) = ladder_bottom_usd.filter(|v| v.is_finite() && *v > 0.0) {
        if last < bottom {
            return bad(format!(
                "the last rung's low_usd {last} is below operator_profile.ladder_bottom_usd {bottom}: a rung below the ladder bottom would recruit the reserve kept for prices below it"
            ));
        }
    }
    Ok(())
}

/// The validated shape as the snapshot states it: the cap is the band containing spot;
/// the first other rung's top is spot (the usable tick at or below it), each later
/// rung's the rung above's bottom. Pure.
pub(crate) fn ladder_rung_settings(rungs: &[LadderRungConfig]) -> Vec<LadderRungSetting> {
    let mut prev_low: Option<f64> = None;
    rungs
        .iter()
        .map(|r| {
            if r.rung == "cap" {
                return LadderRungSetting { rung: r.rung.clone(), share: None, high: "band".to_string(), low_usd: None };
            }
            let high = prev_low.map(|p| fx(p, 8)).unwrap_or_else(|| "spot".to_string());
            prev_low = r.low_usd;
            LadderRungSetting { rung: r.rung.clone(), share: r.share.map(|s| format!("{s}")), high, low_usd: r.low_usd.map(|v| fx(v, 8)) }
        })
        .collect()
}

/// The least a deposit is worth listing for, and the least a remainder is offered for: the
/// plan's dust threshold, the same $1.00 the single-rung card uses (`minimum_usdc`). A
/// tracked rung whose remainder is worth less is deployed; one worth more than its target by
/// at least this is `over`. A display constant, not a setting.
pub(crate) const LADDER_SHAPE_MIN_USDC: f64 = PLAN_DUST_USDC;
/// A rung is deployed by liquidity: its tracked liquidity is at least its target's, to this
/// relative tolerance.
pub(crate) const LADDER_SHAPE_L_TOLERANCE: f64 = 1e-6;
/// Why the price at or below the ladder's bottom leaves nothing to draw (review of #166 rungs).
pub(crate) const LADDER_SHAPE_PASSED_REASON: &str = "the price is at or below every rung's bottom, so a USDC-only deposit is possible in none of them";
pub(crate) const LADDER_SHAPE_TRACKED_NOTE: &str = "Each configured rung's target is a liquidity on its own ticks; a liquidity does not change with the price, so a deployed rung stays deployed when the price moves, in range or out of it. Tracked ladder rungs in this pool (role: ladder) are matched to configured rungs by role, then by ticks (exact ticks first, otherwise the configured rung of that role in order); a position found on-chain and not yet tracked counts for the rung whose pool and ticks it matches exactly. A near or deep rung is measured by what it holds as all USDC at its top (liquidity times the USDC one unit holds there). The plan is frozen by the first near or deep rung deposited: the shares divide what that rung's USDC at its top implies (the largest, when several are deposited), never the total again. Until then the shares divide what the reserve gate allows now, less the cap. A tracked cap is the cap: its liquidity is its target. What is left to deposit is drawn on a matched rung's own ticks.";
/// How the deposits left are cut when the reserve gate allows less than they need.
pub(crate) const LADDER_SHAPE_FIT_RULE: &str = "once a near or deep rung is deposited the cap takes only what the reserve leaves beside the near and deep rungs still to deposit; when they alone need more than the reserve allows, each deposit left is cut pro rata (by the same fraction), so the shape keeps its proportions, and the shortfall is stated";

/// The cap's band, liquidity and the limit that bound it (`shape=configured`).
struct CapSizing {
    tick_lower: i32,
    tick_upper: i32,
    usdc: f64,
    nock: f64,
    l: f64,
    /// USDC and NOCK per unit of liquidity at the pool price, on this band.
    per_l: (f64, f64),
    bound_by: &'static str,
    /// What the USDC limit is: near_share (not frozen yet: the most that keeps the cap at
    /// or below the near rung), near_usdc (the frozen near rung's USDC), reserve (what the
    /// reserve leaves beside the deposits listed) or total_usdc.
    bound_by_limit: &'static str,
    /// The NOCK budget and the USDC limit it was sized against.
    budget: f64,
    limit: f64,
    note: String,
    band_rule: String,
}

/// A tracked ladder (role: ladder) in the shape's pool, or a position found on-chain that
/// is not tracked yet but matches a rung's pool and ticks exactly.
struct TrackedRung {
    position_id: String,
    rung: Option<String>,
    tl: i32,
    tu: i32,
    /// Its liquidity (never changes with the price).
    l: f64,
    /// What it holds as all USDC at its top: its liquidity times the USDC one unit holds there.
    v_top: f64,
    /// What it holds now (stated, never a sizing input).
    usdc_now: f64,
    nock_now: f64,
    /// Found on-chain, not tracked yet (position_detect.rs).
    found: bool,
    /// The block of its deposit, when its receipt was read: the order rungs were deposited.
    block: Option<i64>,
}

/// One configured rung of the shape, planned against what is tracked.
struct PlanRung {
    pos: usize,
    role: String,
    share: Option<f64>,
    draw: Option<(i32, i32)>,
    passed: bool,
    passed_reason: Option<String>,
    /// Near and deep: the target and the tracked rungs as USDC at the top, and the
    /// liquidity on the draw ticks they come to. The cap: its USDC and NOCK now.
    target_usdc: f64,
    target_nock: f64,
    target_l: Option<f64>,
    tracked_l: f64,
    deployed_usdc: f64,
    deployed_nock: f64,
    counterparts: Vec<String>,
    found: Vec<String>,
    /// The earliest deposit block of its counterparts; None when any is not known.
    first_block: Option<i64>,
    cap: Option<CapSizing>,
    status: &'static str,
    reason: Option<String>,
    note: Option<String>,
    /// What is left: as USDC at the top (near and deep), its liquidity on the draw ticks,
    /// and the amounts that liquidity takes at the pool price now.
    remaining_v: f64,
    remaining_l: f64,
    remaining_usdc: f64,
    remaining_nock: f64,
    /// The fraction the reserve gate's cut left (1 when it fits).
    fit_factor: f64,
    /// Not offered because the reserve leaves too little for it (not because the shape is
    /// small): the card says the reserve leaves no room.
    no_room: bool,
}

impl LadderCtx<'_> {
    /// The band of `spacing` ticks that contains the pool's tick (tickLower <= tick <
    /// tickUpper), and the sentence that says which band and why.
    fn cap_band(&self, spacing: i32) -> (i32, i32, String) {
        let tick = self.pool.template.tick;
        let lo = tick.div_euclid(spacing) * spacing;
        let rule = if tick == lo {
            format!(
                "the pool's tick {tick} is exactly on a band edge: the band {lo}..{} is used, the one that contains it (a range is in range when tickLower <= tick < tickUpper)",
                lo + spacing
            )
        } else {
            format!("the band {lo}..{} of the {spacing}-tick grid contains the pool's tick {tick}", lo + spacing)
        };
        (lo, lo + spacing, rule)
    }

    /// The range of ticks [tl, tu) in USD terms.
    fn range_of_ticks(&self, tl: i32, tu: i32) -> Range {
        let (lo_usd, hi_usd) = if self.pool.nock_is_token1 { (tu, tl) } else { (tl, tu) };
        self.pool.range_from_ticks(lo_usd, hi_usd)
    }

    /// The USDC one unit of liquidity holds at the top of [tl, tu): what a near or deep
    /// rung is measured by.
    fn usdc_per_l_at_top(&self, tl: i32, tu: i32) -> f64 {
        let range = self.range_of_ticks(tl, tu);
        self.pool.amounts(1.0, range.s_top, range.sa, range.sb).0
    }

    /// The cap sized in its band: its NOCK is the LP NOCK budget (`budget`), or less when
    /// the USDC the curve pairs with it would exceed `limit` (the near rung's USDC). Err
    /// is the reason there is no cap.
    fn cap_sizing(&self, band: (i32, i32, String), budget: Option<f64>, limit: f64, limit_name: &'static str) -> Result<CapSizing, String> {
        let (tl, tu, band_rule) = band;
        let budget = match budget {
            None => return Err("lp_nock_budget is not in this snapshot (2.9.0): the cap is sized by it, so there is no cap".to_string()),
            Some(b) if !(b > 0.0) => {
                return Err(format!("the NOCK that may go into LP (lp_nock_budget) is {}: there is no NOCK for a cap, so there is none", fx(b, 4)))
            }
            Some(b) => b,
        };
        let range = self.range_of_ticks(tl, tu);
        let (u1, n1) = self.pool.amounts(1.0, self.pool.s_now, range.sa, range.sb);
        if !(n1 > 0.0) || !(u1 > 0.0) {
            return Err(format!("at the pool price the band {tl}..{tu} holds only one token: it cannot pair NOCK with USDC, so there is no cap"));
        }
        let l_budget = budget / n1;
        let usdc_budget = l_budget * u1;
        let limit = limit.max(0.0);
        let limit_words = match limit_name {
            "near_share" => "most that keeps it at or below the near rung's USDC",
            "near_usdc" => "near rung's USDC",
            "reserve" => "USDC the reserve leaves for it beside the other deposits listed",
            _ => "ladder total",
        };
        // The near rung's share (not frozen) and the near rung's USDC (frozen) are one rule.
        let bound_name = if limit_name == "near_share" { "near_usdc" } else { limit_name };
        let (l, bound_by, note) = if usdc_budget <= limit + 1e-9 {
            (l_budget, "lp_nock_budget", format!("the cap holds the whole LP NOCK budget, {} NOCK; the curve pairs it with ${} USDC", fx(budget, 4), fx(usdc_budget, 2)))
        } else {
            let l = limit / u1;
            (
                l,
                bound_name,
                format!(
                    "the budget's {} NOCK would pair with ${} USDC, more than the {limit_words}, ${}: the cap's liquidity is reduced until its USDC equals that, so it holds {} NOCK, less than the budget",
                    fx(budget, 4),
                    fx(usdc_budget, 2),
                    fx(limit, 2),
                    fx(l * n1, 4)
                ),
            )
        };
        Ok(CapSizing { tick_lower: tl, tick_upper: tu, usdc: l * u1, nock: l * n1, l, per_l: (u1, n1), bound_by, bound_by_limit: limit_name, budget, limit, note, band_rule })
    }

    /// The tracked ladders (role: ladder) in the ladder pool, with what each holds.
    fn tracked_rungs(&self) -> Vec<TrackedRung> {
        let raw = &self.book.raw;
        let mut out: Vec<TrackedRung> = raw
            .positions
            .iter()
            .filter(|p| p.pool_id == self.pool.template.pool_id && raw.ladders.contains_key(&p.position_id))
            .map(|p| {
                let tag = &raw.ladders[&p.position_id];
                let (usdc_now, nock_now) = existing_at_ratio(p, 1.0);
                let l = p.liquidity as f64;
                let block = self.env.data.ladders.iter().find(|r| r.position_id == p.position_id).and_then(|r| r.since_deposit.deposit_block);
                TrackedRung {
                    position_id: p.position_id.clone(),
                    rung: tag.rung.clone(),
                    tl: p.tick_lower,
                    tu: p.tick_upper,
                    l,
                    v_top: l * self.usdc_per_l_at_top(p.tick_lower, p.tick_upper),
                    usdc_now,
                    nock_now,
                    found: false,
                    block,
                }
            })
            .collect();
        out.sort_by(|a, b| a.position_id.cmp(&b.position_id));
        out
    }

    /// Positions found on-chain this cycle and not counted (position_detect.rs), in the
    /// ladder pool, that could be a ladder rung: each with its ticks and liquidity. One
    /// another address minted, or worth less at its top than the minimum, is left out: a
    /// stranger's dust on a rung's ticks never stands for that rung.
    fn found_in_pool(&self) -> Vec<TrackedRung> {
        let pool_id = self.pool.template.pool_id.to_lowercase();
        let wallet = self.pool.template.wallet.to_lowercase();
        let found = self
            .env
            .data
            .derived_metrics
            .position_detection
            .as_ref()
            .and_then(|d| d.pointer("/found_not_counted/positions"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default();
        found
            .iter()
            .filter(|f| f.get("pool_id").and_then(|v| v.as_str()).map(|p| p.to_lowercase()) == Some(pool_id.clone()))
            .filter(|f| f.get("choices").and_then(|c| c.as_array()).is_some_and(|c| c.iter().any(|x| x.as_str().is_some_and(|x| x.starts_with("ladder:")))))
            .filter(|f| f.get("minted_by").and_then(|v| v.as_str()).map_or(true, |by| by.to_lowercase() == wallet))
            .filter_map(|f| {
                let tl = i32::try_from(f.get("tick_lower")?.as_i64()?).ok()?;
                let tu = i32::try_from(f.get("tick_upper")?.as_i64()?).ok()?;
                let l: f64 = f.pointer("/holds/liquidity")?.as_str()?.parse().ok()?;
                let id = f.get("position_id").and_then(|v| v.as_str()).map(str::to_string).unwrap_or_else(|| format!("nft #{}", f.get("nft_id").and_then(|v| v.as_u64()).unwrap_or(0)));
                let range = self.range_of_ticks(tl, tu);
                let (usdc_now, nock_now) = self.pool.amounts(l, self.pool.s_now, range.sa, range.sb);
                let v_top = l * self.usdc_per_l_at_top(tl, tu);
                (tl < tu && l > 0.0 && v_top + nock_now * self.pool.price >= LADDER_SHAPE_MIN_USDC).then(|| TrackedRung {
                    position_id: id,
                    rung: None,
                    tl,
                    tu,
                    l,
                    v_top,
                    usdc_now,
                    nock_now,
                    found: true,
                    block: f.get("mint_block").and_then(|v| v.as_i64()),
                })
            })
            .collect()
    }
}

/// `shape=configured`: the operator's ladder_rungs from the snapshot, planned against the
/// tracked ladder rungs already deposited in the pool (review of the bottle, 2026-11-21):
/// - every configured rung's target is a liquidity on its own ticks, so a deployed rung
///   stays deployed whatever the price does (a cap whose band the price has left included);
/// - the plan is frozen by the first near or deep rung deposited (the shares divide what
///   its USDC at its top implies), never re-divided from the reserve gate each cycle;
/// - what is left is drawn on the rung's own ticks at the price now, then cut pro rata to
///   what the reserve gate allows, with the shortfall stated: never all or nothing;
/// - a remainder worth less than `LADDER_SHAPE_MIN_USDC` is never offered.
/// A configured rung above the price is `passed`, with its own reason; with every rung
/// passed the status is price_at_or_below_bottom. A valid configuration never answers a
/// parameter error.
fn simulate_shape(ctx: &LadderCtx, req: &LadderRequest, shape: &ShapeRequest, now: DateTime<Utc>, health: &str) -> Result<Value, LadderError> {
    let gs = &ctx.env.data.derived_metrics.global_status;
    let Some(setting) = gs.ladder_rungs.as_ref().filter(|r| !r.is_empty()) else {
        return Err(LadderError::not_simulated(
            "SHAPE_NOT_CONFIGURED",
            gs.ladder_rungs_note.clone().unwrap_or_else(|| "operator_profile.ladder_rungs is not set in this snapshot: there is no configured shape".to_string()),
        ));
    };
    let Some(spacing) = ctx.spacing else {
        return Err(LadderError::not_simulated(
            "SHAPE_NEEDS_TICK_SPACING",
            "the configured shape is drawn on the pool's tick grid (the usable tick at or below spot, the cap's band): give position_id",
        ));
    };
    let pool = &ctx.pool;
    let minimum = LADDER_SHAPE_MIN_USDC;
    let unreadable = |i: usize| LadderError::not_simulated("SHAPE_NOT_CONFIGURED", format!("rung {} of the stored shape could not be read", i + 1));
    let usd = |t: i32| pool.usd_at_tick(t);
    let has_cap = setting.first().map_or(false, |r| r.rung == "cap");
    // The setting, by position.
    let mut plan: Vec<PlanRung> = Vec::new();
    let mut lows: Vec<Option<i32>> = Vec::new();
    for (i, r) in setting.iter().enumerate() {
        let (share, low) = if r.rung == "cap" {
            (None, None)
        } else {
            let share = r.share.as_deref().and_then(|v| v.parse::<f64>().ok()).ok_or_else(|| unreadable(i))?;
            let low = r.low_usd.as_deref().and_then(|v| v.parse::<f64>().ok()).ok_or_else(|| unreadable(i))?;
            (Some(share), Some(nearest_usable_tick(pool.tick_f(low), spacing)))
        };
        lows.push(low);
        plan.push(PlanRung {
            pos: i,
            role: r.rung.clone(),
            share,
            draw: None,
            passed: false,
            passed_reason: None,
            target_usdc: 0.0,
            target_nock: 0.0,
            target_l: None,
            tracked_l: 0.0,
            deployed_usdc: 0.0,
            deployed_nock: 0.0,
            counterparts: Vec::new(),
            found: Vec::new(),
            first_block: None,
            cap: None,
            status: "to_deposit",
            reason: None,
            note: None,
            remaining_v: 0.0,
            remaining_l: 0.0,
            remaining_usdc: 0.0,
            remaining_nock: 0.0,
            fit_factor: 1.0,
            no_room: false,
        });
    }
    // Passed rungs and the drawing ticks: the first rung still drawable starts at the
    // usable tick at or below spot; each later one at the rung above's bottom. Each passed
    // rung says why in its own terms.
    let spot_tick = pool.tick_at_or_below_price(spacing);
    let price_words = format!("${}", fx(pool.price, 8));
    let mut prev_low: Option<i32> = None;
    let mut first_drawn = false;
    for (i, p) in plan.iter_mut().enumerate() {
        let Some(t_low) = lows[i] else { continue };
        let bottom_words = format!("${}", fx(usd(t_low), 8));
        if usd(spot_tick) <= usd(t_low) * (1.0 + 1e-12) {
            p.passed = true;
            p.passed_reason = Some(if pool.price <= usd(t_low) * (1.0 + 1e-12) {
                format!("the price {price_words} is at or below this rung's bottom {bottom_words}: a USDC-only deposit there is not possible")
            } else if has_cap {
                format!(
                    "the price {price_words} is less than one tick spacing above this rung's bottom {bottom_words}: the cap's band, the one containing the price, covers all that is left of it, so it has no range of its own"
                )
            } else {
                format!("the price {price_words} is less than one tick spacing above this rung's bottom {bottom_words}: its range would be empty")
            });
        } else {
            let top = if first_drawn { prev_low.unwrap_or(spot_tick) } else { spot_tick };
            first_drawn = true;
            if top != t_low {
                p.draw = Some((top.min(t_low), top.max(t_low)));
            } else {
                p.passed = true;
                p.passed_reason = Some(format!("its top and its bottom {bottom_words} are the same tick, so its range would be empty"));
            }
        }
        prev_low = Some(t_low);
    }
    let all_passed = plan.iter().filter(|p| p.role != "cap").all(|p| p.passed);
    let cap_band = ctx.cap_band(spacing);
    for p in plan.iter_mut().filter(|p| p.role == "cap") {
        p.passed = all_passed;
        if all_passed {
            p.passed_reason = Some("every rung below the cap is passed (the price is at or below the ladder's bottom), so no cap is drawn".to_string());
        } else {
            p.draw = Some((cap_band.0, cap_band.1));
        }
    }
    // The gate: the deepest configured bottom, or the operator's ladder bottom if lower.
    let deepest = lows.iter().flatten().map(|t| usd(*t)).fold(f64::INFINITY, f64::min);
    let (r_gate, gate_usd, gate_rule) = ctx.gate(deepest);
    // Tracked rungs, matched by role, then ticks.
    let tracked = ctx.tracked_rungs();
    let mut owner: Vec<Option<usize>> = vec![None; tracked.len()];
    let mut unmatched: Vec<Value> = Vec::new();
    for (k, t) in tracked.iter().enumerate() {
        if let Some(role) = &t.rung {
            owner[k] = plan.iter().position(|p| &p.role == role && p.draw == Some((t.tl, t.tu)));
        }
    }
    for (k, t) in tracked.iter().enumerate() {
        if owner[k].is_some() {
            continue;
        }
        let reason = match &t.rung {
            None => Some("its rung is not set, so it matches no configured rung".to_string()),
            Some(role) => {
                let taken: Vec<usize> = owner.iter().flatten().copied().collect();
                match plan.iter().position(|p| &p.role == role && !taken.contains(&p.pos)) {
                    Some(i) => {
                        owner[k] = Some(i);
                        None
                    }
                    None if plan.iter().any(|p| &p.role == role) => Some(format!("every configured {role} rung already has a tracked counterpart")),
                    None => Some(format!("the configured shape has no {role} rung")),
                }
            }
        };
        if let Some(reason) = reason {
            unmatched.push(json!({
                "position_id": t.position_id,
                "rung": t.rung,
                "tick_lower": t.tl,
                "tick_upper": t.tu,
                "deployed_usdc": fx(t.usdc_now, 2),
                "reason": reason,
                "note": "it fills no configured rung; the reserve gate counts what it holds like any other range",
            }));
        }
    }
    // A position found on-chain and not tracked yet counts for a rung no tracked ladder
    // holds, when its pool and ticks are exactly that rung's this cycle.
    let found = ctx.found_in_pool();
    let mut found_owner: Vec<Option<usize>> = vec![None; found.len()];
    for (k, f) in found.iter().enumerate() {
        let taken: Vec<usize> = owner.iter().flatten().copied().chain(found_owner.iter().flatten().copied()).collect();
        found_owner[k] = plan.iter().position(|p| !taken.contains(&p.pos) && p.draw == Some((f.tl, f.tu)));
    }
    let counted: Vec<(usize, &TrackedRung)> = owner
        .iter()
        .zip(tracked.iter())
        .chain(found_owner.iter().zip(found.iter()))
        .filter_map(|(o, t)| o.map(|i| (i, t)))
        .collect();
    for (i, t) in &counted {
        let p = &mut plan[*i];
        let first = p.counterparts.is_empty() && p.found.is_empty();
        p.first_block = match (first, p.first_block, t.block) {
            (true, _, b) => b,
            (false, Some(a), Some(b)) => Some(a.min(b)),
            _ => None,
        };
        p.deployed_usdc += if p.role == "cap" { t.usdc_now } else { t.v_top };
        p.deployed_nock += t.nock_now;
        if t.found {
            p.found.push(t.position_id.clone());
        } else {
            p.counterparts.push(t.position_id.clone());
        }
        if first {
            // What is left to deposit keeps the matched rung's own ticks.
            p.draw = Some((t.tl, t.tu));
            if p.role != "cap" {
                let range = ctx.range_of_ticks(t.tl, t.tu);
                p.passed = !(range.low_usd < pool.price);
                p.passed_reason = p.passed.then(|| {
                    format!(
                        "the price {price_words} is below this rung's range (${} to ${}): what is left of it would hold only NOCK, so no USDC deposit is possible",
                        fx(range.low_usd, 8),
                        fx(range.high_usd, 8)
                    )
                });
            } else {
                p.passed = false;
                p.passed_reason = None;
            }
        }
    }
    // Near and deep rungs are measured by their USDC at the top: what their liquidity is
    // worth there, whatever the price does now.
    for (i, t) in &counted {
        let p = &mut plan[*i];
        if p.role != "cap" {
            if let Some((tl, tu)) = p.draw {
                let w = ctx.usdc_per_l_at_top(tl, tu);
                if w > 0.0 {
                    p.tracked_l += t.v_top / w;
                }
            }
        } else {
            p.tracked_l += t.l;
        }
    }
    let v_of = |p: &PlanRung| -> f64 { p.draw.map(|(tl, tu)| p.tracked_l * ctx.usdc_per_l_at_top(tl, tu)).unwrap_or(p.deployed_usdc) };

    // The headroom: what the shared reserve gate allows for what is left to deposit. Every
    // rung, tracked or not, holds no USDC at the gate (it is at or below their bottoms), so
    // this is the wallet's USDC plus every range's USDC at the gate, less the reserve.
    // `headroom` is cut to whole cents for display; `headroom_exact` is what the deposits
    // are fitted to, so a shortfall under a cent is absorbed, never stated as "$0.00".
    let (headroom, headroom_exact, headroom_rule) = match shape.total_usdc {
        Some(t) => (t, t, "given (shape_usdc): what is left to deposit is sized to it".to_string()),
        None => {
            let Some(res) = ctx.reserve_amount() else {
                return Err(LadderError::not_simulated(
                    "SHAPE_TOTAL_UNKNOWN",
                    "no valid operator_profile.cash_reserve_usdc is set, so the reserve gate has no most: give shape_usdc",
                ));
            };
            let Some((eu, _)) = ctx.existing(r_gate) else {
                return Err(LadderError::not_simulated("SHAPE_TOTAL_UNKNOWN", ctx.book.problems.join("; ")));
            };
            (
                floor_cents(ctx.wallet_usdc + eu - res).min(floor_cents(ctx.wallet_usdc)).max(0.0),
                (ctx.wallet_usdc + eu - res).min(ctx.wallet_usdc).max(0.0),
                format!(
                    "the shared reserve gate's most at ${}: the wallet's USDC plus the USDC every range holds there, less the ${} reserve, and never more than the wallet holds",
                    fx(gate_usd, 8),
                    fx(res, 2)
                ),
            )
        }
    };

    // The anchor: the first near or deep rung deposited freezes the plan. With every
    // deposit block known it is the earliest; otherwise the one implying the most, so a
    // rung deposited short asks for its own remainder rather than shrinking the others.
    let held: Vec<(f64, String, f64, Option<i64>)> = plan
        .iter()
        .filter(|p| p.role != "cap" && !(p.counterparts.is_empty() && p.found.is_empty()))
        .filter_map(|p| p.share.filter(|s| *s > 0.0).map(|s| (v_of(p) / s, p.role.clone(), v_of(p), p.first_block)))
        .collect();
    let by_block = !held.is_empty() && held.iter().all(|h| h.3.is_some());
    let anchor: Option<(f64, String, f64)> = held
        .iter()
        .fold(None::<&(f64, String, f64, Option<i64>)>, |acc, x| match acc {
            Some(a) if by_block && (a.3 < x.3 || (a.3 == x.3 && a.0 >= x.0)) => Some(a),
            Some(a) if !by_block && a.0 >= x.0 => Some(a),
            _ => Some(x),
        })
        .map(|a| (a.0, a.1.clone(), a.2));
    let near_share = plan.iter().find(|p| p.role == "near").and_then(|p| p.share);
    let cap_untracked = |p: &PlanRung| p.role == "cap" && p.counterparts.is_empty() && p.found.is_empty();

    // A near or deep rung by liquidity: deployed, over, found, passed, under the minimum or
    // what is left of it at the price now.
    let plan_lower = |p: &mut PlanRung| {
        let held = !(p.counterparts.is_empty() && p.found.is_empty());
        let found_only = p.counterparts.is_empty() && !p.found.is_empty();
        let v_tracked = v_of(p);
        let rem_v = p.target_usdc - v_tracked;
        let tol = (p.target_usdc * LADDER_SHAPE_L_TOLERANCE).max(1e-9);
        (p.status, p.reason) = if held && -rem_v >= minimum.max(tol) {
            ("over", Some(format!("the tracked rung holds ${} of USDC at its top, ${} more than its target", fx(v_tracked, 2), fx(-rem_v, 2))))
        } else if found_only {
            ("found", Some(format!("the position {} is in this pool on exactly this rung's ticks; it is counted once it is tracked, so it is not offered again", p.found.join(", "))))
        } else if held && rem_v <= tol {
            ("deployed", None)
        } else if held && rem_v < minimum {
            p.note = Some(format!("${} of its target is not deposited: under the ${} minimum, so it is not offered", fx(rem_v, 2), fx(minimum, 2)));
            ("deployed", None)
        } else if p.passed {
            ("passed", p.passed_reason.clone())
        } else if rem_v < minimum {
            ("below_minimum", Some(format!("its ${} is under the ${} minimum, so it is not offered", fx(rem_v.max(0.0), 2), fx(minimum, 2))))
        } else {
            p.remaining_v = rem_v;
            let (tl, tu) = p.draw.expect("a rung that is not passed is drawn");
            let w = ctx.usdc_per_l_at_top(tl, tu);
            p.remaining_l = rem_v / w;
            let range = ctx.range_of_ticks(tl, tu);
            // Wholly below the price it holds only USDC: exactly its USDC at the top.
            let (u, n) = if range.high_usd <= pool.price { (rem_v, 0.0) } else { pool.amounts(p.remaining_l, pool.s_now, range.sa, range.sb) };
            p.remaining_usdc = u;
            p.remaining_nock = n;
            ("to_deposit", None)
        };
    };
    let set_targets = |plan: &mut Vec<PlanRung>, rest: f64| {
        for p in plan.iter_mut().filter(|p| p.role != "cap") {
            p.target_usdc = rest * p.share.unwrap_or(0.0);
            p.target_l = p.draw.map(|(tl, tu)| ctx.usdc_per_l_at_top(tl, tu)).filter(|w| *w > 0.0).map(|w| p.target_usdc / w);
        }
    };
    // The cap: tracked, it is the cap; otherwise sized in the band containing spot, by the
    // LP NOCK budget and never more USDC than `limit`.
    let mut cap_reason: Option<String> = None;
    let mut size_cap = |plan: &mut Vec<PlanRung>, limit: f64, limit_name: &'static str| {
        for p in plan.iter_mut().filter(|p| p.role == "cap") {
            if !(p.counterparts.is_empty() && p.found.is_empty()) {
                p.target_usdc = p.deployed_usdc;
                p.target_nock = p.deployed_nock;
                p.target_l = Some(p.tracked_l);
                continue;
            }
            if p.passed {
                continue;
            }
            match ctx.cap_sizing(cap_band.clone(), ctx.budget, limit, limit_name) {
                Ok(c) => {
                    p.target_usdc = c.usdc;
                    p.target_nock = c.nock;
                    p.target_l = Some(c.l);
                    p.cap = Some(c);
                }
                Err(why) => cap_reason = Some(why),
            }
        }
    };
    let (rest, rest_rule) = match &anchor {
        // Frozen: the near and deep rungs first, then the cap within the rule and within
        // what the reserve leaves beside what they still need, so a price move inside the
        // cap's band changes the cap, never them.
        Some((r, role, v)) => {
            set_targets(&mut plan, *r);
            plan.iter_mut().filter(|p| p.role != "cap").for_each(&plan_lower);
            let others: f64 = plan.iter().filter(|p| p.role != "cap" && p.status == "to_deposit").map(|p| p.remaining_usdc).sum();
            let reserve_left = (headroom_exact - others).max(0.0);
            let (rule_name, rule_limit) = match near_share {
                Some(sh) => ("near_usdc", sh * r),
                None => ("total_usdc", headroom_exact),
            };
            let (name, limit) = if reserve_left < rule_limit { ("reserve", reserve_left) } else { (rule_name, rule_limit) };
            size_cap(&mut plan, limit, name);
            (
                *r,
                format!(
                    "frozen by the {role} rung already deposited ({}): its ${} of USDC at its top is its share of ${}, which the near and deep shares divide; it does not move with the price",
                    if by_block { "the first deposited, by its deposit block" } else { "the deposit order is not known for every rung, so the one implying the most" },
                    fx(*v, 2),
                    fx(*r, 2)
                ),
            )
        }
        // Not frozen yet: the cap within the rule, then the shares divide what is left.
        None => {
            let (name, limit) = match near_share {
                Some(sh) => ("near_share", sh * headroom / (1.0 + sh)),
                None => ("total_usdc", headroom),
            };
            size_cap(&mut plan, limit, name);
            let cap_usdc: f64 = plan.iter().filter(|p| cap_untracked(p)).map(|p| p.target_usdc).sum();
            let rest = (headroom - cap_usdc).max(0.0);
            set_targets(&mut plan, rest);
            plan.iter_mut().filter(|p| p.role != "cap").for_each(&plan_lower);
            (rest, "what the reserve gate allows now, less the cap's USDC: no near or deep rung is deposited yet, so this follows the gate until one is".to_string())
        }
    };
    for p in plan.iter_mut().filter(|p| p.role == "cap") {
        let held = !(p.counterparts.is_empty() && p.found.is_empty());
        let found_only = p.counterparts.is_empty() && !p.found.is_empty();
        (p.status, p.reason) = if held {
            if found_only {
                ("found", Some(format!("the position {} is in this pool on exactly this rung's ticks; it is counted once it is tracked, so it is not offered again", p.found.join(", "))))
            } else {
                ("deployed", None)
            }
        } else if p.passed {
            ("passed", p.passed_reason.clone())
        } else if p.cap.is_none() {
            ("none", cap_reason.clone())
        } else if p.target_usdc + p.target_nock * pool.price < minimum {
            let worth = fx(p.target_usdc + p.target_nock * pool.price, 2);
            if p.cap.as_ref().is_some_and(|c| c.bound_by_limit == "reserve") {
                p.no_room = true;
                ("below_minimum", Some(format!("your reserve leaves room for a cap worth ${worth} beside the other deposits listed, under the ${} minimum", fx(minimum, 2))))
            } else {
                ("below_minimum", Some(format!("the cap would be worth ${worth}, under the ${} minimum", fx(minimum, 2))))
            }
        } else {
            let c = p.cap.as_ref().expect("checked above");
            p.remaining_l = c.l;
            p.remaining_nock = c.nock;
            // The cap's USDC by its NOCK, paired on its curve.
            p.remaining_usdc = c.nock / c.per_l.1 * c.per_l.0;
            ("to_deposit", None)
        };
    }

    // The NOCK budget: the cap's NOCK comes first (it is sized by the budget); a remainder
    // of an in-range rung that needs NOCK the rest of the budget cannot give is held back,
    // with what it would need and what the budget allows.
    let need_nock: f64 = plan.iter().filter(|p| p.status == "to_deposit").map(|p| p.remaining_nock).sum();
    let mut nock_left = ctx.budget;
    for p in plan.iter_mut().filter(|p| p.status == "to_deposit" && p.remaining_nock > LADDER_NOCK_DUST) {
        match nock_left {
            None => {
                p.status = "held";
                p.reason = Some(format!(
                    "it would need {} NOCK now, and the floor + tolerance budget (lp_nock_budget) is not reported, so it cannot be checked against it",
                    fx(p.remaining_nock, 4)
                ));
            }
            Some(b) if p.remaining_nock > b + 1e-9 => {
                p.status = "held";
                let all = if (need_nock - p.remaining_nock).abs() > 1e-9 { format!("; the deposits left that hold NOCK would need {} in all", fx(need_nock, 4)) } else { String::new() };
                p.reason = Some(format!(
                    "it would need {} NOCK now{all}, and the floor + tolerance budget has {} left for it",
                    fx(p.remaining_nock, 4),
                    fx(b.max(0.0), 4)
                ));
            }
            Some(b) => nock_left = Some(b - p.remaining_nock),
        }
    }

    // The fit: the deposits left together within what the reserve gate allows. Cut pro
    // rata when they need more; never all or nothing.
    let needed: f64 = plan.iter().filter(|p| p.status == "to_deposit").map(|p| p.remaining_usdc).sum();
    let any_left = plan.iter().any(|p| matches!(p.status, "to_deposit" | "held"));
    let fit_status = if !any_left {
        "nothing_left"
    } else if needed <= headroom_exact + 1e-9 {
        "fits"
    } else if headroom_exact < minimum {
        "no_room"
    } else if needed - headroom_exact < 0.01 {
        // Under a cent over: cut silently below; the amounts typed are cut to cents anyway.
        "fits"
    } else {
        "reduced"
    };
    if needed > headroom_exact + 1e-9 && fit_status != "no_room" {
        let f = headroom_exact / needed;
        for p in plan.iter_mut().filter(|p| p.status == "to_deposit") {
            p.fit_factor = if fit_status == "reduced" { f } else { 1.0 };
            p.remaining_l *= f;
            p.remaining_usdc *= f;
            p.remaining_nock *= f;
            p.remaining_v *= f;
            if p.remaining_usdc + p.remaining_nock * pool.price < minimum {
                p.status = "below_minimum";
                p.no_room = true;
                p.reason = Some(format!(
                    "the reserve gate's cut leaves ${} for it, under the ${} minimum, so it is not offered",
                    fx(p.remaining_usdc + p.remaining_nock * pool.price, 2),
                    fx(minimum, 2)
                ));
            }
        }
    }
    let fit_json = json!({
        "status": fit_status,
        "allowed_usdc": fx(headroom, 2),
        "allowed_rule": headroom_rule,
        "needed_usdc": fx(needed, 2),
        "shortfall_usdc": (fit_status == "reduced" || fit_status == "no_room").then(|| fx(needed - headroom_exact, 2)),
        "factor": (fit_status == "reduced").then(|| fx(headroom_exact / needed, 6)),
        "minimum_usdc": fx(minimum, 2),
        "rule": LADDER_SHAPE_FIT_RULE,
    });

    // What is left to deposit, drawn as rungs; the body's rung index is the setting's.
    let draw_now = fit_status != "no_room";
    let mut rungs: Vec<RungRequest> = Vec::new();
    for p in plan.iter().filter(|p| draw_now && p.status == "to_deposit") {
        let Some((tl, tu)) = p.draw else { continue };
        let range = ctx.range_of_ticks(tl, tu);
        rungs.push(RungRequest {
            index: p.pos + 1,
            rung: Some(p.role.clone()),
            usdc: p.remaining_usdc,
            nock: None,
            low: Bound::Usd(range.low_usd),
            high: Bound::Usd(range.high_usd),
        });
    }
    let status = if !rungs.is_empty() {
        "computed"
    } else if plan.iter().filter(|p| p.role != "cap").all(|p| p.status == "passed") {
        "price_at_or_below_bottom"
    } else if fit_status == "no_room" {
        "no_room"
    } else if plan.iter().any(|p| p.status == "held") {
        "held"
    } else if plan.iter().any(|p| p.no_room) {
        "no_room"
    } else if plan.iter().any(|p| p.status == "below_minimum") && !plan.iter().any(|p| matches!(p.status, "deployed" | "found")) {
        // Nothing is deposited and nothing left is worth listing.
        "no_room"
    } else {
        "deployed"
    };
    let plan_json: Vec<Value> = plan
        .iter()
        .map(|p| {
            json!({
                "index": p.pos + 1,
                "rung": p.role,
                "share": p.share.map(|s| format!("{s}")),
                "status": p.status,
                "reason": p.reason,
                "note": p.note,
                "tick_lower": p.draw.map(|d| d.0),
                "tick_upper": p.draw.map(|d| d.1),
                "target_usdc": fx(p.target_usdc, 6),
                "target_nock": (p.role == "cap").then(|| fx(p.target_nock, 4)),
                "target_liquidity": p.target_l.map(|l| fx(l, 0)),
                "tracked_liquidity": fx(p.tracked_l, 0),
                "deployed_usdc": fx(p.deployed_usdc, 6),
                "deployed_nock": (p.role == "cap").then(|| fx(p.deployed_nock, 4)),
                "remaining_usdc": fx(p.remaining_usdc.max(0.0), 6),
                "remaining_usdc_at_top": (p.role != "cap").then(|| fx(p.remaining_v.max(0.0), 6)),
                "remaining_nock": fx(p.remaining_nock.max(0.0), 4),
                "remaining_liquidity": fx(p.remaining_l.max(0.0), 0),
                "fit_factor": (p.fit_factor < 1.0).then(|| fx(p.fit_factor, 6)),
                "over_by_usdc": (p.status == "over").then(|| fx((v_of(p) - p.target_usdc).max(0.0), 2)),
                "counterparts": p.counterparts,
                "found": p.found,
                "cap": p.cap.as_ref().map(|c| json!({
                    "tick_lower": c.tick_lower,
                    "tick_upper": c.tick_upper,
                    "pool_tick": pool.template.tick,
                    "band_rule": c.band_rule,
                    "lp_nock_budget": fx(c.budget, 4),
                    "nock": fx(c.nock, 4),
                    "usdc": fx(c.usdc, 2),
                    "bound_by": c.bound_by,
                    "usdc_limit": fx(c.limit, 2),
                    "usdc_limit_is": c.bound_by_limit,
                    "limit_rule": "never more USDC than the first near rung's target; once a near or deep rung is deposited, never more than the reserve leaves for it beside the other deposits listed",
                    "note": c.note,
                })),
            })
        })
        .collect();
    let total_usdc = rest + plan.iter().filter(|p| p.role == "cap").map(|p| p.target_usdc).sum::<f64>();
    let shape_json = json!({
        "source": "operator_profile.ladder_rungs",
        "note": "the operator's own shape, sized here; NockNock does not choose it",
        "status": status,
        "total_usdc": fx(total_usdc, 2),
        "total_rule": "the near and deep rungs' USDC at their tops plus the cap's USDC now",
        "gate_price_usd": fx(gate_usd, 8),
        "gate_price_rule": gate_rule,
        "cap": if !has_cap { json!("not in the shape") } else if plan.iter().any(|p| p.role == "cap" && (p.cap.is_some() || !p.counterparts.is_empty() || !p.found.is_empty())) { json!("sized") } else { json!("none") },
        "cap_reason": cap_reason,
        "rest_usdc": fx(rest, 2),
        "rest_rule": rest_rule,
        "rest_note": "the near and deep shares divide this USDC, measured at each rung's top",
        "headroom_usdc": fx(headroom, 2),
        "headroom_rule": headroom_rule,
        "fit": fit_json,
        "tracked_note": LADDER_SHAPE_TRACKED_NOTE,
        "tracked_usdc": fx(counted.iter().filter(|(_, t)| !t.found).map(|(_, t)| t.usdc_now).sum::<f64>(), 2),
        "unmatched": unmatched,
        "dust": { "usdc": minimum, "liquidity_tolerance": LADDER_SHAPE_L_TOLERANCE, "note": LADDER_DISPLAY_NOTE },
        "plan": plan_json,
        "rungs": setting,
    });
    if rungs.is_empty() {
        return Ok(json!({
            "snapshot": ctx.snapshot_block(now, health),
            "inputs": { "shape": shape_json, "position_id": req.position_id },
            "pool": ctx.pool_block(),
            "shape_status": status,
            "statement": LADDER_STATEMENT,
        }));
    }
    let shaped = LadderRequest { rungs, indexed: true, shape: None, ..req.clone() };
    let mut body = simulate_rungs(ctx, &shaped, now, health, Some(deepest))?;
    if let Some(arr) = body["rungs"].as_array_mut() {
        for r in arr.iter_mut() {
            let idx = r["index"].as_u64().unwrap_or(0) as usize;
            let Some(p) = idx.checked_sub(1).and_then(|i| plan.get(i)) else { continue };
            if p.role == "cap" {
                r["bounds"]["high_rule"] = json!("the tick band containing the pool's tick (the cap)");
            } else if p.draw.map(|(tl, tu)| if pool.nock_is_token1 { tl } else { tu }) == Some(spot_tick) {
                r["bounds"]["high_rule"] = json!("the usable tick at or below the pool price (the first rung still drawable)");
            }
            r["cap"] = shape_json["plan"][p.pos]["cap"].clone();
            r["plan"] = json!({
                "status": p.status,
                "target_usdc": fx(p.target_usdc, 6),
                "deployed_usdc": fx(p.deployed_usdc, 6),
                "remaining_usdc": fx(p.remaining_usdc.max(0.0), 6),
                "counterparts": p.counterparts,
            });
        }
    }
    body["inputs"]["shape"] = shape_json;
    body["shape_status"] = json!(status);
    Ok(body)
}

/// What `derived_metrics.ladder_shape` says it is, in every block.
pub(crate) const LADDER_SHAPE_STATEMENT: &str = "The operator's own ladder shape (operator_profile.ladder_rungs), planned from this snapshot against the ladder rungs already tracked and sized to the shared reserve gate's most. A measurement for a card with no inputs: NockNock does not choose the shape, and nothing is signed or sent.";

/// `derived_metrics.ladder_shape` (2.10.0): the configured shape (`shape=configured`)
/// worked out by the cycle in the price-source pool: each configured rung's target, what
/// tracked rungs already hold of it, and what is left to deposit (with the exact mint of
/// the remainder only), so a card renders it with no inputs and no call. None when no
/// shape is configured; `status: unavailable` with the reason when it cannot be drawn.
/// Pure: the stored snapshot only, no clock (`at` is the cycle's time).
pub(crate) fn ladder_shape_block(data: &SnapshotData, at: DateTime<Utc>) -> Option<Value> {
    let gs = &data.derived_metrics.global_status;
    if gs.ladder_rungs.is_none() && gs.ladder_rungs_note.is_none() {
        return None;
    }
    let unavailable = |reason: String| {
        Some(json!({
            "status": "unavailable",
            "reason": reason,
            "source": "operator_profile.ladder_rungs",
            "statement": LADDER_SHAPE_STATEMENT,
        }))
    };
    if let Some(note) = &gs.ladder_rungs_note {
        return unavailable(note.clone());
    }
    let Some(source) = data.positions.iter().find(|p| p.is_price_source) else {
        return unavailable("no live price-source position is in this snapshot: there is no pool to draw the shape in".to_string());
    };
    let env = SnapshotEnvelope {
        meta: SnapshotMeta { generated_at: at, state_hash: String::new(), version: SNAPSHOT_SPEC_VERSION.to_string(), snapshot_id: Uuid::nil() },
        data: data.clone(),
    };
    let params: HashMap<String, String> =
        [("shape", "configured"), ("position_id", source.position_id.as_str())].into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    let body = match LadderRequest::from_params(&params).and_then(|r| simulate_ladder(&env, &r, at, "ok")) {
        Ok(b) => b,
        Err(e) => return unavailable(format!("{}: {}", e.code.unwrap_or(e.error), e.detail)),
    };
    let sh = &body["inputs"]["shape"];
    let tick = data.pool_mechanics.iter().find(|m| m.pool_id == source.pool_id).and_then(|m| m.tick_current);
    let num = |v: &Value| v.as_str().and_then(|x| x.parse::<f64>().ok());
    let inv = |v: &Value| num(v).filter(|x| *x > 0.0).map(|x| fx(1.0 / x, 4));
    let body_rungs = body["rungs"].as_array().cloned().unwrap_or_default();
    let rungs: Vec<Value> = sh["plan"]
        .as_array()
        .cloned()
        .unwrap_or_default()
        .iter()
        .map(|p| {
            // The setting's own share, by position (never looked up by role).
            let idx = p["index"].clone();
            let r = body_rungs.iter().find(|r| r["index"] == idx).cloned().unwrap_or(Value::Null);
            let drawn = !r.is_null();
            let (tl, tu) = (p["tick_lower"].as_i64(), p["tick_upper"].as_i64());
            let in_range = match (tl, tu, tick) {
                (Some(a), Some(b), Some(t)) => json!(a <= t as i64 && (t as i64) < b),
                _ => Value::Null,
            };
            let mint = &r["mint"];
            let f = |v: &Value| if drawn { v.clone() } else { Value::Null };
            json!({
                "index": idx,
                "rung": p["rung"],
                "share": p["share"],
                "status": p["status"],
                "reason": p["reason"],
                "note": p["note"],
                "target_usdc": p["target_usdc"],
                "target_nock": p["target_nock"],
                "target_liquidity": p["target_liquidity"],
                "tracked_liquidity": p["tracked_liquidity"],
                "deployed_usdc": p["deployed_usdc"],
                "deployed_nock": p["deployed_nock"],
                "remaining_usdc": p["remaining_usdc"],
                "remaining_usdc_at_top": p["remaining_usdc_at_top"],
                "remaining_nock": p["remaining_nock"],
                "remaining_liquidity": p["remaining_liquidity"],
                "fit_factor": p["fit_factor"],
                "over_by_usdc": p["over_by_usdc"],
                "counterparts": p["counterparts"],
                "found": p["found"],
                "tick_lower": p["tick_lower"],
                "tick_upper": p["tick_upper"],
                "low_usd": f(&r["bounds"]["low_usd"]),
                "high_usd": f(&r["bounds"]["high_usd"]),
                "low_pct": f(&r["bounds"]["low_pct"]),
                "high_pct": f(&r["bounds"]["high_pct"]),
                "nock_per_usdc_at_low": if drawn { json!(inv(&r["bounds"]["low_usd"])) } else { Value::Null },
                "nock_per_usdc_at_high": if drawn { json!(inv(&r["bounds"]["high_usd"])) } else { Value::Null },
                "high_rule": f(&r["bounds"]["high_rule"]),
                "in_range_now": in_range,
                "out_of_range_above_now": f(&r["top"]["out_of_range_above_now"]),
                "remaining_note": if drawn { json!("the figures below are for what is left to deposit only") } else { Value::Null },
                "liquidity_raw": f(&mint["liquidity_raw"]),
                "liquidity_human": f(&r["sizing"]["liquidity_human"]),
                "usdc": f(&mint["usdc"]),
                "usdc_raw": f(&mint["usdc_raw"]),
                "nock": f(&mint["nock"]),
                "nock_raw": f(&mint["nock_raw"]),
                "amount0_raw": f(&mint["amount0_raw"]),
                "amount1_raw": f(&mint["amount1_raw"]),
                "mint_note": f(&mint["note"]),
                "nock_held_at_bottom": f(&r["bottom"]["nock_held"]),
                "nock_bought_at_bottom": f(&r["bottom"]["nock_bought"]),
                "avg_buy_price_usd": f(&r["bottom"]["avg_buy_price_usd"]),
                "fee_estimate": f(&r["fee_estimate"]),
                "cap": p["cap"],
                "round_trip": f(&r["round_trip"]),
                "if_pulled_at_fill": f(&r["if_pulled_at_fill"]),
            })
        })
        .collect();
    let computed = body.get("rungs").is_some();
    let rc = &body["reserve_check"];
    let fc = &body["floor_check"];
    let c = &body["combined"];
    let g = |v: &Value| if computed { v.clone() } else { Value::Null };
    Some(json!({
        "status": sh["status"],
        "reason": match sh["status"].as_str() {
            Some("deployed") => json!("nothing is left to deposit: every configured rung is deployed by liquidity (or passed, found on-chain, under the minimum or with no cap, as each says)"),
            Some("price_at_or_below_bottom") => json!(LADDER_SHAPE_PASSED_REASON),
            Some("no_room") => json!(format!(
                "the reserve gate allows ${} for what is left, under the ${} minimum: nothing is drawn",
                sh["fit"]["allowed_usdc"].as_str().unwrap_or("?"),
                sh["fit"]["minimum_usdc"].as_str().unwrap_or("?")
            )),
            Some("held") => json!("what is left needs NOCK the floor + tolerance budget does not allow: each held rung says how much"),
            _ => Value::Null,
        },
        "source": "operator_profile.ladder_rungs",
        "statement": LADDER_SHAPE_STATEMENT,
        "computed_at": at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        "pool": {
            "position_id": body["pool"]["position_id"],
            "label": body["pool"]["label"],
            "tick_spacing": body["pool"]["tick_spacing"],
            "pool_price_usd": body["pool"]["pool_price_usd"],
            "pool_tick": tick,
        },
        "total_usdc": sh["total_usdc"],
        "total_rule": sh["total_rule"],
        "rest_usdc": sh["rest_usdc"],
        "rest_rule": sh["rest_rule"],
        "headroom_usdc": sh["headroom_usdc"],
        "headroom_rule": sh["headroom_rule"],
        "fit": sh["fit"],
        "cap": sh["cap"],
        "cap_reason": sh["cap_reason"],
        "tracked_note": sh["tracked_note"],
        "tracked_usdc": sh["tracked_usdc"],
        "unmatched": sh["unmatched"],
        "dust": sh["dust"],
        "rungs": rungs,
        "combined": if computed { json!({
            "remaining_usdc": c["total_usdc"],
            "remaining_nock_needed": c["total_nock_needed"],
            "deepest_bottom_usd": c["deepest_bottom_usd"],
            "nock_held_at_deepest_bottom": c["nock_held_at_deepest_bottom"],
            "nock_bought_at_deepest_bottom": c["nock_bought_at_deepest_bottom"],
            "avg_buy_price_usd": c["avg_buy_price_usd"],
            "nock_kept_if_deep_rungs_pulled": c["nock_kept_if_deep_rungs_pulled"],
            "avg_kept_price_usd": c["avg_kept_price_usd"],
            "overlaps": c["overlaps"],
            "note": "for what is left to deposit only",
        }) } else { Value::Null },
        "gate": {
            "price_usd": sh["gate_price_usd"],
            "rule": sh["gate_price_rule"],
            "cash_reserve_usdc": g(&rc["cash_reserve_usdc"]),
            "stable_at_gate_usdc": g(&rc["stable_at_bottom_usdc"]),
            "margin_usdc": g(&rc["margin_usdc"]),
            "max_usdc_total": sh["headroom_usdc"],
            "status": g(&rc["status"]),
        },
        "nock_budget": {
            "lp_nock_budget": g(&fc["lp_nock_budget"]),
            "nock_needed": g(&fc["nock_needed"]),
            "nock_needed_by_rung": g(&fc["nock_needed_by_rung"]),
            "status": g(&fc["status"]),
        },
        "floor_breach_effect": g(&fc["floor_breach_effect"]),
        "grid": body["grid"].as_array().map(|g| g.iter().map(|row| json!({
            "price_usd": row["price_usd"],
            "pct_of_spot": row["pct_of_spot"],
            "ladder_nock": row["ladder_nock"],
            "ladder_nock_acquired": row["ladder_nock_acquired"],
            "ladder_usdc": row["ladder_usdc"],
            "rungs_in_range": row["rungs_in_range"],
            "markers": row["markers"],
        })).collect::<Vec<_>>()),
    }))
}

/// The floor-crossing price before and after adding the ladder's rungs (#165's
/// `floor_breach_with`): their NOCK moves out of the wallet into the ranges. Pure.
fn floor_breach_effect(data: &SnapshotData, book: &StoredBook, ladders: &[RawPosition], n0: f64, usdc: f64, rounded_ticks: bool) -> Value {
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
    // NOCK is conserved at spot: what leaves the wallet is the hypothetical ranges' own
    // NOCK on their (possibly rounded) ticks, not the unrounded figure.
    let ladder_nock: f64 = ladders
        .iter()
        .map(|l| {
            let n = if is_nock_token(&l.token1) { l.amount1 } else { l.amount0 };
            if n < LADDER_NOCK_DUST { 0.0 } else { n }
        })
        .sum();
    let after = floor_breach_with(&rebuild(ladder_nock, usdc), &profile, ladders);
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
