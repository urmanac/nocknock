//! Gauge rewards reported as collectible earnings — issue #142.
//!
//! Every `REAL_*` constant below is a raw JSON-RPC result captured from Base mainnet
//! through https://base-rpc.publicnode.com on 2026-11-17 (and read again through
//! https://mainnet.base.org with the same answer), at the block its comment names, and
//! is recorded with its call in ONCHAIN_INTEGRATION.md §9, "The claim call, proven".
//! The positions themselves are the #93 fixtures (tests_aero_rewards.rs, block
//! 58077673); only the reward reads, the wallet's AERO and the AERO price are from the
//! blocks around the operator's own claim. Nothing here is hand-made except where a
//! test says "synthetic".

use super::tests_aero_rewards::*;
use super::*;

// ---- Block 58618045 (`0x37e70bd`, 2026-11-17T11:13:04Z, hash 0xbe11c94f…814f): the
// ---- block before the operator's claim on the CL gauge ----
/// clGauge.earned(wallet, [2914490]): 17.654015228100787903 AERO.
const REAL_CL_EARNED_BEFORE_CLAIM: &str = "0x000000000000000000000000000000000000000000000000f4ffa950a248bebf";
/// basicGauge.earned(wallet): 0.210431026731433905 AERO (the same in the next block).
const REAL_BASIC_EARNED_AT_CLAIM: &str = "0x00000000000000000000000000000000000000000000000002eb99e7660c3fb1";
/// AERO price pool 0xd0acf293…6816 slot0() and liquidity(): $0.438859 per AERO.
const REAL_PRICE_SLOT0_AT_CLAIM: &str = "0x000000000000000000000000000000000000000000000b1d4342a214d0d4227cfffffffffffffffffffffffffffffffffffffffffffffffffffffffffffba86f00000000000000000000000000000000000000000000000000000000000002f9000000000000000000000000000000000000000000000000000000000000070900000000000000000000000000000000000000000000000000000000000007090000000000000000000000000000000000000000000000000000000000000001";
const REAL_PRICE_LIQUIDITY_AT_CLAIM: &str = "0x000000000000000000000000000000000000000000000014dcd3d0ac883a5ab7";

// ---- Block 58618046 (`0x37e70be`, hash 0x453956f0…1c38): the claim's block ----
/// clGauge.earned(wallet, [2914490]) after the claim: zero.
const REAL_CL_EARNED_AFTER_CLAIM: &str = "0x0000000000000000000000000000000000000000000000000000000000000000";
/// AERO.balanceOf(wallet) after the claim: exactly what `earned` said one block before.
const REAL_WALLET_AERO_AFTER_CLAIM: &str = "0x000000000000000000000000000000000000000000000000f4ffa950a248bebf";

// ---- The operator's claim: tx 0x2d2b609b…a16e, from the wallet to the CL gauge ----
const OPERATOR_CLAIM_TX: &str = "0x2d2b609b74142a5e56951850c646963d3553adac0c5280b70847f0bf0ed7a16e";
/// The transaction's `input`: selector 0x09902477, (wallet, wallet, [2914490]).
const REAL_OPERATOR_CLAIM_INPUT: &str = "0x09902477000000000000000000000000ee114cc97ec5adeb34ba5645e315b497d8487a45000000000000000000000000ee114cc97ec5adeb34ba5645e315b497d8487a450000000000000000000000000000000000000000000000000000000000000060000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000002c78ba";
/// Its receipt: gasUsed 0x662e0 (418,528), effectiveGasPrice 0x895440 (0.009 gwei).
const REAL_OPERATOR_CLAIM_GAS_USED: u128 = 0x662e0;
const REAL_OPERATOR_CLAIM_GAS_PRICE: u128 = 0x895440;

/// The `input` of the real basic-gauge claim by another account (tx 0x189f6ddf…b8f8,
/// block 57635701): selector 0x8ed86282, (claimer, claimer).
const REAL_BASIC_CLAIM_INPUT: &str = "0x8ed862820000000000000000000000007c12cd5b7db841f7ba9b3fd1b7a6d02c1304386d0000000000000000000000007c12cd5b7db841f7ba9b3fd1b7a6d02c1304386d";

/// eth_estimateGas of 0x8ed86282(wallet, wallet) from the wallet on the basic gauge,
/// 2026-11-17 near block 58619693: 390,860 gas.
const REAL_ESTIMATE_BASIC_CLAIM: &str = "0x5f6cc";
/// `eth_price_usd` of the deployment's snapshot of 2026-11-17T11:05:11Z.
const ETH_USD_2026_11_17: f64 = 1790.24;

/// The token and gauge logs of the operator's claim receipt (the other eight logs are
/// voter and messaging events).
fn operator_claim_receipt() -> serde_json::Value {
    let transfer = TRANSFER_EVENT_TOPIC;
    let zero = "0x0000000000000000000000000000000000000000000000000000000000000000";
    let wallet_topic = "0x000000000000000000000000ee114cc97ec5adeb34ba5645e315b497d8487a45";
    json!({
        "transactionHash": OPERATOR_CLAIM_TX,
        "blockNumber": "0x37e70be",
        "from": "0xee114cc97ec5adeb34ba5645e315b497d8487a45",
        "to": CL_GAUGE,
        "status": "0x1",
        "gasUsed": "0x662e0",
        "logs": [
            // The gauge's EmissionsClaimed(address,uint256,address,uint256): wallet, NFT, wallet.
            { "address": CL_GAUGE, "logIndex": "0x10",
              "topics": ["0x7b6bd720815adbeaaaed4fc759e1a535705901f33709fbb33e62ee9a89427bb1",
                         wallet_topic,
                         "0x00000000000000000000000000000000000000000000000000000000002c78ba",
                         wallet_topic],
              "data": "0x000000000000000000000000000000000000000000000000f4ffa950a248bebf" },
            // xAERO minted to and burned from an intermediary: not the wallet's.
            { "address": "0x6cdda0e730744dc6ff67654b35350adad05ecee7", "logIndex": "0x11",
              "topics": [transfer, zero, "0x0000000000000000000000008c37f30f3c8fd48beddb14086fa5148291d422c3"],
              "data": "0x000000000000000000000000000000000000000000000000f4ffa950a248bebf" },
            { "address": "0x6cdda0e730744dc6ff67654b35350adad05ecee7", "logIndex": "0x12",
              "topics": [transfer, "0x0000000000000000000000008c37f30f3c8fd48beddb14086fa5148291d422c3", zero],
              "data": "0x000000000000000000000000000000000000000000000000f4ffa950a248bebf" },
            // AERO minted to the wallet.
            { "address": "0x3f5bf3edac59d650d8fdf9657a71f2f904a3a4ee", "logIndex": "0x13",
              "topics": [transfer, zero, wallet_topic],
              "data": "0x000000000000000000000000000000000000000000000000f4ffa950a248bebf" },
            // A further 5% minted to a third account: not the wallet's.
            { "address": "0x3f5bf3edac59d650d8fdf9657a71f2f904a3a4ee", "logIndex": "0x14",
              "topics": [transfer, zero, "0x0000000000000000000000009beb95cced809c0667bbb6577ee4066ac5cc1156"],
              "data": "0x0000000000000000000000000000000000000000000000000c3ffbaa6e83a323" }
        ]
    })
}

// ------------------------------------------------------------------
// Fixture builders
// ------------------------------------------------------------------

fn price_at_claim() -> Result<f64> {
    aero_price_from_pool_reads(
        DEFAULT_AERO_PRICE_POOL,
        REAL_PRICE_POOL_TOKEN0,
        REAL_PRICE_POOL_TOKEN1,
        REAL_PRICE_SLOT0_AT_CLAIM,
        REAL_PRICE_LIQUIDITY_AT_CLAIM,
    )
}

/// The real answers of the two gauges, with the CL gauge's given by the caller.
fn earned_with(cl: &'static str) -> impl Fn(&str) -> Result<Option<String>> {
    move |gauge: &str| {
        if gauge.eq_ignore_ascii_case(BASIC_GAUGE) {
            ok_hex(REAL_BASIC_EARNED_AT_CLAIM)
        } else if gauge.eq_ignore_ascii_case(CL_GAUGE) {
            ok_hex(cl)
        } else {
            panic!("unexpected gauge {gauge}")
        }
    }
}

fn cycle(wallet_aero: &str, price: &dyn Fn() -> Result<f64>, earned: &dyn Fn(&str) -> Result<Option<String>>) -> RawOnChainData {
    real_cycle_with(&|cfgs, facts| aero_reads_from(cfgs, facts, wallet_aero, price(), earned).unwrap())
}

/// The cycle one block before the operator's claim: 17.654015 AERO in the CL gauge,
/// 0.210431 in the basic gauge, none in the wallet.
fn before_claim() -> RawOnChainData {
    cycle(REAL_AERO_BALANCE_WALLET, &price_at_claim, &earned_with(REAL_CL_EARNED_BEFORE_CLAIM))
}

/// The cycle in the claim's block: the CL gauge answers zero and the wallet holds it.
fn after_claim() -> RawOnChainData {
    cycle(REAL_WALLET_AERO_AFTER_CLAIM, &price_at_claim, &earned_with(REAL_CL_EARNED_AFTER_CLAIM))
}

/// Synthetic: the same cycle with the CL gauge answering `aero` whole AERO.
fn cycle_with_cl_reward(aero: u128) -> RawOnChainData {
    let earned = move |gauge: &str| {
        if gauge.eq_ignore_ascii_case(CL_GAUGE) {
            ok_hex(&format!("{:064x}", aero * 10u128.pow(18)))
        } else {
            ok_hex(REAL_BASIC_EARNED_AT_CLAIM)
        }
    };
    cycle(REAL_AERO_BALANCE_WALLET, &price_at_claim, &earned)
}

