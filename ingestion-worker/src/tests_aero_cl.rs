//! Aero concentrated-liquidity (Slipstream) positions — issue #91.
//!
//! Every `REAL_*` constant below is a raw `eth_call` result captured from Base mainnet
//! through https://base-rpc.publicnode.com on 2026-11-16, at a pinned block (58061668,
//! 58063800 or 58074373, as each group says), and is
//! recorded with its call in ONCHAIN_INTEGRATION.md §8. Nothing here is hand-made
//! except where a test says "synthetic".

use super::*;

// ---- Block 58061668 (0x375f364, 2026-11-16 04:18:29 UTC): the operator's position ----
const WALLET: &str = "0xeE114cC97eC5adeB34bA5645e315B497D8487a45";
const NPM: &str = "0x4857db576ee0e45f10525ec59527bfd10beec60b";
const POOL: &str = "0x957256c98d61A714081DE2EE70ae3A96c9b40874";
const GAUGE: &str = "0x3bf70c880adc8370917994ab0df68e763b7a60fd";
const NFT_ID: u64 = 2914490;

/// NPM.positions(2914490)
const REAL_POSITIONS_2914490: &str = "0x0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000042000000000000000000000000000000000000060000000000000000000000009b5e262cf9bb04869ab40b19af91d2dc8576172200000000000000000000000000000000000000000000000000000000000000c80000000000000000000000000000000000000000000000000000000000011eb8000000000000000000000000000000000000000000000000000000000001243000000000000000000000000000000000000000000000000322f4dd43fbc87e73000000000000000000000000000000000000574a55555810582e165fb0bd53ef000000000000000000000000000000000212d3d750e1bdf94505a584b88be1f500000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
/// NPM.ownerOf(2914490): the gauge, not the wallet.
const REAL_OWNER_OF_2914490: &str = "0x0000000000000000000000003bf70c880adc8370917994ab0df68e763b7a60fd";
/// pool.slot0(): six words, no feeProtocol.
const REAL_SLOT0: &str = "0x000000000000000000000000000000000000002871f7214287269e35ad48582c000000000000000000000000000000000000000000000000000000000001211200000000000000000000000000000000000000000000000000000000000000cf000000000000000000000000000000000000000000000000000000000000038400000000000000000000000000000000000000000000000000000000000003840000000000000000000000000000000000000000000000000000000000000001";
/// pool.liquidity() (equal to pool.stakedLiquidity() at this block).
const REAL_POOL_LIQUIDITY: &str = "0x00000000000000000000000000000000000000000000000f961342c4493dea67";
/// pool.tickSpacing() = 200, pool.fee() = 9700 pips, pool.gauge(), pool.token0/1().
const REAL_POOL_TICK_SPACING: &str = "0x00000000000000000000000000000000000000000000000000000000000000c8";
const REAL_POOL_FEE: &str = "0x00000000000000000000000000000000000000000000000000000000000025e4";
const REAL_POOL_GAUGE: &str = "0x0000000000000000000000003bf70c880adc8370917994ab0df68e763b7a60fd";
const REAL_POOL_TOKEN0: &str = "0x0000000000000000000000004200000000000000000000000000000000000006";
const REAL_POOL_TOKEN1: &str = "0x0000000000000000000000009b5e262cf9bb04869ab40b19af91d2dc85761722";
/// gauge.stakedContains(wallet, 2914490) = true.
const REAL_STAKED_CONTAINS_WALLET: &str = "0x0000000000000000000000000000000000000000000000000000000000000001";
/// gauge.stakedContains(0x4857db57… (not the staker), 2914490) = false.
const REAL_STAKED_CONTAINS_OTHER: &str = "0x0000000000000000000000000000000000000000000000000000000000000000";
/// ETH/USD pool 0xd0b53d92… slot0() word 0 and v4 StateView getSlot0(0x064a5df2…) word 0
/// at the same block: the prices the snapshot would have valued the position at.
const REAL_ETH_POOL_SQRT_PRICE: &str = "0x00000000000000000000000000000000000000000002cbeb385c1543a56751eb";
const REAL_ETH_POOL_TICK_WORD: &str = "0xfffffffffffffffffffffffffffffffffffffffffffffffffffffffffffcedde";
const REAL_NOCK_USDC_SQRT_PRICE: &str = "0x00000000000000000000000000000000000e5cd8e048808dc15423dd0eac323a";
/// Wallet balances at the same block (raw): NOCK, USDC, WETH, ETH.
const REAL_WALLET_NOCK_RAW: u128 = 0x32d8df9d3e39b43145;
const REAL_WALLET_USDC_RAW: u128 = 0x352169a5;
const REAL_WALLET_WETH_RAW: u128 = 0x329d58d09a8f5;
const REAL_WALLET_ETH_RAW: u128 = 0x2c417829806937;

// ---- Block 58063800 (0x375fbb8): an UNSTAKED position held by an EOA, negative ticks ----
// NFT #3117536 on the same position manager, held by 0xe3c74593…a88a (no code). It was
// burned a few hundred blocks later; the reads are pinned to this block.
const UNSTAKED_OWNER: &str = "0xe3c745934428440875dca2d15047fd32472ba88a";
const REAL_POSITIONS_3117536: &str = "0x00000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000833589fcd6edb6e08f4c7c32d4f71b54bda02913000000000000000000000000b2000000000000000000004884b426556b92883d000000000000000000000000000000000000000000000000000000000000000afffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffe98fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffffef200000000000000000000000000000000000000000000000000000040fc88043cfffffffffffffffffffffffffffffffffff2fedfc3e30063b9a4dc71d5ae6b99fffffffffffffffffffffffffffffffffff5eb5f85ebc8bc2a673c9c41487cd100000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
const REAL_OWNER_OF_3117536: &str = "0x000000000000000000000000e3c745934428440875dca2d15047fd32472ba88a";
/// NPM.collect((3117536, owner, max, max)) simulated FROM the owner: (129192, 599) raw,
/// although positions() above reports tokensOwed0 = tokensOwed1 = 0.
const REAL_COLLECT_3117536_FROM_OWNER: &str = "0x000000000000000000000000000000000000000000000000000000000001f8a80000000000000000000000000000000000000000000000000000000000000257";

// ---- Block 58074373 (0x3762505, 2026-11-16 05:00:50 UTC): whose pool is it ----
/// pool.nft() on the WETH/NOCK CL200 pool: the position manager above.
const REAL_POOL_NFT: &str = "0x0000000000000000000000004857db576ee0e45f10525ec59527bfd10beec60b";
/// The Uniswap v3 WETH/NOCK 1% pool: same tokens, and tickSpacing() = 200 as well.
/// Its nft() call REVERTS (JSON-RPC error code 3, "execution reverted").
const UNISWAP_V3_WETH_NOCK_POOL: &str = "0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a";
const REAL_UNI_V3_POOL_TICK_SPACING: &str = "0x00000000000000000000000000000000000000000000000000000000000000c8";
const REAL_UNI_V3_POOL_NFT_RESPONSE: &str = r#"{"jsonrpc":"2.0","id":1,"error":{"code":3,"message":"execution reverted"}}"#;
/// The LEGACY Slipstream deployment (factory 0x5e7BB104…809A) has its own WETH/USDC
/// CL100 pool, 0xb2cc224c…dc59, beside the current deployment's 0x2a430beb…bc24: same
/// tokens, same tick spacing, another position manager. nft() of each:
const LEGACY_WETH_USDC_CL100_POOL: &str = "0xb2cc224c1c9fee385f8ad6a55b4d94e92359dc59";
const REAL_LEGACY_POOL_NFT: &str = "0x000000000000000000000000827922686190790b37229fd06084350e74485b72";
const REAL_CURRENT_WETH_USDC_CL100_POOL_NFT: &str = "0x0000000000000000000000004857db576ee0e45f10525ec59527bfd10beec60b";

fn cl_config() -> PositionConfig {
    let yaml = format!(
        "id: aero-cl200-weth-nock\nprotocol: aerodrome-cl\nnetwork: base\nnft_id: {NFT_ID}\nnpm_address: \"{NPM}\"\npool_address: \"{POOL}\"\n"
    );
    serde_yaml::from_str(&yaml).expect("position config parses")
}

fn word_u(hex: &str) -> u128 {
    parse_hex_word_u128(&hex.trim_start_matches("0x")[32..]).unwrap()
}

