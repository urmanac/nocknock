//! Tracked positions with no NOCK leg (#151, #152, #153): the operator's gas-side
//! WETH/USDC ranges, Uniswap v3 #6607225 and Aero CL #3449558 (staked in its gauge).
//! They are measured, never steered: valuation and the totals count them, while the NOCK
//! LP target, the rebuild, the engine label, plan headroom and positions_nock do not.
//! Nothing here touches the network or needs a database or a config.yaml.
use super::*;

const WALLET: &str = "0xee114cc97ec5adeb34ba5645e315b497d8487a45";
const GAUGE: &str = "0x46a9dca72d78366571be583aa69c1dc16e6889fb";
const ETH_USD: f64 = 1800.0;
/// USDC inside the two WETH/USDC ranges.
const GAS_LP_USDC: f64 = 76.893265 + 80.549340;

pub(crate) fn weth() -> PositionTokenInfo {
    PositionTokenInfo { address: WETH_BASE.to_string(), symbol: "WETH".to_string(), decimals: 18 }
}
pub(crate) fn usdc() -> PositionTokenInfo {
    PositionTokenInfo { address: USDC_BASE.to_string(), symbol: "USDC".to_string(), decimals: 6 }
}
pub(crate) fn nock() -> PositionTokenInfo {
    PositionTokenInfo { address: NOCK_BASE.to_string(), symbol: "NOCK".to_string(), decimals: 16 }
}

/// sqrtPriceX96 at a tick (synthetic positions only).
pub(crate) fn sqrt_x96_at_tick(tick: i32) -> U256 {
    let v = tick_sqrt_price(tick) * 79228162514264337593543950336.0;
    let two128 = 340282366920938463463374607431768211456.0f64;
    let hi = (v / two128).floor();
    U256(hi as u128, (v - hi * two128) as u128)
}

pub(crate) fn with_amounts(mut p: RawPosition) -> RawPosition {
    let (a0, a1) = concentrated_amounts_raw(p.liquidity, p.sqrt_price_x96, p.tick_lower, p.tick_upper);
    p.amount0 = a0 / 10f64.powi(p.token0.decimals);
    p.amount1 = a1 / 10f64.powi(p.token1.decimals);
    p
}

/// USDC/NOCK v4 3.00% wide range (#3163450), the price source.
pub(crate) fn nock_usdc_wide() -> RawPosition {
    with_amounts(RawPosition {
        position_id: "uniswap-v4:base:3163450".to_string(),
        network: "base".to_string(),
        protocol: "uniswap-v4".to_string(),
        wallet: WALLET.to_string(),
        nft_id: 3163450,
        pool_id: "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13".to_string(),
        npm_address: None,
        fee_tier_bps: 300,
        token0: usdc(),
        token1: nock(),
        liquidity: 185454734478733,
        sqrt_price_x96: U256::from_hex_be("0xa9d5d9b7b19cc983c088d1f6cd97b").unwrap(),
        tick: 269065,
        tick_lower: 262200,
        tick_upper: 282300,
        amount0: 0.0,
        amount1: 0.0,
        fee0: 0.0,
        fee1: 0.0,
        in_range: true,
        is_price_source: true,
    })
}

/// WETH/NOCK v3 1.00% (#6336662).
pub(crate) fn weth_nock_v3() -> RawPosition {
    with_amounts(RawPosition {
        position_id: "uniswap-v3:base:6336662".to_string(),
        network: "base".to_string(),
        protocol: "uniswap-v3".to_string(),
        wallet: WALLET.to_string(),
        nft_id: 6336662,
        pool_id: "0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a".to_string(),
        npm_address: Some(UNISWAP_V3_NPM_BASE.to_string()),
        fee_tier_bps: 100,
        token0: weth(),
        token1: nock(),
        liquidity: 1305339058220032430,
        sqrt_price_x96: U256::from_hex_be("0x21d595ed92e90d8945a5ef7766").unwrap(),
        tick: 70433,
        tick_lower: 70400,
        tick_upper: 74800,
        amount0: 0.0,
        amount1: 0.0,
        fee0: 0.0,
        fee1: 0.0,
        in_range: true,
        is_price_source: false,
    })
}

