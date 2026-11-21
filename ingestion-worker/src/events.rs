//! Calls to action as structured events (issue #169): `derived_metrics.events[]`.
//!
//! Every call to action the snapshot carries is one entry, with a tier:
//! - `top`: maintenance the operator signed up for (the ladder's rules, a due tranche of the
//!   plan, a detected buy waiting to be confirmed, a deposit of the ladder suggestion, a
//!   position found on-chain waiting for a tap), and floor and reserve breaches. The
//!   dashboard and the phone cockpit show these at the top of the page.
//! - `quiet`: optional pickups and facts (collectable fees, AERO rewards, the LP rebuild,
//!   the range exit, a closed position, the floor-crossing price, a cap that sold its NOCK,
//!   a reserve dip that the ranges' own buying explains). They stay where they are shown
//!   today.
//!
//! Each entry is shaped for a later push channel (v0.10): `id`, `kind`, `tier`, `title`,
//! `what_changed`, `text`, `numbers` (the measured figures, as strings), `since`,
//! `deep_link` (an anchor the dashboard and the cockpit render), `links`, `would_sign`
//! (what the operator would sign, or null) and `card` (the dashboard card that renders the
//! entry in full, or null when the top-of-page strip does). The texts are built from the
//! snapshot's own figures; nothing here changes the directive or a push (notify.rs is
//! unchanged). `since` is carried from the previous snapshot by id while the cycles are
//! continuous. Pure.
use crate::*;
use serde_json::{json, Value};


/// The tier of a directive title: top for a floor breach (DEFENSIVE LOCK), a due tranche
/// and a plan that is not running (DCA PLAN INVALID); quiet for every other title (the
/// first ` · ` segment decides). Pure.
pub(crate) fn directive_tier(action: &str) -> &'static str {
    let first = action.split(" · ").next().unwrap_or("").trim();
    if first.starts_with("DEFENSIVE LOCK") || first.starts_with("DCA TRANCHE DUE") || first.starts_with("DCA PLAN INVALID") {
        "top"
    } else {
        "quiet"
    }
}

/// The anchor of an event id. Pure.
pub(crate) fn event_anchor(id: &str) -> String {
    let slug: String = id.chars().map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' }).collect();
    format!("#event-{slug}")
}

fn f(s: &str) -> Option<f64> {
    s.trim().parse::<f64>().ok().filter(|v| v.is_finite())
}

fn m2(v: f64) -> String {
    let r = (v * 100.0).round() / 100.0 + 0.0;
    format!("{:.2}", r)
}

fn is_usdc_sym(t: &PositionTokenInfo) -> bool {
    t.symbol == "USDC"
}

/// USDC a NOCK/USDC range holds at or above its top, from its liquidity; a full-range
/// position's USDC now (it has no top). None for a pair without USDC and NOCK. Pure.
pub(crate) fn usdc_at_top_of(p: &Position) -> Option<f64> {
    let (nock1, usdc0) = (is_nock_token(&p.token1), is_usdc_sym(&p.token0));
    let (nock0, usdc1) = (is_nock_token(&p.token0), is_usdc_sym(&p.token1));
    if !((nock1 && usdc0) || (nock0 && usdc1)) {
        return None;
    }
    if p.full_range {
        return f(if usdc0 { &p.amount0 } else { &p.amount1 });
    }
    let l = f(&p.liquidity_amount)?;
    let (sa, sb) = (tick_sqrt_price(p.lower_tick), tick_sqrt_price(p.upper_tick));
    if usdc0 {
        let (a0, _) = concentrated_amounts_at(l, sa, sa, sb);
        Some(a0 / 10f64.powi(p.token0.decimals))
    } else {
        let (_, a1) = concentrated_amounts_at(l, sb, sa, sb);
        Some(a1 / 10f64.powi(p.token1.decimals))
    }
}

