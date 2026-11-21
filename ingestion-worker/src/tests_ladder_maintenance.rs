//! Ladder maintenance and the calls-to-action events (issue #169), on the operator's real
//! bottle as chudmax read it on 2026-11-21T16:10Z (testdata/ladder_maintenance/
//! live_bottle_2026-11-21.json): cap #3244699 (273300..273600) out of range above after it
//! sold its NOCK, near #3244700 (273600..275400) and deep #3244701 (275400..276300) holding
//! only USDC, pool tick 273280.
use super::*;
use serde_json::Value;

const NEAR: &str = "uniswap-v4:base:3244700";
const DEEP: &str = "uniswap-v4:base:3244701";
const CAP: &str = "uniswap-v4:base:3244699";
const POOL: &str = "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13";
const GAP: i64 = 390; // sync window 300 s + the /health grace

fn t(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

pub(crate) fn live_bottle() -> SnapshotEnvelope {
    serde_json::from_str(include_str!("../testdata/ladder_maintenance/live_bottle_2026-11-21.json")).expect("the live bottle parses")
}

/// The operator's restraint settings for a test. Examples, never defaults.
pub(crate) fn example_cfg() -> LadderMaintenanceConfig {
    LadderMaintenanceConfig {
        min_hours_out_of_range: Some(24.0),
        cooldown_hours: Some(24.0),
        hysteresis_pct: Some(2.0),
        max_gas_payback_days: Some(30.0),
        deep_pull_fill_fraction: Some(0.5),
        deep_pull_recovery_pct: Some(10.0),
    }
}

fn profile(cfg: Option<LadderMaintenanceConfig>) -> OperatorProfile {
    OperatorProfile { ladder_maintenance: cfg, ..OperatorProfile::default() }
}

/// One cycle: the snapshot `env` read at `now`, with the previous cycle's envelope.
pub(crate) fn cycle(env: &SnapshotEnvelope, cfg: Option<LadderMaintenanceConfig>, prev: Option<&SnapshotEnvelope>, now: DateTime<Utc>) -> SnapshotEnvelope {
    let mut out = env.clone();
    out.meta.generated_at = now;
    out.data.derived_metrics.ladder_maintenance = None;
    out.data.derived_metrics.events = None;
    attach_maintenance_and_events(&mut out.data, &profile(cfg), prev, GAP, now);
    out
}

fn rung<'a>(env: &'a SnapshotEnvelope, id: &str) -> &'a Value {
    env.data.derived_metrics.ladder_maintenance.as_ref().unwrap()["rungs"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["position_id"] == id)
        .unwrap()
}

fn events(env: &SnapshotEnvelope) -> Vec<Value> {
    env.data.derived_metrics.events.as_ref().unwrap().as_array().unwrap().clone()
}

fn event<'a>(evs: &'a [Value], id: &str) -> Option<&'a Value> {
    evs.iter().find(|e| e["id"] == id)
}

fn top_ids(env: &SnapshotEnvelope) -> Vec<String> {
    events(env).iter().filter(|e| e["tier"] == "top").map(|e| e["id"].as_str().unwrap().to_string()).collect()
}

/// The tick of a USD price in this USDC(6)/NOCK(16) pool (NOCK token1).
fn tick_of(price: f64) -> i32 {
    (-(price / 1e10).ln() / 1.0001f64.ln()).floor() as i32
}

/// Move the pool (and every rung's pool price) to `price`.
fn at_price(env: &mut SnapshotEnvelope, price: f64) {
    for m in env.data.pool_mechanics.iter_mut().filter(|m| m.pool_id == POOL) {
        m.tick_current = Some(tick_of(price));
    }
    for r in env.data.ladders.iter_mut() {
        r.pool_price_usd = Some(format!("{price:.8}"));
    }
}

fn reading<'a>(env: &'a mut SnapshotEnvelope, id: &str) -> &'a mut LadderReading {
    env.data.ladders.iter_mut().find(|r| r.position_id == id).unwrap()
}

fn near_top(env: &SnapshotEnvelope) -> f64 {
    env.data.ladders.iter().find(|r| r.position_id == NEAR).unwrap().high_price_usd.as_ref().unwrap().parse().unwrap()
}

/// Every text a rung or an event states: no steering words.
fn assert_measured_words(env: &SnapshotEnvelope) {
    let m = env.data.derived_metrics.ladder_maintenance.as_ref().unwrap();
    let mut texts: Vec<String> = m["rungs"].as_array().unwrap().iter().map(|r| r["text"].as_str().unwrap().to_string()).collect();
    texts.extend(events(env).iter().filter(|e| e["kind"] != "directive").map(|e| e["text"].as_str().unwrap_or("").to_string()));
    for text in texts {
        let low = text.to_lowercase();
        for word in ["should", "recommend", " must ", "you need to"] {
            assert!(!low.contains(word), "{word:?} in {text}");
        }
    }
}

#[test]
fn live_bottle_with_settings_unset_states_every_rule_off_and_suggests_nothing() {
    let env = live_bottle();
    let now = env.meta.generated_at;
    let c = cycle(&env, None, None, now);
    let m = c.data.derived_metrics.ladder_maintenance.as_ref().unwrap();
    assert_eq!(m["rules"]["near_move"]["status"], "off");
    assert_eq!(m["rules"]["deep_pull"]["status"], "off");
    assert_eq!(m["rules"]["deep_bottom_report"]["status"], "on");
    // The rule names every setting that turns it on, and no value is filled in.
    let near_text = m["rules"]["near_move"]["text"].as_str().unwrap();
    for k in NEAR_KEYS {
        assert!(near_text.contains(&format!("operator_profile.ladder_maintenance.{k}")), "{near_text}");
    }
    for k in ALL_KEYS {
        assert!(m["settings"][k].is_null(), "{k} has no default");
    }
    // near: out of range above, stated, rule off.
    let near = rung(&c, NEAR);
    assert_eq!(near["status"], "off");
    assert!(near["text"].as_str().unwrap().contains("out of range above since 2026-11-21 16:10 UTC"), "{near}");
    assert!(near["move"].is_null() && near["event"].is_null());
    // deep: waits for a dip, never moved.
    assert_eq!(rung(&c, DEEP)["status"], "waiting_for_dip");
    assert!(rung(&c, DEEP)["text"].as_str().unwrap().contains("never moved up or chased"));
    // cap: sold its NOCK by design, stated quietly.
    let cap = rung(&c, CAP);
    assert_eq!(cap["status"], "sold");
    assert!(cap["text"].as_str().unwrap().contains(CAP_NO_REFILL));
    let evs = events(&c);
    let cap_ev = event(&evs, &format!("ladder_cap_sold:{CAP}")).unwrap();
    assert_eq!(cap_ev["tier"], "quiet");
    assert!(cap_ev["would_sign"].is_null());
    // At the top of the page only the reserve: wallet USDC $778.77 is below the $950 reserve
    // (the operator's rule, 2026-11-21). HOLD is quiet, the suggestion card is not shown, and
    // no ladder rule is on.
    assert_eq!(top_ids(&c), vec!["reserve_below".to_string()]);
    assert_eq!(event(&evs, "directive").unwrap()["tier"], "quiet");
    assert_measured_words(&c);
}

