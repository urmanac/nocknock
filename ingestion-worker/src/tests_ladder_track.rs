//! A deployed ladder tracked as `role: ladder` (#167).
//!
//! The ladder rung here is a real Uniswap v4 mint into the USDC/NOCK 3% pool, recorded from
//! Base on 2026-11-20 with eth_getTransactionReceipt (testdata/ladder/receipt_v4_mint_*.json):
//! tx 0xafe42fe2…7a58, block 59660587 (2026-11-19T21:08:13Z), NFT #3240369, ticks
//! 270900..887100, liquidity 934864722135742, paid 1221.851161 USDC + 247.5789900788685019
//! NOCK. It sits in the pool of the tracked v4 range (#3163450) at that fixture's recorded
//! pool state. Also recorded: a zap that mints and swaps in one transaction (0x11347d28…,
//! refused), and the operator's own v3 WETH/NOCK IncreaseLiquidity (0xe59b6dc7…). Nothing
//! here touches the network, a database or a config.yaml.
use super::*;
use crate::tests_gas_lp::{nock, nock_only, nock_usdc_wide, profile, sqrt_x96_at_tick, usdc, weth, weth_nock_v3, with_amounts};

/// The wallet that sent the recorded v4 mint (public chain data).
pub(crate) const MINTER: &str = "0x4598b3ba7d6714091b18da5d110b64f56e94e826";
pub(crate) const MINT_TX: &str = "0xafe42fe29de67337d5b489226dbfdf8a8b315e2deabaf6c744b47c2dd9ff7a58";
pub(crate) const MINT_L: u128 = 934_864_722_135_742;
pub(crate) const MINT_USDC_RAW: u128 = 1_221_851_161;
pub(crate) const MINT_NOCK_RAW: u128 = 2_475_789_900_788_685_019;
pub(crate) const OPERATOR: &str = "0xee114cc97ec5adeb34ba5645e315b497d8487a45";
const V3_INCREASE_TX: &str = "0xe59b6dc73cadf94daf4d882221b528cd25c9c968320e26c642ca6fdd56c1ef3d";

fn recorded(name: &str) -> serde_json::Value {
    let text = match name {
        "v4_mint" => include_str!("../testdata/ladder/receipt_v4_mint_0xafe42fe2.json"),
        "v4_zap" => include_str!("../testdata/ladder/receipt_v4_zap_mint_swap_0x11347d28.json"),
        "v3_increase" => include_str!("../testdata/ladder/receipt_v3_increase_0xe59b6dc7.json"),
        other => panic!("no fixture {other}"),
    };
    serde_json::from_str::<serde_json::Value>(text).unwrap()["result"].clone()
}

fn block_header(block_hex: &str) -> serde_json::Value {
    let all: serde_json::Value = serde_json::from_str(include_str!("../testdata/ladder/block_headers.json")).unwrap();
    all[block_hex].clone()
}

/// The recorded rung #3240369 at a pool tick (and that tick's sqrt price). At the tracked
/// range's recorded tick (269065) the price is above the rung: it holds only USDC.
pub(crate) fn ladder_rung_at(tick: i32) -> RawPosition {
    let src = nock_usdc_wide();
    let sqrt = if tick == src.tick { src.sqrt_price_x96 } else { sqrt_x96_at_tick(tick) };
    with_amounts(RawPosition {
        position_id: "uniswap-v4:base:3240369".to_string(),
        network: "base".to_string(),
        protocol: "uniswap-v4".to_string(),
        wallet: OPERATOR.to_string(),
        nft_id: 3240369,
        pool_id: src.pool_id.clone(),
        npm_address: None,
        fee_tier_bps: 300,
        token0: usdc(),
        token1: nock(),
        liquidity: MINT_L,
        sqrt_price_x96: sqrt,
        tick,
        tick_lower: 270900,
        tick_upper: 887100,
        amount0: 0.0,
        amount1: 0.0,
        fee0: 0.0,
        fee1: 0.0,
        in_range: tick_in_range(tick, 270900, 887100),
        is_price_source: false,
    })
}

/// A synthetic deep rung in the same pool: ticks 276000..280200 (spacing 300), far below.
pub(crate) fn deep_rung() -> RawPosition {
    let src = nock_usdc_wide();
    with_amounts(RawPosition {
        position_id: "uniswap-v4:base:3299001".to_string(),
        nft_id: 3299001,
        liquidity: 400_000_000_000_000,
        tick_lower: 276000,
        tick_upper: 280200,
        in_range: false,
        is_price_source: false,
        ..src
    })
}

pub(crate) fn tag(id: &str, rung: Option<&str>, tx: Option<&str>) -> LadderTag {
    LadderTag { config_id: id.to_string(), rung: rung.map(str::to_string), deposit_tx: tx.map(str::to_string) }
}

/// The verified baseline of the recorded mint, as the cycle would hold it.
pub(crate) fn verified_mint() -> LadderBaselineRead {
    let mut d = parse_ladder_deposit(&recorded("v4_mint"), MINTER, &ladder_rung_at(nock_usdc_wide().tick)).expect("the recorded mint verifies");
    d.at = block_time_from_response(&json!({ "result": block_header("0x38e592b") }));
    LadderBaselineRead::Verified(d)
}

/// The tracked NOCK book (#3163450 price source, v3 WETH/NOCK) plus the recorded rung
/// (near, verified deposit) and the deep rung (rung unset, no deposit_tx).
pub(crate) fn ladder_book() -> RawOnChainData {
    let mut raw = nock_only();
    let near = ladder_rung_at(raw.positions[0].tick);
    let deep = deep_rung();
    for p in [&near, &deep] {
        raw.pool_liquidity.insert(p.position_id.clone(), "80173935548453779".to_string());
    }
    // The 3% pool's tick spacing (300), read for v4 positions since #166.
    for id in [raw.positions[0].position_id.clone(), near.position_id.clone(), deep.position_id.clone()] {
        raw.position_facts.insert(id, PositionFacts { tick_spacing: Some(300), ..Default::default() });
    }
    raw.ladders.insert(near.position_id.clone(), tag("ladder-near", Some("near"), Some(MINT_TX)));
    raw.ladders.insert(deep.position_id.clone(), tag("ladder-deep", None, None));
    raw.ladder_baselines.insert(near.position_id.clone(), verified_mint());
    raw.ladder_baselines.insert(deep.position_id.clone(), LadderBaselineRead::NotConfigured);
    raw.positions.push(near);
    raw.positions.push(deep);
    raw
}

/// The book with the 3% pool at another tick: the price source and both rungs move
/// together (one pool, one price), and spot with them; the v3 WETH/NOCK pool does not.
pub(crate) fn ladder_book_at(tick: i32) -> RawOnChainData {
    let mut raw = ladder_book();
    let pool = nock_usdc_wide().pool_id;
    for p in raw.positions.iter_mut().filter(|p| p.pool_id == pool) {
        *p = with_amounts(RawPosition {
            tick,
            sqrt_price_x96: sqrt_x96_at_tick(tick),
            in_range: tick_in_range(tick, p.tick_lower, p.tick_upper),
            ..p.clone()
        });
    }
    raw.spot_price_nock_usd = nock_spot_from_positions(&raw.positions).expect("price source");
    raw
}

/// The same positions with no role: every one of them is NOCK LP.
fn untagged(mut raw: RawOnChainData) -> RawOnChainData {
    raw.ladders.clear();
    raw.ladder_baselines.clear();
    raw
}

pub(crate) fn reserve_profile() -> OperatorProfile {
    OperatorProfile { cash_reserve_usdc: Some(950.0), ..profile() }
}

fn metrics_with(raw: &RawOnChainData, profile: &OperatorProfile, plan: &PlanContext) -> DerivedMetrics {
    compute_derived_metrics_with_plan(
        raw, 0.0, profile, StructuralChannels { channels: vec![] }, &[],
        &LedgerAggregate::default(), &MarketRegime::default(), plan,
    )
}

fn metrics(raw: &RawOnChainData) -> DerivedMetrics {
    metrics_with(raw, &reserve_profile(), &PlanContext::none())
}

fn f(s: &str) -> f64 {
    s.trim().parse().unwrap_or_else(|_| panic!("not a number: {s}"))
}

fn pos_usd(raw: &RawOnChainData, p: &RawPosition) -> f64 {
    p.amount0 * leg_usd_price(&p.token0, raw.spot_price_nock_usd, raw.live_eth_price_usd)
        + p.amount1 * leg_usd_price(&p.token1, raw.spot_price_nock_usd, raw.live_eth_price_usd)
}

fn cfg(yaml: &str) -> PositionConfig {
    serde_yaml::from_str(yaml).expect("position config parses")
}

const V4_LADDER_YAML: &str = "id: ladder-near\nprotocol: uniswap-v4\nnft_id: 3240369\npool_id: \"0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13\"\ncurrency0: \"0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913\"\ncurrency1: \"0x9B5E262cF9bb04869ab40b19AF91D2dc85761722\"\ntick_lower: 270900\ntick_upper: 887100\n";

fn config_error(extra: &str) -> String {
    validate_positions_config(&[cfg(&format!("{V4_LADDER_YAML}{extra}"))]).unwrap_err().to_string()
}


/// The cycle's snapshot of these reads: pool rows and fee rows as fetch_raw_data builds
/// them, then cycle_snapshot_data.
fn snapshot(mut raw: RawOnChainData, m: DerivedMetrics) -> (SnapshotData, Vec<String>) {
    raw.pool_mechanics = build_pool_mechanics(&raw.positions, raw.spot_price_nock_usd, raw.live_eth_price_usd);
    raw.contract_fees = build_contract_fees(&snapshot_positions(&raw), raw.live_gas_cost_usd, &raw.fees_not_lp_claimable);
    cycle_snapshot_data(raw, m, None, "2026-11-20T12:00:00+00:00")
}

