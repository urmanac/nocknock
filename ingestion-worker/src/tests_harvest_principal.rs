//! #156: record-harvest must not book returned liquidity (principal) as fee income.
//!
//! - `testdata/harvest/v3_withdraw_6336662.json` is the real receipt of the operator's
//!   withdrawal of Uniswap v3 WETH/NOCK #6336662 (tx 0x06515a23…b778, block 58831000):
//!   decreaseLiquidity + collect through the NPM, WETH unwrapped and paid as ETH.
//! - `testdata/harvest/v3_positions_6336662_before_withdraw.json` is that NPM's
//!   `positions(6336662)` answer at block 58830999, as the handler reads it.
use super::*;
use serde_json::json;

const WALLET: &str = "0xeE114cC97eC5adeB34bA5645e315B497D8487a45";
const RECEIPT: &str = include_str!("../testdata/harvest/v3_withdraw_6336662.json");
const POSITIONS_BEFORE: &str = include_str!("../testdata/harvest/v3_positions_6336662_before_withdraw.json");

fn receipt() -> serde_json::Value {
    serde_json::from_str(RECEIPT).expect("withdraw receipt parses")
}

fn pair_before() -> (String, String) {
    let v: serde_json::Value = serde_json::from_str(POSITIONS_BEFORE).expect("positions fixture parses");
    let p = decode_v3_positions_return(v["result"].as_str().unwrap()).expect("positions() decodes");
    (p.token0, p.token1)
}

fn rows(r: &serde_json::Value) -> Vec<ParsedHarvestTransfer> {
    merge_harvest_transfers(parse_harvest_transfers(r, WALLET)).expect("rows merge")
}

fn amount(rows: &[ParsedHarvestTransfer], symbol: &str) -> Option<String> {
    rows.iter().find(|r| r.token_symbol == symbol).map(|r| r.amount_raw.clone())
}

fn word(n: u128) -> String {
    format!("{n:064x}")
}

fn topic_addr(a: &str) -> String {
    format!("0x000000000000000000000000{}", a.trim_start_matches("0x").to_lowercase())
}

fn log(address: &str, topics: Vec<String>, words: &[String]) -> serde_json::Value {
    json!({ "address": address, "topics": topics, "data": format!("0x{}", words.concat()) })
}

fn receipt_with(logs: Vec<serde_json::Value>) -> serde_json::Value {
    json!({ "status": "0x1", "from": WALLET, "blockNumber": "0x10", "logs": logs })
}

fn decrease_log(manager: &str, token_id: u128, a0: u128, a1: u128) -> serde_json::Value {
    log(
        manager,
        vec![DECREASE_LIQUIDITY_TOPIC.to_string(), format!("0x{}", word(token_id))],
        &[word(1), word(a0), word(a1)],
    )
}

fn row(symbol: &str, token: &str, amount: u128, decimals: i32) -> ParsedHarvestTransfer {
    ParsedHarvestTransfer {
        token_address: token.to_string(),
        token_symbol: symbol.to_string(),
        from_address: UNISWAP_V3_NPM.to_lowercase(),
        recipient: WALLET.to_string(),
        amount_raw: amount.to_string(),
        amount_human: format_units(amount, decimals),
        decimals,
    }
}

// ---------------------------------------------------------------- the real withdrawal

#[test]
fn test_real_withdrawal_today_books_principal_as_fees() {
    // The defect: what the parser alone books for the withdrawal.
    let r = rows(&receipt());
    assert_eq!(amount(&r, "ETH").as_deref(), Some("103325038812771257"));
    assert_eq!(amount(&r, "NOCK").as_deref(), Some("10068107081028485540"));
    assert_eq!(r.len(), 2);
}

#[test]
fn test_real_withdrawal_scan_reads_one_decrease_through_the_v3_npm() {
    let d = scan_liquidity_decreases(&receipt()).expect("separable");
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].manager, UNISWAP_V3_NPM_BASE);
    assert_eq!(d[0].token_id_word, word(6336662));
    assert_eq!(d[0].amount0, 102325535439356415);
    assert_eq!(d[0].amount1, 9052303512140725855);
    assert_eq!(positions_calldata(&d[0]), format!("0x99fbab88{}", word(6336662)));
}

