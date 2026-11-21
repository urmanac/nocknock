//! The ladder suggestion (issue #168): a top-of-page card that states whether a NOCK ladder
//! fits the operator's own rules this cycle and, when it does, what the range would do.
//!
//! It is a measurement, never a directive:
//! - The bottom is the operator's `operator_profile.ladder_bottom_usd`. It has no default;
//!   without it the card says the setting is needed and shows the simulator's candidate
//!   bottoms as a measurement table, with no row picked.
//! - The top is the usable tick at or below spot, so the range holds no NOCK now.
//! - The USDC is the largest amount the standing reserve (#162) allows when the range is
//!   fully converted at its bottom: the simulator's `max_usdc_for_this_range` (#166). The
//!   reserve is the only gate on the USDC; the plan budget and the tax reserve are stated
//!   lines, not gates (operator decision 2026-11-20). The NOCK must fit the floor +
//!   tolerance budget, which a range holding no NOCK always does.
//! - It shows only while no ladder is tracked (#167), the reserve allows at least
//!   `LADDER_SUGGEST_MIN_USDC`, and the snapshot is fresh. Freshness is judged where the
//!   card is read (the dashboard's sync status, the MCP transports' worker health); the
//!   stored object carries `freshness_rule` so every reader applies the same rule.
//!
//! It never changes the directive or `shot_advice_details`, so it never sends a push. It
//! runs on the assembled snapshot through the simulator's own public entry point
//! (`LadderRequest::from_params` + `simulate_ladder`): one computation for the cycle and
//! for `GET /v1/simulate/ladder`. NockNock signs nothing, sends nothing, and the deposit
//! link prefills no amount (OPERATOR_GUIDANCE §7).
//!
//! One rung for now; rungs and a fee estimate join it after `feat/ladder-rungs` lands.
use crate::*;
use serde_json::Value;

/// The event's stable id, for #169's top-of-page events.
pub(crate) const LADDER_SUGGEST_ID: &str = "ladder_suggestion:NOCK";
/// The least USDC the reserve gate must allow for a suggestion: the plan's dust
/// threshold, echoed in every body as `minimum_usdc`.
pub(crate) const LADDER_SUGGEST_MIN_USDC: f64 = PLAN_DUST_USDC;
/// The setting this card needs.
pub(crate) const LADDER_BOTTOM_KEY: &str = "operator_profile.ladder_bottom_usd";
pub(crate) const LADDER_RESERVE_KEY: &str = "operator_profile.cash_reserve_usdc";
pub(crate) const LADDER_BOTTOM_INVALID_NOTE: &str =
    "operator_profile.ladder_bottom_usd must be a USD price per NOCK above 0 (a finite number); no ladder is sized";
pub(crate) const LADDER_SUGGEST_STATEMENT: &str = "A measurement from this cycle's reads of what the range would do. NockNock does not choose the bottom or the amount, builds no transaction, and signs and sends nothing.";
pub(crate) const LADDER_FRESHNESS_RULE: &str = "Shown only while the snapshot is fresh: the dashboard's sync status is not stale, or the worker's health is ok.";
pub(crate) const LADDER_SUGGEST_ASSUMPTIONS: [&str; 5] = [
    "The amount is the most the standing reserve allows at the ladder's bottom; fees and gas are not counted.",
    "The top is the usable tick at or below spot: the range holds only USDC now and earns no fees until NOCK falls to its top.",
    "Below the bottom the other NOCK ranges keep converting; that reading is stated, not the gate.",
    "The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures.",
    "No fee estimate is stated yet; it joins this card with the rung settings.",
];
const LADDER_STATED_NOT_GATES: &str = "The plan budget, the tax reserve and the cash target are stated, not gates: only the standing reserve gates the ladder's USDC.";

/// The bottom setting as the profile states it: None when unset, Err when invalid. Pure.
pub(crate) fn ladder_bottom_setting(profile: &OperatorProfile) -> std::result::Result<Option<f64>, &'static str> {
    match profile.ladder_bottom_usd {
        None => Ok(None),
        Some(v) if v.is_finite() && v > 0.0 => Ok(Some(v)),
        Some(_) => Err(LADDER_BOTTOM_INVALID_NOTE),
    }
}

/// Fixed decimals, rounded, and never "-0.00". Pure.
fn fx(v: f64, dp: usize) -> String {
    let m = 10f64.powi(dp as i32);
    let r = (v * m).round() / m + 0.0;
    format!("{:.*}", dp, r)
}

/// A whole number with thousands separators: 67,904. Pure.
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

fn s<'a>(v: &'a Value, ptr: &str) -> Option<&'a str> {
    v.pointer(ptr).and_then(|x| x.as_str())
}
fn n(v: &Value, ptr: &str) -> Option<f64> {
    s(v, ptr).and_then(|x| x.trim().parse::<f64>().ok())
}