fn t(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

// ------------------------------------------------------------------
// Configuration
// ------------------------------------------------------------------

#[test]
fn test_role_unset_and_ladder_are_accepted_with_no_default() {
    // Unset keeps the pre-#167 meaning; nothing is defaulted.
    let plain = cfg(V4_LADDER_YAML);
    assert_eq!((plain.role.as_deref(), plain.rung.as_deref(), plain.deposit_tx.as_deref()), (None, None, None));
    assert!(validate_positions_config(&[plain.clone()]).is_ok());
    assert!(!is_ladder_config(&plain));
    for extra in [
        "role: ladder\n".to_string(),
        "role: ladder\nrung: near\n".to_string(),
        "role: ladder\nrung: deep\n".to_string(),
        // The bottle's cap (operator decision, 2026-11-20).
        "role: ladder\nrung: cap\n".to_string(),
        format!("role: ladder\nrung: deep\ndeposit_tx: \"{MINT_TX}\"\n"),
        // An explicit false cannot be told from unset, and is not refused.
        "role: ladder\nrebuild_when_out_of_range: false\n".to_string(),
        // YAML null is unset.
        "role: ladder\nrung:\n".to_string(),
    ] {
        let c = cfg(&format!("{V4_LADDER_YAML}{extra}"));
        assert!(validate_positions_config(&[c.clone()]).is_ok(), "{extra}");
        assert!(is_ladder_config(&c), "{extra}");
    }
    // The tag lowercases the hash and keeps an unset rung unset.
    let c = cfg(&format!("{V4_LADDER_YAML}role: ladder\ndeposit_tx: \"0x{}\"\n", MINT_TX.trim_start_matches("0x").to_uppercase()));
    assert_eq!(ladder_tag(&c), tag("ladder-near", None, Some(MINT_TX)));
}

#[test]
fn test_an_unknown_role_is_a_configuration_error_with_the_exact_message() {
    for bad in ["Ladder", "LADDER", "lp", "gas", "", " ladder"] {
        let e = config_error(&format!("role: \"{bad}\"\n"));
        assert_eq!(
            e,
            format!("Configuration error: position 'ladder-near' has unknown role '{bad}'. Supported: ladder, bet (or leave role unset). Nothing was read; correct `positions:` in config.yaml"),
        );
    }
}

#[test]
fn test_a_ladder_refuses_every_key_that_would_steer_it() {
    let cases: [(&str, &str); 9] = [
        ("role: ladder\nis_price_source: true\n", "is role: ladder and is_price_source"),
        ("role: ladder\nrebuild_weight_bps: 0\n", "sets rebuild_weight_bps (0): a ladder is never a rebuild template, the role already leaves it out"),
        ("role: ladder\nrebuild_weight_bps: 5000\n", "sets rebuild_weight_bps (5000)"),
        ("role: ladder\nrebuild_when_out_of_range: true\n", "sets rebuild_when_out_of_range: true"),
        ("role: ladder\ndeposit_url: \"https://app.uniswap.org\"\n", "sets deposit_url, which only rebuild suggestions use"),
        ("role: ladder\nrung: Near\n", "has unknown rung 'Near'. Supported: cap, near, deep (or leave rung unset)"),
        ("role: ladder\nrung: mid\n", "has unknown rung 'mid'"),
        ("role: ladder\ndeposit_tx: \"0x1234\"\n", "has deposit_tx '0x1234', which is not a transaction hash (0x and 64 hex characters)"),
        ("role: ladder\ndeposit_tx: \"afe42fe29de67337d5b489226dbfdf8a8b315e2deabaf6c744b47c2dd9ff7a58\"\n", "which is not a transaction hash"),
    ];
    for (extra, want) in cases {
        let e = config_error(extra);
        assert!(e.starts_with("Configuration error: position 'ladder-near' ") && e.contains(want), "{extra}: {e}");
        assert!(e.ends_with("Nothing was read; correct `positions:` in config.yaml"), "{e}");
    }
    // A full-range Aero basic pool is never a ladder.
    let basic = cfg("id: basic\nprotocol: aerodrome\npool_address: \"0x770fE8Fc9248c6382950A86E39a9391B41D59e0A\"\nrole: ladder\n");
    let e = validate_positions_config(&[basic]).unwrap_err().to_string();
    assert!(e.contains("is role: ladder on protocol aerodrome, a full-range pool"), "{e}");
    // rung and deposit_tx belong to a ladder only.
    assert!(config_error("rung: deep\n").contains("sets rung, which applies only to role: ladder"));
    assert!(config_error(&format!("deposit_tx: \"{MINT_TX}\"\n")).contains("sets deposit_tx, which applies only to role: ladder"));
    // One bad entry fails the whole list, as for an unknown protocol.
    let e = validate_positions_config(&[cfg(V4_LADDER_YAML), cfg(&format!("{V4_LADDER_YAML}role: lp\n"))]).unwrap_err();
    assert!(e.to_string().contains("unknown role 'lp'"));
}

#[test]
fn test_after_the_read_a_ladder_must_be_a_usdc_nock_concentrated_range() {
    let ladders = |p: &RawPosition| HashMap::from([(p.position_id.clone(), tag("rung", None, None))]);
    let ok = ladder_rung_at(269065);
    assert!(check_ladder_pairs(&[ok.clone()], &ladders(&ok), &HashMap::new()).is_ok());
    // NOCK as token0 passes too.
    let mut flipped = ok.clone();
    std::mem::swap(&mut flipped.token0, &mut flipped.token1);
    assert!(check_ladder_pairs(&[flipped.clone()], &ladders(&flipped), &HashMap::new()).is_ok());
    // An untagged WETH/NOCK range is not checked.
    assert!(check_ladder_pairs(&[weth_nock_v3()], &ladders(&ok), &HashMap::new()).is_ok());
    for (token0, token1) in [(weth(), nock()), (weth(), usdc()), (nock(), weth())] {
        let p = RawPosition { token0: token0.clone(), token1: token1.clone(), ..ok.clone() };
        let e = check_ladder_pairs(&[p.clone()], &ladders(&p), &HashMap::new()).unwrap_err().to_string();
        assert_eq!(
            e,
            format!("Configuration error: position 'rung' is role: ladder but is a {}/{} pair. A ladder is a USDC/NOCK range; AERO and ETH ladders are not supported yet (#170). Correct `positions:` in config.yaml", token0.symbol, token1.symbol)
        );
    }
    let full = RawPosition { tick_lower: MIN_TICK, tick_upper: MAX_TICK, ..ok.clone() };
    let e = check_ladder_pairs(&[full.clone()], &ladders(&full), &HashMap::new()).unwrap_err().to_string();
    assert!(e.contains("spans every price (full range: ticks -887272..887272"), "{e}");
}

// ------------------------------------------------------------------
// The deposit receipt
// ------------------------------------------------------------------

#[test]
fn test_the_recorded_v4_mint_is_a_verified_deposit_with_exact_amounts() {
    let rung = ladder_rung_at(269065);
    let d = parse_ladder_deposit(&recorded("v4_mint"), MINTER, &rung).expect("verified");
    assert_eq!(d, LadderDeposit { tx: MINT_TX.to_string(), block: 59_660_587, at: None, liquidity: MINT_L, amount0_raw: MINT_USDC_RAW, amount1_raw: MINT_NOCK_RAW });
    // The wallet check is case-insensitive.
    assert!(parse_ladder_deposit(&recorded("v4_mint"), &format!("0x{}", MINTER.trim_start_matches("0x").to_uppercase()), &rung).is_ok());
    // The block's time comes from the recorded header: 2026-11-19T21:08:13Z.
    let LadderBaselineRead::Verified(v) = verified_mint() else { panic!() };
    assert_eq!(v.at, Some(t("2026-11-19T21:08:13Z")));
}

/// The recorded mint with one change.
fn refused_with(change: impl Fn(&mut serde_json::Value), p: &RawPosition) -> String {
    let mut r = recorded("v4_mint");
    change(&mut r);
    parse_ladder_deposit(&r, MINTER, p).unwrap_err()
}

fn modify_log(r: &mut serde_json::Value) -> &mut serde_json::Value {
    let logs = r["logs"].as_array_mut().unwrap();
    logs.iter_mut().find(|l| l["topics"][0] == V4_MODIFY_LIQUIDITY_TOPIC).unwrap()
}

fn replace_word(log: &mut serde_json::Value, i: usize, word: &str) {
    let data = log["data"].as_str().unwrap().trim_start_matches("0x").to_string();
    let mut words: Vec<String> = (0..data.len() / 64).map(|k| data[k * 64..(k + 1) * 64].to_string()).collect();
    words[i] = word.to_string();
    log["data"] = json!(format!("0x{}", words.concat()));
}

#[test]
fn test_a_receipt_that_does_not_prove_this_deposit_is_refused() {
    let rung = ladder_rung_at(269065);
    // Another position's NFT.
    let other = RawPosition { nft_id: 3240370, ..rung.clone() };
    assert_eq!(parse_ladder_deposit(&recorded("v4_mint"), MINTER, &other).unwrap_err(), "its salt is not NFT #3240370: it is another position's deposit");
    // Other ticks.
    let ticks = RawPosition { tick_lower: 271200, ..rung.clone() };
    assert_eq!(parse_ladder_deposit(&recorded("v4_mint"), MINTER, &ticks).unwrap_err(), "it deposits into ticks 270900..887100, not this position's 271200..887100");
    // Another pool.
    let pool = RawPosition { pool_id: "0x91a9ef0a8dd231b99fa53a2a145a9e8bca7897e0d85852107abb997916f9f99c".to_string(), ..rung.clone() };
    assert!(parse_ladder_deposit(&recorded("v4_mint"), MINTER, &pool).unwrap_err().starts_with("it modifies pool 0x064a5df2"));
    // Another wallet.
    assert_eq!(
        parse_ladder_deposit(&recorded("v4_mint"), OPERATOR, &rung).unwrap_err(),
        format!("the transaction was sent by {MINTER}, not by the tracked wallet")
    );
    // A failed transaction.
    assert_eq!(refused_with(|r| r["status"] = json!("0x0"), &rung), "the transaction failed (status 0x0): it deposited nothing");
    // Two ModifyLiquidity events.
    let two = refused_with(
        |r| {
            let extra = modify_log(r).clone();
            r["logs"].as_array_mut().unwrap().push(extra);
        },
        &rung,
    );
    assert_eq!(two, "the receipt has 2 PoolManager ModifyLiquidity events; a ladder deposit has exactly one");
    // A withdrawal (negative liquidityDelta).
    let out = refused_with(|r| replace_word(modify_log(r), 2, &"f".repeat(64)), &rung);
    assert_eq!(out, "it removes liquidity: it is a withdrawal, not a deposit");
    // Not through the position manager.
    let direct = refused_with(|r| modify_log(r)["topics"][2] = json!(format!("0x{:0>64}", MINTER.trim_start_matches("0x"))), &rung);
    assert_eq!(direct, "its liquidity was not added through the Uniswap v4 position manager");
    // More USDC paid than the liquidity holds (5 USDC more in the same receipt).
    let more = refused_with(
        |r| {
            let logs = r["logs"].as_array_mut().unwrap();
            let usdc_log = logs.iter_mut().find(|l| l["address"] == USDC_BASE_LC).unwrap();
            replace_word(usdc_log, 0, &format!("{:064x}", MINT_USDC_RAW + 5_000_000));
        },
        &rung,
    );
    assert!(more.starts_with("the amounts paid (1226851161 and 2475789900788685019 raw) are more than liquidity 934864722135742"), "{more}");
    // A refund back from the PoolManager is subtracted; the rest no longer fits the curve.
    let refund = refused_with(
        |r| {
            let logs = r["logs"].as_array_mut().unwrap();
            let mut back = logs.iter().find(|l| l["address"] == NOCK_BASE_LC).unwrap().clone();
            let tp = back["topics"].clone();
            back["topics"] = json!([tp[0], tp[2], tp[1]]);
            replace_word(&mut back, 0, &format!("{:064x}", MINT_NOCK_RAW / 2));
            logs.push(back);
        },
        &rung,
    );
    assert!(refund.contains("do not imply one pool price"), "{refund}");
    // A zap that mints and swaps in one transaction (recorded): its flows are not the deposit.
    let zap = recorded("v4_zap");
    let zap_rung = RawPosition { nft_id: 3233880, tick_lower: 267600, tick_upper: 278700, ..rung.clone() };
    let from = zap["from"].as_str().unwrap().to_string();
    assert_eq!(parse_ladder_deposit(&zap, &from, &zap_rung).unwrap_err(), "the transaction also swaps, so its token flows are not the deposit alone");
}

#[test]
fn test_the_recorded_v3_increase_uses_the_event_amounts_exactly() {
    // The operator's own v3 WETH/NOCK NFT #6336662 (2026-11-14, block 57396344). The parse
    // is pair-agnostic; check_ladder_pairs is what refuses a WETH/NOCK ladder.
    let v3 = RawPosition { liquidity: 18_841_285_041_665_077_684, ..weth_nock_v3() };
    let d = parse_ladder_deposit(&recorded("v3_increase"), OPERATOR, &v3).expect("verified");
    assert_eq!(d.tx, V3_INCREASE_TX);
    assert_eq!((d.block, d.liquidity, d.amount0_raw, d.amount1_raw), (57_396_344, 18_841_285_041_665_077_684, 0x4615ea2aa3d2f6, 0x6acc0639ee60d726a));
    // Another NFT, or two increases, is refused.
    let other = RawPosition { nft_id: 6336663, ..v3.clone() };
    assert_eq!(parse_ladder_deposit(&recorded("v3_increase"), OPERATOR, &other).unwrap_err(), "its IncreaseLiquidity is for another NFT, not #6336663");
    let mut two = recorded("v3_increase");
    let inc = two["logs"].as_array().unwrap().iter().find(|l| l["topics"][0] == INCREASE_LIQUIDITY_TOPIC).unwrap().clone();
    two["logs"].as_array_mut().unwrap().push(inc);
    assert_eq!(parse_ladder_deposit(&two, OPERATOR, &v3).unwrap_err(), "the receipt has 2 IncreaseLiquidity events from the Uniswap v3 position manager; a ladder deposit has exactly one");
    // The pair check refuses it as a ladder.
    let e = check_ladder_pairs(&[v3.clone()], &HashMap::from([(v3.position_id.clone(), tag("uniswap-v3-weth-nock", None, None))]), &HashMap::new()).unwrap_err();
    assert!(e.to_string().contains("is a WETH/NOCK pair") && e.to_string().contains("(#170)"));
}

#[test]
fn test_one_sided_deposits_must_take_exactly_the_edge_amount() {
    // ticks 270900..887100 and the recorded liquidity: all-USDC is 1226.094997… USDC.
    let all0 = concentrated_amounts_at(MINT_L as f64, tick_sqrt_price(270900), tick_sqrt_price(270900), tick_sqrt_price(887100)).0;
    assert!(check_deposit_on_curve(MINT_L, all0.round() as u128, 0, 270900, 887100).is_ok());
    // One raw unit rounded up is fine; 1 USDC short is not.
    assert!(check_deposit_on_curve(MINT_L, all0.round() as u128 + 1, 1, 270900, 887100).is_ok());
    let short = check_deposit_on_curve(MINT_L, all0.round() as u128 - 1_000_000, 0, 270900, 887100).unwrap_err();
    assert!(short.starts_with("a deposit of only token0 into this range takes"), "{short}");
    assert_eq!(check_deposit_on_curve(MINT_L, 0, 0, 270900, 887100).unwrap_err(), "the wallet paid nothing into the position");
}

// ------------------------------------------------------------------
// The baseline read: cached when verified or refused, never when unverified
// ------------------------------------------------------------------

/// A local JSON-RPC node that answers eth_getTransactionReceipt from `receipts` and
/// eth_getBlockByNumber from the recorded headers (when `headers`); anything else is an
/// error. Returns its URL and a count of the requests it answered.
async fn receipt_node(receipts: Vec<(String, serde_json::Value)>, headers: bool) -> (String, Arc<std::sync::atomic::AtomicUsize>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let hits = Arc::new(std::sync::atomic::AtomicUsize::new(0));
    let handler = {
        let hits = hits.clone();
        move |Json(req): Json<serde_json::Value>| {
            let (receipts, hits) = (receipts.clone(), hits.clone());
            async move {
                hits.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
                let p0 = req["params"][0].as_str().unwrap_or_default().to_lowercase();
                let answer = match req["method"].as_str().unwrap_or_default() {
                    "eth_getTransactionReceipt" => match receipts.iter().find(|(tx, _)| *tx == p0) {
                        Some((_, r)) => json!({ "result": r }),
                        None => json!({ "result": null }),
                    },
                    "eth_getBlockByNumber" if headers => json!({ "result": block_header(&p0) }),
                    _ => json!({ "error": { "code": -32000, "message": "not served" } }),
                };
                let mut body = json!({ "jsonrpc": "2.0", "id": req["id"] });
                for (k, v) in answer.as_object().unwrap() {
                    body[k] = v.clone();
                }
                Json(body)
            }
        }
    };
    tokio::spawn(async move {
        let _ = axum::serve(listener, axum::Router::new().route("/", axum::routing::post(handler))).await;
    });
    (url, hits)
}

async fn dead_node() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    url
}