/// Synthetic: the basic position with `usdc_raw` of LP-claimable USDC fees (its
/// claimFees() made to succeed), so the cycle has an LP-fee total to compare.
fn with_lp_fees(mut raw: RawOnChainData, usdc_raw: u128) -> RawOnChainData {
    let basic_id = raw.positions[0].position_id.clone();
    raw.fees_not_lp_claimable.retain(|id| id != &basic_id);
    raw.positions[0] = aerodrome_position_from_reads(
        &basic_config(Some(BASIC_GAUGE)),
        WALLET,
        BASIC_POOL,
        token_info(&REAL_BASIC_TOKEN0[26..]).unwrap(),
        token_info(&REAL_BASIC_TOKEN1[26..]).unwrap(),
        REAL_BASIC_RESERVE0,
        REAL_BASIC_RESERVE1,
        REAL_BASIC_TOTAL_SUPPLY,
        REAL_BASIC_LP_STAKED,
        usdc_raw,
        0,
    )
    .unwrap();
    raw
}

fn metrics(raw: &RawOnChainData, profile: &OperatorProfile, regime: &MarketRegime) -> DerivedMetrics {
    compute_derived_metrics(raw, 0.0, profile, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), regime)
}

fn full(snap: &SnapshotData) -> serde_json::Value {
    serde_json::to_value(snap).unwrap()
}

/// Words that would turn a statement about AERO into advice.
const ADVICE_WORDS: [&str; 22] = [
    "should", "worth claiming", "claim now", "claim_now", "claim recommended", "claim_recommended", "recommend",
    "advis", "accumulate", "sell", "buy", "swap", "convert", "harvest", "hold it", "keep it", "claim it",
    "stake more", "compound", "consider", "you can claim", "time to",
];

fn assert_no_advice(text: &str) {
    // The worker's measured reason for the LP-fee part names "swap fees"; that phrase is
    // a fact about where the pool's fees go (ONCHAIN_INTEGRATION.md §8), not a verb.
    let lower = text.to_lowercase().replace("swap fees on staked liquidity go to the gauge", "");
    for banned in ADVICE_WORDS {
        assert!(!lower.contains(banned), "a statement about AERO must not advise: found '{banned}' in: {text}");
    }
}

/// The lines of a text that speak about AERO or a reward.
fn aero_lines_of(text: &str) -> Vec<&str> {
    text.lines().filter(|l| l.contains("AERO") || l.contains("reward")).collect()
}

// ------------------------------------------------------------------
// The claim call, proven on-chain
// ------------------------------------------------------------------

#[test]
fn test_claim_calldata_is_the_call_real_claims_sent() {
    // CL gauge: byte for byte the input of the operator's own claim.
    assert_eq!(
        verified_reward_claim_call("aerodrome-cl", CL_GAUGE, WALLET, NFT_ID).as_deref(),
        Some(REAL_OPERATOR_CLAIM_INPUT)
    );
    // Basic gauge: byte for byte the input of the real claim by another account.
    assert_eq!(verified_reward_claim_call("aerodrome", BASIC_GAUGE, CLAIMER, 0).as_deref(), Some(REAL_BASIC_CLAIM_INPUT));
    // And for the tracked wallet, the same call with its own address twice.
    assert_eq!(
        verified_reward_claim_call("aerodrome", BASIC_GAUGE, WALLET, 0).as_deref(),
        Some("0x8ed86282000000000000000000000000ee114cc97ec5adeb34ba5645e315b497d8487a45000000000000000000000000ee114cc97ec5adeb34ba5645e315b497d8487a45")
    );
    // The gauge address is matched whatever its case.
    assert!(verified_reward_claim_call("aerodrome-cl", &CL_GAUGE.to_uppercase().replace("0X", "0x"), WALLET, NFT_ID).is_some());
    // No call is proven for any other gauge, for a gauge of the other type, or for a
    // protocol that has no gauge: none is built.
    for (protocol, gauge) in [
        ("aerodrome", LEGACY_BASIC_GAUGE),
        ("aerodrome-cl", LEGACY_CL_GAUGE),
        ("aerodrome", CL_GAUGE),
        ("aerodrome-cl", BASIC_GAUGE),
        ("aerodrome", "0x1111111111111111111111111111111111111111"),
        ("uniswap-v3", CL_GAUGE),
        ("uniswap-v4", BASIC_GAUGE),
        ("something-else", CL_GAUGE),
    ] {
        assert_eq!(verified_reward_claim_call(protocol, gauge, WALLET, NFT_ID), None, "{protocol} {gauge}");
    }
}

#[test]
fn test_the_operators_claim_paid_exactly_what_earned_reported() {
    // One block before the claim earned(wallet, [2914490]) was 17.654015228100787903.
    let earned = decode_amount_word(REAL_CL_EARNED_BEFORE_CLAIM, "earned()").unwrap();
    assert_eq!(earned, 17654015228100787903);
    // The claim minted exactly that to the wallet, and nothing else to it.
    let transfers = parse_harvest_transfers(&operator_claim_receipt(), WALLET);
    assert_eq!(transfers.len(), 1, "{transfers:?}");
    let t = &transfers[0];
    assert_eq!((t.token_symbol.as_str(), t.amount_raw.as_str()), ("AERO", "17654015228100787903"));
    assert_eq!(t.amount_raw, earned.to_string());
    assert_eq!(t.amount_human, "17.654015228100787903");
    assert_eq!(t.from_address, "0x0000000000000000000000000000000000000000", "a claim is a mint");
    // The gauge's own event names the wallet, the NFT and the same amount.
    let event = &operator_claim_receipt()["logs"][0];
    assert_eq!(event["data"].as_str().unwrap(), REAL_CL_EARNED_BEFORE_CLAIM);
    assert_eq!(u64::from_str_radix(&event["topics"][2].as_str().unwrap()[58..], 16).unwrap(), NFT_ID);
    // In the claim's block the gauge answers zero and the wallet holds the amount.
    assert_eq!(decode_amount_word(REAL_CL_EARNED_AFTER_CLAIM, "earned()").unwrap(), 0);
    assert_eq!(decode_amount_word(REAL_WALLET_AERO_AFTER_CLAIM, "AERO balanceOf()").unwrap(), earned);
    // The further 5% (882700761405039395) went to another account, on top of it.
    let third = parse_harvest_transfers(&operator_claim_receipt(), "0x9beb95cced809c0667bbb6577ee4066ac5cc1156");
    assert_eq!(third.iter().map(|t| t.amount_raw.as_str()).collect::<Vec<_>>(), ["882700761405039395"]);
}

// ------------------------------------------------------------------
// The snapshot: each reward as collectible earnings
// ------------------------------------------------------------------

#[test]
fn test_real_cycle_reports_each_reward_as_collectible_earnings() {
    let snap = snapshot_of(before_claim(), &OperatorProfile::default(), 0.0);
    let (basic, cl) = (&snap.positions[0], &snap.positions[1]);
    let reward = |amount: &str, usd: &str, gauge: &str| CollectibleReward {
        status: "read".to_string(),
        token_symbol: "AERO".to_string(),
        token_address: "0x3f5bF3EdAc59d650D8fDf9657a71F2f904A3a4Ee".to_string(),
        amount: Some(amount.to_string()),
        usd: Some(usd.to_string()),
        gauge: Some(gauge.to_string()),
        collected_in: "the Aero app".to_string(),
        collect_url: "https://app.aero.xyz".to_string(),
        note: None,
    };
    assert_eq!(basic.collectible_reward, Some(reward("0.210431", "0.09", BASIC_GAUGE)));
    assert_eq!(cl.collectible_reward, Some(reward("17.654015", "7.75", CL_GAUGE)));
    // The fields #93 added keep their names and say the same thing.
    assert_eq!((basic.unclaimed_aero.as_deref(), basic.unclaimed_aero_usd.as_deref()), (Some("0.210431"), Some("0.09")));
    assert_eq!((cl.unclaimed_aero.as_deref(), cl.unclaimed_aero_usd.as_deref()), (Some("17.654015"), Some("7.75")));
    assert_eq!((basic.unclaimed_aero_note.as_deref(), cl.unclaimed_aero_note.as_deref()), (None, None));

    // The totals: rewards beside LP fees, and their sum. LP fees keep their own field.
    let g = &snap.derived_metrics.global_status;
    assert_eq!(g.aero_price_usd.as_deref(), Some("0.438859"));
    assert_eq!(g.unclaimed_aero_amount.as_deref(), Some("17.864446"));
    assert_eq!(g.total_fees_claimable_usd, "0.00");
    assert_eq!(g.collectible_rewards_usd.as_deref(), Some("7.84"));
    assert_eq!(g.collectible_rewards_usd, g.unclaimed_aero_usd);
    assert_eq!(g.total_collectible_usd.as_deref(), Some("7.84"));
    assert_eq!(g.collectible_rewards_note, None);

    // The velocity rows: the reward is the position's earnings, under a kind of its own.
    for (row, p, reason) in [
        (&snap.derived_metrics.velocity_gas_sandbox.contracts[0], basic, "not LP-claimable (claimFees() reverts)"),
        (&snap.derived_metrics.velocity_gas_sandbox.contracts[1], cl, "not LP-claimable (staked: swap fees on staked liquidity go to the gauge)"),
    ] {
        assert_eq!(row.earnings_kind.as_deref(), Some("gauge_reward"));
        assert_eq!(row.reward, p.collectible_reward);
        assert_eq!(row.lp_fees_note.as_deref(), Some(reason));
        // The LP-fee part is unchanged: not claimable, $0 of LP fees, no harvest word.
        assert_eq!(row.recommended_action, "not_claimable");
        assert_eq!(row.claimable_usd, "0.00");
        assert!(row.note.as_deref().unwrap().starts_with(reason), "{:?}", row.note);
    }
    // The fee rows carry the same three fields.
    for (row, p) in snap.contract_fees.iter().zip([basic, cl]) {
        assert_eq!(row.earnings_kind.as_deref(), Some("gauge_reward"));
        assert_eq!(row.reward, p.collectible_reward);
        assert!(row.lp_fees_note.as_deref().unwrap().starts_with("not LP-claimable ("));
        assert!(!row.claimable);
        assert!(row.amount.starts_with("not LP-claimable ("), "{}", row.amount);
    }
    // JSON shape: a position object a consumer can read without parsing a note.
    let json = full(&snap);
    assert_eq!(
        json["positions"][1]["collectible_reward"],
        json!({
            "status": "read", "token_symbol": "AERO", "token_address": "0x3f5bF3EdAc59d650D8fDf9657a71F2f904A3a4Ee",
            "amount": "17.654015", "usd": "7.75", "gauge": "0x3bf70c880adc8370917994ab0df68e763b7a60fd",
            "collected_in": "the Aero app", "collect_url": "https://app.aero.xyz", "note": null
        })
    );
}