/// The staked position exactly as fetch_aero_cl_position assembles it, from the real reads.
fn real_staked_position() -> (RawPosition, PositionFacts) {
    let cfg = cl_config();
    let decoded = decode_slipstream_positions_return(REAL_POSITIONS_2914490).unwrap();
    let (sqrt_price_x96, tick) = decode_slipstream_slot0(REAL_SLOT0).unwrap();
    let owner = address_from_word(REAL_OWNER_OF_2914490).unwrap();
    let pool_gauge = address_from_word(REAL_POOL_GAUGE).unwrap();
    let holder = classify_nft_holder(&cfg.id, cfg.nft_id, WALLET, &owner, &pool_gauge, None).unwrap();
    let NftHolder::Gauge(gauge) = holder else { panic!("the NFT is held by the gauge") };
    require_staked_by_wallet(&cfg.id, cfg.nft_id, WALLET, &gauge, REAL_STAKED_CONTAINS_WALLET).unwrap();
    aero_cl_position_from_reads(
        &cfg,
        WALLET,
        NPM,
        POOL,
        &decoded,
        &pool_position_manager(&cfg.id, POOL, NPM, Some(REAL_POOL_NFT)).unwrap(),
        &address_from_word(REAL_POOL_TOKEN0).unwrap(),
        &address_from_word(REAL_POOL_TOKEN1).unwrap(),
        word_u(REAL_POOL_TICK_SPACING) as i32,
        word_u(REAL_POOL_FEE) as u32,
        sqrt_price_x96,
        tick,
        Some(gauge),
        AeroClFees::StakedNotLpClaimable,
    )
    .unwrap()
}

fn real_eth_usd() -> f64 {
    sqrt_price_x96_to_raw(U256::from_hex_be(REAL_ETH_POOL_SQRT_PRICE).unwrap()) * 1e18 / 1e6
}

fn real_nock_usd() -> f64 {
    // v4 USDC(6)/NOCK(16) pool: raw = NOCK_raw per USDC_raw.
    1e10 / sqrt_price_x96_to_raw(U256::from_hex_be(REAL_NOCK_USDC_SQRT_PRICE).unwrap())
}

/// One cycle's reads for the real wallet at block 58061668, holding only the staked CL
/// position (the wallet's other positions are left out to keep the case about #91).
fn real_cycle() -> RawOnChainData {
    let (pos, facts) = real_staked_position();
    let (nock_usd, eth_usd) = (real_nock_usd(), real_eth_usd());
    let wallet_nock = REAL_WALLET_NOCK_RAW as f64 / 1e16;
    let wallet_usdc = REAL_WALLET_USDC_RAW as f64 / 1e6;
    let wallet_weth = REAL_WALLET_WETH_RAW as f64 / 1e18;
    let wallet_eth = REAL_WALLET_ETH_RAW as f64 / 1e18;
    let bal = |symbol: &str, address: &str, amount: String, usd: f64| WalletBalance {
        wallet_address: WALLET.to_string(),
        token_symbol: symbol.to_string(),
        token_address: address.to_string(),
        amount,
        amount_usd: format!("{:.2}", usd),
        updated_at: "2026-11-16T04:18:29+00:00".to_string(),
    };
    let mut raw = RawOnChainData {
        wallet_balances: vec![
            bal("NOCK", NOCK_BASE, format!("{:.4}", wallet_nock), wallet_nock * nock_usd),
            bal("USDC", USDC_BASE, format!("{:.4}", wallet_usdc), wallet_usdc),
            bal("ETH", NATIVE_ETH, format!("{:.6}", wallet_eth), wallet_eth * eth_usd),
            bal("WETH", WETH_BASE, format!("{:.6}", wallet_weth), wallet_weth * eth_usd),
        ],
        spot_price_nock_usd: nock_usd,
        live_eth_price_usd: eth_usd,
        live_gas_cost_usd: 0.0041,
        wallet_eth: Some(wallet_eth),
        wallet_weth: Some(wallet_weth),
        ..Default::default()
    };
    raw.pool_liquidity.insert(pos.position_id.clone(), word_u(REAL_POOL_LIQUIDITY).to_string());
    raw.rebuild_rules.insert(pos.position_id.clone(), RebuildRule::default());
    raw.fees_not_lp_claimable.push(pos.position_id.clone());
    raw.position_facts.insert(pos.position_id.clone(), facts);
    raw.positions.push(pos);
    raw
}

/// The explicit numbers of config.yaml.example (a test profile, not a product default).
fn example_profile() -> OperatorProfile {
    OperatorProfile {
        treasury_floor_nock: 100_000.0,
        lp_floor_tolerance_bps: 0,
        tax_reserve_ratio: 0.40,
        target_weights_bps: TargetWeightsBps { treasury: 8000, uniswap_lp: 1500, cash: 500 },
    }
}

/// The tail of fetch_raw_data plus run_ingestion_cycle's pure steps, as tests.rs's
/// golden_snapshot does. The positions come from `snapshot_positions`, the same call
/// run_ingestion_cycle publishes from (test_published_positions_carry_the_aero_cl_facts).
fn snapshot_of(mut raw: RawOnChainData, profile: &OperatorProfile) -> SnapshotData {
    let (spot, eth) = (raw.spot_price_nock_usd, raw.live_eth_price_usd);
    let positions = snapshot_positions(&raw);
    raw.pool_mechanics = build_pool_mechanics(&raw.positions, spot, eth);
    raw.contract_fees = build_contract_fees(&positions, raw.live_gas_cost_usd, &raw.fees_not_lp_claimable);
    let plan = PlanContext {
        cfg: None,
        fills: PlanFills::default(),
        ledger_unavailable: false,
        now: Utc.with_ymd_and_hms(2026, 11, 16, 4, 20, 0).unwrap(),
    };
    let derived_metrics = compute_derived_metrics_with_plan(
        &raw,
        0.0,
        profile,
        StructuralChannels { channels: vec![] },
        &[],
        &LedgerAggregate::default(),
        &MarketRegime::default(),
        &plan,
    );
    SnapshotData {
        wallet_balances: raw.wallet_balances,
        positions,
        pool_mechanics: raw.pool_mechanics,
        concentrated_liquidity: raw.concentrated_liquidity,
        contract_fees: raw.contract_fees,
        derived_metrics,
    }
}

/// sqrtPriceX96 at a tick, for the synthetic out-of-range cases only.
fn sqrt_x96_at_tick(tick: i32) -> U256 {
    let v = tick_sqrt_price(tick) * 79228162514264337593543950336.0;
    let two128 = 340282366920938463463374607431768211456.0f64;
    let hi = (v / two128).floor();
    U256(hi as u128, (v - hi * two128) as u128)
}

// ------------------------------------------------------------------
// Decoding from real responses
// ------------------------------------------------------------------

#[test]
fn test_slipstream_positions_decode_real_nft_2914490() {
    let d = decode_slipstream_positions_return(REAL_POSITIONS_2914490).unwrap();
    assert_eq!(d.token0, "0x4200000000000000000000000000000000000006");
    assert_eq!(d.token1, "0x9b5e262cf9bb04869ab40b19af91d2dc85761722");
    // Word 4 is tickSpacing (200 = CL200), not a fee tier.
    assert_eq!(d.tick_spacing, 200);
    assert_eq!(d.tick_lower, 73400);
    assert_eq!(d.tick_upper, 74800);
    assert_eq!(d.liquidity, 57859113596792372851);
    assert_eq!((d.tokens_owed0, d.tokens_owed1), (0, 0));
    // Ticks sit on the pool's spacing.
    assert_eq!(d.tick_lower % d.tick_spacing, 0);
    assert_eq!(d.tick_upper % d.tick_spacing, 0);
}

#[test]
fn test_slipstream_positions_decode_real_negative_ticks() {
    // NFT #3117536 (USDC / an unregistered token, tick spacing 10): both ticks are
    // negative and arrive sign-extended. The pre-#91 decoder rejected these words.
    let d = decode_slipstream_positions_return(REAL_POSITIONS_3117536).unwrap();
    assert_eq!(d.token0, "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913");
    assert_eq!(d.token1, "0xb2000000000000000000004884b426556b92883d");
    assert_eq!(d.tick_spacing, 10);
    assert_eq!(d.tick_lower, -360);
    assert_eq!(d.tick_upper, -270);
    assert_eq!(d.liquidity, 279114679356);
    assert_eq!((d.tokens_owed0, d.tokens_owed1), (0, 0));
    // The same words through the Uniswap v3 decoder, which shares the tick parsing.
    let v3 = decode_v3_positions_return(REAL_POSITIONS_3117536).unwrap();
    assert_eq!((v3.tick_lower, v3.tick_upper), (-360, -270));
}