/// A rung with an id no other test uses: the receipt cache is process-wide.
fn rung_with_id(id: &str) -> RawPosition {
    RawPosition { position_id: id.to_string(), ..ladder_rung_at(269065) }
}

#[tokio::test]
async fn test_an_unverified_read_is_not_cached_and_is_retried_next_cycle() {
    let client = Client::new();
    let rung = rung_with_id("uniswap-v4:base:3240369:unverified-test");
    let tg = tag("ladder-near", Some("near"), Some(MINT_TX));
    // The RPC does not answer: unverified, with the reason; never 0.
    let dead = dead_node().await;
    match read_ladder_baseline(&client, &dead, MINTER, &rung, &tg).await {
        LadderBaselineRead::Unverified { tx, reason } => {
            assert_eq!(tx, MINT_TX);
            assert_eq!(reason, "the receipt could not be read this cycle (the RPC did not answer); it is read again next cycle");
        }
        other => panic!("{other:?}"),
    }
    // The node has no receipt for it: unverified too.
    let (empty, _) = receipt_node(vec![], true).await;
    let r = read_ladder_baseline(&client, &empty, MINTER, &rung, &tg).await;
    assert!(matches!(&r, LadderBaselineRead::Unverified { reason, .. } if reason.contains("the node has no receipt for it")), "{r:?}");
    // Next cycle the receipt is there: it is read and verified (nothing was cached).
    let (live, hits) = receipt_node(vec![(MINT_TX.to_string(), recorded("v4_mint"))], true).await;
    let LadderBaselineRead::Verified(d) = read_ladder_baseline(&client, &live, MINTER, &rung, &tg).await else { panic!() };
    assert_eq!((d.liquidity, d.at), (MINT_L, Some(t("2026-11-19T21:08:13Z"))));
    assert_eq!(hits.load(std::sync::atomic::Ordering::SeqCst), 2, "the receipt and the block header");
    // From then on it is cached: a dead RPC still gives the verified baseline.
    assert!(matches!(read_ladder_baseline(&client, &dead, MINTER, &rung, &tg).await, LadderBaselineRead::Verified(_)));
    // No deposit_tx: nothing is read.
    assert_eq!(read_ladder_baseline(&client, &dead, MINTER, &rung, &tag("x", None, None)).await, LadderBaselineRead::NotConfigured);
}

