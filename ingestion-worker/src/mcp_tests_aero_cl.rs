//! stdio MCP binary: fee rows of an Aero CL position (issue #91). Included from
//! src/bin/mcp.rs; the strings are the ones the worker emits (adapters/aero_cl.rs).

use super::*;

const STAKED: &str =
    "not LP-claimable (staked: swap fees on staked liquidity go to the gauge); unclaimed AERO 11.922078 ($4.98)";

fn staked_row() -> Value {
    json!({ "fee_token_symbol": "WETH+NOCK (Aero CL #2914490)", "amount": STAKED,
            "amount_usd": "0.000000", "claimable": false, "gas_break_even_usd": "0.0041" })
}

#[test]
fn test_fallback_claim_text_states_why_an_aero_cl_position_has_no_fee_figure() {
    let data = json!({ "contract_fees": [
        staked_row(),
        { "fee_token_symbol": "USDC+NOCK (Uniswap V4 #1)", "amount": "0.1 USDC + 2 NOCK",
          "amount_usd": "0.12", "claimable": true, "gas_break_even_usd": "0.0014" }
    ] });
    let text = format_claim_simulation(&data);
    // The reason is printed in place of a figure: no "$0.000000" and no gas for it.
    assert!(text.contains(&format!("  - WETH+NOCK (Aero CL #2914490): {STAKED}\n")), "{text}");
    assert!(!text.contains("0.000000"), "{text}");
    // Gas and totals come only from the row that can be claimed.
    assert!(text.contains("Estimated Gas: ~$0.0014 USD"), "{text}");
    assert!(text.contains("Total Gross Claimable: $0.1200 USD"), "{text}");
}

#[test]
fn test_fallback_claim_text_with_nothing_lp_claimable_does_not_blame_the_gas_estimate() {
    // Only a staked Aero CL position: gas was read (0.0041 on the row), but there is
    // no claim to cost. Said as such, not as "live gas estimate unavailable".
    let text = format_claim_simulation(&json!({ "contract_fees": [staked_row()] }));
    assert!(text.contains("• Estimated Gas: n/a (nothing LP-claimable)\n"), "{text}");
    assert!(text.contains("• Net Accretion if Harvested: n/a (nothing LP-claimable)\n"), "{text}");
    assert!(!text.contains("unavailable"), "{text}");
    // No rows at all, or a row whose gas could not be read, keep the old wording.
    let text = format_claim_simulation(&json!({ "contract_fees": [] }));
    assert!(text.contains("• Estimated Gas: unavailable\n"), "{text}");
    let text = format_claim_simulation(&json!({ "contract_fees": [
        staked_row(),
        { "fee_token_symbol": "USDC+NOCK (Uniswap V4 #1)", "amount": "0.1 USDC + 2 NOCK",
          "amount_usd": "0.12", "claimable": true, "gas_break_even_usd": "" }
    ] }));
    assert!(text.contains("• Net Accretion if Harvested: unknown (live gas estimate unavailable)\n"), "{text}");
}