#[test]
fn near_rung_waits_on_the_first_cycle_then_is_stated_after_every_gate() {
    let env = live_bottle();
    let t0 = env.meta.generated_at; // 16:10:35, 0.7 h after the near rung's deposit
    let first = cycle(&env, Some(example_cfg()), None, t0);
    let near = rung(&first, NEAR);
    assert_eq!(near["status"], "waiting");
    assert_eq!(near["clock"]["above_since"], t0.to_rfc3339());
    let failing: Vec<&str> = near["gates"].as_array().unwrap().iter().filter(|g| g["passes"] != true).map(|g| g["gate"].as_str().unwrap()).collect();
    assert_eq!(failing, vec!["min_hours_out_of_range", "cooldown_hours"]);
    assert!(near["text"].as_str().unwrap().contains("Not stated yet: out of range above: 0.0 h of your 24 h"), "{}", near["text"]);
    // The move keeps the rung's width: 1,800 ticks from the usable tick at or below spot.
    assert_eq!(near["move"]["new_tick_lower"], 273300);
    assert_eq!(near["move"]["new_tick_upper"], 275100);
    assert_eq!(near["move"]["width_ticks"], 1800);
    assert_eq!(near["move"]["usdc"], "111.09");

    // Watched every 5 minutes for 25 hours: every gate passes.
    let mut prev = first;
    let mut now = t0;
    for _ in 0..300 {
        now += chrono::Duration::minutes(5);
        prev = cycle(&env, Some(example_cfg()), Some(&prev), now);
    }
    let near = rung(&prev, NEAR);
    assert_eq!(near["status"], "suggested", "{}", near["text"]);
    assert_eq!(near["clock"]["above_since"], t0.to_rfc3339());
    let ev = event(&events(&prev), &format!("ladder_near_move:{NEAR}")).unwrap().clone();
    assert_eq!(ev["tier"], "top");
    assert_eq!(ev["title"], "LADDER: MOVE THE NEAR RUNG");
    assert_eq!(ev["deep_link"], "#event-ladder-near-move-uniswap-v4-base-3244700");
    assert_eq!(ev["numbers"]["width_ticks"], "1800");
    assert_eq!(ev["numbers"]["new_tick_lower"], "273300");
    assert_eq!(ev["numbers"]["usdc"], "111.09");
    // Gas: two transactions at the cycle's per-transaction estimate ($0.0035).
    assert_eq!(ev["numbers"]["gas_usd"], "0.0070");
    // Fees per day in range: the pool's measured rate times the moved range's liquidity.
    let l: f64 = near["move"]["liquidity_raw"].as_str().unwrap().parse().unwrap();
    let per_day: f64 = ev["numbers"]["fees_per_day_in_range_usd"].as_str().unwrap().parse().unwrap();
    assert!((per_day - 5.850575e-15 * l).abs() < 1e-4, "{per_day} vs {}", 5.850575e-15 * l);
    assert!(ev["would_sign"].as_str().unwrap().starts_with("Two transactions in your wallet: remove all of Near rung #3244700's liquidity"));
    let links = ev["links"].as_array().unwrap();
    assert_eq!(links[0]["url"], "https://app.uniswap.org/positions/v4/base/3244700");
    assert!(ev["text"].as_str().unwrap().contains("Whether and when is your decision."));
    assert_eq!(ev["since"], (t0 + chrono::Duration::hours(24)).to_rfc3339(), "since: when every gate first passed (24 h out of range)");
    assert!(top_ids(&prev).contains(&format!("ladder_near_move:{NEAR}")));
    assert_measured_words(&prev);
}

/// A rung out of range above for 25 h, its suggestion state as given.
fn near_after_25h(price_pct_above_top: f64, suggested: bool, cfg: LadderMaintenanceConfig) -> SnapshotEnvelope {
    let mut env = live_bottle();
    let top = near_top(&env);
    at_price(&mut env, top * (1.0 + price_pct_above_top / 100.0));
    let now = t("2026-11-22T17:30:00Z");
    let mut prev = env.clone();
    prev.meta.generated_at = now - chrono::Duration::minutes(5);
    prev.data.derived_metrics.ladder_maintenance = Some(json!({ "rungs": [{
        "position_id": NEAR,
        "clock": { "above_since": "2026-11-21T16:10:35+00:00", "suggested_since": if suggested { json!("2026-11-22T17:00:00+00:00") } else { Value::Null }, "last_cleared_at": null, "episode": null },
    }]}));
    cycle(&env, Some(cfg), Some(&prev), now)
}