#[test]
fn test_totals_state_lp_fees_and_rewards_side_by_side_and_add_up() {
    // The figures of 2026-11-17 11:05Z: $0.542769 + $1.407023 of LP fees on the two
    // Uniswap positions, and the two rewards read above.
    let raw = before_claim();
    let pillar = aero_pillar(&raw.aero);
    let totals = collectible_totals(&raw.aero, &pillar, 0.542769 + 1.407023);
    assert_eq!(totals, CollectibleTotals { rewards_usd: Some("7.84".to_string()), total_usd: Some("9.79".to_string()), note: None });

    // End to end, with LP-claimable fees in the cycle (synthetic: $8 of USDC fees on the
    // basic position). LP fees keep their meaning; the reward does not enter them.
    let with_fees = snapshot_of(with_lp_fees(before_claim(), 8_000_000), &OperatorProfile::default(), 0.0);
    let mut no_aero = with_lp_fees(before_claim(), 8_000_000);
    no_aero.aero = AeroReads::default();
    no_aero.wallet_balances.pop();
    let no_aero = snapshot_of(no_aero, &OperatorProfile::default(), 0.0);
    let (g, g0) = (&with_fees.derived_metrics.global_status, &no_aero.derived_metrics.global_status);
    assert_eq!(g.total_fees_claimable_usd, "8.00");
    assert_eq!(g.collectible_rewards_usd.as_deref(), Some("7.84"));
    assert_eq!(g.total_collectible_usd.as_deref(), Some("15.84"));
    for (a, b) in [
        (&g.total_fees_claimable_usd, &g0.total_fees_claimable_usd),
        (&g.total_fees_accrued_usd, &g0.total_fees_accrued_usd),
        (&g.gas_to_fee_delta_usd, &g0.gas_to_fee_delta_usd),
        (&g.total_portfolio_usd, &g0.total_portfolio_usd),
    ] {
        assert_eq!(a, b);
    }
    // With nothing read (the fixture default) there is no figure and no reason to give.
    assert_eq!((&g0.collectible_rewards_usd, &g0.total_collectible_usd, &g0.collectible_rewards_note), (&None, &None, &None));
    // The printed parts always add up to the printed total.
    for lp in [0.0, 0.004, 0.005, 1.949792, 4.995, 123.456] {
        let t = collectible_totals(&raw.aero, &pillar, lp);
        let sum = f(&format!("{:.2}", lp)) + f(t.rewards_usd.as_deref().unwrap());
        assert_eq!(t.total_usd.as_deref().unwrap(), format!("{:.2}", sum), "lp {lp}");
    }

    // A row with a fee action carries no reward and no kind: no harvest word beside AERO.
    let rows = &with_fees.derived_metrics.velocity_gas_sandbox.contracts;
    assert_eq!(rows[0].recommended_action, "claim_now");
    assert_eq!((&rows[0].earnings_kind, &rows[0].reward, &rows[0].lp_fees_note, &rows[0].note), (&None, &None, &None, &None));
    assert_eq!(rows[1].earnings_kind.as_deref(), Some("gauge_reward"));
    // The position keeps its reward, and its fee row lists it without changing its kind.
    assert_eq!(with_fees.positions[0].collectible_reward.as_ref().unwrap().amount.as_deref(), Some("0.210431"));
    assert_eq!(with_fees.contract_fees[0].earnings_kind, None);
    assert_eq!(with_fees.contract_fees[0].reward, with_fees.positions[0].collectible_reward);
    assert!(with_fees.contract_fees[0].claimable);
}

#[test]
fn test_rewards_total_is_the_summed_amount_at_the_price_not_the_sum_of_rounded_rows() {
    // The real basic reward and AERO price of block 58618045 with a synthetic CL reward
    // of 9.987260 AERO: the rows print $0.09 and $4.38, and the summed amount
    // (10.197691 AERO at $0.438859) is $4.48, one cent more than the rows add up to.
    let earned = |gauge: &str| {
        if gauge.eq_ignore_ascii_case(CL_GAUGE) {
            ok_hex(&format!("{:064x}", 9_987_260_000_000_000_000u128))
        } else {
            ok_hex(REAL_BASIC_EARNED_AT_CLAIM)
        }
    };
    let snap = snapshot_of(cycle(REAL_AERO_BALANCE_WALLET, &price_at_claim, &earned), &OperatorProfile::default(), 0.0);
    let usd = |i: usize| snap.positions[i].collectible_reward.as_ref().unwrap().usd.clone().unwrap();
    assert_eq!((usd(0).as_str(), usd(1).as_str()), ("0.09", "4.38"));
    let g = &snap.derived_metrics.global_status;
    assert_eq!(g.unclaimed_aero_amount.as_deref(), Some("10.197691"));
    // The stated total is the amount at the price: the figure `unclaimed_aero_usd` has
    // carried since #93, so the same AERO never prints with two dollar values.
    assert_eq!(g.collectible_rewards_usd.as_deref(), Some("4.48"));
    assert_eq!(g.collectible_rewards_usd, g.unclaimed_aero_usd);
    assert_eq!(g.collectible_rewards_usd, Some(format!("{:.2}", 10.197691026731433905 * price_at_claim().unwrap())));
    // It is documented as such, and not as the sum of the rounded rows (which is $4.47).
    assert_ne!(f(&usd(0)) + f(&usd(1)), f(g.collectible_rewards_usd.as_deref().unwrap()));
    let spec = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/../specs/core/snapshot.yaml")).unwrap();
    let at = spec.find("        collectible_rewards_usd:").expect("the field is in the spec");
    let description: String = spec[at..at + spec[at..].find("example:").unwrap()].split_whitespace().collect::<Vec<_>>().join(" ");
    assert!(description.contains("summed amount at the cycle's AERO price"), "{description}");
    assert!(description.contains("not the sum of the rounded positions[].collectible_reward.usd values"), "{description}");
    // LP fees + rewards = total still holds to the cent, and one line states both the
    // amount and its value.
    assert_eq!(g.total_collectible_usd.as_deref(), Some("4.48"));
    let advice = format_shot_advice_text(&full(&snap));
    assert!(advice.contains("• Collectible AERO rewards: $4.48 USD (10.197691 AERO in tracked gauges, collected in the Aero app)\n"), "{advice}");
    let state = format_portfolio_state_text(&full(&snap));
    assert!(state.contains("• Unclaimed AERO: 10.197691 AERO ($4.48) in tracked gauges"), "{state}");
    assert!(state.contains("AERO rewards $4.48; total collectible $4.48\n"), "{state}");
}

// ------------------------------------------------------------------
// What must not change
// ------------------------------------------------------------------

#[test]
fn test_unclaimed_reward_never_enters_the_portfolio_total() {
    let profile = example_profile();
    // Nothing read at all, the real rewards, and a synthetic million AERO in the gauge.
    let mut none = before_claim();
    none.aero = AeroReads::default();
    none.wallet_balances.pop();
    let base = snapshot_of(none, &profile, 0.0);
    for raw in [before_claim(), cycle_with_cl_reward(1_000), cycle_with_cl_reward(1_000_000)] {
        let snap = snapshot_of(raw, &profile, 0.0);
        let (g, g0) = (&snap.derived_metrics.global_status, &base.derived_metrics.global_status);
        assert_eq!(g.total_portfolio_usd, g0.total_portfolio_usd);
        assert_eq!(g.high_water_mark_usd, g0.high_water_mark_usd);
        assert_eq!(g.total_native_usd, g0.total_native_usd);
        assert_eq!(g.total_stable_usd, g0.total_stable_usd);
        assert_eq!(g.total_eth_usd, g0.total_eth_usd);
        assert_eq!(g.wallet_aero_usd.as_deref(), Some("0.00"));
        assert_eq!(snap.derived_metrics.lp_rebuild.target_usd, base.derived_metrics.lp_rebuild.target_usd);
        assert_eq!(
            snap.derived_metrics.total_value_vs_baseline.portfolio_value_series,
            base.derived_metrics.total_value_vs_baseline.portfolio_value_series
        );
        // No position's value or fee figure contains it either.
        for (p, p0) in snap.positions.iter().zip(&base.positions) {
            assert_eq!((&p.amount_usd, &p.fees_usd), (&p0.amount_usd, &p0.fees_usd));
        }
        // The four pillars still add up to the total without it.
        let sum = f(&g.total_native_usd) + f(&g.total_stable_usd) + f(g.total_eth_usd.as_deref().unwrap()) + f(g.wallet_aero_usd.as_deref().unwrap());
        assert!((sum - f(&g.total_portfolio_usd)).abs() < 0.03, "{sum} vs {}", g.total_portfolio_usd);
    }
    // A million AERO is stated as collectible, and only there.
    let big = snapshot_of(cycle_with_cl_reward(1_000_000), &profile, 0.0);
    let g = &big.derived_metrics.global_status;
    assert_eq!(g.collectible_rewards_usd.as_deref(), Some("438859.11"));
    assert_eq!(g.total_collectible_usd, g.collectible_rewards_usd);
    // Once claimed it is wallet AERO, which is counted (the real cycle after the claim).
    let after = snapshot_of(after_claim(), &profile, 0.0);
    let ga = &after.derived_metrics.global_status;
    assert_eq!(ga.wallet_aero_amount.as_deref(), Some("17.654015"));
    assert_eq!(ga.wallet_aero_usd.as_deref(), Some("7.75"));
    let step = f(&ga.total_portfolio_usd) - f(&base.derived_metrics.global_status.total_portfolio_usd);
    assert!((step - 7.75).abs() < 0.011, "the total steps up by the claimed AERO, once: {step}");
}

