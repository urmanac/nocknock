//! The ladder simulator (issue #166). The fixture is a synthetic snapshot at spot
//! $0.012944, built through the worker's own snapshot path (valuation, advice, the
//! floor-crossing price) from reads: the v4 USDC/NOCK 3% range (ticks 262200/282300,
//! tick spacing 300), the Aero basic full-range pool (its own price +2.3% from spot), two
//! gas-side WETH/USDC ranges, wallet USDC 1545.8357 and NOCK 90086.7891, floor 100,000
//! with 1,000 bps tolerated and a $950 standing reserve.
//!
//! Every expected number below was computed independently, by a script that shares no
//! code with the worker (/tmp/nn-v090/ladder_check/check.py, closed-form sqrt formulas
//! and its own bisection), and agrees with the #166 design and its critique.
use super::*;
use crate::tests_gas_lp::{gas_range_v3, nock, usdc, with_amounts};
use serde_json::Value;

const WALLET: &str = "0xee114cc97ec5adeb34ba5645e315b497d8487a45";
const V4_ID: &str = "uniswap-v4:base:3163450";
const AERO_ID: &str = "aerodrome:base:0x770fe8fc9248c6382950a86e39a9391b41d59e0a";

fn rel(a: f64, b: f64) -> f64 {
    ((a - b) / b).abs()
}

/// sqrtPriceX96 of a raw price (token1_raw per token0_raw), as the Aero adapter builds it.
fn sqrt_x96_of_raw(raw_price: f64) -> U256 {
    let v = raw_price.sqrt() * 79228162514264337593543950336.0;
    let two128 = 340282366920938463463374607431768211456.0f64;
    let hi = (v / two128).floor();
    U256(hi as u128, (v - hi * two128) as u128)
}

/// floor(sqrt(1e10 / 0.012944) * 2^96): spot is $0.012944.
fn spot_sqrt() -> U256 {
    U256::from_hex_be("0d69693142532bf9792f0b505c97d4").unwrap()
}

fn v4() -> RawPosition {
    let mut p = with_amounts(RawPosition {
        position_id: V4_ID.to_string(),
        network: "base".to_string(),
        protocol: "uniswap-v4".to_string(),
        wallet: WALLET.to_string(),
        nft_id: 3163450,
        pool_id: "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13".to_string(),
        npm_address: None,
        fee_tier_bps: 300,
        token0: usdc(),
        token1: nock(),
        liquidity: 283687495614364,
        sqrt_price_x96: spot_sqrt(),
        tick: 273743,
        tick_lower: 262200,
        tick_upper: 282300,
        amount0: 0.0,
        amount1: 0.0,
        fee0: 0.0,
        fee1: 0.0,
        in_range: true,
        is_price_source: true,
    });
    p.fee0 = 0.213182;
    p.fee1 = 113.759403;
    p
}

/// The Aero basic pool holding `u` USDC and `n` NOCK of the wallet's share; its price is
/// its own reserves' ratio, as the adapter reads it.
fn aero(u: f64, n: f64) -> RawPosition {
    let raw_price = n * 1e16 / (u * 1e6);
    RawPosition {
        position_id: AERO_ID.to_string(),
        network: "base".to_string(),
        protocol: "aerodrome".to_string(),
        wallet: WALLET.to_string(),
        nft_id: 0,
        pool_id: "0x770fe8fc9248c6382950a86e39a9391b41d59e0a".to_string(),
        npm_address: None,
        fee_tier_bps: 30,
        token0: usdc(),
        token1: nock(),
        liquidity: 96764372596543,
        sqrt_price_x96: sqrt_x96_of_raw(raw_price),
        tick: (raw_price.ln() / 1.0001f64.ln()).floor() as i32,
        tick_lower: MIN_TICK,
        tick_upper: MAX_TICK,
        amount0: u,
        amount1: n,
        fee0: 0.0,
        fee1: 0.0,
        in_range: true,
        is_price_source: false,
    }
}

fn row(symbol: &str, address: &str, amount: &str) -> WalletBalance {
    WalletBalance {
        wallet_address: WALLET.to_string(),
        token_symbol: symbol.to_string(),
        token_address: address.to_string(),
        amount: amount.to_string(),
        amount_usd: "0".to_string(),
        updated_at: "now".to_string(),
    }
}

fn raw_book(positions: Vec<RawPosition>) -> RawOnChainData {
    let mut raw = RawOnChainData {
        spot_price_nock_usd: nock_spot_from_positions(&positions).expect("price source"),
        live_eth_price_usd: 3000.0,
        live_gas_cost_usd: 0.004,
        wallet_eth: Some(0.016388),
        wallet_weth: Some(0.0),
        wallet_balances: vec![row("USDC", USDC_BASE, "1545.8357"), row("NOCK", NOCK_BASE, "90086.7891")],
        positions,
        ..Default::default()
    };
    for p in &raw.positions {
        raw.pool_liquidity.insert(p.position_id.clone(), "80173935548453779".to_string());
    }
    raw.position_facts.insert(V4_ID.to_string(), PositionFacts { tick_spacing: Some(300), ..Default::default() });
    raw
}

fn fixture_positions() -> Vec<RawPosition> {
    let mut gas2 = gas_range_v3();
    gas2.position_id = "uniswap-v3:base:6607226".to_string();
    gas2.nft_id = 6607226;
    vec![v4(), aero(111.367340, 8408.058135), gas_range_v3(), gas2]
}

fn profile(reserve: Option<f64>) -> OperatorProfile {
    OperatorProfile {
        treasury_floor_nock: 100_000.0,
        lp_floor_tolerance_bps: 1000,
        cash_reserve_usdc: reserve,
        ..OperatorProfile::default()
    }
}

fn generated_at() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-11-20T12:00:00Z").unwrap().with_timezone(&Utc)
}

/// The snapshot a cycle with these reads would store (the golden path's steps).
pub(crate) fn envelope_of(mut raw: RawOnChainData, profile: &OperatorProfile) -> SnapshotEnvelope {
    let positions = snapshot_positions(&raw);
    raw.pool_mechanics = build_pool_mechanics(&raw.positions, raw.spot_price_nock_usd, raw.live_eth_price_usd);
    raw.contract_fees = build_contract_fees(&positions, raw.live_gas_cost_usd, &raw.fees_not_lp_claimable);
    let derived_metrics = compute_derived_metrics_with_plan(
        &raw,
        0.0,
        profile,
        compute_structural_channels_sync(&[]),
        &[],
        &LedgerAggregate::default(),
        &MarketRegime::default(),
        &PlanContext::none(),
    );
    SnapshotEnvelope {
        meta: SnapshotMeta {
            generated_at: generated_at(),
            state_hash: "fixture".to_string(),
            version: SNAPSHOT_SPEC_VERSION.to_string(),
            snapshot_id: Uuid::nil(),
        },
        data: SnapshotData {
            ladders: Vec::new(),
            closed_positions: vec![],
            wallet_balances: raw.wallet_balances,
            positions,
            pool_mechanics: raw.pool_mechanics,
            concentrated_liquidity: vec![],
            contract_fees: raw.contract_fees,
            derived_metrics,
        },
    }
}

pub(crate) fn fixture() -> SnapshotEnvelope {
    envelope_of(raw_book(fixture_positions()), &profile(Some(950.0)))
}

fn req(pairs: &[(&str, &str)]) -> Result<LadderRequest, LadderError> {
    LadderRequest::from_params(&pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect())
}

fn sim_on(env: &SnapshotEnvelope, pairs: &[(&str, &str)]) -> Result<Value, LadderError> {
    let r = req(pairs)?;
    simulate_ladder(env, &r, generated_at() + chrono::Duration::minutes(5), "ok")
}

fn sim(pairs: &[(&str, &str)]) -> Value {
    sim_on(&fixture(), pairs).expect("simulated")
}

fn f(v: &Value, ptr: &str) -> f64 {
    v.pointer(ptr).and_then(|x| x.as_str()).unwrap_or_else(|| panic!("{ptr} missing in {v}")).parse().unwrap()
}

fn st<'a>(v: &'a Value, ptr: &str) -> &'a str {
    v.pointer(ptr).and_then(|x| x.as_str()).unwrap_or_else(|| panic!("{ptr} missing"))
}