#[test]
fn hysteresis_at_the_range_edge_never_churns() {
    // 4% above the top, hysteresis 5%: not stated (the move would change the range).
    let cfg = LadderMaintenanceConfig { hysteresis_pct: Some(5.0), ..example_cfg() };
    let c = near_after_25h(4.0, false, cfg.clone());
    let near = rung(&c, NEAR);
    assert_eq!(near["status"], "waiting", "{}", near["text"]);
    let hyst = near["gates"].as_array().unwrap().iter().find(|g| g["gate"] == "hysteresis_pct").unwrap();
    assert_eq!(hyst["passes"], false);
    // The same price with the move already standing: it stays (no churn on a dip toward the edge).
    let c = near_after_25h(4.0, true, cfg.clone());
    assert_eq!(rung(&c, NEAR)["status"], "suggested");
    assert_eq!(rung(&c, NEAR)["clock"]["suggested_since"], "2026-11-22T17:00:00+00:00");
    // A gate that flips while the move stands (the gas test, here) does not withdraw it.
    let c = near_after_25h(4.0, true, LadderMaintenanceConfig { max_gas_payback_days: Some(0.0001), ..cfg.clone() });
    assert_eq!(rung(&c, NEAR)["status"], "suggested");
    // Within one tick spacing of the top: a move would not change the range, whatever the settings.
    let c = near_after_25h(0.5, false, LadderMaintenanceConfig { hysteresis_pct: Some(0.0), ..example_cfg() });
    assert_eq!(rung(&c, NEAR)["status"], "waiting");
    assert!(rung(&c, NEAR)["text"].as_str().unwrap().contains("within one tick spacing of its top"));
    // Back in range: the standing move clears and the cooldown starts there.
    let mut env = live_bottle();
    reading(&mut env, NEAR).state = "in_range".to_string();
    let standing = near_after_25h(4.0, true, cfg.clone());
    let now = standing.meta.generated_at + chrono::Duration::minutes(5);
    let back = cycle(&env, Some(cfg.clone()), Some(&standing), now);
    let near = rung(&back, NEAR);
    assert_eq!(near["status"], "trading");
    assert!(near["clock"]["suggested_since"].is_null());
    assert_eq!(near["clock"]["last_cleared_at"], now.to_rfc3339());
    assert!(near["clock"]["above_since"].is_null());
}

#[test]
fn the_cooldown_counts_from_the_deposit_and_from_the_last_suggestion() {
    let cfg = example_cfg();
    // 25 h out of range but the rung was deposited 2026-11-21 15:30 and the cooldown is 48 h.
    let c = near_after_25h(4.0, false, LadderMaintenanceConfig { cooldown_hours: Some(48.0), ..cfg.clone() });
    let near = rung(&c, NEAR);
    assert_eq!(near["status"], "waiting");
    let cool = near["gates"].as_array().unwrap().iter().find(|g| g["gate"] == "cooldown_hours").unwrap();
    assert_eq!(cool["passes"], false);
    assert!(cool["text"].as_str().unwrap().contains("since its deposit of your 48 h"), "{cool}");
    // A suggestion that cleared an hour ago: the cooldown counts from there.
    let mut env = live_bottle();
    let top = near_top(&env);
    at_price(&mut env, top * 1.04);
    let now = t("2026-11-23T12:00:00Z");
    let mut prev = env.clone();
    prev.meta.generated_at = now - chrono::Duration::minutes(5);
    prev.data.derived_metrics.ladder_maintenance = Some(json!({ "rungs": [{
        "position_id": NEAR,
        "clock": { "above_since": "2026-11-22T06:00:00+00:00", "suggested_since": null, "last_cleared_at": "2026-11-23T11:00:00+00:00", "episode": null },
    }]}));
    let c = cycle(&env, Some(cfg.clone()), Some(&prev), now);
    let cool = rung(&c, NEAR)["gates"].as_array().unwrap().iter().find(|g| g["gate"] == "cooldown_hours").unwrap().clone();
    assert_eq!(cool["passes"], false);
    assert!(cool["text"].as_str().unwrap().contains("1.0 h since the last suggestion cleared of your 24 h"), "{cool}");
    // No deposit time read: the cooldown is unknown and nothing is stated.
    let mut env2 = env.clone();
    reading(&mut env2, NEAR).since_deposit.deposited_at = None;
    let c = cycle(&env2, Some(cfg), Some(&prev), now);
    let cool = rung(&c, NEAR)["gates"].as_array().unwrap().iter().find(|g| g["gate"] == "cooldown_hours").unwrap().clone();
    assert!(cool["passes"].is_null());
    assert_eq!(rung(&c, NEAR)["status"], "waiting");
}

#[test]
fn the_gas_test_passes_fails_and_is_unknown_without_a_fee_rate() {
    let pass = near_after_25h(4.0, false, example_cfg());
    assert_eq!(rung(&pass, NEAR)["status"], "suggested");
    let payback: f64 = rung(&pass, NEAR)["move"]["payback_days_in_range"].as_str().unwrap().parse().unwrap();
    assert!(payback > 0.0001 && payback < 30.0, "{payback} {}", rung(&pass, NEAR)["move"]);
    // A payback limit below the measured payback fails the test.
    let fail = near_after_25h(4.0, false, LadderMaintenanceConfig { max_gas_payback_days: Some(0.0001), ..example_cfg() });
    let near = rung(&fail, NEAR);
    assert_eq!(near["status"], "waiting");
    assert!(near["text"].as_str().unwrap().contains("repaid in"), "{}", near["text"]);
    // No fee measurement in the snapshot: unknown, never "free", never stated.
    let mut env = live_bottle();
    let top = near_top(&env);
    at_price(&mut env, top * 1.04);
    env.data.derived_metrics.fee_yield = None;
    let base = near_after_25h(4.0, false, example_cfg());
    let mut prev = base.clone();
    prev.meta.generated_at = base.meta.generated_at - chrono::Duration::minutes(5);
    // Out of range for 25 h, with no move standing yet.
    for r in prev.data.derived_metrics.ladder_maintenance.as_mut().unwrap()["rungs"].as_array_mut().unwrap() {
        r["clock"]["suggested_since"] = Value::Null;
    }
    let c = cycle(&env, Some(example_cfg()), Some(&prev), base.meta.generated_at);
    let near = rung(&c, NEAR);
    let gas = near["gates"].as_array().unwrap().iter().find(|g| g["gate"] == "max_gas_payback_days").unwrap();
    assert!(gas["passes"].is_null());
    assert!(gas["text"].as_str().unwrap().contains("unknown, no fee measurement is in this snapshot"));
    assert!(near["event"].is_null());
}

