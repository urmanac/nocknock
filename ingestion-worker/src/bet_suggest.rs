//! WETH/NOCK bets (issue #181): `role: bet` and the bet card, `derived_metrics.bet_suggestion`.
//!
//! A bet is a small two-sided WETH/NOCK range in one Aero CL pool around the price, staked
//! for AERO (the app's default). It is a lotto ticket, not a chore: a new bet is suggested
//! only when no bet is in range, and a bet out of range counts as cashed out. The operator's
//! decisions of 2026-11-21 are the rules; every number is the operator's own setting
//! (`operator_profile.nock_bets`), never a default:
//!
//! - Size: `bet_usd`, both sides at the pool's price (WETH at the cycle's ETH/USD, NOCK at
//!   the pool's own NOCK per WETH).
//! - Range: the pool's tick minus and plus `width_ticks` (or `width_pct` as ticks), each
//!   end rounded to the pool's tick spacing, and never so that the price is outside it.
//! - Exact amounts from `mint_math::exact_mint` at the pool's price, printed beside the link
//!   (the Aero app takes no amount in its link).
//! - Gates, in the issue's order: the settings; the gas floor (wallet ETH plus WETH above
//!   `gas_floor_eth` must cover the ETH side, OPERATOR_GUIDANCE §12 amended for bets only);
//!   total NOCK held at or above `treasury_floor_nock` (NOCK moved into a bet is still held);
//!   no bet in range. Short of gas, the card measures how much WETH would have to come out
//!   of the ETH/USDC ranges first; it never says to pull it.
//!
//! A bet (`role: bet`) is counted in every total, the NOCK floor and the floor-crossing price
//! (#165), and left out of NOCK LP: no LP target, no rebuild, no engine label
//! (`contracts::is_nock_lp`). Staked, it has no LP-claimable fee; the AERO it earns is read
//! from its gauge (aero_rewards.rs). Pure: the snapshot, the profile and the cycle's bet
//! detection; nothing here reads the chain, signs or sends.
use crate::*;
use serde_json::Value;

/// The one `role` of a bet. Exact and case-sensitive, like `ladder`.
pub(crate) const BET_ROLE: &str = "bet";
/// The deviation-matrix row of the bets: measured, no target (#181).
pub(crate) const BET_BUCKET_NAME: &str = "NOCK bets (measured)";
/// The card's and its top-of-page event's stable id.
pub(crate) const BET_SUGGEST_ID: &str = "bet_suggestion:WETH/NOCK";

pub(crate) const BET_KEY_POOL: &str = "operator_profile.nock_bets.pool";
pub(crate) const BET_KEY_USD: &str = "operator_profile.nock_bets.bet_usd";
pub(crate) const BET_KEY_WIDTH: &str = "operator_profile.nock_bets.width_ticks (or width_pct)";
pub(crate) const BET_KEY_GAS: &str = "operator_profile.nock_bets.gas_floor_eth";

pub(crate) const BET_STATEMENT: &str = "A measurement from this cycle's reads of what a bet would be. NockNock does not choose its size, width or pool, builds no transaction, and signs and sends nothing.";
pub(crate) const BET_AFTER_DEPOSIT_TEXT: &str = "After you deposit there is nothing to copy or configure. The Aero app stakes the bet in the pool's gauge (its default, rewards=aero), and NockNock reads that gauge for your wallet's stakes at the start of every cycle (CHECK NOW runs one at once). A bet your wallet deposited itself into this pool is tracked by itself; anything else it finds staked there waits for one tap.";
pub(crate) const BET_LINK_FORMAT_NOTE: &str = "The link opens the bet pool's deposit page with the range filled in. The Aero app has no amount in its link: type the amounts shown here. Check the pool and the range in the app against the figures here.";
pub(crate) const BET_ASSUMPTIONS: [&str; 5] = [
    "Both sides are valued at the pool's price: WETH at this cycle's ETH/USD, NOCK at the pool's own NOCK per WETH.",
    "The amounts are exact for the pool's price this cycle; the app recomputes the other side from whichever amount is typed.",
    "Staked in the gauge, a bet earns AERO instead of swap fees: it has no LP-claimable fee figure.",
    "A bet that leaves its range holds one token only and earns nothing more; it counts as cashed out and does not stop a new one.",
    "The outcomes hold ETH/USD constant: a NOCK/USD move is the same NOCK/ETH move.",
];

/// A config entry configured `role: bet`.
pub(crate) fn is_bet_config(c: &PositionConfig) -> bool {
    c.role.as_deref() == Some(BET_ROLE)
}

/// The checks of a `role: bet` config entry, before any read. `Err` is what the entry does
/// wrong, completed by validate_role_config into a configuration error. Pure.
pub(crate) fn validate_bet_role_config(c: &PositionConfig) -> std::result::Result<(), String> {
    if Protocol::parse(&c.protocol) != Some(Protocol::AeroCl) {
        return Err(format!("is role: bet on protocol {}: a bet is an Aero CL position ({AERO_CL_PROTOCOL})", c.protocol));
    }
    if c.rung.is_some() {
        return Err("sets rung, which applies only to role: ladder".to_string());
    }
    if c.deposit_tx.is_some() {
        return Err("sets deposit_tx, which applies only to role: ladder".to_string());
    }
    if c.is_price_source {
        return Err("is role: bet and is_price_source: the price source is a USDC/NOCK pool; flag another position".to_string());
    }
    if let Some(w) = c.rebuild_weight_bps {
        return Err(format!("is role: bet and sets rebuild_weight_bps ({w}): a bet is never a rebuild template, the role already leaves it out; remove the key"));
    }
    if c.rebuild_when_out_of_range {
        return Err("is role: bet and sets rebuild_when_out_of_range: true: a bet is never a rebuild template; remove the key".to_string());
    }
    if c.deposit_url.is_some() {
        return Err("is role: bet and sets deposit_url, which only rebuild suggestions use; remove the key".to_string());
    }
    Ok(())
}