/// Uniswap v3 WETH/USDC 0.05% #6607225, in range; amounts as read on 2026-11-17.
pub(crate) fn gas_range_v3() -> RawPosition {
    let tick = -201250;
    RawPosition {
        position_id: "uniswap-v3:base:6607225".to_string(),
        network: "base".to_string(),
        protocol: "uniswap-v3".to_string(),
        wallet: WALLET.to_string(),
        nft_id: 6607225,
        pool_id: "0xd0b53d9277642d899df5c87a3966a349a798f224".to_string(),
        npm_address: Some(UNISWAP_V3_NPM_BASE.to_string()),
        fee_tier_bps: 5,
        token0: weth(),
        token1: usdc(),
        liquidity: 1_000_000_000_000_000,
        sqrt_price_x96: sqrt_x96_at_tick(tick),
        tick,
        tick_lower: -203320,
        tick_upper: -196420,
        amount0: 0.100163,
        amount1: 76.893265,
        fee0: 0.0,
        fee1: 0.000674,
        in_range: true,
        is_price_source: false,
    }
}

/// Aero CL WETH/USDC #3449558, staked in its gauge; amounts as read on 2026-11-17.
fn gas_range_aero_cl() -> RawPosition {
    let tick = -201250;
    RawPosition {
        position_id: "aerodrome-cl:base:3449558".to_string(),
        network: "base".to_string(),
        protocol: "aerodrome-cl".to_string(),
        wallet: WALLET.to_string(),
        nft_id: 3449558,
        pool_id: "0x4a7af46bc8abc00048c6c7920a1488daf568366c".to_string(),
        npm_address: None,
        fee_tier_bps: 0,
        token0: weth(),
        token1: usdc(),
        liquidity: 1_000_000_000_000_000,
        sqrt_price_x96: sqrt_x96_at_tick(tick),
        tick,
        tick_lower: -203350,
        tick_upper: -196550,
        amount0: 0.100773,
        amount1: 80.549340,
        fee0: 0.0,
        fee1: 0.0,
        in_range: true,
        is_price_source: false,
    }
}

fn gas_ids() -> [String; 2] {
    [gas_range_v3().position_id, gas_range_aero_cl().position_id]
}

/// The NOCK positions alone: the reference every NOCK-strategy figure must match.
pub(crate) fn nock_only() -> RawOnChainData {
    let positions = vec![nock_usdc_wide(), weth_nock_v3()];
    let mut raw = RawOnChainData {
        spot_price_nock_usd: nock_spot_from_positions(&positions).expect("price source"),
        positions,
        live_eth_price_usd: ETH_USD,
        live_gas_cost_usd: 0.0036,
        wallet_eth: Some(0.05),
        wallet_weth: Some(0.0),
        wallet_balances: vec![
            WalletBalance {
                wallet_address: WALLET.to_string(),
                token_symbol: "NOCK".to_string(),
                token_address: NOCK_BASE.to_string(),
                amount: "115747.8001".to_string(),
                amount_usd: "0".to_string(),
                updated_at: "now".to_string(),
            },
            WalletBalance {
                wallet_address: WALLET.to_string(),
                token_symbol: "USDC".to_string(),
                token_address: USDC_BASE.to_string(),
                amount: "1053.0".to_string(),
                amount_usd: "1053.0".to_string(),
                updated_at: "now".to_string(),
            },
        ],
        ..Default::default()
    };
    let ids: Vec<String> = raw.positions.iter().map(|p| p.position_id.clone()).collect();
    for id in ids {
        raw.pool_liquidity.insert(id, "80173935548453779".to_string());
    }
    raw
}