#[test]
fn test_signed_tick_words_decode_and_encode() {
    // Real sign-extended words: the two ticks above and the ETH/USD pool's slot0 tick.
    let lower = &REAL_POSITIONS_3117536[2 + 5 * 64..2 + 6 * 64];
    assert_eq!(parse_hex_word_i32(lower).unwrap(), -360);
    assert_eq!(parse_hex_word_i32(REAL_ETH_POOL_TICK_WORD).unwrap(), -201250);
    // Encoding a negative tick for calldata reproduces the chain's own word.
    assert_eq!(int24_calldata_word(-360), lower);
    assert_eq!(int24_calldata_word(-201250), REAL_ETH_POOL_TICK_WORD.trim_start_matches("0x"));
    // Positive ticks are unchanged from the previous encoding and decoding.
    assert_eq!(int24_calldata_word(73400), format!("{:064x}", 73400u32));
    assert_eq!(parse_hex_word_i32(&format!("{:064x}", 74002u32)).unwrap(), 74002);
    assert_eq!(parse_hex_word_i32("0x12112").unwrap(), 74002);
    // A word that is not a small signed integer is an error, not a truncated tick.
    assert!(parse_hex_word_i32(REAL_ETH_POOL_SQRT_PRICE).is_err());
    assert!(parse_hex_word_i32(&format!("{:064x}", 0x8000_0000u64)).is_err());
    assert!(parse_hex_word_i32("zz").is_err());
}

#[test]
fn test_slipstream_slot0_decode_real_six_words() {
    // 6 words (Uniswap v3 returns 7): sqrtPriceX96, tick, observationIndex,
    // observationCardinality, observationCardinalityNext, unlocked.
    assert_eq!(REAL_SLOT0.trim_start_matches("0x").len(), 6 * 64);
    let (sqrt, tick) = decode_slipstream_slot0(REAL_SLOT0).unwrap();
    assert_eq!(sqrt, U256::from_hex_be("0x2871f7214287269e35ad48582c").unwrap());
    assert_eq!(tick, 74002);
    // The tick agrees with the sqrt price it came with.
    let implied = (sqrt_price_x96_to_raw(sqrt).ln() / 1.0001f64.ln()).floor() as i32;
    assert_eq!(implied, 74002);
    assert!(decode_slipstream_slot0("0x").is_err());
    assert!(decode_slipstream_slot0(&format!("0x{}", "0".repeat(128))).is_err(), "zero price = uninitialized pool");
}

#[test]
fn test_collect_return_decode_real_unstaked_position() {
    // What an unstaked LP can claim is what collect() simulated from the owner returns;
    // positions() showed tokensOwed 0/0 for the same NFT at the same block.
    assert_eq!(decode_collect_return(REAL_COLLECT_3117536_FROM_OWNER).unwrap(), (129192, 599));
    assert!(decode_collect_return("0x").is_err());
    assert!(decode_collect_return(&format!("0x{}{}", "f".repeat(64), "0".repeat(64))).is_err());
}

// ------------------------------------------------------------------
// Amounts, valuation, range
// ------------------------------------------------------------------

#[test]
fn test_aero_cl_amounts_from_real_reads() {
    let (p, facts) = real_staked_position();
    assert_eq!(p.position_id, "aerodrome-cl:base:2914490");
    assert_eq!(p.protocol, "aerodrome-cl");
    assert_eq!(p.nft_id, NFT_ID);
    assert_eq!(p.pool_id, POOL.to_lowercase());
    assert_eq!(p.npm_address.as_deref(), Some(NPM));
    assert_eq!((p.token0.symbol.as_str(), p.token1.symbol.as_str()), ("WETH", "NOCK"));
    // fee() 9700 pips = 0.97% = 97 bps; never the tick spacing.
    assert_eq!(p.fee_tier_bps, 97);
    assert_eq!((p.tick_lower, p.tick, p.tick_upper), (73400, 74002, 74800));
    assert!(p.in_range);
    // Liquidity 57859113596792372851 between ticks 73400..74800 at this sqrt price:
    // 0.055908 WETH + 6,945.83 NOCK (the deposit, block 57425824, was 0.081995 ETH +
    // 2,755 NOCK; NOCK has since fallen against ETH, so the range holds more NOCK).
    assert!((p.amount0 - 0.0559082).abs() < 1e-6, "WETH {}", p.amount0);
    assert!((p.amount1 - 6945.8297).abs() < 1e-3, "NOCK {}", p.amount1);
    // Staked: no fee figure, and the facts say why.
    assert_eq!((p.fee0, p.fee1), (0.0, 0.0));
    assert_eq!(facts.tick_spacing, Some(200));
    assert_eq!(facts.staked_in_gauge.as_deref(), Some(GAUGE));
    assert_eq!(facts.fees_note.as_deref(), Some(AERO_CL_STAKED_FEES_NOTE));
    // The position is about a fifth of the pool's in-range liquidity.
    let share = p.liquidity as f64 / word_u(REAL_POOL_LIQUIDITY) as f64;
    assert!((share - 0.2012).abs() < 1e-3, "share {share}");
    assert_eq!(position_label(&p), "Aero CL WETH/NOCK 0.97% #2914490");
}

#[test]
fn test_aero_cl_is_valued_with_the_snapshot_prices() {
    let raw = real_cycle();
    let (nock_usd, eth_usd) = (raw.spot_price_nock_usd, raw.live_eth_price_usd);
    assert!((eth_usd - 1820.907).abs() < 0.01, "ETH/USD {eth_usd}");
    assert!((nock_usd - 0.0112868).abs() < 1e-6, "NOCK/USD {nock_usd}");
    let snap = snapshot_of(raw, &OperatorProfile::default());
    let p = &snap.positions[0];
    assert_eq!(p.protocol, "aerodrome-cl");
    assert_eq!(p.amount0, "0.055908");
    assert_eq!(p.amount1, "6945.829656");
    // 0.055908 WETH * $1,820.91 + 6,945.83 NOCK * $0.0112868 = $180.20
    assert_eq!(p.amount_usd, "180.20");
    assert_eq!(p.fee_tier_bps, 97);
    assert_eq!(p.tick_spacing, Some(200));
    assert_eq!(p.staked_in_gauge.as_deref(), Some(GAUGE));
    assert_eq!(p.fees_note.as_deref(), Some(AERO_CL_STAKED_FEES_NOTE));
    assert_eq!(p.pool_active, Some(true));
    assert!(!p.full_range);
    assert!(p.in_range);
    // ETH per NOCK, lower bound from the upper tick.
    assert_eq!((p.lower_price.as_str(), p.upper_price.as_str()), ("0.00000564", "0.00000649"));

    let m = &snap.pool_mechanics[0];
    assert_eq!(m.platform, "Aerodrome");
    assert_eq!(m.tick_current, Some(74002));
    assert_eq!(m.fee_tier_bps, 97);
    assert_eq!(m.liquidity_usd, "180.20");
}

#[test]
fn test_aero_cl_nock_counts_toward_the_total_and_the_floor_and_weth_toward_eth() {
    let raw = real_cycle();
    let wallet_nock = REAL_WALLET_NOCK_RAW as f64 / 1e16; // 93,796.46
    let wallet_gas = (REAL_WALLET_ETH_RAW + REAL_WALLET_WETH_RAW) as f64 / 1e18; // 0.013347
    let snap = snapshot_of(raw, &example_profile());
    let g = &snap.derived_metrics.global_status;
    // NOCK total = wallet + the position's 6,945.83; floor 100,000.
    let total_nock: f64 = g.total_nock_amount.parse().unwrap();
    assert!((total_nock - (wallet_nock + 6945.83)).abs() < 1.0, "total NOCK {total_nock}");
    assert!(wallet_nock < 100_000.0 && total_nock > 100_000.0, "the position's NOCK is what clears the floor");
    let delta: f64 = g.nock_floor_delta.parse().unwrap();
    assert!((delta - (total_nock - 100_000.0)).abs() < 1.0, "floor delta {delta}");
    assert!(!g.shot_advice_action.contains("DEFENSIVE LOCK"), "{}", g.shot_advice_action);
    // ETH pillar = wallet gas + the position's WETH; wallet gas itself is unchanged.
    let total_eth: f64 = g.total_eth_amount.as_deref().unwrap().parse().unwrap();
    assert!((total_eth - (wallet_gas + 0.055908)).abs() < 1e-5, "ETH pillar {total_eth}");
    let gas: f64 = g.wallet_gas_eth.as_deref().unwrap().parse().unwrap();
    assert!((gas - wallet_gas).abs() < 1e-5, "wallet gas {gas}");
    // The total counts the position once: wallet NOCK + USDC + gas + $180.20 of LP.
    let total: f64 = g.total_portfolio_usd.parse().unwrap();
    let expect = wallet_nock * real_nock_usd() + 891.382181 + wallet_gas * real_eth_usd() + 180.1995;
    assert!((total - expect).abs() < 0.02, "total {total} vs {expect}");
    // Per-position NOCK series for the chart carries it too.
    let series = &snap.derived_metrics.asset_aggregation_series.positions_nock[0];
    assert_eq!(series.position_id, "aerodrome-cl:base:2914490");
    assert_eq!(series.series, vec!["6945.83".to_string()]);
}