fn grid_row<'a>(v: &'a Value, pct: &str) -> &'a Value {
    v["grid"].as_array().unwrap().iter().find(|r| r["pct_of_spot"] == pct).unwrap_or_else(|| panic!("no {pct} row"))
}

fn err(pairs: &[(&str, &str)]) -> LadderError {
    sim_on(&fixture(), pairs).expect_err("not simulated")
}

const CHAT: [(&str, &str); 3] = [("usdc", "900"), ("low_pct", "-30"), ("high_pct", "2")];
const SPOT_741: [(&str, &str); 3] = [("usdc", "741"), ("low", "0.01"), ("high", "spot")];

#[test]
fn test_ladder_fixture_is_the_stated_book() {
    let env = fixture();
    let gs = &env.data.derived_metrics.global_status;
    // Spot is exact from the price-source sqrtPriceX96, never the 5-dp series.
    let b = sim(&SPOT_741);
    assert_eq!(st(&b, "/snapshot/spot_usd"), "0.01294400");
    assert_eq!(gs.reserve_stable_usdc.as_deref(), Some("1769.54"));
    assert_eq!(gs.lp_nock_budget.as_deref(), Some("86.7891"));
    assert_eq!(gs.lp_floor_tolerance_bps, Some(1000));
    assert_eq!(gs.lp_nock_budget_note, None);
    assert_eq!(env.data.positions[0].tick_spacing, Some(300));
    // The Aero basic pool's own price is not spot (+2.3%).
    let aero_px = 111.367340 / 8408.058135;
    assert!(rel(aero_px, 0.012944) > 0.02);
}

#[test]
fn test_ladder_chat_agent_sizing_unsnapped() {
    let b = sim(&CHAT);
    assert_eq!(st(&b, "/bounds/low_usd"), "0.00906080");
    assert_eq!(st(&b, "/bounds/high_usd"), "0.01320288");
    assert_eq!(b["bounds"]["tick_lower"], Value::Null);
    assert!(rel(f(&b, "/sizing/liquidity_human"), 48430.1458) < 1e-6);
    // The Chat agent's "about 4.2k NOCK".
    assert!((f(&b, "/sizing/nock_needed_at_spot") - 4193.9773).abs() < 0.01);
    assert_eq!(st(&b, "/sizing/nock_check"), "not_given");
    assert!((f(&b, "/bottom/nock_held") - 87298.5623).abs() < 0.01);
    assert!((f(&b, "/bottom/nock_bought") - 83104.5850).abs() < 0.01);
    // The Chat agent's average of $0.01083.
    assert_eq!(st(&b, "/bottom/avg_buy_price_usd"), "0.01082973");
    assert_eq!(st(&b, "/top/usdc_held"), "954.83");
    assert_eq!(st(&b, "/top/avg_sell_price_usd"), "0.01307280");
    assert_eq!(b["top"]["out_of_range_above_now"], json!(false));
    assert_eq!(b["pool"]["snapped"], json!(false));
    assert!(st(&b, "/pool/note").starts_with("unsnapped"));
    assert_eq!(b["top_neighbours"], Value::Null);
    assert!(st(&b, "/top_neighbours_reason").contains("unsnapped"));
}

#[test]
fn test_ladder_grid_rows_chat_case() {
    let b = sim(&CHAT);
    for (pct, nock, usdc) in [
        ("-5.00", 15252.3984, "760.48"),
        ("-10.00", 27220.0041, "617.25"),
        ("-20.00", 54438.4865, "318.30"),
        ("-25.00", 70046.6414, "161.80"),
        ("-30.00", 87298.5623, "0.00"),
        ("-35.00", 87298.5623, "0.00"),
        ("-40.00", 87298.5623, "0.00"),
    ] {
        let r = grid_row(&b, pct);
        assert!((f(r, "/ladder_nock") - nock).abs() < 0.01, "{pct}: {r}");
        assert_eq!(st(r, "/ladder_usdc"), usdc, "{pct}");
    }
    // The bottom row carries its marker; the top (+2%) is above spot and listed too.
    assert!(grid_row(&b, "-30.00")["markers"].as_array().unwrap().contains(&json!("bottom")));
    assert!(grid_row(&b, "+2.00")["markers"].as_array().unwrap().contains(&json!("top")));
    assert_eq!(st(grid_row(&b, "-25.00"), "/stable_usdc"), "973.18");
    assert_eq!(st(grid_row(&b, "-25.00"), "/reserve_headroom_usdc"), "23.18");
    // Rows are in falling price.
    let prices: Vec<f64> = b["grid"].as_array().unwrap().iter().map(|r| f(r, "/price_usd")).collect();
    assert!(prices.windows(2).all(|w| w[0] > w[1]));
    assert_eq!(b["inputs"]["grid_to_pct"], json!(-40.0));
    assert_eq!(b["inputs"]["grid_step_pct"], json!(5.0));
}

#[test]
fn test_ladder_reserve_gating_breached_chat_case() {
    let b = sim(&CHAT);
    let rc = &b["reserve_check"];
    assert_eq!(rc["status"], "breached");
    // v4 59.6205 + Aero 93.1766 at the bottom.
    assert_eq!(st(rc, "/existing_ranges_usdc_at_bottom"), "152.80");
    assert_eq!(st(rc, "/stable_at_bottom_usdc"), "798.63");
    assert_eq!(st(rc, "/margin_usdc"), "-151.37");
    assert_eq!(st(rc, "/max_usdc_for_this_range"), "748.63");
    assert_eq!(st(rc, "/cash_reserve_usdc"), "950.00");
    assert_eq!(st(rc, "/stable_at_spot_usdc"), "1769.54");
}

#[test]
fn test_ladder_reserve_holds_741_top_at_spot_zero_nock() {
    let b = sim(&SPOT_741);
    assert_eq!(st(&b, "/sizing/nock_needed_at_spot"), "0.0000");
    assert!(rel(f(&b, "/sizing/liquidity_human"), 53805.9887) < 1e-6);
    assert!((f(&b, "/bottom/nock_held") - 65130.4316).abs() < 0.01);
    // The average is the geometric mean sqrt(0.01 * 0.012944).
    assert_eq!(st(&b, "/bottom/avg_buy_price_usd"), "0.01137717");
    assert!(rel(f(&b, "/bottom/avg_buy_price_usd"), (0.01f64 * 0.012944).sqrt()) < 1e-6);
    assert_eq!(b["top"]["out_of_range_above_now"], json!(true));
    assert_eq!(st(&b, "/top/usdc_held"), "741.00");
    assert_eq!(b["top"]["avg_sell_price_usd"], Value::Null);
    let r10 = grid_row(&b, "-10.00");
    assert!((f(r10, "/ladder_nock") - 25581.9618).abs() < 0.01);
    assert_eq!(st(r10, "/ladder_usdc"), "426.86");
    let r20 = grid_row(&b, "-20.00");
    assert!((f(r20, "/ladder_nock") - 55821.7500).abs() < 0.01);
    assert_eq!(st(r20, "/ladder_usdc"), "94.73");

    let rc = &b["reserve_check"];
    assert_eq!(rc["status"], "holds");
    // v4 73.2709 + Aero 97.8867 at $0.01.
    assert_eq!(st(rc, "/existing_ranges_usdc_at_bottom"), "171.16");
    assert_eq!(st(rc, "/stable_at_bottom_usdc"), "975.99");
    assert_eq!(st(rc, "/margin_usdc"), "25.99");
    assert_eq!(st(rc, "/max_usdc_for_this_range"), "766.99");
    // Below the bottom the existing ranges keep converting: a separate measured field,
    // which does not move the gate.
    let below = &rc["existing_ranges_below_bottom"];
    assert_eq!(st(below, "/pct_of_spot"), "-40.00");
    assert_eq!(st(below, "/stable_usdc"), "930.69");
    assert_eq!(st(below, "/headroom_usdc"), "-19.31");
    // The top reading: all USDC, the stable balance at spot unchanged.
    assert_eq!(rc["at_top"]["out_of_range_above_now"], json!(true));
    assert_eq!(st(rc, "/at_top/stable_usdc"), "1769.54");
    assert_eq!(st(rc, "/at_top/ladder_usdc"), "741.00");
    // Stated lines, not gates.
    assert_eq!(rc["accumulation_plan_unspent"], Value::Null);
    assert_eq!(st(rc, "/accumulation_plan_unspent_reason"), "no accumulation_plan is configured");
    assert_eq!(st(rc, "/tax_reserve_usd/is"), "stated, not a gate");

    let fc = &b["floor_check"];
    assert_eq!(fc["status"], "fits");
    assert_eq!(st(fc, "/lp_nock_budget"), "86.7891");
    assert_eq!(fc["max_usdc_within_budget"], Value::Null);
    assert_eq!(st(fc, "/floor_breach_effect/status"), "none");
}