/// The NOCK positions plus both WETH/USDC ranges, the Aero one staked and earning AERO.
pub(crate) fn with_gas_ranges() -> RawOnChainData {
    let mut raw = nock_only();
    let (v3, cl) = (gas_range_v3(), gas_range_aero_cl());
    for p in [&v3, &cl] {
        raw.pool_liquidity.insert(p.position_id.clone(), "5000000000000000000".to_string());
    }
    raw.position_facts.insert(
        cl.position_id.clone(),
        PositionFacts {
            tick_spacing: Some(100),
            staked_in_gauge: Some(GAUGE.to_string()),
            fees_note: Some(AERO_CL_STAKED_FEES_NOTE.to_string()),
        },
    );
    raw.aero = AeroReads {
        wallet_aero: Some(0.0),
        price_usd: Some(0.5),
        price_note: None,
        unclaimed: Some(vec![AeroUnclaimed {
            position_id: cl.position_id.clone(),
            gauge: Some(GAUGE.to_string()),
            amount: Some(0.25),
            note: None,
        }]),
    };
    raw.positions.push(v3);
    raw.positions.push(cl);
    raw
}

pub(crate) fn profile() -> OperatorProfile {
    OperatorProfile {
        treasury_floor_nock: 100_000.0,
        lp_floor_tolerance_bps: 0,
        tax_reserve_ratio: 0.0,
        target_weights_bps: TargetWeightsBps { treasury: 8000, uniswap_lp: 1500, cash: 500 },
        cash_reserve_usdc: None,
        ladder_bottom_usd: None,
        ladder_rungs: None,
    }
}

fn metrics(raw: &RawOnChainData, plan: &PlanContext) -> DerivedMetrics {
    compute_derived_metrics_with_plan(
        raw, 0.0, &profile(), StructuralChannels { channels: vec![] }, &[],
        &LedgerAggregate::default(), &MarketRegime::default(), plan,
    )
}

fn usd(s: &str) -> f64 {
    s.parse().unwrap_or_else(|_| panic!("not a number: {s}"))
}

fn position_usd(raw: &RawOnChainData, p: &RawPosition) -> f64 {
    p.amount0 * leg_usd_price(&p.token0, raw.spot_price_nock_usd, raw.live_eth_price_usd)
        + p.amount1 * leg_usd_price(&p.token1, raw.spot_price_nock_usd, raw.live_eth_price_usd)
}

#[test]
fn test_is_nock_pair_is_the_symbol_rule() {
    assert!(is_nock_pair(&nock_usdc_wide()));
    assert!(is_nock_pair(&weth_nock_v3()));
    assert!(!is_nock_pair(&gas_range_v3()));
    assert!(!is_nock_pair(&gas_range_aero_cl()));
    // NOCK as token0 counts too.
    let mut flipped = nock_usdc_wide();
    std::mem::swap(&mut flipped.token0, &mut flipped.token1);
    assert!(is_nock_pair(&flipped));
}

#[test]
fn test_gas_ranges_are_valued_at_what_they_hold() {
    let raw = with_gas_ranges();
    let v3 = position_usd(&raw, &raw.positions[2]);
    let cl = position_usd(&raw, &raw.positions[3]);
    // 0.100163 WETH × $1800 + 76.893265 USDC; 0.100773 WETH × $1800 + 80.549340 USDC.
    assert!((v3 - 257.186665).abs() < 1e-6, "{v3}");
    assert!((cl - 261.94074).abs() < 1e-6, "{cl}");
}

#[test]
fn test_rebuild_judges_nock_lp_only_and_still_suggests() {
    let reference = metrics(&nock_only(), &PlanContext::none());
    let raw = with_gas_ranges();
    let m = metrics(&raw, &PlanContext::none());
    let nock_lp: f64 = raw.positions.iter().filter(|p| is_nock_pair(p)).map(|p| position_usd(&raw, p)).sum();
    let all_lp: f64 = raw.positions.iter().map(|p| position_usd(&raw, p)).sum();
    let target = usd(&m.lp_rebuild.target_usd);
    // The fixture is the issue's case: all LP is over the target, NOCK LP is under it.
    assert!(all_lp > target && nock_lp < target, "all {all_lp}, nock {nock_lp}, target {target}");
    assert_eq!(m.lp_rebuild.actual_usd, format!("{:.2}", nock_lp));
    assert_eq!(m.lp_rebuild.actual_usd, reference.lp_rebuild.actual_usd);
    assert_eq!(m.lp_rebuild.status, "under_target");
    // The target is a share of the whole portfolio, which includes the gas ranges.
    assert!(target > usd(&reference.lp_rebuild.target_usd));
    assert!(!m.lp_rebuild.suggestions.is_empty());
    let added: f64 = m.lp_rebuild.suggestions.iter().map(|s| usd(&s.add_usd)).sum();
    assert!(added > 0.0);
    // The gap is NOCK LP's: a completed rebuild lands on the target, not short of it by
    // the gas ranges' value.
    assert!((added - usd(&m.lp_rebuild.gap_usd)).abs() < 0.05, "added {added}, gap {}", m.lp_rebuild.gap_usd);
    // The bias text names NOCK LP.
    assert!(m.global_status.shot_advice_details.contains(&format!("NOCK LP holds ${:.2} against", nock_lp)), "{}", m.global_status.shot_advice_details);
}