#[test]
fn test_the_directive_is_unchanged_by_any_reward_size() {
    let regimes = [MarketRegime::default(), MarketRegime { rsi_5m: Some(80.0), rsi_1d: Some(75.0), overextended: true }];
    // LP fees just under and just over the fixed $5.00 harvest threshold, and none.
    for lp_fees_raw in [0u128, 4_990_000, 5_010_000] {
        let build = |raw: RawOnChainData| if lp_fees_raw == 0 { raw } else { with_lp_fees(raw, lp_fees_raw) };
        for profile in [OperatorProfile::default(), example_profile()] {
            for regime in &regimes {
                // The reference: the same cycle with every gauge answering zero.
                let zero = build(cycle(REAL_AERO_BALANCE_WALLET, &price_at_claim, &|_| ok_hex(REAL_CL_EARNED_AFTER_CLAIM)));
                let reference = metrics(&zero, &profile, regime);
                let g0 = &reference.global_status;
                for raw in [build(before_claim()), build(cycle_with_cl_reward(12)), build(cycle_with_cl_reward(1_000)), build(cycle_with_cl_reward(1_000_000))] {
                    let m = metrics(&raw, &profile, regime);
                    let g = &m.global_status;
                    assert_eq!(g.shot_advice_action, g0.shot_advice_action, "fees {lp_fees_raw}");
                    assert_eq!(g.shot_advice_details, g0.shot_advice_details, "fees {lp_fees_raw}");
                    assert!(!g.shot_advice_action.contains("AERO") && !g.shot_advice_details.contains("AERO"), "{}", g.shot_advice_details);
                    // What the harvest rule reads is LP fees, and they did not move.
                    assert_eq!(g.total_fees_claimable_usd, g0.total_fees_claimable_usd);
                    assert_eq!(g.gas_to_fee_delta_usd, g0.gas_to_fee_delta_usd);
                    assert_eq!(m.lp_rebuild, reference.lp_rebuild);
                    assert_eq!(m.accumulation_plan, reference.accumulation_plan);
                    // The LP-fee action of every row is the one it had.
                    let actions = |d: &DerivedMetrics| d.velocity_gas_sandbox.contracts.iter().map(|c| c.recommended_action.clone()).collect::<Vec<_>>();
                    assert_eq!(actions(&m), actions(&reference));
                    // No notification can come from it: the notifier compares directives.
                    assert_eq!(
                        cycle_notification(Some(&g0.shot_advice_action), &g.shot_advice_action, &g.shot_advice_details, None, None, None, None),
                        None
                    );
                }
            }
        }
    }
    // $4.99 of LP fees and a million AERO collectible: still no harvest directive.
    let m = metrics(&with_lp_fees(cycle_with_cl_reward(1_000_000), 4_990_000), &OperatorProfile::default(), &MarketRegime::default());
    assert!(!m.global_status.shot_advice_action.contains("HARVEST"), "{}", m.global_status.shot_advice_action);
    assert_eq!(m.global_status.total_fees_claimable_usd, "4.99");
    assert_eq!(m.global_status.total_collectible_usd.as_deref(), Some("438864.10"));
    // $5.01 of LP fees: the harvest directive, with or without a reward, and it names LP fees only.
    let due = metrics(&with_lp_fees(before_claim(), 5_010_000), &OperatorProfile::default(), &MarketRegime::default());
    assert_eq!(due.global_status.shot_advice_action, "BATCH HARVEST SHOT (Execute Claim)");
    assert!(due.global_status.shot_advice_details.starts_with("Uncollected LP fees ($5.01)"), "{}", due.global_status.shot_advice_details);
}

#[test]
fn test_no_advice_word_attaches_to_aero_on_any_surface() {
    let cases = [
        snapshot_of(before_claim(), &example_profile(), 0.0),
        snapshot_of(after_claim(), &example_profile(), 0.0),
        snapshot_of(with_lp_fees(before_claim(), 8_000_000), &example_profile(), 0.0),
        snapshot_of(cycle_with_cl_reward(1_000_000), &OperatorProfile::default(), 0.0),
        // A gauge that could not be read, and an unpriced reward.
        snapshot_of(
            cycle(REAL_AERO_BALANCE_WALLET, &price_at_claim, &|g| {
                if g.eq_ignore_ascii_case(CL_GAUGE) { Err(anyhow::anyhow!("network error: timed out")) } else { ok_hex(REAL_BASIC_EARNED_AT_CLAIM) }
            }),
            &example_profile(),
            0.0,
        ),
        snapshot_of(
            cycle(REAL_AERO_BALANCE_WALLET, &|| Err(anyhow::anyhow!("AERO price pool slot0() read failed")), &earned_with(REAL_CL_EARNED_BEFORE_CLAIM)),
            &example_profile(),
            0.0,
        ),
    ];
    let every_gas = [
        RewardGas::NotRequested,
        RewardGas::NotVerified,
        RewardGas::Estimated { units: 390_860, usd: Some(0.0063) },
        RewardGas::Estimated { units: 390_860, usd: None },
        RewardGas::Failed("the estimate did not return in time".to_string()),
    ];
    for snap in &cases {
        let data = full(snap);
        // 1. The snapshot itself: a reward never sits under a harvest word.
        for row in &snap.derived_metrics.velocity_gas_sandbox.contracts {
            if row.reward.is_some() {
                assert_eq!(row.recommended_action, "not_claimable", "{row:?}");
                assert_eq!(row.earnings_kind.as_deref(), Some("gauge_reward"));
            }
            for word in ["claim_now", "claim_recommended", "accumulate"] {
                assert_ne!(row.earnings_kind.as_deref(), Some(word));
                if row.recommended_action == word {
                    assert!(row.reward.is_none() && row.earnings_kind.is_none(), "{row:?}");
                    assert!(!row.note.as_deref().unwrap_or("").contains("AERO"), "{row:?}");
                }
            }
        }
        for row in &snap.contract_fees {
            assert!(matches!(row.earnings_kind.as_deref(), None | Some("gauge_reward")), "{row:?}");
        }
        // Every string of every reward object, and the note beside the totals.
        let mut strings = Vec::new();
        for p in &snap.positions {
            if let Some(r) = p.collectible_reward.as_ref() {
                all_strings(&serde_json::to_value(r).unwrap(), &mut strings);
                strings.push(reward_detail(p).unwrap());
            }
        }
        strings.extend(snap.derived_metrics.global_status.collectible_rewards_note.clone());
        // 2. Every text surface: each line that speaks of AERO or a reward.
        let mut texts = vec![
            format_portfolio_state_text(&data),
            format_shot_advice_text(&data),
            format_accumulation_plan_text(&data),
            collectible_advice_lines(&data),
            collectible_earnings_line(&data),
        ];
        for gas in &every_gas {
            texts.push(claim_simulation_text(&data, &vec![gas.clone(); snap.positions.len()]));
            strings.push(reward_gas_text(gas, false));
            strings.push(reward_gas_text(gas, true));
            // 3. The claim payload's reward rows.
            for row in claim_reward_rows(&snap.positions, &vec![gas.clone(); snap.positions.len()]) {
                all_strings(&serde_json::to_value(&row).unwrap(), &mut strings);
            }
        }
        for text in &texts {
            for line in aero_lines_of(text) {
                assert_no_advice(line);
            }
        }
        for s in &strings {
            assert_no_advice(s);
        }
        // The directive never names AERO.
        let g = &snap.derived_metrics.global_status;
        assert!(!g.shot_advice_action.contains("AERO") && !g.shot_advice_details.contains("AERO"));
    }
    // The link is the Aero app, never the retired one.
    let data = full(&cases[0]);
    let mut strings = Vec::new();
    all_strings(&data, &mut strings);
    assert!(strings.iter().any(|s| s == "https://app.aero.xyz"));
    assert!(strings.iter().all(|s| !s.contains("aerodrome.finance")));
}

// ------------------------------------------------------------------
// Not read, unpriced, zero
// ------------------------------------------------------------------