#[test]
fn a_gap_restarts_the_out_of_range_clock_and_the_standing_move() {
    let standing = near_after_25h(4.0, true, example_cfg());
    assert_eq!(rung(&standing, NEAR)["status"], "suggested");
    let mut env = live_bottle();
    let top = near_top(&env);
    at_price(&mut env, top * 1.04);
    // The worker was down for an hour: the next cycle restarts the clocks.
    let now = standing.meta.generated_at + chrono::Duration::hours(1);
    let c = cycle(&env, Some(example_cfg()), Some(&standing), now);
    let near = rung(&c, NEAR);
    assert_eq!(near["clock"]["above_since"], now.to_rfc3339());
    assert_eq!(near["status"], "waiting");
    assert_eq!(c.data.derived_metrics.ladder_maintenance.as_ref().unwrap()["watched_since_previous"], false);
}

/// Every rung below its range at `price`: each holds only NOCK, its USDC all converted.
fn all_below(env: &mut SnapshotEnvelope, price: f64) {
    at_price(env, price);
    for r in env.data.ladders.iter_mut() {
        r.state = "holds_only_nock".to_string();
        r.nock_now = r.vs_top.nock_at_bottom.clone().unwrap();
        r.usdc_now = "0.00".to_string();
        r.vs_top.converted_pct = Some("100.00".to_string());
    }
}

/// The deep rung as the chain would show it at `price`: on its own curve between its bottom
/// and top (from its reading's amounts at the edges), never a state the chain cannot give.
fn deep_at(env: &mut SnapshotEnvelope, price: f64) -> (f64, f64) {
    at_price(env, price);
    let r = reading(env, DEEP);
    let a: f64 = r.low_price_usd.as_ref().unwrap().parse().unwrap();
    let b: f64 = r.high_price_usd.as_ref().unwrap().parse().unwrap();
    let u_top: f64 = r.vs_top.usdc_at_top.as_ref().unwrap().parse().unwrap();
    let n_bottom: f64 = r.vs_top.nock_at_bottom.as_ref().unwrap().parse().unwrap();
    let (state, usdc, nock) = if price >= b {
        ("holds_only_usdc", u_top, 0.0)
    } else if price <= a {
        ("holds_only_nock", 0.0, n_bottom)
    } else {
        let usdc = u_top * (price.sqrt() - a.sqrt()) / (b.sqrt() - a.sqrt());
        let nock = n_bottom * (1.0 / price.sqrt() - 1.0 / b.sqrt()) / (1.0 / a.sqrt() - 1.0 / b.sqrt());
        ("in_range", usdc, nock)
    };
    r.state = state.to_string();
    r.usdc_now = format!("{usdc:.2}");
    r.nock_now = format!("{nock:.4}");
    r.vs_top.converted_pct = Some(format!("{:.2}", (1.0 - usdc / u_top) * 100.0));
    (usdc, nock)
}

#[test]
fn deep_rung_fully_converted_is_reported_and_no_lower_ladder_is_suggested() {
    let mut env = live_bottle();
    all_below(&mut env, 0.0099);
    for cfg in [None, Some(example_cfg())] {
        let c = cycle(&env, cfg, None, env.meta.generated_at);
        let deep = rung(&c, DEEP);
        assert_eq!(deep["status"], "fully_converted", "{}", deep["text"]);
        let text = deep["text"].as_str().unwrap();
        assert!(text.contains(NO_LOWER_LADDER), "{text}");
        assert!(text.contains("an average of $0.01049"), "{text}");
        let evs = events(&c);
        let ev = event(&evs, &format!("ladder_deep_converted:{DEEP}")).unwrap();
        assert_eq!(ev["tier"], "top");
        assert!(ev["would_sign"].is_null(), "a report: nothing to sign");
        // No event anywhere offers a lower range.
        for e in &evs {
            let t = e["text"].as_str().unwrap_or("").to_lowercase();
            assert!(!t.contains("lower ladder") || t.contains("does not suggest a lower ladder"), "{t}");
        }
        assert!(deep["move"].is_null());
        assert!(text.contains("holds 60,038 NOCK"), "{text}");
        // The near rung below its range holds NOCK for the scalp; nothing is stated for it.
        assert_eq!(rung(&c, NEAR)["status"], "holds_nock");
        assert!(rung(&c, NEAR)["event"].is_null());
    }
}

/// The review's walk on the live rung (about 9.4% wide) with the operator's 10% recovery:
/// a dump to $0.0101, then a rise through the rung. The pull can never be stated, because the
/// recovery point is above the rung's top; the rule says so at the top of the page.
#[test]
fn a_recovery_at_or_above_the_rungs_width_is_reported_as_unable_to_fire() {
    let cfg = example_cfg();
    let mut env = live_bottle();
    // Before any dip, measured from its bottom: 0.01002405 * 1.10 is above its top 0.01096801.
    let live = cycle(&env, Some(cfg.clone()), None, env.meta.generated_at);
    let deep = rung(&live, DEEP);
    assert_eq!(deep["status"], "rule_cannot_fire", "{}", deep["text"]);
    let ev = event(&events(&live), &format!("ladder_rule_cannot_fire:{DEEP}")).unwrap().clone();
    assert_eq!(ev["tier"], "top");
    assert_eq!(ev["numbers"]["rung_width_pct"], "9.42");
    let text = ev["text"].as_str().unwrap();
    assert!(text.contains("operator_profile.ladder_maintenance.deep_pull_recovery_pct of 10%"), "{text}");
    assert!(text.contains("The rung is 9.42% wide"), "{text}");
    assert!(text.contains("A recovery below 9.42% would let the pull be stated before that."), "{text}");
    // The walk: never stated, the rule keeps saying why, and at the top the NOCK is sold back.
    let t0 = t("2026-11-25T10:00:00Z");
    let mut prev: Option<SnapshotEnvelope> = None;
    for (i, price) in [0.0101, 0.0103, 0.0105, 0.0107, 0.0109, 0.01099, 0.0112].iter().enumerate() {
        deep_at(&mut env, *price);
        let c = cycle(&env, Some(cfg.clone()), prev.as_ref(), t0 + chrono::Duration::minutes(5 * i as i64));
        let d = rung(&c, DEEP);
        assert_ne!(d["status"], "suggested", "at {price}");
        assert_eq!(d["status"], "rule_cannot_fire", "at {price}: {}", d["text"]);
        assert!(event(&events(&c), &format!("ladder_rule_cannot_fire:{DEEP}")).is_some());
        prev = Some(c);
    }
    assert_eq!(rung(prev.as_ref().unwrap(), DEEP)["state"], "holds_only_usdc");
}