#[test]
fn test_gas_ranges_are_neither_templates_nor_exclusions() {
    let m = metrics(&with_gas_ranges(), &PlanContext::none());
    for id in gas_ids() {
        assert!(m.lp_rebuild.suggestions.iter().all(|s| s.position_id != id), "{id} suggested");
        assert!(m.lp_rebuild.excluded.iter().all(|e| e.position_id != id), "{id} listed as excluded");
    }
    // Called directly, with any target, the same holds.
    let (s, x) = lp_rebuild_suggestions(
        &with_gas_ranges(),
        10_000.0,
        RebuildFunds { nock: 1_000_000.0, floor_tolerance_nock: 0.0, floor_set: true, usdc: 10_000.0 },
    );
    for id in gas_ids() {
        assert!(s.iter().all(|sg| sg.position_id != id) && x.iter().all(|e| e.position_id != id));
    }
    // The WETH/NOCK range is still excluded for its gas leg, as before.
    assert!(x.iter().any(|e| e.nft_id == 6336662 && e.reason == LP_GAS_LEG_REASON));
}

#[test]
fn test_only_gas_ranges_tracked_is_no_nock_positions() {
    let mut raw = with_gas_ranges();
    raw.positions.retain(|p| !is_nock_pair(p));
    let m = metrics(&raw, &PlanContext::none());
    assert_eq!(m.lp_rebuild.status, "no_positions");
    assert!(m.lp_rebuild.suggestions.is_empty() && m.lp_rebuild.excluded.is_empty());
    let (summary, active) = lp_positions_summary(&raw);
    assert!(!active);
    assert_eq!(summary, "No NOCK LP positions tracked. +2 ETH/USDC ranges, measured only (2 in range).");
}

#[test]
fn test_deviation_matrix_targets_nock_lp_and_measures_gas_lp() {
    let raw = with_gas_ranges();
    let m = metrics(&raw, &PlanContext::none());
    let total = usd(&m.global_status.total_portfolio_usd);
    let rows = &m.portfolio_deviation_matrix.allocations;
    let names: Vec<&str> = rows.iter().map(|r| r.bucket_name.as_str()).collect();
    assert_eq!(
        names,
        ["Sovereign Treasury", "NOCK LP (NOCK-pair positions)", "Liquid USDC (Escrow/Vault)", "Gas (wallet ETH+WETH)", "AERO (wallet)", "ETH/USDC LP (measured)"]
    );
    let nock_lp: f64 = raw.positions.iter().filter(|p| is_nock_pair(p)).map(|p| position_usd(&raw, p)).sum();
    let gas_lp: f64 = raw.positions.iter().filter(|p| !is_nock_pair(p)).map(|p| position_usd(&raw, p)).sum();
    let lp = &rows[1];
    assert_eq!(lp.actual_weight_bps, ((nock_lp / total) * 10000.0) as i32);
    assert_eq!(lp.target_weight_bps, 1500);
    assert_eq!(lp.status, "under_funded");
    let g = &rows[5];
    assert_eq!(g.target_weight_bps, 0);
    assert_eq!(g.status, "no_target");
    assert_eq!(g.actual_weight_bps, ((gas_lp / total) * 10000.0) as i32);
    assert_eq!(g.deviation_bps, g.actual_weight_bps);
    // The total still counts both ranges: they are held.
    let reference = metrics(&nock_only(), &PlanContext::none());
    let diff = total - usd(&reference.global_status.total_portfolio_usd);
    assert!((diff - gas_lp).abs() < 0.011, "total moved by {diff}, gas LP {gas_lp}");
    // Without a gas range there is no ETH/USDC row: the matrix is what it was.
    let ref_names: Vec<&str> = reference.portfolio_deviation_matrix.allocations.iter().map(|r| r.bucket_name.as_str()).collect();
    assert_eq!(ref_names.len(), 5);
    assert!(!ref_names.contains(&"ETH/USDC LP (measured)"));
}

