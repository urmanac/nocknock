//! Stdio MCP fallback text for fill detection (issue #118). Lives outside src/bin so
//! cargo does not take it for another binary; included by src/bin/mcp.rs under test.

use super::*;

const BUY: &str = "0x78376f3cc9e80f2eb124a945a452df40556649e3d9983be526b2bd97586e31b9";

fn snapshot_with(fill_detection: Value) -> Value {
    json!({ "derived_metrics": {
        "accumulation_plan": {
            "plan_id": "2026-11-reentry", "status": "tranche_due", "tranche_index": 3, "tranches_total": 5,
            "tranche_usdc": "20.00", "nominal_tranche_usdc": "20.00", "next_due_at": null,
            "spent_usdc": "40.00", "budget_usdc": "100.00", "avg_price_usd": "0.012055",
            "fill_detection": fill_detection
        },
        "lp_rebuild": { "status": "on_target", "gap_usd": "0.00" }
    } })
}

#[test]
fn test_fallback_plan_text_lists_offers_read_only() {
    let text = format_accumulation_plan(&snapshot_with(json!({
        "status": "ok", "detail": null,
        "checked_back_to": "2026-11-14T00:00:00+00:00", "last_checked_at": "2026-11-16T04:00:00+00:00",
        "offers": [{
            "tx_hash": BUY, "confirmed_at": "2026-11-16T03:08:37+00:00", "block_number": 58040708,
            "usdc_spent": "20", "nock_received": "1780.3592976960049083", "effective_price_usd": "0.01123369",
            "timing": "on_schedule", "reference_spot_usd": null, "reference_spot_source": null, "execution_cost_bps": null
        }],
        "dismissed": []
    })));
    assert!(text.contains("• Swap detection: checked 2026-11-16T04:00:00+00:00 back to 2026-11-14T00:00:00+00:00\n"), "{text}");
    assert!(text.contains("• 1 detected swap(s) not counted yet."), "{text}");
    assert!(text.contains("(MCP is read-only)"), "{text}");
    assert!(
        text.contains(&format!(
            "   ? {BUY} · 2026-11-16T03:08:37+00:00 · $20 USDC → 1780.3592976960049083 NOCK at $0.01123369 · cost over spot unavailable · on schedule\n"
        )),
        "{text}"
    );
    // Offers are listed, not counted.
    assert!(text.contains("Progress: $40.00 of $100.00 spent"), "{text}");
    assert!(text.contains("LP rebuild bias: on_target"), "{text}");
}

#[test]
fn test_fallback_could_not_check_is_never_none_found() {
    let down = format_accumulation_plan(&snapshot_with(json!({
        "status": "unavailable",
        "detail": "the wallet's transfer history could not be read (alchemy_getAssetTransfers gave no answer)",
        "checked_back_to": null, "last_checked_at": null, "offers": [], "dismissed": []
    })));
    assert!(down.contains("• Swap detection: could not check this cycle (the wallet's transfer history could not be read"), "{down}");
    assert!(down.contains("last successful check: none on record"), "{down}");
    assert!(!down.contains("no uncounted swap found"), "{down}");

    let none = format_accumulation_plan(&snapshot_with(json!({
        "status": "ok", "detail": null, "checked_back_to": "2026-11-14T00:00:00+00:00",
        "last_checked_at": "2026-11-16T04:00:00+00:00", "offers": [], "dismissed": []
    })));
    assert!(none.contains("; no uncounted swap found\n"), "{none}");

    // A worker older than v0.6.0 reports no detection: the text claims nothing about it.
    let old = format_accumulation_plan(&snapshot_with(Value::Null));
    assert!(!old.contains("Swap detection"), "{old}");
}

#[test]
fn test_stdio_mcp_gains_no_write_tool() {
    let src = include_str!("bin/mcp.rs");
    for banned in ["\"confirm", "\"dismiss", "\"restore", "\"record_"] {
        assert!(!src.contains(&format!("\"name\": {banned}")), "{banned}");
    }
    assert_eq!(src.matches("\"inputSchema\"").count(), 4, "four tools, as before");
}