/// With a recovery under the rung's width the pull is stated on the way back up, and it
/// says how much of the NOCK bought has been sold back since the low.
#[test]
fn deep_rung_pull_after_it_bought_low_and_the_price_recovered() {
    let cfg = LadderMaintenanceConfig { deep_pull_recovery_pct: Some(5.0), ..example_cfg() };
    let mut env = live_bottle();
    let t0 = t("2026-11-25T10:00:00Z");
    let at = |i: i64| t0 + chrono::Duration::minutes(5 * i);
    // The dump to $0.0101: in range, most of its USDC converted.
    let (_, nock_low) = deep_at(&mut env, 0.0101);
    let low = cycle(&env, Some(cfg.clone()), None, at(0));
    let deep = rung(&low, DEEP);
    assert_eq!(deep["status"], "holding", "{}", deep["text"]);
    assert_eq!(deep["clock"]["episode"]["low_price_usd"], "0.01010000");
    assert!(deep["setting_event"].is_null(), "a 5% recovery fits inside the 9.42% rung");
    // +1.98% and +3.96%: still holding.
    deep_at(&mut env, 0.0103);
    let a = cycle(&env, Some(cfg.clone()), Some(&low), at(1));
    assert_eq!(rung(&a, DEEP)["status"], "holding");
    deep_at(&mut env, 0.0105);
    let b = cycle(&env, Some(cfg.clone()), Some(&a), at(2));
    assert_eq!(rung(&b, DEEP)["status"], "holding");
    // +5.94%: past the recovery point $0.010605, still inside the rung (top $0.01097).
    let (_, nock_up) = deep_at(&mut env, 0.0107);
    let up = cycle(&env, Some(cfg.clone()), Some(&b), at(3));
    let deep = rung(&up, DEEP);
    assert_eq!(deep["state"], "in_range");
    assert_eq!(deep["status"], "suggested", "{}", deep["text"]);
    let sold: f64 = deep["pull"]["nock_sold_back_since_low"].as_str().unwrap().parse().unwrap();
    assert!((sold - (nock_low - nock_up)).abs() < 0.01, "{sold} vs {}", nock_low - nock_up);
    let ev = event(&events(&up), &format!("ladder_deep_pull:{DEEP}")).unwrap().clone();
    assert_eq!(ev["tier"], "top");
    assert_eq!(ev["title"], "LADDER: PULL THE DEEP RUNG");
    let text = ev["text"].as_str().unwrap();
    assert!(text.contains("Since then it has sold back"), "{text}");
    assert!(text.contains("removing it now keeps the"), "{text}");
    assert!(!text.contains("keeps the NOCK it bought"), "{text}");
    assert!(ev["would_sign"].as_str().unwrap().starts_with("One transaction in your wallet: remove all of Deep rung #3244701's liquidity"));
    // A dip inside the 2% hysteresis band keeps it (no churn).
    deep_at(&mut env, 0.0104);
    let dip = cycle(&env, Some(cfg.clone()), Some(&up), at(4));
    assert_eq!(rung(&dip, DEEP)["status"], "suggested");
    // A new low below the bottom clears it: fully converted, reported.
    deep_at(&mut env, 0.0100);
    let new_low = cycle(&env, Some(cfg.clone()), Some(&dip), at(5));
    let deep = rung(&new_low, DEEP);
    assert_eq!(deep["status"], "fully_converted", "{}", deep["text"]);
    assert_eq!(deep["clock"]["episode"]["low_price_usd"], "0.01000000");
    assert!(deep["clock"]["last_cleared_at"].is_string());
    // Below the fill setting the pull is never stated, whatever the recovery.
    // (A 2% recovery, so the rule can fire inside the rung: about 38% converted at $0.0106.)
    let cfg2 = LadderMaintenanceConfig { deep_pull_recovery_pct: Some(2.0), ..cfg.clone() };
    let mut env2 = live_bottle();
    deep_at(&mut env2, 0.0106);
    let a = cycle(&env2, Some(cfg2.clone()), None, at(0));
    deep_at(&mut env2, 0.0109);
    let b = cycle(&env2, Some(cfg2), Some(&a), at(1));
    assert_eq!(rung(&b, DEEP)["status"], "holding", "{}", rung(&b, DEEP)["text"]);
    // With its settings unset the pull is off and says which settings turn it on.
    deep_at(&mut env, 0.0107);
    let off = cycle(&env, None, Some(&dip), at(5));
    assert_eq!(rung(&off, DEEP)["status"], "off");
    assert!(rung(&off, DEEP)["text"].as_str().unwrap().contains("operator_profile.ladder_maintenance.deep_pull_fill_fraction"));
}

#[test]
fn an_invalid_setting_turns_its_rule_off_and_says_so() {
    let env = live_bottle();
    let cfg = LadderMaintenanceConfig { deep_pull_fill_fraction: Some(1.5), min_hours_out_of_range: Some(-1.0), ..example_cfg() };
    let c = cycle(&env, Some(cfg), None, env.meta.generated_at);
    let m = c.data.derived_metrics.ladder_maintenance.as_ref().unwrap();
    assert_eq!(m["rules"]["deep_pull"]["status"], "invalid");
    assert_eq!(m["rules"]["near_move"]["status"], "invalid");
    assert!(m["rules"]["deep_pull"]["text"].as_str().unwrap().contains("operator_profile.ladder_maintenance.deep_pull_fill_fraction is 1.5: valid values are above 0 and at most 1"));
    assert_eq!(rung(&c, NEAR)["status"], "invalid");
}

#[test]
fn no_ladder_no_maintenance_block() {
    let mut env = live_bottle();
    env.data.ladders.clear();
    let c = cycle(&env, Some(example_cfg()), None, env.meta.generated_at);
    assert!(c.data.derived_metrics.ladder_maintenance.is_none());
    assert!(c.data.derived_metrics.events.is_some());
}

