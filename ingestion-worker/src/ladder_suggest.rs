//! The ladder suggestion (issue #168): a top-of-page card that states whether a NOCK ladder
//! fits the operator's own rules this cycle and, when it does, what the range would do.
//!
//! It is a measurement, never a directive. With no shape configured (one rung):
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
//! Two paths, one card:
//! - No `operator_profile.ladder_rungs`: one rung from spot down to the bottom, as #168
//!   shipped it (its bytes are pinned by testdata/ladder_suggest/golden_ladder_suggest.json).
//! - A configured shape (the operator's "bottle": cap | near | deep): `shape_suggestion`
//!   builds the card from `derived_metrics.ladder_shape` (ladder.rs), one numbered deposit
//!   per rung still to deposit, with the rungs already deployed, passed or over stated
//!   beside them. A tracked ladder does not hide this card: the shape counts it and lists
//!   what is left. Each deposit carries the shape's exact mint (liquidity and raw
//!   amounts), its own link (pool and range, never an amount) and its fee estimate; a
//!   deposit that holds NOCK (the cap) states the NOCK limit and the field that keeps it,
//!   because the app works out the other side from whichever amount is typed. The card
//!   also states what the deposits leave in the wallet. Its bytes are pinned by
//!   testdata/ladder_suggest/golden_ladder_bottle.json (tests_ladder_bottle.rs).
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
pub(crate) const LADDER_SUGGEST_BOTTOM_INVALID_NOTE: &str =
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
        Some(_) => Err(LADDER_SUGGEST_BOTTOM_INVALID_NOTE),
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
        "ladder_deployed" => "Ladder deployed: nothing is left to deposit.",
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

    // 0. A configured shape (operator_profile.ladder_rungs) is its own path: the card
    //    lists what is left of it, tracked rungs included.
    if gs.ladder_rungs.is_some() || gs.ladder_rungs_note.is_some() {
        return shape_suggestion(env, bottom, out);
    }

    // 1. A ladder is already tracked (#167): its maintenance is #169's, not a new one.
    //    A ladder found on-chain and tracked from the database (v0.9.0) says so. One that
    //    holds nothing (emptied, its NFT still in the wallet) is not a deployed rung: it
    //    is listed and does not stop a new measurement.
    let tracked = tracked_ladders(data);
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
    let wallet: Option<String> = data.positions.iter().find(|p| p.is_price_source).or(data.positions.first()).map(|p| p.wallet.to_lowercase());
    let waiting: Vec<&Value> = data
        .derived_metrics
        .position_detection
        .as_ref()
        .and_then(|d| d.pointer("/found_not_counted/positions"))
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter(|p| p.get("choices").and_then(|c| c.as_array()).is_some_and(|c| c.iter().any(|x| x.as_str().is_some_and(|x| x.starts_with("ladder:")))))
        // Only the wallet's own mint: a stranger's position minted or transferred to the
        // wallet (dust is cheap) never holds the card back.
        .filter(|p| p.get("minted_by").and_then(|v| v.as_str()).is_some_and(|by| wallet.as_deref() == Some(by.to_lowercase().as_str())))
        .filter(|p| p.get("received_from").and_then(|v| v.as_str()).map_or(true, |from| from.to_lowercase() == crate::position_detect::ZERO_ADDRESS))
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