#[tokio::test]
async fn test_a_refused_read_is_cached_and_a_missing_block_time_is_not() {
    let client = Client::new();
    let rung = rung_with_id("uniswap-v4:base:3240369:refused-test");
    let tg = tag("ladder-near", None, Some(MINT_TX));
    // Another wallet sent it: refused, and cached (a receipt does not change).
    let (live, _) = receipt_node(vec![(MINT_TX.to_string(), recorded("v4_mint"))], true).await;
    let r = read_ladder_baseline(&client, &live, OPERATOR, &rung, &tg).await;
    assert!(matches!(&r, LadderBaselineRead::Refused { reason, .. } if reason.contains("not by the tracked wallet")), "{r:?}");
    let dead = dead_node().await;
    assert_eq!(read_ladder_baseline(&client, &dead, OPERATOR, &rung, &tg).await, r);
    // Verified, but the header read fails: the baseline stands with no time, and it is
    // read again until the time is known.
    let rung2 = rung_with_id("uniswap-v4:base:3240369:no-header-test");
    let (no_headers, _) = receipt_node(vec![(MINT_TX.to_string(), recorded("v4_mint"))], false).await;
    let LadderBaselineRead::Verified(d) = read_ladder_baseline(&client, &no_headers, MINTER, &rung2, &tg).await else { panic!() };
    assert_eq!(d.at, None);
    assert!(matches!(read_ladder_baseline(&client, &dead, MINTER, &rung2, &tg).await, LadderBaselineRead::Unverified { .. }));
}

// ------------------------------------------------------------------
// The reading
// ------------------------------------------------------------------

fn reading_of(raw: &RawOnChainData, id: &str) -> LadderReading {
    ladder_readings(raw).into_iter().find(|r| r.position_id == id).unwrap_or_else(|| panic!("no reading for {id}"))
}

/// RawOnChainData derives no Clone (production never copies it): the fields read here.
fn copy_of(raw: &RawOnChainData) -> RawOnChainData {
    RawOnChainData {
        wallet_balances: raw.wallet_balances.clone(),
        positions: raw.positions.clone(),
        spot_price_nock_usd: raw.spot_price_nock_usd,
        live_eth_price_usd: raw.live_eth_price_usd,
        live_gas_cost_usd: raw.live_gas_cost_usd,
        wallet_eth: raw.wallet_eth,
        wallet_weth: raw.wallet_weth,
        pool_liquidity: raw.pool_liquidity.clone(),
        position_facts: raw.position_facts.clone(),
        ladders: raw.ladders.clone(),
        ladder_baselines: raw.ladder_baselines.clone(),
        ..Default::default()
    }
}

/// The deep rung at another pool tick.
fn deep_at(tick: i32) -> RawPosition {
    with_amounts(RawPosition { tick, sqrt_price_x96: sqrt_x96_at_tick(tick), in_range: tick_in_range(tick, 276000, 280200), ..deep_rung() })
}

/// The same range with the tokens swapped: NOCK token0, ticks negated.
fn mirrored(p: &RawPosition) -> RawPosition {
    let mut q = p.clone();
    std::mem::swap(&mut q.token0, &mut q.token1);
    (q.tick_lower, q.tick_upper) = (-p.tick_upper, -p.tick_lower);
    q.tick = -p.tick;
    q.sqrt_price_x96 = sqrt_x96_at_tick(-p.tick);
    q.in_range = tick_in_range(q.tick, q.tick_lower, q.tick_upper);
    with_amounts(q)
}

#[test]
fn test_the_state_is_what_the_range_holds_in_either_token_order() {
    let base = ladder_book();
    // NOCK is token1: a lower tick is a higher USD price.
    for (tick, want) in [(269065, "holds_only_usdc"), (277000, "in_range"), (281000, "holds_only_nock")] {
        let mut raw = copy_of(&base);
        raw.positions[3] = deep_at(tick);
        assert_eq!(ladder_state(&raw, &raw.positions[3]), want, "{tick}");
        let plain = reading_of(&raw, "uniswap-v4:base:3299001");
        // NOCK as token0: the same range mirrored reads the same state, bounds and amounts.
        let mut mraw = copy_of(&base);
        mraw.positions[3] = mirrored(&deep_at(tick));
        assert_eq!(ladder_state(&mraw, &mraw.positions[3]), want, "mirrored {tick}");
        let m = reading_of(&mraw, "uniswap-v4:base:3299001");
        assert_eq!((&m.low_price_usd, &m.high_price_usd), (&plain.low_price_usd, &plain.high_price_usd), "{tick}");
        assert_eq!((m.usdc_now.as_str(), m.nock_now.as_str()), (plain.usdc_now.as_str(), plain.nock_now.as_str()), "{tick}");
        assert_eq!(m.vs_top, plain.vs_top, "{tick}");
        if want == "holds_only_nock" {
            assert_eq!(plain.usdc_now, "0.00");
            assert_eq!(Some(plain.nock_now.clone()), plain.vs_top.nock_at_bottom);
            assert_eq!(plain.state_text, "holds only NOCK: the price is below the range's bottom ($0.00678699); a rise back into it sells NOCK for USDC");
        }
    }
    // An emptied NFT and a frozen pool.
    let mut raw = copy_of(&base);
    raw.positions[3].liquidity = 0;
    assert_eq!(ladder_state(&raw, &raw.positions[3]), "empty");
    let mut raw = copy_of(&base);
    raw.pool_liquidity.insert(raw.positions[3].position_id.clone(), "0".to_string());
    assert_eq!(ladder_state(&raw, &raw.positions[3]), "pool_stale");
    assert_eq!(reading_of(&raw, "uniswap-v4:base:3299001").state_text, "its pool has no in-range liquidity, so the pool price is frozen and the state is not a market reading");
}

#[test]
fn test_the_curve_figures_need_no_baseline_and_match_the_range_edges() {
    let raw = ladder_book();
    let near = reading_of(&raw, "uniswap-v4:base:3240369");
    // Above the range: only USDC, the all-USDC amount of liquidity 934864722135742.
    assert_eq!(near.state, "holds_only_usdc");
    assert_eq!(near.state_text, "holds only USDC: the price is above the range, so it converts nothing until NOCK falls into it at $0.01720088");
    assert_eq!((near.usdc_now.as_str(), near.nock_now.as_str()), ("1226.09", "0.0000"));
    // Its bottom is the pool's lowest usable price (tick 887100 at spacing 300): no bottom
    // price and no NOCK at the bottom are stated (a figure beyond any supply).
    assert_eq!(near.vs_top, LadderVsTop {
        usdc_at_top: Some("1226.09".to_string()),
        nock_at_bottom: None,
        nock_held_vs_top: "0.0000".to_string(),
        usdc_converted_vs_top: Some("0.00".to_string()),
        curve_avg_price_usd: None,
        converted_pct: Some("0.00".to_string()),
    });
    assert_eq!((near.low_price_usd.as_deref(), near.low_pct_of_spot.as_deref()), (None, None));
    assert_eq!(near.open_end.as_deref(), Some("the bottom reaches the pool's lowest usable price (tick 887100): the range buys all the way down, so no bottom price and no NOCK at the bottom are stated"));
    assert_eq!((near.high_price_usd.as_deref(), near.high_pct_of_spot.as_deref(), near.spot_price_usd.as_deref()), (Some("0.01720088"), Some("-16.76"), Some("0.02066445")));
    // The deep rung's edges are concentrated_amounts_at at its ticks.
    let deep = reading_of(&raw, "uniswap-v4:base:3299001");
    let (sa, sb) = (tick_sqrt_price(276000), tick_sqrt_price(280200));
    let l = 400_000_000_000_000f64;
    assert_eq!(deep.vs_top.usdc_at_top, Some(format!("{:.2}", concentrated_amounts_at(l, sa, sa, sb).0 / 1e6)));
    assert_eq!(deep.vs_top.nock_at_bottom, Some(format!("{:.4}", concentrated_amounts_at(l, sb, sa, sb).1 / 1e16)));
    assert_eq!((deep.low_price_usd.as_deref(), deep.high_price_usd.as_deref(), deep.vs_top.usdc_at_top.as_deref(), deep.vs_top.nock_at_bottom.as_deref()), (Some("0.00678699"), Some("0.01032932"), Some("77.00"), Some("9196.4045")));
    assert_eq!(deep.open_end, None);
    assert_eq!((deep.rung, deep.rung_note.as_deref()), (None, Some(RUNG_UNSET_NOTE)));
    // In range, the curve's average price from the top to the pool price is the geometric
    // mean of the two: sqrt(top x pool price).
    let mut raw = ladder_book();
    raw.positions[2] = ladder_rung_at(272400);
    let r = reading_of(&raw, "uniswap-v4:base:3240369");
    assert_eq!(r.state, "in_range");
    let avg = f(r.vs_top.curve_avg_price_usd.as_deref().unwrap());
    let geo = (f(r.high_price_usd.as_deref().unwrap()) * f(r.pool_price_usd.as_deref().unwrap())).sqrt();
    assert!((avg / geo - 1.0).abs() < 1e-6, "{avg} vs {geo}");
    // Holdings, not trades: converted = top less now, held = now.
    assert_eq!(r.vs_top.nock_held_vs_top, r.nock_now);
    assert!((f(r.vs_top.usdc_converted_vs_top.as_deref().unwrap()) - (f(r.vs_top.usdc_at_top.as_deref().unwrap()) - f(&r.usdc_now))).abs() <= 0.011);
    assert_eq!(r.state_text, "in range: it converts USDC to NOCK as the price falls and NOCK back to USDC as it rises");
}