#[test]
fn test_stable_total_counts_gas_range_usdc_but_plan_headroom_does_not() {
    let reference = metrics(&nock_only(), &PlanContext::none());
    let m = metrics(&with_gas_ranges(), &PlanContext::none());
    // The USDC pillar is what is held: the gas ranges' USDC is in it.
    let moved = usd(&m.global_status.total_stable_usd) - usd(&reference.global_status.total_stable_usd);
    assert!((moved - GAS_LP_USDC).abs() < 0.011, "{moved}");
    assert_eq!(m.global_status.total_usdc_amount, m.global_status.total_stable_usd);
    // The working-capital note names NOCK-pair USDC only.
    let nock_usdc: f64 = nock_only().positions.iter().map(|p| if is_usdc_token(&p.token0) { p.amount0 } else { 0.0 }).sum();
    assert!(nock_usdc > 0.0);
    let note = format!("The ${:.2} of USDC inside NOCK LP is working capital", nock_usdc);
    assert!(m.global_status.shot_advice_details.contains(&note), "{}", m.global_status.shot_advice_details);

    // PlanGuards.lp_usdc: a protected cash reserve is measured on wallet + NOCK-pair USDC.
    let cfg = AccumulationPlanConfig {
        id: "gas-lp-test".to_string(),
        budget_usdc: 1000.0,
        tranches: 2,
        interval_hours: 24.0,
        starts_at: None,
        venue: None,
        max_price_usd: None,
        pause_when_overextended: true,
        respect_cash_target: false,
        cash_reserve_usdc: Some(1000.0),
    };
    let now = Utc.with_ymd_and_hms(2026, 11, 17, 12, 0, 0).unwrap();
    let ctx = PlanContext { cfg: Some(cfg), fills: PlanFills::default(), ledger_unavailable: false, now };
    let plan = metrics(&with_gas_ranges(), &ctx).accumulation_plan.expect("plan");
    let constraint = plan.binding_constraint.expect("the reserve binds");
    let expected = format!(
        "stable balance ${:.2} (wallet $1053.00 + NOCK LP ${:.2}) less the $1000.00 protected cash reserve",
        1053.0 + nock_usdc,
        nock_usdc
    );
    assert!(constraint.contains(&expected), "{constraint}");
    // Same headroom as without the gas ranges.
    let plan_ref = metrics(&nock_only(), &ctx).accumulation_plan.expect("plan");
    assert_eq!(plan.tranche_usdc, plan_ref.tranche_usdc);
}

#[test]
fn test_positions_nock_leaves_out_ranges_with_no_nock_leg() {
    let raw = with_gas_ranges();
    let m = metrics(&raw, &PlanContext::none());
    let ids: Vec<&str> = m.asset_aggregation_series.positions_nock.iter().map(|s| s.position_id.as_str()).collect();
    assert_eq!(ids, ["uniswap-v4:base:3163450", "uniswap-v3:base:6336662"]);
    // The LP value series still counts every position.
    let all_lp: f64 = raw.positions.iter().map(|p| position_usd(&raw, p)).sum();
    assert_eq!(m.asset_aggregation_series.active_protocol_series, vec![format!("{:.2}", all_lp)]);
}

