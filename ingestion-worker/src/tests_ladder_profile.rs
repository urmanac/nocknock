//! The ladder shape (#169, operator 2026-11-21): bars on a log price axis whose area is their
//! capital, on the operator's live bottle (testdata/ladder_maintenance/live_bottle_2026-11-21.json).
use super::*;
use crate::tests_ladder_maintenance::{cycle, live_bottle};
use serde_json::Value;

fn profile_of(env: &SnapshotEnvelope) -> Value {
    let c = cycle(env, None, None, env.meta.generated_at);
    c.data.derived_metrics.ladder_profile.clone().expect("a tracked ladder has a profile")
}

fn bar<'a>(p: &'a Value, id: &str) -> &'a Value {
    p["bars"].as_array().unwrap().iter().find(|b| b["position_id"] == id).unwrap_or_else(|| panic!("no bar {id}"))
}

fn n(v: &Value, k: &str) -> f64 {
    v[k].as_str().unwrap().parse().unwrap()
}

#[test]
fn the_live_bottle_bars_are_the_deployed_ranges_with_their_capital() {
    let p = profile_of(&live_bottle());
    let deep = bar(&p, "uniswap-v4:base:3244701");
    assert_eq!((deep["role"].as_str(), deep["capital_usd"].as_str(), deep["capital_basis"].as_str()), (Some("deep"), Some("629.52"), Some("usdc_at_top")));
    assert_eq!((deep["low_usd"].as_str(), deep["high_usd"].as_str()), (Some("0.01002405"), Some("0.01096801")));
    let near = bar(&p, "uniswap-v4:base:3244700");
    assert_eq!((near["capital_usd"].as_str(), near["low_usd"].as_str(), near["high_usd"].as_str()), (Some("111.09"), Some("0.01096801"), Some("0.01313098")));
    let cap = bar(&p, "uniswap-v4:base:3244699");
    assert_eq!((cap["capital_usd"].as_str(), cap["low_usd"].as_str(), cap["high_usd"].as_str()), (Some("27.63"), Some("0.01313098"), Some("0.01353085")));
    assert_eq!(cap["text"], "cap $27.63");
    // The wide v4 range: its value now, over about $0.0055 to $0.041.
    let wide = bar(&p, "uniswap-v4:base:3163450");
    assert_eq!((wide["role"].as_str(), wide["capital_usd"].as_str(), wide["capital_basis"].as_str()), (Some("other"), Some("260.40"), Some("value_now")));
    assert!((n(wide, "low_usd") - 0.0055).abs() < 0.0001 && (n(wide, "high_usd") - 0.04106).abs() < 0.00001, "{wide}");
    assert_eq!(wide["label"], "v4 USDC/NOCK 3.00% #3163450");
    // The Aero full-range pool is stated, not drawn; the ETH/USDC ranges hold no NOCK.
    assert_eq!(p["bars"].as_array().unwrap().len(), 4);
    let fr = &p["full_range"][0];
    assert_eq!(fr["value_usd"], "225.36");
    assert!(fr["text"].as_str().unwrap().contains("full range, no finite width, not drawn"));
    // Markers.
    assert_eq!(p["spot_usd"], "0.01355744");
    assert_eq!(p["ladder_bottom_usd"], "0.01000000");
    assert_eq!(p["reserve_note"], "$950.00 reserve kept for prices below the ladder bottom");
}

#[test]
fn each_bars_area_on_the_log_axis_is_its_capital_to_the_cent() {
    let p = profile_of(&live_bottle());
    for b in p["bars"].as_array().unwrap() {
        let area = n(b, "density") * (n(b, "high_usd") / n(b, "low_usd")).ln();
        assert!((area - n(b, "capital_usd")).abs() < 0.005, "{b}: area {area}");
    }
    // The bottle: the deep rung is the densest ladder bar, the cap the thinnest per unit of capital.
    let d = |id: &str| n(bar(&p, id), "density");
    assert!(d("uniswap-v4:base:3244701") > d("uniswap-v4:base:3244700"));
}