#[test]
fn test_since_deposit_is_the_receipt_or_null_with_a_reason() {
    // Verified, and the price rose since: it sold the NOCK it was deposited with. The
    // signed figures say so; there is no average price.
    let raw = ladder_book();
    let sd = reading_of(&raw, "uniswap-v4:base:3240369").since_deposit;
    assert_eq!(sd, LadderSinceDeposit {
        status: "verified".to_string(),
        deposit_tx: Some(MINT_TX.to_string()),
        deposit_block: Some(59_660_587),
        deposited_at: Some("2026-11-19T21:08:13+00:00".to_string()),
        deposited_usdc: Some("1221.85".to_string()),
        deposited_nock: Some("247.5790".to_string()),
        deposit_liquidity: Some(MINT_L.to_string()),
        current_liquidity: MINT_L.to_string(),
        net_nock_bought: Some("-247.5790".to_string()),
        net_usdc_spent: Some("-4.24".to_string()),
        avg_buy_price_usd: None,
        reason: None,
    });
    // Verified, and the price fell into the range: a net buy at an average price between
    // the deposit's price and the pool price now.
    let mut fell = ladder_book();
    fell.positions[2] = ladder_rung_at(272400);
    let r = reading_of(&fell, "uniswap-v4:base:3240369");
    let sd = &r.since_deposit;
    let (bought, spent, avg) = (f(sd.net_nock_bought.as_deref().unwrap()), f(sd.net_usdc_spent.as_deref().unwrap()), f(sd.avg_buy_price_usd.as_deref().unwrap()));
    assert!(bought > 0.0 && spent > 0.0);
    assert!((avg - spent / bought).abs() < 1e-6);
    let deposit_price = 0.017082013799784877; // the price the recorded amounts imply
    assert!(avg < deposit_price && avg > f(r.pool_price_usd.as_deref().unwrap()), "{avg}");
    assert_eq!((sd.net_nock_bought.as_deref(), sd.net_usdc_spent.as_deref(), sd.avg_buy_price_usd.as_deref()), (Some("5303.7885"), Some("84.35"), Some("0.01590283")));
    // Added to or partly withdrawn: liquidity_changed, both figures, no bought figure.
    let mut changed = ladder_book();
    changed.positions[2].liquidity = MINT_L + 1;
    let sd = reading_of(&changed, "uniswap-v4:base:3240369").since_deposit;
    assert_eq!(sd.status, "liquidity_changed");
    assert_eq!((sd.deposit_liquidity.as_deref(), sd.current_liquidity.as_str()), (Some("934864722135742"), "934864722135743"));
    assert_eq!((sd.net_nock_bought, sd.net_usdc_spent, sd.avg_buy_price_usd), (None, None, None));
    assert!(sd.reason.unwrap().starts_with("liquidity was added or partly withdrawn since the deposit (deposited 934864722135742, now 934864722135743)"));
    // An emptied NFT reads liquidity_changed against its deposit too.
    let mut emptied = ladder_book();
    emptied.positions[2].liquidity = 0;
    assert_eq!(reading_of(&emptied, "uniswap-v4:base:3240369").since_deposit.status, "liquidity_changed");
    // No deposit_tx: no baseline, and nothing is inferred.
    let sd = reading_of(&raw, "uniswap-v4:base:3299001").since_deposit;
    assert_eq!(sd.status, "no_baseline");
    assert_eq!(sd.reason.as_deref(), Some("no deposit_tx is set for this ladder in config.yaml: NockNock has no baseline for what it bought and does not invent one"));
    assert_eq!((sd.deposit_tx, sd.deposited_usdc, sd.net_nock_bought, sd.avg_buy_price_usd), (None, None, None, None));
    // Refused and unverified: the reason, and null figures.
    for (read, status, reason) in [
        (
            LadderBaselineRead::Refused { tx: MINT_TX.to_string(), reason: "the transaction also swaps, so its token flows are not the deposit alone".to_string() },
            "refused",
            "the deposit_tx receipt does not prove a deposit into this position: the transaction also swaps, so its token flows are not the deposit alone",
        ),
        (
            LadderBaselineRead::Unverified { tx: MINT_TX.to_string(), reason: "the receipt could not be read this cycle (the RPC did not answer); it is read again next cycle".to_string() },
            "unverified",
            "the receipt could not be read this cycle (the RPC did not answer); it is read again next cycle",
        ),
    ] {
        let mut raw = ladder_book();
        raw.ladder_baselines.insert("uniswap-v4:base:3240369".to_string(), read);
        let sd = reading_of(&raw, "uniswap-v4:base:3240369").since_deposit;
        assert_eq!((sd.status.as_str(), sd.reason.as_deref(), sd.deposit_tx.as_deref()), (status, Some(reason), Some(MINT_TX)));
        assert_eq!((sd.deposited_usdc, sd.net_nock_bought, sd.net_usdc_spent, sd.avg_buy_price_usd), (None, None, None, None));
    }
    // A deposit_tx with no read this cycle (a fixture): unverified, never no_baseline.
    let mut raw = ladder_book();
    raw.ladder_baselines.clear();
    assert_eq!(reading_of(&raw, "uniswap-v4:base:3240369").since_deposit.status, "unverified");
}

#[test]
fn test_no_average_price_without_a_net_buy_for_usdc() {
    // NOCK held up but USDC up too (more than was deposited): no price, never a negative
    // or an infinite one.
    let mut raw = ladder_book();
    raw.positions[2] = ladder_rung_at(272400);
    raw.positions[2].amount0 = 2000.0;
    let sd = reading_of(&raw, "uniswap-v4:base:3240369").since_deposit;
    assert!(f(sd.net_nock_bought.as_deref().unwrap()) > 0.0 && f(sd.net_usdc_spent.as_deref().unwrap()) < 0.0);
    assert_eq!(sd.avg_buy_price_usd, None);
}

#[test]
fn test_without_a_spot_the_percentages_and_value_are_null() {
    let mut raw = ladder_book();
    raw.spot_price_nock_usd = 0.0;
    let r = reading_of(&raw, "uniswap-v4:base:3240369");
    assert_eq!((r.low_pct_of_spot, r.high_pct_of_spot, r.spot_price_usd, r.value_usd), (None, None, None, None));
    // The amounts are measured from the position's own pool all the same.
    assert_eq!((r.usdc_now.as_str(), r.vs_top.usdc_at_top.as_deref()), ("1226.09", Some("1226.09")));
}

// ------------------------------------------------------------------
// Counted everywhere, steered nowhere
// ------------------------------------------------------------------

#[test]
fn test_a_ladder_counts_in_every_total_and_in_the_floor_crossing_price() {
    let mut raw = ladder_book();
    raw.positions[2] = ladder_rung_at(272400); // holds NOCK and USDC
    let m = metrics(&raw);
    let all_lp: f64 = raw.positions.iter().map(|p| pos_usd(&raw, p)).sum();
    // The same positions with no role: NOCK LP. Every total is the same.
    let plain = metrics(&untagged(copy_of(&raw)));
    let g = &m.global_status;
    let pg = &plain.global_status;
    for (k, a, b) in [
        ("total_portfolio_usd", &g.total_portfolio_usd, &pg.total_portfolio_usd),
        ("total_native_usd", &g.total_native_usd, &pg.total_native_usd),
        ("total_stable_usd", &g.total_stable_usd, &pg.total_stable_usd),
        ("total_nock_amount", &g.total_nock_amount, &pg.total_nock_amount),
        ("total_fees_accrued_usd", &g.total_fees_accrued_usd, &pg.total_fees_accrued_usd),
        ("reserve_stable_usdc", g.reserve_stable_usdc.as_ref().unwrap(), pg.reserve_stable_usdc.as_ref().unwrap()),
        ("reserve_headroom_usdc", g.reserve_headroom_usdc.as_ref().unwrap(), pg.reserve_headroom_usdc.as_ref().unwrap()),
        ("dry_powder_usdc", g.dry_powder_usdc.as_ref().unwrap(), pg.dry_powder_usdc.as_ref().unwrap()),
    ] {
        assert_eq!(a, b, "{k}");
    }
    assert_eq!(m.asset_aggregation_series.active_protocol_series[0], format!("{:.2}", all_lp));
    let ladder_nock = raw.positions[2].amount1;
    assert!(ladder_nock > 5_000.0);
    // The floor-crossing price counts the ladder's NOCK and lists it.
    let fb = m.floor_breach.as_ref().unwrap();
    assert_eq!(fb.nock_now, plain.floor_breach.as_ref().unwrap().nock_now);
    assert_eq!(fb.crossing, plain.floor_breach.as_ref().unwrap().crossing);
    assert_eq!(fb.nock_now.as_deref(), Some(g.total_nock_amount.as_str()));
    assert!(fb.positions.iter().any(|p| p.position_id == "uniswap-v4:base:3240369"));
    // positions_nock and the brackets keep the ladder: the NOCK stack adds up to the total.
    assert!(m.asset_aggregation_series.positions_nock.iter().any(|s| s.position_id == "uniswap-v4:base:3240369" && s.series[0] == format!("{:.2}", ladder_nock)));
    assert_eq!(m.tick_bracket_hypervisor.brackets.len(), raw.positions.len());
}