#[test]
fn test_real_withdrawal_positions_before_give_weth_and_nock() {
    let (t0, t1) = pair_before();
    assert!(t0.eq_ignore_ascii_case(WETH_BASE));
    assert!(t1.eq_ignore_ascii_case(NOCK_BASE));
}

#[test]
fn test_real_withdrawal_books_only_the_fees() {
    let r = receipt();
    let d = scan_liquidity_decreases(&r).unwrap();
    let principal = principal_by_token(&d, &[pair_before()]).unwrap();
    let split = subtract_principal(rows(&r), &principal).expect("split");
    // Collect − DecreaseLiquidity, per token: 0.000999503373414842 WETH (paid as ETH)
    // and 101.5803568887759685 NOCK.
    assert_eq!(amount(&split.fees, "ETH").as_deref(), Some("999503373414842"));
    assert_eq!(amount(&split.fees, "NOCK").as_deref(), Some("1015803568887759685"));
    let eth = split.fees.iter().find(|f| f.token_symbol == "ETH").unwrap();
    assert_eq!(eth.amount_human, "0.000999503373414842");
    let nock = split.fees.iter().find(|f| f.token_symbol == "NOCK").unwrap();
    assert_eq!(nock.amount_human, "101.5803568887759685");
    // The principal left out is reported, per row.
    assert_eq!(amount(&split.principal, "ETH").as_deref(), Some("102325535439356415"));
    assert_eq!(amount(&split.principal, "NOCK").as_deref(), Some("9052303512140725855"));
    // Fees + principal = what the wallet received; nothing is lost or invented.
    for sym in ["ETH", "NOCK"] {
        let total: u128 = amount(&rows(&r), sym).unwrap().parse().unwrap();
        let f: u128 = amount(&split.fees, sym).unwrap().parse().unwrap();
        let p: u128 = amount(&split.principal, sym).unwrap().parse().unwrap();
        assert_eq!(f + p, total, "{sym}");
    }
}

#[test]
fn test_block_before_tag() {
    assert_eq!(block_before_tag(58831000).as_deref(), Some("0x381b097"));
    assert_eq!(block_before_tag(0), None);
    assert_eq!(block_before_tag(-1), None);
}

// ---------------------------------------------------------------- fee claims unchanged

#[test]
fn test_fee_only_claims_have_no_principal() {
    // A v3 collect-only claim: Collect from the NPM and a token Transfer, no decrease.
    let collect = log(
        UNISWAP_V3_NPM,
        vec![
            "0x40d0efd1a53d60ecbf40971b9daf7dc90178c3aadc7aab1765632738fa8b8f01".to_string(),
            format!("0x{}", word(6336662)),
        ],
        &[word(0), word(5), word(7)],
    );
    assert_eq!(scan_liquidity_decreases(&receipt_with(vec![collect])), Ok(vec![]));
    // A v4 fee claim: a zero-delta ModifyLiquidity.
    let v4 = log(
        "0x498581ff718922c3f8e6a244956af099b2652b2b",
        vec![V4_MODIFY_LIQUIDITY_TOPIC.to_string(), format!("0x{}", word(1)), topic_addr(WALLET)],
        &[word(0), word(0), word(0), word(0)],
    );
    assert_eq!(scan_liquidity_decreases(&receipt_with(vec![v4])), Ok(vec![]));
    // A CL pool "poke" Burn of zero liquidity by any owner releases nothing.
    let poke = log(
        "0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a",
        vec![CL_POOL_BURN_TOPIC.to_string(), topic_addr("0x1111111111111111111111111111111111111111"), format!("0x{}", word(1)), format!("0x{}", word(2))],
        &[word(0), word(0), word(0)],
    );
    assert_eq!(scan_liquidity_decreases(&receipt_with(vec![poke])), Ok(vec![]));
    // The real Aero gauge claim used by the #93 tests.
    assert_eq!(scan_liquidity_decreases(&super::tests_aero_rewards::real_claim_receipt()), Ok(vec![]));
}

#[test]
fn test_no_principal_leaves_rows_unchanged() {
    let r = vec![row("USDC", USDC_BASE, 1_234_567, 6), row("NOCK", NOCK_BASE, 5, 16)];
    let split = subtract_principal(r.clone(), &[]).unwrap();
    assert_eq!(split.fees, r);
    assert!(split.principal.is_empty());
}

// ---------------------------------------------------------------- refusals