#[test]
fn test_aero_cl_range_shows_in_the_tick_bracket_panel() {
    let snap = snapshot_of(real_cycle(), &OperatorProfile::default());
    let b = &snap.derived_metrics.tick_bracket_hypervisor.brackets[0];
    assert!(b.in_range);
    assert!(!b.full_range);
    assert_eq!((b.lower_price.as_str(), b.upper_price.as_str()), ("0.00000564", "0.00000649"));
    assert_eq!(b.liquidity_usd, "180.20");
    // ETH-denominated bounds have no honest bps distance to a USD spot.
    assert_eq!(b.distance_bps, None);
}

#[test]
fn test_aero_cl_out_of_range_is_single_sided_on_the_right_side() {
    // Synthetic prices around the REAL liquidity and ticks: the maths is shared with
    // Uniswap v3 (evm::concentrated_amounts_raw); only the pool price is made up here.
    let cfg = cl_config();
    let decoded = decode_slipstream_positions_return(REAL_POSITIONS_2914490).unwrap();
    let build = |tick: i32, sqrt: U256| {
        aero_cl_position_from_reads(
            &cfg, WALLET, NPM, POOL, &decoded, NPM, WETH_BASE, NOCK_BASE, 200, 9700, sqrt, tick,
            Some(GAUGE.to_string()), AeroClFees::StakedNotLpClaimable,
        )
        .unwrap()
        .0
    };
    // Pool tick below the range (NOCK dearer in ETH than the range's top price): the
    // range has sold all its NOCK and holds only WETH.
    let below = build(73000, sqrt_x96_at_tick(73000));
    assert!(!below.in_range);
    assert!(below.amount0 > 0.0 && below.amount1 == 0.0, "{} / {}", below.amount0, below.amount1);
    // Pool tick above the range (NOCK cheaper than the range's bottom price): all NOCK.
    let above = build(75200, sqrt_x96_at_tick(75200));
    assert!(!above.in_range);
    assert!(above.amount0 == 0.0 && above.amount1 > 0.0, "{} / {}", above.amount0, above.amount1);
    // The upper tick itself is already out of range (exclusive bound).
    assert!(tick_in_range(74799, 73400, 74800));
    assert!(!tick_in_range(74800, 73400, 74800));
    assert!(tick_in_range(73400, 73400, 74800));
    assert!(!tick_in_range(73399, 73400, 74800));
    // Out of range reaches the bracket panel and the rebuild exclusion like any ranged position.
    let mut raw = real_cycle();
    raw.positions[0] = above;
    let snap = snapshot_of(raw, &example_profile());
    assert!(!snap.positions[0].in_range);
    assert!(!snap.derived_metrics.tick_bracket_hypervisor.brackets[0].in_range);
}

// ------------------------------------------------------------------
// Ownership
// ------------------------------------------------------------------

#[test]
fn test_staked_ownership_is_proved_by_the_gauge_from_real_reads() {
    let cfg = cl_config();
    let owner = address_from_word(REAL_OWNER_OF_2914490).unwrap();
    let pool_gauge = address_from_word(REAL_POOL_GAUGE).unwrap();
    // ownerOf is the gauge, not the wallet.
    assert_eq!(owner, GAUGE);
    assert_ne!(owner, WALLET.to_lowercase());
    assert_eq!(
        classify_nft_holder(&cfg.id, NFT_ID, WALLET, &owner, &pool_gauge, None).unwrap(),
        NftHolder::Gauge(GAUGE.to_string())
    );
    // The gauge lists the NFT among the wallet's stakes: counted.
    require_staked_by_wallet(&cfg.id, NFT_ID, WALLET, GAUGE, REAL_STAKED_CONTAINS_WALLET).unwrap();
    // The same gauge asked about an account that did not stake it answers false:
    // staked, but by someone else. Not counted, and the message says so.
    let err = require_staked_by_wallet(&cfg.id, NFT_ID, NPM, GAUGE, REAL_STAKED_CONTAINS_OTHER)
        .unwrap_err()
        .to_string();
    assert!(err.contains("NFT #2914490 is staked in gauge"), "{err}");
    assert!(err.contains("not by the tracked wallet"), "{err}");
    assert!(err.contains("not counted"), "{err}");
    // A short or non-boolean answer is an error, never "true".
    assert!(require_staked_by_wallet(&cfg.id, NFT_ID, WALLET, GAUGE, "0x").is_err());
    assert!(require_staked_by_wallet(&cfg.id, NFT_ID, WALLET, GAUGE, REAL_POOL_FEE).is_err());
    // A configured gauge_address that matches the pool's gauge is accepted (any case).
    assert!(classify_nft_holder(&cfg.id, NFT_ID, WALLET, &owner, &pool_gauge, Some(&GAUGE.to_uppercase().replace("0X", "0x"))).is_ok());
}

#[test]
fn test_nft_held_by_neither_wallet_nor_gauge_fails_the_cycle() {
    let cfg = cl_config();
    let pool_gauge = address_from_word(REAL_POOL_GAUGE).unwrap();
    // Held by an unrelated account (the real EOA from the unstaked fixture).
    let err = classify_nft_holder(&cfg.id, NFT_ID, WALLET, UNSTAKED_OWNER, &pool_gauge, None)
        .unwrap_err()
        .to_string();
    assert!(err.contains("NFT #2914490 is held by 0xe3c745934428440875dca2d15047fd32472ba88a"), "{err}");
    assert!(err.contains("neither the tracked wallet"), "{err}");
    assert!(err.contains("not counted"), "{err}");
    // A pool with no gauge: the holder cannot be a staking contract at all.
    let err = classify_nft_holder(&cfg.id, NFT_ID, WALLET, GAUGE, NATIVE_ETH, None).unwrap_err().to_string();
    assert!(err.contains("has no gauge"), "{err}");
    // gauge_address in config that is not the pool's gauge is a configuration error.
    let err = classify_nft_holder(&cfg.id, NFT_ID, WALLET, GAUGE, &pool_gauge, Some(NPM)).unwrap_err().to_string();
    assert!(err.starts_with("Configuration error"), "{err}");
}

#[test]
fn test_unstaked_nft_held_by_the_wallet_is_counted_with_its_collectable_fees() {
    // Real reads for NFT #3117536: ownerOf is the EOA itself, so with that EOA as the
    // tracked wallet the position is unstaked and its fees are the collect() simulation.
    let owner = address_from_word(REAL_OWNER_OF_3117536).unwrap();
    assert_eq!(owner, UNSTAKED_OWNER);
    assert_eq!(
        classify_nft_holder("x", 3117536, UNSTAKED_OWNER, &owner, GAUGE, None).unwrap(),
        NftHolder::Wallet
    );
    let (f0, f1) = decode_collect_return(REAL_COLLECT_3117536_FROM_OWNER).unwrap();
    // The fee path on the operator's own pair (synthetic: this NFT's token1 is not in
    // the registry, so the real raw fee amounts are applied to the WETH/NOCK fixture).
    let cfg = cl_config();
    let decoded = decode_slipstream_positions_return(REAL_POSITIONS_2914490).unwrap();
    let (sqrt, tick) = decode_slipstream_slot0(REAL_SLOT0).unwrap();
    let build = |fees: AeroClFees| {
        aero_cl_position_from_reads(&cfg, WALLET, NPM, POOL, &decoded, NPM, WETH_BASE, NOCK_BASE, 200, 9700, sqrt, tick, None, fees)
            .unwrap()
    };
    let (p, facts) = build(AeroClFees::Claimable(f0, f1));
    assert!((p.fee0 - 129192.0 / 1e18).abs() < 1e-24);
    assert!((p.fee1 - 599.0 / 1e16).abs() < 1e-24);
    assert_eq!(facts.staked_in_gauge, None);
    assert_eq!(facts.fees_note, None);
    assert_eq!(fee_contract_for(&p.protocol, p.npm_address.as_deref(), &p.pool_id, None), NPM);

    // Read fees are plain figures on every surface: no note, counted as claimable.
    let mut raw = real_cycle();
    raw.fees_not_lp_claimable.clear();
    raw.position_facts.insert(p.position_id.clone(), facts);
    raw.positions[0] = p;
    let snap = snapshot_of(raw, &OperatorProfile::default());
    assert_eq!(snap.positions[0].fees_note, None);
    assert_eq!(snap.positions[0].staked_in_gauge, None);
    assert!(snap.contract_fees[0].amount.ends_with("WETH + 0.00000000 NOCK"), "{}", snap.contract_fees[0].amount);
}