/// A pool the ladder could sit in, or why it cannot.
fn eligibility(p: &Position) -> std::result::Result<(), String> {
    let usdc_nock = (is_usdc_token(&p.token0) && is_nock_token(&p.token1)) || (is_nock_token(&p.token0) && is_usdc_token(&p.token1));
    if !usdc_nock {
        return Err(format!("{}/{} pool; a ladder is a USDC/NOCK range", p.token0.symbol, p.token1.symbol));
    }
    if p.full_range {
        return Err("a basic Aero pool is full range; it cannot hold a ladder".to_string());
    }
    match p.protocol.as_str() {
        "uniswap-v4" => {
            match p.fee_raw {
                None => return Err("the pool's fee was not read this cycle (poolKeys)".to_string()),
                Some(f) if f & 0x800000 != 0 => return Err("a dynamic-fee pool: its fee is not a fixed tier".to_string()),
                Some(_) => {}
            }
            match p.hooks.as_deref() {
                None => return Err("the pool's hooks address was not read this cycle (poolKeys)".to_string()),
                Some(h) if h.trim_start_matches("0x").chars().any(|c| c != '0') => {
                    return Err(format!("a pool with hooks ({h}): a hooked pool's swap math is not verified"))
                }
                Some(_) => {}
            }
        }
        "aerodrome-cl" => {}
        other => return Err(format!("a {other} position: its tick spacing is not read, so a range cannot be snapped")),
    }
    if p.tick_spacing.is_none() {
        return Err("the pool's tick spacing was not read this cycle".to_string());
    }
    match p.pool_active {
        Some(true) => Ok(()),
        Some(false) => Err("this pool has no in-range liquidity, so its price is frozen".to_string()),
        None => Err("the pool's in-range liquidity was not read this cycle".to_string()),
    }
}

/// The simulator on this snapshot with a bottom, the top at spot and an amount. Pure.
fn sim(env: &SnapshotEnvelope, position_id: &str, low: f64, usdc: f64) -> std::result::Result<Value, LadderError> {
    sim_with(env, position_id, ("low", low), usdc)
}

/// `sim` with the bottom given as `low` (USD) or `low_pct` (percent of the pool price).
fn sim_with(env: &SnapshotEnvelope, position_id: &str, low: (&str, f64), usdc: f64) -> std::result::Result<Value, LadderError> {
    let params: HashMap<String, String> = [
        ("usdc", format!("{usdc}")),
        (low.0, format!("{}", low.1)),
        ("high", "spot".to_string()),
        ("position_id", position_id.to_string()),
    ]
    .into_iter()
    .map(|(k, v)| (k.to_string(), v))
    .collect();
    let req = LadderRequest::from_params(&params)?;
    simulate_ladder(env, &req, env.meta.generated_at, "ok")
}

/// The Uniswap web app's new-position page for this pool and range, or the Aero pool's
/// deposit page, with no amount (OPERATOR_GUIDANCE §7.6). Pure.
fn deep_link(p: &Position, tick_lower: i32, tick_upper: i32) -> Value {
    match new_position_url(p, tick_lower, tick_upper) {
        Ok((url, prefilled)) => json!({
            "url": url,
            "host": reqwest::Url::parse(&url).ok().and_then(|u| u.host_str().map(|h| h.to_string())),
            "prefilled": prefilled,
            "not_prefilled": ["amount"],
            "format_note": "The link format is the app's own and undocumented: check the pool, fee and range in the app against the ticks and prices stated here.",
            "reason": Value::Null,
        }),
        Err(reason) => json!({ "url": Value::Null, "host": Value::Null, "prefilled": [], "not_prefilled": ["amount"], "format_note": Value::Null, "reason": reason }),
    }
}

/// The config entry that tracks one minted rung today (#167). Pure.
fn config_snippet(p: &Position, index: usize, tick_lower: i32, tick_upper: i32) -> String {
    let mut lines = vec![
        format!("- id: \"nock-ladder-{index}\""),
        format!("  protocol: \"{}\"", p.protocol),
        format!("  network: \"{}\"", p.network),
        "  nft_id: <the new NFT's id>".to_string(),
    ];
    if p.protocol == "uniswap-v4" {
        lines.push(format!("  pool_id: \"{}\"", p.pool_id));
        lines.push(format!("  currency0: \"{}\"   # {}", p.token0.address, p.token0.symbol));
        lines.push(format!("  currency1: \"{}\"   # {}", p.token1.address, p.token1.symbol));
    } else {
        lines.push(format!("  npm_address: \"{}\"", p.npm_address.clone().unwrap_or_default()));
        lines.push(format!("  pool_address: \"{}\"", p.pool_id));
    }
    lines.push(format!("  tick_lower: {tick_lower}   # the minted ticks; the app may adjust them"));
    lines.push(format!("  tick_upper: {tick_upper}"));
    lines.push("  role: \"ladder\"".to_string());
    lines.push("  rung: \"<near or deep: your choice>\"".to_string());
    lines.push("  deposit_tx: \"0x<the mint transaction hash>\"".to_string());
    lines.join("\n")
}