#[test]
fn test_engine_label_judges_nock_ranges_and_names_the_gas_ranges() {
    let raw = with_gas_ranges();
    let (summary, active) = lp_positions_summary(&raw);
    assert!(active);
    assert_eq!(summary, "2 NOCK LP positions tracked: 2 ranged (2 in range). +2 ETH/USDC ranges, measured only (2 in range).");

    // NOCK ranges out of range, ETH/USDC ranges in range: the NOCK engine is not active.
    let mut out = with_gas_ranges();
    for p in out.positions.iter_mut().filter(|p| is_nock_pair(p)) {
        p.in_range = false;
    }
    let (summary, active) = lp_positions_summary(&out);
    assert!(!active, "{summary}");
    assert!(summary.contains("2 ranged (0 in range, 2 out of range)") && summary.contains("measured only (2 in range)"), "{summary}");
    let m = metrics(&out, &PlanContext::none());
    assert_ne!(m.global_status.shot_advice_action, "HOLD & ACCRETE (Dual Growth Engine Active)");

    // Without gas ranges the text is what it always was.
    let (plain, _) = lp_positions_summary(&nock_only());
    assert_eq!(plain, "2 LP positions tracked: 2 ranged (2 in range).");

    // One gas range alone is singular.
    let mut one = with_gas_ranges();
    one.positions.retain(|p| p.nft_id != 3449558);
    assert!(lp_positions_summary(&one).0.ends_with(" +1 ETH/USDC range, measured only (1 in range)."));
}

#[test]
fn test_no_nock_lp_labels() {
    // Only gas ranges: the HOLD label says there is no NOCK LP, never "Dual Growth".
    let mut raw = with_gas_ranges();
    raw.positions.retain(|p| !is_nock_pair(p));
    let m = metrics(&raw, &PlanContext::none());
    let a = &m.global_status.shot_advice_action;
    assert!(!a.contains("Dual Growth"), "{a}");
    if a.starts_with("HOLD & ACCRETE") {
        assert_eq!(a, "HOLD & ACCRETE (No NOCK LP Tracked)");
    }
}

#[test]
fn test_claim_rule_for_the_gas_range_fee() {
    // #153: claimable $0.000674 against $0.0036 gas is below the gas cost.
    assert_eq!(recommended_claim_action(0.0007, 0.0036), "accumulate");
    let m = metrics(&with_gas_ranges(), &PlanContext::none());
    let rows = &m.velocity_gas_sandbox.contracts;
    // The Uniswap range's fee row: claimable rounds to $0.00 and gas is $0.0036.
    let v3 = rows.iter().find(|r| r.contract_name.contains("6607225")).expect("v3 fee row");
    assert_eq!(v3.recommended_action, "accumulate");
    // The staked Aero range's fees are not LP-claimable: no fee action at all.
    let cl = rows.iter().find(|r| r.contract_name.contains("3449558")).expect("Aero fee row");
    assert_eq!(cl.recommended_action, "not_claimable");
}

#[test]
fn test_mcp_plan_text_says_nock_lp_and_never_zero_short() {
    let m = metrics(&with_gas_ranges(), &PlanContext::none());
    let snap = json!({ "derived_metrics": serde_json::to_value(&m).unwrap() });
    let text = format_accumulation_plan_text(&snap);
    assert!(text.contains("• NOCK LP rebuild bias: under_target"), "{text}");
    for id in ["6607225", "3449558"] {
        assert!(!text.contains(id), "a gas range is not listed in the rebuild: {text}");
    }

    let over = json!({ "derived_metrics": { "lp_rebuild": {
        "status": "over_target", "actual_usd": "1008.79", "target_usd": "623.66", "gap_usd": "0.00",
        "suggestions": [], "excluded": []
    } } });
    let text = format_accumulation_plan_text(&over);
    assert!(text.contains("NOCK LP rebuild bias: over_target"), "{text}");
    assert!(text.contains("over target by $385.13"), "{text}");
    assert!(!text.contains("$0.00 short") && !text.contains(" short"), "{text}");

    let on = json!({ "derived_metrics": { "lp_rebuild": {
        "status": "on_target", "actual_usd": "489.52", "target_usd": "519.80", "gap_usd": "30.28"
    } } });
    let text = format_accumulation_plan_text(&on);
    assert!(text.contains("NOCK LP rebuild bias: on_target") && !text.contains("short"), "{text}");
}

