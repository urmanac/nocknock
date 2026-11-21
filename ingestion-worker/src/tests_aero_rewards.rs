//! AERO, the fourth pillar — issue #93.
//!
//! Every `REAL_*` constant below is a raw JSON-RPC result captured from Base mainnet
//! through https://base-rpc.publicnode.com on 2026-11-16, pinned to block 58077673
//! (`0x37631e9`, 2026-11-16T05:11:50Z) unless its comment names another block, and is
//! recorded with its call in ONCHAIN_INTEGRATION.md §9. Nothing here is hand-made
//! except where a test says "synthetic". The fixture builders are `pub(super)` so that
//! tests_rewards.rs (#142) builds on the same real cycle.

use super::*;

pub(super) const WALLET: &str = "0xeE114cC97eC5adeB34bA5645e315B497D8487a45";

// ---- The token ----
/// The older token that also carries the symbol "AERO" (name "Aerodrome"). Not what
/// the gauges pay, and not read.
const LEGACY_AERODROME_AERO: &str = "0x940181a94A35A4569E4529A3CDfB74e38FD98631";
/// AERO.decimals() = 18, AERO.symbol() = "AERO", AERO.name() = "Aero".
const REAL_AERO_DECIMALS: &str = "0x0000000000000000000000000000000000000000000000000000000000000012";
const REAL_AERO_SYMBOL: &str = "0x000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000044145524f00000000000000000000000000000000000000000000000000000000";
const REAL_AERO_NAME: &str = "0x000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000044165726f00000000000000000000000000000000000000000000000000000000";
/// AERO.balanceOf(wallet) = 0: the operator has not claimed yet.
pub(super) const REAL_AERO_BALANCE_WALLET: &str = "0x0000000000000000000000000000000000000000000000000000000000000000";

// ---- The basic (volatile pool) gauge ----
pub(super) const BASIC_POOL: &str = "0x770fE8Fc9248c6382950A86E39a9391B41D59e0A";
pub(super) const BASIC_GAUGE: &str = "0xc82782ee66d82c4d19db0429652091a085083526";
/// gauge.earned(wallet): 0.121026111298851597 AERO.
const REAL_BASIC_EARNED_WALLET: &str = "0x00000000000000000000000000000000000000000000000001adf89b6483d30d";
/// gauge.earned(0x…dead), an account with no stake: zero.
const REAL_BASIC_EARNED_NO_STAKE: &str = "0x0000000000000000000000000000000000000000000000000000000000000000";
/// gauge.rewardToken(): reverts on both gauges (the whole JSON-RPC response).
const REAL_REWARD_TOKEN_RESPONSE: &str = r#"{"jsonrpc":"2.0","id":1,"error":{"code":3,"message":"execution reverted"}}"#;
/// Pool reads for the position itself.
pub(super) const REAL_BASIC_TOKEN0: &str = "0x000000000000000000000000833589fcd6edb6e08f4c7c32d4f71b54bda02913";
pub(super) const REAL_BASIC_TOKEN1: &str = "0x0000000000000000000000009b5e262cf9bb04869ab40b19af91d2dc85761722";
pub(super) const REAL_BASIC_RESERVE0: u128 = 0x365530f6;
pub(super) const REAL_BASIC_RESERVE1: u128 = 0x2c1ef9786aa216adb1;
pub(super) const REAL_BASIC_TOTAL_SUPPLY: u128 = 0x30f5cf6d2eb46;
/// pool.balanceOf(wallet) = 0; gauge.balanceOf(wallet) = 29684955447174 (all staked).
pub(super) const REAL_BASIC_LP_STAKED: u128 = 0x1aff91397f86;

// ---- Legacy Aerodrome gauges: the same reads, the older token (block 58093329) ----
// Read at block 58093329 (`0x3766f11`, 2026-11-16T06:04:01Z).
/// The gauge of the legacy vAMM USDC/NOCK pool 0x85f1aa3a…d626:
/// Voter 0x16613524…80A5 `gauges(pool)` `0xb9a09fd5` returns it, and its
/// `stakingToken()` is that pool.
pub(super) const LEGACY_BASIC_GAUGE: &str = "0x4cbb6188c6101cacd4eb1126cc14882bd93dc3db";
/// legacyGauge.rewardToken(): the OLDER token 0x940181a9…8631 (name "Aerodrome").
const REAL_LEGACY_REWARD_TOKEN: &str = "0x000000000000000000000000940181a94a35a4569e4529a3cdfb74e38fd98631";
/// legacyGauge.earned(wallet): it answers the same `earned(address)` (0 for the wallet).
const REAL_LEGACY_EARNED_WALLET: &str = "0x0000000000000000000000000000000000000000000000000000000000000000";
/// The gauge of the legacy Slipstream CL100 WETH/USDC pool 0xb2cc224c…dc59
/// (`pool.gauge()` `0xa6f19c84`) names the older token too. Its
/// `earned(address,uint256[])` reverts, so it could not be misread anyway.
pub(super) const LEGACY_CL_GAUGE: &str = "0xf33a96b5932d9e9b9a0eda447abd8c9d48d2e0c8";
const REAL_LEGACY_CL_REWARD_TOKEN: &str = "0x000000000000000000000000940181a94a35a4569e4529a3cdfb74e38fd98631";

// ---- The CL gauge and NFT #2914490 ----
const NPM: &str = "0x4857db576ee0e45f10525ec59527bfd10beec60b";
const CL_POOL: &str = "0x957256c98d61A714081DE2EE70ae3A96c9b40874";
pub(super) const CL_GAUGE: &str = "0x3bf70c880adc8370917994ab0df68e763b7a60fd";
pub(super) const NFT_ID: u64 = 2914490;
/// gauge.earned(wallet, [2914490]): 11.922077777221318098 AERO.
const REAL_CL_EARNED_WALLET: &str = "0x000000000000000000000000000000000000000000000000a573ba928fdaf5d2";
/// gauge.earned(wallet, []), no token ids: zero.
const REAL_CL_EARNED_NO_IDS: &str = "0x0000000000000000000000000000000000000000000000000000000000000000";
/// gauge.earned(wallet, [2914491]), an id the wallet has not staked: reverts "NA".
const REAL_CL_EARNED_NOT_STAKED_RESPONSE: &str = r#"{"jsonrpc":"2.0","id":1,"error":{"code":3,"message":"execution reverted: NA","data":"0x08c379a0000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000024e41000000000000000000000000000000000000000000000000000000000000"}}"#;
/// gauge.earned(address,uint256), the older Slipstream gauge's selector: reverts here.
const REAL_CL_EARNED_OLD_SELECTOR_RESPONSE: &str = r#"{"jsonrpc":"2.0","id":1,"error":{"code":3,"message":"execution reverted"}}"#;
const REAL_POSITIONS_2914490: &str = "0x0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000042000000000000000000000000000000000000060000000000000000000000009b5e262cf9bb04869ab40b19af91d2dc8576172200000000000000000000000000000000000000000000000000000000000000c80000000000000000000000000000000000000000000000000000000000011eb8000000000000000000000000000000000000000000000000000000000001243000000000000000000000000000000000000000000000000322f4dd43fbc87e73000000000000000000000000000000000000574a55555810582e165fb0bd53ef000000000000000000000000000000000212d3d750e1bdf94505a584b88be1f500000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
const REAL_OWNER_OF_2914490: &str = "0x0000000000000000000000003bf70c880adc8370917994ab0df68e763b7a60fd";
const REAL_CL_SLOT0: &str = "0x00000000000000000000000000000000000000286ce4e4dbb539f36c942684b6000000000000000000000000000000000000000000000000000000000001210800000000000000000000000000000000000000000000000000000000000000d0000000000000000000000000000000000000000000000000000000000000038400000000000000000000000000000000000000000000000000000000000003840000000000000000000000000000000000000000000000000000000000000001";
const REAL_CL_LIQUIDITY: &str = "0x00000000000000000000000000000000000000000000000eadef6f2e97f2a3c6";
const REAL_CL_GAUGE_WORD: &str = "0x0000000000000000000000003bf70c880adc8370917994ab0df68e763b7a60fd";
const REAL_CL_NFT: &str = "0x0000000000000000000000004857db576ee0e45f10525ec59527bfd10beec60b";
const REAL_CL_TOKEN0: &str = "0x0000000000000000000000004200000000000000000000000000000000000006";
const REAL_CL_TOKEN1: &str = "0x0000000000000000000000009b5e262cf9bb04869ab40b19af91d2dc85761722";
const REAL_STAKED_CONTAINS_WALLET: &str = "0x0000000000000000000000000000000000000000000000000000000000000001";

// ---- The AERO/USD price pool: Aero CL200 AERO/USDC 0xd0acf293…6816 ----
pub(super) const REAL_PRICE_POOL_TOKEN0: &str = "0x0000000000000000000000003f5bf3edac59d650d8fdf9657a71f2f904a3a4ee";
pub(super) const REAL_PRICE_POOL_TOKEN1: &str = "0x000000000000000000000000833589fcd6edb6e08f4c7c32d4f71b54bda02913";
/// slot0(): six words (a Slipstream pool); word 0 is sqrtPriceX96, the tick is -285057.
const REAL_PRICE_POOL_SLOT0: &str = "0x000000000000000000000000000000000000000000000ad784fbb267f75a7626fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffba67f00000000000000000000000000000000000000000000000000000000000001d1000000000000000000000000000000000000000000000000000000000000070900000000000000000000000000000000000000000000000000000000000007090000000000000000000000000000000000000000000000000000000000000001";
const REAL_PRICE_POOL_LIQUIDITY: &str = "0x000000000000000000000000000000000000000000000026ee51c707e7470a2d";
/// The same pool at block 57635701 (`0x36f7375`, 2026-11-15T04:38:35Z), the block of the
/// real basic-gauge claim below.
const REAL_PRICE_POOL_SLOT0_AT_CLAIM: &str = "0x000000000000000000000000000000000000000000000b1d397eb6115a61877afffffffffffffffffffffffffffffffffffffffffffffffffffffffffffba86f00000000000000000000000000000000000000000000000000000000000003db000000000000000000000000000000000000000000000000000000000000070900000000000000000000000000000000000000000000000000000000000007090000000000000000000000000000000000000000000000000000000000000001";
const REAL_PRICE_POOL_LIQUIDITY_AT_CLAIM: &str = "0x00000000000000000000000000000000000000000000000e165b3720f45d71f4";
/// A dead pool of the same pair: Aero CL1 AERO/USDC 0xb27460d0…f863. Same tokens, an
/// initialized price ($0.7044, 69% above the live one) and zero in-range liquidity.
const DEAD_PRICE_POOL: &str = "0xb27460d0e2b4ec0fd2b2c29f0c2b29d4c045f863";
const REAL_DEAD_POOL_SLOT0: &str = "0x000000000000000000000000000000000000000000000e14cfb37aedad132234fffffffffffffffffffffffffffffffffffffffffffffffffffffffffffbbaec0000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000000000010000000000000000000000000000000000000000000000000000000000000001";
const REAL_DEAD_POOL_LIQUIDITY: &str = "0x0000000000000000000000000000000000000000000000000000000000000000";
/// Cross-check: Aero CL200 AERO/WETH 0xc85d5cb7…c9b3 slot0 word 0 (WETH per AERO).
const REAL_AERO_WETH_SQRT_PRICE: &str = "0x000000000000000000000000000000000000000003e0fd7ad1a3064d1658cef5";

// ---- Prices and wallet balances the snapshot would have used at the same block ----
/// ETH/USD pool 0xd0b53d92… slot0() word 0 and its tokens (WETH, USDC).
const REAL_ETH_POOL_SQRT_PRICE: &str = "0x00000000000000000000000000000000000000000002ca08b36955a1a8abadcf";
const REAL_ETH_POOL_TOKEN0: &str = "0x0000000000000000000000004200000000000000000000000000000000000006";
const REAL_ETH_POOL_LIQUIDITY: &str = "0x00000000000000000000000000000000000000000000000007233f497b44dc72";
/// v4 StateView getSlot0(0x064a5df2…) word 0: the NOCK/USD price source.
const REAL_NOCK_USDC_SQRT_PRICE: &str = "0x00000000000000000000000000000000000e5cd8e048808dc15423dd0eac323a";
const REAL_WALLET_NOCK_RAW: u128 = 0x32d8df9d3e39b43145;
const REAL_WALLET_USDC_RAW: u128 = 0x352169a5;
const REAL_WALLET_WETH_RAW: u128 = 0x329d58d09a8f5;
const REAL_WALLET_ETH_RAW: u128 = 0x2c417829806937;

