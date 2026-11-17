//! stdio MCP binary: closed positions in the snapshot-API fallback (issue #144).
//! Included from src/bin/mcp.rs. The snapshots are the two the worker's own tests pin
//! in testdata/golden_snapshot_closed.json: the tracked wallet's real book at Base
//! block 58646285 with the withdrawn Aero CL NFT #2914490 still configured, and the
//! same book with its Uniswap v4 price-source entry pointed at a token id that does
//! not exist. The fallback must print what the worker's HTTP transport prints.

use super::*;

fn golden() -> Value {
    serde_json::from_str(include_str!("../testdata/golden_snapshot_closed.json")).expect("the golden file parses")
}

#[test]
fn test_fallback_prints_the_closed_positions_the_worker_prints() {
    let golden = golden();
    for (case, expected_line) in [
        (
            "aero_cl_withdrawn_block_58646285",
            "   ✗ Aero CL #2914490 (aero-cl200-weth-nock): closed. NFT #2914490 does not exist on position manager 0x4857db576ee0e45f10525ec59527bfd10beec60b: withdrawn and burned, or a mistyped nft_id. Not counted in any total. You can remove its entry (aero-cl200-weth-nock) from config.yaml.\n",
        ),
        (
            "v4_price_source_closed_block_58646285",
            "   ✗ v4 USDC/NOCK #3229273 (uniswap-v4-nock-usdc-gone): closed. NFT #3229273 does not exist on position manager 0x7c5f5a4bbd8fd63184577525326123b519429bdc: withdrawn and burned, or a mistyped nft_id. Not counted in any total. Its pool still supplies the NOCK/USD spot: flag another position as is_price_source before removing its entry from config.yaml.\n",
        ),
    ] {
        let data = &golden[case]["snapshot"];
        let worker = &golden[case]["mcp_text"];
        let block = format!("• Closed positions (1), not counted in any total:\n{expected_line}");
        assert_eq!(closed_positions_lines(data), block, "{case}");

        // get_portfolio_state and simulate_claim_fees: byte for byte the worker's text.
        let state = format_portfolio_state(data);
        assert_eq!(state, worker["get_portfolio_state"].as_str().unwrap(), "{case}");
        assert!(state.contains(&block), "{case}: {state}");
        let claim = format_claim_simulation(data);
        assert_eq!(claim, worker["simulate_claim_fees"].as_str().unwrap(), "{case}");
        assert!(claim.ends_with(&block), "{case}: {claim}");
        // The directive text is the worker's own and says nothing about the closing.
        let advice = format_shot_advice(data);
        assert_eq!(advice, worker["get_shot_advice"].as_str().unwrap(), "{case}");
        assert!(!advice.to_lowercase().contains("closed"), "{case}: {advice}");

        // A closed position is stated, never valued: no "$" on its line, and it is in
        // no row of the pool breakdown.
        assert!(!expected_line.contains('$'));
        let breakdown = claim.split("Pool Breakdown:\n").nth(1).unwrap().split("• Closed positions").next().unwrap();
        let id = data["closed_positions"][0]["nft_id"].as_u64().unwrap().to_string();
        assert!(!breakdown.contains(&id), "{case}: {breakdown}");
    }
}

#[test]
fn test_fallback_text_is_unchanged_for_a_snapshot_without_closed_positions() {
    let golden = golden();
    let with = golden["aero_cl_withdrawn_block_58646285"]["snapshot"].clone();
    // The same snapshot as a worker without a closed position publishes it: no list,
    // and no count or note in global_status (they are absent, not null).
    let mut without = with.clone();
    without.as_object_mut().unwrap().remove("closed_positions");
    let gs = without["derived_metrics"]["global_status"].as_object_mut().unwrap();
    gs.remove("closed_positions_count");
    gs.remove("closed_positions_note");
    assert_eq!(closed_positions_lines(&without), "");
    let block = closed_positions_lines(&with);
    assert_eq!(format_portfolio_state(&with).replace(&block, ""), format_portfolio_state(&without));
    assert_eq!(format_claim_simulation(&with).replace(&block, ""), format_claim_simulation(&without));
    assert!(!format_portfolio_state(&without).contains("Closed positions"));
    // An empty or null list prints nothing either.
    for empty in [json!([]), Value::Null] {
        let mut data = without.clone();
        data["closed_positions"] = empty;
        assert_eq!(closed_positions_lines(&data), "");
    }
    // An entry without the worker's sentence (never produced; a hand-made payload) still
    // states the fact from the id and invents nothing else.
    let bare = json!({ "closed_positions": [{ "config_id": "x", "nft_id": 7 }] });
    assert_eq!(
        closed_positions_lines(&bare),
        "• Closed positions (1), not counted in any total:\n   ✗ x: closed. NFT #7 no longer exists (withdrawn and burned, or a mistyped nft_id). Not counted in any total. You can remove its entry (x) from config.yaml.\n"
    );
}