#[test]
fn test_a_failed_gauge_read_is_not_read_and_the_total_is_null() {
    // The CL gauge's read fails (synthetic failure; the basic gauge's real answer stands).
    for failure in [
        (|| -> Result<Option<String>> { Err(anyhow::anyhow!("network error: operation timed out")) }) as fn() -> Result<Option<String>>,
        || Ok(None),
    ] {
        let raw = cycle(REAL_AERO_BALANCE_WALLET, &price_at_claim, &|g| {
            if g.eq_ignore_ascii_case(CL_GAUGE) { failure() } else { ok_hex(REAL_BASIC_EARNED_AT_CLAIM) }
        });
        let snap = snapshot_of(raw, &OperatorProfile::default(), 0.0);
        let cl = snap.positions[1].collectible_reward.as_ref().unwrap();
        assert_eq!(cl.status, "not_read");
        assert_eq!((&cl.amount, &cl.usd), (&None, &None));
        assert_eq!(cl.gauge.as_deref(), Some(CL_GAUGE));
        let reason = cl.note.clone().unwrap();
        assert!(reason.starts_with("gauge 0x3bf70c880adc8370917994ab0df68e763b7a60fd earned() "), "{reason}");
        assert_eq!(snap.positions[1].unclaimed_aero_note.as_deref(), Some(reason.as_str()));
        // The gauge that answered keeps its own figure.
        let basic = snap.positions[0].collectible_reward.as_ref().unwrap();
        assert_eq!((basic.status.as_str(), basic.amount.as_deref(), basic.usd.as_deref()), ("read", Some("0.210431"), Some("0.09")));

        // No total: not 0, and not the $0.09 of the gauge that was read.
        let g = &snap.derived_metrics.global_status;
        assert_eq!((&g.collectible_rewards_usd, &g.total_collectible_usd), (&None, &None));
        assert_eq!(
            g.collectible_rewards_note.as_deref(),
            Some(format!("gauge reward not read for aerodrome-cl:base:2914490 ({reason})").as_str())
        );
        // LP fees are still stated.
        assert_eq!(g.total_fees_claimable_usd, "0.00");
        // The velocity row says "not read" in its reward, under the same kind.
        let row = &snap.derived_metrics.velocity_gas_sandbox.contracts[1];
        assert_eq!((row.earnings_kind.as_deref(), row.recommended_action.as_str()), (Some("gauge_reward"), "not_claimable"));
        assert_eq!(row.reward.as_ref().unwrap().status, "not_read");

        let data = full(&snap);
        let advice = format_shot_advice_text(&data);
        assert!(advice.contains(&format!("• Collectible AERO rewards: not totalled (gauge reward not read for aerodrome-cl:base:2914490 ({reason}))\n")), "{advice}");
        assert!(advice.contains("• Total collectible (LP fees + AERO rewards): not totalled (the AERO rewards part is not totalled)\n"), "{advice}");
        let claim = format_claim_simulation_text(&data);
        assert!(
            claim.contains(&format!("  - WETH+NOCK (Aero CL #2914490): reward not read ({reason}); LP fees not LP-claimable (staked: swap fees on staked liquidity go to the gauge)\n")),
            "{claim}"
        );
        assert!(claim.contains("• Collectible AERO rewards: not totalled ("), "{claim}");
        let state = format_portfolio_state_text(&data);
        assert!(state.contains("AERO rewards not totalled (gauge reward not read for aerodrome-cl:base:2914490"), "{state}");
        // Nowhere is the unread reward a zero or the partial sum a total.
        for text in [&advice, &claim, &state] {
            for line in text.lines().filter(|l| l.contains("Collectible AERO rewards") || l.contains("Total collectible") || l.contains("Collectible earnings")) {
                assert!(!line.contains("AERO rewards: $") && !line.contains("AERO rewards $") && !line.contains("0.09"), "{line}");
            }
            assert!(!text.contains("#2914490): reward 0"), "{text}");
        }
        // The payload row: not read, no figure, no gas.
        let rows = claim_reward_rows(&snap.positions, &[RewardGas::NotRequested, RewardGas::NotRequested]);
        assert_eq!((rows[1].status.as_str(), &rows[1].amount, &rows[1].usd, &rows[1].gas_estimate), ("not_read", &None, &None, &None));
        assert_eq!(rows[1].gas_note, "gas not estimated (the reward was not read)");
        assert_eq!(reward_gas_request(&snap.positions[1], WALLET), Err(RewardGas::NotRequested));
    }

    // An Aero basic position with no gauge configured: the same, with that reason.
    let no_gauge = real_cycle_with(&|_, facts| {
        aero_reads_from(&[basic_config(None), cl_config()], facts, REAL_AERO_BALANCE_WALLET, price_at_claim(), &earned_with(REAL_CL_EARNED_BEFORE_CLAIM)).unwrap()
    });
    let snap = snapshot_of(no_gauge, &OperatorProfile::default(), 0.0);
    let basic = snap.positions[0].collectible_reward.as_ref().unwrap();
    assert_eq!((basic.status.as_str(), &basic.amount, &basic.gauge), ("not_read", &None, &None));
    assert_eq!(basic.note.as_deref(), Some("no gauge_address is configured for this position"));
    let g = &snap.derived_metrics.global_status;
    assert_eq!((&g.collectible_rewards_usd, &g.total_collectible_usd), (&None, &None));
    assert!(g.collectible_rewards_note.as_deref().unwrap().contains("no gauge_address is configured"), "{:?}", g.collectible_rewards_note);
}

#[test]
fn test_an_unpriced_reward_has_an_amount_and_a_null_usd() {
    let failed = || -> Result<f64> { Err(anyhow::anyhow!("AERO price pool 0xd0acf293547457fb4fe9ee32ca93e89497626816 slot0() read failed")) };
    let snap = snapshot_of(cycle(REAL_AERO_BALANCE_WALLET, &failed, &earned_with(REAL_CL_EARNED_BEFORE_CLAIM)), &OperatorProfile::default(), 0.0);
    for (p, amount) in snap.positions.iter().zip(["0.210431", "17.654015"]) {
        let r = p.collectible_reward.as_ref().unwrap();
        assert_eq!((r.status.as_str(), r.amount.as_deref(), &r.usd, &r.note), ("read", Some(amount), &None, &None));
    }
    let g = &snap.derived_metrics.global_status;
    assert_eq!(g.unclaimed_aero_amount.as_deref(), Some("17.864446"));
    assert_eq!((&g.collectible_rewards_usd, &g.total_collectible_usd), (&None, &None));
    assert_eq!(
        g.collectible_rewards_note.as_deref(),
        Some("17.864446 AERO in tracked gauges has no USD value: AERO/USD price unavailable: AERO price pool 0xd0acf293547457fb4fe9ee32ca93e89497626816 slot0() read failed")
    );
    let data = full(&snap);
    let claim = format_claim_simulation_text(&data);
    assert!(
        claim.contains("  - WETH+NOCK (Aero CL #2914490): reward 17.654015 AERO (USD value unavailable), collectible from gauge 0x3bf70c880adc8370917994ab0df68e763b7a60fd in the Aero app | "),
        "{claim}"
    );
    assert!(claim.contains("• Total collectible (LP fees + AERO rewards, before gas): not totalled (the AERO rewards part is not totalled)\n"), "{claim}");
    assert!(!claim.contains("AERO ($0.00)") && !claim.contains("AERO rewards: $"), "{claim}");
    // A reward that is zero needs no price: it is worth $0 exactly (the real cycle after
    // the claim, with the same failed price read; the wallet's AERO is left at zero here
    // because a failed price fails the cycle while the wallet holds AERO).
    let zero = snapshot_of(cycle(REAL_AERO_BALANCE_WALLET, &failed, &|_| ok_hex(REAL_CL_EARNED_AFTER_CLAIM)), &OperatorProfile::default(), 0.0);
    let r = zero.positions[1].collectible_reward.as_ref().unwrap();
    assert_eq!((r.amount.as_deref(), r.usd.as_deref()), (Some("0.000000"), Some("0.00")));
    assert_eq!(zero.derived_metrics.global_status.collectible_rewards_usd.as_deref(), Some("0.00"));
    // Every gauge answered zero: the total says there is nothing to collect, and does
    // not describe 0.000000 AERO as collected anywhere.
    let advice = format_shot_advice_text(&full(&zero));
    assert!(advice.contains("• Collectible AERO rewards: $0.00 USD (0.000000 AERO in tracked gauges: nothing to collect)\n"), "{advice}");
}

#[test]
fn test_after_the_claim_the_reward_is_a_read_zero_and_the_wallet_holds_it() {
    let snap = snapshot_of(after_claim(), &OperatorProfile::default(), 0.0);
    let cl = snap.positions[1].collectible_reward.as_ref().unwrap();
    assert_eq!((cl.status.as_str(), cl.amount.as_deref(), cl.usd.as_deref(), &cl.note), ("read", Some("0.000000"), Some("0.00"), &None));
    let g = &snap.derived_metrics.global_status;
    assert_eq!(g.collectible_rewards_usd.as_deref(), Some("0.09"));
    assert_eq!(g.total_collectible_usd.as_deref(), Some("0.09"));
    assert_eq!(g.wallet_aero_amount.as_deref(), Some("17.654015"));
    let data = full(&snap);
    // A read zero is stated as nothing to collect, never as an amount "collectible in
    // the Aero app", and no gas statement is attached to it, whatever was estimated.
    for gas_for_cl in [RewardGas::NotRequested, RewardGas::Estimated { units: 418_528, usd: Some(0.0067) }] {
        let claim = claim_simulation_text(&data, &[RewardGas::Estimated { units: 390_860, usd: Some(0.0063) }, gas_for_cl]);
        assert!(
            claim.contains("  - WETH+NOCK (Aero CL #2914490): reward 0.000000 AERO ($0.00), nothing to collect in gauge 0x3bf70c880adc8370917994ab0df68e763b7a60fd; LP fees not LP-claimable (staked: swap fees on staked liquidity go to the gauge)\n"),
            "{claim}"
        );
    }
    let claim = claim_simulation_text(&data, &[RewardGas::Estimated { units: 390_860, usd: Some(0.0063) }, RewardGas::NotRequested]);
    let cl_value = serde_json::to_value(cl).unwrap();
    assert_eq!(reward_statement(&cl_value), "0.000000 AERO ($0.00), nothing to collect in gauge 0x3bf70c880adc8370917994ab0df68e763b7a60fd");
    assert_eq!(reward_where_text(&cl_value), "nothing to collect in gauge 0x3bf70c880adc8370917994ab0df68e763b7a60fd");
    // Every surface built from the statement: the position row of get_portfolio_state,
    // the claim action's detail and the payload row.
    let state = format_portfolio_state_text(&data);
    assert!(
        state.contains(
            "   - Aero USDC/NOCK: 0.210431 AERO ($0.09), collectible from gauge 0xc82782ee66d82c4d19db0429652091a085083526 in the Aero app\n   \
             - Aero CL WETH/NOCK #2914490: 0.000000 AERO ($0.00), nothing to collect in gauge 0x3bf70c880adc8370917994ab0df68e763b7a60fd\n"
        ),
        "{state}"
    );
    assert_eq!(
        reward_detail(&snap.positions[1]).as_deref(),
        Some("reward 0.000000 AERO ($0.00), nothing to collect in gauge 0x3bf70c880adc8370917994ab0df68e763b7a60fd")
    );
    let rows = claim_reward_rows(&snap.positions, &[]);
    assert_eq!((rows[1].amount.as_deref(), rows[1].gas_note.as_str()), (Some("0.000000"), "nothing to collect, gas not estimated"));
    for text in [&claim, &state] {
        assert!(!text.contains("($0.00), collectible"), "a zero is not described as collectible: {text}");
    }
    // The gauge that still holds a reward keeps its gas statement.
    assert!(claim.contains("  - USDC+NOCK (Aerodrome): reward 0.210431 AERO ($0.09), collectible from gauge 0xc82782ee66d82c4d19db0429652091a085083526 in the Aero app | gas est. 390860 units (~$0.0063); LP fees not LP-claimable (claimFees() reverts)\n"), "{claim}");
    // And no estimate is asked for it.
    assert_eq!(reward_gas_request(&snap.positions[1], WALLET), Err(RewardGas::NotRequested));
}

// ------------------------------------------------------------------
// Gas: only for a proven claim call
// ------------------------------------------------------------------