#[test]
fn test_unstaked_collect_revert_fails_the_cycle_instead_of_publishing_zero_fees() {
    // An unstaked position's collect() simulation is a required read, like the Uniswap
    // fee reads. A revert has no "unavailable" value to carry: AeroClFees has only the
    // two cases below, so an unread fee cannot reach the snapshot, the claim payload or
    // the fee totals as 0.
    for fees in [AeroClFees::Claimable(1, 2), AeroClFees::StakedNotLpClaimable] {
        match fees {
            AeroClFees::Claimable(..) => assert_eq!(fees.note(), None),
            AeroClFees::StakedNotLpClaimable => assert_eq!(fees.note(), Some(AERO_CL_STAKED_FEES_NOTE)),
        }
    }
    let err = collect_reverted_error("aero-cl200-weth-nock", NFT_ID, NPM, WALLET).to_string();
    assert!(err.starts_with("position aero-cl200-weth-nock: collect() for NFT #2914490"), "{err}");
    assert!(err.contains("reverted"), "{err}");
    assert!(err.contains("cannot be read"), "{err}");
    assert!(err.contains("the cycle fails rather than report them as 0"), "{err}");
    // The revert that triggers it is the node's own answer (JSON-RPC error code 3), as
    // captured for the Uniswap v3 pool's nft() call; an outage is a different error.
    let revert: serde_json::Value = serde_json::from_str(REAL_UNI_V3_POOL_NFT_RESPONSE).unwrap();
    assert_eq!(classify_eth_call_response(&revert).unwrap(), None);
    assert!(classify_eth_call_response(&json!({ "error": { "code": -32005, "message": "rate limited" } })).is_err());
    // The adapter has no other outcome for that revert: the source says so.
    let adapter = include_str!("adapters/aero_cl.rs");
    assert!(adapter.contains("Ok(None) => return Err(collect_reverted_error("), "collect revert must fail the cycle");
    assert!(!adapter.contains("fees unavailable"), "no degraded fee value may come back");
}

#[test]
fn test_pool_that_is_not_the_nfts_pool_is_a_configuration_error() {
    let cfg = cl_config();
    let decoded = decode_slipstream_positions_return(REAL_POSITIONS_2914490).unwrap();
    let (sqrt, tick) = decode_slipstream_slot0(REAL_SLOT0).unwrap();
    let build = |pool_nft: &str, t0: &str, t1: &str, spacing: i32| {
        aero_cl_position_from_reads(
            &cfg, WALLET, NPM, POOL, &decoded, pool_nft, t0, t1, spacing, 9700, sqrt, tick, None, AeroClFees::Claimable(0, 0),
        )
    };
    let real_nft = pool_position_manager(&cfg.id, POOL, NPM, Some(REAL_POOL_NFT)).unwrap();
    assert_eq!(real_nft, NPM);
    assert!(build(&real_nft, WETH_BASE, NOCK_BASE, 200).is_ok());
    for err in [
        build(NPM, WETH_BASE, NOCK_BASE, 100).unwrap_err().to_string(), // another tick spacing
        build(NPM, USDC_BASE, NOCK_BASE, 200).unwrap_err().to_string(), // another pair
        build(NPM, NOCK_BASE, WETH_BASE, 200).unwrap_err().to_string(), // tokens swapped
    ] {
        assert!(err.starts_with("Configuration error: position aero-cl200-weth-nock: pool_address"), "{err}");
    }
    // A token outside the registry is an explicit error, not a guessed symbol.
    let other = decode_slipstream_positions_return(REAL_POSITIONS_3117536).unwrap();
    let err = aero_cl_position_from_reads(
        &cfg, WALLET, NPM, POOL, &other, NPM, &other.token0, &other.token1, 10, 100, sqrt, tick, None, AeroClFees::Claimable(0, 0),
    )
    .unwrap_err()
    .to_string();
    assert!(err.contains("Unregistered token"), "{err}");
}

#[test]
fn test_pool_of_another_position_manager_is_a_configuration_error_from_real_reads() {
    let cfg = cl_config();
    let decoded = decode_slipstream_positions_return(REAL_POSITIONS_2914490).unwrap();
    let (sqrt, tick) = decode_slipstream_slot0(REAL_SLOT0).unwrap();

    // (a) The Uniswap v3 WETH/NOCK 1% pool, the entry just above this one in the example
    // config: same token0/token1 and, really, the same tick spacing (200). Tokens and
    // spacing alone would accept it; its nft() call reverts, and that is the answer.
    assert_eq!(word_u(REAL_UNI_V3_POOL_TICK_SPACING), 200);
    let reverted: serde_json::Value = serde_json::from_str(REAL_UNI_V3_POOL_NFT_RESPONSE).unwrap();
    let nft_call = classify_eth_call_response(&reverted).unwrap();
    assert_eq!(nft_call, None, "a revert, not an outage");
    let err = pool_position_manager(&cfg.id, UNISWAP_V3_WETH_NOCK_POOL, NPM, nft_call.as_deref())
        .unwrap_err()
        .to_string();
    assert!(
        err.starts_with(
            "Configuration error: position aero-cl200-weth-nock: pool_address 0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a is not an Aero CL pool of position manager 0x4857db576ee0e45f10525ec59527bfd10beec60b"
        ),
        "{err}"
    );
    assert!(err.contains("nft() call reverts"), "{err}");
    // An address with no such function at all (empty return data) is the same error.
    let err = pool_position_manager(&cfg.id, WALLET, NPM, Some("")).unwrap_err().to_string();
    assert!(err.starts_with("Configuration error") && err.contains("returned no address"), "{err}");

    // (b) The legacy Slipstream deployment's pool for a pair: same tokens and spacing
    // as the current deployment's pool, but its nft() is the legacy position manager.
    let legacy_nft = pool_position_manager(&cfg.id, LEGACY_WETH_USDC_CL100_POOL, NPM, Some(REAL_LEGACY_POOL_NFT)).unwrap();
    assert_eq!(legacy_nft, "0x827922686190790b37229fd06084350e74485b72");
    assert_eq!(address_from_word(REAL_CURRENT_WETH_USDC_CL100_POOL_NFT).unwrap(), NPM);
    // Applied to the operator's NFT with every other read matching: still refused,
    // before any amount is computed at that pool's price.
    let err = aero_cl_position_from_reads(
        &cfg, WALLET, NPM, LEGACY_WETH_USDC_CL100_POOL, &decoded, &legacy_nft, WETH_BASE, NOCK_BASE, 200, 9700, sqrt, tick,
        None, AeroClFees::Claimable(0, 0),
    )
    .unwrap_err()
    .to_string();
    assert!(
        err.starts_with(
            "Configuration error: position aero-cl200-weth-nock: pool_address 0xb2cc224c1c9fee385f8ad6a55b4d94e92359dc59 is not a pool of position manager 0x4857db576ee0e45f10525ec59527bfd10beec60b (its nft() is 0x827922686190790b37229fd06084350e74485b72)"
        ),
        "{err}"
    );
    // The address comparison ignores case and the 0x prefix.
    assert!(check_pool_is_the_nfts_pool(&cfg, &NPM.to_uppercase().replace("0X", "0x"), POOL, &decoded, NPM, WETH_BASE, NOCK_BASE, 200).is_ok());
}

// ------------------------------------------------------------------
// What is published: the snapshot's own positions carry the Aero CL facts
// ------------------------------------------------------------------

#[test]
fn test_published_positions_carry_the_aero_cl_facts() {
    // run_ingestion_cycle publishes `snapshot_positions(&raw_data)`. For the real
    // staked position that row must carry the staking, the fees note and the tick
    // spacing: without them the card prints "Fees: $0.000000" and the claim payload
    // simulates a collect() that reverts (the pre-release bug this test pins).
    let raw = real_cycle();
    let published = snapshot_positions(&raw);
    assert_eq!(published.len(), 1);
    let p = &published[0];
    assert_eq!(p.staked_in_gauge.as_deref(), Some(GAUGE));
    assert_eq!(p.fees_note.as_deref(), Some(AERO_CL_STAKED_FEES_NOTE));
    assert_eq!(p.tick_spacing, Some(200));
    assert_eq!(p.pool_active, Some(true));
    let json = serde_json::to_value(p).unwrap();
    assert_eq!(json["staked_in_gauge"], GAUGE);
    assert_eq!(json["fees_note"], AERO_CL_STAKED_FEES_NOTE);
    assert_eq!(json["tick_spacing"], 200);
    // So the claim payload, which reads the published snapshot, settles it without a
    // simulation and keeps it out of the totals of the actions that can execute.
    let mut action = ClaimFeeAction {
        platform: AERO_CL_PROTOCOL.to_string(),
        label: String::new(),
        to: NPM.to_string(),
        data: build_v3_collect_calldata(NFT_ID, WALLET),
        value: "0x0".to_string(),
        expected_amount_0: None,
        expected_amount_1: None,
        expected_usd: None,
        gas_estimate: None,
        gas_usd: None,
        simulation: "unavailable".to_string(),
        simulation_detail: None,
    };
    assert!(aero_cl_mark_not_claimable(&mut action, Some(p)));
    assert_eq!(action.simulation, "not_claimable");
    let mut v4 = action.clone();
    v4.platform = "uniswap-v4".to_string();
    v4.simulation = "success".to_string();
    v4.gas_usd = Some("0.0100".to_string());
    assert_eq!(
        claim_payload_totals(&[v4, action], "1.00", true),
        (Some("0.0100".to_string()), Some("0.9900".to_string()))
    );
    // Without the facts the same reads give the row the bug published: this is what
    // the two bare steps produce, and why nothing may publish from them directly.
    let mut bare = build_position_snapshots(&raw.positions, raw.spot_price_nock_usd, raw.live_eth_price_usd);
    apply_pool_activity(&mut bare, &raw.pool_liquidity);
    assert_eq!(bare[0].staked_in_gauge, None);
    assert_eq!(bare[0].fees_note, None);
    // Both production call sites go through the one function (source check: the cycle
    // itself needs a database and a node, so it cannot run here).
    let ingest = include_str!("ingest.rs");
    assert!(ingest.contains("let positions = snapshot_positions(&raw_data);"), "run_ingestion_cycle must publish snapshot_positions");
    assert!(!ingest.contains("build_position_snapshots("), "run_ingestion_cycle must not rebuild positions without the facts");
    let onchain = include_str!("onchain.rs");
    assert!(onchain.contains("snapshot_positions_from(&raw_positions,"), "fetch_raw_data must build fee rows from the same positions");
    assert!(!onchain.contains("build_position_snapshots("));
    // The snapshot the golden file pins is built from the same call.
    let snap = snapshot_of(real_cycle(), &example_profile());
    assert_eq!(serde_json::to_value(&snap.positions).unwrap(), serde_json::to_value(&published).unwrap());
}