// ---- A real claim: tx 0x189f6ddf…b8f8, block 57635701, account 0x7c12cd5b…386d ----
// The account called claimEmissions on the basic gauge above. One block earlier
// gauge.earned(account) returned 0x12df9e0749351725, exactly what was minted to it.
pub(super) const CLAIMER: &str = "0x7c12cd5b7db841f7ba9b3fd1b7a6d02c1304386d";
const REAL_EARNED_BEFORE_CLAIM: &str = "0x00000000000000000000000000000000000000000000000012df9e0749351725";

/// The receipt's token logs (the other nine logs are voter and messaging events).
pub(super) fn real_claim_receipt() -> serde_json::Value {
    let transfer = TRANSFER_EVENT_TOPIC;
    let zero = "0x0000000000000000000000000000000000000000000000000000000000000000";
    json!({
        "transactionHash": "0x189f6ddfbb0e278ed37b2049d8eb5deeeb78f618a1b20c213c47f42511f9b8f8",
        "blockNumber": "0x36f7375",
        "from": CLAIMER,
        "to": BASIC_GAUGE,
        "status": "0x1",
        "logs": [
            // xAERO 0x6cdda0e7… minted to and burned from an intermediary: not the wallet's.
            { "address": "0x6cdda0e730744dc6ff67654b35350adad05ecee7", "logIndex": "0x51",
              "topics": [transfer, zero, "0x0000000000000000000000008c37f30f3c8fd48beddb14086fa5148291d422c3"],
              "data": "0x00000000000000000000000000000000000000000000000012df9e0749351725" },
            { "address": "0x6cdda0e730744dc6ff67654b35350adad05ecee7", "logIndex": "0x52",
              "topics": [transfer, "0x0000000000000000000000008c37f30f3c8fd48beddb14086fa5148291d422c3", zero],
              "data": "0x00000000000000000000000000000000000000000000000012df9e0749351725" },
            // AERO minted to the claimer.
            { "address": "0x3f5bf3edac59d650d8fdf9657a71f2f904a3a4ee", "logIndex": "0x53",
              "topics": [transfer, zero, "0x0000000000000000000000007c12cd5b7db841f7ba9b3fd1b7a6d02c1304386d"],
              "data": "0x00000000000000000000000000000000000000000000000012df9e0749351725" },
            // A further 5% minted to a third account: not the wallet's.
            { "address": "0x3f5bf3edac59d650d8fdf9657a71f2f904a3a4ee", "logIndex": "0x54",
              "topics": [transfer, zero, "0x0000000000000000000000009beb95cced809c0667bbb6577ee4066ac5cc1156"],
              "data": "0x00000000000000000000000000000000000000000000000000f194b39075dac1" },
            // The gauge's EmissionsClaimed(address,address,uint256).
            { "address": BASIC_GAUGE, "logIndex": "0x5b",
              "topics": ["0x41a8f9b73fc5b26eed668484bc6e1ca460c33a1be2aaf9cc8d6ae0f6dd67a968",
                         "0x0000000000000000000000007c12cd5b7db841f7ba9b3fd1b7a6d02c1304386d",
                         "0x0000000000000000000000007c12cd5b7db841f7ba9b3fd1b7a6d02c1304386d"],
              "data": "0x00000000000000000000000000000000000000000000000012df9e0749351725" }
        ]
    })
}

// ------------------------------------------------------------------
// Fixture builders: one real cycle at block 58077673
// ------------------------------------------------------------------

pub(super) fn basic_config(gauge: Option<&str>) -> PositionConfig {
    let gauge_line = gauge.map(|g| format!("gauge_address: \"{g}\"\n")).unwrap_or_default();
    serde_yaml::from_str(&format!(
        "id: aero-basic-nock-usdc\nprotocol: aerodrome\nnetwork: base\npool_address: \"{BASIC_POOL}\"\n{gauge_line}"
    ))
    .expect("position config parses")
}

pub(super) fn cl_config() -> PositionConfig {
    serde_yaml::from_str(&format!(
        "id: aero-cl200-weth-nock\nprotocol: aerodrome-cl\nnetwork: base\nnft_id: {NFT_ID}\nnpm_address: \"{NPM}\"\npool_address: \"{CL_POOL}\"\n"
    ))
    .expect("position config parses")
}

fn word_u(hex: &str) -> u128 {
    parse_hex_word_u128(&hex.trim_start_matches("0x")[32..]).unwrap()
}

/// The basic position as fetch_aerodrome_position assembles it: the wallet's LP is all
/// staked in the gauge, and claimFees() reverted at this block (no LP-claimable fees).
fn real_basic_position() -> RawPosition {
    aerodrome_position_from_reads(
        &basic_config(Some(BASIC_GAUGE)),
        WALLET,
        BASIC_POOL,
        token_info(&REAL_BASIC_TOKEN0[26..]).unwrap(),
        token_info(&REAL_BASIC_TOKEN1[26..]).unwrap(),
        REAL_BASIC_RESERVE0,
        REAL_BASIC_RESERVE1,
        REAL_BASIC_TOTAL_SUPPLY,
        REAL_BASIC_LP_STAKED,
        0,
        0,
    )
    .unwrap()
}

/// The staked CL position as fetch_aero_cl_position assembles it.
fn real_cl_position() -> (RawPosition, PositionFacts) {
    let cfg = cl_config();
    let decoded = decode_slipstream_positions_return(REAL_POSITIONS_2914490).unwrap();
    let (sqrt_price_x96, tick) = decode_slipstream_slot0(REAL_CL_SLOT0).unwrap();
    let owner = address_from_word(REAL_OWNER_OF_2914490).unwrap();
    let pool_gauge = address_from_word(REAL_CL_GAUGE_WORD).unwrap();
    let NftHolder::Gauge(gauge) = classify_nft_holder(&cfg.id, cfg.nft_id, WALLET, &owner, &pool_gauge, None).unwrap()
    else {
        panic!("the NFT is held by the gauge")
    };
    require_staked_by_wallet(&cfg.id, cfg.nft_id, WALLET, &gauge, REAL_STAKED_CONTAINS_WALLET).unwrap();
    aero_cl_position_from_reads(
        &cfg,
        WALLET,
        NPM,
        CL_POOL,
        &decoded,
        &pool_position_manager(&cfg.id, CL_POOL, NPM, Some(REAL_CL_NFT)).unwrap(),
        &address_from_word(REAL_CL_TOKEN0).unwrap(),
        &address_from_word(REAL_CL_TOKEN1).unwrap(),
        200,
        9700,
        sqrt_price_x96,
        tick,
        Some(gauge),
        AeroClFees::StakedNotLpClaimable,
    )
    .unwrap()
}

pub(super) fn real_eth_usd() -> f64 {
    sqrt_price_x96_to_raw(U256::from_hex_be(REAL_ETH_POOL_SQRT_PRICE).unwrap()) * 1e18 / 1e6
}

pub(super) fn real_nock_usd() -> f64 {
    // v4 USDC(6)/NOCK(16) pool: raw = NOCK_raw per USDC_raw.
    1e10 / sqrt_price_x96_to_raw(U256::from_hex_be(REAL_NOCK_USDC_SQRT_PRICE).unwrap())
}

pub(super) fn real_aero_price() -> Result<f64> {
    aero_price_from_pool_reads(
        DEFAULT_AERO_PRICE_POOL,
        REAL_PRICE_POOL_TOKEN0,
        REAL_PRICE_POOL_TOKEN1,
        REAL_PRICE_POOL_SLOT0,
        REAL_PRICE_POOL_LIQUIDITY,
    )
}

pub(super) fn ok_hex(hex: &str) -> Result<Option<String>> {
    Ok(Some(hex.trim_start_matches("0x").to_string()))
}

/// The AERO reads as fetch_aero_reads assembles them, with the gauge answers supplied
/// by `earned` (gauge address → the outcome of its eth_call). Asserts that the calldata
/// each position sends is the call that was verified on-chain.
pub(super) fn aero_reads_from(
    configs: &[PositionConfig],
    facts: &HashMap<String, PositionFacts>,
    wallet_aero_hex: &str,
    price: Result<f64>,
    earned: &dyn Fn(&str) -> Result<Option<String>>,
) -> Result<AeroReads> {
    let wallet_aero = require_wallet_aero(decode_amount_word(wallet_aero_hex, "AERO balanceOf()").ok().map(aero_units))?;
    let mut unclaimed = Vec::new();
    for cfg in configs {
        let id = position_key(cfg);
        match aero_reward_source(cfg, facts.get(&id)) {
            AeroRewardSource::NotEarning => {}
            AeroRewardSource::GaugeNotConfigured => unclaimed.push(unclaimed_without_gauge(&id)),
            AeroRewardSource::Gauge { gauge, calldata } => {
                let data = calldata(WALLET);
                if gauge.eq_ignore_ascii_case(BASIC_GAUGE) || gauge.eq_ignore_ascii_case(LEGACY_BASIC_GAUGE) {
                    assert_eq!(data, "0x008cc262000000000000000000000000ee114cc97ec5adeb34ba5645e315b497d8487a45");
                } else {
                    assert_eq!(
                        data,
                        "0xd942be03000000000000000000000000ee114cc97ec5adeb34ba5645e315b497d8487a450000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000002c78ba"
                    );
                }
                // As fetch_aero_reads: which token the gauge pays is asked first.
                unclaimed.push(match gauge_pays_aero(&gauge, &real_reward_token(&gauge)) {
                    Err(note) => unclaimed_not_read(&id, &gauge, note),
                    Ok(()) => unclaimed_from_earned_call(&id, &gauge, &earned(&gauge)),
                });
            }
        }
    }
    let (price_usd, price_note) = resolve_aero_price(wallet_aero, price)?;
    Ok(AeroReads { wallet_aero: Some(wallet_aero), price_usd, price_note, unclaimed: Some(unclaimed) })
}

/// Each gauge's real answer to `rewardToken()`: the Aero app's two gauges revert, the
/// legacy gauges name the older token.
fn real_reward_token(gauge: &str) -> Result<Option<String>> {
    if gauge.eq_ignore_ascii_case(BASIC_GAUGE) || gauge.eq_ignore_ascii_case(CL_GAUGE) {
        classify_eth_call_response(&serde_json::from_str(REAL_REWARD_TOKEN_RESPONSE).unwrap())
    } else if gauge.eq_ignore_ascii_case(LEGACY_BASIC_GAUGE) {
        ok_hex(REAL_LEGACY_REWARD_TOKEN)
    } else if gauge.eq_ignore_ascii_case(LEGACY_CL_GAUGE) {
        ok_hex(REAL_LEGACY_CL_REWARD_TOKEN)
    } else {
        panic!("unexpected gauge {gauge}")
    }
}

pub(super) fn real_earned(gauge: &str) -> Result<Option<String>> {
    if gauge.eq_ignore_ascii_case(BASIC_GAUGE) {
        ok_hex(REAL_BASIC_EARNED_WALLET)
    } else if gauge.eq_ignore_ascii_case(LEGACY_BASIC_GAUGE) {
        ok_hex(REAL_LEGACY_EARNED_WALLET)
    } else if gauge.eq_ignore_ascii_case(CL_GAUGE) {
        ok_hex(REAL_CL_EARNED_WALLET)
    } else {
        panic!("unexpected gauge {gauge}")
    }
}