#[test]
fn test_gas_is_stated_only_for_a_verified_claim_call() {
    let snap = snapshot_of(before_claim(), &OperatorProfile::default(), 0.0);
    // Both of the operator's gauges have a proven call: that call, to that gauge.
    assert_eq!(
        reward_gas_request(&snap.positions[1], WALLET),
        Ok((REAL_OPERATOR_CLAIM_INPUT.to_string(), CL_GAUGE.to_string()))
    );
    let (basic_call, basic_gauge) = reward_gas_request(&snap.positions[0], WALLET).unwrap();
    assert_eq!(basic_gauge, BASIC_GAUGE);
    assert!(basic_call.starts_with("0x8ed86282") && basic_call.len() == 10 + 128);

    // The same reward on a gauge with no proven call: no estimate is made (synthetic gauge).
    let mut other = snap.positions[1].clone();
    other.collectible_reward.as_mut().unwrap().gauge = Some("0x1111111111111111111111111111111111111111".to_string());
    assert_eq!(reward_gas_request(&other, WALLET), Err(RewardGas::NotVerified));
    assert_eq!(reward_gas_text(&RewardGas::NotVerified, false), "gas not estimated (claim call not verified for this gauge)");
    // A position with no reward at all asks for nothing.
    let mut none = snap.positions[1].clone();
    none.collectible_reward = None;
    assert_eq!(reward_gas_request(&none, WALLET), Err(RewardGas::NotRequested));

    // The real estimate for the basic gauge, at the gas price the operator's claim paid
    // and the morning's ETH/USD: 390,860 gas, $0.0063.
    let est = reward_gas_from_estimate(&Ok(Some(REAL_ESTIMATE_BASIC_CLAIM.to_string())), Some(REAL_OPERATOR_CLAIM_GAS_PRICE), Some(ETH_USD_2026_11_17));
    let RewardGas::Estimated { units, usd } = est.clone() else { panic!("{est:?}") };
    assert_eq!(units, 390_860);
    assert!((usd.unwrap() - 0.0062976).abs() < 1e-6, "{usd:?}");
    assert_eq!(reward_gas_text(&est, false), "gas est. 390860 units (~$0.0063)");
    // The operator's real claim used 418,528 gas at that price: $0.0067, the same order.
    let real_cost = REAL_OPERATOR_CLAIM_GAS_USED as f64 * REAL_OPERATOR_CLAIM_GAS_PRICE as f64 / 1e18 * ETH_USD_2026_11_17;
    assert!((real_cost - 0.0067434).abs() < 1e-6, "{real_cost}");
    // Without a gas price or an ETH price there are units and no dollars, never a default.
    for (gp, ep) in [(None, Some(ETH_USD_2026_11_17)), (Some(REAL_OPERATOR_CLAIM_GAS_PRICE), None), (Some(REAL_OPERATOR_CLAIM_GAS_PRICE), Some(0.0))] {
        let g = reward_gas_from_estimate(&Ok(Some("5f6cc".to_string())), gp, ep);
        assert_eq!(g, RewardGas::Estimated { units: 390_860, usd: None });
        assert_eq!(reward_gas_text(&g, false), "gas est. 390860 units (USD value unavailable)");
    }
    // A revert, a failed call or a non-quantity is "not estimated" with the reason.
    let reverted = reward_gas_from_estimate(&Ok(None), Some(1), Some(1.0));
    assert_eq!(reward_gas_text(&reverted, false), "gas not estimated (the claim call reverts when estimated from the wallet)");
    let failed = reward_gas_from_estimate(&Err(anyhow::anyhow!("network error: connection refused")), Some(1), Some(1.0));
    assert_eq!(reward_gas_text(&failed, false), "gas not estimated (the estimate was not answered: network error: connection refused)");
    assert!(matches!(reward_gas_from_estimate(&Ok(Some("0x".to_string())), Some(1), Some(1.0)), RewardGas::Failed(_)));
    assert!(matches!(reward_gas_from_estimate(&Ok(Some("0x0".to_string())), Some(1), Some(1.0)), RewardGas::Failed(_)));
    // Text built from a stored snapshot alone says that no estimate was made.
    assert_eq!(reward_gas_text(&RewardGas::NotRequested, false), "gas not estimated (no live estimate was made for this text)");

    // The payload rows carry the estimate only where there is one.
    let rows = claim_reward_rows(&snap.positions, &[est, RewardGas::NotVerified]);
    assert_eq!((rows[0].gas_estimate.as_deref(), rows[0].gas_usd.as_deref()), (Some("0x5f6cc"), Some("0.0063")));
    assert_eq!(rows[0].gas_note, "gas est. 390860 units (~$0.0063)");
    assert_eq!((&rows[1].gas_estimate, &rows[1].gas_usd), (&None, &None));
    assert_eq!(rows[1].gas_note, "gas not estimated (claim call not verified for this gauge)");
}

#[tokio::test]
async fn test_a_failed_live_estimate_never_shows_the_rpc_url() {
    // The real function against a port nothing listens on. The URL carries a key.
    let url = "http://127.0.0.1:9/v2/SECRET-RPC-KEY";
    let snap = snapshot_of(before_claim(), &OperatorProfile::default(), 0.0);
    let started = std::time::Instant::now();
    let gas = estimate_reward_claim_gas(&Client::new(), url, WALLET, &snap.positions, Some(ETH_USD_2026_11_17), GasPrice::ReadNow, std::time::Duration::from_millis(1500)).await;
    assert!(started.elapsed() < std::time::Duration::from_millis(2000), "the estimate is bounded: {:?}", started.elapsed());
    assert_eq!(gas.len(), 2);
    for g in &gas {
        let RewardGas::Failed(why) = g else { panic!("no estimate can come from a refused port: {g:?}") };
        let text = reward_gas_text(g, false);
        for secret in ["SECRET-RPC-KEY", "127.0.0.1", "/v2/", "http://"] {
            assert!(!why.contains(secret) && !text.contains(secret), "{text}");
        }
        assert!(text.starts_with("gas not estimated ("), "{text}");
    }
    // A snapshot with nothing to estimate makes no call at all (an unroutable URL returns at once).
    let none = snapshot_of(after_claim(), &OperatorProfile::default(), 0.0);
    let mut positions = none.positions.clone();
    positions[0].collectible_reward = None;
    let quick = estimate_reward_claim_gas(&Client::new(), url, WALLET, &positions, None, GasPrice::ReadNow, std::time::Duration::from_millis(1500)).await;
    assert_eq!(quick, vec![RewardGas::NotRequested, RewardGas::NotRequested]);
    // No key configured: no URL is built.
    assert_eq!(request_rpc_url(&[]), None);
    assert_eq!(request_rpc_url(&["YOUR_ALCHEMY_API_KEY".to_string()]), None);
    assert_eq!(request_rpc_url(&[" ".to_string()]), None);
    assert_eq!(request_rpc_url(&["abc".to_string()]).as_deref(), Some("https://base-mainnet.g.alchemy.com/v2/abc"));
}

/// A local JSON-RPC server for the request-time estimate. `eth_estimateGas` is answered
/// per gauge: the basic gauge with the real estimate of 2026-11-17 (0x5f6cc), the CL
/// gauge with the gas the operator's real claim used (0x662e0). `eth_gasPrice` is
/// answered with `gas_price_hex`. Anything else is an RPC error, never an invented
/// answer. Returns the URL and the method of each request received.
async fn estimate_rpc(gas_price_hex: &'static str) -> (String, Arc<std::sync::Mutex<Vec<String>>>) {
    let seen: Arc<std::sync::Mutex<Vec<String>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a local port");
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handler = {
        let seen = seen.clone();
        move |Json(req): Json<serde_json::Value>| {
            let seen = seen.clone();
            async move {
                let method = req["method"].as_str().unwrap_or_default().to_string();
                seen.lock().unwrap().push(method.clone());
                let to = req["params"][0]["to"].as_str().unwrap_or_default().to_lowercase();
                let result = match (method.as_str(), to.as_str()) {
                    ("eth_gasPrice", _) => Some(gas_price_hex.to_string()),
                    ("eth_estimateGas", BASIC_GAUGE) => Some(REAL_ESTIMATE_BASIC_CLAIM.to_string()),
                    ("eth_estimateGas", CL_GAUGE) => Some(format!("0x{:x}", REAL_OPERATOR_CLAIM_GAS_USED)),
                    _ => None,
                };
                Json(match result {
                    Some(r) => json!({ "jsonrpc": "2.0", "id": req["id"], "result": r }),
                    None => json!({ "jsonrpc": "2.0", "id": req["id"], "error": { "code": -32000, "message": "no answer for this call" } }),
                })
            }
        }
    };
    let app = axum::Router::new().route("/", axum::routing::post(handler));
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (url, seen)
}

#[tokio::test]
async fn test_the_callers_gas_price_is_used_and_not_read_a_second_time() {
    // The claim payload has already read eth_gasPrice for its LP actions. The reward rows
    // are priced at that same price: no second read, which could land on another base fee.
    // The server would answer eth_gasPrice with ten times the caller's price (synthetic),
    // so a second read would show in the dollars.
    let (url, seen) = estimate_rpc("0x55d4a80").await;
    let snap = snapshot_of(before_claim(), &OperatorProfile::default(), 0.0);
    let budget = std::time::Duration::from_millis(1500);
    let known = GasPrice::Known(Some(REAL_OPERATOR_CLAIM_GAS_PRICE));
    let gas = estimate_reward_claim_gas(&Client::new(), &url, WALLET, &snap.positions, Some(ETH_USD_2026_11_17), known, budget).await;
    let methods = seen.lock().unwrap().clone();
    assert_eq!(methods, vec!["eth_estimateGas".to_string(), "eth_estimateGas".to_string()], "no eth_gasPrice read");
    assert_eq!(reward_gas_text(&gas[0], false), "gas est. 390860 units (~$0.0063)");
    assert_eq!(reward_gas_text(&gas[1], false), "gas est. 418528 units (~$0.0067)");

    // The caller's own read failed: units and no dollars, and still no second read
    // that would price the reward rows when the LP actions of the same payload are not.
    seen.lock().unwrap().clear();
    let gas = estimate_reward_claim_gas(&Client::new(), &url, WALLET, &snap.positions, Some(ETH_USD_2026_11_17), GasPrice::Known(None), budget).await;
    assert_eq!(seen.lock().unwrap().iter().filter(|m| m.as_str() == "eth_gasPrice").count(), 0);
    assert_eq!(gas, vec![RewardGas::Estimated { units: 390_860, usd: None }, RewardGas::Estimated { units: 418_528, usd: None }]);

    // A caller with no gas price of its own (the HTTP MCP): exactly one read, and the
    // node's price is the one used.
    seen.lock().unwrap().clear();
    let gas = estimate_reward_claim_gas(&Client::new(), &url, WALLET, &snap.positions, Some(ETH_USD_2026_11_17), GasPrice::ReadNow, budget).await;
    let methods = seen.lock().unwrap().clone();
    assert_eq!(methods, vec!["eth_gasPrice".to_string(), "eth_estimateGas".to_string(), "eth_estimateGas".to_string()]);
    assert_eq!(reward_gas_text(&gas[0], false), "gas est. 390860 units (~$0.0630)");

    // The claim payload handler passes the price it read for its LP actions.
    let claims = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/claims.rs")).unwrap();
    assert!(claims.contains("GasPrice::Known(gas_price_wei)"), "claims.rs reuses its own eth_gasPrice read");
    assert_eq!(claims.matches("\"eth_gasPrice\"").count(), 1, "claims.rs reads the gas price once");
}