#[test]
fn test_v4_liquidity_removal_is_refused() {
    let mut neg = "f".repeat(63);
    neg.push('0'); // a negative int256
    let v4 = log(
        "0x498581ff718922c3f8e6a244956af099b2652b2b",
        vec![V4_MODIFY_LIQUIDITY_TOPIC.to_string(), format!("0x{}", word(1)), topic_addr(WALLET)],
        &[word(0), word(0), neg, word(0)],
    );
    let e = scan_liquidity_decreases(&receipt_with(vec![v4])).unwrap_err();
    assert_eq!(e.code, "PRINCIPAL_NOT_SEPARABLE");
    assert!(e.detail.contains("Uniswap v4"), "{}", e.detail);
}

#[test]
fn test_full_range_burns_are_refused() {
    for topic in [AERO_BASIC_BURN_TOPIC, V2_POOL_BURN_TOPIC] {
        let burn = log(
            "0x770fe8fc9248c6382950a86e39a9391b41d59e0a",
            vec![topic.to_string(), topic_addr(WALLET), topic_addr(WALLET)],
            &[word(10), word(20)],
        );
        let e = scan_liquidity_decreases(&receipt_with(vec![burn])).unwrap_err();
        assert_eq!(e.code, "PRINCIPAL_NOT_SEPARABLE");
        assert!(e.detail.contains("full-range"), "{}", e.detail);
    }
}

#[test]
fn test_cl_burn_for_an_unknown_owner_is_refused() {
    let burn = log(
        "0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a",
        vec![CL_POOL_BURN_TOPIC.to_string(), topic_addr("0x827922686190790b37229fd06084350E74485b72"), format!("0x{}", word(1)), format!("0x{}", word(2))],
        &[word(5), word(10), word(0)],
    );
    let e = scan_liquidity_decreases(&receipt_with(vec![burn])).unwrap_err();
    assert_eq!(e.code, "PRINCIPAL_NOT_SEPARABLE");
    // The same Burn owned by a known manager is read from its DecreaseLiquidity.
    let known = log(
        "0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a",
        vec![CL_POOL_BURN_TOPIC.to_string(), topic_addr(AERO_CL_POSITION_MANAGER_BASE), format!("0x{}", word(1)), format!("0x{}", word(2))],
        &[word(5), word(10), word(0)],
    );
    let d = scan_liquidity_decreases(&receipt_with(vec![known, decrease_log(AERO_CL_POSITION_MANAGER_BASE, 3449558, 10, 0)])).unwrap();
    assert_eq!(d.len(), 1);
    assert_eq!(d[0].manager, AERO_CL_POSITION_MANAGER_BASE);
}

#[test]
fn test_decrease_from_an_unknown_manager_is_not_read_as_principal() {
    let d = scan_liquidity_decreases(&receipt_with(vec![decrease_log("0x827922686190790b37229fd06084350E74485b72", 1, 10, 10)])).unwrap();
    assert!(d.is_empty());
}

#[test]
fn test_unreadable_decrease_is_refused() {
    let short = log(UNISWAP_V3_NPM, vec![DECREASE_LIQUIDITY_TOPIC.to_string(), format!("0x{}", word(1))], &[word(1), word(2)]);
    assert_eq!(scan_liquidity_decreases(&receipt_with(vec![short])).unwrap_err().code, "PRINCIPAL_NOT_SEPARABLE");
    let huge = log(
        UNISWAP_V3_NPM,
        vec![DECREASE_LIQUIDITY_TOPIC.to_string(), format!("0x{}", word(1))],
        &[word(1), "1".repeat(64), word(2)],
    );
    assert_eq!(scan_liquidity_decreases(&receipt_with(vec![huge])).unwrap_err().code, "PRINCIPAL_NOT_SEPARABLE");
}

#[test]
fn test_principal_exceeding_the_inflow_is_refused() {
    // 100 NOCK principal but the wallet received only 60: paid elsewhere or swapped.
    let p = [PrincipalAmount { token_address: NOCK_BASE.to_lowercase(), amount_raw: 100 }];
    let e = subtract_principal(vec![row("NOCK", NOCK_BASE, 60, 16)], &p).unwrap_err();
    assert_eq!(e.code, "PRINCIPAL_EXCEEDS_INFLOW");
    // Principal of a booked token with no row at all is refused too.
    let e = subtract_principal(vec![row("USDC", USDC_BASE, 60, 6)], &p).unwrap_err();
    assert_eq!(e.code, "PRINCIPAL_EXCEEDS_INFLOW");
}