#[test]
fn every_audited_call_to_action_has_its_tier() {
    // The directive titles, by their first segment.
    for (title, tier) in [
        ("DEFENSIVE LOCK (Floor Recovery)", "top"),
        ("DEFENSIVE LOCK (Floor Recovery) · DCA TRANCHE DUE (2/5)", "top"),
        ("DCA TRANCHE DUE (2/5)", "top"),
        ("DCA PLAN INVALID", "top"),
        ("RANGE EXIT FLASHPOINT (Above Range)", "quiet"),
        ("BATCH HARVEST SHOT (Execute Claim)", "quiet"),
        ("REBUILD LP TRANCHES (No Net Cash Change)", "quiet"),
        ("REBUILD LP TRANCHES (No Net Cash Change) · DIP ZONE (No Buy Plan Configured)", "quiet"),
        ("OVEREXTENDED (Daily RSI ≥ 70, No Buy Advised)", "quiet"),
        ("TAX RESERVE SHORT (No Buy Advised)", "quiet"),
        ("BUILD FROM ZERO (No Buy Plan Configured)", "quiet"),
        ("DIP ZONE (No Buy Plan Configured)", "quiet"),
        ("HOLD & ACCRETE (Dual Growth Engine Active)", "quiet"),
        ("NOTHING TO TRACK YET (Empty Wallet)", "quiet"),
        ("DCA ON SCHEDULE (3/5 next)", "quiet"),
        ("DCA PLAN PENDING", "quiet"),
        ("DCA PAUSED (2/5)", "quiet"),
        ("DCA BLOCKED (2/5)", "quiet"),
        ("DCA BLOCKED (ledger unreadable)", "quiet"),
    ] {
        assert_eq!(directive_tier(title), tier, "{title}");
    }
    // The live snapshot's other calls to action.
    let env = live_bottle();
    let c = cycle(&env, None, None, env.meta.generated_at);
    let evs = events(&c);
    let harvest = event(&evs, "harvest").unwrap();
    assert_eq!(harvest["tier"], "quiet");
    assert_eq!(harvest["numbers"]["claimable_usd"], "3.02");
    assert!(harvest["text"].as_str().unwrap().contains("the BATCH HARVEST directive and its push still follow the $5.00 rule"));
    assert_eq!(event(&evs, "aero_rewards").unwrap()["tier"], "quiet");
    assert_eq!(event(&evs, "floor_crossing").unwrap()["tier"], "quiet");
    // Kinds with their own top-of-page card say which.
    let mut env2 = live_bottle();
    env2.data.derived_metrics.ladder_suggestion.as_mut().unwrap()["shown"] = json!(true);
    env2.data.derived_metrics.position_detection.as_mut().unwrap()["found_not_counted"]["count"] = json!(1);
    let c2 = cycle(&env2, None, None, env2.meta.generated_at);
    let evs2 = events(&c2);
    assert_eq!(event(&evs2, LADDER_SUGGEST_ID).unwrap()["card"], "ladder_suggestion");
    assert_eq!(event(&evs2, "found_positions").unwrap()["tier"], "top");
    assert_eq!(event(&evs2, "found_positions").unwrap()["card"], "found_positions");
    // Top entries come first.
    let tiers: Vec<&str> = evs2.iter().map(|e| e["tier"].as_str().unwrap()).collect();
    let first_quiet = tiers.iter().position(|t| *t == "quiet").unwrap();
    assert!(tiers[first_quiet..].iter().all(|t| *t == "quiet"), "{tiers:?}");
    // Every event carries the push-ready shape.
    for e in &evs2 {
        for k in ["id", "kind", "tier", "title", "what_changed", "text", "numbers", "since", "deep_link", "links", "would_sign", "card"] {
            assert!(e.get(k).is_some(), "{k} missing in {e}");
        }
    }
}

/// The operator's rule (2026-11-21, "Top when wallet < $950"): top whenever wallet USDC alone
/// is below the reserve, with the split and the ETH/USDC ranges' line. Numbers recomputed
/// independently from the live fixture (each range's liquidity on its own pool's curve).
#[test]
fn wallet_usdc_below_the_reserve_is_top_with_the_split_and_the_eth_ranges_line() {
    let live = live_bottle();
    let c = cycle(&live, None, None, live.meta.generated_at);
    let evs = events(&c);
    let r = event(&evs, "reserve_below").unwrap();
    assert_eq!(r["tier"], "top", "{r}");
    let text = r["text"].as_str().unwrap();
    assert!(text.starts_with("Wallet USDC $778.77 is $171.23 below your $950.00 reserve."), "{text}");
    assert!(text.contains("(wallet USDC $778.77 + NOCK LP USDC $233.86 + ladder USDC $768.24), $830.87 above it."), "{text}");
    let n = &r["numbers"];
    assert_eq!(n["wallet_shortfall_usdc"], "171.23");
    assert_eq!(n["eth_ranges_usdc"], "99.09");
    assert_eq!(n["eth_ranges_weth"], "0.2345");
    assert_eq!(n["wallet_usdc_if_eth_ranges_withdrawn"], "877.85");
    assert_eq!(n["eth_price_usd"], "1677.54");
    let covering: f64 = n["eth_price_covering_shortfall_usd"].as_str().unwrap().parse().unwrap();
    assert!((covering - 1830.74).abs() < 0.02, "{covering}");
    assert_eq!(n["eth_covering_pct_above_now"], "9.1");
    assert_eq!(n["eth_ranges_weth_at_covering"], "0.1933");
    assert!(text.contains("withdrawn now, wallet USDC would be $877.85"), "{text}");
    assert!(text.contains("would cover the $171.23 shortfall at ETH about $1830.7"), "{text}");
    assert!(!text.to_lowercase().contains("pull now") && !text.to_lowercase().contains("should"), "{text}");
    // When their USDC already covers the shortfall, the line says so.
    let mut env = live_bottle();
    env.data.derived_metrics.global_status.cash_reserve_usdc = Some("850.00".to_string());
    let c = cycle(&env, None, None, env.meta.generated_at);
    let evs = events(&c);
    let r = event(&evs, "reserve_below").unwrap();
    assert_eq!(r["tier"], "top");
    assert!(r["text"].as_str().unwrap().contains("Their USDC already covers the $71.23 shortfall."), "{}", r["text"]);
    // Wallet USDC at or above the reserve: no entry.
    env.data.derived_metrics.global_status.cash_reserve_usdc = Some("700.00".to_string());
    let c = cycle(&env, None, None, env.meta.generated_at);
    assert!(event(&events(&c), "reserve_below").is_none());
}

