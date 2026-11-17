//! stdio MCP binary: the fourth pillar in the snapshot-API fallback text (issue #93).
//! Included from src/bin/mcp.rs. The figures are the ones the worker published for the
//! real wallet at Base block 58077673 (tests_aero_rewards.rs, ONCHAIN_INTEGRATION.md §9).

use super::*;

fn snapshot_with_aero() -> Value {
    json!({
        "positions": [
            { "protocol": "aerodrome", "nft_id": 0, "token0": { "symbol": "USDC" }, "token1": { "symbol": "NOCK" },
              "unclaimed_aero": "0.121026", "unclaimed_aero_usd": "0.05" },
            { "protocol": "aerodrome-cl", "nft_id": 2914490, "token0": { "symbol": "WETH" }, "token1": { "symbol": "NOCK" },
              "unclaimed_aero": "11.922078", "unclaimed_aero_usd": "4.98" },
            { "protocol": "uniswap-v4", "nft_id": 3163450, "token0": { "symbol": "USDC" }, "token1": { "symbol": "NOCK" } }
        ],
        "derived_metrics": { "global_status": {
            "total_portfolio_usd": "2413.57", "total_native_usd": "1312.88", "total_stable_usd": "1033.71",
            "total_eth_usd": "66.41", "wallet_gas_eth": "0.012500", "wallet_gas_usd": "22.99",
            "wallet_aero_amount": "1.359979", "wallet_aero_usd": "0.57",
            "unclaimed_aero_amount": "12.043104", "unclaimed_aero_usd": "5.03", "aero_price_usd": "0.417608",
            "aero_note": null,
            "total_nock_amount": "115000", "nock_floor_target": "0", "nock_floor_delta": "115000",
            "total_usdc_amount": "1033.71", "high_water_mark_usd": "2475.27", "shot_advice_action": "HOLD"
        } }
    })
}

#[test]
fn test_fallback_state_reports_the_four_pillars_and_unclaimed_aero() {
    let text = format_portfolio_state(&snapshot_with_aero());
    assert!(text.contains("• Pillars: NOCK $1312.88 · USDC $1033.71 · ETH $66.41 · AERO $0.57\n"), "{text}");
    assert!(text.contains("• AERO held (wallet): 1.359979 AERO ($0.57), counted in the total; not cash, not deployable, no target\n"), "{text}");
    assert!(text.contains("• Unclaimed AERO: 12.043104 AERO ($5.03) in tracked gauges, not counted in the total\n   - Aero USDC/NOCK: 0.121026 AERO ($0.05)\n   - Aero CL WETH/NOCK #2914490: 11.922078 AERO ($4.98)\n"), "{text}");
    assert!(text.contains("• AERO/USD: $0.417608\n• Collectible earnings (not counted in the total): "), "{text}");
    // The Uniswap position earns no AERO and gets no row.
    assert!(!text.contains("uniswap"), "{text}");
    // Amounts only: nothing tells the reader what to do with AERO.
    for banned in ["sell", "accumulate", "swap", "convert", "should", "recommend"] {
        assert!(!text.to_lowercase().contains(banned), "found '{banned}' in {text}");
    }
}

#[test]
fn test_fallback_state_says_not_reported_for_a_snapshot_without_aero_fields() {
    // An older snapshot (or a worker that predates #93): no AERO field at all.
    let mut old = snapshot_with_aero();
    {
        let g = old["derived_metrics"]["global_status"].as_object_mut().unwrap();
        for k in ["wallet_aero_amount", "wallet_aero_usd", "unclaimed_aero_amount", "unclaimed_aero_usd", "aero_price_usd", "aero_note"] {
            g.remove(k);
        }
    }
    for p in old["positions"].as_array_mut().unwrap() {
        p.as_object_mut().unwrap().remove("unclaimed_aero");
        p.as_object_mut().unwrap().remove("unclaimed_aero_usd");
    }
    let text = format_portfolio_state(&old);
    assert!(text.contains(" · ETH $66.41 · AERO not reported\n"), "{text}");
    assert!(text.contains("• AERO held (wallet): not reported\n• Unclaimed AERO: not reported\n• AERO/USD: not reported\n"), "{text}");
    assert!(!text.contains("AERO $0") && !text.contains("0.000000 AERO"), "{text}");
}

#[test]
fn test_fallback_state_carries_the_workers_reason_for_an_unread_aero_value() {
    let mut data = snapshot_with_aero();
    let g = &mut data["derived_metrics"]["global_status"];
    g["aero_price_usd"] = Value::Null;
    g["unclaimed_aero_usd"] = Value::Null;
    g["wallet_aero_amount"] = json!("0.000000");
    g["wallet_aero_usd"] = json!("0.00");
    g["aero_note"] = json!("AERO/USD price unavailable: Stale AERO/USD price: price pool 0xb27460d0 has zero in-range liquidity");
    for p in data["positions"].as_array_mut().unwrap() {
        p.as_object_mut().unwrap().remove("unclaimed_aero_usd");
    }
    let text = format_portfolio_state(&data);
    assert!(text.contains("• Unclaimed AERO: 12.043104 AERO (USD value not reported) in tracked gauges, not counted in the total\n"), "{text}");
    assert!(text.contains("   - Aero CL WETH/NOCK #2914490: 11.922078 AERO (USD value not reported)\n"), "{text}");
    assert!(text.contains("• AERO/USD: not reported\n• AERO not read: AERO/USD price unavailable: Stale AERO/USD price"), "{text}");
    // A gauge that could not be read is a reason on its row, not an amount.
    data["positions"][0] = json!({ "protocol": "aerodrome", "token0": { "symbol": "USDC" }, "token1": { "symbol": "NOCK" },
        "unclaimed_aero_note": "gauge 0xc82782ee66d82c4d19db0429652091a085083526 earned() reverted" });
    data["derived_metrics"]["global_status"]["unclaimed_aero_amount"] = Value::Null;
    let text = format_portfolio_state(&data);
    assert!(text.contains("• Unclaimed AERO: not reported\n   - Aero USDC/NOCK: not read (gauge 0xc82782ee66d82c4d19db0429652091a085083526 earned() reverted)\n"), "{text}");
}
