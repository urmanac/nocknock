use crate::*;

/// Adaptive-precision price formatting: 5 decimals for readable values, 8 for
/// sub-cent ones (e.g. ETH-per-NOCK ≈ 0.0000081) so brackets never collapse to
/// "0.00001".
pub(crate) fn fmt_price(v: f64) -> String {
    if v != 0.0 && v.abs() < 0.001 {
        format!("{:.8}", v)
    } else {
        format!("{:.5}", v)
    }
}

/// NOCK/USD spot from the designated price-source position. The pool must quote
/// NOCK against USDC (either ordering); the conversion follows the pool's decimals.
pub(crate) fn nock_spot_from_positions(positions: &[RawPosition]) -> Option<f64> {
    let src = positions.iter().find(|p| p.is_price_source)?;
    let raw = sqrt_price_x96_to_raw(src.sqrt_price_x96); // token1_raw per token0_raw
    let price = if is_usdc_token(&src.token0) && is_nock_token(&src.token1) {
        // raw = NOCK_raw per USDC_raw -> USDC per NOCK = raw^-1 * 10^(dec1-dec0)
        10f64.powi(src.token1.decimals - src.token0.decimals) / raw
    } else if is_nock_token(&src.token0) && is_usdc_token(&src.token1) {
        // raw = USDC_raw per NOCK_raw -> USDC per NOCK = raw * 10^(dec1-dec0)... dec0=16, dec1=6
        raw * 10f64.powi(src.token1.decimals - src.token0.decimals)
    } else {
        return None;
    };
    // A zero sqrtPrice (uninitialized pool) gives +inf: not a price.
    if price.is_finite() && price > 0.0 { Some(price) } else { None }
}

/// USD price of one human unit of a position leg.
pub(crate) fn leg_usd_price(token: &PositionTokenInfo, nock_usd: f64, eth_usd: f64) -> f64 {
    if is_usdc_token(token) {
        1.0
    } else if is_nock_token(token) {
        nock_usd
    } else if is_eth_token(token) {
        eth_usd
    } else {
        0.0
    }
}

/// Build the snapshot `positions` array from raw reads + live prices.
pub(crate) fn build_position_snapshots(raw_positions: &[RawPosition], nock_usd: f64, eth_usd: f64) -> Vec<Position> {
    raw_positions
        .iter()
        .map(|p| {
            let p0 = leg_usd_price(&p.token0, nock_usd, eth_usd);
            let p1 = leg_usd_price(&p.token1, nock_usd, eth_usd);
            let amount_usd = p.amount0 * p0 + p.amount1 * p1;
            let fees_usd = p.fee0 * p0 + p.fee1 * p1;
            Position {
                position_id: p.position_id.clone(),
                network: p.network.clone(),
                wallet: p.wallet.clone(),
                protocol: p.protocol.clone(),
                nft_id: p.nft_id,
                pool_id: p.pool_id.clone(),
                npm_address: p.npm_address.clone(),
                fee_tier_bps: p.fee_tier_bps,
                token0: p.token0.clone(),
                token1: p.token1.clone(),
                amount0: format!("{:.6}", p.amount0),
                amount1: format!("{:.6}", p.amount1),
                amount_usd: format!("{:.2}", amount_usd),
                tokens_owed_0: format!("{:.6}", p.fee0),
                tokens_owed_1: format!("{:.6}", p.fee1),
                fees_usd: format!("{:.6}", fees_usd),
                lower_tick: p.tick_lower,
                upper_tick: p.tick_upper,
                // Display orientation: bigger tick -> lower price (token0-per-token1).
                lower_price: if is_full_range(p.tick_lower, p.tick_upper) {
                    "0".to_string()
                } else {
                    fmt_price(tick_to_display_price_dec(p.tick_upper, p.token0.decimals, p.token1.decimals))
                },
                upper_price: if is_full_range(p.tick_lower, p.tick_upper) {
                    "0".to_string()
                } else {
                    fmt_price(tick_to_display_price_dec(p.tick_lower, p.token0.decimals, p.token1.decimals))
                },
                liquidity_amount: p.liquidity.to_string(),
                in_range: p.in_range,
                is_price_source: p.is_price_source,
                pool_active: None, // filled by apply_pool_activity once pool reads are in
                full_range: is_full_range(p.tick_lower, p.tick_upper),
            }
        })
        .collect()
}