/// One cycle's reads for the real wallet at block 58077673, holding its two staked Aero
/// positions (the wallet's two Uniswap positions are left out to keep the case about
/// #93). Gas cost is a stated figure, not a pinned read.
pub(super) fn real_cycle_with(aero: &dyn Fn(&[PositionConfig], &HashMap<String, PositionFacts>) -> AeroReads) -> RawOnChainData {
    let basic = real_basic_position();
    let (cl, facts) = real_cl_position();
    let (nock_usd, eth_usd) = (real_nock_usd(), real_eth_usd());
    let wallet_nock = REAL_WALLET_NOCK_RAW as f64 / 1e16;
    let wallet_usdc = REAL_WALLET_USDC_RAW as f64 / 1e6;
    let wallet_weth = REAL_WALLET_WETH_RAW as f64 / 1e18;
    let wallet_eth = REAL_WALLET_ETH_RAW as f64 / 1e18;
    let now = "2026-11-16T05:11:50+00:00";
    let bal = |symbol: &str, address: &str, amount: String, usd: f64| WalletBalance {
        wallet_address: WALLET.to_string(),
        token_symbol: symbol.to_string(),
        token_address: address.to_string(),
        amount,
        amount_usd: format!("{:.2}", usd),
        updated_at: now.to_string(),
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
    raw.pool_liquidity.insert(basic.position_id.clone(), REAL_BASIC_TOTAL_SUPPLY.to_string());
    raw.pool_liquidity.insert(cl.position_id.clone(), word_u(REAL_CL_LIQUIDITY).to_string());
    for p in [&basic, &cl] {
        raw.rebuild_rules.insert(p.position_id.clone(), RebuildRule::default());
        raw.fees_not_lp_claimable.push(p.position_id.clone());
    }
    raw.position_facts.insert(cl.position_id.clone(), facts);
    raw.positions = vec![basic, cl];
    raw.aero = aero(&[basic_config(Some(BASIC_GAUGE)), cl_config()], &raw.position_facts);
    raw.wallet_balances.push(aero_wallet_balance(WALLET, raw.aero.wallet_aero.unwrap(), raw.aero.price_usd, now));
    raw
}

pub(super) fn real_cycle() -> RawOnChainData {
    real_cycle_with(&|cfgs, facts| {
        aero_reads_from(cfgs, facts, REAL_AERO_BALANCE_WALLET, real_aero_price(), &real_earned).unwrap()
    })
}

/// The explicit numbers of config.yaml.example (a test profile, not a product default).
pub(super) fn example_profile() -> OperatorProfile {
    OperatorProfile {
        treasury_floor_nock: 100_000.0,
        lp_floor_tolerance_bps: 0,
        tax_reserve_ratio: 0.40,
        target_weights_bps: TargetWeightsBps { treasury: 8000, uniswap_lp: 1500, cash: 500 },
        cash_reserve_usdc: None,
        ladder_bottom_usd: None,
    }
}

/// The tail of fetch_raw_data plus run_ingestion_cycle's pure steps.
pub(super) fn snapshot_of(mut raw: RawOnChainData, profile: &OperatorProfile, prev_hwm: f64) -> SnapshotData {
    let (spot, eth) = (raw.spot_price_nock_usd, raw.live_eth_price_usd);
    let positions = snapshot_positions(&raw);
    raw.pool_mechanics = build_pool_mechanics(&raw.positions, spot, eth);
    raw.contract_fees = build_contract_fees(&positions, raw.live_gas_cost_usd, &raw.fees_not_lp_claimable);
    let plan = PlanContext {
        cfg: None,
        fills: PlanFills::default(),
        ledger_unavailable: false,
        now: Utc.with_ymd_and_hms(2026, 11, 16, 5, 15, 0).unwrap(),
    };
    let derived_metrics = compute_derived_metrics_with_plan(
        &raw,
        prev_hwm,
        profile,
        StructuralChannels { channels: vec![] },
        &[],
        &LedgerAggregate::default(),
        &MarketRegime::default(),
        &plan,
    );
    SnapshotData {
        ladders: Vec::new(),
        closed_positions: vec![],
        wallet_balances: raw.wallet_balances,
        positions,
        pool_mechanics: raw.pool_mechanics,
        concentrated_liquidity: raw.concentrated_liquidity,
        contract_fees: raw.contract_fees,
        derived_metrics,
    }
}

pub(super) fn f(s: &str) -> f64 {
    s.parse().unwrap()
}

fn abi_string(hex: &str) -> String {
    let clean = hex.trim_start_matches("0x");
    let len = usize::from_str_radix(&clean[64..128], 16).unwrap();
    String::from_utf8((0..len).map(|i| u8::from_str_radix(&clean[128 + 2 * i..130 + 2 * i], 16).unwrap()).collect()).unwrap()
}

/// Every string in a JSON value, for checks over a whole snapshot.
pub(super) fn all_strings(v: &serde_json::Value, out: &mut Vec<String>) {
    match v {
        serde_json::Value::String(s) => out.push(s.clone()),
        serde_json::Value::Array(a) => a.iter().for_each(|x| all_strings(x, out)),
        serde_json::Value::Object(o) => o.values().for_each(|x| all_strings(x, out)),
        _ => {}
    }
}

// ------------------------------------------------------------------
// The token and the calls
// ------------------------------------------------------------------

#[test]
fn test_aero_is_the_token_the_gauges_pay_not_the_older_one_with_the_same_symbol() {
    assert_eq!(word_u(REAL_AERO_DECIMALS) as i32, AERO_DECIMALS);
    assert_eq!(abi_string(REAL_AERO_SYMBOL), "AERO");
    assert_eq!(abi_string(REAL_AERO_NAME), "Aero");
    // The real claim minted this token to the claimer.
    let receipt = real_claim_receipt();
    let paid: Vec<&str> = receipt["logs"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|l| l["topics"][2].as_str().unwrap().ends_with(&CLAIMER[2..]))
        .filter(|l| l["topics"][0] == TRANSFER_EVENT_TOPIC)
        .map(|l| l["address"].as_str().unwrap())
        .collect();
    assert_eq!(paid.len(), 1);
    assert!(paid[0].eq_ignore_ascii_case(AERO_BASE));
    assert!(!AERO_BASE.eq_ignore_ascii_case(LEGACY_AERODROME_AERO));
    // The price pool's token0 is that same token.
    assert!(address_from_word(REAL_PRICE_POOL_TOKEN0).unwrap().eq_ignore_ascii_case(AERO_BASE));
    // AERO is not a position leg: the registry does not know it, so a pool holding it
    // cannot be tracked by accident and valued at 0.
    assert!(token_info(AERO_BASE).is_err());
}

#[test]
fn test_earned_calldata_is_the_call_verified_on_chain_for_each_gauge_type() {
    assert_eq!(
        gauge_earned_basic_calldata(WALLET),
        "0x008cc262000000000000000000000000ee114cc97ec5adeb34ba5645e315b497d8487a45"
    );
    assert_eq!(
        gauge_earned_cl_calldata(WALLET, NFT_ID),
        "0xd942be03000000000000000000000000ee114cc97ec5adeb34ba5645e315b497d8487a450000000000000000000000000000000000000000000000000000000000000040000000000000000000000000000000000000000000000000000000000000000100000000000000000000000000000000000000000000000000000000002c78ba"
    );
    assert_eq!(
        erc20_balance_of_calldata(WALLET),
        "0x70a08231000000000000000000000000ee114cc97ec5adeb34ba5645e315b497d8487a45"
    );
}

#[test]
fn test_each_protocol_reads_unclaimed_aero_from_the_right_place() {
    let gauge_of = |cfg: &PositionConfig, facts: Option<&PositionFacts>| match aero_reward_source(cfg, facts) {
        AeroRewardSource::Gauge { gauge, calldata } => format!("{gauge} {}", &calldata(WALLET)[..10]),
        AeroRewardSource::GaugeNotConfigured => "not configured".to_string(),
        AeroRewardSource::NotEarning => "not earning".to_string(),
    };
    // Basic pool: the configured gauge, `earned(address)`; without one, said as such.
    assert_eq!(gauge_of(&basic_config(Some(BASIC_GAUGE)), None), format!("{BASIC_GAUGE} 0x008cc262"));
    assert_eq!(gauge_of(&basic_config(None), None), "not configured");
    assert_eq!(gauge_of(&basic_config(Some("  ")), None), "not configured");
    // CL: the gauge the cycle proved the wallet staked the NFT in, `earned(address,uint256[])`.
    let (_, staked) = real_cl_position();
    assert_eq!(gauge_of(&cl_config(), Some(&staked)), format!("{CL_GAUGE} 0xd942be03"));
    // An unstaked CL NFT earns swap fees, not AERO; a Uniswap position earns none.
    let unstaked = PositionFacts { tick_spacing: Some(200), staked_in_gauge: None, fees_note: None, ..Default::default() };
    assert_eq!(gauge_of(&cl_config(), Some(&unstaked)), "not earning");
    assert_eq!(gauge_of(&cl_config(), None), "not earning");
    for protocol in ["uniswap-v3", "uniswap-v4"] {
        let mut cfg = cl_config();
        cfg.protocol = protocol.to_string();
        cfg.gauge_address = Some(CL_GAUGE.to_string());
        assert_eq!(gauge_of(&cfg, Some(&staked)), "not earning");
    }
}

// ------------------------------------------------------------------
// Unclaimed AERO from real gauge answers
// ------------------------------------------------------------------

#[test]
fn test_unclaimed_aero_decodes_from_real_gauge_reads() {
    let basic = unclaimed_from_earned_call("b", BASIC_GAUGE, &ok_hex(REAL_BASIC_EARNED_WALLET));
    assert!((basic.amount.unwrap() - 0.121026111298851597).abs() < 1e-15);
    assert_eq!(basic.note, None);
    let cl = unclaimed_from_earned_call("c", CL_GAUGE, &ok_hex(REAL_CL_EARNED_WALLET));
    assert!((cl.amount.unwrap() - 11.922077777221318098).abs() < 1e-12);
    assert_eq!(cl.gauge.as_deref(), Some(CL_GAUGE));
}

#[test]
fn test_a_gauge_with_zero_earned_is_a_read_zero_not_an_unread_value() {
    // Real answers: an account with no stake on the basic gauge, no token ids on the CL one.
    for (gauge, hex) in [(BASIC_GAUGE, REAL_BASIC_EARNED_NO_STAKE), (CL_GAUGE, REAL_CL_EARNED_NO_IDS)] {
        let e = unclaimed_from_earned_call("p", gauge, &ok_hex(hex));
        assert_eq!(e.amount, Some(0.0));
        assert_eq!(e.note, None);
        // Zero needs no price to be worth $0.
        assert_eq!(
            aero_position_fields(&e, None),
            (Some("0.000000".to_string()), Some("0.00".to_string()), None)
        );
        assert_eq!(
            aero_statement(Some("0.000000"), Some("0.00"), None).as_deref(),
            Some("unclaimed AERO 0.000000 ($0.00)")
        );
    }
    let reads = AeroReads {
        wallet_aero: Some(0.0),
        price_usd: None,
        price_note: Some("AERO/USD price unavailable: test".to_string()),
        unclaimed: Some(vec![unclaimed_from_earned_call("p", BASIC_GAUGE, &ok_hex(REAL_BASIC_EARNED_NO_STAKE))]),
    };
    let pillar = aero_pillar(&reads);
    assert_eq!((pillar.unclaimed_amount, pillar.unclaimed_usd), (Some(0.0), Some(0.0)));
}

#[test]
fn test_a_reverting_or_failed_gauge_read_is_stated_never_zero() {
    // Real: the CL gauge reverts "NA" for an id the wallet has not staked, and the
    // older gauge's earned(address,uint256) reverts on it.
    for response in [REAL_CL_EARNED_NOT_STAKED_RESPONSE, REAL_CL_EARNED_OLD_SELECTOR_RESPONSE, REAL_REWARD_TOKEN_RESPONSE] {
        let call = classify_eth_call_response(&serde_json::from_str(response).unwrap());
        assert!(matches!(call, Ok(None)), "a revert is an answer, not an outage");
        let e = unclaimed_from_earned_call("aerodrome-cl:base:2914490", CL_GAUGE, &call);
        assert_eq!(e.amount, None);
        assert_eq!(e.note.as_deref(), Some("gauge 0x3bf70c880adc8370917994ab0df68e763b7a60fd earned() reverted"));
    }
    // The node did not answer (synthetic transport error).
    let e = unclaimed_from_earned_call("p", BASIC_GAUGE, &Err(anyhow::anyhow!("HTTP 429 Too Many Requests")));
    assert_eq!(e.amount, None);
    assert!(e.note.as_deref().unwrap().contains("earned() read failed: HTTP 429"), "{:?}", e.note);
    // An answer that is not one word (synthetic).
    let e = unclaimed_from_earned_call("p", BASIC_GAUGE, &Ok(Some("00".to_string())));
    assert_eq!(e.amount, None);
    assert!(e.note.as_deref().unwrap().contains("expected 64"), "{:?}", e.note);

    // On every surface: no amount, the reason, and no total (a partial sum would understate).
    let failing = |gauge: &str| -> Result<Option<String>> {
        if gauge.eq_ignore_ascii_case(CL_GAUGE) {
            Err(anyhow::anyhow!("HTTP 429 Too Many Requests"))
        } else {
            real_earned(gauge)
        }
    };
    let raw = real_cycle_with(&|cfgs, facts| {
        aero_reads_from(cfgs, facts, REAL_AERO_BALANCE_WALLET, real_aero_price(), &failing).unwrap()
    });
    let snap = snapshot_of(raw, &OperatorProfile::default(), 0.0);
    let g = &snap.derived_metrics.global_status;
    assert_eq!(g.unclaimed_aero_amount, None);
    assert_eq!(g.unclaimed_aero_usd, None);
    let note = g.aero_note.as_deref().unwrap();
    assert!(note.contains("unclaimed AERO not read for aerodrome-cl:base:2914490"), "{note}");
    assert!(note.contains("HTTP 429"), "{note}");
    let cl = &snap.positions[1];
    assert_eq!((cl.unclaimed_aero.as_deref(), cl.unclaimed_aero_usd.as_deref()), (None, None));
    assert!(cl.unclaimed_aero_note.as_deref().unwrap().contains("earned() read failed"));
    assert!(snap.contract_fees[1].amount.ends_with("; unclaimed AERO not read (gauge 0x3bf70c880adc8370917994ab0df68e763b7a60fd earned() read failed: HTTP 429 Too Many Requests)"), "{}", snap.contract_fees[1].amount);
    // The gauge that did answer still shows its amount.
    assert_eq!(snap.positions[0].unclaimed_aero.as_deref(), Some("0.121026"));
    let text = format_portfolio_state_text(&serde_json::to_value(&snap).unwrap());
    assert!(text.contains("• Unclaimed AERO: not reported\n"), "{text}");
    assert!(text.contains("   - Aero CL WETH/NOCK #2914490: not read (gauge "), "{text}");
    assert!(text.contains("• AERO not read: unclaimed AERO not read for aerodrome-cl:base:2914490"), "{text}");
}

#[test]
fn test_basic_pool_without_a_gauge_says_its_aero_is_not_read() {
    let e = unclaimed_without_gauge("aerodrome:base:0x770f");
    assert_eq!((e.gauge.clone(), e.amount), (None, None));
    assert_eq!(
        aero_statement(None, None, e.note.as_deref()).as_deref(),
        Some("unclaimed AERO not read (no gauge_address is configured for this position)")
    );
    // Nothing was read, so there is no total: not 0 (which would say "nothing unclaimed"),
    // and the note names the position.
    let reads = AeroReads { wallet_aero: Some(0.0), price_usd: Some(0.4176), price_note: None, unclaimed: Some(vec![e.clone()]) };
    let pillar = aero_pillar(&reads);
    assert_eq!((pillar.unclaimed_amount, pillar.unclaimed_usd), (None, None));
    assert!(pillar.note.as_deref().unwrap().contains("no gauge_address is configured"));
    // Beside a gauge that WAS read (the real CL amount) the total is still None, never
    // the partial sum 11.922078: the basic gauge really held 0.121026 AERO for the wallet.
    let cl = unclaimed_from_earned_call("aerodrome-cl:base:2914490", CL_GAUGE, &ok_hex(REAL_CL_EARNED_WALLET));
    assert_eq!(cl.amount.map(|a| format!("{a:.6}")).as_deref(), Some("11.922078"));
    let mixed = AeroReads { wallet_aero: Some(0.0), price_usd: Some(0.4176), price_note: None, unclaimed: Some(vec![e, cl]) };
    let pillar = aero_pillar(&mixed);
    assert_eq!((pillar.unclaimed_amount, pillar.unclaimed_usd), (None, None));
    assert!(pillar.note.as_deref().unwrap().contains("unclaimed AERO not read for aerodrome:base:0x770f (no gauge_address is configured"));
    // On the published snapshot: null total, the reason, the read gauge's own amount kept.
    let raw = real_cycle_with(&|_, facts| {
        aero_reads_from(&[basic_config(None), cl_config()], facts, REAL_AERO_BALANCE_WALLET, real_aero_price(), &real_earned).unwrap()
    });
    let snap = snapshot_of(raw, &OperatorProfile::default(), 0.0);
    let g = &snap.derived_metrics.global_status;
    assert_eq!((g.unclaimed_aero_amount.as_deref(), g.unclaimed_aero_usd.as_deref()), (None, None));
    assert!(g.aero_note.as_deref().unwrap().contains("no gauge_address is configured"), "{:?}", g.aero_note);
    assert_eq!(snap.positions[0].unclaimed_aero, None);
    assert_eq!(snap.positions[1].unclaimed_aero.as_deref(), Some("11.922078"));
    let text = format_portfolio_state_text(&serde_json::to_value(&snap).unwrap());
    assert!(text.contains("• Unclaimed AERO: not reported\n   - Aero USDC/NOCK: not read (no gauge_address is configured for this position)\n   - Aero CL WETH/NOCK #2914490: 11.922078 AERO ($4.98)"), "{text}");
    // An older worker's snapshot could carry a 0 total beside an unread row; the MCP text
    // does not present that 0 as an amount read from a gauge.
    let data = json!({
        "positions": [{ "protocol": "aerodrome", "token0": { "symbol": "USDC" }, "token1": { "symbol": "NOCK" },
                        "unclaimed_aero_note": "no gauge_address is configured for this position" }],
        "derived_metrics": { "global_status": {
            "wallet_aero_amount": "0.000000", "wallet_aero_usd": "0.00",
            "unclaimed_aero_amount": "0.000000", "unclaimed_aero_usd": "0.00", "aero_price_usd": "0.417600" } }
    });
    let text = aero_lines(&data);
    assert!(text.contains("• Unclaimed AERO: no gauge was read\n   - Aero USDC/NOCK: not read (no gauge_address is configured for this position)"), "{text}");
}

// ------------------------------------------------------------------
// The AERO/USD price
// ------------------------------------------------------------------

#[test]
fn test_aero_price_from_the_real_pool() {
    let price = real_aero_price().unwrap();
    assert!((price - 0.417608).abs() < 1e-6, "AERO/USD {price}");
    // Cross-check against a second live market at the same block: AERO/WETH times
    // ETH/USD. The two agree within the pools' swap fees.
    let weth_per_aero = sqrt_price_x96_to_raw(U256::from_hex_be(REAL_AERO_WETH_SQRT_PRICE).unwrap());
    let cross = weth_per_aero * real_eth_usd();
    assert!((cross - 0.415842).abs() < 1e-6, "cross {cross}");
    assert!((price / cross - 1.0).abs() < 0.01);
    // The same pool at the block of the real claim (57635701).
    let at_claim = aero_price_from_pool_reads(
        DEFAULT_AERO_PRICE_POOL,
        REAL_PRICE_POOL_TOKEN0,
        REAL_PRICE_POOL_TOKEN1,
        REAL_PRICE_POOL_SLOT0_AT_CLAIM,
        REAL_PRICE_POOL_LIQUIDITY_AT_CLAIM,
    )
    .unwrap();
    assert!((at_claim - 0.438847).abs() < 1e-6, "AERO/USD at the claim {at_claim}");
}

#[test]
fn test_a_stale_aero_price_pool_is_refused_from_real_reads() {
    // A real pool of the same pair with no in-range liquidity: its frozen price is 69%
    // above the market, and it is never used.
    let err = aero_price_from_pool_reads(
        DEAD_PRICE_POOL,
        REAL_PRICE_POOL_TOKEN0,
        REAL_PRICE_POOL_TOKEN1,
        REAL_DEAD_POOL_SLOT0,
        REAL_DEAD_POOL_LIQUIDITY,
    )
    .unwrap_err()
    .to_string();
    assert!(err.starts_with("Stale AERO/USD price: price pool 0xb27460d0"), "{err}");
    assert!(err.contains("zero in-range liquidity") && err.contains("aero_price_pool"), "{err}");
    let frozen = sqrt_price_x96_to_raw(U256::from_hex_be(&REAL_DEAD_POOL_SLOT0[2..66]).unwrap()) * 1e12;
    assert!((frozen / real_aero_price().unwrap() - 1.0) > 0.6, "frozen {frozen}");
}

#[test]
fn test_a_price_pool_that_is_not_aero_usdc_is_a_configuration_error() {
    // The real ETH/USD pool (WETH, USDC) given as aero_price_pool.
    let err = aero_price_from_pool_reads(
        DEFAULT_ETH_PRICE_POOL,
        REAL_ETH_POOL_TOKEN0,
        REAL_PRICE_POOL_TOKEN1,
        &format!("{REAL_ETH_POOL_SQRT_PRICE}{}", "0".repeat(64)),
        REAL_ETH_POOL_LIQUIDITY,
    )
    .unwrap_err()
    .to_string();
    assert!(err.starts_with("Configuration error: aero_price_pool 0xd0b53d92"), "{err}");
    assert!(err.contains("is not an AERO/USDC pool"), "{err}");
    // A pool of the OLDER "AERO" against USDC is refused too (synthetic token0 word).
    let legacy_word = format!("0x{:0>64}", LEGACY_AERODROME_AERO[2..].to_lowercase());
    assert!(aero_price_from_pool_reads("0xpool", &legacy_word, REAL_PRICE_POOL_TOKEN1, REAL_PRICE_POOL_SLOT0, REAL_PRICE_POOL_LIQUIDITY).is_err());
    // An uninitialized pool and a short answer are errors, never a price (synthetic).
    let zero_slot0 = format!("0x{}", "0".repeat(128));
    assert!(aero_price_from_pool_reads("0xpool", REAL_PRICE_POOL_TOKEN0, REAL_PRICE_POOL_TOKEN1, &zero_slot0, REAL_PRICE_POOL_LIQUIDITY).unwrap_err().to_string().contains("not initialized"));
    assert!(aero_price_from_pool_reads("0xpool", REAL_PRICE_POOL_TOKEN0, REAL_PRICE_POOL_TOKEN1, "0x", REAL_PRICE_POOL_LIQUIDITY).is_err());
    // Token order does not matter (synthetic: the real reads with the tokens swapped and
    // the price inverted give the same AERO/USD).
    let real = real_aero_price().unwrap();
    let raw = sqrt_price_x96_to_raw(U256::from_hex_be(&REAL_PRICE_POOL_SLOT0[2..66]).unwrap());
    let inv_sqrt = (1.0 / raw).sqrt() * 79228162514264337593543950336.0;
    let inverted_slot0 = format!("0x{:064x}{}", inv_sqrt as u128, "0".repeat(64));
    let swapped = aero_price_from_pool_reads("0xpool", REAL_PRICE_POOL_TOKEN1, REAL_PRICE_POOL_TOKEN0, &inverted_slot0, REAL_PRICE_POOL_LIQUIDITY).unwrap();
    assert!((swapped / real - 1.0).abs() < 1e-9, "{swapped} vs {real}");
}

#[test]
fn test_required_and_optional_aero_reads() {
    // The wallet balance is counted in the total: required.
    let err = require_wallet_aero(None).unwrap_err().to_string();
    assert!(err.contains("Failed to read wallet AERO balance") && err.contains("rather than counting it as 0"), "{err}");
    assert_eq!(require_wallet_aero(Some(0.0)).unwrap(), 0.0);

    let failed = || -> Result<f64> { Err(anyhow::anyhow!("AERO price pool 0xd0ac slot0() read failed")) };
    // No AERO in the wallet: the price cannot fail the cycle. It degrades with its reason.
    let (price, note) = resolve_aero_price(0.0, failed()).unwrap();
    assert_eq!(price, None);
    assert_eq!(note.as_deref(), Some("AERO/USD price unavailable: AERO price pool 0xd0ac slot0() read failed"));
    // The wallet holds AERO: its value is in the total, so the price is required.
    let err = resolve_aero_price(1.359979, failed()).unwrap_err().to_string();
    assert!(err.starts_with("Failed to read the AERO/USD price (AERO price pool 0xd0ac slot0() read failed)"), "{err}");
    assert!(err.contains("the wallet holds 1.359979 AERO") && err.contains("the cycle fails"), "{err}");
    // A stale pool is a failed price like any other.
    let stale = aero_price_from_pool_reads(DEAD_PRICE_POOL, REAL_PRICE_POOL_TOKEN0, REAL_PRICE_POOL_TOKEN1, REAL_DEAD_POOL_SLOT0, REAL_DEAD_POOL_LIQUIDITY);
    assert!(resolve_aero_price(0.5, stale).unwrap_err().to_string().contains("Stale AERO/USD price"));
    // A good read is a price with no note, whatever the balance.
    assert_eq!(resolve_aero_price(0.0, Ok(0.4176)).unwrap(), (Some(0.4176), None));
    assert_eq!(resolve_aero_price(2.0, Ok(0.4176)).unwrap(), (Some(0.4176), None));
}

// ------------------------------------------------------------------
// One real cycle
// ------------------------------------------------------------------

#[test]
fn test_real_cycle_states_wallet_and_unclaimed_aero() {
    let snap = snapshot_of(real_cycle(), &OperatorProfile::default(), 0.0);
    let g = &snap.derived_metrics.global_status;
    assert_eq!(g.aero_price_usd.as_deref(), Some("0.417608"));
    assert_eq!(g.wallet_aero_amount.as_deref(), Some("0.000000"));
    assert_eq!(g.wallet_aero_usd.as_deref(), Some("0.00"));
    assert_eq!(g.unclaimed_aero_amount.as_deref(), Some("12.043104"));
    assert_eq!(g.unclaimed_aero_usd.as_deref(), Some("5.03"));
    assert_eq!(g.aero_note, None);

    // Per position: the amount read from its own gauge.
    let (basic, cl) = (&snap.positions[0], &snap.positions[1]);
    assert_eq!((basic.unclaimed_aero.as_deref(), basic.unclaimed_aero_usd.as_deref()), (Some("0.121026"), Some("0.05")));
    assert_eq!((cl.unclaimed_aero.as_deref(), cl.unclaimed_aero_usd.as_deref()), (Some("11.922078"), Some("4.98")));
    assert_eq!((basic.unclaimed_aero_note.as_deref(), cl.unclaimed_aero_note.as_deref()), (None, None));
    // The fee rows and the velocity panel say why there is no fee figure, then the AERO.
    assert_eq!(snap.contract_fees[0].amount, "not LP-claimable (claimFees() reverts); unclaimed AERO 0.121026 ($0.05)");
    assert_eq!(
        snap.contract_fees[1].amount,
        "not LP-claimable (staked: swap fees on staked liquidity go to the gauge); unclaimed AERO 11.922078 ($4.98)"
    );
    let v = &snap.derived_metrics.velocity_gas_sandbox.contracts;
    assert_eq!(v[0].note.as_deref(), Some(snap.contract_fees[0].amount.as_str()));
    assert_eq!(v[1].note.as_deref(), Some(snap.contract_fees[1].amount.as_str()));
    assert!(v.iter().all(|c| c.recommended_action == "not_claimable" && c.claimable_usd == "0.00"));
    // The reason on the card is the reason alone.
    assert_eq!(cl.fees_note.as_deref(), Some("not LP-claimable (staked: swap fees on staked liquidity go to the gauge)"));
    // A wallet_balances row, after the four that were there.
    let symbols: Vec<&str> = snap.wallet_balances.iter().map(|w| w.token_symbol.as_str()).collect();
    assert_eq!(symbols, ["NOCK", "USDC", "ETH", "WETH", "AERO"]);
    let row = &snap.wallet_balances[4];
    assert_eq!((row.token_address.as_str(), row.amount.as_str(), row.amount_usd.as_str()), (AERO_BASE, "0.000000", "0.00"));
    // Nothing that said "not read (#93)" is left anywhere in the snapshot or the MCP text.
    let full = serde_json::to_value(&snap).unwrap();
    let mut strings = Vec::new();
    all_strings(&full, &mut strings);
    for text in [format_portfolio_state_text(&full), format_shot_advice_text(&full), format_claim_simulation_text(&full), format_accumulation_plan_text(&full)] {
        strings.push(text);
    }
    assert!(strings.iter().all(|s| !s.contains("#93") && !s.contains("AERO rewards not read")), "a surface still says AERO is not read");
}

#[test]
fn test_unclaimed_aero_is_not_in_the_total_and_the_four_pillars_add_up() {
    let with_aero = snapshot_of(real_cycle(), &example_profile(), 0.0);
    // The same cycle with no AERO read at all (as before #93).
    let mut raw = real_cycle();
    raw.aero = AeroReads::default();
    raw.wallet_balances.pop();
    let without = snapshot_of(raw, &example_profile(), 0.0);
    let (g, g0) = (&with_aero.derived_metrics.global_status, &without.derived_metrics.global_status);
    // $5.03 of unclaimed AERO moves no total: it is treated as uncollected LP fees are.
    assert_eq!(g.total_portfolio_usd, g0.total_portfolio_usd);
    assert_eq!(g.high_water_mark_usd, g0.high_water_mark_usd);
    assert_eq!(g.total_fees_accrued_usd, g0.total_fees_accrued_usd);
    assert_eq!(with_aero.derived_metrics.lp_rebuild.target_usd, without.derived_metrics.lp_rebuild.target_usd);
    // NOCK + USDC + ETH + AERO = total.
    let sum = f(&g.total_native_usd) + f(&g.total_stable_usd) + f(g.total_eth_usd.as_deref().unwrap()) + f(g.wallet_aero_usd.as_deref().unwrap());
    assert!((sum - f(&g.total_portfolio_usd)).abs() < 0.03, "{sum} vs {}", g.total_portfolio_usd);
    // And no directive is about it.
    assert_eq!(g.shot_advice_action, g0.shot_advice_action);
    assert_eq!(g.shot_advice_details, g0.shot_advice_details);
    // Before #93 nothing was read: every AERO figure is null, never 0.
    for v in [&g0.aero_price_usd, &g0.wallet_aero_amount, &g0.wallet_aero_usd, &g0.unclaimed_aero_amount, &g0.unclaimed_aero_usd, &g0.aero_note] {
        assert_eq!(*v, None);
    }
}

#[test]
fn test_wallet_aero_counts_in_the_one_total_everything_else_uses() {
    // Synthetic balance: the wallet holds the amount of the real claim (1.359979 AERO),
    // priced by the real pool read. Everything else is the real cycle.
    let claimed = REAL_EARNED_BEFORE_CLAIM;
    let holding = real_cycle_with(&|cfgs, facts| aero_reads_from(cfgs, facts, claimed, real_aero_price(), &real_earned).unwrap());
    let profile = example_profile();
    let base = snapshot_of(real_cycle(), &profile, 0.0);
    let snap = snapshot_of(holding, &profile, 0.0);
    let (g, g0) = (&snap.derived_metrics.global_status, &base.derived_metrics.global_status);
    let aero_usd = 1.359979366619354917 * real_aero_price().unwrap();
    assert_eq!(g.wallet_aero_amount.as_deref(), Some("1.359979"));
    assert_eq!(g.wallet_aero_usd.as_deref(), Some("0.57"));
    assert!((f(g.wallet_aero_usd.as_deref().unwrap()) - aero_usd).abs() < 0.006);
    // The total steps up by exactly the wallet's AERO, once.
    let total = f(&g.total_portfolio_usd);
    assert!((total - f(&g0.total_portfolio_usd) - aero_usd).abs() < 0.011, "{} vs {}", g.total_portfolio_usd, g0.total_portfolio_usd);
    // The four pillars are the total.
    let sum = f(&g.total_native_usd) + f(&g.total_stable_usd) + f(g.total_eth_usd.as_deref().unwrap()) + f(g.wallet_aero_usd.as_deref().unwrap());
    assert!((sum - total).abs() < 0.03, "{sum} vs {total}");
    // The high-water mark, the value series, the counterfactual and the LP target all use it.
    assert_eq!(g.high_water_mark_usd, g.total_portfolio_usd);
    assert_eq!(snap.derived_metrics.total_value_vs_baseline.portfolio_value_series, vec![g.total_portfolio_usd.clone()]);
    assert_eq!(snap.derived_metrics.strategy_counterfactual.active_portfolio_series.last(), Some(&g.total_portfolio_usd));
    let lp = &snap.derived_metrics.lp_rebuild;
    assert!((f(&lp.target_usd) - total * 0.15).abs() < 0.011, "LP target {} of {total}", lp.target_usd);
    // Deviation weights are shares of that same total: wallet NOCK, LP, wallet USDC, gas
    // and AERO partition it.
    let rows = &snap.derived_metrics.portfolio_deviation_matrix.allocations;
    let names: Vec<&str> = rows.iter().map(|a| a.bucket_name.as_str()).collect();
    assert_eq!(names, ["Sovereign Treasury", "NOCK LP (NOCK-pair positions)", "Liquid USDC (Escrow/Vault)", "Gas (wallet ETH+WETH)", "AERO (wallet)"]);
    let weights: i32 = rows.iter().map(|a| a.actual_weight_bps).sum();
    assert!((9996..=10000).contains(&weights), "weights sum to {weights} bps");
    let aero_row = &rows[4];
    assert_eq!((aero_row.status.as_str(), aero_row.target_weight_bps), ("no_target", 0));
    assert_eq!(aero_row.actual_weight_bps, ((aero_usd / total) * 10000.0) as i32);
    assert_eq!(aero_row.deviation_bps, aero_row.actual_weight_bps);
    // AERO is not NOCK, not cash and not gas.
    assert_eq!(g.total_native_usd, g0.total_native_usd);
    assert_eq!(g.total_stable_usd, g0.total_stable_usd);
    assert_eq!(g.total_usdc_amount, g0.total_usdc_amount);
    assert_eq!(g.total_eth_usd, g0.total_eth_usd);
    assert_eq!(g.total_nock_amount, g0.total_nock_amount);
    // The wallet row carries the same figures.
    let row = snap.wallet_balances.iter().find(|w| w.token_symbol == "AERO").unwrap();
    assert_eq!((row.amount.as_str(), row.amount_usd.as_str()), ("1.359979", "0.57"));
}

#[test]
fn test_no_advice_about_aero_anywhere() {
    let claimed = REAL_EARNED_BEFORE_CLAIM;
    let holding = || real_cycle_with(&|cfgs, facts| aero_reads_from(cfgs, facts, claimed, real_aero_price(), &real_earned).unwrap());
    let regimes = [MarketRegime::default(), MarketRegime { rsi_5m: Some(80.0), rsi_1d: Some(75.0), overextended: true }];
    for profile in [OperatorProfile::default(), example_profile()] {
        for regime in &regimes {
            for raw in [real_cycle(), holding()] {
                let m = compute_derived_metrics(&raw, 0.0, &profile, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), regime);
                let g = &m.global_status;
                // No directive names AERO at all.
                assert!(!g.shot_advice_action.contains("AERO"), "{}", g.shot_advice_action);
                assert!(!g.shot_advice_details.contains("AERO"), "{}", g.shot_advice_details);
                // No rebuild suggestion is sized from or into AERO.
                for s in &m.lp_rebuild.suggestions {
                    assert!(s.token0_symbol != "AERO" && s.token1_symbol != "AERO");
                }
                // The AERO row has no target and no pressure.
                let row = m.portfolio_deviation_matrix.allocations.iter().find(|a| a.bucket_name == AERO_BUCKET_NAME).unwrap();
                assert_eq!(row.status, "no_target");
                // Unclaimed AERO never makes a harvest "due".
                assert!(!g.shot_advice_action.contains("HARVEST"));
                assert!(m.velocity_gas_sandbox.contracts.iter().all(|c| c.recommended_action == "not_claimable"));
            }
        }
    }
    // The words AERO is described with: amounts only.
    let snap = snapshot_of(holding(), &example_profile(), 0.0);
    let full = serde_json::to_value(&snap).unwrap();
    // (Of a fee row, the part after the reason its fees are not LP-claimable.)
    let aero_part = |amount: &str| amount.rsplit("; ").next().unwrap().to_string();
    let mut aero_text = vec![aero_lines(&full), aero_part(&snap.contract_fees[0].amount), aero_part(&snap.contract_fees[1].amount)];
    assert!(aero_text[1].starts_with("unclaimed AERO ") && aero_text[2].starts_with("unclaimed AERO "));
    aero_text.push(aero_statement(Some("1.0"), None, None).unwrap());
    aero_text.push(aero_statement(None, None, Some("gauge read failed")).unwrap());
    for text in &aero_text {
        let lower = text.to_lowercase();
        for banned in ["sell", "accumulate", "buy", "swap", "convert", "harvest", "should", "recommend", "advis", "claim now", "claim it", "hold it", "keep it", "stake more", "compound"] {
            assert!(!lower.contains(banned), "AERO text must state amounts only, found '{banned}' in: {text}");
        }
    }
    // An AERO-only wallet is not told to do anything with it (synthetic: no NOCK, no USDC).
    let mut raw = RawOnChainData { spot_price_nock_usd: real_nock_usd(), live_eth_price_usd: real_eth_usd(), wallet_eth: Some(0.0), wallet_weth: Some(0.0), ..Default::default() };
    raw.aero = AeroReads { wallet_aero: Some(1.359979), price_usd: Some(0.417608), price_note: None, unclaimed: Some(vec![]) };
    let m = compute_derived_metrics(&raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
    assert_eq!(m.global_status.shot_advice_action, "NOTHING TO TRACK YET (Empty Wallet)");
    assert_eq!(m.global_status.total_portfolio_usd, "0.57");
    let d = &m.global_status.shot_advice_details;
    assert!(d.ends_with("It holds 1.359979 AERO, counted in the total; AERO is not cash and is never deployed."), "{d}");
    // Nor is a wallet holding gas AND AERO (synthetic balances at the real prices). The
    // emptiness test must not subtract gas and AERO back out of the total: in f64,
    // (gas + aero) - gas - aero often leaves a positive residue, which sent four of
    // these six wallets to "HOLD & ACCRETE (No NOCK LP Tracked)".
    for (eth, weth, aero) in [
        (0.003, 0.0, 1.359979),
        (0.0031, 0.0007, 0.121026),
        (0.02, 0.001, 3.3),
        (0.0124567, 0.0, 7.77),
        (0.0123, 0.0, 1.359979366619354917),
        (0.0001, 0.0, 18.0),
    ] {
        let mut raw = RawOnChainData { spot_price_nock_usd: real_nock_usd(), live_eth_price_usd: 1811.33, wallet_eth: Some(eth), wallet_weth: Some(weth), ..Default::default() };
        raw.aero = AeroReads { wallet_aero: Some(aero), price_usd: Some(0.417608), price_note: None, unclaimed: Some(vec![]) };
        let with_aero = compute_derived_metrics(&raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        assert_eq!(with_aero.global_status.shot_advice_action, "NOTHING TO TRACK YET (Empty Wallet)", "eth {eth} weth {weth} aero {aero}");
        // AERO does not change which directive is chosen: the same wallet without it.
        raw.aero.wallet_aero = Some(0.0);
        let without = compute_derived_metrics(&raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        assert_eq!(without.global_status.shot_advice_action, with_aero.global_status.shot_advice_action);
        // Both are still counted in the total.
        let total: f64 = f(&with_aero.global_status.total_portfolio_usd);
        assert!((total - ((eth + weth) * 1811.33 + aero * 0.417608)).abs() < 0.011, "{total}");
    }
}

// ------------------------------------------------------------------
// A wallet with no Aero position, and a failed price read
// ------------------------------------------------------------------

#[test]
fn test_wallet_with_no_aero_position_and_no_aero_cannot_be_failed_by_the_price() {
    // A Uniswap-only book: no position earns AERO, the wallet holds none, and the price
    // read fails (the real stale pool configured as aero_price_pool).
    let mut uniswap = cl_config();
    uniswap.protocol = "uniswap-v3".to_string();
    let stale = aero_price_from_pool_reads(DEAD_PRICE_POOL, REAL_PRICE_POOL_TOKEN0, REAL_PRICE_POOL_TOKEN1, REAL_DEAD_POOL_SLOT0, REAL_DEAD_POOL_LIQUIDITY);
    let reads = aero_reads_from(&[uniswap], &HashMap::new(), REAL_AERO_BALANCE_WALLET, stale, &|g| panic!("no gauge is read: {g}"))
        .expect("the cycle does not fail");
    assert_eq!(reads.wallet_aero, Some(0.0));
    assert_eq!(reads.unclaimed, Some(vec![]));
    assert_eq!(reads.price_usd, None);
    assert!(reads.price_note.as_deref().unwrap().starts_with("AERO/USD price unavailable: Stale AERO/USD price"));

    // The same for a wallet with no positions at all (the from-zero newcomer).
    let mut raw = RawOnChainData {
        spot_price_nock_usd: real_nock_usd(),
        live_eth_price_usd: real_eth_usd(),
        live_gas_cost_usd: 0.0041,
        wallet_eth: Some(0.0),
        wallet_weth: Some(0.0),
        aero: reads,
        ..Default::default()
    };
    raw.wallet_balances.push(WalletBalance {
        wallet_address: WALLET.to_string(),
        token_symbol: "USDC".to_string(),
        token_address: USDC_BASE.to_string(),
        amount: "500.0000".to_string(),
        amount_usd: "500.00".to_string(),
        updated_at: "now".to_string(),
    });
    raw.wallet_balances.push(aero_wallet_balance(WALLET, 0.0, None, "now"));
    let snap = snapshot_of(raw, &OperatorProfile::default(), 0.0);
    let g = &snap.derived_metrics.global_status;
    // Holding none is a fact worth exactly $0; no price is needed to say so.
    assert_eq!((g.wallet_aero_amount.as_deref(), g.wallet_aero_usd.as_deref()), (Some("0.000000"), Some("0.00")));
    assert_eq!((g.unclaimed_aero_amount.as_deref(), g.unclaimed_aero_usd.as_deref()), (Some("0.000000"), Some("0.00")));
    // The price is not invented: null, with the reason.
    assert_eq!(g.aero_price_usd, None);
    assert!(g.aero_note.as_deref().unwrap().contains("Stale AERO/USD price"));
    assert_eq!(g.total_portfolio_usd, "500.00");
    assert_eq!(snap.wallet_balances.last().unwrap().amount_usd, "0.00");
    let text = format_portfolio_state_text(&serde_json::to_value(&snap).unwrap());
    assert!(text.contains("• Pillars: NOCK $0.00 · USDC $500.00 · ETH $0.00 · AERO $0.00\n"), "{text}");
    assert!(text.contains("• AERO held (wallet): 0.000000 AERO ($0.00), counted in the total; not cash, not deployable, no target\n"), "{text}");
    assert!(text.contains("• Unclaimed AERO: none: no staked Aero position is tracked\n"), "{text}");
    assert!(text.contains("• AERO/USD: not reported\n"), "{text}");
    assert!(text.contains("• AERO not read: AERO/USD price unavailable: Stale AERO/USD price"), "{text}");
    // Its directive is the one it had before #93.
    assert_eq!(g.shot_advice_action, "BUILD FROM ZERO (No Buy Plan Configured)");
}

#[test]
fn test_failed_price_read_leaves_unclaimed_aero_stated_and_unpriced() {
    // The wallet holds no AERO and earns some; the price read fails. Amounts stay, USD
    // is absent with the reason, and nothing reads as $0.
    let failed = || -> Result<f64> { Err(anyhow::anyhow!("AERO price pool 0xd0acf293547457fb4fe9ee32ca93e89497626816 slot0() read failed")) };
    let raw = real_cycle_with(&|cfgs, facts| aero_reads_from(cfgs, facts, REAL_AERO_BALANCE_WALLET, failed(), &real_earned).unwrap());
    let priced = snapshot_of(real_cycle(), &OperatorProfile::default(), 0.0);
    let snap = snapshot_of(raw, &OperatorProfile::default(), 0.0);
    let g = &snap.derived_metrics.global_status;
    assert_eq!(g.aero_price_usd, None);
    assert_eq!(g.unclaimed_aero_amount.as_deref(), Some("12.043104"));
    assert_eq!(g.unclaimed_aero_usd, None);
    assert_eq!(g.wallet_aero_usd.as_deref(), Some("0.00"));
    assert_eq!(g.aero_note.as_deref(), Some("AERO/USD price unavailable: AERO price pool 0xd0acf293547457fb4fe9ee32ca93e89497626816 slot0() read failed"));
    assert_eq!(snap.positions[1].unclaimed_aero.as_deref(), Some("11.922078"));
    assert_eq!(snap.positions[1].unclaimed_aero_usd, None);
    assert!(snap.contract_fees[1].amount.ends_with("; unclaimed AERO 11.922078 (USD value unavailable)"), "{}", snap.contract_fees[1].amount);
    // The total is the one a priced cycle reports: unclaimed AERO was never in it.
    assert_eq!(g.total_portfolio_usd, priced.derived_metrics.global_status.total_portfolio_usd);
    let text = format_portfolio_state_text(&serde_json::to_value(&snap).unwrap());
    assert!(text.contains("• Unclaimed AERO: 12.043104 AERO (USD value not reported) in tracked gauges, not counted in the total\n"), "{text}");
    assert!(text.contains("   - Aero USDC/NOCK: 0.121026 AERO (USD value not reported), collectible from gauge 0xc82782ee66d82c4d19db0429652091a085083526 in the Aero app\n"), "{text}");
    assert!(text.contains("• AERO/USD: not reported\n"), "{text}");

    // The same failure while the wallet HOLDS AERO fails the cycle (synthetic balance).
    let err = aero_reads_from(&[basic_config(Some(BASIC_GAUGE))], &HashMap::new(), REAL_EARNED_BEFORE_CLAIM, failed(), &real_earned)
        .unwrap_err()
        .to_string();
    assert!(err.contains("the wallet holds 1.359979 AERO") && err.contains("the cycle fails"), "{err}");
    // And an unreadable balance fails it before anything else (synthetic short answer).
    assert!(aero_reads_from(&[], &HashMap::new(), "0x", real_aero_price(), &real_earned).is_err());
}

// ------------------------------------------------------------------
// Older snapshots and both MCP texts
// ------------------------------------------------------------------

#[test]
fn test_older_snapshot_without_aero_fields_reads_not_reported_never_zero() {
    // A stored 2.4.0 payload has none of the fields: it still parses, to None.
    let mut data = serde_json::to_value(&snapshot_of(real_cycle(), &OperatorProfile::default(), 0.0)).unwrap();
    {
        let g = data["derived_metrics"]["global_status"].as_object_mut().unwrap();
        for k in ["aero_price_usd", "wallet_aero_amount", "wallet_aero_usd", "unclaimed_aero_amount", "unclaimed_aero_usd", "aero_note"] {
            assert!(g.remove(k).is_some(), "{k} is emitted");
        }
    }
    for p in data["positions"].as_array_mut().unwrap() {
        for k in ["unclaimed_aero", "unclaimed_aero_usd"] {
            assert!(p.as_object_mut().unwrap().remove(k).is_some(), "{k} is emitted");
        }
    }
    data["wallet_balances"].as_array_mut().unwrap().pop();
    let parsed: SnapshotData = serde_json::from_value(data.clone()).expect("an older payload still parses");
    let g = &parsed.derived_metrics.global_status;
    assert_eq!((&g.wallet_aero_amount, &g.wallet_aero_usd, &g.unclaimed_aero_amount, &g.aero_price_usd), (&None, &None, &None, &None));
    assert_eq!(parsed.positions[0].unclaimed_aero, None);

    let text = format_portfolio_state_text(&data);
    assert!(text.contains(" · AERO not reported\n"), "{text}");
    assert!(text.contains("• AERO held (wallet): not reported\n"), "{text}");
    assert!(text.contains("• Unclaimed AERO: not reported\n"), "{text}");
    assert!(text.contains("• AERO/USD: not reported\n"), "{text}");
    let aero_part = aero_lines(&data);
    assert!(!aero_part.contains('0'), "no figure is invented: {aero_part}");
    // A position that has no AERO field prints no AERO row.
    assert!(!aero_part.contains("   - "), "{aero_part}");
}

#[test]
fn test_mcp_portfolio_text_states_the_fourth_pillar() {
    let claimed = REAL_EARNED_BEFORE_CLAIM;
    let holding = real_cycle_with(&|cfgs, facts| aero_reads_from(cfgs, facts, claimed, real_aero_price(), &real_earned).unwrap());
    let text = format_portfolio_state_text(&serde_json::to_value(&snapshot_of(holding, &OperatorProfile::default(), 0.0)).unwrap());
    assert!(text.contains(" · AERO $0.57\n"), "{text}");
    assert!(text.contains("• AERO held (wallet): 1.359979 AERO ($0.57), counted in the total; not cash, not deployable, no target\n"), "{text}");
    assert!(text.contains("• Unclaimed AERO: 12.043104 AERO ($5.03) in tracked gauges, not counted in the total\n   - Aero USDC/NOCK: 0.121026 AERO ($0.05), collectible from gauge 0xc82782ee66d82c4d19db0429652091a085083526 in the Aero app\n   - Aero CL WETH/NOCK #2914490: 11.922078 AERO ($4.98), collectible from gauge 0x3bf70c880adc8370917994ab0df68e763b7a60fd in the Aero app\n"), "{text}");
    assert!(text.contains("• AERO/USD: $0.417608\n"), "{text}");
    assert!(!text.contains("AERO not read"), "{text}");
    // The lines sit between gas and the high-water mark.
    let (gas, aero, hwm) = (text.find("• Gas held").unwrap(), text.find("• AERO held").unwrap(), text.find("• High-Water Mark").unwrap());
    assert!(gas < aero && aero < hwm);
}

// ------------------------------------------------------------------
// The harvest ledger books claimed AERO
// ------------------------------------------------------------------

#[test]
fn test_harvest_ledger_books_aero_from_a_real_gauge_claim() {
    // One block before the claim, earned(account) was exactly what the claim minted.
    let earned = decode_amount_word(REAL_EARNED_BEFORE_CLAIM, "earned()").unwrap();
    let transfers = parse_harvest_transfers(&real_claim_receipt(), CLAIMER);
    assert_eq!(transfers.len(), 1, "only the AERO minted to the wallet is booked: {transfers:?}");
    let t = &transfers[0];
    assert_eq!(t.token_symbol, "AERO");
    assert!(t.token_address.eq_ignore_ascii_case(AERO_BASE));
    assert_eq!(t.decimals, 18);
    assert_eq!(t.amount_raw, earned.to_string());
    assert_eq!(t.amount_raw, "1359979366619354917");
    assert_eq!(t.amount_human, "1.359979366619354917");
    // A gauge claim is a mint: the sender is the zero address.
    assert_eq!(t.from_address, "0x0000000000000000000000000000000000000000");
    assert_eq!(t.recipient, CLAIMER);
    // The tracked wallet was paid nothing by that transaction.
    assert!(parse_harvest_transfers(&real_claim_receipt(), WALLET).is_empty());
    // The further 5% went to another account and is booked only for that account.
    let third = parse_harvest_transfers(&real_claim_receipt(), "0x9beb95cced809c0667bbb6577ee4066ac5cc1156");
    assert_eq!(third.iter().map(|t| t.amount_raw.as_str()).collect::<Vec<_>>(), ["67998968330967745"]);
    // The older token with the same symbol is not AERO here (synthetic log).
    let mut legacy = real_claim_receipt();
    legacy["logs"][2]["address"] = json!(LEGACY_AERODROME_AERO);
    assert!(parse_harvest_transfers(&legacy, CLAIMER).is_empty());
}

#[test]
fn test_claimed_aero_is_valued_at_the_claims_block_or_not_at_all() {
    let t = &parse_harvest_transfers(&real_claim_receipt(), CLAIMER)[0];
    // The price pool read at the claim's block (57635701), as record-harvest reads it.
    let at_claim = aero_price_from_pool_reads(
        DEFAULT_AERO_PRICE_POOL,
        REAL_PRICE_POOL_TOKEN0,
        REAL_PRICE_POOL_TOKEN1,
        REAL_PRICE_POOL_SLOT0_AT_CLAIM,
        REAL_PRICE_POOL_LIQUIDITY_AT_CLAIM,
    )
    .unwrap();
    let priced = (at_claim, format!("aero_price_pool {DEFAULT_AERO_PRICE_POOL} slot0 at block 57635701"));
    let (unit, source) = harvest_unit_price_with_aero("AERO", None, None, Some(&priced)).unwrap();
    assert_eq!(source, "aero_price_pool 0xd0acf293547457fb4fe9ee32ca93e89497626816 slot0 at block 57635701");
    assert_eq!(harvest_usd_value(&t.amount_human, Some(unit)).as_deref(), Some("0.5968"));
    // Not at today's price ($0.4176 at block 58077673 would give $0.5679).
    let today = harvest_usd_value(&t.amount_human, Some(real_aero_price().unwrap())).unwrap();
    assert_eq!(today, "0.5679");
    // No price at that block: no USD value, never a substitute (ETH's or NOCK's price is
    // not borrowed, and neither is $0).
    let nock = (0.0113, "market_price_history".to_string());
    let eth = (1811.33, "eth_price_pool".to_string());
    assert_eq!(harvest_unit_price_with_aero("AERO", Some(&nock), Some(&eth), None), None);
    assert_eq!(harvest_usd_value(&t.amount_human, None), None);
    // The other tokens are priced exactly as before.
    assert_eq!(harvest_unit_price_with_aero("USDC", None, None, Some(&priced)), Some((1.0, "USDC at $1")));
    assert_eq!(harvest_unit_price_with_aero("NOCK", Some(&nock), Some(&eth), Some(&priced)).map(|p| p.0), Some(0.0113));
    assert_eq!(harvest_unit_price_with_aero("WETH", Some(&nock), Some(&eth), Some(&priced)).map(|p| p.0), Some(1811.33));
    assert_eq!(harvest_unit_price_with_aero("ETH", None, None, Some(&priced)), None);
    // A stale pool at the claim's block gives no price, so no USD value.
    let stale = aero_price_from_pool_reads(DEAD_PRICE_POOL, REAL_PRICE_POOL_TOKEN0, REAL_PRICE_POOL_TOKEN1, REAL_DEAD_POOL_SLOT0, REAL_DEAD_POOL_LIQUIDITY);
    assert!(stale.is_err());
}

#[test]
fn test_koinly_export_describes_aero_as_a_reward_claim() {
    let row = |symbol: &str, amount: &str, usd: Option<&str>| HarvestExportRow {
        platform: "aerodrome".to_string(),
        tx_hash: "0x189f6ddfbb0e278ed37b2049d8eb5deeeb78f618a1b20c213c47f42511f9b8f8".to_string(),
        token_symbol: symbol.to_string(),
        amount_human: amount.to_string(),
        usd_value: usd.map(|u| u.to_string()),
        confirmed_at: "2026-11-15T04:38:35+00:00".to_string(),
    };
    let csv = format_koinly_csv(&[row("AERO", "1.359979366619354917", Some("0.5968")), row("AERO", "2.5", None), row("USDC", "0.12", Some("0.1200"))]);
    let lines: Vec<&str> = csv.lines().collect();
    assert_eq!(
        lines[1],
        "2026-11-15T04:38:35+00:00, , , 1.359979366619354917, AERO, , , 0.5968, USD, income, \"NockNock AERO reward claim (aerodrome)\", 0x189f6ddfbb0e278ed37b2049d8eb5deeeb78f618a1b20c213c47f42511f9b8f8"
    );
    // An unpriced AERO row has no net worth, not $0.
    assert!(lines[2].contains("2.5, AERO, , , , , income, \"NockNock AERO reward claim (aerodrome)\""), "{}", lines[2]);
    // Fees keep their wording.
    assert!(lines[3].contains("\"NockNock fee harvest (aerodrome)\""), "{}", lines[3]);
    assert_eq!(harvest_row_description("AERO", "aerodrome-cl"), "NockNock AERO reward claim (aerodrome-cl)");
    assert_eq!(harvest_row_description("WETH", "uniswap-v3"), "NockNock fee harvest (uniswap-v3)");
}

// ------------------------------------------------------------------
// Config
// ------------------------------------------------------------------

#[test]
fn test_aero_price_pool_is_configurable_with_the_verified_default() {
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/config.yaml.example");
    let cfg = load_app_config(example, Some("/nonexistent/config.yaml"), false).unwrap();
    assert_eq!(cfg.aero_price_pool.as_deref(), Some(DEFAULT_AERO_PRICE_POOL));
    assert!(std::fs::read_to_string(example).unwrap().contains("aero_price_pool: \"0xd0acf293547457fb4fe9ee32ca93e89497626816\""));
    // A config that does not mention it gets the same default; one that does is obeyed.
    let dir = std::env::temp_dir().join(format!("nocknock-aero-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let bare = dir.join("bare.yaml");
    std::fs::write(&bare, "tracked_wallets: [\"0xeE114cC97eC5adeB34bA5645e315B497D8487a45\"]\n").unwrap();
    let cfg = load_app_config("/nonexistent/example", Some(bare.to_str().unwrap()), false).unwrap();
    assert_eq!(cfg.aero_price_pool.as_deref(), Some(DEFAULT_AERO_PRICE_POOL));
    let own = dir.join("own.yaml");
    std::fs::write(&own, "aero_price_pool: \"0x422df9aa896e883c9cdcbad309c0785b7364a1cf\"\n").unwrap();
    let cfg = load_app_config(example, Some(own.to_str().unwrap()), false).unwrap();
    assert_eq!(cfg.aero_price_pool.as_deref(), Some("0x422df9aa896e883c9cdcbad309c0785b7364a1cf"));
    std::fs::remove_dir_all(&dir).ok();
    // The default is the pool the price fixtures were read from.
    assert!(DEFAULT_AERO_PRICE_POOL.starts_with("0xd0acf293") && DEFAULT_AERO_PRICE_POOL.ends_with("6816"));
}

#[test]
fn test_fetch_raw_data_reads_aero_before_it_builds_the_fee_rows() {
    // The published positions and the fee rows are built from the same call, with the
    // AERO reads passed in; the cycle carries them into the metrics.
    let onchain = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/onchain.rs")).unwrap();
    let reads = onchain.find("fetch_aero_reads(").expect("the cycle reads AERO");
    let rows = onchain.find("snapshot_positions_from(&raw_positions,").expect("fee rows are built from the stamped positions");
    assert!(reads < rows);
    assert!(onchain.contains("&position_facts, &aero);"));
    assert!(onchain.contains("aero_wallet_balance(wallet_addr,"));
    // run_ingestion_cycle publishes through snapshot_positions, which passes raw.aero.
    let raw = real_cycle();
    let published = snapshot_positions(&raw);
    assert_eq!(published[0].unclaimed_aero.as_deref(), Some("0.121026"));
    assert_eq!(published[1].unclaimed_aero.as_deref(), Some("11.922078"));
}

// ------------------------------------------------------------------
// Golden snapshot for the fourth pillar (its own file)
// ------------------------------------------------------------------
// Regenerate only for an intended change:
//   NOCKNOCK_BLESS_GOLDEN=1 cargo test test_golden_snapshot

#[test]
fn test_golden_snapshot_aero_rewards_is_byte_identical() {
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
    // 1. The real cycle: both positions staked, AERO earned and not claimed.
    let unclaimed = snapshot_of(real_cycle(), &example_profile(), 0.0);
    // 2. The same cycle with the amount of the real claim in the wallet (synthetic balance).
    let claimed = REAL_EARNED_BEFORE_CLAIM;
    let holding = real_cycle_with(&|cfgs, facts| aero_reads_from(cfgs, facts, claimed, real_aero_price(), &real_earned).unwrap());
    let held = snapshot_of(holding, &example_profile(), 0.0);
    let doc = json!({
        "aero_unclaimed_block_58077673": case(&unclaimed),
        "aero_held_block_58077673": case(&held),
    });
    let actual = format!("{}\n", serde_json::to_string_pretty(&doc).unwrap());
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/golden_snapshot_aero_rewards.json");
    if std::env::var("NOCKNOCK_BLESS_GOLDEN").is_ok() {
        std::fs::write(path, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(path).expect("testdata/golden_snapshot_aero_rewards.json is checked in");
    assert!(
        actual == expected,
        "snapshot bytes changed; compare {path} with a fresh NOCKNOCK_BLESS_GOLDEN=1 run and `git diff` it"
    );
}

// ------------------------------------------------------------------
// Review fixes: a gauge that pays another token, several AERO mints in one
// transaction, a fee row that carries a fee action, a row priced on a retry
// ------------------------------------------------------------------

#[test]
fn test_a_gauge_that_pays_another_token_is_not_read_as_aero() {
    // The Aero app's gauges have no rewardToken(): the real revert means "proceed".
    for gauge in [BASIC_GAUGE, CL_GAUGE] {
        assert_eq!(gauge_pays_aero(gauge, &real_reward_token(gauge)), Ok(()));
    }
    // A gauge that names AERO itself is fine too (synthetic: no such gauge was found).
    let names_aero = format!("{:0>64}", AERO_BASE.trim_start_matches("0x").to_lowercase());
    assert_eq!(gauge_pays_aero("0xgauge", &ok_hex(&names_aero)), Ok(()));
    // Real: the legacy Aerodrome gauges name the older token with the same symbol.
    assert_eq!(address_from_word(REAL_LEGACY_REWARD_TOKEN).unwrap(), LEGACY_AERODROME_AERO.to_lowercase());
    for gauge in [LEGACY_BASIC_GAUGE, LEGACY_CL_GAUGE] {
        let refused = gauge_pays_aero(gauge, &real_reward_token(gauge)).unwrap_err();
        assert_eq!(
            refused,
            format!("gauge {gauge} pays 0x940181a94a35a4569e4529a3cdfb74e38fd98631 (its rewardToken()), not AERO 0x3f5bf3edac59d650d8fdf9657a71f2f904a3a4ee")
        );
    }
    // Not known is not read either: a failed read (synthetic) or a non-address answer.
    let failed = gauge_pays_aero(BASIC_GAUGE, &Err(anyhow::anyhow!("HTTP 429 Too Many Requests"))).unwrap_err();
    assert!(failed.contains("rewardToken() read failed: HTTP 429"), "{failed}");
    assert!(gauge_pays_aero(BASIC_GAUGE, &ok_hex("0x")).unwrap_err().contains("rewardToken() is not an address"));

    // A legacy-pool position with its legacy gauge configured: the gauge answers
    // earned(address) (really: one word, 0 for the wallet), and that answer is NOT
    // published as AERO. No amount, no USD, the reason, and no total.
    assert!(matches!(real_earned(LEGACY_BASIC_GAUGE), Ok(Some(_))), "the legacy gauge does answer earned(address)");
    let reads = aero_reads_from(
        &[basic_config(Some(LEGACY_BASIC_GAUGE)), cl_config()],
        &real_cycle().position_facts,
        REAL_AERO_BALANCE_WALLET,
        real_aero_price(),
        &real_earned,
    )
    .unwrap();
    let entries = reads.unclaimed.as_ref().unwrap();
    assert_eq!(entries[0].amount, None);
    assert_eq!(entries[0].gauge.as_deref(), Some(LEGACY_BASIC_GAUGE));
    assert!(entries[0].note.as_deref().unwrap().contains("pays 0x940181a9"), "{:?}", entries[0].note);
    assert_eq!(entries[1].amount.map(|a| format!("{a:.6}")).as_deref(), Some("11.922078"));
    let pillar = aero_pillar(&reads);
    assert_eq!((pillar.unclaimed_amount, pillar.unclaimed_usd), (None, None));
    let (amount, usd, note) = aero_position_fields(&entries[0], reads.price_usd);
    assert_eq!((amount, usd), (None, None));
    assert_eq!(
        aero_statement(None, None, note.as_deref()).as_deref(),
        Some("unclaimed AERO not read (gauge 0x4cbb6188c6101cacd4eb1126cc14882bd93dc3db pays 0x940181a94a35a4569e4529a3cdfb74e38fd98631 (its rewardToken()), not AERO 0x3f5bf3edac59d650d8fdf9657a71f2f904a3a4ee)")
    );
    // The selector is the one verified on-chain.
    assert_eq!(GAUGE_REWARD_TOKEN, "0xf7c618c1");
}

/// Real: tx 0xe32b6e47…3a67 (block 58077008, status 1) claims from seven CL gauges for
/// 0x091607c3…25f6 in one transaction. These are its fourteen AERO Transfer logs (seven
/// mints to the claimer, each followed by the further 5% to 0x9beb95cc…1156) and two of
/// its seven USDC Transfers to the claimer, which come from two different pools. The
/// other 240 logs are left out.
const MULTI_CLAIMER: &str = "0x091607c3355f5875400851efc6347a839d3b25f6";
fn real_multi_gauge_claim_receipt() -> serde_json::Value {
    let zero = "0x0000000000000000000000000000000000000000000000000000000000000000";
    let claimer = "0x000000000000000000000000091607c3355f5875400851efc6347a839d3b25f6";
    let fee_taker = "0x0000000000000000000000009beb95cced809c0667bbb6577ee4066ac5cc1156";
    let aero = |index: &str, to: &str, amount: &str| {
        json!({ "address": "0x3f5bf3edac59d650d8fdf9657a71f2f904a3a4ee", "logIndex": index,
                "topics": [TRANSFER_EVENT_TOPIC, zero, to],
                "data": format!("0x{:0>64}", amount) })
    };
    let usdc = |from: &str, amount: &str| {
        json!({ "address": "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913",
                "topics": [TRANSFER_EVENT_TOPIC, format!("0x000000000000000000000000{from}"), claimer],
                "data": format!("0x{:0>64}", amount) })
    };
    json!({
        "transactionHash": "0xe32b6e476ceff9040c1e22b5a23e9ba2a8c4e5cff8221939e871e9ec84bb3a67",
        "blockNumber": "0x3762f50",
        "from": "0x0cf406ec4599b8958696685555a149fa518f382b",
        "to": MULTI_CLAIMER,
        "status": "0x1",
        "logs": [
            aero("0x1d", claimer, "068f2d2db7c1f478"), aero("0x1e", fee_taker, "53f5757c634c39"),
            usdc("f668e21993f0acd7ab00bb1cfbaed0df28a3c8ec", "0d8bfc3d"),
            aero("0x41", claimer, "1647bd2b0076db6c"), aero("0x42", fee_taker, "011d2fdbc005f15f"),
            usdc("2836e062a3fe75943a7f4a552e2365b98e414e75", "1c459d9a"),
            aero("0x65", claimer, "14e96780a34a98e1"), aero("0x66", fee_taker, "010bab933b5d5471"),
            aero("0x8a", claimer, "07456a082ea02904"), aero("0x8b", fee_taker, "5d121a0254ced9"),
            aero("0xaf", claimer, "035d1a8fcd7dd542"), aero("0xb0", fee_taker, "2b0e20ca464aa9"),
            aero("0xd4", claimer, "11a215b153672838"), aero("0xd5", fee_taker, "e1b448ddc52869"),
            aero("0xf9", claimer, "0626cf4c56ccda01"), aero("0xfa", fee_taker, "4ebd909df0a480")
        ]
    })
}

#[test]
fn test_several_aero_mints_in_one_transaction_are_booked_as_one_summed_row() {
    // The receipt pays the claimer AERO seven times, once per gauge.
    let parsed = parse_harvest_transfers(&real_multi_gauge_claim_receipt(), MULTI_CLAIMER);
    let aero_logs: Vec<&ParsedHarvestTransfer> = parsed.iter().filter(|t| t.token_symbol == "AERO").collect();
    assert_eq!(aero_logs.len(), 7);
    assert_eq!(aero_logs[0].amount_raw, "472646160276911224");
    // The ledger's key is (tx_hash, token_address): unmerged, the insert of the second
    // AERO row would conflict with the first and 5.59 of 6.07 AERO would be dropped.
    let merged = merge_harvest_transfers(parsed).unwrap();
    assert_eq!(merged.iter().map(|t| t.token_symbol.as_str()).collect::<Vec<_>>(), ["AERO", "USDC"]);
    let mut keys: Vec<String> = merged.iter().map(|t| t.token_address.to_lowercase()).collect();
    keys.dedup();
    assert_eq!(keys.len(), merged.len(), "one row per token address");
    let aero = &merged[0];
    // 472646160276911224 + 1605459784569903980 + 1506849352525191393 + 523941515033061636
    // + 242379159878096194 + 1270601896224565304 + 443269540123302401
    assert_eq!(aero.amount_raw, "6065147408631032132");
    assert_eq!(aero.amount_human, "6.065147408631032132");
    assert_eq!(aero.decimals, 18);
    assert_eq!(aero.from_address, "0x0000000000000000000000000000000000000000");
    assert_eq!(aero.recipient, MULTI_CLAIMER);
    // The 5% minted to the other account is not the claimer's and is not in the sum.
    let fee_taker = merge_harvest_transfers(parse_harvest_transfers(&real_multi_gauge_claim_receipt(), "0x9beb95cced809c0667bbb6577ee4066ac5cc1156")).unwrap();
    assert_eq!(fee_taker.len(), 1);
    assert_eq!(fee_taker[0].amount_raw, "303257370431551604");
    // The same rule for every token: two USDC payments from two pools are one row of
    // their sum, with the first sender kept.
    let usdc = &merged[1];
    assert_eq!(usdc.amount_raw, (0x0d8bfc3du128 + 0x1c459d9a).to_string());
    assert_eq!(usdc.amount_human, "701.602263");
    assert_eq!(usdc.from_address, "0xf668e21993f0acd7ab00bb1cfbaed0df28a3c8ec");
    // The summed row is valued once, at the claim block's price (synthetic price).
    assert_eq!(harvest_usd_value(&aero.amount_human, Some(0.4176)).as_deref(), Some("2.5328"));

    // The operator's case: both gauges claimed in one transaction (synthetic receipt,
    // the two real `earned` amounts as two mints).
    let zero = "0x0000000000000000000000000000000000000000000000000000000000000000";
    let wallet_word = format!("0x000000000000000000000000{}", WALLET.trim_start_matches("0x").to_lowercase());
    let both = json!({ "status": "0x1", "from": WALLET, "logs": [
        { "address": AERO_BASE, "topics": [TRANSFER_EVENT_TOPIC, zero, wallet_word], "data": REAL_BASIC_EARNED_WALLET },
        { "address": AERO_BASE, "topics": [TRANSFER_EVENT_TOPIC, zero, wallet_word], "data": REAL_CL_EARNED_WALLET }
    ]});
    let rows = merge_harvest_transfers(parse_harvest_transfers(&both, WALLET)).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].amount_raw, (0x01adf89b6483d30du128 + 0xa573ba928fdaf5d2).to_string());
    assert_eq!(rows[0].amount_human, "12.043103888520169695");

    // A single-transfer receipt is unchanged, and nothing in is nothing out.
    let single = parse_harvest_transfers(&real_claim_receipt(), CLAIMER);
    assert_eq!(merge_harvest_transfers(single.clone()).unwrap(), single);
    assert!(merge_harvest_transfers(vec![]).unwrap().is_empty());
    // An amount that cannot be summed is an error, never a truncated row (synthetic).
    let mut huge = single.clone();
    huge[0].amount_raw = u128::MAX.to_string();
    huge.push(single[0].clone());
    assert!(merge_harvest_transfers(huge).unwrap_err().to_string().contains("overflow"));
    let mut garbled = single;
    garbled[0].amount_raw = "1.5".to_string();
    assert!(merge_harvest_transfers(garbled).is_err());
}

#[test]
fn test_a_fee_row_with_a_fee_action_carries_no_aero_statement() {
    let aero = || Some("unclaimed AERO 0.121026 ($0.05)".to_string());
    // Not LP-claimable: the reason, then the amount.
    assert_eq!(
        fee_row_note(false, None, aero()).as_deref(),
        Some("not LP-claimable (claimFees() reverts); unclaimed AERO 0.121026 ($0.05)")
    );
    assert_eq!(
        fee_row_note(false, Some(AERO_CL_STAKED_FEES_NOTE.to_string()), aero()).as_deref(),
        Some("not LP-claimable (staked: swap fees on staked liquidity go to the gauge); unclaimed AERO 0.121026 ($0.05)")
    );
    // LP-claimable: the row's action is about LP fees, so it says nothing about AERO.
    assert_eq!(fee_row_note(true, None, aero()), None);
    // No AERO statement: exactly the note the row had before #93.
    assert_eq!(fee_row_note(false, Some("x".to_string()), None).as_deref(), Some("x"));
    assert_eq!(fee_row_note(true, None, None), None);

    // End to end (synthetic: the real basic position, with claimFees() made to succeed
    // and $8 of USDC fees). Its velocity row says claim_now about the LP fees and does
    // not mention AERO; the AERO amount stays on the position and in the pillar lines.
    let mut raw = real_cycle();
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
        8_000_000,
        0,
    )
    .unwrap();
    let snap = snapshot_of(raw, &OperatorProfile::default(), 0.0);
    let rows = &snap.derived_metrics.velocity_gas_sandbox.contracts;
    assert_eq!(rows[0].recommended_action, "claim_now");
    assert_eq!(rows[0].note, None, "a row with a fee action must not sit beside an AERO amount");
    assert_eq!(rows[1].recommended_action, "not_claimable");
    assert!(rows[1].note.as_deref().unwrap().ends_with("; unclaimed AERO 11.922078 ($4.98)"), "{:?}", rows[1].note);
    assert!(!snap.contract_fees[0].amount.contains("AERO"), "{}", snap.contract_fees[0].amount);
    assert_eq!(snap.positions[0].unclaimed_aero.as_deref(), Some("0.121026"));
    let g = &snap.derived_metrics.global_status;
    assert_eq!(g.unclaimed_aero_amount.as_deref(), Some("12.043104"));
    // Whatever the directive is, it is about LP fees and does not name AERO.
    assert!(!g.shot_advice_action.contains("AERO") && !g.shot_advice_details.contains("AERO"), "{}", g.shot_advice_details);
    // No row anywhere pairs a fee action with AERO.
    for row in rows {
        if row.recommended_action != "not_claimable" {
            assert!(!row.note.as_deref().unwrap_or("").contains("AERO"), "{row:?}");
        }
    }
}

#[test]
fn test_a_row_recorded_without_a_price_can_be_priced_on_a_retry_and_never_repriced() {
    // The statement only fills a missing value, on the same transaction, token and
    // amount. (It is not exercised against a database here.)
    let sql = FILL_HARVEST_USD_SQL;
    assert!(sql.starts_with("UPDATE harvest_events SET usd_value = $1, price_usd = $2, price_source = $3 WHERE "), "{sql}");
    for guard in ["tx_hash = $4", "token_address = $5", "amount_raw = $6", "usd_value IS NULL"] {
        assert!(sql.contains(guard), "missing guard '{guard}' in: {sql}");
    }
    assert!(!sql.to_lowercase().contains("amount_human ="), "the booked amount is never rewritten");
}