// ------------------------------------------------------------------
// Fees: staked is "not LP-claimable", never $0 earned
// ------------------------------------------------------------------

#[test]
fn test_staked_position_fees_are_not_lp_claimable_on_every_surface() {
    assert!(AERO_CL_STAKED_FEES_NOTE.starts_with("not LP-claimable"));
    // Since #93 the note is the reason alone: the AERO earned instead is read from the
    // gauge and stated after it (tests_aero_rewards.rs). This fixture carries no AERO read.
    assert!(!AERO_CL_STAKED_FEES_NOTE.contains("not read"));
    let snap = snapshot_of(real_cycle(), &OperatorProfile::default());
    // contract_fees: the reason in place of an amount; the gauge as the contract.
    let f = &snap.contract_fees[0];
    assert_eq!(f.fee_token_symbol, "WETH+NOCK (Aero CL #2914490)");
    assert_eq!(f.amount, AERO_CL_STAKED_FEES_NOTE);
    assert!(!f.claimable);
    assert_eq!(f.contract_address, GAUGE);
    // Velocity panel: not_claimable with the position's own reason, not "accumulate".
    let v = &snap.derived_metrics.velocity_gas_sandbox.contracts[0];
    assert_eq!(v.contract_name, "Aero CL WETH/NOCK 0.97% #2914490");
    assert_eq!(v.recommended_action, "not_claimable");
    assert_eq!(v.note.as_deref(), Some(AERO_CL_STAKED_FEES_NOTE));
    assert_eq!(v.contract_address, GAUGE);
    // Nothing is added to the claimable total, and no harvest is advised for it.
    assert_eq!(snap.derived_metrics.global_status.total_fees_claimable_usd.parse::<f64>().unwrap(), 0.0);
    assert!(!snap.derived_metrics.global_status.shot_advice_action.contains("HARVEST"));
    // MCP text: the reason, no "$0.00" line and no gas estimate for it.
    let text = format_claim_simulation_text(&serde_json::to_value(&snap).unwrap());
    assert!(text.contains(&format!("  - WETH+NOCK (Aero CL #2914490): {AERO_CL_STAKED_FEES_NOTE}\n")), "{text}");
    assert!(!text.contains("Aero CL #2914490): 0."), "{text}");
    assert!(!text.to_lowercase().contains("uniswap"), "{text}");
    // Gas was read ($0.0041 on the row); with nothing claimable there is no claim to
    // cost, which is said as such and not as a failed gas estimate.
    assert_eq!(f.gas_break_even_usd, "0.0041");
    assert!(text.contains("• Estimated Gas: n/a (nothing LP-claimable)\n"), "{text}");
    assert!(text.contains("• Net Accretion if Harvested: n/a (nothing LP-claimable)\n"), "{text}");
    assert!(!text.contains("live gas estimate unavailable"), "{text}");
    // No fee rows at all keeps the old wording (golden_snapshot.json pins it).
    let none = format_claim_simulation_text(&json!({ "contract_fees": [] }));
    assert!(none.contains("• Estimated Gas: unavailable\n"), "{none}");
    // A claimable row beside it: gas and totals come from that row alone.
    let mut both = serde_json::to_value(&snap).unwrap();
    both["contract_fees"].as_array_mut().unwrap().push(json!({
        "fee_token_symbol": "USDC+NOCK (Uniswap V4 #1)", "amount": "0.1 USDC + 2 NOCK",
        "amount_usd": "0.12", "claimable": true, "gas_break_even_usd": "0.0014"
    }));
    let text = format_claim_simulation_text(&both);
    assert!(text.contains("• Estimated Gas: ~$0.0014 USD\n"), "{text}");
    assert!(text.contains("• Total Gross Claimable: $0.1200 USD\n"), "{text}");
    // Even when a caller forgets to list the id, the note alone keeps it not claimable.
    let fees = build_contract_fees(&snap.positions, 0.0041, &[]);
    assert_eq!(fees[0].amount, AERO_CL_STAKED_FEES_NOTE);
    assert!(!fees[0].claimable);
}

#[test]
fn test_claim_payload_marks_a_staked_position_not_claimable() {
    let snap = snapshot_of(real_cycle(), &OperatorProfile::default());
    let action = || ClaimFeeAction {
        platform: AERO_CL_PROTOCOL.to_string(),
        label: "Collect Aero CL LP fees (aero-cl200-weth-nock #2914490)".to_string(),
        to: NPM.to_string(),
        data: build_v3_collect_calldata(NFT_ID, WALLET),
        value: "0x0".to_string(),
        expected_amount_0: None,
        expected_amount_1: None,
        expected_usd: Some("0.0000".to_string()),
        gas_estimate: None,
        gas_usd: None,
        simulation: "unavailable".to_string(),
        simulation_detail: None,
    };
    // Staked: settled without a simulation, with the reason, and no expected USD.
    let mut a = action();
    assert!(aero_cl_mark_not_claimable(&mut a, Some(&snap.positions[0])));
    assert_eq!(a.simulation, "not_claimable");
    assert_eq!(a.simulation_detail.as_deref(), Some(AERO_CL_STAKED_FEES_NOTE));
    assert_eq!(a.expected_usd, None);
    // It stays out of the payload totals like the Aero basic pool's reverting claim.
    assert_eq!(claim_payload_totals(&[a], "0.00", true), (None, None));
    // Unstaked: left for the normal collect simulation.
    let mut unstaked = snap.positions[0].clone();
    unstaked.staked_in_gauge = None;
    unstaked.fees_note = None;
    let mut a = action();
    assert!(!aero_cl_mark_not_claimable(&mut a, Some(&unstaked)));
    assert_eq!(a.simulation, "unavailable");
    // A row that carries a fees note but no gauge (no adapter emits one today; a stored
    // snapshot or a later note could): its placeholder zeros are never copied in as
    // expected amounts. Settled "unavailable" with the note, nothing expected.
    let mut noted = unstaked.clone();
    noted.fees_note = Some("fees not read this cycle".to_string());
    assert_eq!((noted.tokens_owed_0.as_str(), noted.tokens_owed_1.as_str()), ("0.000000", "0.000000"));
    let mut a = action();
    a.expected_amount_0 = Some("0.000000".to_string());
    a.expected_amount_1 = Some("0.000000".to_string());
    assert!(aero_cl_mark_not_claimable(&mut a, Some(&noted)));
    assert_eq!(a.simulation, "unavailable");
    assert_eq!(a.simulation_detail.as_deref(), Some("fees not read this cycle"));
    assert_eq!((a.expected_amount_0, a.expected_amount_1, a.expected_usd), (None, None, None));
    // A blank note is no note.
    noted.fees_note = Some("  ".to_string());
    assert!(!aero_cl_mark_not_claimable(&mut action(), Some(&noted)));
    // Not in the snapshot: unknown, never simulated blind, never "success".
    let mut a = action();
    assert!(aero_cl_mark_not_claimable(&mut a, None));
    assert_eq!(a.simulation, "unavailable");
    assert!(a.simulation_detail.unwrap().contains("not in the latest snapshot"));
    // The collect calldata is the selector verified on the Aero CL position manager.
    assert!(build_v3_collect_calldata(NFT_ID, WALLET).starts_with("0xfc6f7865"));
}

// ------------------------------------------------------------------
// LP rebuild, links, labels
// ------------------------------------------------------------------