#[test]
fn test_ladder_reserve_gate_follows_the_bottom_not_the_grid_depth() {
    // The same ladder read to -20% and to -60%: the gate is the bottom's reading either way.
    let shallow = sim(&[("usdc", "741"), ("low", "0.01"), ("high", "spot"), ("grid_to_pct", "-20")]);
    let deep = sim(&[("usdc", "741"), ("low", "0.01"), ("high", "spot"), ("grid_to_pct", "-60")]);
    for b in [&shallow, &deep] {
        assert_eq!(b["reserve_check"]["status"], "holds");
        assert_eq!(st(b, "/reserve_check/margin_usdc"), "25.99");
    }
    assert_eq!(shallow["reserve_check"]["existing_ranges_below_bottom"], Value::Null);
    assert_eq!(st(&shallow, "/reserve_check/existing_ranges_below_bottom_reason"), "the grid ends at or above the ladder's bottom");
    assert!(f(&deep, "/reserve_check/existing_ranges_below_bottom/headroom_usdc") < -19.31);
}

#[test]
fn test_ladder_floor_budget_gating() {
    // 4,194 NOCK against the 86.79 budget: exceeds; the most USDC that fits is linear in L.
    let b = sim(&CHAT);
    let fc = &b["floor_check"];
    assert_eq!(fc["status"], "exceeds");
    assert_eq!(st(fc, "/lp_nock_budget"), "86.7891");
    assert_eq!(st(fc, "/max_usdc_within_budget"), "18.62");
    assert_eq!(st(fc, "/rebuild_suggested_nock"), "0.0000");
    // At that USDC it fits.
    let fits = sim(&[("usdc", "18.62"), ("low_pct", "-30"), ("high_pct", "2")]);
    assert_eq!(fits["floor_check"]["status"], "fits");
    assert!(f(&fits, "/sizing/nock_needed_at_spot") <= 86.7891);
}

#[test]
fn test_ladder_negative_and_absent_budget() {
    // Wallet NOCK 50,000: 50,000 below the floor less the tolerance, a signed budget.
    let mut raw = raw_book(fixture_positions());
    raw.wallet_balances[1].amount = "50000".to_string();
    let env = envelope_of(raw, &profile(Some(950.0)));
    let gs = &env.data.derived_metrics.global_status;
    assert_eq!(gs.lp_nock_budget.as_deref(), Some("-40000.0000"));
    assert_eq!(
        gs.lp_nock_budget_note.as_deref(),
        Some("none: wallet NOCK is 40000 below the 100000 NOCK floor less the 10000 tolerated below it")
    );
    let b = sim_on(&env, &CHAT).unwrap();
    assert_eq!(b["floor_check"]["status"], "exceeds");
    assert_eq!(st(&b, "/floor_check/lp_nock_budget"), "-40000.0000");
    assert_eq!(st(&b, "/floor_check/max_usdc_within_budget"), "0.00");
    assert!(st(&b, "/floor_check/budget_note").starts_with("none: wallet NOCK is 40000 below"));
    // A ladder that needs no NOCK draws none of a negative budget.
    let z = sim_on(&env, &SPOT_741).unwrap();
    assert_eq!(z["floor_check"]["status"], "fits");
    // A snapshot from before the field: unknown, with the reason; never a default.
    let mut old = fixture();
    old.data.derived_metrics.global_status.lp_nock_budget = None;
    let b = sim_on(&old, &CHAT).unwrap();
    assert_eq!(b["floor_check"]["status"], "unknown");
    assert_eq!(b["floor_check"]["lp_nock_budget"], Value::Null);
    assert_eq!(st(&b, "/floor_check/reason"), "this snapshot predates lp_nock_budget (2.9.0)");
}

#[test]
fn test_lp_nock_budget_matches_the_rebuild_figure() {
    assert_eq!(lp_nock_budget(90086.7891, 100000.0, 1000), (90086.7891 - 100000.0 + 10000.0, 10000.0));
    assert_eq!(lp_nock_budget(5000.0, 0.0, 1000), (5000.0, 0.0));
    assert_eq!(lp_nock_budget_note(5000.0, 0.0, 1000).as_deref(), Some("no treasury_floor_nock is set: the budget is the wallet's NOCK"));
    assert_eq!(lp_nock_budget_note(90086.7891, 100000.0, 1000), None);
    // The rebuild's own reason states the same figure (87).
    let src = include_str!("advice.rs");
    assert!(src.contains("let (budget_nock, floor_tolerance_nock) = lp_nock_budget(wallet_nock, floor, profile.lp_floor_tolerance_bps);"));
}

#[test]
fn test_ladder_full_conversion_identity() {
    // avg buy = sqrt(a * min(P, b)) and nock_bought = usdc / avg, for ranges below and
    // across spot, unsnapped.
    for (low, high) in [("0.005", "0.012"), ("0.009", "0.0129"), ("0.01", "0.0135"), ("0.002", "0.02")] {
        let b = sim(&[("usdc", "500"), ("low", low), ("high", high)]);
        let (a, hi): (f64, f64) = (low.parse().unwrap(), high.parse().unwrap());
        let geo = (a * hi.min(0.012944)).sqrt();
        let avg = f(&b, "/bottom/avg_buy_price_usd");
        assert!(rel(avg, geo) < 1e-6, "{low}-{high}: {avg} vs {geo}");
        assert!(rel(f(&b, "/bottom/nock_bought"), 500.0 / geo) < 1e-6);
        // Fully converted at the bottom: no USDC left in the ladder there.
        let bottom = b["grid"].as_array().unwrap().iter().find(|r| r["markers"].as_array().unwrap().contains(&json!("bottom")));
        if let Some(r) = bottom {
            assert_eq!(st(r, "/ladder_usdc"), "0.00");
        }
    }
}

#[test]
fn test_ladder_nock_given_is_checked() {
    let e = err(&[("usdc", "741"), ("low", "0.01"), ("high", "spot"), ("nock", "50")]);
    assert_eq!((e.status, e.code), (422, Some("NOCK_TOO_MUCH")));
    let body = e.body();
    assert_eq!(body["nock_needed"], "0.0000");
    assert_eq!(body["error"], "LADDER_NOT_SIMULATED");
    assert_eq!(body["bounds"]["high_usd"], "0.01294400");
    let e = err(&[("usdc", "900"), ("low_pct", "-30"), ("high_pct", "2"), ("nock", "4000")]);
    assert_eq!(e.code, Some("NOCK_TOO_LITTLE"));
    assert_eq!(e.body()["nock_needed"], "4193.9773");
    // Within 1 NOCK (10 bps of 4,194 is 4.19 NOCK): matches.
    let b = sim(&[("usdc", "900"), ("low_pct", "-30"), ("high_pct", "2"), ("nock", "4194.5")]);
    assert_eq!(st(&b, "/sizing/nock_check"), "matches");
}

