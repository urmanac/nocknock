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
//! - It shows only while no tracked ladder holds anything (#167; an emptied one is not a
//!   deployed rung), no position found on-chain is waiting for a tap, the reserve allows
//!   at least `LADDER_SUGGEST_MIN_USDC`, and the snapshot is fresh. Freshness is judged where the
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
pub(crate) const LADDER_AFTER_DEPOSIT_TEXT: &str = "After you deposit there is nothing to copy or configure. NockNock looks for the wallet's new position NFT on-chain at the start of every cycle (CHECK NOW runs one at once), so a deposit is normally found in the first cycle after it lands. A position your wallet minted in this pool with exactly a rung's ticks is tracked as a ladder by itself, with its mint as the baseline. Any other range it finds is shown as found on-chain and is not counted until one tap says how to track it.";
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

/// A price with 4 significant digits, for the plain lines: $0.01274, $0.01002. Pure.
pub(crate) fn price4(v: f64) -> String {
    if !(v.is_finite() && v > 0.0) {
        return fx(v, 2);
    }
    let dp = (3 - v.log10().floor() as i32).max(0) as usize;
    fx(v, dp)
}

/// A percent for the plain lines: whole at 10 or more in size, one decimal below, signed:
/// "-23%", "-1.6%". Pure.
pub(crate) fn pct_plain(v: f64) -> String {
    let s = if v.abs() >= 9.5 { fx(v, 0) } else { fx(v, 1) };
    if s.starts_with('-') || s == "0" || s == "0.0" { format!("{s}%") } else { format!("+{s}%") }
}

/// A dollar amount without trailing ".00": $950, $950.50. Pure.
fn dollars_short(v: f64) -> String {
    let s = fx(v, 2);
    s.strip_suffix(".00").map(|w| w.to_string()).unwrap_or(s)
}

/// The pool by its own identity, never by a tracked position's NFT id: "the USDC/NOCK 3%
/// v4 pool". Pure.
pub(crate) fn pool_name(p: &Position) -> String {
    let fee = {
        let f = fx(p.fee_tier_bps as f64 / 100.0, 2);
        let f = f.trim_end_matches('0').trim_end_matches('.').to_string();
        format!("{f}%")
    };
    let venue = match p.protocol.as_str() {
        "uniswap-v4" => "v4",
        "uniswap-v3" => "v3",
        "aerodrome-cl" => "Aero CL",
        other => other,
    };
    format!("the {}/{} {fee} {venue} pool", p.token0.symbol, p.token1.symbol)
}