/// How a minted rung becomes tracked: ONE isolated field and function, so on-chain
/// detection of the wallet's new position NFTs can replace it without touching the rest
/// of the card. Today it is the `manual_config` fallback (#167's `role: ladder` entry),
/// stated as the fallback, not the flow. Pure.
pub(crate) fn after_deposit(rungs: &[(&Position, i32, i32)]) -> Value {
    let snippets: Vec<String> =
        rungs.iter().enumerate().map(|(i, (p, tl, tu))| config_snippet(p, i + 1, *tl, *tu)).collect();
    json!({
        "kind": "manual_config",
        "is": "the current fallback until NockNock detects the wallet's new position NFTs on-chain",
        "text": "NockNock does not yet find a new ladder position by itself. Until it does, a minted rung is tracked by its config.yaml entry (role: ladder, with the minted ticks, the NFT id and the mint transaction as deposit_tx) and a redeploy; the deposit is then verified on-chain from that transaction's receipt (#167).",
        "config_snippets": snippets,
    })
}

/// The card's plain headline for a status that shows no ladder. Pure.
pub(crate) fn headline_for(status: &str) -> &'static str {
    match status {
        "ladder_tracked" => "Ladder: tracked. No new one is suggested.",
        "needs_setting" => "Ladder: a setting is needed to see one.",
        "invalid" => "Ladder: a setting is invalid, so none is measured.",
        "no_target_pool" => "Ladder: no tracked USDC/NOCK pool can hold one this cycle.",
        "several_pools" => "Ladder: several pools could hold one; NockNock does not choose between them.",
        "no_headroom" => "Ladder: no USDC above your protected reserve at the ladder bottom.",
        "bottom_at_or_above_top" => "Ladder: the price is at or near your bottom, so there is nothing to buy below it.",
        "nock_exceeds" => "Ladder: the range would need NOCK the floor budget does not allow.",
        _ => "Ladder: not measured this cycle.",
    }
}

