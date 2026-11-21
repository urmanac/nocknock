//! stdio MCP binary: positions found on-chain (v0.9.0). Included from src/bin/mcp.rs. This
//! binary cannot call the worker's mcp_text, so parity is checked against the text the
//! worker's own golden test wrote from the recorded chain
//! (testdata/position_detect/golden_position_detect.json), through the stdio's own
//! format_portfolio_state and get_shot_advice paths.

use super::*;

fn golden() -> Value {
    serde_json::from_str(include_str!("../testdata/position_detect/golden_position_detect.json")).expect("the golden file parses")
}

#[test]
fn test_stdio_prints_the_found_positions_the_worker_prints() {
    let g = golden();
    assert!(g.as_object().unwrap().len() >= 7);
    for (case, v) in g.as_object().unwrap() {
        let data = json!({ "derived_metrics": { "position_detection": v["position_detection"] } });
        let expected = v["text"].as_str().unwrap();
        assert_eq!(position_detect_text::position_detection_lines(&data), expected, "{case}");
        // get_portfolio_state carries the block, right before the high-water mark.
        let state = format_portfolio_state(&data);
        assert!(state.contains(&format!("{expected}• High-Water Mark")), "{case}: {state}");
        // get_shot_advice carries it only when it bears on the totals.
        let advice = format_shot_advice(&data);
        let with = position_detect_text::with_position_notice(&advice, &data);
        let quiet = v["position_detection"]["status"] == "ok" && v["position_detection"]["found_not_counted"]["count"] == 0;
        if quiet {
            assert_eq!(with, advice, "{case}");
        } else {
            assert_eq!(with, format!("{advice}\n{}", expected.trim_end()), "{case}");
        }
    }
    assert!(g["found_not_counted"]["text"].as_str().unwrap().contains("NOT counted in any total: 1 position worth $240.61"));
}

#[test]
fn test_stdio_tools_list_names_the_found_positions() {
    for src in [include_str!("bin/mcp.rs"), include_str!("mcp_http.rs")] {
        assert!(src.contains("Also states the positions found on-chain (v0.9.0)"));
        assert!(src.contains("A position found on-chain and not counted, or a check for new positions that did not run, is stated after it."));
    }
    // Both transports call the same two functions of the same file.
    assert!(include_str!("bin/mcp.rs").contains("position_detect_text::with_position_notice("));
    assert!(include_str!("mcp_http.rs").contains("crate::position_detect_text::with_position_notice("));
    assert!(include_str!("mcp_text.rs").contains("crate::position_detect_text::position_detection_lines(data)"));
}

#[test]
fn test_an_older_snapshot_prints_no_found_positions_block() {
    let data = json!({ "derived_metrics": {} });
    assert_eq!(position_detect_text::position_detection_lines(&data), "");
    assert_eq!(position_detect_text::position_detection_trailer(&data), "");
    assert_eq!(position_detect_text::with_position_notice("advice", &data), "advice");
    // No MCP tool can track, untrack or dismiss: the one-tap writes are the dashboard's.
    let src = include_str!("bin/mcp.rs");
    assert!(!src.contains("/v1/positions/"));
    assert_eq!(position_detect_text::choice_words("ladder:near"), "TRACK AS NEAR RUNG");
    assert_eq!(position_detect_text::choice_words("lp"), "TRACK AS LP");
    assert_eq!(position_detect_text::choice_words("dismiss"), "NOT NOW");
}