/// Each bet must be a WETH/NOCK range, checked on what was read. Pure.
pub(crate) fn check_bet_pairs(positions: &[RawPosition], bets: &std::collections::HashSet<String>, configs: &[PositionConfig]) -> Result<()> {
    for p in positions.iter().filter(|p| bets.contains(&p.position_id)) {
        let weth_nock = (p.token0.symbol == "WETH" && is_nock_token(&p.token1)) || (is_nock_token(&p.token0) && p.token1.symbol == "WETH");
        if !weth_nock {
            let id = configs.iter().find(|c| position_key(c) == p.position_id).map(|c| c.id.clone()).unwrap_or_else(|| p.position_id.clone());
            return Err(anyhow::anyhow!(
                "Configuration error: position '{id}' is role: bet but is a {}/{} pair. A bet is a WETH/NOCK range. Correct `positions:` in config.yaml",
                p.token0.symbol,
                p.token1.symbol
            ));
        }
    }
    Ok(())
}

/// Stamp `role: bet` on the snapshot's bets. Pure.
pub(crate) fn apply_bet_roles(positions: &mut [Position], bets: &std::collections::HashSet<String>) {
    for p in positions.iter_mut().filter(|p| bets.contains(&p.position_id)) {
        p.role = Some(BET_ROLE.to_string());
    }
}

// ------------------------------------------------------------------
// The settings
// ------------------------------------------------------------------

/// The settings when every one is set and valid.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BetParams {
    pub(crate) pool: String,
    pub(crate) bet_usd: f64,
    /// The reach of each side in ticks (from `width_pct` when that is the key set).
    pub(crate) width_ticks: i32,
    pub(crate) width_pct: Option<f64>,
    pub(crate) gas_floor_eth: f64,
}

/// `operator_profile.nock_bets` read: the keys missing, the ones invalid, and the
/// parameters when there are neither. Pure.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct BetSettings {
    pub(crate) missing: Vec<String>,
    pub(crate) invalid: Vec<String>,
    pub(crate) params: Option<BetParams>,
}

/// Ticks for a percent reach: ln(1 + pct/100) / ln(1.0001), rounded. Pure.
pub(crate) fn pct_to_ticks(pct: f64) -> i32 {
    ((1.0 + pct / 100.0).ln() / 1.0001f64.ln()).round() as i32
}

pub(crate) fn bet_settings(cfg: &NockBetsConfig) -> BetSettings {
    let mut out = BetSettings::default();
    let pool = match cfg.pool.as_deref().map(str::trim) {
        None | Some("") => {
            out.missing.push(BET_KEY_POOL.to_string());
            None
        }
        Some(p) if is_address(p) => Some(p.to_string()),
        Some(p) => {
            out.invalid.push(format!("{BET_KEY_POOL} must be a pool address (0x and 40 hex characters); got '{p}'"));
            None
        }
    };
    let bet_usd = match cfg.bet_usd {
        None => {
            out.missing.push(BET_KEY_USD.to_string());
            None
        }
        Some(v) if v.is_finite() && v > 0.0 => Some(v),
        Some(v) => {
            out.invalid.push(format!("{BET_KEY_USD} must be a USD amount above 0; got {v}"));
            None
        }
    };
    let width = match (cfg.width_ticks, cfg.width_pct) {
        (None, None) => {
            out.missing.push(BET_KEY_WIDTH.to_string());
            None
        }
        (Some(_), Some(_)) => {
            out.invalid.push("operator_profile.nock_bets sets both width_ticks and width_pct: set one of them".to_string());
            None
        }
        (Some(t), None) if t > 0 => Some((t, None)),
        (Some(t), None) => {
            out.invalid.push(format!("operator_profile.nock_bets.width_ticks must be above 0; got {t}"));
            None
        }
        (None, Some(p)) if p.is_finite() && p > 0.0 && p < 100.0 => Some((pct_to_ticks(p).max(1), Some(p))),
        (None, Some(p)) => {
            out.invalid.push(format!("operator_profile.nock_bets.width_pct must be above 0 and below 100; got {p}"));
            None
        }
    };
    let gas = match cfg.gas_floor_eth {
        None => {
            out.missing.push(BET_KEY_GAS.to_string());
            None
        }
        Some(v) if v.is_finite() && v >= 0.0 => Some(v),
        Some(v) => {
            out.invalid.push(format!("{BET_KEY_GAS} must be an ETH amount of 0 or more; got {v}"));
            None
        }
    };
    if let (Some(pool), Some(bet_usd), Some((width_ticks, width_pct)), Some(gas_floor_eth)) = (pool, bet_usd, width, gas) {
        out.params = Some(BetParams { pool, bet_usd, width_ticks, width_pct, gas_floor_eth });
    }
    out
}

// ------------------------------------------------------------------
// The range and the amounts (pure)
// ------------------------------------------------------------------

fn floor_to(v: i32, s: i32) -> i32 {
    v.div_euclid(s) * s
}

fn nearest(v: i32, s: i32) -> i32 {
    ((v as f64) / (s as f64)).round() as i32 * s
}

/// The bet's ticks: the pool's tick minus and plus `width`, each rounded to the nearest
/// multiple of `spacing`, then widened by one spacing where rounding would leave the price
/// outside [lower, upper). Pure.
pub(crate) fn bet_range(tick: i32, spacing: i32, width: i32) -> (i32, i32) {
    let band = floor_to(tick, spacing);
    let lower = nearest(tick - width, spacing).min(band);
    let upper = nearest(tick + width, spacing).max(band + spacing);
    (lower, upper)
}

/// A token's decimals and whether it is WETH, from the registry.
fn leg(addr: &str) -> Option<(i32, bool)> {
    token_info(addr).ok().map(|t| (t.decimals, t.symbol == "WETH"))
}