/// The suggestion for this snapshot. `bottom` is the profile's setting
/// (`ladder_bottom_setting`). Pure: the snapshot, the setting, nothing else.
pub(crate) fn ladder_suggestion(env: &SnapshotEnvelope, bottom: std::result::Result<Option<f64>, &'static str>) -> Value {
    let data = &env.data;
    let gs = &data.derived_metrics.global_status;
    let mut out = json!({
        "id": LADDER_SUGGEST_ID,
        "tier": "top",
        "token": "NOCK",
        "status": "unavailable",
        "shown": false,
        "reason": Value::Null,
        "what_changed": Value::Null,
        "sentence": Value::Null,
        "minimum_usdc": fx(LADDER_SUGGEST_MIN_USDC, 2),
        "freshness_rule": LADDER_FRESHNESS_RULE,
        "missing": [],
        "tracked": [],
        "excluded": [],
        "headline": Value::Null,
        "rungs": [],
        "outcomes": [],
        "picture": Value::Null,
        "numbers": Value::Null,
        "gates": Value::Null,
        "stated": Value::Null,
        "after_deposit": Value::Null,
        "candidates": Value::Null,
        "assumptions": LADDER_SUGGEST_ASSUMPTIONS,
        "statement": LADDER_SUGGEST_STATEMENT,
    });
    let set = |o: &mut Value, status: &str, reason: String| {
        o["status"] = json!(status);
        o["reason"] = json!(reason);
        o["headline"] = json!(headline_for(status));
    };

    // 1. A ladder is already tracked (#167): its maintenance is #169's, not a new one.
    let mut tracked: Vec<Value> = data
        .ladders
        .iter()
        .map(|l| json!({ "position_id": l.position_id, "label": l.label, "rung": l.rung, "state": l.state }))
        .collect();
    for p in data.positions.iter().filter(|p| p.role.as_deref() == Some(LADDER_ROLE)) {
        if !tracked.iter().any(|t| t["position_id"] == json!(p.position_id)) {
            tracked.push(json!({ "position_id": p.position_id, "label": format!("{} #{}", p.protocol, p.nft_id), "rung": p.rung, "state": Value::Null }));
        }
    }
    if !tracked.is_empty() {
        let names: Vec<String> = tracked.iter().filter_map(|t| t["label"].as_str().map(|s| s.to_string())).collect();
        out["tracked"] = json!(tracked);
        set(&mut out, "ladder_tracked", format!("A ladder is tracked ({}): no new one is suggested. Its readings are in the Ladders card.", names.join(", ")));
        return out;
    }

    // 2. The bottom setting must be valid when present.
    let bottom = match bottom {
        Err(note) => {
            set(&mut out, "invalid", note.to_string());
            return out;
        }
        Ok(b) => b,
    };

    // 3. The pool: the one tracked USDC/NOCK concentrated pool that can hold a range.
    let mut eligible: Vec<&Position> = Vec::new();
    let mut excluded: Vec<Value> = Vec::new();
    for p in &data.positions {
        if !(is_nock_token(&p.token0) || is_nock_token(&p.token1)) {
            continue; // gas-side ranges (#151) are never a ladder's pool
        }
        match eligibility(p) {
            Ok(()) => {
                if !eligible.iter().any(|e| e.pool_id == p.pool_id) {
                    eligible.push(p)
                }
            }
            Err(reason) => excluded.push(json!({ "position_id": p.position_id, "reason": reason })),
        }
    }
    out["excluded"] = json!(excluded);
    let target: std::result::Result<&Position, (&str, String)> = match eligible.as_slice() {
        [one] => Ok(*one),
        [] => Err(("no_target_pool", "No tracked USDC/NOCK concentrated pool can hold a ladder this cycle (see excluded).".to_string())),
        many => {
            let ids: Vec<&str> = many.iter().map(|p| p.position_id.as_str()).collect();
            Err((
                "several_pools",
                format!("{} tracked USDC/NOCK pools could each hold a ladder ({}); NockNock does not choose between them. The simulator (GET /v1/simulate/ladder) measures each.", many.len(), ids.join(", ")),
            ))
        }
    };

    // 4. The settings, before the pool: a missing setting is the first thing to say.
    let reserve_state = match (&gs.cash_reserve_usdc, &gs.reserve_note) {
        (Some(_), _) => "set",
        (None, Some(_)) => "invalid",
        (None, None) => "not_set",
    };
    if reserve_state == "invalid" {
        set(&mut out, "invalid", RESERVE_INVALID_NOTE.to_string());
        return out;
    }
    let mut missing: Vec<&str> = Vec::new();
    if bottom.is_none() {
        missing.push(LADDER_BOTTOM_KEY);
    }
    if reserve_state == "not_set" {
        missing.push(LADDER_RESERVE_KEY);
    }
    let wallet_usdc: f64 = data
        .wallet_balances
        .iter()
        .find(|w| w.token_symbol == "USDC")
        .and_then(|w| w.amount.parse().ok())
        .unwrap_or(0.0);
    if !missing.is_empty() {
        out["missing"] = json!(missing);
        out["candidates"] = match &target {
            Ok(t) => candidates(env, t, wallet_usdc),
            Err((_, why)) => json!({ "rows": [], "reason": why, "note": "candidate bottoms, a measurement; none is picked" }),
        };
        let why: Vec<String> = missing
            .iter()
            .map(|k| if *k == LADDER_BOTTOM_KEY { format!("{k} (your bottom price; NockNock does not choose one)") } else { format!("{k} (the reserve that sizes it)") })
            .collect();
        set(&mut out, "needs_setting", format!("No ladder is tracked. A suggestion needs {}.", why.join(" and ")));
        out["headline"] = json!(if bottom.is_none() {
            "Ladder: set your bottom price (operator_profile.ladder_bottom_usd) to see one.".to_string()
        } else {
            "Ladder: set your protected reserve (operator_profile.cash_reserve_usdc) to see one.".to_string()
        });
        return out;
    }
    let bottom = bottom.expect("checked above");
    let target = match target {
        Ok(t) => t,
        Err((status, why)) => {
            set(&mut out, status, why);
            return out;
        }
    };

    // 5. The reserve gate, read at the ladder's own bottom.
    let probe_usdc = LADDER_SUGGEST_MIN_USDC.min(wallet_usdc);
    if !(probe_usdc > 0.0) {
        set(&mut out, "no_headroom", format!("Wallet USDC is ${}: below the ${} minimum, so no ladder can be sized.", fx(wallet_usdc, 2), fx(LADDER_SUGGEST_MIN_USDC, 2)));
        return out;
    }
    let probe = match sim(env, &target.position_id, bottom, probe_usdc) {
        Ok(b) => b,
        Err(e) => {
            let status = match e.code {
                Some("RANGE_ABOVE_SPOT") | Some("RANGE_TOO_NARROW") => "bottom_at_or_above_top",
                _ if e.status == 400 => "invalid",
                _ => "unavailable",
            };
            set(&mut out, status, e.detail.clone());
            return out;
        }
    };
    match s(&probe, "/reserve_check/status") {
        Some("invalid") => {
            set(&mut out, "invalid", RESERVE_INVALID_NOTE.to_string());
            return out;
        }
        Some("unknown") => {
            set(&mut out, "unavailable", s(&probe, "/reserve_check/reason").unwrap_or("the reserve could not be measured").to_string());
            return out;
        }
        _ => {}
    }
    let Some(max_usdc) = n(&probe, "/reserve_check/max_usdc_for_this_range") else {
        set(&mut out, "unavailable", "the reserve ceiling could not be measured for this range".to_string());
        return out;
    };
    let reserve = n(&probe, "/reserve_check/cash_reserve_usdc").unwrap_or(0.0);
    let existing_at_bottom = n(&probe, "/reserve_check/existing_ranges_usdc_at_bottom").unwrap_or(0.0);
    if max_usdc < LADDER_SUGGEST_MIN_USDC {
        let stable = wallet_usdc + existing_at_bottom;
        set(
            &mut out,
            "no_headroom",
            format!(
                "At a ${} bottom the stable balance would be ${} with no ladder (wallet ${} + ${} in the other NOCK ranges): the ${} reserve leaves ${} for a ladder, below the ${} minimum.",
                fx(n(&probe, "/bounds/low_usd").unwrap_or(bottom), 8),
                fx(stable, 2),
                fx(wallet_usdc, 2),
                fx(existing_at_bottom, 2),
                fx(reserve, 2),
                fx(max_usdc.max(0.0), 2),
                fx(LADDER_SUGGEST_MIN_USDC, 2)
            ),
        );
        return out;
    }

    // 6. The range at that amount, and the NOCK gate.
    let body = match sim(env, &target.position_id, bottom, max_usdc) {
        Ok(b) => b,
        Err(e) => {
            set(&mut out, "unavailable", e.detail.clone());
            return out;
        }
    };
    let floor_status = s(&body, "/floor_check/status").unwrap_or("unknown");
    let nock_needed = n(&body, "/sizing/nock_needed_at_spot").unwrap_or(0.0);
    if floor_status == "exceeds" {
        set(
            &mut out,
            "nock_exceeds",
            format!(
                "The range would need {} NOCK now; the floor + tolerance budget allows {}.",
                fx(nock_needed, 4),
                s(&body, "/floor_check/lp_nock_budget").unwrap_or("none")
            ),
        );
        return out;
    }
    if floor_status == "unknown" && nock_needed > 0.0 {
        set(&mut out, "unavailable", s(&body, "/floor_check/reason").unwrap_or("the floor + tolerance budget is not reported").to_string());
        return out;
    }
    let (tl, tu) = (body.pointer("/bounds/tick_lower").and_then(|v| v.as_i64()), body.pointer("/bounds/tick_upper").and_then(|v| v.as_i64()));
    let (Some(tl), Some(tu)) = (tl, tu) else {
        set(&mut out, "unavailable", "the range could not be snapped to the pool's ticks".to_string());
        return out;
    };
    let (tl, tu) = (tl as i32, tu as i32);
    let nock_is_token1 = is_nock_token(&target.token1);
    // NOCK as token1: a higher USD price is a lower tick, so the top is tick_lower.
    let (top_tick, bottom_tick) = if nock_is_token1 { (tl, tu) } else { (tu, tl) };
    let label = s(&body, "/pool/label").unwrap_or(&target.position_id).to_string();
    let top_usd = s(&body, "/bounds/high_usd").unwrap_or("").to_string();
    let bottom_usd = s(&body, "/bounds/low_usd").unwrap_or("").to_string();
    let top_pct = s(&body, "/bounds/high_pct").unwrap_or("").to_string();
    let bottom_pct = s(&body, "/bounds/low_pct").unwrap_or("").to_string();
    let usdc_txt = fx(max_usdc, 2);
    let nock_held = n(&body, "/bottom/nock_held").unwrap_or(0.0);
    let avg = s(&body, "/bottom/avg_buy_price_usd").map(|v| v.to_string());
    let below = body.pointer("/reserve_check/existing_ranges_below_bottom").cloned().unwrap_or(Value::Null);

    let app = if target.protocol == "aerodrome-cl" { "the Aero app" } else { "the Uniswap web app" };
    let (usdc_tok, nock_tok) = if nock_is_token1 { (&target.token0, &target.token1) } else { (&target.token1, &target.token0) };
    let usdc_raw = format!("{}", ((max_usdc * 100.0).round() as u128) * 10u128.pow((usdc_tok.decimals.max(2) - 2) as u32));
    let nock_raw = format!("{}", (nock_needed * 10f64.powi(nock_tok.decimals)).floor().max(0.0) as u128);
    let per_usdc = |usd: &str| usd.trim().parse::<f64>().ok().filter(|v| *v > 0.0).map(|v| fx(1.0 / v, 4));
    let token = |t: &PositionTokenInfo| json!({ "symbol": t.symbol, "address": t.address, "decimals": t.decimals });
    let rung = json!({
        "index": 1,
        "of": 1,
        "rung": Value::Null,
        "rung_note": "one rung for now; near and deep rungs are operator settings that join this card later",
        "pool": {
            "position_id": target.position_id,
            "label": label,
            "protocol": target.protocol,
            "network": target.network,
            "pool_id": target.pool_id,
            "npm_address": target.npm_address,
            "token0": token(&target.token0),
            "token1": token(&target.token1),
            "fee_tier_bps": target.fee_tier_bps,
            "fee_raw": target.fee_raw,
            "tick_spacing": target.tick_spacing,
            "hooks": target.hooks,
            "pool_price_usd": s(&body, "/pool/pool_price_usd"),
            "note": "a ladder is a NEW position (a new NFT) in this pool",
        },
        "tick_lower": tl,
        "tick_upper": tu,
        "top": { "tick": top_tick, "price_usd": top_usd, "pct_of_spot": top_pct, "rule": "the usable tick at or below spot: the range holds no NOCK now" },
        "bottom": { "tick": bottom_tick, "price_usd": bottom_usd, "pct_of_spot": bottom_pct, "requested_usd": format!("{bottom}"), "rule": "your ladder_bottom_usd, snapped to the nearest usable tick" },
        "bounds_usd": { "low": bottom_usd, "high": top_usd },
        // The form the Uniswap app shows for USDC/NOCK: NOCK per USDC. The top (dearest
        // NOCK) is the fewest NOCK per USDC.
        "bounds_nock_per_usdc": { "low": per_usdc(&top_usd), "high": per_usdc(&bottom_usd) },
        "usdc": { "amount": usdc_txt, "raw": usdc_raw, "decimals": usdc_tok.decimals, "address": usdc_tok.address },
        "nock": { "amount": fx(nock_needed, 4), "raw": nock_raw, "decimals": nock_tok.decimals, "address": nock_tok.address },
        "at_bottom": { "nock_held": fx(nock_held, 4), "avg_buy_price_usd": avg },
        "at_top": { "usdc_held": s(&body, "/top/usdc_held"), "out_of_range_above_now": body.pointer("/top/out_of_range_above_now").cloned().unwrap_or(Value::Null) },
        "top_neighbours": body.get("top_neighbours").cloned().unwrap_or(Value::Null),
        "deep_link": deep_link(target, tl, tu),
        "would_sign": format!(
            "In {app}, with your own wallet: one new position in {label}, ticks {tl} to {tu}, holding up to ${usdc_txt} USDC and 0 NOCK (the app may first ask for a USDC approval). NockNock builds no transaction and signs nothing."
        ),
    });
    out["rungs"] = json!([rung]);
    out["numbers"] = json!({
        "usdc_max": usdc_txt,
        "nock_now": fx(nock_needed, 4),
        "nock_at_bottom": fx(nock_held, 4),
        "avg_buy_price_usd": avg,
        "spot_usd": s(&body, "/snapshot/spot_usd"),
        "wallet_usdc": fx(wallet_usdc, 2),
        "cash_reserve_usdc": fx(reserve, 2),
        "existing_ranges_usdc_at_bottom": fx(existing_at_bottom, 2),
        "stable_at_bottom_usdc": s(&body, "/reserve_check/stable_at_bottom_usdc"),
        "lp_nock_budget": s(&body, "/floor_check/lp_nock_budget"),
    });
    out["gates"] = json!({
        "ladder_tracked": { "status": "none", "reason": "no ladder is tracked (#167)" },
        "bottom_setting": { "status": "set", "key": LADDER_BOTTOM_KEY, "value": format!("{bottom}") },
        "reserve": {
            "status": "allows",
            "key": LADDER_RESERVE_KEY,
            "cash_reserve_usdc": fx(reserve, 2),
            "max_usdc_for_this_range": usdc_txt,
            "minimum_usdc": fx(LADDER_SUGGEST_MIN_USDC, 2),
            "stable_at_bottom_usdc": s(&body, "/reserve_check/stable_at_bottom_usdc"),
            "margin_usdc": s(&body, "/reserve_check/margin_usdc"),
            "rule": "the stable balance at the ladder's bottom stays at or above the reserve",
        },
        "nock_budget": {
            "status": floor_status,
            "nock_needed": fx(nock_needed, 4),
            "lp_nock_budget": s(&body, "/floor_check/lp_nock_budget"),
            "reason": body.pointer("/floor_check/reason").cloned().unwrap_or(Value::Null),
        },
        "floor_breach_effect": body.pointer("/floor_check/floor_breach_effect/status").cloned().unwrap_or(Value::Null),
    });
    out["stated"] = json!({
        "note": LADDER_STATED_NOT_GATES,
        "accumulation_plan_unspent": body.pointer("/reserve_check/accumulation_plan_unspent").cloned().unwrap_or(Value::Null),
        "tax_reserve_usd": body.pointer("/reserve_check/tax_reserve_usd").cloned().unwrap_or(Value::Null),
        "existing_ranges_below_bottom": below,
    });
    out["after_deposit"] = after_deposit(&[(target, tl, tu)]);
    out["headline"] = json!(format!(
        "Ladder ready: ${usdc_txt} of idle USDC in 1 deposit, {} NOCK.",
        if nock_needed > 0.0 { fx(nock_needed, 4) } else { "0".to_string() }
    ));
    out["outcomes"] = json!(outcomes(&body, max_usdc, nock_needed, &bottom_usd, &bottom_pct, nock_held, avg.as_deref()));
    out["picture"] = json!({
        "spot_usd": s(&body, "/pool/pool_price_usd"),
        "bottom_usd": bottom_usd,
        "bottom_requested_usd": format!("{bottom}"),
        "reserve_usdc": fx(reserve, 2),
        "reserve_label": format!("${} kept for below ${bottom}", fx(reserve, 0)),
        "bars": [{ "index": 1, "low_usd": bottom_usd, "high_usd": top_usd, "usdc": usdc_txt, "label": format!("${usdc_txt}") }],
    });
    let avg_txt = avg.as_deref().map(|a| format!(", an average of ${a}")).unwrap_or_default();
    let sentence = format!(
        "No ladder is tracked. A USDC-only range in {label} from ${top_usd} (tick {top_tick}, the usable tick at or below spot, {top_pct}%) down to ${bottom_usd} (tick {bottom_tick}, your ${bottom} bottom snapped, {bottom_pct}%) would hold up to ${usdc_txt} USDC and 0 NOCK now: the most that leaves the stable balance at your ${} reserve when fully converted at the bottom. Fully converted it would hold about {} NOCK{avg_txt}.",
        fx(reserve, 2),
        grouped(nock_held)
    );
    out["what_changed"] = json!(format!(
        "No ladder is tracked, and the standing reserve allows ${usdc_txt} in {label} down to your ${bottom} bottom."
    ));
    out["sentence"] = json!(sentence);
    out["status"] = json!("ready");
    out["shown"] = json!(true);
    out
}