#[test]
fn test_aero_cl_is_never_offered_a_rebuild_deposit_and_links_to_the_aero_app() {
    let raw = real_cycle();
    let snap = snapshot_of(raw, &example_profile());
    let lp = &snap.derived_metrics.lp_rebuild;
    // LP is under its 15% target, yet the WETH leg keeps this position out of sizing.
    assert_eq!(lp.status, "under_target");
    assert!(lp.suggestions.is_empty(), "{:?}", lp.suggestions);
    assert_eq!(lp.excluded.len(), 1);
    assert_eq!(lp.excluded[0].label, "Aero CL WETH/NOCK 0.97% #2914490");
    assert_eq!(lp.excluded[0].reason, LP_GAS_LEG_REASON);
    // The deposit link, were one ever shown, is the Aero app, never Uniswap or the
    // retired aerodrome.finance.
    let (p, _) = real_staked_position();
    let url = deposit_url_for(&p, &RebuildRule::default());
    assert_eq!(url, "https://app.aero.xyz/deposit/8453-0x957256c98d61a714081de2ee70ae3a96c9b40874");
    let overridden = RebuildRule { deposit_url: Some("https://app.aero.xyz/dash".to_string()), ..Default::default() };
    assert_eq!(deposit_url_for(&p, &overridden), "https://app.aero.xyz/dash");
    // Nothing in the snapshot or the MCP text calls it Uniswap or links to aerodrome.finance.
    let full = serde_json::to_value(&snap).unwrap();
    for text in [
        full.to_string(),
        format_portfolio_state_text(&full),
        format_shot_advice_text(&full),
        format_claim_simulation_text(&full),
        format_accumulation_plan_text(&full),
    ] {
        assert!(!text.to_lowercase().contains("uniswap v"), "{text}");
        assert!(!text.contains("app.uniswap.org"), "{text}");
        assert!(!text.contains("aerodrome.finance"), "{text}");
    }
    // NockNock states facts about AERO; it never advises buying or selling it.
    let advice = format!(
        "{} {}",
        snap.derived_metrics.global_status.shot_advice_action, snap.derived_metrics.global_status.shot_advice_details
    );
    assert!(!advice.contains("AERO"), "{advice}");
}

#[test]
fn test_aero_cl_without_an_eth_leg_is_still_not_sized_for_a_rebuild() {
    // Synthetic: a USDC/NOCK Aero CL position (no such pool is tracked; the real
    // liquidity and ticks are reused, only the pair is made up). It has no gas leg, so
    // only the explicit Aero CL rule keeps it out: the app's deposit link would mint a
    // new NFT that NockNock does not track.
    let tok = |symbol: &str, address: &str, decimals: i32| PositionTokenInfo {
        address: address.to_string(),
        symbol: symbol.to_string(),
        decimals,
    };
    // A floor the wallet clears with room to spare, so that funding is not what stops it.
    let profile = OperatorProfile { treasury_floor_nock: 50_000.0, ..example_profile() };
    for staked in [true, false] {
        let mut raw = real_cycle();
        let id = raw.positions[0].position_id.clone();
        raw.positions[0].token0 = tok("USDC", USDC_BASE, 6);
        raw.positions[0].amount0 = 20.0;
        if !staked {
            raw.position_facts.get_mut(&id).unwrap().staked_in_gauge = None;
            raw.position_facts.get_mut(&id).unwrap().fees_note = None;
            raw.fees_not_lp_claimable.clear();
        }
        let snap = snapshot_of(raw, &profile);
        let lp = &snap.derived_metrics.lp_rebuild;
        assert_eq!(lp.status, "under_target", "the LP target is unmet, so a sizable position would be offered a deposit");
        assert!(lp.suggestions.is_empty(), "staked={staked}: {:?}", lp.suggestions);
        assert_eq!(lp.excluded.len(), 1);
        assert_eq!(lp.excluded[0].label, "Aero CL USDC/NOCK 0.97% #2914490");
        assert_eq!(lp.excluded[0].reason, LP_AERO_CL_REASON);
        // No surface suggests adding to the NFT or prints a deposit link for it.
        let full = serde_json::to_value(&snap).unwrap();
        for text in [
            full["derived_metrics"]["global_status"]["shot_advice_details"].as_str().unwrap().to_string(),
            format_accumulation_plan_text(&full),
            format_shot_advice_text(&full),
        ] {
            assert!(!text.contains("to Aero CL"), "{text}");
            assert!(!text.contains("app.aero.xyz/deposit"), "{text}");
        }
    }
    assert!(LP_AERO_CL_REASON.starts_with("Aero CL: not sized"));
    assert!(!LP_AERO_CL_REASON.to_lowercase().contains("uniswap"));
    // The same wallet with a Uniswap v3 USDC/NOCK position of that shape IS sized: the
    // rule is about the protocol, and the three existing protocols are untouched by it.
    let mut raw = real_cycle();
    let old_id = raw.positions[0].position_id.clone();
    raw.positions[0].token0 = tok("USDC", USDC_BASE, 6);
    raw.positions[0].amount0 = 20.0;
    raw.positions[0].protocol = "uniswap-v3".to_string();
    raw.positions[0].position_id = "uniswap-v3:base:2914490".to_string();
    let liq = raw.pool_liquidity.remove(&old_id).unwrap();
    raw.pool_liquidity.insert("uniswap-v3:base:2914490".to_string(), liq);
    raw.position_facts.clear();
    raw.fees_not_lp_claimable.clear();
    let snap = snapshot_of(raw, &profile);
    let lp = &snap.derived_metrics.lp_rebuild;
    assert_eq!(lp.suggestions.len(), 1, "{:?}", lp.excluded);
    assert_eq!(lp.suggestions[0].label, "v3 USDC/NOCK 0.97% #2914490");
}

#[test]
fn test_pool_liveness_read_uses_the_pools_own_liquidity() {
    let (p, _) = real_staked_position();
    assert_eq!(pool_liquidity_call(&p).unwrap(), (POOL.to_lowercase(), "0x1a686502".to_string()));
    assert_eq!(word_u(REAL_POOL_LIQUIDITY), 287515221646699784807);
}

// ------------------------------------------------------------------
// Unknown protocol: a configuration error, never Uniswap v4
// ------------------------------------------------------------------

#[test]
fn test_unknown_protocol_is_a_configuration_error() {
    let yaml = r#"
tracked_wallets: ["0xeE114cC97eC5adeB34bA5645e315B497D8487a45"]
positions:
  - id: "aero-cl200-weth-nock"
    protocol: "aerodrome-cl"
    nft_id: 2914490
    npm_address: "0x4857db576ee0e45f10525ec59527bfd10beec60b"
    pool_address: "0x957256c98d61A714081DE2EE70ae3A96c9b40874"
  - id: "mystery"
    protocol: "aerodrome-slipstream"
    nft_id: 7
"#;
    let cfg: AppConfig = serde_yaml::from_str(yaml).expect("yaml parses");
    assert_eq!(position_key(&cfg.positions[0]), "aerodrome-cl:base:2914490");
    assert!(validate_positions_config(&cfg.positions[..1]).is_ok());
    let err = validate_positions_config(&cfg.positions).unwrap_err().to_string();
    assert!(err.starts_with("Configuration error: position 'mystery' has unknown protocol 'aerodrome-slipstream'"), "{err}");
    assert!(err.contains("uniswap-v4, uniswap-v3, aerodrome, aerodrome-cl"), "{err}");
    for typo in ["", "uniswap", "uniswap-v2", "Uniswap-V4", "aerodrome-CL", "aero", "velodrome"] {
        assert_eq!(Protocol::parse(typo), None, "{typo}");
        let mut c = cfg.positions[1].clone();
        c.protocol = typo.to_string();
        assert!(validate_positions_config(&[c]).is_err(), "{typo}");
    }
    assert_eq!(Protocol::parse("uniswap-v4"), Some(Protocol::UniswapV4));
    assert_eq!(Protocol::parse("uniswap-v3"), Some(Protocol::UniswapV3));
    assert_eq!(Protocol::parse("aerodrome"), Some(Protocol::AeroBasic));
    assert_eq!(Protocol::parse("aerodrome-cl"), Some(Protocol::AeroCl));

    // Were such a position ever to reach the pure helpers, none of them answers with
    // Uniswap v4's contract, link, label or read.
    let (mut p, _) = real_staked_position();
    p.protocol = "aerodrome-slipstream".to_string();
    let label = position_label(&p);
    assert!(label.contains("unsupported protocol") && !label.starts_with("v4") && !label.starts_with("v3"), "{label}");
    assert_eq!(deposit_url_for(&p, &RebuildRule::default()), "");
    let contract = fee_contract_for(&p.protocol, None, &p.pool_id, None);
    assert_ne!(contract, UNISWAP_V4_POSM_BASE);
    assert_eq!(pool_platform(&p.protocol), "Other");
    assert_eq!(protocol_display_name(&p.protocol), "aerodrome-slipstream");
    assert!(!velocity_label(&p).contains("Uniswap"));
    let err = pool_liquidity_call(&p).unwrap_err().to_string();
    assert!(err.contains("unknown protocol 'aerodrome-slipstream'"), "{err}");
}