#[test]
fn since_is_carried_while_watched_and_restarts_after_a_gap() {
    let env = live_bottle();
    let t0 = env.meta.generated_at;
    let a = cycle(&env, None, None, t0);
    let b = cycle(&env, None, Some(&a), t0 + chrono::Duration::minutes(5));
    assert_eq!(event(&events(&b), "harvest").unwrap()["since"], t0.to_rfc3339());
    let later = t0 + chrono::Duration::hours(2);
    let c = cycle(&env, None, Some(&b), later);
    assert_eq!(event(&events(&c), "harvest").unwrap()["since"], later.to_rfc3339());
}

#[test]
fn the_state_hash_ignores_the_maintenance_clocks_and_the_events() {
    let env = live_bottle();
    let t0 = env.meta.generated_at;
    let a = cycle(&env, Some(example_cfg()), None, t0);
    let b = cycle(&env, Some(example_cfg()), Some(&a), t0 + chrono::Duration::minutes(55));
    assert_ne!(
        serde_json::to_string(&a.data.derived_metrics.ladder_maintenance).unwrap(),
        serde_json::to_string(&b.data.derived_metrics.ladder_maintenance).unwrap()
    );
    assert_eq!(compute_state_hash(&a.data), compute_state_hash(&b.data));
    let canon = canonical_state_json(&a.data);
    assert!(canon["derived_metrics"].get("events").is_none() && canon["derived_metrics"].get("ladder_maintenance").is_none());
}

#[test]
fn the_cycle_attaches_maintenance_after_the_suggestion_and_before_the_hash() {
    let ingest = include_str!("ingest.rs");
    let suggest = ingest.find("attach_ladder_suggestion(&mut snapshot_data, &profile").unwrap();
    let attach = ingest.find("attach_maintenance_and_events(&mut snapshot_data, &profile, prev.as_ref(), gap_limit, Utc::now())").unwrap();
    let hash = ingest.find("let state_hash = compute_state_hash(&snapshot_data);").unwrap();
    let directive = ingest.find("let next_directive_action = derived_metrics.global_status.shot_advice_action.clone();").unwrap();
    assert!(directive < suggest && suggest < attach && attach < hash);
    // The pushes are unchanged: notify.rs does not read the events.
    assert!(!include_str!("notify.rs").contains("events"));
}

// ---------------- Review fixes (#169) ----------------

/// On a spacing multiple the pool tick is inside [lower, upper): the moved range's top goes
/// one spacing further, so it opens holding only USDC.
#[test]
fn a_move_from_a_pool_tick_on_a_spacing_multiple_opens_out_of_range() {
    let env = live_bottle();
    let mut p = env.data.positions.iter().find(|p| p.position_id == NEAR).unwrap().clone();
    p.lower_tick = 273900;
    p.upper_tick = 275700;
    // The pool tick exactly on 273300: before the fix the range was 273300..275100, in range.
    let (lo, hi) = moved_ticks(&p, 273300, 300).unwrap();
    assert_eq!((lo, hi), (273600, 275400));
    assert!(lo > 273300, "NOCK token1: in range when lower <= tick < upper");
    // Off the multiple: the next multiple up, as before.
    assert_eq!(moved_ticks(&p, 273280, 300), Some((273300, 275100)));
}

/// A standing move survives a pool_stale cycle, unknown reads and a dip to within one
/// spacing above the top; only a real return into the range clears it.
#[test]
fn a_standing_move_is_withdrawn_only_by_a_real_return_into_the_range() {
    let cfg = example_cfg();
    let standing = near_after_25h(4.0, true, cfg.clone());
    assert_eq!(rung(&standing, NEAR)["status"], "suggested");
    let above_since = rung(&standing, NEAR)["clock"]["above_since"].clone();
    let mut env = live_bottle();
    let top = near_top(&env);
    at_price(&mut env, top * 1.04);
    let next = |e: &SnapshotEnvelope, prev: &SnapshotEnvelope| cycle(e, Some(cfg.clone()), Some(prev), prev.meta.generated_at + chrono::Duration::minutes(5));
    // One pool_stale cycle.
    let mut stale = env.clone();
    reading(&mut stale, NEAR).state = "pool_stale".to_string();
    let a = next(&stale, &standing);
    assert_eq!(rung(&a, NEAR)["status"], "suggested", "{}", rung(&a, NEAR)["text"]);
    assert_eq!(rung(&a, NEAR)["clock"]["above_since"], above_since);
    assert!(event(&events(&a), &format!("ladder_near_move:{NEAR}")).is_some());
    // The pool tick not read: unknown, still standing.
    let mut unread = env.clone();
    for m in unread.data.pool_mechanics.iter_mut() {
        m.tick_current = None;
    }
    let b = next(&unread, &a);
    assert_eq!(rung(&b, NEAR)["status"], "suggested");
    // Back at 4% above: still the same move, no clock restarted, no cooldown started.
    let c = next(&env, &b);
    assert_eq!(rung(&c, NEAR)["status"], "suggested");
    assert_eq!(rung(&c, NEAR)["clock"]["suggested_since"], "2026-11-22T17:00:00+00:00");
    assert!(rung(&c, NEAR)["clock"]["last_cleared_at"].is_null());
    // A dip to within one spacing above the top: still above, still standing.
    let mut near_edge = live_bottle();
    at_price(&mut near_edge, top * 1.005);
    let d = next(&near_edge, &c);
    assert_eq!(rung(&d, NEAR)["status"], "suggested", "{}", rung(&d, NEAR)["text"]);
    // A real return into the range clears it and starts the cooldown.
    let mut back = live_bottle();
    at_price(&mut back, top * 0.99);
    reading(&mut back, NEAR).state = "in_range".to_string();
    let e = next(&back, &d);
    assert_eq!(rung(&e, NEAR)["status"], "trading");
    assert!(rung(&e, NEAR)["clock"]["suggested_since"].is_null());
    assert_eq!(rung(&e, NEAR)["clock"]["last_cleared_at"], e.meta.generated_at.to_rfc3339());
}