#[test]
fn test_brackets_stay_one_per_position_and_carry_its_id() {
    // #152: one bracket per position in positions order (older clients pair by index),
    // each naming its position so newer clients pair by id and drop non-NOCK ranges.
    let raw = with_gas_ranges();
    let m = metrics(&raw, &PlanContext::none());
    let brackets = &m.tick_bracket_hypervisor.brackets;
    assert_eq!(brackets.len(), raw.positions.len());
    for (b, p) in brackets.iter().zip(&raw.positions) {
        assert_eq!(b.position_id, p.position_id);
    }
    // A WETH/USDC bracket is WETH per USDC: never a distance from the NOCK spot.
    for b in brackets.iter().filter(|b| gas_ids().contains(&b.position_id)) {
        assert_eq!(b.distance_bps, None);
        let (lo, hi) = (usd(&b.lower_price), usd(&b.upper_price));
        assert!(lo > 0.0003 && hi < 0.0007 && lo < hi, "{lo}..{hi}");
    }
    // The USDC/NOCK bracket keeps its distance.
    assert!(brackets[0].distance_bps.is_some());
    // The field is additive: a pre-2.8.2 bracket without it still deserializes.
    let old: TickBracket = serde_json::from_value(json!({
        "lower_price": "0.01", "upper_price": "0.02", "liquidity_usd": "1.00", "in_range": true, "distance_bps": null
    }))
    .unwrap();
    assert_eq!(old.position_id, "");
}

/// A synthetic USDC/AERO range: a non-NOCK pair that is not ETH/USDC (review of #151).
/// AERO has no leg price in valuation, so only its USDC is valued.
fn usdc_aero_range() -> RawPosition {
    let mut p = gas_range_v3();
    p.position_id = "uniswap-v3:base:9999001".to_string();
    p.nft_id = 9999001;
    p.pool_id = "0x0000000000000000000000000000000000009999".to_string();
    p.token0 = usdc();
    p.token1 = PositionTokenInfo { address: AERO_BASE.to_string(), symbol: "AERO".to_string(), decimals: 18 };
    p.amount0 = 40.0;
    p.amount1 = 50.0;
    p.fee1 = 0.0;
    p
}

fn profile_without_lp_target() -> OperatorProfile {
    OperatorProfile { target_weights_bps: TargetWeightsBps { treasury: 8000, uniswap_lp: 0, cash: 500 }, ..profile() }
}

#[test]
fn test_hold_label_names_nock_lp_when_only_gas_ranges_are_in_range() {
    // NOCK ranges out of range, ETH/USDC ranges in range: the label is about NOCK LP and
    // agrees with the summary, which says the gas ranges are in range.
    let mut raw = with_gas_ranges();
    for p in raw.positions.iter_mut().filter(|p| is_nock_pair(p)) {
        p.in_range = false;
    }
    let m = compute_derived_metrics_with_plan(
        &raw, 0.0, &profile_without_lp_target(), StructuralChannels { channels: vec![] }, &[],
        &LedgerAggregate::default(), &MarketRegime::default(), &PlanContext::none(),
    );
    assert_eq!(m.global_status.shot_advice_action, "HOLD & ACCRETE (No Ranged NOCK LP Confirmed In Range)");
    assert!(m.global_status.shot_advice_details.contains("+2 ETH/USDC ranges, measured only (2 in range)."), "{}", m.global_status.shot_advice_details);
}

#[test]
fn test_non_nock_ranges_are_named_by_one_rule() {
    // All non-NOCK ranges ETH/USDC: "ETH/USDC" in the summary and the matrix row.
    let raw = with_gas_ranges();
    assert_eq!(non_nock_range_kind(&raw.positions), "ETH/USDC");
    assert_eq!(gas_lp_bucket_name(&raw.positions), "ETH/USDC LP (measured)");
    // A NOCK-only book has nothing to name; the rule is vacuous and no row is emitted.
    assert_eq!(non_nock_range_kind(&nock_only().positions), "ETH/USDC");

    // A USDC/AERO range among them: "non-NOCK" everywhere.
    let mut mixed = with_gas_ranges();
    mixed.positions.push(usdc_aero_range());
    mixed.pool_liquidity.insert(usdc_aero_range().position_id, "1000".to_string());
    assert_eq!(non_nock_range_kind(&mixed.positions), "non-NOCK");
    let (summary, _) = lp_positions_summary(&mixed);
    assert!(summary.ends_with(" +3 non-NOCK ranges, measured only (3 in range)."), "{summary}");
    let m = metrics(&mixed, &PlanContext::none());
    let names: Vec<&str> = m.portfolio_deviation_matrix.allocations.iter().map(|r| r.bucket_name.as_str()).collect();
    assert_eq!(names.last(), Some(&"Non-NOCK LP (measured)"));
    assert!(!names.contains(&"ETH/USDC LP (measured)"));

    // The USDC/AERO range alone.
    let mut alone = nock_only();
    alone.positions.push(usdc_aero_range());
    let (summary, _) = lp_positions_summary(&alone);
    assert!(summary.ends_with(" +1 non-NOCK range, measured only (1 in range)."), "{summary}");
    let m = metrics(&alone, &PlanContext::none());
    assert_eq!(m.portfolio_deviation_matrix.allocations.last().unwrap().bucket_name, "Non-NOCK LP (measured)");
}

