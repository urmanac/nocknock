//! stdio MCP binary: collectible gauge rewards in the snapshot-API fallback (issue #142).
//! Included from src/bin/mcp.rs. The figures are the ones the deployed worker published
//! for the real wallet on 2026-11-17 at 11:05Z, eight minutes before the operator's own
//! claim (ONCHAIN_INTEGRATION.md §9): $1.95 of LP fees on two Uniswap positions, and
//! 0.210015 + 17.654015 AERO in the two gauges at $0.438859.

use super::*;

fn reward(amount: &str, usd: &str, gauge: &str) -> Value {
    json!({
        "status": "read", "token_symbol": "AERO", "token_address": "0x3f5bF3EdAc59d650D8fDf9657a71F2f904A3a4Ee",
        "amount": amount, "usd": usd, "gauge": gauge,
        "collected_in": "the Aero app", "collect_url": "https://app.aero.xyz", "note": null
    })
}

const BASIC_GAUGE: &str = "0xc82782ee66d82c4d19db0429652091a085083526";
const CL_GAUGE: &str = "0x3bf70c880adc8370917994ab0df68e763b7a60fd";
const STAKED: &str = "not LP-claimable (staked: swap fees on staked liquidity go to the gauge)";

fn snapshot_with_rewards() -> Value {
    json!({
        "positions": [
            { "protocol": "uniswap-v4", "nft_id": 3163450, "token0": { "symbol": "USDC" }, "token1": { "symbol": "NOCK" } },
            { "protocol": "uniswap-v3", "nft_id": 6336662, "token0": { "symbol": "WETH" }, "token1": { "symbol": "NOCK" } },
            { "protocol": "aerodrome", "nft_id": 0, "token0": { "symbol": "USDC" }, "token1": { "symbol": "NOCK" },
              "unclaimed_aero": "0.210015", "unclaimed_aero_usd": "0.09",
              "collectible_reward": reward("0.210015", "0.09", BASIC_GAUGE) },
            { "protocol": "aerodrome-cl", "nft_id": 2914490, "token0": { "symbol": "WETH" }, "token1": { "symbol": "NOCK" },
              "unclaimed_aero": "17.654015", "unclaimed_aero_usd": "7.75",
              "collectible_reward": reward("17.654015", "7.75", CL_GAUGE) }
        ],
        "contract_fees": [
            { "fee_token_symbol": "USDC+NOCK (Uniswap V4 #3163450)", "amount": "0.154609 USDC + 28.05907500 NOCK",
              "amount_usd": "0.542769", "claimable": true, "gas_break_even_usd": "0.0036" },
            { "fee_token_symbol": "WETH+NOCK (Uniswap V3 #6336662)", "amount": "0.00040500 WETH + 49.31772900 NOCK",
              "amount_usd": "1.407023", "claimable": true, "gas_break_even_usd": "0.0036" },
            { "fee_token_symbol": "USDC+NOCK (Aerodrome)",
              "amount": "not LP-claimable (claimFees() reverts); unclaimed AERO 0.210015 ($0.09)",
              "amount_usd": "0.000000", "claimable": false, "gas_break_even_usd": "0.0036",
              "earnings_kind": "gauge_reward", "reward": reward("0.210015", "0.09", BASIC_GAUGE),
              "lp_fees_note": "not LP-claimable (claimFees() reverts)" },
            { "fee_token_symbol": "WETH+NOCK (Aero CL #2914490)",
              "amount": format!("{STAKED}; unclaimed AERO 17.654015 ($7.75)"),
              "amount_usd": "0.000000", "claimable": false, "gas_break_even_usd": "0.0036",
              "earnings_kind": "gauge_reward", "reward": reward("17.654015", "7.75", CL_GAUGE),
              "lp_fees_note": STAKED }
        ],
        "derived_metrics": { "global_status": {
            "total_portfolio_usd": "2857.78", "total_native_usd": "1506.02", "total_stable_usd": "1001.13",
            "total_eth_usd": "350.63", "wallet_gas_eth": "0.014574", "wallet_gas_usd": "26.09",
            "wallet_aero_amount": "0.000000", "wallet_aero_usd": "0.00",
            "unclaimed_aero_amount": "17.864030", "unclaimed_aero_usd": "7.84", "aero_price_usd": "0.438859",
            "aero_note": null,
            "total_fees_claimable_usd": "1.95", "gas_to_fee_delta_usd": "1.94",
            "collectible_rewards_usd": "7.84", "collectible_rewards_note": null, "total_collectible_usd": "9.79",
            "total_nock_amount": "108943", "nock_floor_target": "100000", "nock_floor_delta": "8943",
            "total_usdc_amount": "1001.13", "high_water_mark_usd": "2857.78",
            "shot_advice_action": "DCA ON SCHEDULE (5 of 5 next)", "shot_advice_details": "Nothing to buy yet."
        } }
    })
}