/// The move states the stretch between the deep rung's top and the moved range's bottom
/// that no rung covers; once minted with USDC only, the moved range is tracked as the near
/// rung by itself.
#[test]
fn a_completed_near_move_is_tracked_and_its_gap_to_the_deep_rung_is_stated() {
    let c = near_after_25h(4.0, false, example_cfg());
    let near = rung(&c, NEAR);
    assert_eq!(near["status"], "suggested");
    assert_eq!(near["move"]["uncovered_from_usd"], "0.01096801");
    assert_eq!(near["move"]["uncovered_to_usd"], "0.01130202");
    assert!(near["text"].as_str().unwrap().contains("$0.01097 to $0.01130 is then covered by no rung"), "{}", near["text"]);
    // Detection reads the standing move from the previous snapshot.
    let data = serde_json::to_value(&c.data).unwrap();
    let rungs = crate::position_detect::suggested_rungs(&data);
    let moved = rungs.iter().find(|r| r.from_move).expect("the moved range is a suggested rung");
    assert_eq!((moved.tick_lower, moved.tick_upper, moved.rung.as_deref()), (273300, 275100, Some("near")));
    let wallet = c.data.positions[0].wallet.clone();
    let pos = c.data.positions.iter().find(|p| p.position_id == NEAR).unwrap();
    let cand = CandidateRow {
        wallet: wallet.clone(),
        protocol: pos.protocol.clone(),
        network: "base".to_string(),
        nft_id: 3300000,
        verdict: "eligible".to_string(),
        owned: true,
        has_liquidity: Some(true),
        pool_id: Some(POOL.to_string()),
        currency0: Some(pos.token0.address.clone()),
        currency1: Some(pos.token1.address.clone()),
        fee_raw: Some(30000),
        tick_spacing: Some(300),
        hooks: Some("0x0000000000000000000000000000000000000000".to_string()),
        tick_lower: Some(273300),
        tick_upper: Some(275100),
        received_from: Some("0x0000000000000000000000000000000000000000".to_string()),
        mint_tx: Some("0xabc".to_string()),
        mint_block: Some(60500000),
        minted_by: Some(wallet.clone()),
        deposit_check: Some("ok".to_string()),
        ..Default::default()
    };
    let tracked = crate::position_detect::tracked_ranges_in_snapshot(&data);
    assert_eq!(
        crate::position_detect::decide(&cand, &wallet, &rungs, &tracked),
        crate::position_detect::Decision::Track { rung: Some("near".to_string()) }
    );
    // A mint that put NOCK in is not the moved (USDC-only) near rung: offered, with why.
    let with_nock = CandidateRow { deposit_check: Some(crate::position_detect::DEPOSIT_OK_HOLDS_NOCK.to_string()), ..cand };
    match crate::position_detect::decide(&with_nock, &wallet, &rungs, &tracked) {
        crate::position_detect::Decision::Offer { why_not_automatic, .. } => {
            assert!(why_not_automatic.iter().any(|w| w.contains("the moved near rung is a USDC-only range")), "{why_not_automatic:?}")
        }
        other => panic!("expected an offer, got {other:?}"),
    }
}

// ---------------- MCP text, both transports ----------------

fn text_case(env: &SnapshotEnvelope, health: Option<&str>) -> Value {
    let data = serde_json::to_value(&env.data).unwrap();
    let dm = json!({ "derived_metrics": { "events": data["derived_metrics"]["events"], "ladder_maintenance": data["derived_metrics"]["ladder_maintenance"] } });
    json!({ "data": dm, "health": health, "text": crate::events_text::events_lines(&dm, health) })
}

#[test]
fn mcp_text_golden_for_both_transports() {
    let env = live_bottle();
    let t0 = env.meta.generated_at;
    let unset = cycle(&env, None, None, t0);
    let standing = near_after_25h(4.0, false, example_cfg());
    let mut conv = live_bottle();
    all_below(&mut conv, 0.0099);
    let converted = cycle(&conv, Some(example_cfg()), None, t0);
    let doc = json!({
        "live_unset": text_case(&unset, Some("ok")),
        "near_move": text_case(&standing, Some("ok")),
        "deep_converted_stale": text_case(&converted, Some("degraded")),
        "predates": { "data": { "derived_metrics": {} }, "health": "ok", "text": crate::events_text::events_lines(&json!({"derived_metrics": {}}), Some("ok")) },
    });
    let actual = format!("{}\n", serde_json::to_string_pretty(&doc).unwrap());
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/ladder_maintenance/golden_events_text.json");
    if std::env::var("NOCKNOCK_BLESS_GOLDEN").is_ok() {
        std::fs::write(path, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(path).expect("testdata/ladder_maintenance/golden_events_text.json is checked in");
    assert!(actual == expected, "events text changed; bless with NOCKNOCK_BLESS_GOLDEN=1 and review the diff");
    // What the lines say.
    let t = doc["live_unset"]["text"].as_str().unwrap();
    assert!(t.starts_with("• Top of page (1):\n  – WALLET USDC BELOW THE STANDING RESERVE: "), "{t}");
    assert!(t.contains("• Quiet ("), "{t}");
    assert!(t.contains("  – Near rung #3244700 (off): "), "{t}");
    let t = doc["near_move"]["text"].as_str().unwrap();
    assert!(t.starts_with("• Top of page (3):\n  – WALLET USDC BELOW THE STANDING RESERVE: "), "{t}");
    assert!(t.contains("\n  – LADDER: MOVE THE NEAR RUNG: "), "{t}");
    assert!(t.contains("\n  – LADDER: DEEP PULL CANNOT FIRE: "), "{t}");
    assert!(t.contains("    Would sign: Two transactions in your wallet"), "{t}");
    let t = doc["deep_converted_stale"]["text"].as_str().unwrap();
    assert!(t.contains("the worker's health is degraded"), "{t}");
    assert!(t.contains("LADDER: DEEP RUNG FULLY CONVERTED"), "{t}");
    assert_eq!(doc["predates"]["text"], "• Top of page: not reported (the snapshot predates 2.11.0).\n");
    // The worker's /v1/mcp composes it after the directive, before the ladder suggestion.
    let advice = format_shot_advice_text(&doc["near_move"]["data"]);
    let full = crate::events_text::with_events(&advice, &doc["near_move"]["data"], Some("ok"));
    assert_eq!(full, format!("{advice}\n{}", doc["near_move"]["text"].as_str().unwrap().trim_end()));
    let http = include_str!("mcp_http.rs");
    assert!(http.contains("crate::events_text::with_events(&format_shot_advice_text(&data_val), &data_val, Some(health))"));
}
