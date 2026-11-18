use crate::*;

// ============================================================
// Uniswap v3/v4 per-position reads (v2.0.0)
// ============================================================
// Addresses and selectors verified live on Base mainnet 2026-10-24 (Phase-1
// discovery; see ONCHAIN_INTEGRATION.md §5). Selectors were recomputed from
// canonical signatures and cross-checked against the responses they produce.

pub(crate) const UNISWAP_V4_STATE_VIEW_BASE: &str = "0xa3c0c9b65bad0b08107aa264b0f3db444b867a71";
/// Canonical Uniswap v3 WETH/USDC 0.05% pool on Base — the ETH/USD spot source.
/// Resolved via factory getPool(WETH, USDC, 500) and price-verified ($2,373 at
/// discovery time); replaces the liquidated Aerodrome vAMM-WETH/USDC feed.
pub(crate) const DEFAULT_ETH_PRICE_POOL: &str = "0xd0b53d9277642d899df5c87a3966a349a798f224";

pub(crate) const V4_GET_POSITION_LIQUIDITY: &str = "0x1efeed33"; // getPositionLiquidity(uint256)
pub(crate) const V4_POOL_KEYS: &str = "0x86b6be7d"; // poolKeys(bytes25)
// (getPoolAndPositionInfo(uint256) = 0x7ba03aad exists on the POSM; the pipeline
// reads ticks/liquidity via StateView + config instead.)
pub(crate) const V4_STATE_VIEW_GET_SLOT0: &str = "0xc815641c"; // getSlot0(bytes32)
pub(crate) const V4_STATE_VIEW_GET_LIQUIDITY: &str = "0xfa6793d5"; // getLiquidity(bytes32)
pub(crate) const V3_LIQUIDITY: &str = "0x1a686502"; // liquidity()
pub(crate) const V4_STATE_VIEW_GET_POSITION_INFO: &str = "0xdacf1d2f"; // getPositionInfo(bytes32,address,int24,int24,bytes32)
pub(crate) const V4_STATE_VIEW_GET_FEE_GROWTH_INSIDE: &str = "0x53e9c1fb"; // getFeeGrowthInside(bytes32,int24,int24)
pub(crate) const V3_POSITIONS: &str = "0x99fbab88"; // positions(uint256)
pub(crate) const V3_SLOT0: &str = "0x3850c7bd"; // slot0()
pub(crate) const V3_FEE_GROWTH_GLOBAL0: &str = "0xf3058399"; // feeGrowthGlobal0X128()
pub(crate) const V3_FEE_GROWTH_GLOBAL1: &str = "0x46141319"; // feeGrowthGlobal1X128()
pub(crate) const V3_TICKS: &str = "0xf30dba93"; // ticks(int24)

pub(crate) const WETH_BASE: &str = "0x4200000000000000000000000000000000000006";
pub(crate) const NATIVE_ETH: &str = "0x0000000000000000000000000000000000000000";

/// Verified token registry for position legs. Unknown tokens fail explicitly —
/// a position leg we cannot symbolize/decimalize is never emitted with guesses.
pub(crate) fn token_info(address: &str) -> Result<PositionTokenInfo> {
    let a = address.trim_start_matches("0x").to_lowercase();
    match a.as_str() {
        "9b5e262cf9bb04869ab40b19af91d2dc85761722" => Ok(PositionTokenInfo {
            address: NOCK_BASE.to_string(),
            symbol: "NOCK".to_string(),
            decimals: 16,
        }),
        "833589fcd6edb6e08f4c7c32d4f71b54bda02913" => Ok(PositionTokenInfo {
            address: USDC_BASE.to_string(),
            symbol: "USDC".to_string(),
            decimals: 6,
        }),
        "4200000000000000000000000000000000000006" => Ok(PositionTokenInfo {
            address: WETH_BASE.to_string(),
            symbol: "WETH".to_string(),
            decimals: 18,
        }),
        "0000000000000000000000000000000000000000" => Ok(PositionTokenInfo {
            address: NATIVE_ETH.to_string(),
            symbol: "ETH".to_string(),
            decimals: 18,
        }),
        other => Err(anyhow::anyhow!(
            "Unregistered token 0x{} in a tracked position; add it to the token registry",
            other
        )),
    }
}

pub(crate) fn is_nock_token(t: &PositionTokenInfo) -> bool {
    t.symbol == "NOCK"
}

/// A position with a NOCK leg (token0 or token1 is NOCK, by the same symbol rule as
/// is_nock_token). Only these are NOCK LP: the rebuild target and templates, the NOCK LP
/// deviation row, the engine label, plan headroom, positions_nock and the brackets the
/// NOCK chart draws. A position with no NOCK leg (a gas-side WETH/USDC range, #151) is
/// measured, never steered: valuation and the totals still count it, because it is held.
pub(crate) fn is_nock_pair(p: &RawPosition) -> bool {
    is_nock_token(&p.token0) || is_nock_token(&p.token1)
}

pub(crate) fn is_usdc_token(t: &PositionTokenInfo) -> bool {
    t.symbol == "USDC"
}

pub(crate) fn is_eth_token(t: &PositionTokenInfo) -> bool {
    t.symbol == "WETH" || t.symbol == "ETH"
}

pub(crate) const USDC_BASE: &str = "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913";
pub(crate) const NOCK_BASE: &str = "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722";