/// What a bet on [tick_lower, tick_upper) is at the pool's price.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BetSize {
    pub(crate) liquidity: mint_math::Big,
    pub(crate) amount0: mint_math::Big,
    pub(crate) amount1: mint_math::Big,
    /// Human units.
    pub(crate) weth: f64,
    pub(crate) nock: f64,
    pub(crate) weth_usd: f64,
    pub(crate) nock_usd: f64,
    /// NOCK at the pool's price, and at each end of the range (ETH/USD held).
    pub(crate) usd_per_nock: f64,
    pub(crate) low_usd: f64,
    pub(crate) high_usd: f64,
    /// What it would hold out of the range: only NOCK below `low_usd`, only WETH above `high_usd`.
    pub(crate) nock_if_below: f64,
    pub(crate) weth_if_above: f64,
}

/// USD per NOCK at a pool tick, with ETH/USD `eth_usd`. Pure.
pub(crate) fn usd_per_nock_at(tick: f64, weth0: bool, d0: i32, d1: i32, eth_usd: f64) -> f64 {
    let raw = 1.0001f64.powf(tick); // token1 raw per token0 raw
    let token1_per_token0 = raw * 10f64.powi(d0 - d1);
    if weth0 {
        eth_usd / token1_per_token0
    } else {
        eth_usd * token1_per_token0
    }
}

/// Size a bet of `bet_usd` on the range at the pool's price, exactly. None when the pool's
/// tokens are not WETH and NOCK or the amounts buy no liquidity. Pure.
pub(crate) fn size_bet(pool: &BetPoolRead, eth_usd: f64, bet_usd: f64, tick_lower: i32, tick_upper: i32) -> Option<BetSize> {
    let ((d0, weth0), (d1, weth1)) = (leg(&pool.token0)?, leg(&pool.token1)?);
    if weth0 == weth1 || !(eth_usd.is_finite() && eth_usd > 0.0) {
        return None;
    }
    let sqrt_x96 = U256::from_hex_be(&pool.sqrt_price_x96).ok()?;
    let s = sqrt_price_x96_to_raw(sqrt_x96).sqrt();
    let (sa, sb) = (tick_sqrt_price(tick_lower), tick_sqrt_price(tick_upper));
    let price = |t: f64| usd_per_nock_at(t, weth0, d0, d1, eth_usd);
    let usd_per_nock = {
        let raw = s * s * 10f64.powi(d0 - d1);
        if weth0 { eth_usd / raw } else { eth_usd * raw }
    };
    let (p0, p1) = if weth0 { (eth_usd, usd_per_nock) } else { (usd_per_nock, eth_usd) };
    let (u0, u1) = concentrated_amounts_at(1.0, s, sa, sb);
    let per_l = u0 / 10f64.powi(d0) * p0 + u1 / 10f64.powi(d1) * p1;
    if !(per_l.is_finite() && per_l > 0.0) {
        return None;
    }
    let l = bet_usd / per_l;
    let big = |v: f64| (0.0..1e38).contains(&v).then(|| mint_math::Big::from_u128(v.round() as u128));
    let (max0, max1) = (big(u0 * l)?, big(u1 * l)?);
    let sqrt_big = mint_math::Big::from_hex(&pool.sqrt_price_x96)?;
    let (liq, a0, a1) = mint_math::exact_mint(pool.tick, &sqrt_big, tick_lower, tick_upper, &max0, &max1)?;
    let human = |raw: &mint_math::Big, d: i32| mint_math::raw_to_decimal(raw, d as u32).parse::<f64>().unwrap_or(0.0);
    let (h0, h1) = (human(&a0, d0), human(&a1, d1));
    let (weth, nock) = if weth0 { (h0, h1) } else { (h1, h0) };
    let lf = liq.to_dec().parse::<f64>().unwrap_or(0.0);
    let all0 = lf * (1.0 / sa - 1.0 / sb) / 10f64.powi(d0);
    let all1 = lf * (sb - sa) / 10f64.powi(d1);
    // A higher tick is more token1 per token0. With WETH as token0 that is cheaper NOCK:
    // at or above the upper tick the range holds only token1 (NOCK).
    let (nock_if_below, weth_if_above) = if weth0 { (all1, all0) } else { (all0, all1) };
    let (low_usd, high_usd) = {
        let (a, b) = (price(tick_lower as f64), price(tick_upper as f64));
        (a.min(b), a.max(b))
    };
    Some(BetSize {
        liquidity: liq,
        amount0: a0,
        amount1: a1,
        weth,
        nock,
        weth_usd: weth * eth_usd,
        nock_usd: nock * usd_per_nock,
        usd_per_nock,
        low_usd,
        high_usd,
        nock_if_below,
        weth_if_above,
    })
}

/// The deposit link: the pool's deposit page with the range (Aero keeps a custom range
/// whose ticks are multiples of the pool's spacing), and `native=true` when the ETH side
/// is paid in native ETH. No amount: the app has no amount key. Pure.
pub(crate) fn bet_link(pool_as_configured: &str, tick_lower: i32, tick_upper: i32, native: bool) -> String {
    let mut url = format!("https://app.aero.xyz/deposit/8453-{}?range=custom&tickLow={tick_lower}&tickHigh={tick_upper}", pool_as_configured.trim());
    if native {
        url.push_str("&native=true");
    }
    url
}

// ------------------------------------------------------------------
// Formatting
// ------------------------------------------------------------------

fn fx(v: f64, dp: usize) -> String {
    let m = 10f64.powi(dp as i32);
    format!("{:.*}", dp, (v * m).round() / m + 0.0)
}

fn grouped(v: f64) -> String {
    let s = fx(v, 0);
    let (sign, digits) = s.strip_prefix('-').map(|d| ("-", d)).unwrap_or(("", s.as_str()));
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    format!("{sign}{out}")
}

fn dollars(v: f64) -> String {
    let s = fx(v, 2);
    s.strip_suffix(".00").map(str::to_string).unwrap_or(s)
}

fn eth4(v: f64) -> String {
    fx(v, 4)
}