#[test]
fn test_ladder_validation_errors() {
    for (pairs, needle) in [
        (vec![("low", "0.01"), ("high", "spot")], "usdc is required"),
        (vec![("usdc", "-1"), ("low", "0.01"), ("high", "spot")], "usdc must be above 0"),
        (vec![("usdc", "0"), ("low", "0.01"), ("high", "spot")], "usdc must be above 0"),
        (vec![("usdc", "NaN"), ("low", "0.01"), ("high", "spot")], "usdc must be a finite number"),
        (vec![("usdc", "741"), ("low", "0"), ("high", "spot")], "low must be above 0"),
        (vec![("usdc", "741"), ("low", "0.012"), ("high", "0.011")], "must be below high"),
        (vec![("usdc", "741"), ("low_pct", "-100"), ("high", "spot")], "low_pct must be above -100"),
        (vec![("usdc", "741"), ("low", "0.01"), ("low_pct", "-20"), ("high", "spot")], "not both"),
        (vec![("usdc", "741"), ("high", "spot")], "low or low_pct is required"),
        (vec![("usdc", "741"), ("low", "0.01")], "high, high=spot or high_pct is required"),
        (vec![("usdc", "741"), ("low", "0.01"), ("high", "spot"), ("hi", "1")], "unknown parameter 'hi'"),
        (vec![("usdc", "741"), ("low", "0.01"), ("high", "spot"), ("tick_spacing", "0")], "tick_spacing must be"),
        (vec![("usdc", "741"), ("low", "0.01"), ("high", "spot"), ("tick_spacing", "300"), ("position_id", V4_ID)], "not both"),
        (vec![("usdc", "741"), ("low", "0.01"), ("high", "spot"), ("grid_step_pct", "0.5"), ("grid_to_pct", "-95")], "at most 60"),
        (vec![("usdc", "741"), ("low", "0.01"), ("high", "spot"), ("grid_to_pct", "-99")], "grid_to_pct must be"),
        (vec![("usdc", "741"), ("low", "0.01"), ("high", "spot"), ("position_id", "uniswap-v4:base:1")], "not a tracked position"),
    ] {
        let e = err(&pairs);
        assert_eq!((e.status, e.error), (400, "INVALID_PARAMETER"), "{pairs:?}");
        assert!(e.detail.contains(needle), "{pairs:?}: {}", e.detail);
    }
    // A mixed pair resolves first, then is compared.
    let e = err(&[("usdc", "741"), ("low", "0.0129"), ("high_pct", "-1")]);
    assert_eq!(e.status, 400);

    for (pairs, code) in [
        (vec![("usdc", "741"), ("low", "0.0130"), ("high", "0.014")], "RANGE_ABOVE_SPOT"),
        (vec![("usdc", "2000"), ("low", "0.01"), ("high", "spot")], "USDC_EXCEEDS_WALLET"),
        (vec![("usdc", "741"), ("low", "0.0127"), ("high", "0.0128"), ("position_id", V4_ID)], "RANGE_TOO_NARROW"),
        (vec![("usdc", "741"), ("low", "0.01"), ("high", "spot"), ("position_id", AERO_ID)], "POOL_NOT_SNAPPABLE"),
        (vec![("usdc", "741"), ("low", "0.01"), ("high", "spot"), ("position_id", "uniswap-v3:base:6607225")], "POOL_NOT_SNAPPABLE"),
    ] {
        let e = err(&pairs);
        assert_eq!((e.status, e.code), (422, Some(code)), "{pairs:?}: {}", e.detail);
    }
    let e = err(&[("usdc", "741"), ("low", "0.0127"), ("high", "0.0128"), ("position_id", V4_ID)]);
    assert!(e.detail.contains("both bounds snap to tick 273900") && e.detail.contains("3.05%"), "{}", e.detail);
    assert!(err(&[("usdc", "741"), ("low", "0.01"), ("high", "spot"), ("position_id", AERO_ID)]).detail.contains("basic Aero pool"));
    // A v4 position whose spacing was not read: stated, not guessed from the fee tier.
    let mut env = fixture();
    env.data.positions[0].tick_spacing = None;
    let e = sim_on(&env, &[("usdc", "741"), ("low", "0.01"), ("high", "spot"), ("position_id", V4_ID)]).unwrap_err();
    assert_eq!(e.code, Some("POOL_NOT_SNAPPABLE"));
    assert!(e.detail.contains("tick spacing not read"));
    // No price-source pool price: no exact spot, so nothing is simulated.
    let mut env = fixture();
    env.data.pool_mechanics[0].sqrt_price_x96 = None;
    let e = sim_on(&env, &SPOT_741).unwrap_err();
    assert_eq!(e.code, Some("SPOT_UNAVAILABLE"));
}

#[test]
fn test_ladder_snapped_top_at_spot_is_the_tick_at_or_below() {
    let b = sim(&[("usdc", "741"), ("low", "0.01"), ("high", "spot"), ("position_id", V4_ID)]);
    assert_eq!(b["pool"]["snapped"], json!(true));
    assert_eq!(b["pool"]["tick_spacing"], json!(300));
    assert_eq!(st(&b, "/pool/spacing_source"), "position");
    assert_eq!((b["bounds"]["tick_lower"].as_i64(), b["bounds"]["tick_upper"].as_i64()), (Some(273900), Some(276300)));
    assert_eq!(st(&b, "/bounds/high_usd"), "0.01274292");
    assert_eq!(st(&b, "/bounds/low_usd"), "0.01002405");
    assert_eq!(st(&b, "/bounds/high_pct"), "-1.55");
    assert_eq!(st(&b, "/sizing/nock_needed_at_spot"), "0.0000");
    assert!((f(&b, "/bottom/nock_held") - 65563.4914).abs() < 0.01);
    assert_eq!(st(&b, "/bottom/avg_buy_price_usd"), "0.01130202");
    assert_eq!(st(&b, "/reserve_check/stable_at_bottom_usdc"), "976.45");
    assert_eq!(st(&b, "/reserve_check/margin_usdc"), "26.45");
    assert_eq!(st(&b, "/reserve_check/max_usdc_for_this_range"), "767.45");
    // Both tops next to spot are listed with their NOCK need.
    let n = b["top_neighbours"].as_array().unwrap();
    assert_eq!((n[0]["tick"].as_i64(), st(&n[0], "/price_usd"), st(&n[0], "/pct_of_spot")), (Some(273900), "0.01274292", "-1.55"));
    assert_eq!(st(&n[0], "/nock_needed"), "0.0000");
    assert_eq!(n[0]["fits_budget"], json!(true));
    assert_eq!(n[0]["selected"], json!(true));
    assert_eq!((n[1]["tick"].as_i64(), st(&n[1], "/price_usd"), st(&n[1], "/pct_of_spot")), (Some(273600), "0.01313098", "+1.44"));
    assert!((f(&n[1], "/nock_needed") - 3408.9424).abs() < 0.05);
    assert_eq!(n[1]["fits_budget"], json!(false));
}

#[test]
fn test_ladder_snapped_numeric_top_uses_the_nearest_tick() {
    // An explicit top at the +1.44% tick: nearest usable tick, 3,409 NOCK, over budget.
    let b = sim(&[("usdc", "741"), ("low", "0.01"), ("high", "0.01313098"), ("position_id", V4_ID)]);
    assert_eq!(b["bounds"]["tick_lower"].as_i64(), Some(273600));
    assert!((f(&b, "/sizing/nock_needed_at_spot") - 3408.9424).abs() < 0.05);
    assert!((f(&b, "/bottom/nock_held") - 68461.1822).abs() < 0.05);
    assert!((f(&b, "/bottom/nock_bought") - 65052.2398).abs() < 0.05);
    assert_eq!(st(&b, "/bottom/avg_buy_price_usd"), "0.01139085");
    assert_eq!(b["floor_check"]["status"], "exceeds");
    assert_eq!(st(&b, "/floor_check/max_usdc_within_budget"), "18.86");

    // The Chat agent's case snapped: ticks 273600/277200.
    let b = sim(&[("usdc", "900"), ("low_pct", "-30"), ("high_pct", "2"), ("position_id", V4_ID)]);
    assert_eq!((b["bounds"]["tick_lower"].as_i64(), b["bounds"]["tick_upper"].as_i64()), (Some(273600), Some(277200)));
    assert_eq!(st(&b, "/bounds/low_usd"), "0.00916134");
    assert!((f(&b, "/sizing/nock_needed_at_spot") - 3130.2784).abs() < 0.01);
    assert!((f(&b, "/bottom/nock_held") - 85777.6089).abs() < 0.01);
    assert_eq!(st(&b, "/bottom/avg_buy_price_usd"), "0.01088964");
    assert_eq!(st(&b, "/reserve_check/stable_at_bottom_usdc"), "800.64");
    assert_eq!(b["reserve_check"]["status"], "breached");
}