/// The tracked ladders (#167), from the readings and from any role-ladder position
/// without one. Pure.
fn tracked_ladders(data: &SnapshotData) -> Vec<Value> {
    // A ladder found on-chain and tracked from the database (v0.9.0) carries its source.
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
    tracked
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

// ------------------------------------------------------------------
// The operator's shape (operator_profile.ladder_rungs): one numbered deposit per rung
// ------------------------------------------------------------------

/// The setting that selects this path.
pub(crate) const LADDER_SHAPE_KEY: &str = "operator_profile.ladder_rungs";
pub(crate) const LADDER_SHAPE_SUGGEST_STATEMENT: &str = "A measurement from this cycle's reads of what your ladder shape would do. NockNock does not choose the shape, the bottom or the amounts, builds no transaction, and signs and sends nothing.";
pub(crate) const LADDER_SHAPE_SUGGEST_ASSUMPTIONS: [&str; 5] = [
    "The amounts are your own shape (operator_profile.ladder_rungs): until a near or deep rung is deposited it is sized to what the standing reserve allows at the gate price; from then on that rung fixes the plan. When the reserve allows less than what is left, every deposit is cut in proportion and the shortfall is stated. Fees and gas are not counted in them.",
    "A cap is in range now and holds NOCK; every other rung holds only USDC now and earns no fees until NOCK falls to its top.",
    "Each rung is measured by its liquidity, which does not change with the price: a deposited rung stays deposited when the price moves, and only what is left is listed. A remainder under $1.00 is not offered.",
    "The ladder's USDC is outside the NOCK LP target and the rebuild: the two are separate figures.",
    "Each fee line is a past measurement on our own tracked range scaled to the rung's liquidity: an estimate, not a forecast.",
];
/// Why a deposit that holds NOCK states which field is typed: the app works out the other
/// side from the price at that moment (critique of the deploy design, "ux" lens).
pub(crate) const LADDER_NOCK_LIMIT_WHY: &str = "When the USDC field is typed instead, the app works out the NOCK side from the price at that moment, and it can come out above the limit; neither the link nor the app applies a NOCK limit of its own.";
/// The same for a cap whose USDC limit binds: the NOCK field would pull more USDC.
pub(crate) const LADDER_USDC_LIMIT_WHY: &str = "When the NOCK field is typed instead, the app works out the USDC side from the price at that moment, and it can come out above the limit; neither the link nor the app applies a USDC limit of its own.";

/// A decimal string cut (never rounded up) to `dp` decimals: "86.7890958577" -> "86.7890".
/// An amount to type must not exceed the exact one. Pure.
pub(crate) fn cut_dp(v: &str, dp: usize) -> String {
    let (whole, frac) = v.split_once('.').unwrap_or((v, ""));
    if dp == 0 {
        return whole.to_string();
    }
    let mut f: String = frac.chars().take(dp).collect();
    while f.len() < dp {
        f.push('0');
    }
    format!("{whole}.{f}")
}

/// "cap", "near rung", "deep rung". Pure.
fn rung_words(role: &str) -> String {
    if role == "cap" { "cap".to_string() } else { format!("{role} rung") }
}

/// The first letter in upper case, for a reason that opens a sentence. Pure.
fn sentence_case(v: &str) -> String {
    let mut c = v.chars();
    match c.next() {
        Some(f) => f.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

/// One deposit as a range in USD terms with its liquidity, for the plain outcomes: what
/// it holds at any price. Pure.
struct Leg {
    top: f64,
    bottom: f64,
    l: f64,
}

impl Leg {
    /// From what the range holds at the pool price `p0`. In range it holds both tokens:
    /// the liquidity is read from the side further from its band edge, where a rounded
    /// price moves it least. Wholly below the price it holds only USDC.
    fn new(top: f64, bottom: f64, p0: f64, usdc: f64, nock: f64) -> Option<Leg> {
        if !(top > bottom && bottom > 0.0) {
            return None;
        }
        let pc = p0.clamp(bottom, top);
        // The two spans as fractions of the band, in sqrt-price terms.
        let usdc_span = (pc.sqrt() - bottom.sqrt()) / (top.sqrt() - bottom.sqrt());
        let l = if nock > 0.0 && usdc_span < 0.5 {
            nock / (1.0 / pc.sqrt() - 1.0 / top.sqrt())
        } else if usdc > 0.0 && pc > bottom {
            usdc / (pc.sqrt() - bottom.sqrt())
        } else if nock > 0.0 && pc < top {
            nock / (1.0 / pc.sqrt() - 1.0 / top.sqrt())
        } else {
            return None;
        };
        (l.is_finite() && l > 0.0).then_some(Leg { top, bottom, l })
    }
    fn usdc_at(&self, p: f64) -> f64 {
        self.l * (p.clamp(self.bottom, self.top).sqrt() - self.bottom.sqrt())
    }
    fn nock_at(&self, p: f64) -> f64 {
        self.l * (1.0 / p.clamp(self.bottom, self.top).sqrt() - 1.0 / self.top.sqrt())
    }
}

/// The fields of a rung's fee estimate the card prints, so a stored row stays small: the
/// figure undiluted and diluted (with the share of the pool's active liquidity the rung
/// would be), its sample size, the in-range share of stored history and the replay bound,
/// or the reason each is not stated. Pure.
fn fee_estimate_summary(fe: &Value) -> Value {
    if !fe.is_object() {
        return json!({ "status": "unavailable", "reason": "no fee estimate is stated for this rung" });
    }
    let pick = |v: &Value, keys: &[&str]| -> Value {
        match v.as_object() {
            Some(o) => Value::Object(keys.iter().filter_map(|k| o.get(*k).map(|x| (k.to_string(), x.clone()))).collect()),
            None => Value::Null,
        }
    };
    json!({
        "status": fe["status"],
        "reason": fe["reason"],
        "per_day_in_range_usd": fe["per_day_in_range_usd"],
        "per_day_in_range_diluted_usd": fe["per_day_in_range_diluted_usd"],
        "dilution": pick(&fe["dilution"], &["share_of_active_pct"]),
        "dilution_reason": fe["dilution_reason"],
        "measured_by": fe["measured_by"],
        "measured_label": fe["measured_label"],
        "measured_at": fe["measured_at"],
        "measured_days": fe["measured_days"],
        "intervals": fe["intervals"],
        "in_range_share_stored_history": pick(&fe["in_range_share_stored_history"], &["share_pct", "seconds", "of_seconds"]),
        "in_range_share_reason": fe["in_range_share_reason"],
        "stored_history_replay": pick(&fe["stored_history_replay"], &["inside_upper_usd", "edge_upper_usd"]),
        "replay_reason": fe["replay_reason"],
        "coverage": pick(&fe["coverage"], &["whole", "note"]),
    })
}

/// Whole cents, cut (never rounded up): what is typed and what is summed. Pure.
fn cents_down(v: f64) -> f64 {
    ((v.max(0.0) * 100.0 + 1e-7).floor() / 100.0) + 0.0
}

/// Every rung of the shape as one stated row: its number, its state against the tracked
/// rungs and one plain line. A deployed rung says what it holds; a passed, over, found,
/// held or absent one says why. Amounts left are cut to cents, like the deposits. Pure.
fn shape_rows(data: &SnapshotData, shape: &Value, target: Option<&Position>) -> Vec<Value> {
    let rungs = shape["rungs"].as_array().cloned().unwrap_or_default();
    let of = rungs.len();
    let fit_reduced = s(shape, "/fit/status") == Some("reduced");
    rungs
        .iter()
        .enumerate()
        .map(|(i, r)| {
            let index = r["index"].as_u64().unwrap_or(i as u64 + 1);
            let role = s(r, "/rung").unwrap_or("?");
            let status = s(r, "/status").unwrap_or("unavailable");
            let label = format!("Deposit {index} of {of} ({})", rung_words(role));
            let ticks = r["tick_lower"].as_i64().zip(r["tick_upper"].as_i64()).map(|(a, b)| (a as i32, b as i32));
            // USD bounds from the ticks (a deployed or passed rung has no drawn bounds).
            let bounds = target.zip(ticks).map(|(p, (tl, tu))| {
                let (a, b) = (tick_prices(p, tl).0, tick_prices(p, tu).0);
                (a.min(b), a.max(b))
            });
            let counterparts: Vec<String> = r["counterparts"].as_array().into_iter().flatten().filter_map(|c| c.as_str().map(|c| c.to_string())).collect();
            let readings: Vec<&LadderReading> = counterparts.iter().filter_map(|id| data.ladders.iter().find(|l| &l.position_id == id)).collect();
            let holds = (!counterparts.is_empty() && readings.len() == counterparts.len()).then(|| {
                let sum = |f: fn(&LadderReading) -> &String| readings.iter().filter_map(|l| f(l).parse::<f64>().ok()).sum::<f64>();
                (sum(|l| &l.usdc_now), sum(|l| &l.nock_now))
            });
            let names: Vec<String> = counterparts.iter().map(|id| data.ladders.iter().find(|l| &l.position_id == id).map(|l| l.label.clone()).unwrap_or_else(|| id.clone())).collect();
            let target_usdc = n(r, "/target_usdc").unwrap_or(0.0);
            let deployed_usdc = n(r, "/deployed_usdc").unwrap_or(0.0);
            let remaining_usdc = n(r, "/remaining_usdc").unwrap_or(0.0);
            let remaining_nock = n(r, "/remaining_nock").unwrap_or(0.0);
            let reason = s(r, "/reason").unwrap_or("no reason given");
            let note = s(r, "/note").map(|t| format!(" {}.", sentence_case(t))).unwrap_or_default();
            let text = match status {
                "to_deposit" => {
                    let amount = if remaining_nock > 0.0 {
                        format!("about {} NOCK + ${} USDC", fx(remaining_nock, 2), fx(cents_down(remaining_usdc), 2))
                    } else {
                        format!("${} USDC", fx(cents_down(remaining_usdc), 2))
                    };
                    let part = if deployed_usdc > 0.0 && role != "cap" { format!(" (${} of its ${} target is already deployed)", fx(deployed_usdc, 2), fx(target_usdc, 2)) } else { String::new() };
                    let cut = if fit_reduced { " after the reserve gate's cut" } else { "" };
                    format!("{label}: left to deposit, {amount}{part}{cut}.")
                }
                "deployed" => match holds {
                    Some((u, k)) => format!("{label}: done. It is deployed in {}, which holds ${} USDC and {} NOCK now.{note}", names.join(" and "), fx(u, 2), grouped(k)),
                    None => format!("{label}: done. Its tracked liquidity is at least its target's.{note}"),
                },
                "found" => format!("{label}: found on-chain, being tracked. {}.", sentence_case(reason)),
                "over" => format!("{label}: over its target; it is stated and changes nothing else. {}.", sentence_case(reason)),
                "passed" => format!("{label}: passed. {}.", sentence_case(reason)),
                "none" => format!("{label}: no cap this cycle. {}.", sentence_case(reason)),
                "below_minimum" => format!("{label}: not offered. {}.", sentence_case(reason)),
                "not_deposited" => format!("{label}: not deposited: {reason}."),
                "withdrawn" => format!("{label}: withdrawn. {}.", sentence_case(reason)),
                "partly_withdrawn" => format!("{label}: partly withdrawn. {}.", sentence_case(reason)),
                "held" => format!("{label}: held back. {}.", sentence_case(reason)),
                other => format!("{label}: {other}. {}.", sentence_case(reason)),
            };
            json!({
                "index": index,
                "of": of,
                "rung": role,
                "status": status,
                "reason": r["reason"],
                "note": r["note"],
                "target_usdc": fx(target_usdc, 2),
                "target_nock": r["target_nock"],
                "target_liquidity": r["target_liquidity"],
                "tracked_liquidity": r["tracked_liquidity"],
                "deployed_usdc": fx(deployed_usdc, 2),
                "deployed_nock": r["deployed_nock"],
                "remaining_usdc": fx(cents_down(remaining_usdc), 2),
                "remaining_nock": r["remaining_nock"],
                "over_by_usdc": r["over_by_usdc"],
                "counterparts": counterparts,
                "found": r["found"],
                "holds_now": holds.map(|(u, k)| json!({ "usdc": fx(u, 2), "nock": fx(k, 4) })),
                "tick_lower": r["tick_lower"],
                "tick_upper": r["tick_upper"],
                "bounds_usd": bounds.map(|(lo, hi)| json!({ "low": fx(lo, 8), "high": fx(hi, 8) })),
                "in_range_now": r["in_range_now"],
                "text": text,
            })
        })
        .collect()
}

/// The positions found on-chain this cycle in the ladder's pool that could be a rung and
/// match none exactly: they are not counted until one tap, and the amounts do not count
/// them. Pure.
fn found_elsewhere(data: &SnapshotData, target: &Position, shape: &Value) -> Vec<Value> {
    let matched: Vec<&str> = shape["rungs"].as_array().into_iter().flatten().flat_map(|r| r["found"].as_array().into_iter().flatten().filter_map(|v| v.as_str())).collect();
    data.derived_metrics
        .position_detection
        .as_ref()
        .and_then(|d| d.pointer("/found_not_counted/positions"))
        .and_then(|v| v.as_array())
        .into_iter()
        .flatten()
        .filter(|p| p.get("pool_id").and_then(|v| v.as_str()).map(|v| v.to_lowercase()) == Some(target.pool_id.to_lowercase()))
        .filter(|p| p.get("choices").and_then(|c| c.as_array()).is_some_and(|c| c.iter().any(|x| x.as_str().is_some_and(|x| x.starts_with("ladder:")))))
        .filter(|p| !p.get("position_id").and_then(|v| v.as_str()).is_some_and(|id| matched.contains(&id)))
        .cloned()
        .collect()
}

/// The suggestion when the operator configured a shape: built from
/// `derived_metrics.ladder_shape`, frozen by the first rung deposited and measured by
/// liquidity (ladder.rs). `out` is the empty body `ladder_suggestion` starts from. Pure.
fn shape_suggestion(env: &SnapshotEnvelope, bottom: std::result::Result<Option<f64>, &'static str>, mut out: Value) -> Value {
    let data = &env.data;
    let gs = &data.derived_metrics.global_status;
    out["assumptions"] = json!(LADDER_SHAPE_SUGGEST_ASSUMPTIONS);
    out["statement"] = json!(LADDER_SHAPE_SUGGEST_STATEMENT);
    out["shape"] = Value::Null;
    out["wallet_after"] = Value::Null;
    out["mint_note"] = Value::Null;
    out["fit"] = Value::Null;
    out["found_note"] = Value::Null;
    // Tracked ladders are listed, never a reason to hide the card: the shape counts them.
    out["tracked"] = json!(tracked_ladders(data));
    let set = |o: &mut Value, status: &str, reason: String| {
        o["status"] = json!(status);
        o["reason"] = json!(reason);
        o["headline"] = json!(headline_for(status));
    };

    // 1. The settings: the shape itself, the bottom and the reserve. None has a default.
    if let Some(note) = &gs.ladder_rungs_note {
        set(&mut out, "invalid", note.clone());
        return out;
    }
    let bottom = match bottom {
        Err(note) => {
            set(&mut out, "invalid", note.to_string());
            return out;
        }
        Ok(b) => b,
    };
    if gs.cash_reserve_usdc.is_none() && gs.reserve_note.is_some() {
        set(&mut out, "invalid", RESERVE_INVALID_NOTE.to_string());
        return out;
    }
    let mut missing: Vec<&str> = Vec::new();
    let mut why: Vec<String> = Vec::new();
    if bottom.is_none() {
        missing.push(LADDER_BOTTOM_KEY);
        why.push(format!("{LADDER_BOTTOM_KEY} (the price the reserve gate is read at; it has no default)"));
    }
    if gs.cash_reserve_usdc.is_none() {
        missing.push(LADDER_RESERVE_KEY);
        why.push(format!("{LADDER_RESERVE_KEY} (the reserve that sizes it)"));
    }
    if !missing.is_empty() {
        out["missing"] = json!(missing);
        set(&mut out, "needs_setting", format!("Your ladder shape ({LADDER_SHAPE_KEY}) is set. Sizing it needs {}.", why.join(" and ")));
        out["headline"] = json!(if missing.len() == 1 && missing[0] == LADDER_RESERVE_KEY {
            "Ladder: needs your protected reserve (operator_profile.cash_reserve_usdc) before one is measured.".to_string()
        } else if missing.len() == 1 {
            "Ladder: needs your ladder bottom (operator_profile.ladder_bottom_usd) before your shape is measured.".to_string()
        } else {
            headline_for("needs_setting").to_string()
        });
        // Without a bottom, the candidate bottoms as a measurement table, none picked.
        if bottom.is_none() {
            if let Some(src) = data.positions.iter().find(|p| p.is_price_source) {
                let wallet_usdc: f64 = data.wallet_balances.iter().find(|w| w.token_symbol == "USDC").and_then(|w| w.amount.parse().ok()).unwrap_or(0.0);
                out["candidates"] = candidates(env, src, wallet_usdc);
            }
        }
        return out;
    }

    // 2. The shape as the cycle worked it out (ladder.rs), or why it could not be drawn.
    let Some(shape) = data.derived_metrics.ladder_shape.clone().or_else(|| ladder_shape_block(data, env.meta.generated_at)) else {
        set(&mut out, "unavailable", format!("Your ladder shape ({LADDER_SHAPE_KEY}) was not worked out this cycle."));
        return out;
    };
    let shape_status = s(&shape, "/status").unwrap_or("unavailable");
    if !matches!(shape_status, "computed" | "deployed" | "price_at_or_below_bottom" | "no_room" | "held") {
        let why = s(&shape, "/reason").unwrap_or("no reason given");
        set(&mut out, "unavailable", format!("Your ladder shape ({LADDER_SHAPE_KEY}) could not be drawn this cycle: {why}"));
        return out;
    }
    let target = s(&shape, "/pool/position_id").and_then(|id| data.positions.iter().find(|p| p.position_id == id));
    let rows = shape_rows(data, &shape, target);
    let of = rows.len();
    let count = |st: &str| rows.iter().filter(|r| r["status"] == st).count();
    let left = count("to_deposit");
    out["shape"] = json!({
        "source": LADDER_SHAPE_KEY,
        "status": shape_status,
        "of": of,
        "left": left,
        "deployed": count("deployed"),
        "passed": count("passed"),
        "over": count("over"),
        "none": count("none"),
        "found": count("found"),
        "below_minimum": count("below_minimum"),
        "held": count("held"),
        "total_usdc": shape["total_usdc"],
        "rest_usdc": shape["rest_usdc"],
        "rest_rule": shape["rest_rule"],
        "tracked_usdc": shape["tracked_usdc"],
        "unmatched": shape["unmatched"],
        "rungs": rows.clone(),
    });
    let tally = || {
        let parts: Vec<String> = [
            ("deployed", "deployed"),
            ("found", "found on-chain, being tracked"),
            ("passed", "passed"),
            ("over", "over target"),
            ("below_minimum", "under the minimum"),
            ("held", "held back"),
            ("none", "with no cap this cycle"),
            ("not_deposited", "not deposited (the reserve leaves no room)"),
            ("partly_withdrawn", "partly withdrawn"),
            ("withdrawn", "withdrawn"),
        ]
        .iter()
        .filter(|(st, _)| count(st) > 0)
        .map(|(st, words)| format!("{} {words}", count(st)))
        .collect();
        parts.join(", ")
    };
    if let Some(t) = target {
        let others = found_elsewhere(data, t, &shape);
        if !others.is_empty() {
            let names: Vec<String> = others
                .iter()
                .map(|p| {
                    // Why it stands for no rung, in detection's own words.
                    let why = p.get("why_not_automatic").and_then(|w| w.as_array()).and_then(|w| w.first()).and_then(|w| w.as_str());
                    match why {
                        Some(why) => format!("NFT #{} on ticks {}..{} ({why})", p["nft_id"], p["tick_lower"], p["tick_upper"]),
                        None => format!("NFT #{} on ticks {}..{}", p["nft_id"], p["tick_lower"], p["tick_upper"]),
                    }
                })
                .collect();
            out["found_note"] = json!(format!(
                "Found on-chain in this pool and not counted yet: {}. It stands for no rung of your shape and is not tracked by itself; one tap under Found on-chain tracks it or sets it aside. The amounts here do not count it.",
                names.join("; ")
            ));
        }
    }

    // 3. Nothing is left to deposit: the ladder is deployed, or the price is below it.
    if shape_status == "deployed" {
        out["status"] = json!("ladder_deployed");
        out["reason"] = json!(format!("Nothing is left to deposit: of the {of} rungs of your shape, {}. Their readings are in the Ladders card.", tally()));
        out["headline"] = json!(if count("deployed") == of {
            format!("Ladder deployed: all {of} deposits are done.")
        } else if count("deployed") + count("not_deposited") == of && count("not_deposited") == 1 && rows.iter().any(|r| r["status"] == "not_deposited" && r["rung"] == "cap") {
            // Near and deep are in; the cap is the last rung, and the reserve has no room.
            format!("Ladder deployed: {} of {of} deposits are done, and the reserve leaves no room for the cap.", count("deployed"))
        } else {
            format!("Ladder: nothing is left to deposit ({}).", tally())
        });
        return out;
    }
    if shape_status == "price_at_or_below_bottom" {
        let passed: Vec<String> = rows.iter().filter(|r| r["status"] == "passed").map(|r| rung_words(s(r, "/rung").unwrap_or("?"))).collect();
        out["status"] = json!("price_at_or_below_bottom");
        out["reason"] = json!(format!(
            "NOCK at ${} is at or below the bottom of your ladder shape: every rung is passed ({}), so no USDC-only deposit is possible in any of them.",
            s(&shape, "/pool/pool_price_usd").map(|p| p.to_string()).unwrap_or_else(|| "?".to_string()),
            passed.join(", ")
        ));
        out["headline"] = json!("Ladder: NOCK is at or below the bottom of your ladder shape, so no deposit is measured above it.");
        return out;
    }

    // 4. The pool the shape is drawn in (the price-source pool) must be able to hold it.
    let Some(target) = target else {
        set(&mut out, "unavailable", "the pool your ladder shape is drawn in is not among this snapshot's positions".to_string());
        return out;
    };
    let name = pool_name(target);
    if let Err(reason) = eligibility(target) {
        out["excluded"] = json!([{ "position_id": target.position_id, "reason": reason }]);
        set(&mut out, "no_target_pool", format!("Your ladder shape is drawn in {name}, the price-source pool, and it cannot hold a ladder this cycle; its reason is listed."));
        return out;
    }

    // 5. The reserve gate and what it allows for what is left.
    let (Some(reserve), Some(gate_usd), Some(pool_price)) = (n(&shape, "/gate/cash_reserve_usdc").or_else(|| gs.cash_reserve_usdc.as_deref().and_then(|v| v.parse().ok())), n(&shape, "/gate/price_usd"), n(&shape, "/pool/pool_price_usd")) else {
        set(&mut out, "unavailable", "the reserve gate of your ladder shape could not be read this cycle".to_string());
        return out;
    };
    // The gate in the operator's own figure when it is their ladder_bottom_usd ("0.01").
    let gate_words = match bottom {
        Some(b) if (b - gate_usd).abs() < 1e-9 => format!("{b}"),
        _ => price4(gate_usd),
    };
    let allowed = n(&shape, "/fit/allowed_usdc").unwrap_or(0.0);
    let needed = n(&shape, "/fit/needed_usdc").unwrap_or(0.0);
    let minimum = n(&shape, "/fit/minimum_usdc").unwrap_or(LADDER_SUGGEST_MIN_USDC);
    if shape_status == "no_room" {
        let left_words = count("to_deposit") + count("below_minimum") + count("not_deposited");
        out["status"] = json!("no_headroom");
        out["headline"] = json!(format!(
            "Ladder: {left_words} of {of} deposits left, but your reserve leaves under ${} for them.",
            fx(minimum, 2)
        ));
        let would_take = if needed > allowed + 0.005 { format!(", which would take ${}", fx(cents_down(needed), 2)) } else { String::new() };
        let done = rows.iter().any(|r| !matches!(r["status"].as_str(), Some("to_deposit") | Some("below_minimum")));
        out["reason"] = json!(format!(
            "With your ${} reserve kept for below ${gate_words}, the reserve gate allows ${} for what is left of your shape{would_take}: nothing above the ${} minimum fits, so no deposit is listed{}.",
            dollars_short(reserve),
            fx(allowed, 2),
            fx(minimum, 2),
            if done { format!(" ({})", tally()) } else { String::new() },
        ));
        return out;
    }
    if shape_status == "held" {
        let held: Vec<String> = rows.iter().filter(|r| r["status"] == "held").map(|r| format!("Deposit {} ({}): {}", r["index"], rung_words(s(r, "/rung").unwrap_or("?")), s(r, "/reason").unwrap_or("no reason given"))).collect();
        set(&mut out, "nock_exceeds", format!("{}. Nothing else is left to deposit.", held.join("; ")));
        return out;
    }
    if s(&shape, "/gate/status") == Some("breached") && n(&shape, "/gate/margin_usdc").unwrap_or(0.0) < -0.01 {
        // The deposits are sized to fit; a breach here is a defect, stated, never hidden.
        set(
            &mut out,
            "unavailable",
            format!(
                "the deposits left were sized to the reserve gate, yet it reads ${} at ${gate_words}, below your ${} reserve: no deposit is listed until this is resolved",
                s(&shape, "/gate/stable_at_gate_usdc").unwrap_or("?"),
                dollars_short(reserve)
            ),
        );
        return out;
    }
    let book = stored_book(data);
    if !book.problems.is_empty() {
        set(&mut out, "unavailable", book.problems.join("; "));
        return out;
    }
    let wallet_usdc: f64 = data.wallet_balances.iter().find(|w| w.token_symbol == "USDC").and_then(|w| w.amount.parse().ok()).unwrap_or(0.0);
    let lp_nock_budget = s(&shape, "/nock_budget/lp_nock_budget").map(|b| b.to_string());

    // 6. One deposit per rung left to deposit, with the exact mint of the remainder.
    let nock_is_token1 = is_nock_token(&target.token1);
    let (usdc_tok, nock_tok) = if nock_is_token1 { (&target.token0, &target.token1) } else { (&target.token1, &target.token0) };
    let token = |t: &PositionTokenInfo| json!({ "symbol": t.symbol, "address": t.address, "decimals": t.decimals });
    let app = if target.protocol == "aerodrome-cl" { "the Aero app" } else { "the Uniswap web app" };
    let shape_rungs = shape["rungs"].as_array().cloned().unwrap_or_default();
    let to_deposit: Vec<&Value> = shape_rungs.iter().filter(|r| r["status"] == "to_deposit").collect();
    let mut rungs: Vec<Value> = Vec::new();
    let mut legs: Vec<(String, Leg)> = Vec::new();
    let mut bars: Vec<Value> = Vec::new();
    // What the card tells the operator to deposit, cut to cents: the headline, the
    // outcomes and the wallet line add these, so the card states one figure per deposit.
    let (mut usdc_sum, mut nock_sum) = (0.0, 0.0);
    for r in &to_deposit {
        let index = r["index"].as_u64().unwrap_or(0) as usize;
        let role = s(r, "/rung").unwrap_or("?").to_string();
        let exact = (
            r["tick_lower"].as_i64(),
            r["tick_upper"].as_i64(),
            s(r, "/liquidity_raw"),
            s(r, "/usdc_raw"),
            s(r, "/nock_raw"),
            s(r, "/usdc"),
            s(r, "/nock"),
        );
        let (Some(tl), Some(tu), Some(liquidity_raw), Some(usdc_raw), Some(nock_raw), Some(usdc_exact), Some(nock_exact)) = exact else {
            set(&mut out, "unavailable", format!("the exact mint of deposit {index} ({}) was not worked out this cycle: no deposit is listed", rung_words(&role)));
            return out;
        };
        let (tl, tu) = (tl as i32, tu as i32);
        // NOCK as token1: a higher USD price is a lower tick, so the top is tick_lower.
        let (top_tick, bottom_tick) = if nock_is_token1 { (tl, tu) } else { (tu, tl) };
        let (top_exact, top_npu) = tick_prices(target, top_tick);
        let (bottom_exact, bottom_npu) = tick_prices(target, bottom_tick);
        let (usdc_f, nock_f) = (usdc_exact.parse::<f64>().unwrap_or(0.0), nock_exact.parse::<f64>().unwrap_or(0.0));
        let holds_nock = nock_f > 0.0;
        let in_range = r["in_range_now"].as_bool();
        let usdc_enter = cut_dp(usdc_exact, 2);
        // Every figure of this deposit's USDC is the same cut cents, typed or "about".
        let usdc_about = fx(cents_down(usdc_f), 2);
        let nock_enter = cut_dp(nock_exact, 4);
        let deployed_usdc = n(r, "/deployed_usdc").unwrap_or(0.0);
        let part = if deployed_usdc > 0.0 && role != "cap" {
            format!("; the rest of this rung (${} of its ${} target is already deployed)", fx(deployed_usdc, 2), fx(n(r, "/target_usdc").unwrap_or(0.0), 2))
        } else {
            String::new()
        };
        let cut = if r["fit_factor"].is_string() { ", after the reserve gate's cut" } else { "" };
        let summary = if holds_nock {
            format!(
                "about {} NOCK + ${} USDC, {}{part}{cut}",
                fx(nock_f, 2),
                fx(cents_down(usdc_f), 2),
                match in_range {
                    Some(true) => "in range now",
                    Some(false) => "not in range now",
                    None => "its range against the price not read",
                }
            )
        } else {
            format!("${usdc_enter} USDC, no NOCK, below the price now{part}{cut}")
        };
        // The app derives the other side from whichever field is typed. A deposit that
        // holds NOCK states both limits, which one binds now, and the field that keeps it.
        let cap = r["cap"].is_object().then(|| &r["cap"]);
        let usdc_limit = cap.and_then(|c| n(c, "/usdc_limit"));
        let bound_by = cap.and_then(|c| s(c, "/bound_by"));
        let limit_owner = match cap.and_then(|c| s(c, "/usdc_limit_is")) {
            Some("near_share") => "the most that keeps it at or below the near rung's USDC",
            Some("reserve") => "what your reserve leaves for it beside the other deposits listed",
            Some("total_usdc") => "the ladder's USDC",
            _ => "the near rung's USDC",
        };
        let usdc_bound = matches!(bound_by, Some("near_usdc") | Some("total_usdc") | Some("reserve"));
        let nock_limit = holds_nock.then(|| match (cap, &lp_nock_budget, usdc_limit) {
            (Some(_), Some(b), Some(ul)) if usdc_bound => json!({
                "nock": b,
                "usdc": fx(ul, 2),
                "is": "near_usdc",
                "binding": "usdc",
                "field_to_type": "USDC",
                "usdc_to_enter": usdc_enter,
                "nock_then_about": fx(nock_f, 4),
                "holds_less_than_budget": true,
                "text": format!(
                    "The USDC side must not exceed ${}, {limit_owner}, which binds now, so the cap holds about {} NOCK, less than your {b} NOCK LP budget. The limit holds when the USDC field is the one typed, at ${usdc_enter} or less. {LADDER_USDC_LIMIT_WHY}",
                    fx(ul, 2),
                    fx(nock_f, 4),
                ),
            }),
            (Some(_), Some(b), ul) => json!({
                "nock": b,
                "usdc": ul.map(|u| fx(u, 2)),
                "is": "lp_nock_budget",
                "binding": "nock",
                "field_to_type": "NOCK",
                "nock_to_enter": nock_enter,
                "usdc_then_about": usdc_about.clone(),
                "holds_less_than_budget": false,
                "text": match ul {
                    Some(ul) => format!(
                        "The NOCK side must not exceed {b} NOCK, your LP NOCK budget, which binds now; the USDC side must not exceed ${}, {limit_owner}. The limits hold when the NOCK field is the one typed, at {nock_enter} NOCK or less, and the app shows at most ${} of USDC: about ${} at this reading, more if the price rises inside the band before you deposit. {LADDER_NOCK_LIMIT_WHY}",
                        fx(ul, 2),
                        fx(ul, 2),
                        usdc_about.clone(),
                    ),
                    None => format!(
                        "The NOCK side must not exceed {b} NOCK, your LP NOCK budget. The limit holds when the NOCK field is the one typed, at {nock_enter} NOCK or less. {LADDER_NOCK_LIMIT_WHY}"
                    ),
                },
            }),
            _ => json!({
                "nock": nock_enter,
                "usdc": Value::Null,
                "is": "this deposit's NOCK",
                "binding": "nock",
                "field_to_type": "NOCK",
                "nock_to_enter": nock_enter,
                "usdc_then_about": usdc_about.clone(),
                "holds_less_than_budget": Value::Null,
                "text": format!(
                    "The NOCK side must not exceed {nock_enter} NOCK, this deposit's own share of your LP NOCK budget. The limit holds when the NOCK field is the one typed, at {nock_enter} NOCK or less. {LADDER_NOCK_LIMIT_WHY}"
                ),
            }),
        });
        let mut link = deep_link(target, tl, tu, &usdc_enter);
        if holds_nock {
            link["amount_to_enter"] = json!(if usdc_bound && cap.is_some() {
                format!("Amount to enter in the app: ${usdc_enter} USDC, in the USDC field; the app then works out the NOCK side, about {} NOCK at this reading.", fx(nock_f, 4))
            } else {
                format!("Amount to enter in the app: {nock_enter} NOCK, in the NOCK field; the app then works out the USDC side, about ${} at this reading.", usdc_about.clone())
            });
        }
        let would_sign = if holds_nock {
            format!(
                "In {app}, with your own wallet: one new position in {name}, ticks {tl} to {tu}, holding up to {nock_enter} NOCK and about ${} USDC (the app may first ask for USDC and NOCK approvals). NockNock builds no transaction and signs nothing.",
                usdc_about.clone()
            )
        } else {
            format!(
                "In {app}, with your own wallet: one new position in {name}, ticks {tl} to {tu}, holding up to ${usdc_enter} USDC and 0 NOCK (the app may first ask for a USDC approval). NockNock builds no transaction and signs nothing."
            )
        };
        let top_usd = fx(top_exact, 8);
        let bottom_usd = fx(bottom_exact, 8);
        rungs.push(json!({
            "index": index,
            "of": of,
            "rung": role,
            "status": "to_deposit",
            "summary": summary,
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
                "pool_price_usd": shape["pool"]["pool_price_usd"],
                "note": "a NEW position (a new NFT) in this pool; position_id is the tracked position the pool was read from",
            },
            "tick_lower": tl,
            "tick_upper": tu,
            "top": { "tick": top_tick, "price_usd": top_usd, "pct_of_spot": r["high_pct"], "rule": r["high_rule"] },
            "bottom": { "tick": bottom_tick, "price_usd": bottom_usd, "pct_of_spot": r["low_pct"] },
            "bounds_usd": { "low": bottom_usd, "high": top_usd },
            // NOCK per USDC, from the ticks: the top (dearest NOCK) is the fewest.
            "bounds_nock_per_usdc": { "low": fx(top_npu, 4), "high": fx(bottom_npu, 4) },
            "in_range_now": r["in_range_now"],
            // Exact, from the shape's mint: `raw` and `mint` are what a transaction builder
            // consumes; `to_enter` is the amount cut to what a person types.
            "usdc": { "amount": usdc_exact, "raw": usdc_raw, "decimals": usdc_tok.decimals, "address": usdc_tok.address, "to_enter": usdc_enter },
            "nock": { "amount": nock_exact, "raw": nock_raw, "decimals": nock_tok.decimals, "address": nock_tok.address, "to_enter": nock_enter },
            "mint": { "liquidity_raw": liquidity_raw, "amount0_raw": r["amount0_raw"], "amount1_raw": r["amount1_raw"] },
            "target": {
                "usdc": fx(n(r, "/target_usdc").unwrap_or(0.0), 2),
                "liquidity": r["target_liquidity"],
                "tracked_liquidity": r["tracked_liquidity"],
                "deployed_usdc": fx(deployed_usdc, 2),
                "counterparts": r["counterparts"],
            },
            "fit_factor": r["fit_factor"],
            "at_bottom": { "nock_held": r["nock_held_at_bottom"], "nock_bought": r["nock_bought_at_bottom"], "avg_buy_price_usd": r["avg_buy_price_usd"] },
            // A cap: which limit sized it (lp_nock_budget, or the near rung's USDC), in words.
            "cap": if r["cap"].is_object() { json!({ "bound_by": r["cap"]["bound_by"], "usdc_limit": r["cap"]["usdc_limit"], "usdc_limit_is": r["cap"]["usdc_limit_is"], "note": r["cap"]["note"] }) } else { Value::Null },
            "nock_limit": nock_limit,
            "fee_estimate": fee_estimate_summary(&r["fee_estimate"]),
            "deep_link": link,
            "would_sign": would_sign,
        }));
        if let Some(leg) = Leg::new(top_exact, bottom_exact, pool_price, usdc_f, nock_f) {
            legs.push((role.clone(), leg));
        }
        usdc_sum += cents_down(usdc_f);
        nock_sum += nock_f;
    }
    if rungs.is_empty() {
        set(&mut out, "unavailable", "your ladder shape reads computed, yet no rung is left to deposit".to_string());
        return out;
    }
    let usdc_sum = (usdc_sum * 100.0).round() / 100.0;

    // 7. The picture: one bar per rung with ticks, deployed ones marked.
    for row in out["shape"]["rungs"].as_array().cloned().unwrap_or_default() {
        let Some(b) = row.get("bounds_usd").filter(|b| b.is_object()) else { continue };
        let status = s(&row, "/status").unwrap_or("");
        if !matches!(status, "to_deposit" | "deployed" | "over" | "found") {
            continue;
        }
        let dep = rungs.iter().find(|r| r["index"] == row["index"]);
        let (usdc, nock) = match dep {
            Some(d) => (n(d, "/usdc/amount").unwrap_or(0.0), n(d, "/nock/amount").unwrap_or(0.0)),
            None => match row.get("holds_now").filter(|h| h.is_object()) {
                Some(h) => (n(h, "/usdc").unwrap_or(0.0), n(h, "/nock").unwrap_or(0.0)),
                None => (n(&row, "/deployed_usdc").unwrap_or(0.0), n(&row, "/deployed_nock").unwrap_or(0.0)),
            },
        };
        let label = match (dep, nock >= 0.5) {
            (_, true) => format!("${} + {} NOCK", fx(cents_down(usdc), 2), grouped(nock)),
            (Some(d), false) => format!("${}", s(d, "/usdc/to_enter").unwrap_or("?")),
            (None, false) => format!("${}", fx(usdc, 2)),
        };
        bars.push(json!({
            "index": row["index"],
            "rung": row["rung"],
            "status": status,
            "deployed": dep.is_none(),
            "low_usd": b["low"],
            "high_usd": b["high"],
            "usdc": match dep.and_then(|d| s(d, "/usdc/to_enter")).filter(|_| nock < 0.5) {
                Some(cut) => cut.to_string(),
                None => fx(cents_down(usdc), 2),
            },
            "label": label,
        }));
    }
    let ladder_low = bars.iter().filter_map(|b| n(b, "/low_usd")).fold(f64::INFINITY, f64::min);
    out["picture"] = json!({
        "spot_usd": shape["pool"]["pool_price_usd"],
        "bottom_usd": fx(ladder_low, 8),
        "bottom_requested_usd": bottom.map(|b| format!("{b}")),
        "reserve_usdc": fx(reserve, 2),
        "reserve_label": format!("${} kept for below ${gate_words}", dollars_short(reserve)),
        "bars": bars,
    });

    // 8. What the deposits leave in the wallet (critique, "funds-safety" lens): the
    //    reserve counts USDC inside NOCK ranges, so the wallet alone ends below it.
    let existing_now: f64 = book.raw.positions.iter().map(|p| existing_at_ratio(p, 1.0).0).sum();
    let existing_at_gate: f64 = book.raw.positions.iter().map(|p| existing_at_ratio(p, gate_usd / pool_price).0).sum();
    let wallet_after = wallet_usdc - usdc_sum;
    if wallet_after < -0.005 {
        // The shape is sized within the wallet; a block that is not is not listed.
        out["picture"] = Value::Null;
        set(&mut out, "no_headroom", format!("The wallet holds ${} USDC, less than the ${} what is left of your shape would take: no deposit is listed.", fx(wallet_usdc, 2), fx(usdc_sum, 2)));
        return out;
    }
    let stable_at_gate = wallet_after + existing_at_gate;
    // The two parts are printed so that they add up to the total as printed.
    let cents = |v: f64| (v * 100.0).round() / 100.0;
    let in_ranges_at_gate = fx(cents(stable_at_gate) - cents(wallet_after), 2);
    let wallet_after_text = format!(
        "After these deposits your wallet holds about ${} USDC; the ${} reserve counts the ${} of USDC inside your existing NOCK ranges. At ${gate_words} with every rung converted, the stable balance reads ${}: the wallet's ${} plus ${in_ranges_at_gate} still inside those ranges.",
        grouped(wallet_after),
        dollars_short(reserve),
        grouped(existing_now),
        fx(stable_at_gate, 2),
        fx(wallet_after, 2),
    );
    out["wallet_after"] = json!({
        "wallet_usdc_now": fx(wallet_usdc, 2),
        "deposits_usdc": fx(usdc_sum, 2),
        "wallet_usdc_after": fx(wallet_after, 2),
        "existing_ranges_usdc_now": fx(existing_now, 2),
        "existing_ranges_usdc_at_gate": in_ranges_at_gate,
        "gate_price_usd": fx(gate_usd, 8),
        "stable_at_gate_usdc": fx(stable_at_gate, 2),
        "cash_reserve_usdc": fx(reserve, 2),
        "wallet_alone_below_reserve": wallet_after < reserve,
        "rule": "the standing reserve is measured on wallet USDC plus USDC inside NOCK ranges, read at the gate price",
        "text": wallet_after_text,
    });

    // 9. The fit: when the reserve gate allows less than what is left, every deposit is
    //    cut in proportion and the shortfall is stated in dollars.
    let fit_status = s(&shape, "/fit/status").unwrap_or("fits");
    out["fit"] = json!({
        "status": fit_status,
        "allowed_usdc": shape["fit"]["allowed_usdc"],
        "needed_usdc": shape["fit"]["needed_usdc"],
        "shortfall_usdc": shape["fit"]["shortfall_usdc"],
        "rule": shape["fit"]["rule"],
        "text": (fit_status == "reduced").then(|| format!(
            "Your ${} reserve, kept for below ${gate_words}, allows ${} for what is left of your shape, ${} short of the ${} it would take: each deposit is cut by the same fraction, so the shape keeps its proportions.",
            dollars_short(reserve),
            fx(allowed, 2),
            s(&shape, "/fit/shortfall_usdc").unwrap_or("?"),
            fx(cents_down(needed), 2),
        )),
    });

    // 10. The numbers, the gates and the stated lines.
    let remaining_usdc = fx(usdc_sum, 2);
    let binding = if usdc_sum >= wallet_usdc - 0.01 { "wallet" } else { "reserve" };
    let nock_at_bottom = n(&shape, "/combined/nock_held_at_deepest_bottom");
    let nock_bought = n(&shape, "/combined/nock_bought_at_deepest_bottom");
    let avg = n(&shape, "/combined/avg_buy_price_usd");
    let deepest = n(&shape, "/combined/deepest_bottom_usd").unwrap_or(ladder_low);
    out["rungs"] = json!(rungs);
    // How every rung's `mint` was worked out, once.
    out["mint_note"] = to_deposit.iter().find_map(|r| r.get("mint_note").filter(|v| v.is_string()).cloned()).unwrap_or(Value::Null);
    let nock_status = s(&shape, "/nock_budget/status").unwrap_or("unknown");
    out["numbers"] = json!({
        "usdc_max": remaining_usdc,
        "binding": binding,
        "nock_now": fx(nock_sum, 4),
        "nock_at_bottom": nock_at_bottom.map(|v| fx(v, 4)),
        "nock_bought_at_bottom": nock_bought.map(|v| fx(v, 4)),
        "avg_buy_price_usd": shape["combined"]["avg_buy_price_usd"],
        "pool_price_usd": shape["pool"]["pool_price_usd"],
        "wallet_usdc": fx(wallet_usdc, 2),
        "wallet_usdc_after": fx(wallet_after, 2),
        "cash_reserve_usdc": fx(reserve, 2),
        "gate_price_usd": fx(gate_usd, 8),
        "existing_ranges_usdc_now": fx(existing_now, 2),
        "existing_ranges_usdc_at_gate": in_ranges_at_gate,
        "stable_at_gate_usdc": fx(stable_at_gate, 2),
        "lp_nock_budget": lp_nock_budget,
        "shape_total_usdc": shape["total_usdc"],
        "shape_rest_usdc": shape["rest_usdc"],
        "shape_tracked_usdc": shape["tracked_usdc"],
        "headroom_usdc": shape["headroom_usdc"],
    });
    out["gates"] = json!({
        "ladder_tracked": {
            "status": if out["tracked"].as_array().map_or(true, |t| t.is_empty()) { "none" } else { "counted" },
            "reason": "tracked ladder rungs are part of your shape: each is matched to a rung, and only what is left is listed",
        },
        "shape_setting": { "status": "set", "key": LADDER_SHAPE_KEY },
        "bottom_setting": match bottom {
            Some(b) => json!({ "status": "set", "key": LADDER_BOTTOM_KEY, "value": format!("{b}") }),
            None => json!({ "status": "not_set", "key": LADDER_BOTTOM_KEY, "value": Value::Null }),
        },
        "reserve": {
            "status": if fit_status == "reduced" { "reduces" } else { "allows" },
            "key": LADDER_RESERVE_KEY,
            "binding": binding,
            "cash_reserve_usdc": fx(reserve, 2),
            "read_at_usd": fx(gate_usd, 8),
            "read_at_rule": shape["gate"]["rule"],
            "max_usdc": shape["headroom_usdc"],
            "shape_total_usdc": shape["total_usdc"],
            "minimum_usdc": fx(minimum, 2),
            "stable_at_gate_usdc": shape["gate"]["stable_at_gate_usdc"],
            "margin_usdc": shape["gate"]["margin_usdc"],
            "rule": "the stable balance at the gate price stays at or above the reserve",
        },
        "nock_budget": {
            "status": nock_status,
            "nock_needed": shape["nock_budget"]["nock_needed"],
            "lp_nock_budget": lp_nock_budget,
            "reason": Value::Null,
        },
        "floor_breach_effect": shape["floor_breach_effect"]["status"],
    });
    // The plan budget and the tax reserve as the simulator states them (any range reads
    // the same two lines); the below-bottom reading belongs to one range and is left out.
    out["stated"] = match sim_with(env, &target.position_id, ("low_pct", -50.0), LADDER_SUGGEST_MIN_USDC.min(wallet_usdc).max(1e-6)) {
        Ok(body) => stated(env, &body, Value::Null),
        Err(_) => json!({ "note": LADDER_STATED_NOT_GATES, "lines": [] }),
    };
    out["after_deposit"] = after_deposit(data.derived_metrics.position_detection.as_ref());

    // 11. The headline counts only what is left, and adds the amounts the card states.
    let nock_words = if nock_sum > 0.0 { format!(" and about {} NOCK", grouped(nock_sum)) } else { String::new() };
    out["headline"] = json!(format!("Ladder: {left} of {of} deposits left, up to ${remaining_usdc} USDC{nock_words}."));
    out["outcomes"] = json!(shape_outcomes(&legs, pool_price, deepest, usdc_sum, nock_bought, avg, left < of));
    let done = if left < of { format!(" ({})", tally()) } else { String::new() };
    let converted = match (nock_at_bottom, nock_bought, avg) {
        (Some(held), Some(bought), Some(avg)) => format!(
            " Fully converted at ${} they would hold about {} NOCK, about {} of it bought, at an average of ${}.",
            price4(deepest),
            grouped(held),
            grouped(bought),
            price4(avg)
        ),
        _ => String::new(),
    };
    let limit_text = if binding == "wallet" {
        "all the wallet's USDC".to_string()
    } else if fit_status == "reduced" {
        format!("cut to what your ${} reserve at ${gate_words} allows", dollars_short(reserve))
    } else {
        format!("sized to leave the stable balance at or above your ${} reserve at ${gate_words}", dollars_short(reserve))
    };
    out["sentence"] = json!(format!(
        "Your ladder shape in {name}: {left} of {of} deposits {} left{done}, holding up to ${remaining_usdc} USDC and {} NOCK now: {limit_text}.{converted}",
        if left == 1 { "is" } else { "are" },
        if nock_sum > 0.0 { fx(nock_sum, 2) } else { "0".to_string() },
    ));
    out["what_changed"] = json!(format!("Your ladder shape has {left} of {of} deposits left in {name}: up to ${remaining_usdc} USDC{nock_words}."));
    out["status"] = json!("ready");
    out["shown"] = json!(true);
    out
}

/// The shape's outcomes in plain words, for what is left to deposit: a fall to the bottom
/// of the first rung below the cap that is not the last, a fall to the ladder's bottom in
/// total, and a rise. `bought` and `avg` are the shape's own combined figures when it
/// states them. Pure.
fn shape_outcomes(legs: &[(String, Leg)], pool_price: f64, deepest: f64, usdc: f64, bought: Option<f64>, avg: Option<f64>, some_done: bool) -> Vec<Value> {
    let mut out = Vec::new();
    if legs.is_empty() || !(pool_price > 0.0) {
        return out;
    }
    let pct = |p: f64| pct_plain((p / pool_price - 1.0) * 100.0);
    let buy_to = |p: f64| -> (f64, f64) {
        let spent: f64 = legs.iter().map(|(_, l)| l.usdc_at(pool_price) - l.usdc_at(p)).sum();
        let nock: f64 = legs.iter().map(|(_, l)| l.nock_at(p) - l.nock_at(pool_price)).sum();
        (spent, nock)
    };
    let these = if some_done { "these deposits" } else { "the ladder" };
    // The first rung below the cap whose bottom is above the ladder's: a fall to it.
    let below_cap: Vec<&(String, Leg)> = legs.iter().filter(|(role, _)| role != "cap").collect();
    if let Some((role, leg)) = below_cap.first().copied().filter(|rl| rl.1.bottom > deepest * (1.0 + 1e-6)) {
        let (spent, nock) = buy_to(leg.bottom);
        if nock > 0.0 {
            let avg_here = spent / nock;
            out.push(json!({
                "kind": "fall_inside",
                "price_usd": fx(leg.bottom, 8),
                "pct_of_spot": pct(leg.bottom),
                "nock_bought": fx(nock, 0),
                "usdc_spent": fx(spent, 2),
                "avg_buy_price_usd": fx(avg_here, 8),
                "text": format!(
                    "A fall to ${} ({}), the {}'s bottom, would buy about {} NOCK for ${} of the USDC, an average of ${}.",
                    price4(leg.bottom), pct(leg.bottom), rung_words(role), grouped(nock), fx(spent, 2), price4(avg_here)
                ),
            }));
        }
    }
    let (spent_all, nock_all) = buy_to(deepest);
    let (nock_total, avg_total) = (bought.unwrap_or(nock_all), avg.unwrap_or(if nock_all > 0.0 { spent_all / nock_all } else { 0.0 }));
    out.push(json!({
        "kind": "fall_to_bottom",
        "price_usd": fx(deepest, 8),
        "pct_of_spot": pct(deepest),
        "nock_bought": fx(nock_total, 0),
        "usdc_spent": fx(usdc, 2),
        "avg_buy_price_usd": fx(avg_total, 8),
        "text": if some_done {
            format!(
                "A fall to ${} ({}), the bottom of what is left to deposit, would buy about {} NOCK in total with all ${}, an average of ${}.",
                price4(deepest), pct(deepest), grouped(nock_total), fx(usdc, 2), price4(avg_total)
            )
        } else {
            format!(
                "A fall to the ladder bottom ${} ({}) would buy about {} NOCK in total with all ${}, an average of ${}.",
                price4(deepest), pct(deepest), grouped(nock_total), fx(usdc, 2), price4(avg_total)
            )
        },
    }));
    // A rise: only a rung that holds NOCK now (the cap) sells anything.
    let holders: Vec<&(String, Leg)> = legs.iter().filter(|(_, l)| l.nock_at(pool_price) > 0.0).collect();
    match holders.as_slice() {
        [] => out.push(json!({
            "kind": "rise",
            "text": format!("A rise sells nothing: {these} hold{} 0 NOCK, so {} USDC stays USDC.", if some_done { "" } else { "s" }, if some_done { "their" } else { "its" }),
        })),
        hs => {
            let top = hs.iter().map(|(_, l)| l.top).fold(0.0, f64::max);
            let sold: f64 = hs.iter().map(|(_, l)| l.nock_at(pool_price)).sum();
            let got: f64 = hs.iter().map(|(_, l)| l.usdc_at(top) - l.usdc_at(pool_price)).sum();
            let who = if hs.len() == 1 { format!("the {}'s NOCK", rung_words(&hs[0].0)) } else { "the NOCK these deposits hold".to_string() };
            out.push(json!({
                "kind": "rise",
                "price_usd": fx(top, 8),
                "pct_of_spot": pct(top),
                "nock_sold": fx(sold, 0),
                "usdc_received": fx(got, 2),
                "text": format!(
                    "A rise to ${} ({}) or above sells {who}, about {}, for about ${} of USDC, and sells nothing else: every other rung holds 0 NOCK.",
                    price4(top), pct(top), grouped(sold), fx(got, 2)
                ),
            }));
        }
    }
    out
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