/// USD per NOCK and NOCK per USDC at a pool tick, from 1.0001^tick and the decimals,
/// never from a rounded string. Pure.
fn tick_prices(p: &Position, tick: i32) -> (f64, f64) {
    let raw = 1.0001f64.powi(tick); // token1 raw per token0 raw
    let (d0, d1) = (p.token0.decimals, p.token1.decimals);
    let token1_per_token0 = raw * 10f64.powi(d0 - d1);
    if is_nock_token(&p.token1) {
        (1.0 / token1_per_token0, token1_per_token0)
    } else {
        (token1_per_token0, 1.0 / token1_per_token0)
    }
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

pub(crate) const LADDER_LINK_FORMAT_NOTE: &str = "The link format is the app's own and undocumented: check the pool and range in the app against the figures here.";

/// The Uniswap web app's new-position page for this pool and range, or the Aero pool's
/// deposit page (nothing prefilled). No amount is passed in the link: on 2026-11-21 the
/// Uniswap app was observed to drop a `depositState` amount (it rewrote it to empty
/// `exactAmounts` and showed 0 USDC), so the amount is printed beside the link as
/// `amount_to_enter` instead. Pure.
fn deep_link(p: &Position, tick_lower: i32, tick_upper: i32, usdc_amount: &str) -> Value {
    let amount_to_enter = format!("Amount to enter in the app: ${usdc_amount} USDC");
    match new_position_url(p, tick_lower, tick_upper, None) {
        Ok((url, prefilled)) => {
            let all = ["pair", "fee and tick spacing", "range ticks", "amount"];
            let not: Vec<&str> = all.iter().copied().filter(|k| !prefilled.contains(k)).collect();
            json!({
                "url": url,
                "host": reqwest::Url::parse(&url).ok().and_then(|u| u.host_str().map(|h| h.to_string())),
                "prefilled": prefilled,
                "not_prefilled": not,
                "format_note": LADDER_LINK_FORMAT_NOTE,
                "amount_to_enter": amount_to_enter,
                "reason": Value::Null,
            })
        }
        Err(reason) => json!({ "url": Value::Null, "host": Value::Null, "prefilled": [], "not_prefilled": ["amount"], "format_note": Value::Null, "amount_to_enter": amount_to_enter, "reason": reason }),
    }
}

/// How a deposited rung becomes tracked: ONE isolated field and function. Since v0.9.0 it
/// is automatic (`tracked_automatically`): position_detect.rs finds the wallet's new
/// position NFT on-chain and tracks it when its pool and ticks are exactly a rung's. The
/// text states how long that may take as what the worker does, with no promised number,
/// and says so when this cycle's check could not run. `detection` is the snapshot's
/// `derived_metrics.position_detection`. Pure.
pub(crate) fn after_deposit(detection: Option<&Value>) -> Value {
    let status = detection.and_then(|d| d.get("status")).and_then(|v| v.as_str());
    let note = match status {
        Some("unavailable") | Some("not_run") => Some(format!(
            "This cycle's check did not run: {} A deposit made now is found once the check runs again.",
            detection.and_then(|d| d.get("reason")).and_then(|v| v.as_str()).unwrap_or("no reason was reported.")
        )),
        Some("partial") => Some(format!(
            "This cycle's check ran with a gap: {}.",
            detection.and_then(|d| d.get("reason")).and_then(|v| v.as_str()).unwrap_or("no reason was reported")
        )),
        _ => None,
    };
    json!({
        "kind": "tracked_automatically",
        "text": LADDER_AFTER_DEPOSIT_TEXT,
        "detection_status": status,
        "detection_note": note,
    })
}

/// The card's plain headline for a status that shows no ladder. Pure.
pub(crate) fn headline_for(status: &str) -> &'static str {
    match status {
        "ladder_tracked" => "Ladder: tracked. No new one is suggested.",
        "found_untracked" => "Ladder: a position found on-chain is waiting for one tap.",
        "needs_setting" => "Ladder: a setting is needed before one is measured.",
        "invalid" => "Ladder: a setting is invalid, so none is measured.",
        "no_target_pool" => "Ladder: no tracked USDC/NOCK pool can hold one this cycle.",
        "several_pools" => "Ladder: several pools could hold one; NockNock does not choose between them.",
        "no_headroom" => "Ladder: no USDC is free for one at the ladder bottom.",
        "price_at_or_below_bottom" => "Ladder: NOCK is at or below your ladder bottom, so none is measured above it.",
        "bottom_at_or_above_top" => "Ladder: NOCK is within one tick spacing of your bottom, so the range would be empty.",
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
    //    A ladder found on-chain and tracked from the database (v0.9.0) says so. One that
    //    holds nothing (emptied, its NFT still in the wallet) is not a deployed rung: it
    //    is listed and does not stop a new measurement.
    let with_source = |mut t: Value, source: Option<&String>| {
        if let Some(src) = source {
            t["source"] = json!(src);
        }
        t
    };
    let mut tracked: Vec<Value> = data
        .ladders
        .iter()
        .map(|l| with_source(json!({ "position_id": l.position_id, "label": l.label, "rung": l.rung, "state": l.state }), l.source.as_ref()))
        .collect();
    for p in data.positions.iter().filter(|p| p.role.as_deref() == Some(LADDER_ROLE)) {
        if !tracked.iter().any(|t| t["position_id"] == json!(p.position_id)) {
            tracked.push(with_source(
                json!({ "position_id": p.position_id, "label": format!("{} #{}", p.protocol, p.nft_id), "rung": p.rung, "state": Value::Null }),
                p.source.as_ref(),
            ));
        }
    }
    let name_of = |t: &Value| -> Option<String> {
        let label = t["label"].as_str()?;
        Some(if t.get("source").is_some() { format!("{label}, found on-chain") } else { label.to_string() })
    };
    let live: Vec<&Value> = tracked.iter().filter(|t| t["state"] != json!("empty")).collect();
    if !live.is_empty() {
        let names: Vec<String> = live.iter().filter_map(|t| name_of(t)).collect();
        out["tracked"] = json!(tracked);
        set(&mut out, "ladder_tracked", format!("A ladder is tracked ({}): no new one is suggested. Its readings are in the Ladders card.", names.join(", ")));
        return out;
    }
    // Every tracked ladder is empty: say so, and measure.
    let none_tracked = if tracked.is_empty() {
        "No ladder is tracked.".to_string()
    } else {
        let names: Vec<String> = tracked.iter().filter_map(name_of).collect();
        out["tracked"] = json!(tracked);
        format!("No tracked ladder holds anything ({} is empty: emptied, its NFT still in the wallet, so not a deployed rung).", names.join(", "))
    };

    // 1b. A position found on-chain that could be a ladder rung is waiting for a tap
    //     (position_detect.rs). Its USDC already left the wallet and is in no total, so a
    //     new ladder is not measured beside it.
    let waiting: Vec<&Value> = data
        .derived_metrics
        .position_detection
        .as_ref()
        .and_then(|d| d.pointer("/found_not_counted/positions"))
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter(|p| p.get("choices").and_then(|c| c.as_array()).is_some_and(|c| c.iter().any(|x| x.as_str().is_some_and(|x| x.starts_with("ladder:")))))
        .collect();
    if !waiting.is_empty() {
        let names: Vec<String> = waiting
            .iter()
            .map(|p| {
                let worth = p.get("value_usd").and_then(|v| v.as_str()).map(|v| format!(", worth ${v}")).unwrap_or_default();
                format!("{} #{}{worth}", p.get("pool_label").and_then(|v| v.as_str()).unwrap_or("position"), p.get("nft_id").and_then(|v| v.as_u64()).unwrap_or(0))
            })
            .collect();
        out["found_untracked"] = json!(waiting);
        set(
            &mut out,
            "found_untracked",
            format!(
                "A USDC/NOCK position the wallet holds was found on-chain and is not tracked or counted yet ({}). No new ladder is measured until it is tracked or set aside (one tap, under Found on-chain).",
                names.join("; ")
            ),
        );
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
        [] => Err(("no_target_pool", "No tracked USDC/NOCK concentrated pool can hold a ladder this cycle; each pool's reason is listed.".to_string())),
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
        // Candidate bottoms only when the bottom is what is missing: an operator who has
        // set one is never shown other bottoms.
        if bottom.is_none() {
            out["candidates"] = match &target {
                Ok(t) => candidates(env, t, wallet_usdc),
                Err((_, why)) => json!({ "rows": [], "reason": why, "note": "candidate bottoms, a measurement; none is picked" }),
            };
        }
        let why: Vec<String> = missing
            .iter()
            .map(|k| if *k == LADDER_BOTTOM_KEY { format!("{k} (your bottom price; NockNock does not choose one)") } else { format!("{k} (the reserve that sizes it)") })
            .collect();
        set(&mut out, "needs_setting", format!("{none_tracked} A suggestion needs {}.", why.join(" and ")));
        out["headline"] = json!(if bottom.is_none() {
            "Ladder: needs your bottom price (operator_profile.ladder_bottom_usd) before one is measured."
        } else {
            "Ladder: needs your protected reserve (operator_profile.cash_reserve_usdc) before one is measured."
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
    let name = pool_name(target);

    // 5. The price against the bottom: at or below it there is nothing to measure above.
    let probe_usdc = LADDER_SUGGEST_MIN_USDC.min(wallet_usdc);
    let price_now = sim_with(env, &target.position_id, ("low_pct", -50.0), probe_usdc.max(1e-6))
        .ok()
        .and_then(|b| n(&b, "/pool/pool_price_usd"));
    let at_or_below = |o: &mut Value, price: Option<f64>| {
        let p = price.map(|p| format!(" (NOCK ${} in {name})", price4(p))).unwrap_or_default();
        o["status"] = json!("price_at_or_below_bottom");
        o["reason"] = json!(format!("NOCK is at or below your ${bottom} ladder bottom{p}: no ladder is measured above it."));
        o["headline"] = json!(format!("Ladder: NOCK is at or below your ${bottom} ladder bottom, so none is measured above it."));
    };
    if let Some(p) = price_now {
        if p <= bottom {
            at_or_below(&mut out, Some(p));
            return out;
        }
    }

    // 6. The reserve gate.
    if !(probe_usdc > 0.0) {
        set(&mut out, "no_headroom", format!("Wallet USDC is ${}: below the ${} minimum, so no ladder can be sized.", fx(wallet_usdc, 2), fx(LADDER_SUGGEST_MIN_USDC, 2)));
        return out;
    }
    let probe = match sim(env, &target.position_id, bottom, probe_usdc) {
        Ok(b) => b,
        Err(e) => {
            // The simulator's "the bottom must be below the top" 400 and its
            // RANGE_ABOVE_SPOT are both the price at or below the bottom, not a bad setting.
            if e.code == Some("RANGE_ABOVE_SPOT") || (e.status == 400 && e.detail.contains("must be below the top")) {
                at_or_below(&mut out, price_now);
                return out;
            }
            let status = match e.code {
                Some("RANGE_TOO_NARROW") => "bottom_at_or_above_top",
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
    let (Some(reserve), Some(pool_price), Some(snapped_low)) =
        (n(&probe, "/reserve_check/cash_reserve_usdc"), n(&probe, "/pool/pool_price_usd"), n(&probe, "/bounds/low_usd"))
    else {
        set(&mut out, "unavailable", "the reserve ceiling could not be measured for this range".to_string());
        return out;
    };
    // The gate is read at the lower of the snapped bottom and your setting (decisions,
    // 2026-11-20): snapping can move the bottom up, never the reserve's own price. Every
    // existing NOCK range moves by the same ratio from its own price, as in the simulator.
    let gate_usd = snapped_low.min(bottom);
    let book = stored_book(&env.data);
    if !book.problems.is_empty() {
        set(&mut out, "unavailable", book.problems.join("; "));
        return out;
    }
    let existing_at_gate: f64 = book.raw.positions.iter().map(|p| existing_at_ratio(p, gate_usd / pool_price).0).sum();
    let reserve_room = floor_cents_up(wallet_usdc + existing_at_gate - reserve);
    let wallet_room = floor_cents_up(wallet_usdc);
    let (max_usdc, binding) = if wallet_room < reserve_room { (wallet_room, "wallet") } else { (reserve_room, "reserve") };
    if max_usdc < LADDER_SUGGEST_MIN_USDC {
        let why = if binding == "wallet" {
            format!(
                "Wallet USDC is ${}: below the ${} minimum (the reserve alone would leave ${}).",
                fx(wallet_usdc, 2),
                fx(LADDER_SUGGEST_MIN_USDC, 2),
                fx(reserve_room.max(0.0), 2)
            )
        } else {
            format!(
                "At ${} the stable balance would be ${} with no ladder (wallet ${} + ${} in the other NOCK ranges): the ${} reserve leaves ${} for a ladder, below the ${} minimum.",
                price4(gate_usd),
                fx(wallet_usdc + existing_at_gate, 2),
                fx(wallet_usdc, 2),
                fx(existing_at_gate, 2),
                fx(reserve, 2),
                fx(reserve_room.max(0.0), 2),
                fx(LADDER_SUGGEST_MIN_USDC, 2)
            )
        };
        set(&mut out, "no_headroom", why);
        return out;
    }

    // 7. The range at that amount, and the NOCK gate.
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
    let (top_exact, top_npu) = tick_prices(target, top_tick);
    let (bottom_exact, bottom_npu) = tick_prices(target, bottom_tick);
    let top_usd = fx(top_exact, 8);
    let bottom_usd = fx(bottom_exact, 8);
    let top_pct = s(&body, "/bounds/high_pct").unwrap_or("").to_string();
    let bottom_pct = s(&body, "/bounds/low_pct").unwrap_or("").to_string();
    let usdc_txt = fx(max_usdc, 2);
    // A USDC-only range from Pt down to Pb with L = usdc / (√Pt − √Pb) (USD terms).
    let l = max_usdc / (top_exact.sqrt() - bottom_exact.sqrt());
    let nock_held = nock_needed + l * (1.0 / bottom_exact.sqrt() - 1.0 / top_exact.sqrt());
    let avg_exact = (top_exact * bottom_exact).sqrt();
    let avg = Some(fx(avg_exact, 8));
    let below = body.pointer("/reserve_check/existing_ranges_below_bottom").cloned().unwrap_or(Value::Null);

    let app = if target.protocol == "aerodrome-cl" { "the Aero app" } else { "the Uniswap web app" };
    let (usdc_tok, nock_tok) = if nock_is_token1 { (&target.token0, &target.token1) } else { (&target.token1, &target.token0) };
    let usdc_raw = format!("{}", ((max_usdc * 100.0).round() as u128) * 10u128.pow((usdc_tok.decimals.max(2) - 2) as u32));
    let nock_raw = format!("{}", (nock_needed * 10f64.powi(nock_tok.decimals)).floor().max(0.0) as u128);
    let token = |t: &PositionTokenInfo| json!({ "symbol": t.symbol, "address": t.address, "decimals": t.decimals });
    let limit_text = if binding == "wallet" {
        format!("all the wallet's USDC (the ${} reserve would allow ${})", fx(reserve, 2), fx(reserve_room, 2))
    } else {
        format!("the most that leaves the stable balance at your ${} reserve at your ${bottom} bottom", fx(reserve, 2))
    };
    let rung = json!({
        "index": 1,
        "of": 1,
        "rung": Value::Null,
        "rung_note": "one rung for now; near and deep rungs are operator settings that join this card later",
        "pool": {
            "position_id": target.position_id,
            "label": name,
            "new_position": true,
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
            "note": "a NEW position (a new NFT) in this pool; position_id is the tracked position the pool was read from",
        },
        "tick_lower": tl,
        "tick_upper": tu,
        "top": { "tick": top_tick, "price_usd": top_usd, "pct_of_spot": top_pct, "rule": "the usable tick at or below spot: the range holds no NOCK now" },
        "bottom": { "tick": bottom_tick, "price_usd": bottom_usd, "pct_of_spot": bottom_pct, "requested_usd": format!("{bottom}"), "rule": "your ladder_bottom_usd, snapped to the nearest usable tick" },
        "bounds_usd": { "low": bottom_usd, "high": top_usd },
        // The form the Uniswap app shows for USDC/NOCK: NOCK per USDC, from the ticks. The
        // top (dearest NOCK) is the fewest NOCK per USDC.
        "bounds_nock_per_usdc": { "low": fx(top_npu, 4), "high": fx(bottom_npu, 4) },
        "usdc": { "amount": usdc_txt, "raw": usdc_raw, "decimals": usdc_tok.decimals, "address": usdc_tok.address },
        "nock": { "amount": fx(nock_needed, 4), "raw": nock_raw, "decimals": nock_tok.decimals, "address": nock_tok.address },
        "at_bottom": { "nock_held": fx(nock_held, 4), "avg_buy_price_usd": avg },
        "at_top": { "usdc_held": s(&body, "/top/usdc_held"), "out_of_range_above_now": body.pointer("/top/out_of_range_above_now").cloned().unwrap_or(Value::Null) },
        "top_neighbours": body.get("top_neighbours").cloned().unwrap_or(Value::Null),
        "deep_link": deep_link(target, tl, tu, &usdc_txt),
        "would_sign": format!(
            "In {app}, with your own wallet: one new position in {name}, ticks {tl} to {tu}, holding up to ${usdc_txt} USDC and 0 NOCK (the app may first ask for a USDC approval). NockNock builds no transaction and signs nothing."
        ),
    });
    out["rungs"] = json!([rung]);
    out["numbers"] = json!({
        "usdc_max": usdc_txt,
        "binding": binding,
        "nock_now": fx(nock_needed, 4),
        "nock_at_bottom": fx(nock_held, 4),
        "avg_buy_price_usd": avg,
        "spot_usd": s(&body, "/snapshot/spot_usd"),
        "wallet_usdc": fx(wallet_usdc, 2),
        "cash_reserve_usdc": fx(reserve, 2),
        "gate_price_usd": fx(gate_usd, 8),
        "existing_ranges_usdc_at_gate": fx(existing_at_gate, 2),
        "stable_at_gate_usdc": fx(wallet_usdc - max_usdc + existing_at_gate, 2),
        "lp_nock_budget": s(&body, "/floor_check/lp_nock_budget"),
    });
    out["gates"] = json!({
        "ladder_tracked": { "status": "none", "reason": if tracked.is_empty() { "no ladder is tracked (#167)" } else { "every tracked ladder is empty (#167)" } },
        "bottom_setting": { "status": "set", "key": LADDER_BOTTOM_KEY, "value": format!("{bottom}") },
        "reserve": {
            "status": "allows",
            "key": LADDER_RESERVE_KEY,
            "binding": binding,
            "cash_reserve_usdc": fx(reserve, 2),
            "read_at_usd": fx(gate_usd, 8),
            "read_at_rule": "the lower of the snapped bottom and your ladder_bottom_usd",
            "reserve_allows_usdc": fx(reserve_room, 2),
            "wallet_usdc": fx(wallet_room, 2),
            "max_usdc": usdc_txt,
            "minimum_usdc": fx(LADDER_SUGGEST_MIN_USDC, 2),
            "stable_at_gate_usdc": fx(wallet_usdc - max_usdc + existing_at_gate, 2),
            "rule": "the stable balance at your bottom stays at or above the reserve",
        },
        "nock_budget": {
            "status": floor_status,
            "nock_needed": fx(nock_needed, 4),
            "lp_nock_budget": s(&body, "/floor_check/lp_nock_budget"),
            "reason": body.pointer("/floor_check/reason").cloned().unwrap_or(Value::Null),
        },
        "floor_breach_effect": body.pointer("/floor_check/floor_breach_effect/status").cloned().unwrap_or(Value::Null),
    });
    out["stated"] = stated(env, &body, below);
    out["after_deposit"] = after_deposit(data.derived_metrics.position_detection.as_ref());
    out["headline"] = json!(format!(
        "Ladder ready: up to ${usdc_txt} USDC in 1 deposit, {} NOCK.",
        if nock_needed > 0.0 { grouped(nock_needed) } else { "0".to_string() }
    ));
    out["outcomes"] = json!(outcomes(max_usdc, nock_needed, top_exact, bottom_exact, pool_price));
    out["picture"] = json!({
        "spot_usd": s(&body, "/pool/pool_price_usd"),
        "bottom_usd": bottom_usd,
        "bottom_requested_usd": format!("{bottom}"),
        "reserve_usdc": fx(reserve, 2),
        "reserve_label": format!("${} kept for below ${bottom}", dollars_short(reserve)),
        "bars": [{ "index": 1, "low_usd": bottom_usd, "high_usd": top_usd, "usdc": usdc_txt, "label": format!("${usdc_txt}") }],
    });
    let sentence = format!(
        "{none_tracked} A new USDC-only position in {name} from ${} ({}) down to ${} ({}) would hold up to ${usdc_txt} USDC and 0 NOCK now: {limit_text}. Fully converted it would hold about {} NOCK, an average of ${}.",
        price4(top_exact),
        pct_plain((top_exact / pool_price - 1.0) * 100.0),
        price4(bottom_exact),
        pct_plain((bottom_exact / pool_price - 1.0) * 100.0),
        grouped(nock_held),
        price4(avg_exact),
    );
    // With no ladder tracked the sentence is what it was before v0.9.0.
    let lead = if tracked.is_empty() { "No ladder is tracked, and".to_string() } else { format!("{none_tracked} Now") };
    out["what_changed"] = json!(if binding == "wallet" {
        format!("{lead} the wallet's ${usdc_txt} USDC fits in {name} down to your ${bottom} bottom.")
    } else {
        format!("{lead} the standing reserve allows ${usdc_txt} in {name} down to your ${bottom} bottom.")
    });
    out["sentence"] = json!(sentence);
    out["status"] = json!("ready");
    out["shown"] = json!(true);
    out
}

/// Whole cents, rounded down. Pure.
fn floor_cents_up(v: f64) -> f64 {
    ((v * 100.0 + 1e-7).floor() / 100.0) + 0.0
}

/// The lines that are stated, never gates: the plan's unspent budget, the tax reserve and
/// the cash target, plus the below-bottom reading. Pure.
fn stated(env: &SnapshotEnvelope, body: &Value, below: Value) -> Value {
    let dm = &env.data.derived_metrics;
    let mut lines: Vec<String> = Vec::new();
    let plan = body.pointer("/reserve_check/accumulation_plan_unspent/usdc").and_then(|v| v.as_str()).and_then(|v| v.parse::<f64>().ok());
    match plan {
        Some(p) => lines.push(format!("Plan budget not yet spent: ${} (stated, not a gate).", fx(p, 2))),
        None => lines.push("No accumulation plan is configured.".to_string()),
    }
    let tax = body.pointer("/reserve_check/tax_reserve_usd/usd").and_then(|v| v.as_str()).and_then(|v| v.parse::<f64>().ok());
    if let Some(t) = tax {
        lines.push(format!("Tax reserve: ${} (stated, not a gate).", fx(t, 2)));
    }
    let total = dm.global_status.total_portfolio_usd.trim().parse::<f64>().ok();
    let cash_bps = dm
        .portfolio_deviation_matrix
        .allocations
        .iter()
        .find(|a| a.bucket_name.starts_with("Liquid USDC"))
        .map(|a| a.target_weight_bps);
    let cash_target = match (total, cash_bps) {
        (Some(t), Some(b)) if b > 0 => {
            let usd = t * b as f64 / 10000.0;
            lines.push(format!("Cash target: ${} ({}% of the portfolio; stated, not a gate).", fx(usd, 2), fx(b as f64 / 100.0, 0)));
            Some(usd)
        }
        (_, Some(0)) => {
            lines.push("No cash target is set.".to_string());
            None
        }
        _ => None,
    };
    if let Some(price) = below.get("price_usd").and_then(|v| v.as_str()).and_then(|v| v.parse::<f64>().ok()) {
        if let Some(stable) = below.get("stable_usdc").and_then(|v| v.as_str()) {
            lines.push(format!(
                "Below the bottom the other NOCK ranges keep converting: at ${} the stable balance would be ${stable} (a measurement, not the gate).",
                price4(price)
            ));
        }
    }
    json!({
        "note": LADDER_STATED_NOT_GATES,
        "accumulation_plan_unspent": body.pointer("/reserve_check/accumulation_plan_unspent").cloned().unwrap_or(Value::Null),
        "tax_reserve_usd": body.pointer("/reserve_check/tax_reserve_usd").cloned().unwrap_or(Value::Null),
        "cash_target_usd": cash_target.map(|c| fx(c, 2)),
        "existing_ranges_below_bottom": below,
        "lines": lines,
    })
}

/// What the range would do, in plain words: a fall to a grid depth inside the range, a
/// fall to the bottom, and a rise. Exact for a USDC-only range: the average buy price
/// between two prices is their geometric mean. Pure.
fn outcomes(usdc: f64, nock_now: f64, top: f64, bottom: f64, pool_price: f64) -> Vec<Value> {
    let mut out = Vec::new();
    if !(top > bottom && bottom > 0.0 && usdc > 0.0) {
        return out;
    }
    let (st, sb) = (top.sqrt(), bottom.sqrt());
    let l = usdc / (st - sb);
    let buy_to = |p: f64| -> (f64, f64, f64) {
        let sp = p.sqrt();
        let spent = l * (st - sp);
        let bought = l * (1.0 / sp - 1.0 / st);
        (spent, bought, (top * p).sqrt())
    };
    // The grid depth nearest the middle of the range (in log price), strictly inside it.
    let mid = (top * bottom).sqrt().ln();
    let steps = (-LADDER_GRID_DEFAULT_TO_PCT / LADDER_GRID_DEFAULT_STEP_PCT + 1e-9).floor() as usize;
    let best = (1..=steps)
        .map(|k| pool_price * (1.0 - k as f64 * LADDER_GRID_DEFAULT_STEP_PCT / 100.0))
        .filter(|p| *p < top * (1.0 - 1e-9) && *p > bottom * (1.0 + 1e-9))
        .min_by(|a, b| (a.ln() - mid).abs().partial_cmp(&(b.ln() - mid).abs()).unwrap_or(std::cmp::Ordering::Equal));
    let pct = |p: f64| pct_plain((p / pool_price - 1.0) * 100.0);
    if let Some(p) = best {
        let (spent, bought, avg) = buy_to(p);
        out.push(json!({
            "kind": "fall_inside",
            "price_usd": fx(p, 8),
            "pct_of_spot": pct(p),
            "nock_bought": fx(bought, 0),
            "usdc_spent": fx(spent, 2),
            "avg_buy_price_usd": fx(avg, 8),
            "text": format!(
                "A fall to ${} ({}) would buy about {} NOCK for ${} of the USDC, an average of ${}.",
                price4(p), pct(p), grouped(bought), fx(spent, 2), price4(avg)
            ),
        }));
    }
    let (_, bought, avg) = buy_to(bottom);
    out.push(json!({
        "kind": "fall_to_bottom",
        "price_usd": fx(bottom, 8),
        "pct_of_spot": pct(bottom),
        "nock_bought": fx(bought, 0),
        "usdc_spent": fx(usdc, 2),
        "avg_buy_price_usd": fx(avg, 8),
        "text": format!(
            "A fall to the ladder bottom ${} ({}) would buy about {} NOCK with all ${}, an average of ${}.",
            price4(bottom), pct(bottom), grouped(bought), fx(usdc, 2), price4(avg)
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