/// What the range would do, in plain words, from the simulator's own readings: a fall to
/// a grid depth inside the range, a fall to the bottom, and a rise. Measurements. Pure.
fn outcomes(body: &Value, usdc: f64, nock_now: f64, bottom_usd: &str, bottom_pct: &str, nock_at_bottom: f64, avg: Option<&str>) -> Vec<Value> {
    let mut out = Vec::new();
    // The grid row nearest the middle of the range (in log price), strictly inside it.
    let (top, bot) = (n(body, "/bounds/high_usd").unwrap_or(0.0), n(body, "/bounds/low_usd").unwrap_or(0.0));
    if top > 0.0 && bot > 0.0 {
        let mid = (top * bot).sqrt();
        let inside = body.get("grid").and_then(|g| g.as_array()).into_iter().flatten().filter(|r| {
            n(r, "/pool_price_usd").map_or(false, |p| p < top * (1.0 - 1e-9) && p > bot * (1.0 + 1e-9))
        });
        let best = inside.min_by(|a, b| {
            let d = |r: &Value| (n(r, "/pool_price_usd").unwrap_or(0.0).ln() - mid.ln()).abs();
            d(a).partial_cmp(&d(b)).unwrap_or(std::cmp::Ordering::Equal)
        });
        if let Some(r) = best {
            let bought = n(r, "/ladder_nock_acquired").unwrap_or(0.0);
            let spent = usdc - n(r, "/ladder_usdc").unwrap_or(usdc);
            if bought > 0.0 && spent > 0.0 {
                out.push(json!({
                    "kind": "fall_inside",
                    "price_usd": s(r, "/pool_price_usd"),
                    "pct_of_spot": s(r, "/pct_of_spot"),
                    "nock_bought": fx(bought, 0),
                    "usdc_spent": fx(spent, 2),
                    "avg_buy_price_usd": fx(spent / bought, 8),
                    "text": format!(
                        "A fall to ${} ({}%) would buy about {} NOCK for ${} of the USDC, an average of ${}.",
                        s(r, "/pool_price_usd").unwrap_or("?"), s(r, "/pct_of_spot").unwrap_or("?"), grouped(bought), fx(spent, 2), fx(spent / bought, 8)
                    ),
                }));
            }
        }
    }
    let bought = nock_at_bottom - nock_now;
    out.push(json!({
        "kind": "fall_to_bottom",
        "price_usd": bottom_usd,
        "pct_of_spot": bottom_pct,
        "nock_bought": fx(bought, 0),
        "usdc_spent": fx(usdc, 2),
        "avg_buy_price_usd": avg,
        "text": format!(
            "A fall to the ladder bottom ${bottom_usd} ({bottom_pct}%) would buy about {} NOCK with all ${}{}.",
            grouped(bought),
            fx(usdc, 2),
            avg.map(|a| format!(", an average of ${a}")).unwrap_or_default()
        ),
    }));
    if nock_now <= 0.0 {
        out.push(json!({
            "kind": "rise",
            "text": "A rise sells nothing: the ladder holds 0 NOCK, so its USDC stays USDC.",
        }));
    }
    out
}