/// The reserve dip (#162 reserve, operator 2026-11-20): an entry only while the stable
/// balance is below the standing reserve. Quiet when the NOCK ranges' own buying explains
/// it, that is when wallet USDC plus the USDC those ranges hold back at their tops (the
/// USDC a rise back above them returns) still covers the reserve; top otherwise. Pure.
pub(crate) fn reserve_event(data: &SnapshotData) -> Option<Value> {
    let g = &data.derived_metrics.global_status;
    if let Some(note) = g.reserve_note.as_ref() {
        return Some(json!({
            "id": "reserve_setting",
            "kind": "reserve_setting",
            "tier": "quiet",
            "title": "RESERVE SETTING NOT APPLIED",
            "what_changed": "The standing reserve setting is not applied.",
            "text": note,
            "numbers": {},
            "links": [],
            "would_sign": Value::Null,
            "card": Value::Null,
            "source": "derived_metrics.global_status.reserve_note",
        }));
    }
    let reserve = g.cash_reserve_usdc.as_deref().and_then(f)?;
    let headroom = g.reserve_headroom_usdc.as_deref().and_then(f)?;
    if headroom >= 0.0 {
        return None;
    }
    let stable = g.reserve_stable_usdc.as_deref().and_then(f).unwrap_or(reserve + headroom);
    let wallet: f64 = data
        .wallet_balances
        .iter()
        .filter(|b| b.token_symbol == "USDC")
        .filter_map(|b| f(&b.amount))
        .sum();
    let ladder = g.reserve_ladder_usdc.as_deref().and_then(f).unwrap_or(0.0);
    let lp = (stable - wallet - ladder).max(0.0);
    let at_tops: f64 = data.positions.iter().filter_map(usdc_at_top_of).sum();
    let recoverable = wallet + at_tops;
    let explained = recoverable >= reserve;
    let split = format!(
        "Stable ${} (wallet USDC ${} + NOCK LP USDC ${}{}) is ${} below your ${} reserve.",
        m2(stable),
        m2(wallet),
        m2(lp),
        if ladder > 0.0 { format!(" + ladder USDC ${}", m2(ladder)) } else { String::new() },
        m2(-headroom),
        m2(reserve)
    );
    let (tier, text) = if explained {
        (
            "quiet",
            format!(
                "{split} The NOCK ranges bought NOCK with it on the way down: wallet USDC plus the USDC those ranges hold back at their tops is ${}, at or above the reserve, so the dip is their buying, which is what the reserve is for."
                , m2(recoverable)
            ),
        )
    } else {
        (
            "top",
            format!(
                "{split} More than the ranges' buying: wallet USDC plus the USDC the NOCK ranges hold back at their tops is ${}, ${} below the reserve."
                , m2(recoverable), m2(reserve - recoverable)
            ),
        )
    };
    Some(json!({
        "id": "reserve_below",
        "kind": "reserve_below",
        "tier": tier,
        "title": "BELOW THE STANDING RESERVE",
        "what_changed": "The stable balance is below your standing reserve.",
        "text": text,
        "numbers": {
            "reserve_usdc": m2(reserve),
            "stable_usdc": m2(stable),
            "shortfall_usdc": m2(-headroom),
            "wallet_usdc": m2(wallet),
            "nock_lp_usdc": m2(lp),
            "ladder_usdc": m2(ladder),
            "wallet_plus_ranges_at_tops_usdc": m2(recoverable),
        },
        "links": [],
        "would_sign": Value::Null,
        "card": Value::Null,
        "source": "derived_metrics.global_status",
    }))
}