#[test]
fn test_the_three_existing_protocols_keep_their_names_links_and_reads() {
    let tok = |symbol: &str, address: &str, decimals: i32| PositionTokenInfo {
        address: address.to_string(),
        symbol: symbol.to_string(),
        decimals,
    };
    let (base, _) = real_staked_position();
    let v4 = RawPosition {
        position_id: "uniswap-v4:base:3163450".to_string(),
        protocol: "uniswap-v4".to_string(),
        nft_id: 3163450,
        pool_id: "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13".to_string(),
        npm_address: None,
        fee_tier_bps: 300,
        token0: tok("USDC", USDC_BASE, 6),
        token1: tok("NOCK", NOCK_BASE, 16),
        ..base.clone()
    };
    let v3 = RawPosition {
        position_id: "uniswap-v3:base:6336662".to_string(),
        protocol: "uniswap-v3".to_string(),
        nft_id: 6336662,
        pool_id: "0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a".to_string(),
        npm_address: Some("0x03a520b32C04BF3bEEf7BEb72E919cf822Ed34f1".to_string()),
        fee_tier_bps: 100,
        ..base.clone()
    };
    let aero = RawPosition {
        position_id: "aerodrome:base:0x770fe8fc9248c6382950a86e39a9391b41d59e0a".to_string(),
        protocol: "aerodrome".to_string(),
        nft_id: 0,
        pool_id: "0x770fe8fc9248c6382950a86e39a9391b41d59e0a".to_string(),
        npm_address: None,
        fee_tier_bps: 30,
        token0: tok("USDC", USDC_BASE, 6),
        token1: tok("NOCK", NOCK_BASE, 16),
        tick_lower: MIN_TICK,
        tick_upper: MAX_TICK,
        ..base.clone()
    };
    let rule = RebuildRule::default();
    assert_eq!(position_label(&v4), "v4 USDC/NOCK 3.00% #3163450");
    assert_eq!(position_label(&v3), "v3 WETH/NOCK 1.00% #6336662");
    assert_eq!(position_label(&aero), "Aero USDC/NOCK 0.30% (full range)");
    assert_eq!(deposit_url_for(&v4, &rule), "https://app.uniswap.org/positions/v4/base/3163450");
    assert_eq!(deposit_url_for(&v3, &rule), "https://app.uniswap.org/positions/v3/base/6336662");
    assert_eq!(deposit_url_for(&aero, &rule), "https://app.aero.xyz/deposit/8453-0x770fe8fc9248c6382950a86e39a9391b41d59e0a");
    assert_eq!(velocity_label(&v4), "Uniswap V4 (USDC/NOCK 3.00%, #3163450)");
    assert_eq!(velocity_label(&v3), "Uniswap V3 (WETH/NOCK 1.00%, #6336662)");
    assert_eq!(velocity_label(&aero), "Aero USDC/NOCK 0.30% (full range)");
    assert_eq!(fee_contract_for("uniswap-v4", None, &v4.pool_id, None), UNISWAP_V4_POSM_BASE);
    assert_eq!(fee_contract_for("uniswap-v3", v3.npm_address.as_deref(), &v3.pool_id, None), "0x03a520b32C04BF3bEEf7BEb72E919cf822Ed34f1");
    assert_eq!(fee_contract_for("aerodrome", None, &aero.pool_id, None), aero.pool_id);
    assert_eq!(
        (pool_platform("uniswap-v4"), pool_platform("uniswap-v3"), pool_platform("aerodrome")),
        ("UniswapV4", "UniswapV3", "Aerodrome")
    );
    assert_eq!(pool_liquidity_call(&v3).unwrap(), (v3.pool_id.clone(), "0x1a686502".to_string()));
    assert_eq!(pool_liquidity_call(&aero).unwrap(), (aero.pool_id.clone(), "0x18160ddd".to_string()));
    assert_eq!(
        pool_liquidity_call(&v4).unwrap(),
        (
            UNISWAP_V4_STATE_VIEW_BASE.to_string(),
            "0xfa6793d5064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13".to_string()
        )
    );
    // Their snapshot rows carry none of the new optional fields, so stored payloads
    // and the golden file keep their bytes.
    let snaps = build_position_snapshots(&[v4, v3, aero], 0.0113, 1820.0);
    for s in &snaps {
        let json = serde_json::to_value(s).unwrap();
        for key in ["tick_spacing", "staked_in_gauge", "fees_note"] {
            assert!(json.get(key).is_none(), "{} must not serialize {key}", s.position_id);
        }
    }
    let fees = build_contract_fees(&snaps, 0.004, &[snaps[2].position_id.clone()]);
    assert_eq!(fees[0].fee_token_symbol, "USDC+NOCK (Uniswap V4 #3163450)");
    assert_eq!(fees[1].fee_token_symbol, "WETH+NOCK (Uniswap V3 #6336662)");
    assert_eq!(fees[2].fee_token_symbol, "USDC+NOCK (Aerodrome)");
    assert_eq!(fees[2].amount, AERO_NOT_LP_CLAIMABLE);
}

#[test]
fn test_example_config_positions_are_all_known_protocols() {
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/config.yaml.example");
    let cfg = load_app_config(example, Some("/nonexistent/config.yaml"), false).expect("example config loads");
    assert!(!cfg.positions.is_empty());
    validate_positions_config(&cfg.positions).expect("every example position has a supported protocol");
    // The Aero CL entry is a commented example: the operator adds their own.
    assert!(cfg.positions.iter().all(|p| p.protocol != AERO_CL_PROTOCOL));
    let text = std::fs::read_to_string(example).unwrap();
    assert!(text.contains("#   protocol: \"aerodrome-cl\""), "the example config documents the new protocol");
}

// ------------------------------------------------------------------
// Golden snapshot for the new protocol (its own file; golden_snapshot.json is untouched)
// ------------------------------------------------------------------
// Regenerate only for an intended change:
//   NOCKNOCK_BLESS_GOLDEN=1 cargo test test_golden_snapshot

#[test]
fn test_golden_snapshot_aero_cl_is_byte_identical() {
    let data = snapshot_of(real_cycle(), &example_profile());
    let full = serde_json::to_value(&data).unwrap();
    let doc = json!({
        "aero_cl_staked_block_58061668": {
            "state_hash": compute_state_hash(&data),
            "snapshot": canonical_state_json(&data),
            "mcp_text": {
                "get_portfolio_state": format_portfolio_state_text(&full),
                "get_shot_advice": format_shot_advice_text(&full),
                "simulate_claim_fees": format_claim_simulation_text(&full),
                "get_accumulation_plan": format_accumulation_plan_text(&full),
            },
        },
    });
    let actual = format!("{}\n", serde_json::to_string_pretty(&doc).unwrap());
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/golden_snapshot_aero_cl.json");
    if std::env::var("NOCKNOCK_BLESS_GOLDEN").is_ok() {
        std::fs::write(path, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(path).expect("testdata/golden_snapshot_aero_cl.json is checked in");
    assert!(
        actual == expected,
        "snapshot bytes changed; compare {path} with a fresh NOCKNOCK_BLESS_GOLDEN=1 run and `git diff` it"
    );
}

/// Both spec files must parse as YAML. A plain scalar containing ": " (as a description
/// added with this feature once did) is rejected by a strict parser, so the spec could
/// not be loaded by tooling even though every substring check passed.
#[test]
fn test_spec_files_parse_as_yaml() {
    for (name, text) in [
        ("specs/core/snapshot.yaml", include_str!("../../specs/core/snapshot.yaml")),
        ("specs/core/api.yaml", include_str!("../../specs/core/api.yaml")),
    ] {
        let parsed: Result<serde_yaml::Value, _> = serde_yaml::from_str(text);
        assert!(parsed.is_ok(), "{name} is not valid YAML: {:?}", parsed.err());
    }
}

/// A failed RPC request must not carry its URL into the error text: the URL holds the
/// Alchemy key, and this text is published (/health `last_error`, snapshot notes, MCP).
/// reqwest appends "for url (...)" to a send error unless it is stripped. Port 1 on
/// loopback refuses the connection, so no network is needed.
#[tokio::test]
async fn test_rpc_network_error_text_never_contains_the_url_or_key() {
    let url = "http://127.0.0.1:1/v2/SECRETKEY-do-not-print";
    let payload = json!({ "jsonrpc": "2.0", "id": 1, "method": "eth_call", "params": [] });
    let err = eth_call_allow_revert(&Client::new(), url, &payload).await.expect_err("nothing listens on port 1");
    let text = format!("{err} | {err:?} | {err:#}");
    assert!(text.contains("network error"), "{text}");
    for banned in ["SECRETKEY", "/v2/", "127.0.0.1:1", "for url"] {
        assert!(!text.contains(banned), "RPC error text leaks {banned:?}: {text}");
    }
}
