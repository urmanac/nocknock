//! Stdio MCP fallback text for route quotes (issue #115). Lives outside src/bin so cargo
//! does not take it for another binary; included by src/bin/mcp.rs under test. The
//! figures are the comparison read on Base at block 58081910 (testdata/quotes).

use super::*;

fn snapshot_with(route_quotes: Value) -> Value {
    json!({ "derived_metrics": {
        "accumulation_plan": {
            "plan_id": "2026-11-reentry", "status": "tranche_due", "tranche_index": 4, "tranches_total": 5,
            "tranche_usdc": "20.00", "nominal_tranche_usdc": "20.00", "next_due_at": null,
            "spent_usdc": "60.00", "budget_usdc": "100.00", "avg_price_usd": "0.011777",
            "route_quotes": route_quotes
        },
        "lp_rebuild": { "status": "on_target", "gap_usd": "0.00" }
    } })
}

fn real_quotes() -> Value {
    json!({
        "status": "ok", "detail": null, "amount_usdc": "20.00",
        "block_number": 58081910, "quoted_at": "2026-11-16T05:25:57+00:00",
        "reference_spot_usd": "0.01128675",
        "venues": [
            {
                "venue": "Aero", "status": "quoted", "reason": null, "swap_url": "https://app.aero.xyz/swap",
                "nock_out": "1773.0768618618146014", "effective_price_usd": "0.01127983", "cost_over_spot_pct": "-0.06",
                "route": "USDC → USDT → WETH → NOCK",
                "hops": [
                    { "pool": "0xd448990883e3bf9cdd8ebca368a3ecf9d629aaf7", "pool_kind": "Aero concentrated", "token_in": "USDC", "token_out": "USDT", "fee_pct": "0.0008" },
                    { "pool": "0x2dbe0e264c7c246e05c6614794899937f264cf7b", "pool_kind": "Aero concentrated", "token_in": "USDT", "token_out": "WETH", "fee_pct": "0.0468" },
                    { "pool": "0x957256c98d61a714081de2ee70ae3a96c9b40874", "pool_kind": "Aero concentrated", "token_in": "WETH", "token_out": "NOCK", "fee_pct": "0.97" }
                ],
                "routes_tried": []
            },
            {
                "venue": "Uniswap", "status": "quoted", "reason": null, "swap_url": "https://app.uniswap.org/swap",
                "nock_out": "1772.2085413951150619", "effective_price_usd": "0.01128535", "cost_over_spot_pct": "-0.01",
                "route": "USDC → WETH → NOCK",
                "hops": [
                    { "pool": "0xd0b53d9277642d899df5c87a3966a349a798f224", "pool_kind": "Uniswap v3", "token_in": "USDC", "token_out": "WETH", "fee_pct": "0.05" },
                    { "pool": "0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a", "pool_kind": "Uniswap v3", "token_in": "WETH", "token_out": "NOCK", "fee_pct": "1" }
                ],
                "routes_tried": []
            }
        ],
        "more_nock_venue": "Aero", "more_nock_by_pct": "0.05",
        "comparison": "Aero returns more NOCK by 0.05% for $20.00 USDC at block 58081910: 1773.0768 NOCK against 1772.2085 on Uniswap."
    })
}

#[test]
fn test_fallback_plan_text_shows_both_venues_and_the_caveat() {
    let data = snapshot_with(real_quotes());
    let text = format_accumulation_plan(&data);
    // The fallback prints the very lines the worker's own /v1/mcp prints.
    let shared = route_quote_lines(&data["derived_metrics"]["accumulation_plan"]);
    assert!(text.contains(&shared), "{text}");
    assert!(text.contains("• Route quotes for $20.00 USDC → NOCK at block 58081910 (2026-11-16T05:25:57+00:00), reference spot $0.01128675:"), "{text}");
    assert!(text.contains("Aero: 1773.0768 NOCK at $0.01127983 (0.06% under the reference spot) via USDC → USDT → WETH → NOCK"), "{text}");
    assert!(text.contains("Uniswap: 1772.2085 NOCK at $0.01128535 (0.01% under the reference spot) via USDC → WETH → NOCK"), "{text}");
    assert!(text.contains("WETH→NOCK Aero concentrated 0x957256c98d61a714081de2ee70ae3a96c9b40874 fee 0.97%"), "{text}");
    assert!(text.contains("swap page: https://app.aero.xyz/swap"), "{text}");
    assert!(text.contains("swap page: https://app.uniswap.org/swap"), "{text}");
    assert!(text.contains("Aero returns more NOCK by 0.05% for $20.00 USDC at block 58081910"), "{text}");
    assert!(text.contains("The wallet app's own router may pick a different route."), "{text}");
    assert!(text.contains("it is not a recommendation to buy"), "{text}");
    assert!(!text.contains("aerodrome.finance"));
    // The quotes sit between the plan lines and the LP line.
    assert!(text.find("• Route quotes").unwrap() < text.find("• NOCK LP rebuild bias").unwrap());
}

#[test]
fn test_fallback_unavailable_is_never_a_number_and_absent_is_silent() {
    let down = format_accumulation_plan(&snapshot_with(json!({
        "status": "unavailable", "detail": "no Alchemy RPC key is configured", "amount_usdc": "20.00",
        "block_number": null, "quoted_at": null, "reference_spot_usd": null,
        "venues": [
            { "venue": "Aero", "status": "unavailable", "reason": "no Alchemy RPC key is configured", "swap_url": "https://app.aero.xyz/swap" },
            { "venue": "Uniswap", "status": "unavailable", "reason": "no Alchemy RPC key is configured", "swap_url": "https://app.uniswap.org/swap" }
        ],
        "more_nock_venue": null, "more_nock_by_pct": null,
        "comparison": "Neither venue could be quoted, so there is nothing to compare."
    })));
    assert!(down.contains("Aero: unavailable (no Alchemy RPC key is configured)"), "{down}");
    assert!(down.contains("Uniswap: unavailable (no Alchemy RPC key is configured)"), "{down}");
    assert!(down.contains("Neither venue could be quoted"), "{down}");
    assert!(!down.contains(" NOCK at $"), "{down}");

    // One venue down: the other is still shown, and nobody is said to return more.
    let mut partial = real_quotes();
    partial["venues"][1] = json!({ "venue": "Uniswap", "status": "unavailable", "reason": "the quote request timed out", "swap_url": "https://app.uniswap.org/swap" });
    partial["comparison"] = json!("Only Aero could be quoted at block 58081910; with one quote there is nothing to compare.");
    let text = format_accumulation_plan(&snapshot_with(partial));
    assert!(text.contains("Aero: 1773.0768 NOCK"), "{text}");
    assert!(text.contains("Uniswap: unavailable (the quote request timed out)"), "{text}");
    assert!(!text.contains("returns more NOCK"), "{text}");

    // No quotes in the snapshot: no tranche due, quoting off, or an older worker.
    let none = format_accumulation_plan(&snapshot_with(Value::Null));
    assert!(!none.contains("Route quotes"), "{none}");
}