#[test]
fn test_ladder_caller_tick_spacing_and_unsnapped_spot() {
    // A spacing the caller states snaps the same way, sized at the price-source price.
    let b = sim(&[("usdc", "741"), ("low", "0.01"), ("high", "spot"), ("tick_spacing", "300")]);
    assert_eq!(st(&b, "/pool/spacing_source"), "caller");
    assert_eq!(b["pool"]["position_id"], Value::Null);
    assert_eq!(b["bounds"]["tick_lower"].as_i64(), Some(273900));
    assert_eq!(st(&b, "/bottom/avg_buy_price_usd"), "0.01130202");
    assert!(st(&b, "/pool/note").contains("tick spacing you gave (300)"));
    // A finer spacing: the at-or-below tick of spacing 60 is 273780 (still no NOCK).
    let b = sim(&[("usdc", "741"), ("low", "0.01"), ("high", "spot"), ("tick_spacing", "60")]);
    assert_eq!(b["bounds"]["tick_lower"].as_i64(), Some(273780));
    assert_eq!(st(&b, "/sizing/nock_needed_at_spot"), "0.0000");
    // Unsnapped, high=spot is spot itself.
    let b = sim(&SPOT_741);
    assert_eq!(st(&b, "/bounds/high_usd"), "0.01294400");
    assert_eq!(st(&b, "/bounds/high_rule"), "unsnapped");
}

#[test]
fn test_ladder_floor_breach_effect_with_and_without_nock_at_the_top() {
    // Without NOCK at spot the ladder sells nothing on a rise: none, before == after.
    let b = sim(&[("usdc", "741"), ("low", "0.01"), ("high", "spot"), ("position_id", V4_ID)]);
    let e = &b["floor_check"]["floor_breach_effect"];
    assert_eq!(e["status"], "none");
    assert_eq!(e["before"]["crossing"], e["after"]["crossing"]);
    assert_eq!(st(e, "/before/crossing/price_usd"), "0.025404");
    assert_eq!(st(e, "/before/crossing/pct_above_spot"), "96.3");
    // "before" is the snapshot's own floor-crossing figure.
    let stored = b["floor_check"]["floor_breach_effect"]["before"]["crossing"].clone();
    assert_eq!(stored, serde_json::to_value(&fixture().data.derived_metrics.floor_breach.unwrap().crossing).unwrap());

    // The Chat agent's case snapped: 3,130 NOCK leaves the wallet into a range that has
    // sold it all at +1.44%: the hard floor is crossed at x1.5329 instead of x1.9626.
    let b = sim(&[("usdc", "900"), ("low_pct", "-30"), ("high_pct", "2"), ("position_id", V4_ID)]);
    let e = &b["floor_check"]["floor_breach_effect"];
    assert_eq!(e["status"], "lowers");
    assert_eq!(st(e, "/before/crossing/multiple"), "1.9626");
    assert_eq!(st(e, "/after/crossing/multiple"), "1.5329");
    assert_eq!(st(e, "/after/crossing/price_usd"), "0.019842");
    // The tolerated level (90,000): never before (wallet + fees are above it), x8.2364 after.
    assert_eq!(st(e, "/before/tolerated/status"), "never");
    assert_eq!(st(e, "/after/tolerated/multiple"), "8.2364");
    // Unsnapped, the bounds are rounded to whole ticks for this figure, and it says so.
    let b = sim(&CHAT);
    let e = &b["floor_check"]["floor_breach_effect"];
    assert_eq!(e["status"], "lowers");
    assert!(st(e, "/assumption").contains("rounded to whole ticks"));
}

#[test]
fn test_ladder_row0_is_the_snapshot_and_total_nock_matches() {
    let env = fixture();
    let b = sim(&CHAT);
    let r0 = grid_row(&b, "0.00");
    assert!(r0["markers"].as_array().unwrap().contains(&json!("spot")));
    // stable at spot = wallet 1545.8357 + v4 112.3395 + Aero 111.3673, the reserve measure.
    assert_eq!(st(r0, "/stable_usdc"), env.data.derived_metrics.global_status.reserve_stable_usdc.as_deref().unwrap());
    assert_eq!(st(r0, "/stable_usdc"), "1769.54");
    // total_nock_amount is {:.0} (109542.50 prints 109543 there).
    let total: f64 = env.data.derived_metrics.global_status.total_nock_amount.parse().unwrap();
    assert!((f(r0, "/total_tracked_nock") - total).abs() <= 0.5, "{}", r0);
    assert_eq!(st(r0, "/total_tracked_nock"), "109542.50");
    // The existing ranges at r = 1 are the snapshot's own amounts.
    let usdc_in_snapshot: f64 = env.data.positions[..2].iter().map(|p| p.amount0.parse::<f64>().unwrap()).sum();
    assert!((f(r0, "/existing_nock_lp_usdc") - usdc_in_snapshot).abs() < 0.006);
}

#[test]
fn test_ladder_full_range_pool_moves_from_its_own_price() {
    // The Aero pool at +1.0% of spot (the live gap, 0.0130762 vs 0.0129445): it moves by
    // the same % as spot from its own price, so its USDC at the $0.01 bottom is
    // 111.367340 * sqrt(0.01 / 0.012944) = 97.8867, not sqrt(k * 0.01) = 97.4009.
    let aero_nock = 111.367340 / (1.01 * 0.012944);
    let env = envelope_of(raw_book(vec![v4(), aero(111.367340, aero_nock)]), &profile(Some(950.0)));
    let aero_px = 111.367340 / aero_nock;
    assert!((aero_px / 0.012944 - 1.01).abs() < 1e-9);
    let b = sim_on(&env, &SPOT_741).unwrap();
    assert_eq!(st(&b, "/reserve_check/existing_ranges_usdc_at_bottom"), "171.16");
    assert_eq!(st(&b, "/reserve_check/stable_at_bottom_usdc"), "975.99");
    assert_eq!(st(&b, "/reserve_check/max_usdc_for_this_range"), "766.99");
    assert_eq!(st(grid_row(&b, "0.00"), "/stable_usdc"), "1769.54");
    // The pool's own price is stated beside spot.
    let aero_raw = stored_book(&env.data).raw.positions.into_iter().find(|p| p.position_id == AERO_ID).unwrap();
    assert!(rel(existing_at_ratio(&aero_raw, 0.01 / 0.012944).0, 97.8867) < 1e-5);
}

#[test]
fn test_ladder_ignores_gas_side_ranges() {
    let with = sim(&CHAT);
    let env = envelope_of(raw_book(vec![v4(), aero(111.367340, 8408.058135)]), &profile(Some(950.0)));
    let without = sim_on(&env, &CHAT).unwrap();
    for k in ["grid", "bottom", "top", "reserve_check", "floor_check", "sizing", "bounds"] {
        assert_eq!(with[k], without[k], "{k}");
    }
}

#[test]
fn test_ladder_reserve_unset_invalid_and_unknown() {
    let env = envelope_of(raw_book(fixture_positions()), &profile(None));
    let b = sim_on(&env, &SPOT_741).unwrap();
    let rc = &b["reserve_check"];
    assert_eq!(rc["status"], "not_configured");
    assert_eq!(st(rc, "/reason"), "operator_profile.cash_reserve_usdc is not set: no reserve rule");
    assert_eq!(rc["margin_usdc"], Value::Null);
    assert_eq!(rc["max_usdc_for_this_range"], Value::Null);
    assert_eq!(grid_row(&b, "0.00")["reserve_headroom_usdc"], Value::Null);
    // The stable balance is still a measurement.
    assert_eq!(st(rc, "/stable_at_bottom_usdc"), "975.99");

    let env = envelope_of(raw_book(fixture_positions()), &profile(Some(-5.0)));
    let b = sim_on(&env, &SPOT_741).unwrap();
    assert_eq!(b["reserve_check"]["status"], "invalid");
    assert_eq!(st(&b, "/reserve_check/reason"), RESERVE_INVALID_NOTE);
    assert_eq!(b["reserve_check"]["margin_usdc"], Value::Null);

    // The Aero pool's price missing from the snapshot: unknown, naming the position.
    let mut env = fixture();
    env.data.pool_mechanics[1].sqrt_price_x96 = None;
    let b = sim_on(&env, &SPOT_741).unwrap();
    assert_eq!(b["reserve_check"]["status"], "unknown");
    assert!(st(&b, "/reserve_check/reason").contains(AERO_ID));
    assert_eq!(grid_row(&b, "0.00")["stable_usdc"], Value::Null);
    assert_eq!(st(&b, "/floor_check/floor_breach_effect/status"), "unknown");
}

