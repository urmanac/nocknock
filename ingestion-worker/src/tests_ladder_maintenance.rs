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
    // Nothing at the top of the page: HOLD is quiet, the suggestion card is not shown.
    assert!(top_ids(&c).is_empty(), "{:?}", top_ids(&c));
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

fn deep_converting(env: &mut SnapshotEnvelope, price: f64, converted_pct: f64, nock: f64, usdc: f64) {
    at_price(env, price);
    let r = reading(env, DEEP);
    r.state = "in_range".to_string();
    r.vs_top.converted_pct = Some(format!("{converted_pct:.2}"));
    r.nock_now = format!("{nock:.4}");
    r.usdc_now = format!("{usdc:.2}");
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

#[test]
fn deep_rung_pull_after_it_bought_low_and_the_price_recovered() {
    let cfg = example_cfg();
    let mut env = live_bottle();
    // The dump: 92% converted at a low of $0.0101.
    deep_converting(&mut env, 0.0101, 92.0, 55000.0, 50.0);
    let t0 = t("2026-11-25T10:00:00Z");
    let low = cycle(&env, Some(cfg.clone()), None, t0);
    let deep = rung(&low, DEEP);
    assert_eq!(deep["status"], "holding", "{}", deep["text"]);
    assert_eq!(deep["clock"]["episode"]["low_price_usd"], "0.01010000");
    assert_eq!(deep["clock"]["episode"]["peak_converted_pct"], "92.00");
    // A rebound to $0.0112 (+10.9%): the rung starts selling back; the pull is stated.
    deep_converting(&mut env, 0.0112, 60.0, 36000.0, 240.0);
    let up = cycle(&env, Some(cfg.clone()), Some(&low), t0 + chrono::Duration::minutes(5));
    let deep = rung(&up, DEEP);
    assert_eq!(deep["status"], "suggested", "{}", deep["text"]);
    assert_eq!(deep["pull"]["peak_converted_pct"], "92.00");
    let ev = event(&events(&up), &format!("ladder_deep_pull:{DEEP}")).unwrap().clone();
    assert_eq!(ev["tier"], "top");
    assert_eq!(ev["title"], "LADDER: PULL THE DEEP RUNG");
    assert!(ev["would_sign"].as_str().unwrap().starts_with("One transaction in your wallet: remove all of Deep rung #3244701's liquidity"));
    // A dip inside the hysteresis band keeps it (no churn): 2% under the recovery point.
    deep_converting(&mut env, 0.0101 * 1.10 * 0.985, 65.0, 39000.0, 200.0);
    let dip = cycle(&env, Some(cfg.clone()), Some(&up), t0 + chrono::Duration::minutes(10));
    assert_eq!(rung(&dip, DEEP)["status"], "suggested");
    // A new low clears it and moves the recovery point down.
    deep_converting(&mut env, 0.0100, 99.0, 59000.0, 5.0);
    let new_low = cycle(&env, Some(cfg.clone()), Some(&dip), t0 + chrono::Duration::minutes(15));
    let deep = rung(&new_low, DEEP);
    assert_eq!(deep["status"], "holding", "{}", deep["text"]);
    assert_eq!(deep["clock"]["episode"]["low_price_usd"], "0.01000000");
    assert!(deep["clock"]["last_cleared_at"].is_string());
    // Below the fill setting the pull is never stated, whatever the recovery.
    let mut env2 = live_bottle();
    deep_converting(&mut env2, 0.0105, 30.0, 18000.0, 440.0);
    let a = cycle(&env2, Some(cfg.clone()), None, t0);
    deep_converting(&mut env2, 0.0120, 30.0, 18000.0, 440.0);
    let b = cycle(&env2, Some(cfg.clone()), Some(&a), t0 + chrono::Duration::minutes(5));
    assert_eq!(rung(&b, DEEP)["status"], "holding");
    // With its settings unset the pull is off and says which settings turn it on.
    let off = cycle(&env, None, Some(&dip), t0 + chrono::Duration::minutes(15));
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

#[test]
fn a_reserve_dip_from_the_ranges_buying_is_quiet_and_a_wallet_shortfall_is_top() {
    // Stable $1,780.87 against a $1,800 reserve: wallet $778.77 plus the ranges' USDC at
    // their tops covers it, so the dip is their buying: quiet, with the split.
    let mut env = live_bottle();
    {
        let g = &mut env.data.derived_metrics.global_status;
        g.cash_reserve_usdc = Some("1800.00".to_string());
        g.reserve_headroom_usdc = Some("-19.13".to_string());
    }
    let c = cycle(&env, None, None, env.meta.generated_at);
    let evs = events(&c);
    let r = event(&evs, "reserve_below").unwrap();
    assert_eq!(r["tier"], "quiet", "{r}");
    let text = r["text"].as_str().unwrap();
    assert!(text.starts_with("Stable $1780.87 (wallet USDC $778.77 + NOCK LP USDC $233.86 + ladder USDC $768.24) is $19.13 below your $1800.00 reserve."), "{text}");
    assert!(text.contains("the dip is their buying"), "{text}");
    // A reserve the ranges' USDC at their tops cannot cover: top of the page.
    {
        let g = &mut env.data.derived_metrics.global_status;
        g.cash_reserve_usdc = Some("2400.00".to_string());
        g.reserve_headroom_usdc = Some("-619.13".to_string());
    }
    let c = cycle(&env, None, None, env.meta.generated_at);
    let evs = events(&c);
    let r = event(&evs, "reserve_below").unwrap();
    assert_eq!(r["tier"], "top", "{r}");
    assert!(r["text"].as_str().unwrap().contains("More than the ranges' buying"));
    // Above the reserve (live: $830.87 of headroom): no entry.
    let live = cycle(&live_bottle(), None, None, live_bottle().meta.generated_at);
    assert!(event(&events(&live), "reserve_below").is_none());
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
    assert!(t.starts_with("• Top of page: nothing needs you.\n"), "{t}");
    assert!(t.contains("• Quiet ("), "{t}");
    assert!(t.contains("  – Near rung #3244700 (off): "), "{t}");
    let t = doc["near_move"]["text"].as_str().unwrap();
    assert!(t.starts_with("• Top of page (1):\n  – LADDER: MOVE THE NEAR RUNG: "), "{t}");
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
