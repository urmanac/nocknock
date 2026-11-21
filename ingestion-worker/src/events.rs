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

/// A square-root price from a pool's `sqrt_price_x96` hex word, as a plain number. Pure.
fn sqrt_from_x96_hex(h: &str) -> Option<f64> {
    let h = h.trim().trim_start_matches("0x");
    if h.is_empty() {
        return None;
    }
    let mut v = 0f64;
    for c in h.chars() {
        v = v * 16.0 + c.to_digit(16)? as f64;
    }
    let s = v / 2f64.powi(96);
    (s.is_finite() && s > 0.0).then_some(s)
}

/// One gas-side ETH/USDC range on its own pool's curve (#151): it holds USDC and WETH, and
/// is never in the reserve gate.
pub(crate) struct EthRange {
    pub(crate) l: f64,
    pub(crate) sa: f64,
    pub(crate) sb: f64,
    /// The pool's square-root price now (raw token1 per raw token0).
    pub(crate) s_now: f64,
    /// token0 is WETH (raw price = USDC per WETH): a higher ETH price is a higher raw price.
    pub(crate) weth0: bool,
    pub(crate) usdc_dec: i32,
    pub(crate) weth_dec: i32,
}

impl EthRange {
    /// (USDC, WETH) it holds when ETH is `m` times its pool's price now. Pure.
    pub(crate) fn at_multiple(&self, m: f64) -> (f64, f64) {
        let s = if self.weth0 { self.s_now * m.sqrt() } else { self.s_now / m.sqrt() };
        let (a0, a1) = concentrated_amounts_at(self.l, s, self.sa, self.sb);
        let (weth_raw, usdc_raw) = if self.weth0 { (a0, a1) } else { (a1, a0) };
        (usdc_raw / 10f64.powi(self.usdc_dec), weth_raw / 10f64.powi(self.weth_dec))
    }
}

/// The gas-side ETH/USDC ranges of a snapshot, each on its own pool's curve. A range whose
/// pool price or liquidity is not in the snapshot is left out (and named by the caller's
/// count). Pure.
pub(crate) fn eth_ranges(data: &SnapshotData) -> (Vec<EthRange>, usize) {
    let mut out = Vec::new();
    let mut unread = 0;
    for p in &data.positions {
        let is_weth = |t: &PositionTokenInfo| t.symbol == "WETH" || t.symbol == "ETH";
        let weth0 = is_weth(&p.token0) && is_usdc_sym(&p.token1);
        let weth1 = is_usdc_sym(&p.token0) && is_weth(&p.token1);
        if !(weth0 || weth1) {
            continue;
        }
        let s_now = data
            .pool_mechanics
            .iter()
            .find(|m| m.pool_id.eq_ignore_ascii_case(&p.pool_id))
            .and_then(|m| m.sqrt_price_x96.as_deref())
            .and_then(sqrt_from_x96_hex);
        let (Some(s_now), Some(l)) = (s_now, f(&p.liquidity_amount)) else {
            unread += 1;
            continue;
        };
        let (sa, sb) = if p.full_range { (tick_sqrt_price(-887220), tick_sqrt_price(887220)) } else { (tick_sqrt_price(p.lower_tick), tick_sqrt_price(p.upper_tick)) };
        let (usdc_dec, weth_dec) = if weth0 { (p.token1.decimals, p.token0.decimals) } else { (p.token0.decimals, p.token1.decimals) };
        out.push(EthRange { l, sa, sb, s_now, weth0, usdc_dec, weth_dec });
    }
    (out, unread)
}

/// The ETH/USDC ranges against the wallet's shortfall (operator, 2026-11-21: "we can plan to
/// pull the ETH LPs some time after it would cover the $950 usdc reserve"): what they hold,
/// what wallet USDC would be if they were withdrawn now, and the ETH price at which their
/// USDC side alone covers the shortfall, with the WETH they would still hold there. A
/// measurement; they stay outside the reserve gate (#151, #162). Pure.
pub(crate) fn eth_ranges_line(data: &SnapshotData, wallet: f64, reserve: f64) -> Option<(String, serde_json::Map<String, Value>)> {
    let (ranges, unread) = eth_ranges(data);
    if ranges.is_empty() {
        return None;
    }
    let eth = data.derived_metrics.global_status.eth_price_usd.as_deref().and_then(f)?;
    let total = |m: f64| ranges.iter().map(|r| r.at_multiple(m)).fold((0.0, 0.0), |(u, w), (a, b)| (u + a, w + b));
    let (usdc_now, weth_now) = total(1.0);
    let shortfall = (reserve - wallet).max(0.0);
    let withdrawn = wallet + usdc_now;
    let mut n = serde_json::Map::new();
    n.insert("eth_ranges_usdc".into(), json!(m2(usdc_now)));
    n.insert("eth_ranges_weth".into(), json!(format!("{:.4}", weth_now)));
    n.insert("wallet_usdc_if_eth_ranges_withdrawn".into(), json!(m2(withdrawn)));
    n.insert("eth_price_usd".into(), json!(m2(eth)));
    let head = format!(
        "The gas-side ETH/USDC ranges (outside the reserve gate) hold ${} USDC and {:.4} WETH; withdrawn now, wallet USDC would be ${}.",
        m2(usdc_now),
        weth_now,
        m2(withdrawn)
    );
    let unread_note = if unread > 0 { format!(" {unread} more range(s) could not be valued from this snapshot.") } else { String::new() };
    if usdc_now >= shortfall {
        n.insert("eth_ranges_cover_shortfall".into(), json!(true));
        return Some((format!("{head} Their USDC already covers the ${} shortfall.{unread_note}", m2(shortfall)), n));
    }
    // Their USDC grows as ETH rises (they sell WETH for USDC): the least multiple at which
    // it covers the shortfall, by bisection up to every range's top.
    let hi_m = 1e6;
    if total(hi_m).0 < shortfall {
        n.insert("eth_ranges_cover_shortfall".into(), json!(false));
        return Some((
            format!("{head} Even above every range's top their USDC side (${}) would not cover the ${} shortfall.{unread_note}", m2(total(hi_m).0), m2(shortfall)),
            n,
        ));
    }
    let (mut lo, mut hi) = (1.0f64, hi_m);
    for _ in 0..200 {
        let mid = (lo * hi).sqrt();
        if total(mid).0 >= shortfall {
            hi = mid;
        } else {
            lo = mid;
        }
    }
    let (_, weth_left) = total(hi);
    let price = eth * hi;
    n.insert("eth_ranges_cover_shortfall".into(), json!(false));
    n.insert("eth_price_covering_shortfall_usd".into(), json!(m2(price)));
    n.insert("eth_covering_pct_above_now".into(), json!(format!("{:.1}", (hi - 1.0) * 100.0)));
    n.insert("eth_ranges_weth_at_covering".into(), json!(format!("{:.4}", weth_left)));
    Some((
        format!(
            "{head} Their USDC side alone would cover the ${} shortfall at ETH about ${} ({:+.1}% from ${}), each range on its own pool's curve, with about {:.4} WETH still in them there.{unread_note}",
            m2(shortfall),
            m2(price),
            (hi - 1.0) * 100.0,
            m2(eth),
            weth_left
        ),
        n,
    ))
}