fn pct_of(v: f64, base: f64) -> String {
    pct_plain((v / base - 1.0) * 100.0)
}

fn num(s: &str) -> Option<f64> {
    s.trim().parse::<f64>().ok().filter(|v| v.is_finite())
}

/// The headline of every status. Pure.
pub(crate) fn bet_headline(status: &str) -> &'static str {
    match status {
        "needs_setting" => "Bets: a setting is needed before one is measured.",
        "invalid" => "Bets: a setting is invalid, so none is measured.",
        "pool_unavailable" => "Bets: the bet pool could not be read as one this cycle.",
        "found_untracked" => "Bets: a bet found staked in the gauge is waiting for one tap.",
        "gas_short" => "Bets: the ETH above your gas floor does not cover a new bet's ETH side.",
        "gas_split" => "Bets: the ETH above your gas floor covers a new bet's ETH side only as WETH and ETH together.",
        "pool_inactive" => "Bets: the bet pool has no in-range liquidity, so its price is frozen and no new bet is sized.",
        "gauge_unread" => "Bets: the gauge's stakes were not fully read this cycle, so no new bet is suggested.",
        "nock_below_floor" => "Bets: total NOCK held is below your floor, so no bet is suggested.",
        "nock_short" => "Bets: the wallet's NOCK does not cover a new bet's NOCK side.",
        "bet_in_range" => "Bets: a bet is in range. No new one is suggested.",
        "ready" => "Bet: one fits your settings now.",
        _ => "Bets: not measured this cycle.",
    }
}

/// The NOCK the snapshot's wallet holds, and its ETH and WETH.
fn wallet_amount(data: &SnapshotData, symbol: &str) -> Option<f64> {
    data.wallet_balances.iter().find(|w| w.token_symbol == symbol).and_then(|w| num(&w.amount))
}

/// One tracked bet as the card lists it.
fn bet_line(p: &Position, pool: Option<&BetPoolRead>, eth_usd: Option<f64>) -> Value {
    let weth0 = p.token0.symbol == "WETH";
    let (d0, d1) = (p.token0.decimals, p.token1.decimals);
    let range_usd = eth_usd.map(|e| {
        let (a, b) = (usd_per_nock_at(p.lower_tick as f64, weth0, d0, d1, e), usd_per_nock_at(p.upper_tick as f64, weth0, d0, d1, e));
        (a.min(b), a.max(b))
    });
    let (weth, nock) = if weth0 { (p.amount0.clone(), p.amount1.clone()) } else { (p.amount1.clone(), p.amount0.clone()) };
    let status = if p.in_range { "in_range" } else { "out_of_range" };
    let holds = if p.in_range {
        "in range".to_string()
    } else if num(&nock).unwrap_or(0.0) > 0.0 && num(&weth).unwrap_or(0.0) <= 0.0 {
        "out of range, cashed out: it holds only NOCK".to_string()
    } else {
        "out of range, cashed out: it holds only WETH".to_string()
    };
    let aero = match (p.unclaimed_aero.as_deref(), p.unclaimed_aero_usd.as_deref()) {
        (Some(a), Some(u)) => format!("{} AERO earned (${u})", fx(num(a).unwrap_or(0.0), 2)),
        (Some(a), None) => format!("{} AERO earned (unpriced)", fx(num(a).unwrap_or(0.0), 2)),
        _ => p.unclaimed_aero_note.clone().map(|n| format!("AERO not read: {n}")).unwrap_or_else(|| "AERO not read".to_string()),
    };
    let range_text = range_usd.map(|(lo, hi)| format!("${}–${} per NOCK", price4(lo), price4(hi))).unwrap_or_else(|| format!("ticks {}..{}", p.lower_tick, p.upper_tick));
    let in_bet_pool = pool.is_none_or(|b| lc(&p.pool_id) == b.pool);
    json!({
        "position_id": p.position_id,
        "source": p.source,
        "status": status,
        "tick_lower": p.lower_tick,
        "tick_upper": p.upper_tick,
        "low_usd": range_usd.map(|r| fx(r.0, 8)),
        "high_usd": range_usd.map(|r| fx(r.1, 8)),
        "weth": weth,
        "nock": nock,
        "value_usd": p.amount_usd,
        "unclaimed_aero": p.unclaimed_aero,
        "unclaimed_aero_usd": p.unclaimed_aero_usd,
        "aero_note": p.unclaimed_aero_note,
        "fees_note": p.fees_note,
        "staked": p.staked_in_gauge.is_some(),
        "in_bet_pool": in_bet_pool,
        "line": format!("Bet {range_text}: {holds}; ${} now; {aero}.", p.amount_usd),
    })
}

fn lc(s: &str) -> String {
    s.trim().to_lowercase()
}

/// The pool block of the card. Pure.
fn pool_json(p: &BetPoolRead, eth_usd: Option<f64>) -> Value {
    let (d0, weth0) = leg(&p.token0).unwrap_or((18, true));
    let d1 = leg(&p.token1).map(|l| l.0).unwrap_or(16);
    let usd = eth_usd.map(|e| usd_per_nock_at(p.tick as f64, weth0, d0, d1, e));
    let nock_per_weth = {
        let raw = 1.0001f64.powi(p.tick) * 10f64.powi(d0 - d1);
        if weth0 { raw } else { 1.0 / raw }
    };
    json!({
        "address": p.pool,
        "block": p.block,
        "token0": p.token0,
        "token1": p.token1,
        "tick_spacing": p.tick_spacing,
        "fee_pips": p.fee_pips,
        "fee_pct": fx(p.fee_pips as f64 / 10_000.0, 2),
        "gauge": p.gauge,
        "tick": p.tick,
        "sqrt_price_x96": p.sqrt_price_x96,
        "liquidity": p.liquidity.to_string(),
        "nock_per_weth": fx(nock_per_weth, 0),
        "usd_per_nock": usd.map(|u| fx(u, 8)),
        "label": format!("the Aero CL{} WETH/NOCK {}% pool", p.tick_spacing, fx(p.fee_pips as f64 / 10_000.0, 2)),
    })
}

