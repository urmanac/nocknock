//! stdio MCP binary: the LP rebuild line in the snapshot-API fallback (issue #151).
//! Included from src/bin/mcp.rs. The rebuild judges NOCK-pair LP only, so the line says
//! "NOCK LP rebuild bias"; only under_target has a shortfall, and an over_target book is
//! stated as over, never "$0.00 short".

use super::*;

fn with_lp(lp: Value) -> Value {
    json!({ "derived_metrics": { "lp_rebuild": lp } })
}

#[test]
fn test_fallback_over_target_states_the_overage_never_zero_short() {
    // The issue's case: gas-side ranges counted as LP put the book $385.13 over target.
    let text = format_accumulation_plan(&with_lp(json!({
        "status": "over_target", "actual_usd": "1008.79", "target_usd": "623.66", "gap_usd": "0.00"
    })));
    assert!(text.contains("• NOCK LP rebuild bias: over_target (over target by $385.13;"), "{text}");
    assert!(!text.contains("short"), "{text}");

    let on = format_accumulation_plan(&with_lp(json!({
        "status": "on_target", "actual_usd": "489.52", "target_usd": "519.80", "gap_usd": "30.28"
    })));
    assert!(on.contains("• NOCK LP rebuild bias: on_target") && !on.contains("short"), "{on}");

    let under = format_accumulation_plan(&with_lp(json!({
        "status": "under_target", "actual_usd": "220.27", "target_usd": "641.14", "gap_usd": "420.87"
    })));
    assert!(under.contains("• NOCK LP rebuild bias: under_target ($420.87 short of target;"), "{under}");

    let none = format_accumulation_plan(&with_lp(json!({ "status": "no_positions" })));
    assert!(none.contains("No NOCK LP positions are tracked"), "{none}");
}