#[test]
fn test_ladder_states_snapshot_age_and_health() {
    let b = sim(&SPOT_741);
    assert_eq!(st(&b, "/snapshot/generated_at"), "2026-11-20T12:00:00Z");
    assert_eq!(b["snapshot"]["age_minutes"], json!(5));
    assert_eq!(b["snapshot"]["age_seconds"], json!(300));
    assert_eq!(st(&b, "/snapshot/worker_health"), "ok");
    assert_eq!(st(&b, "/statement"), LADDER_STATEMENT);
    assert!(st(&b, "/snapshot/spot_source").contains("v4 USDC/NOCK 3.00% #3163450"));
}

#[test]
fn test_ladder_text_is_measurement_only() {
    let text = ladder_text::ladder_text(&sim(&[("usdc", "741"), ("low", "0.01"), ("high", "spot"), ("position_id", V4_ID)]));
    assert!(text.starts_with("🧮 Ladder Simulation (what-if, read-only)\n"), "{text}");
    for needle in [
        "• Range: $0.01002405 – $0.01274292 (-22.56% / -1.55% of the pool price $0.01294400), snapped to v4 USDC/NOCK 3.00% #3163450 (tick spacing 300;",
        "• Tops next to spot: $0.01274292 (-1.55%) needs 0 NOCK (fits the budget); $0.01313098 (+1.44%) needs 3,409 NOCK (exceeds the budget)",
        "• Bottom: fully NOCK at $0.01002405 (-22.56%): 65,563 NOCK held, 65,563 bought, average $0.01130202",
        "• Reserve: stable $976.45 at the bottom against the $950.00 reserve: holds by $26.45 (this range takes at most $767.45;",
        "• Floor budget: needs 0 of the 87 NOCK that may go into LP: fits.",
        "• Floor-crossing price: none: the ladder holds no NOCK at or above spot",
        "Snapshot generated 2026-11-20T12:00:00Z (5 min ago) · worker health: ok",
    ] {
        assert!(text.contains(needle), "missing {needle:?} in\n{text}");
    }
    let lower = text.to_lowercase();
    for word in ["deposit", "should", "recommend"] {
        assert!(!lower.contains(word), "{word} in\n{text}");
    }
    // A breached reserve is a measured fact.
    let text = ladder_text::ladder_text(&sim(&CHAT));
    assert!(text.contains("breached: below it by $151.37"), "{text}");
    assert!(text.contains("$18.62 is the most USDC this range holds within it"), "{text}");
    let e = ladder_text::ladder_error_text(&err(&[("usdc", "741"), ("low", "0.01"), ("high", "spot"), ("nock", "50")]).body());
    assert!(e.contains("• NOCK_TOO_MUCH: "), "{e}");
    assert!(e.contains("• Measured: this range needs 0.0000 NOCK at the pool price for this USDC"), "{e}");
}

#[test]
fn test_ladder_mcp_arguments() {
    let q = ladder_text::ladder_query_from_args(Some(&json!({"usdc": 741, "low": 0.01, "high_at_spot": true}))).unwrap();
    assert_eq!(q, vec![("usdc".into(), "741".into()), ("low".into(), "0.01".into()), ("high".into(), "spot".into())]);
    // Numeric strings pass; a number and high_at_spot together do not.
    assert!(ladder_text::ladder_query_from_args(Some(&json!({"usdc": "741", "low": "0.01", "high": 0.0128}))).is_ok());
    let e = ladder_text::ladder_query_from_args(Some(&json!({"usdc": 741, "high": 0.0128, "high_at_spot": true}))).unwrap_err();
    assert_eq!(e["error"], "INVALID_PARAMETER");
    assert!(ladder_text::ladder_query_from_args(Some(&json!({"usdc": [1]}))).is_err());
    assert!(ladder_text::ladder_query_from_args(Some(&json!({"bogus": 1}))).is_err());
    assert!(ladder_text::ladder_query_from_args(Some(&json!(5))).is_err());
    // The tool runs the same simulation and says when it is an error.
    let env = fixture();
    let now = generated_at() + chrono::Duration::minutes(5);
    let (text, is_error) = simulate_ladder_mcp(Some(&json!({"usdc": 741, "low": 0.01, "high_at_spot": true})), &env, now, "ok");
    assert!(!is_error);
    assert_eq!(text, ladder_text::ladder_text(&sim(&SPOT_741)));
    let (text, is_error) = simulate_ladder_mcp(Some(&json!({"usdc": 741, "low": 0.013, "high": 0.014})), &env, now, "ok");
    assert!(is_error);
    assert!(text.contains("RANGE_ABOVE_SPOT"), "{text}");
    // The schema has no union types.
    let tool = ladder_text::simulate_ladder_tool();
    for (k, v) in tool["inputSchema"]["properties"].as_object().unwrap() {
        assert!(v["type"].is_string(), "{k} has a union type");
    }
    assert_eq!(tool["inputSchema"]["properties"]["high_at_spot"]["type"], "boolean");
}

fn offline_state(snapshot: Option<SnapshotEnvelope>) -> Arc<AppState> {
    let config: AppConfig = serde_yaml::from_str("alchemy_api_keys: [\"YOUR_ALCHEMY_API_KEY\"]\n").unwrap();
    Arc::new(AppState {
        // Unreachable, and given up on quickly: the no-snapshot case tries the database.
        db: sqlx::postgres::PgPoolOptions::new()
            .acquire_timeout(std::time::Duration::from_millis(300))
            .connect_lazy("postgres://nobody@127.0.0.1:1/none")
            .unwrap(),
        http_client: Client::new(),
        config: Arc::new(config),
        latest_snapshot: Mutex::new(snapshot),
        last_sync_duration_ms: std::sync::atomic::AtomicU64::new(0),
        last_ingest_error: std::sync::Mutex::new(None),
        cycle_gate: CycleGate::default(),
        quote_refresh_at: std::sync::Mutex::new(None),
    })
}

async fn serve(state: Arc<AppState>) -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, api_router(state)).await.unwrap() });
    format!("http://{addr}")
}

#[tokio::test]
async fn test_ladder_endpoint_is_a_read_and_refuses_writes() {
    // Two days old, so the worker's health verdict is deterministically "degraded"
    // (past the sync window) and the age is 2,880 minutes.
    let mut env = fixture();
    let generated = Utc::now() - chrono::Duration::days(2);
    let generated = DateTime::parse_from_rfc3339(&generated.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)).unwrap().with_timezone(&Utc);
    env.meta.generated_at = generated;
    let base = serve(offline_state(Some(env))).await;
    let client = Client::new();
    let url = format!("{base}/v1/simulate/ladder?usdc=741&low=0.01&high=spot");
    // A cross-site GET is a read: it answers.
    let r = client
        .get(&url)
        .header("Origin", "https://evil.example")
        .header("Sec-Fetch-Site", "cross-site")
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), reqwest::StatusCode::OK);
    let body: Value = r.json().await.unwrap();
    assert_eq!(body["bottom"]["avg_buy_price_usd"], "0.01137717");
    assert_eq!(body["reserve_check"]["status"], "holds");
    // A cross-site POST is refused by the guard before any handler.
    let r = client.post(&url).header("Origin", "https://evil.example").header("Sec-Fetch-Site", "cross-site").send().await.unwrap();
    assert_eq!(r.status(), reqwest::StatusCode::FORBIDDEN);
    assert_eq!(r.json::<Value>().await.unwrap()["error"], "CROSS_SITE_REFUSED");
    // A POST from curl reaches the router: the route has no write method.
    let r = client.post(&url).send().await.unwrap();
    assert_eq!(r.status(), reqwest::StatusCode::METHOD_NOT_ALLOWED);
    // 400 and 422 bodies.
    let r = client.get(format!("{base}/v1/simulate/ladder?usdc=741&low=0&high=spot")).send().await.unwrap();
    assert_eq!(r.status(), reqwest::StatusCode::BAD_REQUEST);
    assert_eq!(r.json::<Value>().await.unwrap()["error"], "INVALID_PARAMETER");
    let r = client.get(format!("{base}/v1/simulate/ladder?usdc=741&low=0.01&high=spot&nock=50")).send().await.unwrap();
    assert_eq!(r.status(), reqwest::StatusCode::UNPROCESSABLE_ENTITY);
    let e: Value = r.json().await.unwrap();
    assert_eq!((e["code"].as_str(), e["nock_needed"].as_str()), (Some("NOCK_TOO_MUCH"), Some("0.0000")));
    // The HTTP MCP tool, with arguments and isError.
    let call = |args: Value| json!({"jsonrpc": "2.0", "id": 7, "method": "tools/call", "params": {"name": "simulate_ladder", "arguments": args}});
    let r: Value = client.post(format!("{base}/v1/mcp")).json(&call(json!({"usdc": 741, "low": 0.01, "high_at_spot": true}))).send().await.unwrap().json().await.unwrap();
    assert_eq!(r["result"]["isError"], json!(false));
    let text = r["result"]["content"][0]["text"].as_str().unwrap();
    assert!(text.contains("average $0.01137717"), "{text}");
    // The source line comes from the body's own snapshot block: its time, its age and
    // the worker's health, in the words of the other tools' footer.
    let gen = generated.to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    let last = text.lines().last().unwrap();
    assert!(
        last == format!("Snapshot generated {gen} (2880 min ago) · worker health: degraded")
            || last == format!("Snapshot generated {gen} (2881 min ago) · worker health: degraded"),
        "{last}"
    );
    let r: Value = client.post(format!("{base}/v1/mcp")).json(&call(json!({"usdc": 741, "low": 0.01}))).send().await.unwrap().json().await.unwrap();
    assert_eq!(r["result"]["isError"], json!(true));
    // tools/list carries the shared definition.
    let r: Value = client.post(format!("{base}/v1/mcp")).json(&json!({"jsonrpc": "2.0", "id": 1, "method": "tools/list"})).send().await.unwrap().json().await.unwrap();
    let tools = r["result"]["tools"].as_array().unwrap();
    assert_eq!(tools.iter().find(|t| t["name"] == "simulate_ladder").unwrap(), &ladder_text::simulate_ladder_tool());
}

