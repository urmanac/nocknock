//! stdio MCP binary: tracked ladders (issue #167) in the snapshot-API fallback. Included
//! from src/bin/mcp.rs. This binary cannot call the worker's mcp_text, so parity is checked
//! against the text the worker's own golden test wrote (testdata/golden_snapshot_ladder.json).

use super::*;

fn golden() -> Value {
    serde_json::from_str(include_str!("../testdata/golden_snapshot_ladder.json")).expect("the golden file parses")
}

#[test]
fn test_fallback_prints_the_ladder_lines_the_worker_prints() {
    for (case, v) in golden().as_object().unwrap() {
        let data = &v["snapshot"];
        let state = format_portfolio_state(data);
        // The whole portfolio text is byte-identical to the worker's.
        assert_eq!(state, v["mcp_text"]["get_portfolio_state"].as_str().unwrap(), "{case}");
        let block = ladder_track_text::ladders_lines(data);
        assert!(block.starts_with("• Ladders (measured, not NOCK LP; counted in the totals, the reserve and the floor-crossing price):\n"), "{case}");
        // Right after the AERO lines, before the high-water mark.
        assert!(state.contains(&format!("• AERO/USD: not reported\n{block}• High-Water Mark")), "{case}: {state}");
        // The reserve line names the ladders' USDC, as the worker's does.
        assert!(state.contains("• Protected cash reserve: $950.00 (wallet + NOCK LP and ladder USDC ($"), "{case}");
        // The plan text says ladders are not in the rebuild.
        let plan = format_accumulation_plan(data);
        assert!(plan.contains("\n• Ladders are not rebuild templates and are not in the NOCK LP target (2 tracked, measured only)."), "{case}: {plan}");
        assert!(v["mcp_text"]["get_accumulation_plan"].as_str().unwrap().contains(&ladder_track_text::ladder_rebuild_note(data)), "{case}");
    }
}

#[test]
fn test_fallback_states_a_verified_buy_and_a_missing_baseline() {
    let g = golden();
    let state = format_portfolio_state(&g["near_rung_bought_plan_due"]["snapshot"]);
    assert!(state.contains("since deposit (tx 0xafe42fe2…, block 59660587): net 5303.7885 NOCK bought for net $84.35 USDC spent, average $0.01590283"), "{state}");
    assert!(state.contains("Plus the net buys of 1 of 2 tracked ladders since a verified deposit: 5303.7885 NOCK for $84.35."), "{state}");
    assert!(state.contains("(rung unset)") && state.contains("since deposit: no_baseline: no deposit_tx is set for this ladder in config.yaml"), "{state}");
}

#[test]
fn test_fallback_without_ladders_prints_no_ladder_line() {
    // Every other golden file has no ladder: no line, no note, no changed word.
    for s in [
        include_str!("../testdata/golden_snapshot.json"),
        include_str!("../testdata/golden_snapshot_closed.json"),
        include_str!("../testdata/golden_snapshot_aero_cl.json"),
    ] {
        let v: Value = serde_json::from_str(s).unwrap();
        for (case, c) in v.as_object().unwrap() {
            let data = &c["snapshot"];
            assert_eq!(ladder_track_text::ladders_lines(data), "", "{case}");
            assert_eq!(ladder_track_text::ladder_rebuild_note(data), "", "{case}");
            assert!(!format_portfolio_state(data).contains("adder"), "{case}");
            assert!(!format_accumulation_plan(data).contains("adder"), "{case}");
        }
    }
}