#[test]
fn overlapping_ranges_stack_into_the_depth_profile() {
    let p = profile_of(&live_bottle());
    let d = |id: &str| n(bar(&p, id), "density");
    let seg = p["segments"].as_array().unwrap().iter().find(|s| s["low_usd"] == "0.01002405").unwrap();
    assert_eq!(seg["high_usd"], "0.01096801");
    assert!((n(seg, "density") - (d("uniswap-v4:base:3244701") + d("uniswap-v4:base:3163450"))).abs() < 1e-3, "{seg}");
    assert_eq!(seg["position_ids"].as_array().unwrap().len(), 2);
    // Below the deep rung only the wide range is deployed.
    let below = p["segments"].as_array().unwrap().iter().find(|s| s["high_usd"] == "0.01002405").unwrap();
    assert!((n(below, "density") - d("uniswap-v4:base:3163450")).abs() < 1e-3);
}

#[test]
fn a_wide_range_is_clipped_to_the_ladder_zone_and_keeps_its_full_width_density() {
    let p = profile_of(&live_bottle());
    // The view: the ladder bottom $0.01 / 1.05 to spot $0.01355744 * 1.05.
    assert!((n(&p, "view_low_usd") - 0.01 / 1.05).abs() < 1e-8);
    assert!((n(&p, "view_high_usd") - 0.01355744 * 1.05).abs() < 1e-8);
    let wide = bar(&p, "uniswap-v4:base:3163450");
    assert_eq!((wide["clipped_low"].as_bool(), wide["clipped_high"].as_bool()), (Some(true), Some(true)));
    assert!(wide["clip_text"].as_str().unwrap().starts_with("continues from $0.005"), "{wide}");
    assert!(wide["clip_text"].as_str().unwrap().ends_with("to $0.04106"), "{wide}");
    let full = 260.40 / (n(wide, "high_usd") / n(wide, "low_usd")).ln();
    assert!((n(wide, "density") - full).abs() < 1e-3, "density on its full width, never on the clipped part");
    assert_eq!(bar(&p, "uniswap-v4:base:3244701")["clipped_low"], false);
}

#[test]
fn a_rung_whose_capital_is_unknown_has_no_height() {
    let mut env = live_bottle();
    env.data.ladders.iter_mut().find(|l| l.position_id == "uniswap-v4:base:3244700").unwrap().vs_top.usdc_at_top = None;
    let p = profile_of(&env);
    let near = bar(&p, "uniswap-v4:base:3244700");
    assert!(near["capital_usd"].is_null() && near["density"].is_null());
    assert_eq!(near["status"], "capital_unknown");
    assert_eq!(near["text"], "near: capital unknown");
    let seg = p["segments"].as_array().unwrap().iter().find(|s| s["low_usd"] == "0.01096801").unwrap();
    assert_eq!(seg["includes_unknown"], true);
}

#[test]
fn no_ladder_no_profile_and_the_mcp_table() {
    let mut env = live_bottle();
    let p = profile_of(&env);
    let text = crate::events_text::profile_lines(&json!({ "derived_metrics": { "ladder_profile": p } }));
    assert!(text.starts_with("• Ladder shape (each bar on a log price axis; height capital / ln(high/low), so area = capital):\n"), "{text}");
    assert!(text.contains("  – deep (v4 USDC/NOCK 3.00% #3244701): $0.01002–$0.01097, $629.52 (all USDC at its top), density "), "{text}");
    assert!(text.contains("  – #3163450 (v4 USDC/NOCK 3.00% #3163450): $0.005"), "{text}");
    assert!(text.contains("(value now)"), "{text}");
    assert!(text.contains("  Markers: spot $0.01356; ladder bottom $0.01000; $950.00 reserve kept for prices below the ladder bottom.\n"), "{text}");
    assert!(text.contains("  Not drawn: Aero USDC/NOCK 0.30% (full range) $225.36: full range, no finite width, not drawn.\n"), "{text}");
    env.data.ladders.clear();
    let c = cycle(&env, None, None, env.meta.generated_at);
    assert!(c.data.derived_metrics.ladder_profile.is_none());
}