#[test]
fn test_fallback_texts_state_lp_fees_rewards_and_the_total() {
    let data = snapshot_with_rewards();
    let advice = format_shot_advice(&data);
    assert!(
        advice.contains(
            "• Uncollected Claimable Fees: $1.95 USD\n\
             • Net of two claims' gas: +$1.94 USD\n\
             • Collectible AERO rewards: $7.84 USD (17.864030 AERO in tracked gauges, collected in the Aero app)\n\
             • Total collectible (LP fees + AERO rewards): $9.79 USD\n\
             • Harvest rule: "
        ),
        "{advice}"
    );
    let state = format_portfolio_state(&data);
    assert!(
        state.contains(
            "   - Aero USDC/NOCK: 0.210015 AERO ($0.09), collectible from gauge 0xc82782ee66d82c4d19db0429652091a085083526 in the Aero app\n   \
             - Aero CL WETH/NOCK #2914490: 17.654015 AERO ($7.75), collectible from gauge 0x3bf70c880adc8370917994ab0df68e763b7a60fd in the Aero app\n\
             • AERO/USD: $0.438859\n\
             • Collectible earnings (not counted in the total): LP fees $1.95; AERO rewards $7.84; total collectible $9.79\n\
             • High-Water Mark: $2857.78 USD\n"
        ),
        "{state}"
    );
    // The whole claim simulation: this binary makes no live estimate, and says so.
    assert_eq!(
        format_claim_simulation(&data),
        "⚡ Fee Claim Simulation\n\
         =======================\n\
         • Collectible LP fees: $1.9498 USD\n\
         • Estimated Gas: ~$0.0036 USD\n\
         • Net Accretion if Harvested: $1.9462 USD\n\
         • Collectible AERO rewards: $7.84 USD (17.864030 AERO in tracked gauges, collected in the Aero app)\n\
         • Total collectible (LP fees + AERO rewards, before gas): $9.79 USD\n\
         Pool Breakdown:\n  \
         - USDC+NOCK (Uniswap V4 #3163450): 0.154609 USDC + 28.05907500 NOCK (~$0.542769 USD | Gas Est: $0.0036)\n  \
         - WETH+NOCK (Uniswap V3 #6336662): 0.00040500 WETH + 49.31772900 NOCK (~$1.407023 USD | Gas Est: $0.0036)\n  \
         - USDC+NOCK (Aerodrome): reward 0.210015 AERO ($0.09), collectible from gauge 0xc82782ee66d82c4d19db0429652091a085083526 in the Aero app | gas not estimated (no live estimate was made for this text); LP fees not LP-claimable (claimFees() reverts)\n  \
         - WETH+NOCK (Aero CL #2914490): reward 17.654015 AERO ($7.75), collectible from gauge 0x3bf70c880adc8370917994ab0df68e763b7a60fd in the Aero app | gas not estimated (no live estimate was made for this text); LP fees not LP-claimable (staked: swap fees on staked liquidity go to the gauge)\n"
    );
    // These are the worker's own functions, compiled from the same source file.
    assert!(advice.contains(&collectible_advice_lines(&data)));
    assert!(state.contains(&collectible_earnings_line(&data)));
    assert_eq!(format_claim_simulation(&data), claim_simulation_text(&data, &[]));
}