/// The reserve event (#162 reserve; operator's choice, 2026-11-21: "Top when wallet <
/// $950"): top tier whenever wallet USDC alone is below `cash_reserve_usdc`, with no
/// exceptions. Its text gives the wallet / NOCK LP / ladder split of the stable balance the
/// reserve is measured on, and one measured line on the gas-side ETH/USDC ranges. Pure.
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
            "card": "reserve",
            "source": "derived_metrics.global_status.reserve_note",
        }));
    }
    let reserve = g.cash_reserve_usdc.as_deref().and_then(f)?;
    let wallet: f64 = data.wallet_balances.iter().filter(|b| b.token_symbol == "USDC").filter_map(|b| f(&b.amount)).sum();
    if wallet >= reserve {
        return None;
    }
    let stable = g.reserve_stable_usdc.as_deref().and_then(f);
    let headroom = g.reserve_headroom_usdc.as_deref().and_then(f);
    let ladder = g.reserve_ladder_usdc.as_deref().and_then(f).unwrap_or(0.0);
    let lp = stable.map(|s| (s - wallet - ladder).max(0.0));
    let mut text = format!("Wallet USDC ${} is ${} below your ${} reserve.", m2(wallet), m2(reserve - wallet), m2(reserve));
    if let (Some(st), Some(lp), Some(h)) = (stable, lp, headroom) {
        text.push_str(&format!(
            " The stable balance the reserve gate counts is ${} (wallet USDC ${} + NOCK LP USDC ${}{}), ${} {} it.",
            m2(st),
            m2(wallet),
            m2(lp),
            if ladder > 0.0 { format!(" + ladder USDC ${}", m2(ladder)) } else { String::new() },
            m2(h.abs()),
            if h >= 0.0 { "above" } else { "below" }
        ));
    }
    let mut numbers = serde_json::Map::new();
    numbers.insert("reserve_usdc".into(), json!(m2(reserve)));
    numbers.insert("wallet_usdc".into(), json!(m2(wallet)));
    numbers.insert("wallet_shortfall_usdc".into(), json!(m2(reserve - wallet)));
    numbers.insert("stable_usdc".into(), json!(stable.map(m2)));
    numbers.insert("nock_lp_usdc".into(), json!(lp.map(m2)));
    numbers.insert("ladder_usdc".into(), json!(m2(ladder)));
    numbers.insert("stable_headroom_usdc".into(), json!(headroom.map(m2)));
    if let Some((line, n)) = eth_ranges_line(data, wallet, reserve) {
        text.push(' ');
        text.push_str(&line);
        numbers.extend(n);
    }
    Some(json!({
        "id": "reserve_below",
        "kind": "reserve_below",
        "tier": "top",
        "title": "WALLET USDC BELOW THE STANDING RESERVE",
        "what_changed": "Wallet USDC alone is below your standing reserve.",
        "text": text,
        "numbers": Value::Object(numbers),
        "links": [],
        "would_sign": Value::Null,
        "card": "reserve",
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
            // A rung's own event, and the event of a setting that cannot work for it.
            for key in ["event", "setting_event"] {
                let Some(ev) = r.get(key).filter(|e| e.is_object()) else { continue };
                let mut e = ev.clone();
                let pid = r.get("position_id").and_then(|v| v.as_str()).unwrap_or("");
                let kind = e.get("kind").and_then(|v| v.as_str()).unwrap_or("ladder").to_string();
                e["id"] = json!(format!("{kind}:{pid}"));
                e["card"] = json!("ladders");
                e["source"] = json!("derived_metrics.ladder_maintenance.rungs");
                all.push(e);
            }
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