/// Stamp each snapshot position with whether its pool is live. Unknown stays None.
pub(crate) fn apply_pool_activity(positions: &mut [Position], pool_liquidity: &HashMap<String, String>) {
    for p in positions.iter_mut() {
        p.pool_active = pool_liquidity
            .get(&p.position_id)
            .and_then(|v| v.parse::<u128>().ok())
            .map(|l| l > 0);
    }
}

/// One PoolMechanic per tracked position's pool. volatile_assets carry the tracked
/// wallet's amounts in that pool (NOT pool-wide reserves).
pub(crate) fn build_pool_mechanics(raw_positions: &[RawPosition], nock_usd: f64, eth_usd: f64) -> Vec<PoolMechanic> {
    raw_positions
        .iter()
        .map(|p| {
            let platform = match p.protocol.as_str() {
                "uniswap-v4" => "UniswapV4",
                "uniswap-v3" => "UniswapV3",
                "aerodrome" => "Aerodrome",
                _ => "Other",
            };
            let p0 = leg_usd_price(&p.token0, nock_usd, eth_usd);
            let p1 = leg_usd_price(&p.token1, nock_usd, eth_usd);
            PoolMechanic {
                pool_id: p.pool_id.clone(),
                platform: platform.to_string(),
                pool_type: "concentrated".to_string(),
                volatile_assets: vec![
                    PoolAsset {
                        token_address: p.token0.address.clone(),
                        token_symbol: p.token0.symbol.clone(),
                        reserve_amount: format!("{:.6}", p.amount0),
                        reserve_usd: format!("{:.2}", p.amount0 * p0),
                    },
                    PoolAsset {
                        token_address: p.token1.address.clone(),
                        token_symbol: p.token1.symbol.clone(),
                        reserve_amount: format!("{:.6}", p.amount1),
                        reserve_usd: format!("{:.2}", p.amount1 * p1),
                    },
                ],
                liquidity_usd: format!("{:.2}", p.amount0 * p0 + p.amount1 * p1),
                fee_accrued_usd: format!("{:.6}", p.fee0 * p0 + p.fee1 * p1),
                fee_tier_bps: p.fee_tier_bps,
                tick_current: Some(p.tick),
                sqrt_price_x96: Some(format!("{:032x}{:032x}", p.sqrt_price_x96.0, p.sqrt_price_x96.1)),
            }
        })
        .collect()
}

/// One ContractFee per tracked position with its unclaimed fees. A position listed in
/// `not_lp_claimable` (an Aero pool whose claimFees() reverts) says so in `amount`
/// and is not claimable, rather than reading as $0 of fees.
pub(crate) fn build_contract_fees(positions: &[Position], live_gas_cost_usd: f64, not_lp_claimable: &[String]) -> Vec<ContractFee> {
    positions
        .iter()
        .map(|p| {
            let not_claimable = not_lp_claimable.iter().any(|id| id == &p.position_id);
            let label = match p.protocol.as_str() {
                "uniswap-v4" => "Uniswap V4".to_string(),
                "uniswap-v3" => "Uniswap V3".to_string(),
                "aerodrome" => "Aerodrome".to_string(),
                _ => p.protocol.clone(),
            };
            ContractFee {
                contract_address: if p.protocol == "uniswap-v3" {
                    p.npm_address.clone().unwrap_or_else(|| p.pool_id.clone())
                } else if p.protocol == "aerodrome" {
                    p.pool_id.clone()
                } else {
                    UNISWAP_V4_POSM_BASE.to_string()
                },
                fee_token_address: p.token0.address.clone(),
                fee_token_symbol: if p.protocol == "aerodrome" {
                    format!("{}+{} ({})", p.token0.symbol, p.token1.symbol, label)
                } else {
                    format!("{}+{} ({} #{})", p.token0.symbol, p.token1.symbol, label, p.nft_id)
                },
                amount: if not_claimable {
                    AERO_NOT_LP_CLAIMABLE.to_string()
                } else {
                    format!(
                        "{:.p0$} {} + {:.p1$} {}",
                        p.tokens_owed_0.parse::<f64>().unwrap_or(0.0),
                        p.token0.symbol,
                        p.tokens_owed_1.parse::<f64>().unwrap_or(0.0),
                        p.token1.symbol,
                        p0 = (p.token0.decimals as usize).min(8),
                        p1 = (p.token1.decimals as usize).min(8),
                    )
                },
                amount_usd: p.fees_usd.clone(),
                claimable: !not_claimable && p.fees_usd.parse::<f64>().unwrap_or(0.0) > 0.0,
                gas_break_even_usd: format!("{:.4}", live_gas_cost_usd),
            }
        })
        .collect()
}