#[test]
fn test_a_ladder_is_not_nock_lp_target_rebuild_or_engine() {
    let raw = ladder_book();
    let m = metrics(&raw);
    let nock_lp: f64 = nock_only().positions.iter().map(|p| pos_usd(&raw, p)).sum();
    // The rebuild is judged on NOCK LP alone.
    assert_eq!(m.lp_rebuild.actual_usd, format!("{:.2}", nock_lp));
    assert_eq!(m.lp_rebuild.status, "under_target");
    for id in ["uniswap-v4:base:3240369", "uniswap-v4:base:3299001"] {
        assert!(m.lp_rebuild.suggestions.iter().all(|s| s.position_id != id), "{id} suggested");
        assert!(m.lp_rebuild.excluded.iter().all(|e| e.position_id != id), "{id} excluded");
    }
    // The NOCK LP row is NOCK LP's; the ladder row is appended last, measured, no target.
    let rows = &m.portfolio_deviation_matrix.allocations;
    let total = f(&m.global_status.total_portfolio_usd);
    let lp_row = rows.iter().find(|r| r.bucket_name == NOCK_LP_BUCKET_NAME).unwrap();
    assert_eq!(lp_row.actual_weight_bps, ((nock_lp / total) * 10000.0) as i32);
    let last = rows.last().unwrap();
    assert_eq!((last.bucket_name.as_str(), last.target_weight_bps, last.status.as_str()), (LADDER_BUCKET_NAME, 0, "no_target"));
    let ladder_usd = pos_usd(&raw, &raw.positions[2]) + pos_usd(&raw, &raw.positions[3]);
    assert_eq!(last.actual_weight_bps, ((ladder_usd / total) * 10000.0) as i32);
    assert_eq!(last.actual_weight_bps, 2576);
    // Untagged, the same ranges are NOCK LP: the role is the switch.
    let plain = metrics(&untagged(ladder_book()));
    assert!(plain.portfolio_deviation_matrix.allocations.iter().all(|r| r.bucket_name != LADDER_BUCKET_NAME));
    assert!(f(&plain.lp_rebuild.actual_usd) > f(&m.lp_rebuild.actual_usd) + 1000.0);
    // Called directly, an in-range ladder on a live pool with no rebuild keys is neither a
    // template nor an exclusion.
    let mut inr = ladder_book();
    inr.positions[2] = ladder_rung_at(272400);
    let (s, x) = lp_rebuild_suggestions(&inr, 100_000.0, RebuildFunds { nock: 1e9, floor_tolerance_nock: 0.0, floor_set: true, usdc: 1e9 });
    assert!(s.iter().all(|sg| sg.nft_id != 3240369) && x.iter().all(|e| e.nft_id != 3240369));
    assert!(s.iter().any(|sg| sg.nft_id == 3163450), "the NOCK LP template is still sized");
}

#[test]
fn test_the_summary_names_ladders_by_what_they_hold_and_never_out_of_range() {
    let raw = ladder_book();
    let (summary, active) = lp_positions_summary(&raw);
    assert_eq!(summary, "2 NOCK LP positions tracked: 2 ranged (2 in range). +2 ladders, measured only (2 holding only USDC).");
    assert!(active);
    let mut mixed = ladder_book();
    mixed.positions[2] = ladder_rung_at(272400);
    assert_eq!(lp_positions_summary(&mixed).0, "2 NOCK LP positions tracked: 2 ranged (2 in range). +2 ladders, measured only (1 holding only USDC, 1 in range).");
    assert!(!lp_positions_summary(&mixed).0.contains("out of range"));
    // Only ladders, one in range: the engine is not active.
    let mut only = ladder_book();
    only.positions.retain(|p| p.nft_id == 3240369 || p.nft_id == 3299001);
    only.positions[0] = ladder_rung_at(272400);
    let (summary, active) = lp_positions_summary(&only);
    assert_eq!(summary, "No NOCK LP positions tracked. +2 ladders, measured only (1 holding only USDC, 1 in range).");
    assert!(!active);
}

#[test]
fn test_a_ladder_only_book_is_no_nock_lp_tracked() {
    // The spot from a closed v4 price source's pool (#144): the only live NOCK range is a ladder.
    let mut raw = ladder_book();
    raw.positions.retain(|p| p.nft_id == 3240369);
    raw.positions[0] = ladder_rung_at(272400);
    let m = metrics(&raw);
    assert_eq!(m.lp_rebuild.status, "no_positions");
    assert!(m.lp_rebuild.suggestions.is_empty() && m.lp_rebuild.excluded.is_empty());
    assert!(m.global_status.shot_advice_action.starts_with("HOLD & ACCRETE (No NOCK LP Tracked)"), "{}", m.global_status.shot_advice_action);
    assert!(m.global_status.shot_advice_details.contains("No NOCK LP positions tracked. +1 ladder, measured only (1 in range)."), "{}", m.global_status.shot_advice_details);
    // Its NOCK is still in the total.
    assert_eq!(m.global_status.total_nock_amount, format!("{:.0}", 115_747.8001 + raw.positions[0].amount1));
}

#[test]
fn test_the_reserve_counts_ladder_usdc_and_names_it() {
    let raw = ladder_book();
    let m = metrics(&raw);
    let g = &m.global_status;
    // wallet 1053.00 + NOCK LP USDC + the two ladders' USDC (1226.09 + 77.00).
    let nock_lp_usdc: f64 = nock_only().positions.iter().map(|p| if is_usdc_token(&p.token0) { p.amount0 } else { 0.0 }).sum();
    let ladder_usdc = raw.positions[2].amount0 + raw.positions[3].amount0;
    assert_eq!(g.reserve_stable_usdc.clone().unwrap(), format!("{:.2}", 1053.0 + nock_lp_usdc + ladder_usdc));
    assert_eq!((g.reserve_stable_usdc.as_deref(), g.reserve_ladder_usdc.as_deref(), g.reserve_headroom_usdc.as_deref()), (Some("2485.13"), Some("1303.10"), Some("1535.13")));
    let d = &g.shot_advice_details;
    assert!(d.contains("The $950.00 protected cash reserve is measured on wallet USDC plus the $129.04 of USDC in NOCK LP and the $1303.10 in ladders: $2485.13, $1535.13 above it."), "{d}");
    assert!(d.contains("The $1432.13 of USDC inside NOCK LP and ladders ($1303.10 of it in ladders) is working capital, not dry powder"), "{d}");
    // The MCP reserve line names them; the frontend's reserveText does the same (reserve.test.ts).
    let full = serde_json::to_value(&m).unwrap();
    assert_eq!(
        crate::reserve_text::reserve_line(full.get("global_status")),
        "• Protected cash reserve: $950.00 (wallet + NOCK LP and ladder USDC ($1303.10 in ladders) $2485.13, $1535.13 above it; liquid dry powder $800.09); kept for buying lower, never drawn by a plan buy\n"
    );
    // No reserve set: no ladder field either.
    let none = metrics_with(&raw, &profile(), &PlanContext::none());
    assert_eq!((none.global_status.reserve_ladder_usdc, none.global_status.reserve_stable_usdc), (None, None));
    // A ladder holding no USDC is stated (0.00) but not named in the text.
    let mut nock_held = ladder_book();
    nock_held.positions.truncate(3);
    nock_held.ladders.remove("uniswap-v4:base:3299001");
    nock_held.positions[2] = deep_at(281000);
    nock_held.positions[2].position_id = "uniswap-v4:base:3240369".to_string();
    let m2 = metrics(&nock_held);
    assert_eq!(m2.global_status.reserve_ladder_usdc.as_deref(), Some("0.00"));
    assert!(m2.global_status.shot_advice_details.contains("of USDC in NOCK LP: $"), "{}", m2.global_status.shot_advice_details);
    assert!(!m2.global_status.shot_advice_details.contains("in ladders"), "{}", m2.global_status.shot_advice_details);
}