#[test]
fn test_mismatched_pairs_are_refused() {
    let d = vec![LiquidityDecrease { manager: UNISWAP_V3_NPM_BASE.to_string(), token_id_word: word(1), amount0: 1, amount1: 1 }];
    assert_eq!(principal_by_token(&d, &[]).unwrap_err().code, "PRINCIPAL_NOT_SEPARABLE");
}

// ---------------------------------------------------------------- arithmetic

#[test]
fn test_principal_of_an_untracked_token_is_ignored() {
    let cbbtc = "0xcbb7c0000ab88b473b1f5afd9ef808440eed33bf";
    let p = [PrincipalAmount { token_address: cbbtc.to_string(), amount_raw: 5 }];
    let r = vec![row("USDC", USDC_BASE, 9, 6)];
    let split = subtract_principal(r.clone(), &p).unwrap();
    assert_eq!(split.fees, r);
    assert!(split.principal.is_empty());
}

#[test]
fn test_weth_principal_comes_from_weth_first_then_eth() {
    let p = [PrincipalAmount { token_address: WETH_BASE.to_lowercase(), amount_raw: 70 }];
    let r = vec![row("ETH", NATIVE_ETH_LEDGER_ADDRESS, 50, 18), row("WETH", WETH_BASE, 40, 18)];
    let split = subtract_principal(r, &p).unwrap();
    // 40 from the WETH row (gone), 30 from the ETH row (20 left).
    assert_eq!(amount(&split.fees, "WETH"), None);
    assert_eq!(amount(&split.fees, "ETH").as_deref(), Some("20"));
    assert_eq!(amount(&split.principal, "WETH").as_deref(), Some("40"));
    assert_eq!(amount(&split.principal, "ETH").as_deref(), Some("30"));
}

#[test]
fn test_principal_only_leaves_no_fee_rows() {
    let p = [PrincipalAmount { token_address: USDC_BASE.to_lowercase(), amount_raw: 9 }];
    let split = subtract_principal(vec![row("USDC", USDC_BASE, 9, 6)], &p).unwrap();
    assert!(split.fees.is_empty());
    assert_eq!(amount(&split.principal, "USDC").as_deref(), Some("9"));
}

#[test]
fn test_two_decreases_sum_per_token_and_aero_is_untouched() {
    let d = vec![
        LiquidityDecrease { manager: AERO_CL_POSITION_MANAGER_BASE.to_string(), token_id_word: word(1), amount0: 3, amount1: 0 },
        LiquidityDecrease { manager: UNISWAP_V3_NPM_BASE.to_string(), token_id_word: word(2), amount0: 4, amount1: 7 },
    ];
    let pairs = vec![
        (WETH_BASE.to_string(), USDC_BASE.to_string()),
        (WETH_BASE.to_uppercase().replace("0X", "0x"), NOCK_BASE.to_string()),
    ];
    let p = principal_by_token(&d, &pairs).unwrap();
    assert_eq!(p, vec![
        PrincipalAmount { token_address: WETH_BASE.to_lowercase(), amount_raw: 7 },
        PrincipalAmount { token_address: NOCK_BASE.to_lowercase(), amount_raw: 7 },
    ]);
    let r = vec![row("WETH", WETH_BASE, 10, 18), row("NOCK", NOCK_BASE, 7, 16), row("AERO", AERO_BASE, 99, 18)];
    let split = subtract_principal(r, &p).unwrap();
    assert_eq!(amount(&split.fees, "WETH").as_deref(), Some("3"));
    assert_eq!(amount(&split.fees, "NOCK"), None);
    assert_eq!(amount(&split.fees, "AERO").as_deref(), Some("99"));
}

#[test]
fn test_the_decrease_and_burn_topics_match_the_real_receipt() {
    let topics: Vec<String> = receipt()["logs"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| l["topics"][0].as_str().unwrap().to_string())
        .collect();
    assert!(topics.iter().any(|t| t == DECREASE_LIQUIDITY_TOPIC));
    assert!(topics.iter().any(|t| t == CL_POOL_BURN_TOPIC));
}