/// The floor-crossing price before and after the hypothetical bet (#165's
/// `floor_breach_with`): its NOCK leaves the wallet and sells as NOCK rises. Pure.
fn floor_effect(data: &SnapshotData, pool: &BetPoolRead, size: &BetSize, tick_lower: i32, tick_upper: i32) -> Value {
    let unknown = |reason: String| json!({ "status": "unknown", "before": Value::Null, "after": Value::Null, "reason": reason, "text": format!("The floor-crossing price with this bet could not be worked out: {reason}.") });
    let Some(stored) = data.derived_metrics.floor_breach.as_ref() else {
        return unknown("this snapshot has no floor-crossing price".to_string());
    };
    let Ok(floor) = stored.floor_nock.trim().parse::<f64>() else {
        return unknown(format!("floor_breach.floor_nock '{}' could not be read", stored.floor_nock));
    };
    let bps = data.derived_metrics.global_status.lp_floor_tolerance_bps.unwrap_or(0);
    let book = stored_book(data);
    if !book.problems.is_empty() {
        return unknown(book.problems.join("; "));
    }
    let profile = OperatorProfile { treasury_floor_nock: floor, lp_floor_tolerance_bps: bps, ..Default::default() };
    let (Ok(t0), Ok(t1)) = (token_info(&pool.token0), token_info(&pool.token1)) else {
        return unknown("the pool's tokens are not in the registry".to_string());
    };
    let Ok(sqrt) = U256::from_hex_be(&pool.sqrt_price_x96) else {
        return unknown("the pool's price could not be read".to_string());
    };
    let weth0 = t0.symbol == "WETH";
    let id = "bet:hypothetical".to_string();
    let bet = RawPosition {
        position_id: id.clone(),
        network: TRACKED_NETWORK_BASE.to_string(),
        protocol: AERO_CL_PROTOCOL.to_string(),
        wallet: String::new(),
        nft_id: 0,
        pool_id: pool.pool.clone(),
        npm_address: None,
        fee_tier_bps: pool.fee_pips / 100,
        token0: t0,
        token1: t1,
        liquidity: size.liquidity.to_u128().unwrap_or(0),
        sqrt_price_x96: sqrt,
        tick: pool.tick,
        tick_lower,
        tick_upper,
        amount0: if weth0 { size.weth } else { size.nock },
        amount1: if weth0 { size.nock } else { size.weth },
        fee0: 0.0,
        fee1: 0.0,
        in_range: true,
        is_price_source: false,
    };
    let rebuild = |nock_out: f64| {
        let mut pool_liquidity = book.raw.pool_liquidity.clone();
        pool_liquidity.insert(id.clone(), pool.liquidity.to_string());
        RawOnChainData {
            wallet_balances: book
                .raw
                .wallet_balances
                .iter()
                .map(|w| {
                    if w.token_symbol != "NOCK" || nock_out == 0.0 {
                        return w.clone();
                    }
                    let amt: f64 = w.amount.parse().unwrap_or(0.0);
                    WalletBalance { amount: format!("{:.10}", amt - nock_out), ..w.clone() }
                })
                .collect(),
            positions: book.raw.positions.clone(),
            spot_price_nock_usd: book.raw.spot_price_nock_usd,
            pool_liquidity,
            ..Default::default()
        }
    };
    let before = floor_breach_with(&rebuild(0.0), &profile, &[]);
    let after = floor_breach_with(&rebuild(size.nock), &profile, std::slice::from_ref(&bet));
    let view = |f: &FloorBreach| json!({ "status": f.status, "price_usd": f.crossing.as_ref().and_then(|c| c.price_usd.clone()), "pct_above_spot": f.crossing.as_ref().and_then(|c| c.pct_above_spot.clone()) });
    let px = |f: &FloorBreach| f.crossing.as_ref().and_then(|c| c.price_usd.as_deref()).and_then(num);
    let pct = |f: &FloorBreach| f.crossing.as_ref().and_then(|c| c.pct_above_spot.clone()).unwrap_or_default();
    let level = grouped(floor);
    let text = if before.status == "no_floor" {
        "No treasury floor is set, so there is no floor-crossing price.".to_string()
    } else if after.status == "unavailable" || before.status == "unavailable" {
        format!("The floor-crossing price with this bet could not be worked out: {}.", after.reason.clone().or(before.reason.clone()).unwrap_or_default())
    } else {
        match (after.status.as_str(), px(&after), px(&before)) {
            ("crosses", Some(a), Some(b)) => format!(
                "With this bet, NOCK could rise to ${} (+{}%) before the ranges would sell the stack below the {level} NOCK floor (${} (+{}%) without it).",
                price4(a), pct(&after), price4(b), pct(&before)
            ),
            ("crosses", Some(a), None) => format!("With this bet, NOCK could rise to ${} (+{}%) before the ranges would sell the stack below the {level} NOCK floor; without it no rise does.", price4(a), pct(&after)),
            ("already_below", _, _) => format!("Total NOCK held is already below the {level} NOCK floor, with or without this bet."),
            _ => format!("No rise of NOCK sells the stack below the {level} NOCK floor, with this bet or without it."),
        }
    };
    json!({ "status": after.status, "before": view(&before), "after": view(&after), "reason": Value::Null, "text": text })
}