#[test]
fn test_plan_headroom_counts_ladder_usdc_and_says_so() {
    let raw = ladder_book();
    let plan_cfg: AccumulationPlanConfig = serde_yaml::from_str("id: test-plan\nbudget_usdc: 500\ntranches: 5\ninterval_hours: 24\nrespect_cash_target: false\n").unwrap();
    let plan = PlanContext { cfg: Some(plan_cfg), fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-20T12:00:00Z") };
    // A standing reserve just under the stable balance: the plan is capped by it.
    let tight = OperatorProfile { cash_reserve_usdc: Some(2450.0), ..profile() };
    let m = metrics_with(&raw, &tight, &plan);
    let ps = m.accumulation_plan.as_ref().unwrap();
    assert_eq!(ps.status, "tranche_due");
    assert_eq!(ps.tranche_usdc, "35.13");
    assert_eq!(
        ps.binding_constraint.as_deref(),
        Some("stable balance $2485.13 (wallet $1053.00 + NOCK LP $129.04 + ladders $1303.10) less the standing $2450.00 protected cash reserve")
    );
    assert!(m.global_status.shot_advice_details.contains("The plan never spends the stable balance (wallet + NOCK LP and ladder USDC) below $2450.00."), "{}", m.global_status.shot_advice_details);
    // The same book with no ladder: the old words.
    let m = metrics_with(&nock_only(), &tight, &plan);
    assert!(!m.global_status.shot_advice_details.contains("ladder"), "{}", m.global_status.shot_advice_details);
}

#[test]
fn test_without_a_ladder_no_new_field_is_emitted() {
    let m = metrics(&nock_only());
    let (data, _) = snapshot(nock_only(), m);
    let text = serde_json::to_string(&data).unwrap();
    for k in ["\"ladders\"", "\"role\"", "\"rung\"", "reserve_ladder_usdc", "ladder_buys_since", "NOCK ladders", "ladder"] {
        assert!(!text.contains(k), "{k} in a snapshot without a ladder");
    }
}

#[test]
fn test_the_snapshot_carries_role_rung_and_one_reading_per_ladder() {
    let raw = ladder_book();
    let m = metrics(&raw);
    let (data, _) = snapshot(raw, m);
    let roles: Vec<(u64, Option<&str>, Option<&str>)> = data.positions.iter().map(|p| (p.nft_id, p.role.as_deref(), p.rung.as_deref())).collect();
    assert_eq!(roles, vec![(3163450, None, None), (6336662, None, None), (3240369, Some("ladder"), Some("near")), (3299001, Some("ladder"), None)]);
    assert_eq!(data.ladders.iter().map(|l| l.config_id.as_str()).collect::<Vec<_>>(), vec!["ladder-near", "ladder-deep"]);
    // It parses back (the stored row is read by the simulator and the MCP fallback).
    let back: SnapshotData = serde_json::from_value(serde_json::to_value(&data).unwrap()).unwrap();
    assert_eq!(back.ladders, data.ladders);
}

// ------------------------------------------------------------------
// The whole-stack cost basis
// ------------------------------------------------------------------

fn opening(as_of: &str) -> OpeningBasisState {
    OpeningBasisState::Set(OpeningBasis { nock: 90_086.79, cost_usd: 1_525.05, as_of: t(as_of), source: "test figures".to_string() })
}

#[test]
fn test_verified_ladder_buys_after_the_opening_enter_the_whole_stack_basis() {
    let mut raw = ladder_book();
    raw.positions[2] = ladder_rung_at(272400);
    let ladders = ladder_readings(&raw);
    let fills: Vec<BasisFill> = vec![];
    let cb = compute_cost_basis_with_ladders(Ok(&fills), &opening("2026-11-19T00:00:00Z"), Some(0.016), Some(120_000.0), &ladders);
    let ws = &cb.whole_stack;
    assert_eq!(ws.status, "ok");
    let lb = ws.ladder_buys_since.as_ref().unwrap();
    assert_eq!((lb.ladders_counted, lb.nock.as_str(), lb.usdc.as_str()), (1, "5303.7885", "84.35"));
    assert_eq!(lb.not_counted, vec!["uniswap-v4:base:3299001: no verified deposit baseline (no_baseline)".to_string()]);
    assert_eq!(ws.basis_nock.as_deref(), Some(format!("{:.4}", 90_086.79 + 5_303.7885).as_str()));
    assert_eq!(ws.basis_cost_usd.as_deref(), Some("1609.40"));
    assert!(ws.includes.iter().any(|i| i == "The net NOCK bought by 1 of 2 tracked ladders since a receipt-verified deposit after 2026-11-19T00:00:00+00:00: 5303.7885 NOCK for $84.35, measured from each deposit_tx receipt to now."), "{:?}", ws.includes);
    assert!(ws.excludes.iter().any(|e| e.starts_with("Tracked ladders left out: uniswap-v4:base:3299001")));
    // Plan buys are untouched: ladders are not plan buys.
    assert_eq!(cb.plan_buys, compute_cost_basis(Ok(&fills), &opening("2026-11-19T00:00:00Z"), Some(0.016), Some(120_000.0)).plan_buys);
    // With no ladder the block is the one compute_cost_basis gives.
    assert_eq!(
        compute_cost_basis_with_ladders(Ok(&fills), &opening("2026-11-19T00:00:00Z"), Some(0.016), Some(120_000.0), &[]),
        compute_cost_basis(Ok(&fills), &opening("2026-11-19T00:00:00Z"), Some(0.016), Some(120_000.0))
    );
    // The MCP line names the ladder part (both transports print cost_basis_text).
    let full = json!({ "derived_metrics": { "cost_basis": serde_json::to_value(&cb).unwrap() } });
    assert!(crate::cost_basis_text::cost_basis_lines(&full).contains("plus 0 verified buy(s) since. Plus the net buys of 1 of 2 tracked ladders since a verified deposit: 5303.7885 NOCK for $84.35."));
}

#[test]
fn test_a_ladder_buy_that_cannot_be_placed_after_the_opening_is_left_out_with_the_reason() {
    let mut raw = ladder_book();
    raw.positions[2] = ladder_rung_at(272400);
    let r = reading_of(&raw, "uniswap-v4:base:3240369");
    assert_eq!(ladder_buys_for_basis(&r, t("2026-11-19T00:00:00Z")).unwrap(), (5303.7885, 84.35));
    // Deposited before the opening basis: its buys may be inside the opening figures.
    assert_eq!(
        ladder_buys_for_basis(&r, t("2026-11-20T00:00:00Z")).unwrap_err(),
        "uniswap-v4:base:3240369: deposited at or before the opening basis (2026-11-20T00:00:00+00:00), so what it bought cannot be split from the opening figures"
    );
    // No block time.
    let mut no_time = r.clone();
    no_time.since_deposit.deposited_at = None;
    assert!(ladder_buys_for_basis(&no_time, t("2026-11-19T00:00:00Z")).unwrap_err().contains("the deposit block's time was not read"));
    // A net sale since the deposit is not a buy.
    let sold = reading_of(&ladder_book(), "uniswap-v4:base:3240369");
    assert_eq!(ladder_buys_for_basis(&sold, t("2026-11-19T00:00:00Z")).unwrap_err(), "uniswap-v4:base:3240369: no net buy since its deposit (-247.5790 NOCK for $-4.24)");
    // liquidity_changed: no baseline.
    let mut changed = ladder_book();
    changed.positions[2].liquidity += 1;
    assert!(ladder_buys_for_basis(&reading_of(&changed, "uniswap-v4:base:3240369"), t("2026-11-19T00:00:00Z")).unwrap_err().contains("(liquidity_changed)"));
}

// ------------------------------------------------------------------
// The simulator's stored book (#166)
// ------------------------------------------------------------------

#[test]
fn test_the_simulator_counts_a_tracked_ladder_as_an_existing_range_and_agrees_on_the_reserve() {
    let raw = ladder_book();
    let m = metrics(&raw);
    let (data, _) = snapshot(raw, m);
    let book = stored_book(&data);
    assert!(book.problems.is_empty(), "{:?}", book.problems);
    let mut ids: Vec<&String> = book.raw.ladders.keys().collect();
    ids.sort();
    assert_eq!(ids, vec!["uniswap-v4:base:3240369", "uniswap-v4:base:3299001"]);
    // Rebuilt from the snapshot, a ladder is not NOCK LP there either.
    assert!(book.raw.positions.iter().filter(|p| is_nock_lp(&book.raw, p)).all(|p| p.nft_id == 3163450 || p.nft_id == 6336662));
    let env = SnapshotEnvelope {
        meta: SnapshotMeta { generated_at: t("2026-11-20T12:00:00Z"), state_hash: compute_state_hash(&data), version: SNAPSHOT_SPEC_VERSION.to_string(), snapshot_id: Uuid::nil() },
        data: data.clone(),
    };
    let params: HashMap<String, String> = [("usdc", "100"), ("low", "0.01"), ("high_pct", "-5")].into_iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    let req = LadderRequest::from_params(&params).expect("valid request");
    let body = simulate_ladder(&env, &req, t("2026-11-20T12:00:00Z"), "ok").expect("simulated");
    // The reserve at spot is the snapshot's own reserve_stable_usdc, ladders included.
    assert_eq!(body["reserve_check"]["stable_at_spot_usdc"], json!(data.derived_metrics.global_status.reserve_stable_usdc.clone().unwrap()));
    assert_eq!(body["reserve_check"]["existing_ladders"], json!(["uniswap-v4:base:3240369", "uniswap-v4:base:3299001"]));
    let text = crate::ladder_text::ladder_text(&body);
    assert!(text.contains("USDC there, 2 tracked ladders among them)"), "{text}");
}

// ------------------------------------------------------------------
// A burned ladder
// ------------------------------------------------------------------

#[test]
fn test_a_burned_ladder_is_named_as_a_ladder_in_its_closed_label() {
    let c = cfg(&format!("{V4_LADDER_YAML}role: ladder\n"));
    assert_eq!(closed_position_label(&c), "v4 USDC/NOCK #3240369 (ladder-near, ladder)");
    assert_eq!(closed_position_label(&cfg(V4_LADDER_YAML)), "v4 USDC/NOCK #3240369 (ladder-near)");
}

// ------------------------------------------------------------------
// MCP text, and the golden file
// ------------------------------------------------------------------

#[test]
fn test_the_mcp_text_states_each_ladder_and_its_baseline() {
    let raw = ladder_book();
    let m = metrics(&raw);
    let (data, _) = snapshot(raw, m);
    let full = serde_json::to_value(&data).unwrap();
    let state = format_portfolio_state_text(&full);
    let block = crate::ladder_track_text::ladders_lines(&full);
    assert!(state.contains(&block) && block.starts_with("• Ladders (measured, not NOCK LP; counted in the totals, the reserve and the floor-crossing price):\n"), "{state}");
    assert!(block.contains("   – v4 USDC/NOCK 3.00% #3240369 (rung near): no bottom – $0.01720088 (no bottom / -16.76% of spot); price above the range: all USDC; holds $1226.09 USDC + 0.0000 NOCK\n     the bottom reaches"), "{block}");
    assert!(block.contains("since deposit (tx 0xafe42fe2…, block 59660587): net -247.5790 NOCK bought for net $-4.24 USDC spent; no average price (it has not net bought NOCK with USDC)"), "{block}");
    assert!(block.contains("(rung unset)") && block.contains("since deposit: no_baseline: no deposit_tx is set for this ladder in config.yaml"), "{block}");
    // The plan text says ladders are not in the rebuild.
    let plan = format_accumulation_plan_text(&full);
    assert!(plan.contains("\n• Ladders are not rebuild templates and are not in the NOCK LP target (2 tracked, measured only)."), "{plan}");
    // Measurement words only.
    for w in ["should", "recommend", "re-centre", "withdraw it", "pull "] {
        assert!(!block.to_lowercase().contains(w), "{w}: {block}");
    }
}

/// One golden case: the snapshot through cycle_snapshot_data, its cost basis and the MCP text.
fn golden_ladder_case(raw: RawOnChainData, plan: &PlanContext, opening_as_of: &str) -> serde_json::Value {
    let mut m = metrics_with(&raw, &reserve_profile(), plan);
    let ladders = ladder_readings(&raw);
    let fills: Vec<BasisFill> = vec![];
    let held = m.global_status.total_nock_amount.parse::<f64>().ok();
    m.cost_basis = Some(compute_cost_basis_with_ladders(Ok(&fills), &opening(opening_as_of), Some(raw.spot_price_nock_usd), held, &ladders));
    let (data, _) = snapshot(raw, m);
    let full = serde_json::to_value(&data).unwrap();
    json!({
        "state_hash": compute_state_hash(&data),
        "snapshot": canonical_state_json(&data),
        "mcp_text": {
            "get_portfolio_state": format_portfolio_state_text(&full),
            "get_shot_advice": format_shot_advice_text(&full),
            "simulate_claim_fees": format_claim_simulation_text(&full),
            "get_accumulation_plan": format_accumulation_plan_text(&full),
        },
    })
}

//   NOCKNOCK_BLESS_GOLDEN=1 cargo test test_golden_snapshot
#[test]
fn test_golden_snapshot_ladder_is_byte_identical() {
    let plan_cfg: AccumulationPlanConfig = serde_yaml::from_str("id: test-plan\nbudget_usdc: 500\ntranches: 5\ninterval_hours: 24\n").unwrap();
    let plan = PlanContext { cfg: Some(plan_cfg), fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-20T12:00:00Z") };
    // 1. The book above the rungs: the near rung sold its deposited NOCK back; the deep rung
    //    has no baseline.
    let above = golden_ladder_case(ladder_book(), &PlanContext::none(), "2026-11-19T00:00:00Z");
    // 2. The 3% pool fell into the near rung (spot with it): a verified net buy, in the
    //    whole-stack basis, with a plan tranche due.
    let bought = golden_ladder_case(ladder_book_at(272400), &plan, "2026-11-19T00:00:00Z");
    let doc = json!({ "ladders_above_range": above, "near_rung_bought_plan_due": bought });
    let actual = format!("{}\n", serde_json::to_string_pretty(&doc).unwrap());
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/golden_snapshot_ladder.json");
    if std::env::var("NOCKNOCK_BLESS_GOLDEN").is_ok() {
        std::fs::write(path, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(path).expect("testdata/golden_snapshot_ladder.json is checked in");
    assert!(actual == expected, "snapshot bytes changed; compare {path} with a fresh NOCKNOCK_BLESS_GOLDEN=1 run and `git diff` it");
}

#[test]
fn test_the_example_config_documents_role_rung_and_deposit_tx_and_sets_none() {
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/config.yaml.example");
    let text = std::fs::read_to_string(path).unwrap();
    for k in [
        "#   role        unset (the default meaning since #151: classified by its pair),",
        "\"bet\" (#181: an aerodrome-cl WETH/NOCK bet",
        "#   rung        optional, ladder only: \"cap\" (the tick band containing spot, holding the",
        "#   deposit_tx  optional, ladder only: the transaction that minted this NFT",
        "#   role: \"ladder\"",
        "#   rung: \"deep\"",
    ] {
        assert!(text.contains(k), "config.yaml.example: {k}");
    }
    // The example is commented out: the loaded example sets no role, rung or deposit_tx.
    let example = load_app_config(path, Some("/nonexistent/config.yaml"), false).unwrap();
    assert!(example.positions.iter().all(|c| c.role.is_none() && c.rung.is_none() && c.deposit_tx.is_none()));
    // The bets (#181) are commented out too: no bet settings are loaded.
    assert!(example.operator_profile.nock_bets.is_none());
    assert!(validate_positions_config(&example.positions).is_ok());
}

// ------------------------------------------------------------------
// Review findings (#167): usable extremes, and one position manager per protocol
// ------------------------------------------------------------------

#[test]
fn test_a_full_range_nft_on_the_usable_ticks_is_refused_at_every_spacing() {
    let base = ladder_rung_at(269065);
    let ladders = |p: &RawPosition| HashMap::from([(p.position_id.clone(), tag("rung", None, None))]);
    let facts = |p: &RawPosition, s: i32| HashMap::from([(p.position_id.clone(), PositionFacts { tick_spacing: Some(s), ..Default::default() })]);
    // What the Uniswap and Aero apps mint for "Full range": the usable extremes.
    for (spacing, edge) in [(60, 887220), (200, 887200), (300, 887100)] {
        let full = RawPosition { tick_lower: -edge, tick_upper: edge, ..base.clone() };
        assert_eq!(ladder_bounds_at_extremes(-edge, edge, Some(spacing)), (true, true), "{spacing}");
        let e = check_ladder_pairs(&[full.clone()], &ladders(&full), &facts(&full, spacing)).unwrap_err().to_string();
        assert_eq!(
            e,
            format!("Configuration error: position 'rung' is role: ladder but spans every price (full range: ticks {}..{edge} reach the pool's usable extremes at tick spacing {spacing}): a ladder is a concentrated range. Correct `positions:` in config.yaml", -edge)
        );
        // One spacing in from the extremes still counts as reaching them; two do not.
        assert_eq!(ladder_bounds_at_extremes(-edge + spacing, edge - spacing, Some(spacing)), (true, true));
        assert_eq!(ladder_bounds_at_extremes(-edge + 2 * spacing, edge - 2 * spacing, Some(spacing)), (false, false));
        // A rung that reaches only one extreme is a ladder (it buys all the way down).
        let one_sided = RawPosition { tick_lower: 270900 / spacing * spacing, tick_upper: edge, ..base.clone() };
        assert!(check_ladder_pairs(&[one_sided.clone()], &ladders(&one_sided), &facts(&one_sided, spacing)).is_ok(), "{spacing}");
    }
    // Uniswap v3: the spacing comes from the factory's fee table when it was not read.
    let v3 = RawPosition { protocol: "uniswap-v3".to_string(), fee_tier_bps: 100, tick_lower: -887200, tick_upper: 887200, ..base.clone() };
    assert_eq!(ladder_tick_spacing(&HashMap::new(), &v3), Some(200));
    assert!(check_ladder_pairs(&[v3.clone()], &ladders(&v3), &HashMap::new()).unwrap_err().to_string().contains("at tick spacing 200"));
    assert_eq!(ladder_tick_spacing(&HashMap::new(), &RawPosition { fee_tier_bps: 30, ..v3.clone() }), Some(60));
    // An unread spacing falls back to 1,000 ticks from MIN_TICK/MAX_TICK, and says so.
    let unknown = RawPosition { tick_lower: -887000, tick_upper: 887000, ..base.clone() };
    assert_eq!(ladder_tick_spacing(&HashMap::new(), &unknown), None);
    let e = check_ladder_pairs(&[unknown.clone()], &ladders(&unknown), &HashMap::new()).unwrap_err().to_string();
    assert!(e.contains("within 1000 ticks (tick spacing not read)"), "{e}");
}

#[test]
fn test_an_open_ended_bound_is_stated_in_words_never_as_a_number() {
    let raw = ladder_book();
    let full = serde_json::to_value(&ladder_readings(&raw)).unwrap();
    let near = &full[0];
    assert_eq!(near["low_price_usd"], json!(null));
    assert_eq!(near["vs_top"]["nock_at_bottom"], json!(null));
    let text = crate::ladder_track_text::ladder_statement(near);
    assert!(text.contains("(rung near): no bottom – $0.01720088 (no bottom / -16.76% of spot);"), "{text}");
    assert!(text.contains("\n     the bottom reaches the pool's lowest usable price (tick 887100)"), "{text}");
    assert!(text.contains("all NOCK at the bottom: not stated (no bottom)"), "{text}");
    assert!(!text.contains("0.00000000") && !text.contains("1709690206954494208"), "{text}");
    // NOCK as token0: the same rung mirrored has its bottom at the lowest usable tick.
    let mut m = copy_of(&raw);
    m.positions[2] = mirrored(&raw.positions[2]);
    let r = reading_of(&m, "uniswap-v4:base:3240369");
    assert_eq!((r.low_price_usd, r.vs_top.nock_at_bottom), (None, None));
    assert!(r.open_end.unwrap().contains("(tick -887100)"));
    // A top at the highest usable price: no top, no USDC at the top, no converted figure.
    let mut top = copy_of(&raw);
    let mut q = mirrored(&deep_at(277000));
    q.tick_upper = 887100;
    top.positions[3] = with_amounts(q);
    let r = reading_of(&top, "uniswap-v4:base:3299001");
    assert_eq!((r.high_price_usd.as_deref(), r.vs_top.usdc_at_top.as_deref(), r.vs_top.usdc_converted_vs_top.as_deref(), r.vs_top.converted_pct.as_deref()), (None, None, None, None));
    assert!(r.open_end.unwrap().starts_with("the top reaches the pool's highest usable price (tick 887100)"));
}

#[test]
fn test_an_increase_from_the_other_protocols_position_manager_is_refused() {
    // The recorded v3 IncreaseLiquidity, emitted as if by the Aero CL manager: for a v3
    // ladder with the same NFT id and ticks it is another protocol's NFT.
    let v3 = RawPosition { liquidity: 18_841_285_041_665_077_684, ..weth_nock_v3() };
    let mut as_cl = recorded("v3_increase");
    for l in as_cl["logs"].as_array_mut().unwrap() {
        if l["topics"][0] == INCREASE_LIQUIDITY_TOPIC {
            l["address"] = json!(AERO_CL_POSITION_MANAGER_BASE);
        }
    }
    assert_eq!(
        parse_ladder_deposit(&as_cl, OPERATOR, &v3).unwrap_err(),
        "its IncreaseLiquidity was emitted by another position manager, not the Uniswap v3 position manager: it is another protocol's NFT"
    );
    // The same receipt is a deposit for an Aero CL ladder with that NFT id and ticks...
    let cl = RawPosition { protocol: "aerodrome-cl".to_string(), ..v3.clone() };
    assert!(parse_ladder_deposit(&as_cl, OPERATOR, &cl).is_ok());
    // ...and the recorded (v3) receipt is not one for it.
    assert!(parse_ladder_deposit(&recorded("v3_increase"), OPERATOR, &cl).unwrap_err().contains("not the Aero CL position manager"));
}
