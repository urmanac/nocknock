//! stdio MCP binary: the floor-crossing price line (issue #165) in the snapshot-API
//! fallback. Included from src/bin/mcp.rs. This binary cannot call the worker's
//! mcp_text, so parity is checked against the text the worker's own golden tests wrote:
//! every golden case, crosses, already_below, never and no_floor among them.

use super::*;

fn goldens() -> Vec<(&'static str, Value)> {
    [
        ("golden_snapshot.json", include_str!("../testdata/golden_snapshot.json")),
        ("golden_snapshot_aero_cl.json", include_str!("../testdata/golden_snapshot_aero_cl.json")),
        ("golden_snapshot_aero_rewards.json", include_str!("../testdata/golden_snapshot_aero_rewards.json")),
        ("golden_snapshot_closed.json", include_str!("../testdata/golden_snapshot_closed.json")),
        ("golden_snapshot_rewards.json", include_str!("../testdata/golden_snapshot_rewards.json")),
    ]
    .into_iter()
    .map(|(name, s)| (name, serde_json::from_str(s).expect("the golden file parses")))
    .collect()
}

#[test]
fn test_fallback_prints_the_floor_crossing_line_the_worker_prints() {
    let mut statuses = Vec::new();
    for (file, golden) in goldens() {
        for (case, v) in golden.as_object().unwrap() {
            let data = &v["snapshot"];
            let state = format_portfolio_state(data);
            assert_eq!(state, v["mcp_text"]["get_portfolio_state"].as_str().unwrap(), "{file}:{case}");
            let fb = data.pointer("/derived_metrics/floor_breach");
            let status = fb.and_then(|f| f["status"].as_str()).unwrap().to_string();
            let line = floor_breach_text::floor_breach_line(fb);
            if status == "no_floor" {
                assert_eq!(line, "", "{file}:{case}");
                assert!(!state.contains("Floor crossing"), "{file}:{case}");
            } else {
                // Right under the stack line.
                assert!(state.contains(&format!(")\n{line}• Stable Stack")), "{file}:{case}: {state}");
            }
            statuses.push(status);
        }
    }
    for s in ["crosses", "already_below", "never", "no_floor"] {
        assert!(statuses.iter().any(|x| x == s), "a golden case is {s}: {statuses:?}");
    }
}

#[test]
fn test_fallback_states_unavailable_and_absent_figures() {
    let mut data = json!({"derived_metrics": {"global_status": {
        "total_nock_amount": "109542", "nock_floor_target": "100000", "nock_floor_delta": "9542",
        "total_usdc_amount": "1545.84", "shot_advice_action": "HOLD"
    }}});
    data["derived_metrics"]["floor_breach"] = json!({
        "status": "unavailable", "floor_nock": "100000", "positions": [], "assumption": "",
        "reason": "v4 USDC/NOCK 3.00% #3163450: pool price could not be read (sqrtPrice is zero)"
    });
    assert!(format_portfolio_state(&data).contains(
        "• Floor crossing: unavailable: v4 USDC/NOCK 3.00% #3163450: pool price could not be read (sqrtPrice is zero)\n• Stable Stack"
    ));
    // A snapshot from before 2.9.0 says so, never "safe".
    data["derived_metrics"].as_object_mut().unwrap().remove("floor_breach");
    let text = format_portfolio_state(&data);
    assert!(text.contains("• Floor crossing: not reported (snapshot predates 2.9.0)\n"), "{text}");
    assert!(!text.contains("$0.00"));
}