// ------------------------------------------------------------------
// The claim payload: stated, never built
// ------------------------------------------------------------------

#[test]
fn test_claim_payload_states_rewards_and_builds_no_transaction_for_them() {
    let snap = snapshot_of(before_claim(), &OperatorProfile::default(), 0.0);
    let action = |simulation: &str, detail: Option<&str>| ClaimFeeAction {
        platform: "aerodrome-cl".to_string(),
        label: "Collect Aero CL LP fees (aero-cl200-weth-nock #2914490)".to_string(),
        to: "0x4857db576ee0e45f10525ec59527bfd10beec60b".to_string(),
        data: build_v3_collect_calldata(NFT_ID, WALLET),
        value: "0x0".to_string(),
        expected_amount_0: None,
        expected_amount_1: None,
        expected_usd: None,
        gas_estimate: None,
        gas_usd: None,
        simulation: simulation.to_string(),
        simulation_detail: detail.map(|d| d.to_string()),
    };
    // The staked CL position: the existing reason, then the reward and where it is collected.
    let mut cl = action("unavailable", None);
    assert!(aero_cl_mark_not_claimable(&mut cl, Some(&snap.positions[1])));
    state_reward_on_not_claimable(&mut cl, Some(&snap.positions[1]));
    assert_eq!(cl.simulation, "not_claimable");
    assert_eq!(
        cl.simulation_detail.as_deref(),
        Some("not LP-claimable (staked: swap fees on staked liquidity go to the gauge); reward 17.654015 AERO ($7.75), collectible from gauge 0x3bf70c880adc8370917994ab0df68e763b7a60fd in the Aero app")
    );
    // Its calldata is still the LP-fee collect, addressed to the position manager.
    assert_eq!(cl.data, build_v3_collect_calldata(NFT_ID, WALLET));
    assert!(!cl.data.starts_with(GAUGE_CLAIM_CL) && !cl.to.eq_ignore_ascii_case(CL_GAUGE));
    // The Aero basic pool whose claimFees() reverts.
    let (sim, detail) = aero_claim_simulation(&Ok(None));
    let mut basic = action(sim, detail.as_deref());
    state_reward_on_not_claimable(&mut basic, Some(&snap.positions[0]));
    assert_eq!(
        basic.simulation_detail.as_deref(),
        Some("not LP-claimable (claimFees() reverts); reward 0.210431 AERO ($0.09), collectible from gauge 0xc82782ee66d82c4d19db0429652091a085083526 in the Aero app")
    );
    // An action that can execute, or one that is unavailable, is left exactly as built.
    for simulation in ["success", "failed", "unavailable"] {
        let mut a = action(simulation, Some("x"));
        state_reward_on_not_claimable(&mut a, Some(&snap.positions[1]));
        assert_eq!(a.simulation_detail.as_deref(), Some("x"));
    }
    // A position with no reward object (a snapshot from before #142) adds nothing.
    let mut old = snap.positions[1].clone();
    old.collectible_reward = None;
    let mut a = action("not_claimable", Some("not LP-claimable (staked: swap fees on staked liquidity go to the gauge)"));
    state_reward_on_not_claimable(&mut a, Some(&old));
    assert_eq!(a.simulation_detail.as_deref(), Some("not LP-claimable (staked: swap fees on staked liquidity go to the gauge)"));

    // The payload's reward rows: facts, with no field a wallet could be sent.
    let rows = claim_reward_rows(&snap.positions, &[]);
    assert_eq!(rows.len(), 2);
    assert_eq!(
        serde_json::to_value(&rows[1]).unwrap(),
        json!({
            "position_id": "aerodrome-cl:base:2914490", "label": "WETH+NOCK (Aero CL #2914490)", "status": "read",
            "token_symbol": "AERO", "amount": "17.654015", "usd": "7.75",
            "gauge": "0x3bf70c880adc8370917994ab0df68e763b7a60fd", "collected_in": "the Aero app",
            "collect_url": "https://app.aero.xyz", "note": null, "gas_estimate": null, "gas_usd": null,
            "gas_note": "gas not estimated (no live estimate was made for this text)"
        })
    );
    for row in &rows {
        let v = serde_json::to_value(row).unwrap();
        for key in ["to", "data", "value", "calldata", "simulation"] {
            assert!(v.get(key).is_none(), "a reward row must not be dispatchable: it has '{key}'");
        }
    }
    // Totals for LP actions ignore rewards: they are computed from the actions alone.
    assert_eq!(claim_payload_totals(&[cl, basic], "0.00", true), (None, None));

    // The handler never puts a claim call in an action: the only use of the proven
    // calldata is the read-only estimate in rewards.rs.
    let claims_src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/claims.rs")).unwrap();
    for needle in ["verified_reward_claim_call", "GAUGE_CLAIM_BASIC", "GAUGE_CLAIM_CL", "0x8ed86282", "0x09902477", "reward_gas_request"] {
        assert!(!claims_src.contains(needle), "claims.rs must not build a reward claim: found {needle}");
    }
    assert!(claims_src.contains("rewards,") && claims_src.contains("state_reward_on_not_claimable("));
    let rewards_src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/rewards.rs")).unwrap();
    assert_eq!(rewards_src.matches("\"eth_estimateGas\"").count(), 1);
    assert!(!rewards_src.contains("eth_sendRawTransaction") && !rewards_src.contains("eth_sendTransaction"));
}

// ------------------------------------------------------------------
// The MCP text, both transports (one source file)
// ------------------------------------------------------------------

#[test]
fn test_mcp_texts_state_lp_fees_rewards_and_the_total_each_labelled() {
    // $8 of LP-claimable fees on the basic position (synthetic) beside the real rewards.
    let snap = snapshot_of(with_lp_fees(before_claim(), 8_000_000), &OperatorProfile::default(), 0.0);
    let data = full(&snap);

    let advice = format_shot_advice_text(&data);
    assert!(
        advice.contains(
            "• Uncollected Claimable Fees: $8.00 USD\n\
             • Net of two claims' gas: +$7.99 USD\n\
             • Collectible AERO rewards: $7.84 USD (17.864446 AERO in tracked gauges, collected in the Aero app)\n\
             • Total collectible (LP fees + AERO rewards): $15.84 USD\n\
             • Harvest rule: "
        ),
        "{advice}"
    );

    let state = format_portfolio_state_text(&data);
    assert!(
        state.contains(
            "• Unclaimed AERO: 17.864446 AERO ($7.84) in tracked gauges, not counted in the total\n   \
             - Aero USDC/NOCK: 0.210431 AERO ($0.09), collectible from gauge 0xc82782ee66d82c4d19db0429652091a085083526 in the Aero app\n   \
             - Aero CL WETH/NOCK #2914490: 17.654015 AERO ($7.75), collectible from gauge 0x3bf70c880adc8370917994ab0df68e763b7a60fd in the Aero app\n\
             • AERO/USD: $0.438859\n\
             • Collectible earnings (not counted in the total): LP fees $8.00; AERO rewards $7.84; total collectible $15.84\n\
             • High-Water Mark: "
        ),
        "{state}"
    );

    // The claim simulation, with a live estimate for the basic gauge's reward.
    let gas = [RewardGas::Estimated { units: 390_860, usd: Some(0.0062976) }, RewardGas::Estimated { units: 418_528, usd: Some(0.0067434) }];
    let claim = claim_simulation_text(&data, &gas);
    assert_eq!(
        claim,
        "⚡ Fee Claim Simulation\n\
         =======================\n\
         • Collectible LP fees: $8.0000 USD\n\
         • Estimated Gas: ~$0.0041 USD\n\
         • Net Accretion if Harvested: $7.9959 USD\n\
         • Collectible AERO rewards: $7.84 USD (17.864446 AERO in tracked gauges, collected in the Aero app)\n\
         • Total collectible (LP fees + AERO rewards, before gas): $15.84 USD\n\
         Pool Breakdown:\n  \
         - USDC+NOCK (Aerodrome): 8.000000 USDC + 0.00000000 NOCK (~$8.000000 USD | Gas Est: $0.0041)\n      \
         gauge reward: 0.210431 AERO ($0.09), collectible from gauge 0xc82782ee66d82c4d19db0429652091a085083526 in the Aero app | gas est. 390860 units (~$0.0063)\n  \
         - WETH+NOCK (Aero CL #2914490): reward 17.654015 AERO ($7.75), collectible from gauge 0x3bf70c880adc8370917994ab0df68e763b7a60fd in the Aero app | gas est. 418528 units (~$0.0067); LP fees not LP-claimable (staked: swap fees on staked liquidity go to the gauge)\n"
    );
    // From the snapshot alone (the golden files, and the stdio fallback): no gas figure.
    let stored = format_claim_simulation_text(&data);
    assert_eq!(stored, claim_simulation_text(&data, &[]));
    assert!(stored.contains("in the Aero app | gas not estimated (no live estimate was made for this text); LP fees not LP-claimable (staked"), "{stored}");
    assert!(!stored.contains("gas est."), "{stored}");

    // A book with nothing LP-claimable (the real cycle): the reward lines stand, and the
    // LP gas lines still say there is no LP claim to cost.
    let real = full(&snapshot_of(before_claim(), &OperatorProfile::default(), 0.0));
    let text = format_claim_simulation_text(&real);
    assert!(text.contains("• Collectible LP fees: $0.0000 USD\n• Estimated Gas: n/a (nothing LP-claimable)\n• Net Accretion if Harvested: n/a (nothing LP-claimable)\n• Collectible AERO rewards: $7.84 USD (17.864446 AERO in tracked gauges, collected in the Aero app)\n• Total collectible (LP fees + AERO rewards, before gas): $7.84 USD\n"), "{text}");

    // A wallet with no staked Aero position (the from-zero newcomer): "none", and a total.
    let mut raw = RawOnChainData { spot_price_nock_usd: real_nock_usd(), live_eth_price_usd: real_eth_usd(), live_gas_cost_usd: 0.0041, wallet_eth: Some(0.0), wallet_weth: Some(0.0), ..Default::default() };
    raw.aero = AeroReads { wallet_aero: Some(0.0), price_usd: Some(0.438859), price_note: None, unclaimed: Some(vec![]) };
    let zero = full(&snapshot_of(raw, &OperatorProfile::default(), 0.0));
    let advice = format_shot_advice_text(&zero);
    assert!(advice.contains("• Collectible AERO rewards: none (no staked Aero position is tracked)\n• Total collectible (LP fees + AERO rewards): $0.00 USD\n"), "{advice}");
}