#[test]
fn test_fallback_reward_lines_state_facts_and_never_advise() {
    let data = snapshot_with_rewards();
    for text in [format_shot_advice(&data), format_portfolio_state(&data), format_claim_simulation(&data)] {
        for line in text.lines().filter(|l| l.contains("AERO") || l.contains("reward")) {
            // The worker's measured reason for the LP-fee part names "swap fees": a fact
            // about where the pool's fees go, not a verb.
            let lower = line.to_lowercase().replace("swap fees on staked liquidity go to the gauge", "");
            for banned in ["should", "worth claiming", "claim now", "claim_now", "claim recommended", "claim_recommended", "recommend", "advis", "accumulate", "sell", "buy", "swap", "convert", "harvest", "hold it", "claim it", "compound"] {
                assert!(!lower.contains(banned), "found '{banned}' in: {line}");
            }
        }
        assert!(!text.contains("aerodrome.finance"), "{text}");
    }
}

#[test]
fn test_fallback_says_not_read_and_not_totalled_never_zero() {
    // The CL gauge could not be read this cycle (as the worker publishes that case).
    let mut data = snapshot_with_rewards();
    let reason = "gauge 0x3bf70c880adc8370917994ab0df68e763b7a60fd earned() reverted";
    let not_read = json!({
        "status": "not_read", "token_symbol": "AERO", "token_address": "0x3f5bF3EdAc59d650D8fDf9657a71F2f904A3a4Ee",
        "amount": null, "usd": null, "gauge": CL_GAUGE,
        "collected_in": "the Aero app", "collect_url": "https://app.aero.xyz", "note": reason
    });
    let p = data["positions"][3].as_object_mut().unwrap();
    p.remove("unclaimed_aero");
    p.remove("unclaimed_aero_usd");
    p.insert("unclaimed_aero_note".to_string(), json!(reason));
    p.insert("collectible_reward".to_string(), not_read.clone());
    data["contract_fees"][3]["reward"] = not_read;
    data["contract_fees"][3]["amount"] = json!(format!("{STAKED}; unclaimed AERO not read ({reason})"));
    let note = format!("gauge reward not read for aerodrome-cl:base:2914490 ({reason})");
    {
        let g = data["derived_metrics"]["global_status"].as_object_mut().unwrap();
        g.insert("unclaimed_aero_amount".to_string(), Value::Null);
        g.insert("unclaimed_aero_usd".to_string(), Value::Null);
        g.insert("collectible_rewards_usd".to_string(), Value::Null);
        g.insert("total_collectible_usd".to_string(), Value::Null);
        g.insert("collectible_rewards_note".to_string(), json!(note));
    }
    let advice = format_shot_advice(&data);
    assert!(advice.contains(&format!("• Collectible AERO rewards: not totalled ({note})\n")), "{advice}");
    assert!(advice.contains("• Total collectible (LP fees + AERO rewards): not totalled (the AERO rewards part is not totalled)\n"), "{advice}");
    // LP fees are still what they were.
    assert!(advice.contains("• Uncollected Claimable Fees: $1.95 USD\n"), "{advice}");
    let claim = format_claim_simulation(&data);
    assert!(claim.contains(&format!("  - WETH+NOCK (Aero CL #2914490): reward not read ({reason}); LP fees {STAKED}\n")), "{claim}");
    assert!(claim.contains("• Collectible LP fees: $1.9498 USD\n"), "{claim}");
    let state = format_portfolio_state(&data);
    assert!(state.contains(&format!("• Collectible earnings (not counted in the total): LP fees $1.95; AERO rewards not totalled ({note})\n")), "{state}");
    for text in [&advice, &claim, &state] {
        // Neither a zero nor the $0.09 of the gauge that was read stands in for the total.
        assert!(!text.contains("AERO rewards: $") && !text.contains("AERO rewards $") && !text.contains("total collectible $"), "{text}");
        assert!(!text.contains("#2914490): reward 0"), "{text}");
    }
}