/// The simulator's grid depths as candidate bottoms, a measurement table with no row
/// picked: each row is the bottom snapped, the most the reserve allows there (null with
/// no reserve), the NOCK that amount holds at the bottom, NOCK per $100 and the average
/// buy price. Pure.
fn candidates(env: &SnapshotEnvelope, target: &Position, wallet_usdc: f64) -> Value {
    let probe_usdc = LADDER_SUGGEST_MIN_USDC.min(wallet_usdc);
    let mut rows: Vec<Value> = Vec::new();
    let mut skipped: Vec<Value> = Vec::new();
    let mut ticks_seen: Vec<i64> = Vec::new();
    let steps = (-LADDER_GRID_DEFAULT_TO_PCT / LADDER_GRID_DEFAULT_STEP_PCT + 1e-9).floor() as usize;
    if !(probe_usdc > 0.0) {
        return json!({
            "to_pct": fx(LADDER_GRID_DEFAULT_TO_PCT, 0),
            "step_pct": fx(LADDER_GRID_DEFAULT_STEP_PCT, 0),
            "rows": [],
            "reason": "no wallet USDC to measure with",
            "note": "candidate bottoms, a measurement; none is picked",
        });
    }
    for k in 1..=steps {
        let pct = -(k as f64) * LADDER_GRID_DEFAULT_STEP_PCT;
        match sim_with(env, &target.position_id, ("low_pct", pct), probe_usdc) {
            Ok(b) => {
                let nock_is_token1 = is_nock_token(&target.token1);
                let tick = b.pointer(if nock_is_token1 { "/bounds/tick_upper" } else { "/bounds/tick_lower" }).and_then(|v| v.as_i64());
                if let Some(t) = tick {
                    if ticks_seen.contains(&t) {
                        continue;
                    }
                    ticks_seen.push(t);
                }
                let per_usdc = n(&b, "/bottom/nock_held").unwrap_or(0.0) / probe_usdc;
                let ceiling = n(&b, "/reserve_check/max_usdc_for_this_range");
                rows.push(json!({
                    "grid_pct": fx(pct, 0),
                    "tick": tick,
                    "price_usd": s(&b, "/bounds/low_usd"),
                    "pct_of_spot": s(&b, "/bounds/low_pct"),
                    "usdc_ceiling": ceiling.map(|c| fx(c.max(0.0), 2)),
                    "nock_at_bottom": ceiling.map(|c| fx(per_usdc * c.max(0.0), 0)),
                    "nock_per_100_usdc": fx(per_usdc * 100.0, 0),
                    "avg_buy_price_usd": s(&b, "/bottom/avg_buy_price_usd"),
                    "existing_ranges_usdc_at_bottom": s(&b, "/reserve_check/existing_ranges_usdc_at_bottom"),
                }));
            }
            Err(e) => skipped.push(json!({ "grid_pct": fx(pct, 0), "reason": e.detail })),
        }
    }
    json!({
        "to_pct": fx(LADDER_GRID_DEFAULT_TO_PCT, 0),
        "step_pct": fx(LADDER_GRID_DEFAULT_STEP_PCT, 0),
        "tick_spacing": target.tick_spacing,
        "rows": rows,
        "skipped": skipped,
        "reason": Value::Null,
        "note": "candidate bottoms, a measurement; none is picked. The depth is the simulator's display grid, not an operator number.",
    })
}

/// Fill `derived_metrics.ladder_suggestion` on an assembled snapshot (the cycle and the
/// golden paths). Pure: the snapshot, the profile and the time it is stamped with.
pub(crate) fn attach_ladder_suggestion(data: &mut SnapshotData, profile: &OperatorProfile, now: DateTime<Utc>) {
    let env = SnapshotEnvelope {
        meta: SnapshotMeta { generated_at: now, state_hash: String::new(), version: SNAPSHOT_SPEC_VERSION.to_string(), snapshot_id: Uuid::nil() },
        data: data.clone(),
    };
    data.derived_metrics.ladder_suggestion = Some(ladder_suggestion(&env, ladder_bottom_setting(profile)));
}