#[tokio::test]
async fn test_ladder_endpoint_without_a_snapshot() {
    let base = serve(offline_state(None)).await;
    let r = Client::new().get(format!("{base}/v1/simulate/ladder?usdc=741&low=0.01&high=spot")).send().await.unwrap();
    assert_eq!(r.status(), reqwest::StatusCode::SERVICE_UNAVAILABLE);
    assert_eq!(r.json::<Value>().await.unwrap()["error"], "NO_SNAPSHOT");
    let r: Value = Client::new()
        .post(format!("{base}/v1/mcp"))
        .json(&json!({"jsonrpc": "2.0", "id": 7, "method": "tools/call", "params": {"name": "simulate_ladder", "arguments": {"usdc": 1}}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(r["result"]["isError"], json!(true));
    assert!(r["result"]["content"][0]["text"].as_str().unwrap().starts_with("No snapshot available yet"));
}

#[test]
fn test_ladder_rebuilt_book_reproduces_every_golden_floor_breach() {
    // The stored book, rebuilt from the snapshot alone, gives back the snapshot's own
    // floor-crossing figure in every golden case: the "before" the ladder compares with.
    let mut checked: Vec<String> = Vec::new();
    for (file, src) in [
        ("golden_snapshot.json", include_str!("../testdata/golden_snapshot.json")),
        ("golden_snapshot_aero_cl.json", include_str!("../testdata/golden_snapshot_aero_cl.json")),
        ("golden_snapshot_aero_rewards.json", include_str!("../testdata/golden_snapshot_aero_rewards.json")),
        ("golden_snapshot_closed.json", include_str!("../testdata/golden_snapshot_closed.json")),
        ("golden_snapshot_rewards.json", include_str!("../testdata/golden_snapshot_rewards.json")),
    ] {
        let golden: Value = serde_json::from_str(src).unwrap();
        for (case, v) in golden.as_object().unwrap() {
            // The golden "snapshot" is the canonical state (volatile times stripped), so
            // only the arrays the book is rebuilt from are read.
            let snap = &v["snapshot"];
            let positions: Vec<Position> = serde_json::from_value(snap["positions"].clone()).unwrap();
            let pools: Vec<PoolMechanic> = serde_json::from_value(snap["pool_mechanics"].clone()).unwrap();
            let mut balances = snap["wallet_balances"].clone();
            for b in balances.as_array_mut().unwrap() {
                b["updated_at"] = json!("");
            }
            let balances: Vec<WalletBalance> = serde_json::from_value(balances).unwrap();
            let Some(stored) = serde_json::from_value::<Option<FloorBreach>>(snap["derived_metrics"]["floor_breach"].clone()).unwrap() else { continue };
            let bps = snap["derived_metrics"]["global_status"]["lp_floor_tolerance_bps"].as_u64().expect("2.9.0") as u32;
            let floor: f64 = stored.floor_nock.parse().unwrap();
            // A case with no live price-source position has no exact spot: the ladder
            // answers SPOT_UNAVAILABLE there and never reaches this figure.
            if !positions.iter().any(|p| p.is_price_source) {
                continue;
            }
            let book = stored_book_from(&positions, &pools, &balances);
            assert!(book.problems.is_empty(), "{file}:{case}: {:?}", book.problems);
            let profile = OperatorProfile { treasury_floor_nock: floor, lp_floor_tolerance_bps: bps, ..Default::default() };
            let again = floor_breach_with(&book.raw, &profile, &[]);
            assert_eq!(again.crossing, stored.crossing, "{file}:{case}: {:?} {:?}", again.status, again.reason);
            assert_eq!(again.tolerated, stored.tolerated, "{file}:{case}");
            assert_eq!(again.status, stored.status, "{file}:{case}");
            checked.push(stored.status.clone());
        }
    }
    assert!(checked.len() >= 3, "{checked:?}");
    assert!(checked.iter().any(|s| s == "crosses"), "{checked:?}");
}

#[test]
fn test_v4_pool_key_tick_spacing_from_the_recorded_answers() {
    // The recorded poolKeys answers of the cycle fixture: the 3% pool (fee 30000,
    // spacing 300) and any other v4 pool it holds; the spacing is word 3.
    let fixture: Value = serde_json::from_str(include_str!("../testdata/closed/cycle_fixture.json")).unwrap();
    let mut seen = Vec::new();
    let mut walk = vec![&fixture];
    while let Some(v) = walk.pop() {
        match v {
            Value::String(s) => {
                let clean = s.trim_start_matches("0x");
                if clean.len() == 320 && clean.get(128..192).map_or(false, |w| w.trim_start_matches('0') == "7530") {
                    seen.push(v4_pool_key_fee_and_spacing(clean));
                }
            }
            Value::Array(a) => walk.extend(a.iter()),
            Value::Object(o) => walk.extend(o.values()),
            _ => {}
        }
    }
    assert!(!seen.is_empty(), "the fixture holds the 3% pool's poolKeys answer");
    assert!(seen.iter().all(|s| *s == (300, Some(300))), "{seen:?}");
    // A short answer has no spacing: unknown, never derived from the fee.
    assert_eq!(v4_pool_key_fee_and_spacing(&"0".repeat(160)).1, None);
}

// Regenerate only for an intended change:
//   NOCKNOCK_BLESS_GOLDEN=1 cargo test test_golden_ladder
// The stdio MCP (mcp_bin_tests_ladder.rs) and the dashboard (frontend/src/ladder.test.ts)
// read these bodies and texts, so all three surfaces are pinned to the worker's output.
#[test]
fn test_golden_ladder_is_byte_identical() {
    let env = fixture();
    let now = generated_at() + chrono::Duration::minutes(5);
    let ok = |pairs: &[(&str, &str)]| {
        let body = sim(pairs);
        let args: serde_json::Map<String, Value> = pairs.iter().map(|(k, v)| (k.to_string(), json!(v))).collect();
        let (text, is_error) = simulate_ladder_mcp(Some(&Value::Object(args.clone())), &env, now, "ok");
        assert!(!is_error);
        json!({ "query": args, "status": 200, "body": body, "mcp_text": text })
    };
    let bad = |pairs: &[(&str, &str)]| {
        let e = sim_on(&env, pairs).unwrap_err();
        let args: serde_json::Map<String, Value> = pairs.iter().map(|(k, v)| (k.to_string(), json!(v))).collect();
        let (text, is_error) = simulate_ladder_mcp(Some(&Value::Object(args.clone())), &env, now, "ok");
        assert!(is_error);
        json!({ "query": args, "status": e.status, "body": e.body(), "mcp_text": text })
    };
    let doc = json!({
        "about": "GET /v1/simulate/ladder bodies and the simulate_ladder MCP text over the #166 fixture (tests_ladder.rs): spot $0.012944, wallet USDC 1545.8357 and NOCK 90086.7891, floor 100000 with 1000 bps, reserve $950.",
        "tool": ladder_text::simulate_ladder_tool(),
        "spot_741_snapped": ok(&[("usdc", "741"), ("low", "0.01"), ("high", "spot"), ("position_id", V4_ID)]),
        "spot_741_unsnapped": ok(&SPOT_741),
        "chat_900_unsnapped": ok(&CHAT),
        "chat_900_snapped": ok(&[("usdc", "900"), ("low_pct", "-30"), ("high_pct", "2"), ("position_id", V4_ID)]),
        "nock_too_much": bad(&[("usdc", "741"), ("low", "0.01"), ("high", "spot"), ("nock", "50")]),
        "range_too_narrow": bad(&[("usdc", "741"), ("low", "0.0127"), ("high", "0.0128"), ("position_id", V4_ID)]),
        "invalid_low": bad(&[("usdc", "741"), ("low", "0"), ("high", "spot")]),
    });
    let actual = format!("{}\n", serde_json::to_string_pretty(&doc).unwrap());
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/ladder/golden_ladder.json");
    if std::env::var("NOCKNOCK_BLESS_GOLDEN").is_ok() {
        std::fs::create_dir_all(concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/ladder")).unwrap();
        std::fs::write(path, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(path).expect("testdata/ladder/golden_ladder.json is checked in");
    assert!(actual == expected, "ladder bytes changed; compare {path} with a fresh NOCKNOCK_BLESS_GOLDEN=1 run and `git diff` it");
}

#[test]
fn test_ladder_specs_document_the_endpoint_and_the_new_fields() {
    let api: serde_yaml::Value = serde_yaml::from_str(include_str!("../../specs/core/api.yaml")).expect("api.yaml parses");
    // Paths are relative to the /v1 server URL.
    let get = &api["paths"]["/simulate/ladder"]["get"];
    assert!(get.is_mapping(), "specs/core/api.yaml has /simulate/ladder");
    for code in ["200", "400", "422", "503"] {
        assert!(get["responses"][code].is_mapping(), "{code}");
    }
    let schemas = &api["components"]["schemas"];
    // Every top-level key the worker returns is in the schema.
    let body = sim(&SPOT_741);
    for key in body.as_object().unwrap().keys() {
        assert!(schemas["LadderSimulation"]["properties"][key.as_str()].is_mapping(), "LadderSimulation.{key}");
    }
    for key in body["reserve_check"].as_object().unwrap().keys() {
        assert!(schemas["LadderSimulation"]["properties"]["reserve_check"]["properties"][key.as_str()].is_mapping(), "reserve_check.{key}");
    }
    for key in body["grid"][0].as_object().unwrap().keys() {
        assert!(schemas["LadderGridRow"]["properties"][key.as_str()].is_mapping(), "LadderGridRow.{key}");
    }
    let snap: serde_yaml::Value = serde_yaml::from_str(include_str!("../../specs/core/snapshot.yaml")).expect("snapshot.yaml parses");
    let gs = &snap["components"]["schemas"]["GlobalStatus"]["properties"];
    for f in ["lp_nock_budget", "lp_nock_budget_note", "lp_floor_tolerance_bps"] {
        assert!(gs[f].is_mapping(), "GlobalStatus.{f}");
    }
    assert!(snap["components"]["schemas"]["Position"]["properties"]["tick_spacing"]["description"].as_str().unwrap().contains("uniswap-v4"));
}


/// The fixture with the price-source pool at another sqrtPriceX96 (#166 review).
fn fixture_at_sqrt(sqrt: U256) -> SnapshotEnvelope {
    let mut v = v4();
    v.sqrt_price_x96 = sqrt;
    v.tick = (sqrt_price_x96_to_raw(sqrt).ln() / 1.0001f64.ln()).floor() as i32;
    let v = with_amounts(RawPosition { fee0: 0.213182, fee1: 113.759403, ..v });
    let mut gas2 = gas_range_v3();
    gas2.position_id = "uniswap-v3:base:6607226".to_string();
    envelope_of(raw_book(vec![v, aero(111.367340, 8408.058135), gas_range_v3(), gas2]), &profile(Some(950.0)))
}

#[test]
fn test_ladder_unsnapped_spot_top_holds_no_dust_nock() {
    // At this sqrtPriceX96 (spot $0.00945387) sqrt(usd_to_raw(spot)) lands one ulp below
    // the pool's own sqrt; before the fix n0 came out ~5e-11 NOCK, avg_sell_price_usd
    // printed 0.01465134 and the floor-crossing effect was not "none".
    let env = fixture_at_sqrt(U256::from_hex_be(&format!("{:x}", 81484448971563975728368401080474936u128)).unwrap());
    for pairs in [
        vec![("usdc", "741"), ("low_pct", "-30"), ("high", "spot")],
        vec![("usdc", "741"), ("low_pct", "-30"), ("high_pct", "0")],
    ] {
        let b = sim_on(&env, &pairs).unwrap();
        assert_eq!(st(&b, "/snapshot/spot_usd"), "0.00945387");
        assert_eq!(st(&b, "/sizing/nock_needed_at_spot"), "0.0000", "{pairs:?}");
        assert_eq!(b["top"]["avg_sell_price_usd"], Value::Null, "{pairs:?}");
        assert_eq!(b["top"]["out_of_range_above_now"], json!(true));
        assert_eq!(st(&b, "/floor_check/floor_breach_effect/status"), "none", "{pairs:?}");
    }
    assert!(LADDER_NOCK_DUST <= 1e-9);
}

#[test]
fn test_ladder_floor_effect_conserves_nock_at_spot() {
    // Adding the ladder moves NOCK from the wallet into the range: tracked NOCK at spot
    // (m = 1) is the same before and after, for unsnapped (rounded-tick) and snapped
    // ranges. Before the fix the unrounded n0 left the wallet while the rounded range
    // held more (+8 NOCK for the Chat case, +10 for a spot top).
    for pairs in [
        CHAT.to_vec(),
        SPOT_741.to_vec(),
        vec![("usdc", "50"), ("low", "0.012"), ("high", "0.0135")],
        vec![("usdc", "900"), ("low_pct", "-30"), ("high_pct", "2"), ("position_id", V4_ID)],
    ] {
        let b = sim(&pairs);
        let e = &b["floor_check"]["floor_breach_effect"];
        assert_eq!(st(e, "/before/nock_now"), "109543", "{pairs:?}");
        assert_eq!(e["before"]["nock_now"], e["after"]["nock_now"], "{pairs:?}");
    }
    // A spot top rounds toward the side that holds no NOCK: "none", the same crossing.
    let b = sim(&SPOT_741);
    let e = &b["floor_check"]["floor_breach_effect"];
    assert_eq!(e["status"], "none");
    assert_eq!(e["before"]["crossing"], e["after"]["crossing"]);
}

#[test]
fn test_ladder_rejects_bounds_outside_the_tick_range() {
    // A bottom below the lowest price any tick can hold (1e10 * 1.0001^-887272 ≈ 3e-29).
    let e = err(&[("usdc", "741"), ("low", "1e-30"), ("high", "spot")]);
    assert_eq!((e.status, e.code), (422, Some("RANGE_OUTSIDE_TICKS")), "{}", e.detail);
    assert!(e.detail.contains("outside the pool's tick range [-887272, 887272]"), "{}", e.detail);
    // A range inside it is simulated.
    assert!(sim_on(&fixture(), &[("usdc", "741"), ("low", "1e-20"), ("high", "spot")]).is_ok());
    let api = include_str!("../../specs/core/api.yaml");
    assert!(api.contains("RANGE_OUTSIDE_TICKS"));
}