/// The bet card for this snapshot. `cfg` is `operator_profile.nock_bets`, `detection` the
/// cycle's bet detection. Pure.
pub(crate) fn bet_suggestion(data: &SnapshotData, profile: &OperatorProfile, cfg: &NockBetsConfig, detection: &BetDetection) -> Value {
    let gs = &data.derived_metrics.global_status;
    let eth_usd = gs.eth_price_usd.as_deref().and_then(num).filter(|v| *v > 0.0);
    let settings = bet_settings(cfg);
    let pool = detection.pool.as_ref();
    let mut out = json!({
        "id": BET_SUGGEST_ID,
        "tier": "quiet",
        "status": "unavailable",
        "shown": false,
        "headline": Value::Null,
        "reason": Value::Null,
        "missing": settings.missing,
        "invalid": settings.invalid,
        "settings": {
            "pool": cfg.pool,
            "bet_usd": cfg.bet_usd.map(|v| fx(v, 2)),
            "width_ticks": cfg.width_ticks,
            "width_pct": cfg.width_pct.map(|v| format!("{v}")),
            "width_ticks_used": settings.params.as_ref().map(|p| p.width_ticks),
            "gas_floor_eth": cfg.gas_floor_eth.map(|v| format!("{v}")),
        },
        "pool": pool.map(|p| pool_json(p, eth_usd)),
        "bets": [],
        "found_not_counted": [],
        "suggestion": Value::Null,
        "gates": [],
        "gas": Value::Null,
        "nock": Value::Null,
        "floor_crossing": Value::Null,
        "detection": {
            "status": detection.status,
            "reason": detection.reason,
            "checked_at_block": detection.block,
            "staked_in_gauge": detection.staked.as_ref().map(|s| s.len()),
            "deferred": detection.deferred,
            "unread": detection.unread,
            "refresh_deferred": detection.refresh_deferred,
            "statement": BET_DETECTION_STATEMENT,
        },
        "after_deposit": BET_AFTER_DEPOSIT_TEXT,
        "assumptions": BET_ASSUMPTIONS,
        "statement": BET_STATEMENT,
    });
    let set = |o: &mut Value, status: &str, reason: Option<String>| {
        o["status"] = json!(status);
        o["reason"] = json!(reason);
        o["headline"] = json!(bet_headline(status));
    };

    // The bets the cycle counts, and what the gauge holds for the wallet that is not counted.
    let bets: Vec<&Position> = data.positions.iter().filter(|p| p.role.as_deref() == Some(BET_ROLE)).collect();
    out["bets"] = Value::Array(bets.iter().map(|p| bet_line(p, pool, eth_usd)).collect());
    let offers: Vec<&FoundBet> = detection.found.iter().filter(|f| !f.dismissed).collect();
    out["found_not_counted"] = Value::Array(
        detection
            .found
            .iter()
            .map(|f| {
                json!({
                    "nft_id": f.row.nft_id,
                    "protocol": AERO_CL_PROTOCOL,
                    "dismissed": f.dismissed,
                    "tick_lower": f.row.tick_lower,
                    "tick_upper": f.row.tick_upper,
                    "liquidity": f.liquidity.map(|l| l.to_string()),
                    "why_not_automatic": f.why_not_automatic,
                    "choices": f.choices,
                    "line": format!(
                        "A position staked by your wallet in the bet gauge{} is not counted: {}.",
                        match (f.row.tick_lower, f.row.tick_upper) {
                            (Some(a), Some(b)) => format!(" (ticks {a}..{b})"),
                            _ => String::new(),
                        },
                        f.why_not_automatic.join("; ")
                    ),
                })
            })
            .collect(),
    );

    // Gate 1: the settings.
    let mut gates: Vec<Value> = Vec::new();
    let Some(params) = settings.params.clone() else {
        let status = if settings.invalid.is_empty() { "needs_setting" } else { "invalid" };
        let mut parts: Vec<String> = Vec::new();
        if !settings.missing.is_empty() {
            parts.push(format!("set {}", settings.missing.join(", ")));
        }
        parts.extend(settings.invalid.iter().cloned());
        gates.push(json!({ "gate": "settings", "status": "fail", "text": format!("Settings: {}.", parts.join("; ")) }));
        out["gates"] = Value::Array(gates);
        set(&mut out, status, Some(format!("No bet is measured until {}.", parts.join("; "))));
        return out;
    };
    gates.push(json!({ "gate": "settings", "status": "pass", "text": "Settings: all set." }));
    let Some(pool) = pool else {
        let why = detection.pool_reason.clone().or(detection.reason.clone()).unwrap_or_else(|| "the bet pool was not read".to_string());
        out["gates"] = Value::Array(gates);
        set(&mut out, "pool_unavailable", Some(format!("The bet pool was not read as one this cycle: {why}.")));
        return out;
    };
    if lc(&params.pool) != pool.pool {
        out["gates"] = Value::Array(gates);
        set(&mut out, "pool_unavailable", Some("the pool read is not the configured pool".to_string()));
        return out;
    }
    if pool.liquidity == 0 {
        out["gates"] = Value::Array(gates);
        set(&mut out, "pool_inactive", Some("The bet pool has no in-range liquidity this cycle: its price does not move with the market, so a bet is not sized on it. Bets already deposited are still found and counted.".to_string()));
        return out;
    }
    let Some(eth_usd) = eth_usd else {
        out["gates"] = Value::Array(gates);
        set(&mut out, "unavailable", Some("ETH/USD was not read this cycle, so a bet cannot be sized.".to_string()));
        return out;
    };

    // The range and its exact amounts.
    let (tl, tu) = bet_range(pool.tick, pool.tick_spacing, params.width_ticks);
    let Some(size) = size_bet(pool, eth_usd, params.bet_usd, tl, tu) else {
        out["gates"] = Value::Array(gates);
        set(&mut out, "unavailable", Some(format!("a ${} bet on ticks {tl}..{tu} could not be sized at the pool's price", dollars(params.bet_usd))));
        return out;
    };

    // Gate 2: the gas floor. Wallet ETH and WETH above it fund the ETH side.
    let (eth, weth) = (wallet_amount(data, "ETH"), wallet_amount(data, "WETH"));
    let eth_ranges_weth: f64 = data
        .positions
        .iter()
        .filter(|p| !is_nock_token(&p.token0) && !is_nock_token(&p.token1))
        .map(|p| {
            let w = |t: &PositionTokenInfo, a: &str| if is_eth_token(t) { num(a).unwrap_or(0.0) } else { 0.0 };
            w(&p.token0, &p.amount0) + w(&p.token1, &p.amount1)
        })
        .sum();
    let eth_ranges = data.positions.iter().filter(|p| !is_nock_token(&p.token0) && !is_nock_token(&p.token1) && (is_eth_token(&p.token0) || is_eth_token(&p.token1))).count();
    let need = size.weth;
    // gas_state: covered | short | split (enough above the floor, but only as WETH and ETH
    // together, and the app deposits one or the other) | unread.
    let (gas_ok, native, gas_text, short_by, gas_state) = match (eth, weth) {
        (Some(e), Some(w)) => {
            let above = e + w - params.gas_floor_eth;
            let head = format!(
                "Gas floor: the wallet holds {} ETH and {} WETH; {} ETH is above your {} ETH gas floor, and the bet's ETH side is {} WETH.",
                eth4(e), eth4(w), eth4(above.max(0.0)), params.gas_floor_eth, eth4(need)
            );
            if above + 1e-12 < need {
                let short = need - above.max(0.0);
                let ranges = if eth_ranges == 0 {
                    "No ETH/USDC range is tracked to take it from.".to_string()
                } else {
                    format!(
                        "The {} ETH/USDC range{} hold{} {} WETH; {} WETH would have to come out of {} first. A measurement, not an instruction.",
                        eth_ranges, if eth_ranges == 1 { "" } else { "s" }, if eth_ranges == 1 { "s" } else { "" }, eth4(eth_ranges_weth), eth4(short), if eth_ranges == 1 { "it" } else { "them" }
                    )
                };
                (false, false, format!("{head} Short by {} ETH. {ranges}", eth4(short)), Some(short), "short")
            } else if w + 1e-12 >= need {
                (true, false, format!("{head} The wallet's WETH covers it."), None, "covered")
            } else if e + 1e-12 >= need {
                (true, true, format!("{head} The wallet's WETH does not cover it and its ETH does, so the link deposits native ETH."), None, "covered")
            } else {
                // Enough above the floor, but split across the two: a measurement, no instruction.
                (
                    false,
                    false,
                    format!(
                        "{head} Together they cover it, but the Aero app deposits WETH or native ETH, not both: the wallet's {} WETH and its {} ETH are each below the {} WETH side.",
                        eth4(w), eth4(e), eth4(need)
                    ),
                    None,
                    "split",
                )
            }
        }
        _ => (false, false, "Gas floor: the wallet's ETH and WETH were not read this cycle.".to_string(), None, "unread"),
    };
    out["gas"] = json!({
        "wallet_eth": eth.map(|v| fx(v, 6)),
        "wallet_weth": weth.map(|v| fx(v, 6)),
        "gas_floor_eth": format!("{}", params.gas_floor_eth),
        "above_floor_eth": eth.zip(weth).map(|(e, w)| fx((e + w - params.gas_floor_eth).max(0.0), 6)),
        "needed_eth": fx(need, 6),
        "short_by_eth": short_by.map(|v| fx(v, 6)),
        "eth_usdc_ranges": eth_ranges,
        "eth_usdc_ranges_weth": fx(eth_ranges_weth, 6),
        "deposit_token": if native { "ETH" } else { "WETH" },
        "state": gas_state,
        "text": gas_text,
    });
    gates.push(json!({ "gate": "gas_floor", "status": if gas_ok { "pass" } else { "fail" }, "text": gas_text }));

    // Gate 3: total NOCK held at or above the floor (the LP NOCK budget is not this gate).
    let total = num(&gs.total_nock_amount);
    let floor = profile.treasury_floor_nock;
    let wallet_nock = wallet_amount(data, "NOCK");
    let (nock_ok, nock_text) = match total {
        None => (false, "NOCK floor: total NOCK held was not read this cycle.".to_string()),
        Some(t) if floor > 0.0 && t < floor => (false, format!("NOCK floor: total NOCK held is {}, below your {} NOCK floor.", grouped(t), grouped(floor))),
        Some(t) if floor > 0.0 => (true, format!("NOCK floor: total NOCK held is {}, at or above your {} NOCK floor; NOCK moved into a bet is still held.", grouped(t), grouped(floor))),
        Some(t) => (true, format!("NOCK floor: total NOCK held is {}; no floor is set.", grouped(t))),
    };
    out["nock"] = json!({
        "total_held": total.map(|v| fx(v, 0)),
        "floor": fx(floor, 0),
        "wallet_nock": wallet_nock.map(|v| fx(v, 4)),
        "needed_nock": fx(size.nock, 4),
        "text": nock_text,
    });
    gates.push(json!({ "gate": "nock_floor", "status": if nock_ok { "pass" } else { "fail" }, "text": nock_text }));
    let nock_covered = wallet_nock.is_some_and(|w| w + 1e-9 >= size.nock);

    // Gate 4: no bet in range. A bet out of range is cashed out and blocks nothing.
    let in_range: Vec<&&Position> = bets.iter().filter(|p| p.in_range).collect();
    let range_text = if in_range.is_empty() {
        if bets.is_empty() { "No bet in range: none is tracked.".to_string() } else { format!("No bet in range: {} tracked, all out of range (cashed out).", bets.len()) }
    } else {
        format!("A bet is in range ({} of {} tracked): it is shown instead of a new one.", in_range.len(), bets.len())
    };
    gates.push(json!({ "gate": "no_bet_in_range", "status": if in_range.is_empty() { "pass" } else { "fail" }, "text": range_text }));
    out["gates"] = Value::Array(gates);

    // The suggestion, stated with the link whatever the gates say, and shown only when they all pass.
    let amount_text = format!(
        "Amounts to enter in the Aero app: {} {} and {} NOCK",
        fx(size.weth, 6),
        if native { "ETH" } else { "WETH" },
        fx(size.nock, 2)
    );
    let url = bet_link(&pool.configured, tl, tu, native);
    let width_words = match params.width_pct {
        Some(p) => format!("±{p}% ({} ticks)", params.width_ticks),
        None => format!("±{} ticks", params.width_ticks),
    };
    out["suggestion"] = json!({
        "tick_lower": tl,
        "tick_upper": tu,
        "width": width_words,
        "low_usd": fx(size.low_usd, 8),
        "high_usd": fx(size.high_usd, 8),
        "pool_usd_per_nock": fx(size.usd_per_nock, 8),
        "low_pct": pct_of(size.low_usd, size.usd_per_nock),
        "high_pct": pct_of(size.high_usd, size.usd_per_nock),
        "liquidity": size.liquidity.to_dec(),
        "amount0_raw": size.amount0.to_dec(),
        "amount1_raw": size.amount1.to_dec(),
        "weth": fx(size.weth, 6),
        "nock": fx(size.nock, 2),
        "weth_usd": fx(size.weth_usd, 2),
        "nock_usd": fx(size.nock_usd, 2),
        "value_usd": fx(size.weth_usd + size.nock_usd, 2),
        "bet_usd": fx(params.bet_usd, 2),
        "deposit_token": if native { "ETH" } else { "WETH" },
        "staked": true,
        "link": {
            "url": url,
            "host": "app.aero.xyz",
            "native": native,
            "prefilled": ["pool", "range ticks"],
            "not_prefilled": ["amounts"],
            "amounts_to_enter": amount_text,
            "format_note": BET_LINK_FORMAT_NOTE,
        },
        "range_text": format!(
            "${}–${} per NOCK ({} to {} of the pool's ${}), ticks {tl}..{tu}, {width_words} rounded to the pool's {}-tick spacing.",
            price4(size.low_usd), price4(size.high_usd), pct_of(size.low_usd, size.usd_per_nock), pct_of(size.high_usd, size.usd_per_nock), price4(size.usd_per_nock), pool.tick_spacing
        ),
        "outcomes": [
            { "kind": "fall", "text": format!("If NOCK falls to ${} ({}) or below, the bet holds only NOCK: about {} NOCK, its WETH spent buying NOCK.", price4(size.low_usd), pct_of(size.low_usd, size.usd_per_nock), grouped(size.nock_if_below)) },
            { "kind": "rise", "text": format!("If NOCK rises to ${} ({}) or above, the bet holds only WETH: about {} WETH, its NOCK sold.", price4(size.high_usd), pct_of(size.high_usd, size.usd_per_nock), fx(size.weth_if_above, 4)) },
            { "kind": "staked", "text": "Staked in the gauge it earns AERO while in range; its swap fees go to the gauge, so it has no LP-claimable fee." },
        ],
    });
    // The floor-crossing price with the hypothetical bet: only while no bet is in range,
    // because a new bet is suggested only then.
    if in_range.is_empty() {
        out["floor_crossing"] = floor_effect(data, pool, &size, tl, tu);
    }

    // The status: the first gate that does not pass, in the issue's order.
    if !offers.is_empty() {
        set(&mut out, "found_untracked", Some(format!("{} position{} staked by your wallet in the bet gauge {} not counted until one tap says to track {}.", offers.len(), if offers.len() == 1 { "" } else { "s" }, if offers.len() == 1 { "is" } else { "are" }, if offers.len() == 1 { "it" } else { "them" })));
        out["tier"] = json!("top");
        return out;
    }
    if !gas_ok {
        set(&mut out, if gas_state == "split" { "gas_split" } else { "gas_short" }, Some(gas_text));
        return out;
    }
    if !nock_ok {
        set(&mut out, "nock_below_floor", Some(nock_text));
        return out;
    }
    if !in_range.is_empty() {
        set(&mut out, "bet_in_range", Some(range_text));
        return out;
    }
    // The gauge (review of #181, finding 1): a new bet only when this cycle read every stake
    // of the wallet in it, so a second bet is never suggested on top of one not yet seen.
    let gauge_complete = detection.status == "ok" && detection.staked.is_some() && detection.deferred == 0 && detection.unread == 0;
    if let Some(g) = out["gates"].as_array_mut() {
        g.push(json!({
            "gate": "gauge_read",
            "status": if gauge_complete { "pass" } else { "fail" },
            "text": if gauge_complete { "Gauge: every stake of your wallet in the bet gauge was read this cycle." } else { "Gauge: your wallet's stakes in the bet gauge were not all read this cycle." },
        }));
    }
    if !gauge_complete {
        let why = detection.reason.clone().unwrap_or_else(|| format!("the check's status is {}", detection.status));
        set(&mut out, "gauge_unread", Some(format!("The bet gauge's stakes for your wallet were not fully read this cycle ({why}); a bet may already be deposited. It is read again next cycle.")));
        return out;
    }
    if !nock_covered {
        set(
            &mut out,
            "nock_short",
            Some(format!("The bet's NOCK side is {} NOCK and the wallet holds {} NOCK.", fx(size.nock, 2), wallet_nock.map(|w| fx(w, 2)).unwrap_or_else(|| "an unread amount of".to_string()))),
        );
        return out;
    }
    set(&mut out, "ready", None);
    out["shown"] = json!(true);
    out["tier"] = json!("top");
    out["headline"] = json!(format!(
        "Bet: ${} of WETH/NOCK from ${} to ${} per NOCK, staked for AERO.",
        dollars(params.bet_usd),
        price4(size.low_usd),
        price4(size.high_usd)
    ));
    out["what_changed"] = json!("No bet is in range, and one fits your settings.");
    out
}

/// Fill `derived_metrics.bet_suggestion` on an assembled snapshot. Absent when
/// `operator_profile.nock_bets` is not set. Pure.
pub(crate) fn attach_bet_suggestion(data: &mut SnapshotData, profile: &OperatorProfile, detection: &BetDetection) {
    data.derived_metrics.bet_suggestion = profile.nock_bets.as_ref().map(|cfg| bet_suggestion(data, profile, cfg, detection));
}