fn directive_event(data: &SnapshotData, prev: Option<&Value>) -> Value {
    let g = &data.derived_metrics.global_status;
    let action = g.shot_advice_action.as_str();
    let tier = directive_tier(action);
    let prev_title = prev.and_then(|p| p.get("title")).and_then(|t| t.as_str());
    let what_changed = match prev_title {
        Some(t) if t != action => format!("The directive changed from {t} to {action}."),
        _ => format!("The directive is {action}."),
    };
    let first = action.split(" · ").next().unwrap_or("");
    let would_sign = if first.starts_with("DCA TRANCHE DUE") {
        json!("A swap of the due tranche in your wallet app (the plan lists the venues it quoted). NockNock builds no transaction and signs nothing; the buy is found on-chain after it lands.")
    } else {
        Value::Null
    };
    json!({
        "id": "directive",
        "kind": "directive",
        "tier": tier,
        "title": action,
        "what_changed": what_changed,
        "text": g.shot_advice_details,
        "numbers": {},
        "links": [],
        "would_sign": would_sign,
        "card": "directive",
        "source": "derived_metrics.global_status.shot_advice_action",
    })
}

fn fill_offer_events(data: &SnapshotData) -> Vec<Value> {
    let Some(ps) = data.derived_metrics.accumulation_plan.as_ref() else { return vec![] };
    let Some(fd) = ps.fill_detection.as_ref() else { return vec![] };
    fd.offers
        .iter()
        .map(|o| {
            json!({
                "id": format!("fill_offer:{}", o.tx_hash.to_ascii_lowercase()),
                "kind": "fill_offer",
                "tier": "top",
                "title": "PLAN BUY FOUND: CONFIRM IT",
                "what_changed": "A swap that looks like a plan buy was found on-chain and is not counted yet.",
                "text": format!(
                    "${} USDC for {} NOCK at {} (tx {}). Confirm it or mark it as not part of the plan in the accumulation plan; until then the plan does not count it.",
                    o.usdc_spent.parse::<f64>().map(m2).unwrap_or_else(|_| o.usdc_spent.clone()),
                    o.nock_received.parse::<f64>().map(m2).unwrap_or_else(|_| o.nock_received.clone()),
                    o.confirmed_at,
                    o.tx_hash
                ),
                "numbers": { "usdc_spent": o.usdc_spent, "nock_received": o.nock_received, "effective_price_usd": o.effective_price_usd, "tx_hash": o.tx_hash },
                "links": [],
                "would_sign": Value::Null,
                "card": "accumulation_plan",
                "source": "derived_metrics.accumulation_plan.fill_detection.offers",
            })
        })
        .collect()
}

fn found_positions_event(data: &SnapshotData) -> Option<Value> {
    let d = data.derived_metrics.position_detection.as_ref()?;
    let n = d.pointer("/found_not_counted/count").and_then(|v| v.as_u64()).unwrap_or(0);
    if n == 0 {
        return None;
    }
    let value = d.pointer("/found_not_counted/value_usd").and_then(|v| v.as_str()).map(|v| format!(" worth ${v}")).unwrap_or_default();
    Some(json!({
        "id": "found_positions",
        "kind": "found_positions",
        "tier": "top",
        "title": "FOUND ON-CHAIN: ONE TAP TO TRACK",
        "what_changed": format!("{n} position{} found in the wallet {} not counted yet.", if n == 1 { "" } else { "s" }, if n == 1 { "is" } else { "are" }),
        "text": format!("{n} position{} found on-chain{value} {} not counted until one tap says how to track {}.", if n == 1 { "" } else { "s" }, if n == 1 { "is" } else { "are" }, if n == 1 { "it" } else { "them" }),
        "numbers": { "count": n.to_string() },
        "links": [],
        "would_sign": Value::Null,
        "card": "found_positions",
        "source": "derived_metrics.position_detection.found_not_counted",
    }))
}

fn ladder_suggestion_event(data: &SnapshotData) -> Option<Value> {
    let l = data.derived_metrics.ladder_suggestion.as_ref()?;
    if l.get("shown").and_then(|v| v.as_bool()) != Some(true) {
        return None;
    }
    Some(json!({
        "id": l.get("id").cloned().unwrap_or(json!(LADDER_SUGGEST_ID)),
        "kind": "ladder_suggestion",
        "tier": l.get("tier").cloned().unwrap_or(json!("top")),
        "title": "LADDER",
        "what_changed": l.get("what_changed").cloned().unwrap_or(Value::Null),
        "text": l.get("headline").cloned().unwrap_or(Value::Null),
        "numbers": {},
        "links": [],
        "would_sign": Value::Null,
        "card": "ladder_suggestion",
        "source": "derived_metrics.ladder_suggestion",
    }))
}

