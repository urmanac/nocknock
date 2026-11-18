use crate::*;

/// Significant digits a sub-cent range bound carries (issue #131).
pub(crate) const SUB_CENT_PRICE_SIG_DIGITS: i32 = 6;

/// Adaptive-precision price formatting for range bounds: 5 decimals for readable
/// values; a sub-cent one (e.g. WETH per NOCK ≈ 0.0000056) carries
/// `SUB_CENT_PRICE_SIG_DIGITS` significant digits, so the card's inverted "NOCK per
/// WETH" and the USD bracket agree with the ticks to the digit they show (issue #131:
/// 8 decimals left 3 significant digits, and the card read 177,305 where the ticks
/// give 177,158).
pub(crate) fn fmt_price(v: f64) -> String {
    if v != 0.0 && v.is_finite() && v.abs() < 0.01 {
        let magnitude = v.abs().log10().floor() as i32; // -6 for 0.0000056
        let decimals = (SUB_CENT_PRICE_SIG_DIGITS - 1 - magnitude).max(5) as usize;
        format!("{:.*}", decimals, v)
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
                // Filled by apply_position_facts for the protocols that have them.
                tick_spacing: None,
                staked_in_gauge: None,
                fees_note: None,
                // Filled by apply_aero_rewards for Aero positions whose gauge was read.
                unclaimed_aero: None,
                unclaimed_aero_usd: None,
                unclaimed_aero_note: None,
                collectible_reward: None,
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

/// Stamp each snapshot position with the facts only its protocol has (v0.6.0, #91):
/// an Aero CL position's tick spacing, the gauge it is staked in, and the reason its
/// fee figure is not a read of LP-claimable fees. Positions without an entry are left
/// exactly as built. Run before build_contract_fees, which prints the reason.
pub(crate) fn apply_position_facts(positions: &mut [Position], facts: &HashMap<String, PositionFacts>) {
    for p in positions.iter_mut() {
        if let Some(f) = facts.get(&p.position_id) {
            p.tick_spacing = f.tick_spacing;
            p.staked_in_gauge = f.staked_in_gauge.clone();
            p.fees_note = f.fees_note.clone();
        }
    }
}

/// The snapshot's `positions` array for one cycle's reads: built, then stamped with pool
/// activity, the per-protocol facts and unclaimed AERO (#93). The ONE place those steps are chained:
/// fetch_raw_data (for `contract_fees`) and run_ingestion_cycle (for the published
/// snapshot) both call it, so what is published can never lack a stamp the fee rows
/// were built from (the v0.6.0 pre-release bug: `staked_in_gauge` and `fees_note` were
/// stamped only on the copy used for `contract_fees`). Pure.
pub(crate) fn snapshot_positions_from(
    raw_positions: &[RawPosition],
    nock_usd: f64,
    eth_usd: f64,
    pool_liquidity: &HashMap<String, String>,
    position_facts: &HashMap<String, PositionFacts>,
    aero: &AeroReads,
) -> Vec<Position> {
    let mut positions = build_position_snapshots(raw_positions, nock_usd, eth_usd);
    apply_pool_activity(&mut positions, pool_liquidity);
    apply_position_facts(&mut positions, position_facts);
    apply_aero_rewards(&mut positions, aero);
    positions
}

/// `snapshot_positions_from` over a finished cycle's reads. Pure.
pub(crate) fn snapshot_positions(raw: &RawOnChainData) -> Vec<Position> {
    snapshot_positions_from(
        &raw.positions,
        raw.spot_price_nock_usd,
        raw.live_eth_price_usd,
        &raw.pool_liquidity,
        &raw.position_facts,
        &raw.aero,
    )
}

/// One PoolMechanic per tracked position's pool. volatile_assets carry the tracked
/// wallet's amounts in that pool (NOT pool-wide reserves).
pub(crate) fn build_pool_mechanics(raw_positions: &[RawPosition], nock_usd: f64, eth_usd: f64) -> Vec<PoolMechanic> {
    raw_positions
        .iter()
        .map(|p| {
            let platform = pool_platform(&p.protocol);
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
/// `not_lp_claimable` (an Aero pool whose claimFees() reverts, or an Aero CL position
/// staked in its gauge) says so in `amount` and is not claimable,
/// rather than reading as $0 of fees. An Aero CL position carries its own reason
/// (`fees_note`, stamped by apply_position_facts). The reason is followed by the
/// position's unclaimed AERO, or by why that was not read (stamped by
/// apply_aero_rewards, #93): an amount, never advice. Since #142 the row also carries
/// that reward structured (`reward`), `earnings_kind` = `gauge_reward` when the reward
/// is what the position earns, and the reason alone in `lp_fees_note`.
pub(crate) fn build_contract_fees(positions: &[Position], live_gas_cost_usd: f64, not_lp_claimable: &[String]) -> Vec<ContractFee> {
    positions
        .iter()
        .map(|p| {
            // A position with a fees_note has no fee figure even if a caller left it out
            // of `not_lp_claimable`: the note is the position's own statement.
            let not_claimable =
                p.fees_note.is_some() || not_lp_claimable.iter().any(|id| id == &p.position_id);
            // What the position earns when its LP fees are not LP-claimable (#142): the
            // gauge reward, structured. A row with LP-claimable fees keeps its reward
            // too (this row has no action word), but its earnings kind stays LP fees.
            let (earnings_kind, _, lp_fees_note) =
                fee_row_earnings(!not_claimable, p.fees_note.as_deref(), p.collectible_reward.clone());
            ContractFee {
                contract_address: fee_contract_for(
                    &p.protocol,
                    p.npm_address.as_deref(),
                    &p.pool_id,
                    p.staked_in_gauge.as_deref(),
                ),
                fee_token_address: p.token0.address.clone(),
                fee_token_symbol: fee_token_label(p),
                amount: if not_claimable {
                    join_reason_and_aero(
                        Some(fees_note_text(p.fees_note.as_deref())),
                        aero_statement(
                            p.unclaimed_aero.as_deref(),
                            p.unclaimed_aero_usd.as_deref(),
                            p.unclaimed_aero_note.as_deref(),
                        ),
                    )
                    .unwrap_or_default()
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
                earnings_kind,
                reward: p.collectible_reward.clone(),
                lp_fees_note,
            }
        })
        .collect()
}