#[test]
fn test_fallback_prints_a_snapshot_without_reward_fields_as_it_always_did() {
    // A snapshot from a worker that predates #142. It carries its unclaimed AERO ($7.84)
    // on the AERO lines; a "rewards not reported" line beside that figure would
    // contradict it, so the text gains no reward line at all.
    let mut old = snapshot_with_rewards();
    {
        let g = old["derived_metrics"]["global_status"].as_object_mut().unwrap();
        for k in ["collectible_rewards_usd", "collectible_rewards_note", "total_collectible_usd"] {
            g.remove(k);
        }
    }
    for p in old["positions"].as_array_mut().unwrap() {
        p.as_object_mut().unwrap().remove("collectible_reward");
    }
    for row in old["contract_fees"].as_array_mut().unwrap() {
        for k in ["earnings_kind", "reward", "lp_fees_note"] {
            row.as_object_mut().unwrap().remove(k);
        }
    }
    // The same snapshot as a #142 worker serves it after reading it from its database:
    // the keys are present and null.
    let mut reserialized = old.clone();
    {
        let g = reserialized["derived_metrics"]["global_status"].as_object_mut().unwrap();
        for k in ["collectible_rewards_usd", "collectible_rewards_note", "total_collectible_usd"] {
            g.insert(k.to_string(), Value::Null);
        }
    }
    for old in [&old, &reserialized] {
        let advice = format_shot_advice(old);
        assert!(advice.contains("• Net of two claims' gas: +$1.94 USD\n• Harvest rule: "), "{advice}");
        let state = format_portfolio_state(old);
        // The unclaimed AERO it does carry, the position rows and the next line, as before #142.
        assert!(
            state.contains(
                "• Unclaimed AERO: 17.864030 AERO ($7.84) in tracked gauges, not counted in the total\n   \
                 - Aero USDC/NOCK: 0.210015 AERO ($0.09)\n   \
                 - Aero CL WETH/NOCK #2914490: 17.654015 AERO ($7.75)\n\
                 • AERO/USD: $0.438859\n\
                 • High-Water Mark: $2857.78 USD\n"
            ),
            "{state}"
        );
        let claim = format_claim_simulation(old);
        assert!(claim.contains("• Net Accretion if Harvested: $1.9462 USD\nPool Breakdown:\n"), "{claim}");
        assert!(claim.contains(&format!("  - WETH+NOCK (Aero CL #2914490): {STAKED}; unclaimed AERO 17.654015 ($7.75)\n")), "{claim}");
        for text in [&advice, &state, &claim] {
            for new_line in ["Collectible AERO rewards", "Total collectible", "Collectible earnings", "AERO rewards not reported", "\n\n"] {
                assert!(!text.contains(new_line), "'{new_line}' in: {text}");
            }
        }
        assert_eq!((collectible_advice_lines(old), collectible_earnings_line(old)), (String::new(), String::new()));
    }
    // Nothing at all in the snapshot: no reward line, and never a zero in its place.
    let empty = format_claim_simulation(&json!({}));
    assert!(!empty.contains("AERO") && empty.contains("• Net Accretion if Harvested: unknown (live gas estimate unavailable)\nPool Breakdown:\n"), "{empty}");
    let empty_advice = format_shot_advice(&json!({}));
    assert!(!empty_advice.contains("AERO") && !empty_advice.contains("0.00"), "{empty_advice}");
}

#[test]
fn test_fallback_states_a_read_zero_as_nothing_to_collect() {
    // The operator's CL gauge after the claim of 2026-11-17 (block 58618046): earned() is 0.
    let mut data = snapshot_with_rewards();
    let zero = reward("0.000000", "0.00", CL_GAUGE);
    let p = data["positions"][3].as_object_mut().unwrap();
    p.insert("unclaimed_aero".to_string(), json!("0.000000"));
    p.insert("unclaimed_aero_usd".to_string(), json!("0.00"));
    p.insert("collectible_reward".to_string(), zero.clone());
    data["contract_fees"][3]["reward"] = zero;
    let state = format_portfolio_state(&data);
    assert!(state.contains("   - Aero CL WETH/NOCK #2914490: 0.000000 AERO ($0.00), nothing to collect in gauge 0x3bf70c880adc8370917994ab0df68e763b7a60fd\n"), "{state}");
    let claim = format_claim_simulation(&data);
    assert!(
        claim.contains(&format!("  - WETH+NOCK (Aero CL #2914490): reward 0.000000 AERO ($0.00), nothing to collect in gauge 0x3bf70c880adc8370917994ab0df68e763b7a60fd; LP fees {STAKED}\n")),
        "{claim}"
    );
    // The gauge that does hold a reward still reads as collectible.
    assert!(state.contains("   - Aero USDC/NOCK: 0.210015 AERO ($0.09), collectible from gauge 0xc82782ee66d82c4d19db0429652091a085083526 in the Aero app\n"), "{state}");
    for text in [&state, &claim] {
        assert!(!text.contains("($0.00), collectible"), "a zero is not described as collectible: {text}");
    }
}