fn quiet_events(data: &SnapshotData) -> Vec<Value> {
    let mut out = Vec::new();
    let g = &data.derived_metrics.global_status;
    // Collectable LP fees: every row whose fees cover its own claim's gas (#153).
    // The fee rows at full precision (contract_fees), by the claim rule of the velocity panel.
    let rows: Vec<(f64, f64)> = data
        .contract_fees
        .iter()
        .filter(|c| c.claimable)
        .filter_map(|c| Some((f(&c.amount_usd)?, f(&c.gas_break_even_usd)?)))
        .filter(|(usd, gas)| recommended_claim_action(*usd, *gas) == "claim_now")
        .collect();
    if !rows.is_empty() {
        let total: f64 = rows.iter().map(|(u, _)| u).sum();
        let gas = rows[0].1;
        out.push(json!({
            "id": "harvest",
            "kind": "harvest",
            "tier": "quiet",
            "title": "LP FEES COLLECTABLE",
            "what_changed": "Uncollected LP fees cover their own claim's gas.",
            "text": format!(
                "${} of LP fees in {} position{} can be collected for about ${} of gas each. Optional; the BATCH HARVEST directive and its push still follow the $5.00 rule.",
                m2(total), rows.len(), if rows.len() == 1 { "" } else { "s" }, format!("{:.4}", gas)
            ),
            "numbers": { "claimable_usd": m2(total), "positions": rows.len().to_string(), "gas_per_claim_usd": format!("{:.4}", gas) },
            "links": [],
            "would_sign": "One collect transaction per position, in the venue's app or the Lite cockpit's harvest.",
            "card": "collectible",
            "source": "derived_metrics.velocity_gas_sandbox.contracts",
        }));
    }
    if let Some(r) = g.collectible_rewards_usd.as_deref().and_then(f).filter(|v| *v > 0.0) {
        out.push(json!({
            "id": "aero_rewards",
            "kind": "aero_rewards",
            "tier": "quiet",
            "title": "AERO REWARDS COLLECTABLE",
            "what_changed": "Gauge rewards are waiting in the Aero app.",
            "text": format!("${} of AERO rewards can be collected in the Aero app. Optional.", m2(r)),
            "numbers": { "rewards_usd": m2(r) },
            "links": [],
            "would_sign": Value::Null,
            "card": "collectible",
            "source": "derived_metrics.global_status.collectible_rewards_usd",
        }));
    }
    for c in &data.closed_positions {
        out.push(json!({
            "id": format!("closed:{}", c.position_id),
            "kind": "closed_position",
            "tier": "quiet",
            "title": "POSITION CLOSED",
            "what_changed": format!("{} no longer exists on-chain.", c.label),
            "text": format!("{}: {}", c.label, c.reason),
            "numbers": {},
            "links": [],
            "would_sign": Value::Null,
            "card": "closed_positions",
            "source": "closed_positions",
        }));
    }
    if let Some(fb) = data.derived_metrics.floor_breach.as_ref() {
        if let Some(c) = fb.crossing.as_ref().filter(|c| c.status == "crosses") {
            if let (Some(px), Some(pct)) = (c.price_usd.as_ref(), c.pct_above_spot.as_ref()) {
                out.push(json!({
                    "id": "floor_crossing",
                    "kind": "floor_crossing",
                    "tier": "quiet",
                    "title": "FLOOR-CROSSING PRICE",
                    "what_changed": "The deployed ranges would sell the stack below the floor above a price.",
                    "text": format!("If NOCK rose to ${px} (+{pct}%), the ranges would have sold the stack down to the {} NOCK floor.", c.level_nock),
                    "numbers": { "price_usd": px, "pct_above_spot": pct, "level_nock": c.level_nock },
                    "links": [],
                    "would_sign": Value::Null,
                    "card": "floor_health",
                    "source": "derived_metrics.floor_breach.crossing",
                }));
            }
        }
    }
    out
}