#[test]
fn test_both_transports_share_the_reward_text_and_describe_the_tools_alike() {
    let http = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/mcp_http.rs")).unwrap();
    let stdio = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/bin/mcp.rs")).unwrap();
    // The stdio binary compiles the worker's own text file, and its claim text is that file's.
    assert!(stdio.contains("#[path = \"../reward_text.rs\"]\nmod reward_text;"));
    assert!(stdio.contains("claim_simulation_text(data, &[])"));
    assert!(http.contains("claim_simulation_text(&data_val, &gas)"));
    for needle in ["collectible_advice_lines(data)", "collectible_earnings_line(data)"] {
        assert!(stdio.contains(needle), "{needle}");
    }
    // The two tool lists describe the changed tools with the same words, and neither
    // description advises anything about AERO.
    let description = |src: &str, tool: &str| -> String {
        let at = src.find(&format!("\"name\": \"{tool}\"")).unwrap_or_else(|| panic!("{tool} is listed"));
        let rest = &src[at..];
        let start = rest.find("\"description\": \"").unwrap() + "\"description\": \"".len();
        rest[start..start + rest[start..].find("\",\n").unwrap()].to_string()
    };
    for tool in ["simulate_claim_fees", "get_shot_advice"] {
        let d = description(&http, tool);
        assert_eq!(d, description(&stdio, tool), "{tool}");
        assert!(d.contains("AERO rewards"), "{d}");
        for banned in ["should", "worth claiming", "claim now", "recommend"] {
            assert!(!d.to_lowercase().contains(banned), "{d}");
        }
    }
    assert!(description(&http, "simulate_claim_fees").contains("no transaction is built for a reward"));
}

#[test]
fn test_older_snapshot_without_reward_fields_reads_as_it_did_before() {
    let snap = snapshot_of(before_claim(), &OperatorProfile::default(), 0.0);
    let mut data = full(&snap);
    {
        let g = data["derived_metrics"]["global_status"].as_object_mut().unwrap();
        for k in ["collectible_rewards_usd", "collectible_rewards_note", "total_collectible_usd"] {
            assert!(g.remove(k).is_some(), "{k} is emitted");
        }
    }
    for p in data["positions"].as_array_mut().unwrap() {
        assert!(p.as_object_mut().unwrap().remove("collectible_reward").is_some(), "collectible_reward is emitted");
    }
    for path in ["/contract_fees", "/derived_metrics/velocity_gas_sandbox/contracts"] {
        for row in data.pointer_mut(path).unwrap().as_array_mut().unwrap() {
            for k in ["earnings_kind", "reward", "lp_fees_note"] {
                assert!(row.as_object_mut().unwrap().remove(k).is_some(), "{k} is emitted on {path}");
            }
        }
    }
    // A stored 2.6.0 payload still parses, to None.
    let parsed: SnapshotData = serde_json::from_value(data.clone()).expect("an older payload still parses");
    let g = &parsed.derived_metrics.global_status;
    assert_eq!((&g.collectible_rewards_usd, &g.total_collectible_usd, &g.collectible_rewards_note), (&None, &None, &None));
    assert_eq!(parsed.positions[1].collectible_reward, None);
    assert_eq!(parsed.derived_metrics.velocity_gas_sandbox.contracts[1].earnings_kind, None);
    assert_eq!(parsed.contract_fees[1].reward, None);
    // What #93 stored is still there and still read.
    assert_eq!(parsed.positions[1].unclaimed_aero.as_deref(), Some("17.654015"));

    // Both the raw JSON and the worker's re-serialized form (keys present, null), which
    // is what the worker serves from its database until the first cycle after an upgrade.
    // Such a snapshot states its unclaimed AERO ($7.84); a "rewards not reported" line
    // beside that figure would contradict it. The texts gain no reward line: they read
    // as they did before #142.
    for old in [data.clone(), serde_json::to_value(&parsed).unwrap()] {
        let advice = format_shot_advice_text(&old);
        assert!(advice.contains("• Net of two claims' gas: -$0.01 USD\n• Harvest rule: "), "{advice}");
        let state = format_portfolio_state_text(&old);
        assert!(
            state.contains(
                "• Unclaimed AERO: 17.864446 AERO ($7.84) in tracked gauges, not counted in the total\n   \
                 - Aero USDC/NOCK: 0.210431 AERO ($0.09)\n   \
                 - Aero CL WETH/NOCK #2914490: 17.654015 AERO ($7.75)\n\
                 • AERO/USD: $0.438859\n\
                 • High-Water Mark: "
            ),
            "{state}"
        );
        let claim = format_claim_simulation_text(&old);
        assert!(claim.contains("• Net Accretion if Harvested: n/a (nothing LP-claimable)\nPool Breakdown:\n"), "{claim}");
        // The fee row prints the worker's own `amount`, as it did.
        assert!(
            claim.contains("  - WETH+NOCK (Aero CL #2914490): not LP-claimable (staked: swap fees on staked liquidity go to the gauge); unclaimed AERO 17.654015 ($7.75)\n"),
            "{claim}"
        );
        for text in [&advice, &state, &claim] {
            for new_line in ["Collectible AERO rewards", "Total collectible", "Collectible earnings", "rewards not reported", "rewards: not reported", "\n\n"] {
                assert!(!text.contains(new_line), "'{new_line}' in: {text}");
            }
        }
    }
    // The same snapshot from a #142 worker does state them (the lines are not simply gone).
    let advice = format_shot_advice_text(&full(&snap));
    assert!(advice.contains("• Collectible AERO rewards: $7.84 USD (17.864446 AERO in tracked gauges, collected in the Aero app)\n"), "{advice}");
    // The payload rows of such a snapshot: none, rather than rows of zeros.
    assert!(claim_reward_rows(&parsed.positions, &[]).is_empty());
}

#[test]
fn test_the_cycle_stamps_the_reward_where_it_stamps_unclaimed_aero() {
    // One function publishes both, so a position can never carry one without the other.
    let raw = before_claim();
    let published = snapshot_positions(&raw);
    for p in &published {
        let r = p.collectible_reward.as_ref().expect("an Aero position with a gauge has a reward");
        assert_eq!((&r.amount, &r.usd, &r.note), (&p.unclaimed_aero, &p.unclaimed_aero_usd, &p.unclaimed_aero_note));
    }
    // A position that earns no reward (synthetic: the CL config read as Uniswap v3) has none.
    let mut uniswap = cl_config();
    uniswap.protocol = "uniswap-v3".to_string();
    let reads = aero_reads_from(&[uniswap], &HashMap::new(), REAL_AERO_BALANCE_WALLET, price_at_claim(), &|g| panic!("no gauge is read: {g}")).unwrap();
    assert_eq!(collectible_reward_for(&reads, "uniswap-v3:base:2914490"), None);
    // The snapshot spec version moved with the new fields (2.7.0), and again with the
    // closed positions of #144 (2.8.0).
    assert_eq!(SNAPSHOT_SPEC_VERSION, "2.8.0");
}

// ------------------------------------------------------------------
// Golden snapshot for collectible rewards (its own file)
// ------------------------------------------------------------------
// Regenerate only for an intended change:
//   NOCKNOCK_BLESS_GOLDEN=1 cargo test test_golden_snapshot

#[test]
fn test_golden_snapshot_rewards_is_byte_identical() {
    let case = |data: &SnapshotData| {
        let full = serde_json::to_value(data).unwrap();
        json!({
            "state_hash": compute_state_hash(data),
            "snapshot": canonical_state_json(data),
            "mcp_text": {
                "get_portfolio_state": format_portfolio_state_text(&full),
                "get_shot_advice": format_shot_advice_text(&full),
                "simulate_claim_fees": format_claim_simulation_text(&full),
                "get_accumulation_plan": format_accumulation_plan_text(&full),
            },
        })
    };
    // 1. One block before the operator's claim: both rewards collectible.
    let before = snapshot_of(before_claim(), &example_profile(), 0.0);
    // 2. The claim's block: the CL gauge answers zero, the wallet holds the AERO.
    let after = snapshot_of(after_claim(), &example_profile(), 0.0);
    let doc = json!({
        "rewards_collectible_block_58618045": case(&before),
        "rewards_claimed_block_58618046": case(&after),
    });
    let actual = format!("{}\n", serde_json::to_string_pretty(&doc).unwrap());
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/golden_snapshot_rewards.json");
    if std::env::var("NOCKNOCK_BLESS_GOLDEN").is_ok() {
        std::fs::write(path, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(path).expect("testdata/golden_snapshot_rewards.json is checked in");
    assert!(
        actual == expected,
        "snapshot bytes changed; compare {path} with a fresh NOCKNOCK_BLESS_GOLDEN=1 run and `git diff` it"
    );
}
