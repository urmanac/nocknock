//! stdio MCP binary: fee rows of an Aero CL position (issue #91). Included from
//! src/bin/mcp.rs; the strings are the ones the worker emits (adapters/aero_cl.rs).

use super::*;

#[test]
fn test_fallback_claim_text_states_why_an_aero_cl_position_has_no_fee_figure() {
    let staked = "not LP-claimable (staked: swap fees on staked liquidity go to the gauge); AERO rewards not read (#93)";
    let unread = "fees unavailable (collect() simulation from the wallet reverted); not counted";
    let data = json!({ "contract_fees": [
        { "fee_token_symbol": "WETH+NOCK (Aero CL #2914490)", "amount": staked,
          "amount_usd": "0.000000", "claimable": false, "gas_break_even_usd": "0.0041" },
        { "fee_token_symbol": "WETH+NOCK (Aero CL #7)", "amount": unread,
          "amount_usd": "0.000000", "claimable": false, "gas_break_even_usd": "0.0041" },
        { "fee_token_symbol": "USDC+NOCK (Uniswap V4 #1)", "amount": "0.1 USDC + 2 NOCK",
          "amount_usd": "0.12", "claimable": true, "gas_break_even_usd": "0.0014" }
    ] });
    let text = format_claim_simulation(&data);
    // The reason is printed in place of a figure: no "$0.000000" and no gas for either.
    assert!(text.contains(&format!("  - WETH+NOCK (Aero CL #2914490): {staked}\n")), "{text}");
    assert!(text.contains(&format!("  - WETH+NOCK (Aero CL #7): {unread}\n")), "{text}");
    assert!(!text.contains("0.000000"), "{text}");
    // Gas and totals come only from the row that can be claimed.
    assert!(text.contains("Estimated Gas: ~$0.0014 USD"), "{text}");
    assert!(text.contains("Total Gross Claimable: $0.1200 USD"), "{text}");
}