/// Every call to action of the snapshot, top first, each with its `since` carried from the
/// previous snapshot's events while the cycles are continuous. Pure.
pub(crate) fn events_block(data: &SnapshotData, prev_events: Option<&Value>, continuous: bool, now: DateTime<Utc>) -> Value {
    let prev_list: Vec<&Value> = prev_events.and_then(|v| v.as_array()).map(|a| a.iter().collect()).unwrap_or_default();
    let prev_by_id = |id: &str| prev_list.iter().copied().find(|e| e.get("id").and_then(|v| v.as_str()) == Some(id));
    let mut all: Vec<Value> = Vec::new();
    all.push(directive_event(data, prev_by_id("directive")));
    if let Some(e) = reserve_event(data) {
        all.push(e);
    }
    all.extend(fill_offer_events(data));
    if let Some(e) = found_positions_event(data) {
        all.push(e);
    }
    if let Some(e) = ladder_suggestion_event(data) {
        all.push(e);
    }
    if let Some(rungs) = data.derived_metrics.ladder_maintenance.as_ref().and_then(|m| m.get("rungs")).and_then(|r| r.as_array()) {
        for r in rungs {
            let Some(ev) = r.get("event").filter(|e| e.is_object()) else { continue };
            let mut e = ev.clone();
            let pid = r.get("position_id").and_then(|v| v.as_str()).unwrap_or("");
            let kind = e.get("kind").and_then(|v| v.as_str()).unwrap_or("ladder").to_string();
            e["id"] = json!(format!("{kind}:{pid}"));
            e["card"] = json!("ladders");
            e["source"] = json!("derived_metrics.ladder_maintenance.rungs");
            all.push(e);
        }
    }
    all.extend(quiet_events(data));
    let now_s = now.to_rfc3339();
    for e in all.iter_mut() {
        let id = e["id"].as_str().unwrap_or("").to_string();
        let carried = if continuous { prev_by_id(&id).and_then(|p| p.get("since")).and_then(|s| s.as_str()).map(str::to_string) } else { None };
        // A ladder suggestion stamps its own since (when the rule first stated it).
        let since = e.get("since").and_then(|s| s.as_str()).map(str::to_string).or(carried).unwrap_or_else(|| now_s.clone());
        e["since"] = json!(since);
        e["deep_link"] = json!(event_anchor(&id));
    }
    // Top first, in the order built; the rest after.
    let (top, quiet): (Vec<Value>, Vec<Value>) = all.into_iter().partition(|e| e["tier"].as_str() == Some("top"));
    Value::Array(top.into_iter().chain(quiet).collect())
}

/// Fill `derived_metrics.ladder_maintenance` and `derived_metrics.events` on an assembled
/// snapshot, from the previous snapshot (its blocks and its time). Pure.
pub(crate) fn attach_maintenance_and_events(
    data: &mut SnapshotData,
    profile: &OperatorProfile,
    prev: Option<&SnapshotEnvelope>,
    gap_limit_seconds: i64,
    now: DateTime<Utc>,
) {
    let prev_at = prev.map(|p| p.meta.generated_at);
    let continuous = prev_at.map_or(false, |t| now >= t && (now - t).num_seconds() <= gap_limit_seconds);
    let carry = Carry {
        prev: prev.and_then(|p| p.data.derived_metrics.ladder_maintenance.as_ref()),
        prev_at,
        gap_limit_seconds,
    };
    data.derived_metrics.ladder_maintenance = ladder_maintenance_block(data, profile.ladder_maintenance.as_ref(), &carry, now);
    let prev_events = prev.and_then(|p| p.data.derived_metrics.events.as_ref());
    data.derived_metrics.events = Some(events_block(data, prev_events, continuous, now));
}