#[test]
fn test_usdc_token0_bracket_with_no_nock_leg_has_no_distance() {
    // Before the review's narrowing, any USDC token0 got a distance from the NOCK spot.
    let mut raw = nock_only();
    raw.positions.push(usdc_aero_range());
    let m = metrics(&raw, &PlanContext::none());
    let b = m.tick_bracket_hypervisor.brackets.iter().find(|b| b.position_id == "uniswap-v3:base:9999001").unwrap();
    assert!(!b.full_range);
    assert_eq!(b.distance_bps, None);
    // The USDC/NOCK bracket keeps its distance.
    assert!(m.tick_bracket_hypervisor.brackets[0].distance_bps.is_some());
}

/// Fees on the three LP-claimable positions; the staked Aero range has none to claim.
fn with_fees(usd_each: f64, gas: f64) -> RawOnChainData {
    let mut raw = with_gas_ranges();
    raw.live_gas_cost_usd = gas;
    for p in raw.positions.iter_mut() {
        match p.nft_id {
            3163450 => p.fee0 = usd_each,         // USDC
            6336662 => p.fee0 = usd_each / ETH_USD, // WETH
            6607225 => p.fee1 = usd_each,         // USDC
            _ => {}
        }
    }
    raw
}

#[test]
fn test_batch_harvest_counts_only_fees_worth_claiming() {
    // Three positions with $1.90 each ($5.70 in all) against $2.00 gas per claim: no
    // single claim pays for itself, so there is no harvest, though the total is over $5.
    let m = metrics(&with_fees(1.90, 2.0), &PlanContext::none());
    assert!(usd(&m.global_status.total_fees_claimable_usd) >= 5.0);
    for r in m.velocity_gas_sandbox.contracts.iter().filter(|r| r.recommended_action != "not_claimable") {
        assert_eq!(r.recommended_action, "accumulate", "{}", r.contract_name);
    }
    assert!(!m.global_status.shot_advice_action.contains("BATCH HARVEST"), "{}", m.global_status.shot_advice_action);
    assert!(!m.global_status.shot_advice_details.contains("BATCH HARVEST"));

    // The same fees against $0.50 gas: every claim pays (claim_now), and the efficiency
    // is the fees over the gas of all three claims: 5.70 / (0.50 × 3) = 3.8, "4x".
    let m = metrics(&with_fees(1.90, 0.5), &PlanContext::none());
    let (a, d) = (&m.global_status.shot_advice_action, &m.global_status.shot_advice_details);
    assert!(a.contains("BATCH HARVEST"), "{a}");
    assert!(d.contains("Uncollected LP fees ($5.70) have reached the fixed $5.00 harvest threshold (3.8x gas efficiency over 3 claims)."), "{d}");

    // Only the rows worth claiming count: two at $3.00 with $2.00 gas (claim_recommended)
    // make $6.00 of a $6.10 total; the third, $0.10, is left out of the sum and the count.
    let mut raw = with_fees(3.0, 2.0);
    raw.positions.iter_mut().find(|p| p.nft_id == 6607225).unwrap().fee1 = 0.10;
    let m = metrics(&raw, &PlanContext::none());
    let d = &m.global_status.shot_advice_details;
    assert!(d.contains("Uncollected LP fees ($6.00 of $6.10 uncollected) have reached the fixed $5.00 harvest threshold (1.5x gas efficiency over 2 claims)."), "{d}");
}
