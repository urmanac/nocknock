//! The floor-crossing price (issue #165): how far NOCK can rise before the deployed
//! ranges sell the stack below the floor. Every fixture is built from position amounts;
//! nothing here touches the network or needs a database or a config.yaml.
use super::*;
use crate::tests_gas_lp::{gas_range_v3, nock, nock_usdc_wide, usdc, weth_nock_v3, with_amounts};

const WALLET: &str = "0xee114cc97ec5adeb34ba5645e315b497d8487a45";
const V4_LABEL: &str = "v4 USDC/NOCK 3.00% #3163450";
const AERO_LABEL: &str = "Aero USDC/NOCK 0.30% (full range)";

fn rel(a: f64, b: f64) -> f64 {
    ((a - b) / b).abs()
}

/// The v4 USDC/NOCK 3% price-source range #3163450 as read in the snapshot of
/// 2026-11-21T02:37:59Z: liquidity, sqrtPriceX96, ticks and uncollected NOCK fees.
fn live_v4() -> RawPosition {
    let mut p = with_amounts(RawPosition {
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
        liquidity: 283687495614364,
        sqrt_price_x96: U256::from_hex_be("0xd6958383688afeeb0f34879e7e2c7").unwrap(),
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

/// An Aero basic (full-range) USDC/NOCK position holding `nock` NOCK.
fn aero_basic(usdc_amount: f64, nock_amount: f64) -> RawPosition {
    RawPosition {
        position_id: "aerodrome:base:0x770fe8fc9248c6382950a86e39a9391b41d59e0a".to_string(),
        network: "base".to_string(),
        protocol: "aerodrome".to_string(),
        wallet: WALLET.to_string(),
        nft_id: 0,
        pool_id: "0x770fe8fc9248c6382950a86e39a9391b41d59e0a".to_string(),
        npm_address: None,
        fee_tier_bps: 30,
        token0: usdc(),
        token1: nock(),
        // The wallet's LP-token balance, as aerodrome_position_from_reads sets it.
        liquidity: 96764372596543,
        sqrt_price_x96: U256::from_hex_be("0xd422243ac67ba0000000000000000").unwrap(),
        tick: 273513,
        tick_lower: MIN_TICK,
        tick_upper: MAX_TICK,
        amount0: usdc_amount,
        amount1: nock_amount,
        fee0: 0.0,
        fee1: 0.0,
        in_range: true,
        is_price_source: false,
    }
}

fn nock_row(amount: &str) -> WalletBalance {
    WalletBalance {
        wallet_address: WALLET.to_string(),
        token_symbol: "NOCK".to_string(),
        token_address: NOCK_BASE.to_string(),
        amount: amount.to_string(),
        amount_usd: "0".to_string(),
        updated_at: "now".to_string(),
    }
}

/// Positions, wallet NOCK and pool activity; spot from the price-source range when there
/// is one, else the live spot.
fn raw_of(positions: Vec<RawPosition>, wallet_nock: &str) -> RawOnChainData {
    let mut raw = RawOnChainData {
        spot_price_nock_usd: nock_spot_from_positions(&positions).unwrap_or(0.012944),
        live_eth_price_usd: 1800.0,
        live_gas_cost_usd: 0.004,
        wallet_eth: Some(0.016388),
        wallet_weth: Some(0.001872),
        wallet_balances: vec![nock_row(wallet_nock)],
        positions,
        ..Default::default()
    };
    for p in &raw.positions {
        // Concentrated pools: in-range liquidity; the Aero basic pool: a totalSupply.
        let v = if is_full_range(p.tick_lower, p.tick_upper) { "342542824101829799" } else { "80173935548453779" };
        raw.pool_liquidity.insert(p.position_id.clone(), v.to_string());
    }
    raw
}

/// The live book of 2026-11-21: the v4 range, the Aero full-range pool, the gas-side
/// WETH/USDC v3 range (no NOCK leg) and 90,086.7891 wallet NOCK.
fn live_raw() -> RawOnChainData {
    raw_of(vec![live_v4(), aero_basic(111.367340, 8408.058135), gas_range_v3()], "90086.7891")
}

fn floor_profile(floor: f64, bps: u32) -> OperatorProfile {
    OperatorProfile { treasury_floor_nock: floor, lp_floor_tolerance_bps: bps, ..OperatorProfile::default() }
}

/// The operator's live profile: floor 100,000, 1,000 bps tolerated (lp_rebuild reports
/// floor_tolerance_nock 10000).
fn live_profile() -> OperatorProfile {
    floor_profile(100_000.0, 1000)
}

fn calc(raw: &RawOnChainData, profile: &OperatorProfile) -> BreachCalc {
    floor_breach_calc(raw, profile, &[]).expect("available").expect("a floor is set")
}

fn crosses(c: Crossing) -> f64 {
    match c {
        Crossing::Crosses(m) => m,
        other => panic!("expected a crossing, got {other:?}"),
    }
}

#[test]
fn test_live_state_known_breach_multiple() {
    let raw = live_raw();
    let v4 = &raw.positions[0];
    assert!((v4.amount1 - 10933.413874).abs() < 1e-4, "v4 NOCK {}", v4.amount1);
    assert!((v4.amount0 - 112.345691).abs() < 1e-4, "v4 USDC {}", v4.amount0);

    let c = calc(&raw, &live_profile());
    // At spot the figure is the Floor Health reading.
    assert!((c.nock_now - 109542.0205).abs() < 1e-3, "{}", c.nock_now);
    let dm = compute_derived_metrics(&raw, 0.0, &live_profile(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
    assert_eq!(dm.global_status.total_nock_amount, "109542");
    let m = crosses(c.hard);
    assert!(rel(m, 1.962574554930) < 1e-9, "m = {m}");
    let price = c.spot * m;
    assert!(rel(c.spot, 0.012944499919) < 1e-9, "spot {}", c.spot);
    assert!(rel(price, 0.025404546) < 1e-8, "price {price}");
    // The tolerated level (90,000) is never crossed: wallet + fees alone stay above it.
    assert_eq!(c.tolerated, Some((90_000.0, Crossing::Never)));
    assert!((c.wallet + c.fees - 90200.548503).abs() < 1e-6);
    assert!((c.wallet - 90086.7891).abs() < 1e-9 && (c.fees - 113.759403).abs() < 1e-9);
    // Each NOCK range at the crossing; the gas-side range is not listed.
    assert_eq!(c.positions.len(), 2);
    assert!((c.positions[0].nock_at_crossing.unwrap() - 3797.636).abs() < 0.01);
    assert!((c.positions[1].nock_at_crossing.unwrap() - 6001.815).abs() < 0.01);
    assert_eq!(c.largest_seller.as_deref(), Some(V4_LABEL));
    assert_eq!(c.first_to_sell_out.as_deref(), Some(V4_LABEL));
    assert!(rel(c.positions[0].sold_out_at_multiple.unwrap(), 1.0001f64.powi(11543)) < 1e-12);

    let fb = floor_breach(&raw, &live_profile());
    assert_eq!(fb.status, "crosses");
    assert_eq!(fb.floor_nock, "100000");
    assert_eq!(fb.spot_price_usd.as_deref(), Some("0.012944"));
    assert_eq!(fb.nock_now.as_deref(), Some("109542"));
    assert_eq!(fb.wallet_nock.as_deref(), Some("90087"));
    assert_eq!(fb.fees_nock.as_deref(), Some("114"));
    assert_eq!(fb.nock_if_all_sold.as_deref(), Some("90201"));
    let x = fb.crossing.as_ref().unwrap();
    assert_eq!(
        (x.level_nock.as_str(), x.status.as_str(), x.multiple.as_deref(), x.price_usd.as_deref(), x.pct_above_spot.as_deref()),
        ("100000", "crosses", Some("1.9626"), Some("0.025405"), Some("96.3"))
    );
    let t = fb.tolerated.as_ref().unwrap();
    assert_eq!((t.level_nock.as_str(), t.status.as_str(), t.multiple.as_deref()), ("90000", "never", None));
    assert_eq!(fb.largest_seller.as_deref(), Some(V4_LABEL));
    assert_eq!(fb.first_to_sell_out.as_deref(), Some(V4_LABEL));
    let (v4, aero) = (&fb.positions[0], &fb.positions[1]);
    assert_eq!(
        (v4.label.as_str(), v4.full_range, v4.nock_now.as_str(), v4.nock_at_crossing.as_deref(), v4.sold_out_at_multiple.as_deref(), v4.sold_out_at_price_usd.as_deref(), v4.note.as_deref()),
        (V4_LABEL, false, "10933", Some("3798"), Some("3.1716"), Some("0.041055"), None)
    );
    assert_eq!(
        (aero.label.as_str(), aero.full_range, aero.nock_now.as_str(), aero.nock_at_crossing.as_deref(), aero.sold_out_at_multiple.as_deref()),
        (AERO_LABEL, true, "8408", Some("6002"), None)
    );
    assert_eq!(aero.note.as_deref(), Some("full range: sells gradually as price rises and never empties"));
    assert!(fb.assumption.contains("ETH/USD constant"));
    assert_eq!(fb.reason, None);
    // The snapshot carries the same figure.
    assert_eq!(dm.floor_breach.as_ref(), Some(&fb));
}

#[test]
fn test_closed_form_cross_check() {
    // In range on the whole way up, T(u) with u = 1/sqrt(m) is linear:
    // W + F + L*(s0*u - sa)/1e16 + A*u, so the crossing has a closed form.
    let raw = live_raw();
    let c = calc(&raw, &live_profile());
    let v4 = &raw.positions[0];
    let l = v4.liquidity as f64;
    let s0 = sqrt_price_x96_to_raw(v4.sqrt_price_x96).sqrt();
    let sa = tick_sqrt_price(v4.tick_lower);
    let a = 8408.058135;
    let u = (100_000.0 - c.wallet - c.fees + l * sa / 1e16) / (l * s0 / 1e16 + a);
    let closed = 1.0 / (u * u);
    assert!(s0 * u > sa, "the crossing is inside the range");
    assert!(rel(crosses(c.hard), closed) < 1e-10, "{} vs {closed}", crosses(c.hard));
}

#[test]
fn test_identity_at_spot_reproduces_each_amount() {
    // NOCK at m = 1 is the adapter's own amount, so the figure at spot is Floor Health.
    let raw = live_raw();
    for p in raw.positions.iter().filter(|p| is_nock_pair(p)) {
        assert_eq!(nock_at_multiple(p, 1.0).unwrap(), p.amount1, "{}", p.position_id);
    }
    assert_eq!(nock_at_multiple(&weth_nock_v3(), 1.0).unwrap(), weth_nock_v3().amount1);
    // Every multi-position fixture: nock_now is total_nock_amount whenever it is stated.
    for (name, raw) in [
        ("rebuild_fixture", crate::tests::rebuild_fixture()),
        ("four_position_fixture", crate::tests::four_position_fixture()),
        ("live", live_raw()),
    ] {
        let profile = crate::tests::operator_profile_2026_11();
        let dm = compute_derived_metrics(&raw, 0.0, &profile, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
        let fb = dm.floor_breach.unwrap();
        if fb.status != "unavailable" {
            assert_eq!(fb.nock_now.as_deref(), Some(dm.global_status.total_nock_amount.as_str()), "{name}");
        }
    }
}

#[test]
fn test_full_range_tail_closed_form() {
    // Wallet 95,000 and a full-range pool of 10,000 NOCK: 95,000 + 10,000/sqrt(m) =
    // 100,000 at m = 4 exactly.
    let raw = raw_of(vec![aero_basic(130.0, 10_000.0)], "95000");
    let c = calc(&raw, &floor_profile(100_000.0, 0));
    let m = crosses(c.hard);
    assert!(rel(m, 4.0) < 1e-10, "m = {m}");
    let fb = floor_breach(&raw, &floor_profile(100_000.0, 0));
    assert_eq!(fb.crossing.as_ref().unwrap().multiple.as_deref(), Some("4.0000"));
    assert_eq!(fb.crossing.as_ref().unwrap().price_usd, Some(format!("{:.6}", raw.spot_price_nock_usd * 4.0)));
    assert_eq!(fb.crossing.as_ref().unwrap().pct_above_spot.as_deref(), Some("300.0"));
    assert_eq!(fb.nock_if_all_sold.as_deref(), Some("95000"));
    assert_eq!(fb.tolerated, None, "no tolerance set");
    assert_eq!(fb.positions[0].sold_out_at_multiple, None);
    assert!(fb.positions[0].note.as_deref().unwrap().starts_with("full range"));
    // Only a full-range pool holds NOCK: nothing sells out first.
    assert_eq!(fb.first_to_sell_out, None);
    assert_eq!(fb.largest_seller.as_deref(), Some(AERO_LABEL));
}

#[test]
fn test_one_range_only() {
    let wide = nock_usdc_wide();
    assert!((wide.amount1 - 3748.278).abs() < 1e-3, "{}", wide.amount1);
    let raw = raw_of(vec![wide], "98000");
    let c = calc(&raw, &floor_profile(100_000.0, 0));
    assert!(rel(crosses(c.hard), 1.338086531638) < 1e-9, "{:?}", c.hard);
    let fb = floor_breach(&raw, &floor_profile(100_000.0, 0));
    assert_eq!(fb.positions[0].sold_out_at_multiple.as_deref(), Some("1.9867"));
    assert_eq!(fb.first_to_sell_out.as_deref(), Some(V4_LABEL));
    assert_eq!(fb.nock_if_all_sold.as_deref(), Some("98000"));
}

#[test]
fn test_range_plus_full_range_tail() {
    let raw = raw_of(vec![nock_usdc_wide(), aero_basic(130.0, 10_000.0)], "90000");
    let c = calc(&raw, &floor_profile(100_000.0, 0));
    assert!(rel(crosses(c.hard), 1.429707912477) < 1e-9, "{:?}", c.hard);
    // The range sells out first; by the crossing the range has sold more.
    assert_eq!(c.first_to_sell_out.as_deref(), Some(V4_LABEL));
}

#[test]
fn test_no_lp() {
    let raw = raw_of(vec![], "120000");
    let fb = floor_breach(&raw, &floor_profile(100_000.0, 0));
    assert_eq!(fb.status, "never");
    assert!(fb.positions.is_empty());
    assert_eq!(fb.nock_if_all_sold.as_deref(), Some("120000"));
    assert_eq!((fb.largest_seller.as_deref(), fb.first_to_sell_out.as_deref()), (None, None));
    assert_eq!(fb.crossing.as_ref().unwrap().multiple, None);

    let raw = raw_of(vec![], "80000");
    let fb = floor_breach(&raw, &floor_profile(100_000.0, 0));
    assert_eq!(fb.status, "already_below");
    let x = fb.crossing.unwrap();
    assert_eq!(x.multiple.as_deref(), Some("1.0000"));
    assert_eq!(x.price_usd.as_deref(), Some("0.012944"));
    assert_eq!(x.pct_above_spot.as_deref(), Some("0.0"));
}

#[test]
fn test_wallet_alone_above_floor_never_breached() {
    let mut raw = live_raw();
    raw.wallet_balances = vec![nock_row("100500")];
    let fb = floor_breach(&raw, &live_profile());
    assert_eq!(fb.status, "never");
    assert_eq!(fb.crossing.as_ref().unwrap().multiple, None);
    assert_eq!(fb.nock_if_all_sold.as_deref(), Some("100614"));
    assert_eq!(fb.largest_seller, None);
    // Each range still states where it sells out.
    assert_eq!(fb.first_to_sell_out.as_deref(), Some(V4_LABEL));
}

#[test]
fn test_already_below() {
    let mut raw = live_raw();
    raw.wallet_balances = vec![nock_row("80000")];
    let c = calc(&raw, &live_profile());
    assert!((c.nock_now - 99455.231).abs() < 1e-2, "{}", c.nock_now);
    assert_eq!(c.hard, Crossing::AlreadyBelow);
    let fb = floor_breach(&raw, &live_profile());
    assert_eq!(fb.status, "already_below");
    assert_eq!(fb.crossing.as_ref().unwrap().multiple.as_deref(), Some("1.0000"));
    assert_eq!(fb.largest_seller, None);
    // The tolerated level (90,000) is evaluated on its own: 80,113.76 NOCK stays if
    // every range sells out, so a rise does cross it.
    let t = fb.tolerated.as_ref().unwrap();
    assert_eq!(t.status, "crosses");
    let (_, tc) = c.tolerated.unwrap();
    let m = crosses(tc);
    let v4 = &raw.positions[0];
    let total = 80000.0 + 113.759403 + nock_at_multiple(v4, m).unwrap() + 8408.058135 / m.sqrt();
    assert!((total - 90_000.0).abs() < 1e-6, "{total}");
}

#[test]
fn test_unread_position_is_null_with_a_reason() {
    let mut raw = live_raw();
    raw.positions[0].sqrt_price_x96 = U256(0, 0);
    let fb = floor_breach(&raw, &live_profile());
    assert_eq!(fb.status, "unavailable");
    assert_eq!((fb.crossing.as_ref(), fb.tolerated.as_ref(), fb.nock_if_all_sold.as_deref(), fb.nock_now.as_deref()), (None, None, None, None));
    assert!(fb.positions.is_empty());
    let reason = fb.reason.unwrap();
    assert!(reason.contains(V4_LABEL) && reason.contains("price"), "{reason}");

    // A full-range pool is computed from its amounts, never its sqrt: no reason from it.
    let mut raw = live_raw();
    raw.positions[1].sqrt_price_x96 = U256(0, 0);
    assert_eq!(floor_breach(&raw, &live_profile()).status, "crosses");
    // A full-range NOCK amount that is not a number is unavailable.
    let mut raw = live_raw();
    raw.positions[1].amount1 = f64::NAN;
    let fb = floor_breach(&raw, &live_profile());
    assert_eq!(fb.status, "unavailable");
    assert!(fb.reason.unwrap().contains(AERO_LABEL));
    // No spot is unavailable, never "safe".
    let mut raw = live_raw();
    raw.spot_price_nock_usd = 0.0;
    let fb = floor_breach(&raw, &live_profile());
    assert_eq!((fb.status.as_str(), fb.reason.as_deref()), ("unavailable", Some("no NOCK/USD spot this cycle")));
}

#[test]
fn test_frozen_pool_holding_nock_is_unavailable() {
    let mut raw = live_raw();
    raw.pool_liquidity.insert(raw.positions[0].position_id.clone(), "0".to_string());
    let fb = floor_breach(&raw, &live_profile());
    assert_eq!(fb.status, "unavailable");
    let reason = fb.reason.unwrap();
    assert!(reason.contains(V4_LABEL) && reason.contains("frozen"), "{reason}");

    // The same frozen pool under a range holding no NOCK (price above the range: all
    // USDC) contributes 0 and is fine.
    let mut p = live_v4();
    p.tick_lower = 274000;
    p.tick_upper = 280000;
    p.in_range = false;
    p.is_price_source = false;
    p.fee0 = 0.0;
    p.fee1 = 0.0;
    let p = with_amounts(p);
    assert_eq!(p.amount1, 0.0);
    let mut raw = live_raw();
    raw.positions.push(RawPosition { position_id: "uniswap-v4:base:1".to_string(), nft_id: 1, ..p });
    raw.pool_liquidity.insert("uniswap-v4:base:1".to_string(), "0".to_string());
    let fb = floor_breach(&raw, &live_profile());
    assert_eq!(fb.status, "crosses");
    assert_eq!(fb.positions[2].note.as_deref(), Some("holds no NOCK at this price"));
    assert_eq!(fb.positions[2].sold_out_at_multiple, None);
    assert_eq!(fb.crossing.unwrap().multiple.as_deref(), Some("1.9626"), "a range with no NOCK changes nothing");

    // A failed pool read (no entry) does not disprove activity: computed, with a note.
    let mut raw = live_raw();
    raw.pool_liquidity.remove(&raw.positions[0].position_id.clone());
    let fb = floor_breach(&raw, &live_profile());
    assert_eq!(fb.status, "crosses");
    assert_eq!(fb.positions[0].note.as_deref(), Some("pool activity not read this cycle"));
}

#[test]
fn test_hypothetical_range_reads_its_pools_activity() {
    // A hypothetical range on the v4 pool reads the tracked position's pool entry: a
    // frozen pool makes it unavailable like a tracked one.
    let mut extra = live_v4();
    extra.position_id = "hypothetical:ladder".to_string();
    extra.nft_id = 0;
    extra.fee0 = 0.0;
    extra.fee1 = 0.0;
    let mut raw = live_raw();
    raw.pool_liquidity.insert(raw.positions[0].position_id.clone(), "0".to_string());
    assert_eq!(floor_breach_with(&raw, &live_profile(), &[extra.clone()]).status, "unavailable");
    // On a pool no tracked position reads, it says so.
    let mut raw = live_raw();
    raw.positions.remove(0);
    let fb = floor_breach_with(&raw, &live_profile(), &[extra]);
    assert_eq!(fb.positions.last().unwrap().note.as_deref(), Some("hypothetical range: no pool activity read for its pool"));
}

#[test]
fn test_ladder_above_spot_lowers_the_crossing_and_keeps_the_tolerated_level() {
    // #166: a single-sided ladder whose top sits just above spot holds a little NOCK
    // (here about the 87 NOCK the rebuild budget allows), moved out of the wallet, that
    // sells on a rise. It lowers the hard-floor multiple a little and keeps the tolerated
    // level never crossed.
    let raw = live_raw();
    let before = calc(&raw, &live_profile());
    let mut ladder = live_v4();
    ladder.position_id = "hypothetical:ladder".to_string();
    ladder.fee0 = 0.0;
    ladder.fee1 = 0.0;
    ladder.tick_lower = 273000;
    ladder.tick_upper = 273800;
    ladder.liquidity = 0;
    ladder.is_price_source = false;
    let sa = tick_sqrt_price(273000);
    let s0 = sqrt_price_x96_to_raw(ladder.sqrt_price_x96).sqrt();
    ladder.liquidity = (86.0 * 1e16 / (s0 - sa)) as u128;
    let ladder = with_amounts(ladder);
    assert!(ladder.amount1 > 85.0 && ladder.amount1 < 87.0, "{}", ladder.amount1);
    let mut funded = live_raw();
    funded.wallet_balances = vec![nock_row(&format!("{:.6}", 90086.7891 - ladder.amount1))];
    let after = floor_breach_calc(&funded, &live_profile(), &[ladder.clone()]).unwrap().unwrap();
    assert!(crosses(after.hard) < crosses(before.hard));
    assert!(crosses(before.hard) - crosses(after.hard) < 0.05);
    assert_eq!(after.tolerated.unwrap().1, Crossing::Never);
    // Added on top of the wallet (not moved out of it), it can only hold the stack up.
    let on_top = floor_breach_calc(&raw, &live_profile(), &[ladder]).unwrap().unwrap();
    assert_eq!(crosses(on_top.hard), crosses(before.hard), "it sells out (by 1.0001^743) long before the crossing");
    // A ladder whose top is at or below spot holds no NOCK: no effect on a rise. (NOCK
    // is token1, so a lower NOCK price is a higher tick: the range sits above the tick.)
    let mut low = live_v4();
    low.position_id = "hypothetical:low".to_string();
    low.fee0 = 0.0;
    low.fee1 = 0.0;
    low.tick_lower = 273800;
    low.tick_upper = 275000;
    let low = with_amounts(low);
    assert_eq!(low.amount1, 0.0);
    let same = floor_breach_calc(&raw, &live_profile(), &[low]).unwrap().unwrap();
    assert_eq!(crosses(same.hard), crosses(before.hard));
}

#[test]
fn test_tolerated_margin_less_fees_is_the_rebuild_nock_budget() {
    // The rebuild may put `wallet − floor + tolerance` NOCK into LP (advice.rs). The
    // tolerated level's margin counts uncollected fees too; less those fees it is the
    // same budget, so the two features cannot drift apart.
    let mut raw = live_raw();
    raw.wallet_balances.push(WalletBalance {
        wallet_address: WALLET.to_string(),
        token_symbol: "USDC".to_string(),
        token_address: USDC_BASE.to_string(),
        amount: "1545.8357".to_string(),
        amount_usd: "1545.8357".to_string(),
        updated_at: "now".to_string(),
    });
    let mut profile = live_profile();
    profile.target_weights_bps = TargetWeightsBps { treasury: 5000, uniswap_lp: 4000, cash: 1000 };
    let c = calc(&raw, &profile);
    let (level, _) = c.tolerated.unwrap();
    let margin = c.wallet + c.fees - level;
    let budget = margin - c.fees;
    assert!((budget - 86.7891).abs() < 1e-6, "{budget}");
    let dm = compute_derived_metrics(&raw, 0.0, &profile, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
    let fb = dm.floor_breach.as_ref().unwrap();
    // The rebuild's own words name the same budget.
    let said = format!("only {:.0} NOCK may go into LP", budget);
    let rb = &dm.lp_rebuild;
    let mentioned = rb.suggestions.iter().filter_map(|s| s.limited_by.as_deref()).chain(rb.excluded.iter().map(|e| e.reason.as_str())).any(|r| r.contains(&said));
    assert!(mentioned, "{said} in {:?}", rb);
    assert_eq!(rb.floor_tolerance_nock.as_deref(), Some("10000"));
    // Its worst case is the wallet less what it suggests depositing (nothing here),
    // fees excluded: floor_breach's wallet_nock, with the fees stated beside it.
    assert!(rb.suggestions.is_empty());
    assert_eq!(rb.worst_case_nock, fb.wallet_nock);
    assert_eq!((fb.wallet_nock.as_deref(), fb.fees_nock.as_deref(), fb.nock_if_all_sold.as_deref()), (Some("90087"), Some("114"), Some("90201")));
}

#[test]
fn test_no_floor_states_nothing() {
    let fb = floor_breach(&live_raw(), &OperatorProfile::default());
    assert_eq!(fb.status, "no_floor");
    assert_eq!((&fb.crossing, &fb.tolerated, &fb.nock_if_all_sold, &fb.spot_price_usd), (&None, &None, &None, &None));
    assert!(fb.positions.is_empty());
    let v = serde_json::to_value(&fb).unwrap();
    assert_eq!(crate::floor_breach_text::floor_breach_line(Some(&v)), "");
}

#[test]
fn test_tolerance_at_or_above_the_floor_is_never_crossed() {
    let fb = floor_breach(&live_raw(), &floor_profile(100_000.0, 10_000));
    assert_eq!(fb.tolerated.unwrap().status, "never");
}

#[test]
fn test_weth_nock_holds_eth_constant() {
    // An unpriced ETH (0.0) does not make a WETH/NOCK range unavailable: no ETH price is
    // an input. Its NOCK at m is concentrated_amounts_raw at the moved sqrt price.
    let p = weth_nock_v3();
    let mut raw = raw_of(vec![nock_usdc_wide(), p.clone()], "98000");
    raw.live_eth_price_usd = 0.0;
    let fb = floor_breach(&raw, &floor_profile(100_000.0, 0));
    assert_eq!(fb.status, "crosses");
    assert!(fb.assumption.contains("ETH/USD constant"));
    let m = 1.002f64;
    assert!(nock_at_multiple(&p, m).unwrap() > 0.0, "still in range");
    let s0 = sqrt_price_x96_to_raw(p.sqrt_price_x96).sqrt();
    let s = s0 / m.sqrt();
    let x96 = s * 79228162514264337593543950336.0;
    let two128 = 340282366920938463463374607431768211456.0f64;
    let hi = (x96 / two128).floor();
    let at = U256(hi as u128, (x96 - hi * two128) as u128);
    let (_, a1) = concentrated_amounts_raw(p.liquidity, at, p.tick_lower, p.tick_upper);
    assert!(rel(nock_at_multiple(&p, m).unwrap(), a1 / 1e16) < 1e-9);
}

#[test]
fn test_nock_as_token0_mirrors_token1() {
    // The same range with its legs swapped: ticks negated, sqrt price inverted.
    let p = nock_usdc_wide();
    let s0 = sqrt_price_x96_to_raw(p.sqrt_price_x96).sqrt();
    let x96 = (1.0 / s0) * 79228162514264337593543950336.0;
    let two128 = 340282366920938463463374607431768211456.0f64;
    let hi = (x96 / two128).floor();
    let mirror = RawPosition {
        token0: nock(),
        token1: usdc(),
        sqrt_price_x96: U256(hi as u128, (x96 - hi * two128) as u128),
        tick: -p.tick,
        tick_lower: -p.tick_upper,
        tick_upper: -p.tick_lower,
        amount0: p.amount1,
        amount1: p.amount0,
        is_price_source: false,
        ..p.clone()
    };
    assert!(rel(nock_at_multiple(&mirror, 1.0).unwrap(), p.amount1) < 1e-6);
    let a = crossing_multiple(98_000.0, &[&p], 100_000.0).unwrap();
    let b = crossing_multiple(98_000.0, &[&mirror], 100_000.0).unwrap();
    assert!(rel(crosses(a), crosses(b)) < 1e-6, "{a:?} vs {b:?}");
    assert!(rel(sell_out_edge_multiple(&p).unwrap(), sell_out_edge_multiple(&mirror).unwrap()) < 1e-6);
    assert_eq!(nock_sell_out_multiple(&mirror), nock_sell_out_multiple(&p));
}

#[test]
fn test_empty_range_contributes_nothing() {
    let mut p = live_v4();
    p.liquidity = 0;
    p.amount0 = 0.0;
    p.amount1 = 0.0;
    p.fee1 = 0.0;
    let raw = raw_of(vec![p, aero_basic(130.0, 10_000.0)], "95000");
    let fb = floor_breach(&raw, &floor_profile(100_000.0, 0));
    assert_eq!(fb.positions[0].note.as_deref(), Some("empty"));
    assert_eq!(fb.positions[0].nock_now, "0");
    assert_eq!(fb.crossing.unwrap().multiple.as_deref(), Some("4.0000"));
}

// ---------------- MCP text (both transports compile floor_breach_text.rs) ----------------

fn line_for(raw: &RawOnChainData, profile: &OperatorProfile) -> String {
    let v = serde_json::to_value(floor_breach(raw, profile)).unwrap();
    crate::floor_breach_text::floor_breach_line(Some(&v))
}

#[test]
fn test_mcp_line_for_each_status() {
    let crosses = line_for(&live_raw(), &live_profile());
    assert_eq!(
        crosses,
        "• Floor crossing: NOCK ≥ $0.025405 (1.9626x spot, +96.3%) would take the stack below the 100000 NOCK floor; v4 USDC/NOCK 3.00% #3163450 sells the most by then (10933 → 3798 NOCK). First range to sell out: v4 USDC/NOCK 3.00% #3163450 at NOCK ≥ $0.041055 (3.1716x spot). If every range sold out, 90201 NOCK would remain (90087 wallet + 114 uncollected fees). Tolerated level 90000 NOCK: never crossed by a rise. Each pool moves with spot; WETH/NOCK ranges hold ETH/USD constant. A measurement, not an instruction.\n"
    );
    let mut raw = live_raw();
    raw.wallet_balances = vec![nock_row("100500")];
    assert_eq!(
        line_for(&raw, &live_profile()),
        "• Floor crossing: none. If every range sold out, 100614 NOCK would remain (100500 wallet + 114 uncollected fees), at or above the 100000 NOCK floor. First range to sell out: v4 USDC/NOCK 3.00% #3163450 at NOCK ≥ $0.041055 (3.1716x spot). Tolerated level 90000 NOCK: never crossed by a rise. Each pool moves with spot; WETH/NOCK ranges hold ETH/USD constant. A measurement, not an instruction.\n"
    );
    raw.wallet_balances = vec![nock_row("80000")];
    let below = line_for(&raw, &live_profile());
    assert!(below.starts_with("• Floor crossing: already below the 100000 NOCK floor at spot. Tolerated level 90000 NOCK: crossed at NOCK ≥ $"), "{below}");
    let mut raw = live_raw();
    raw.positions[0].sqrt_price_x96 = U256(0, 0);
    assert_eq!(
        line_for(&raw, &live_profile()),
        "• Floor crossing: unavailable: v4 USDC/NOCK 3.00% #3163450: pool price could not be read (sqrtPrice is zero)\n"
    );
    let tail = line_for(&raw_of(vec![aero_basic(130.0, 10_000.0)], "95000"), &floor_profile(100_000.0, 0));
    assert!(tail.contains("No range sells out: only full-range pools hold NOCK"), "{tail}");
    assert!(!tail.contains("Tolerated"), "{tail}");
    assert_eq!(crate::floor_breach_text::floor_breach_line(None), "• Floor crossing: not reported (snapshot predates 2.9.0)\n");
    assert_eq!(
        crate::floor_breach_text::floor_breach_line(Some(&serde_json::Value::Null)),
        "• Floor crossing: not reported (snapshot predates 2.9.0)\n"
    );
    for text in [&crosses, &below, &tail] {
        assert!(!text.contains("$0.00"), "{text}");
        for word in ["should", "withdraw", "consider", "recommend"] {
            assert!(!text.to_lowercase().contains(word), "{word} in {text}");
        }
    }
}

#[test]
fn test_portfolio_state_text_carries_the_line_under_the_stack() {
    let raw = live_raw();
    let dm = compute_derived_metrics(&raw, 0.0, &live_profile(), StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default());
    let data = serde_json::json!({ "derived_metrics": serde_json::to_value(&dm).unwrap() });
    let text = format_portfolio_state_text(&data);
    assert!(text.contains("• Sovereign NOCK Stack: 109542 NOCK (+9542 above the 100000 NOCK floor)\n• Floor crossing: NOCK ≥ $0.025405"), "{text}");
    // A snapshot from before 2.9.0 says so.
    let mut old = data.clone();
    old["derived_metrics"].as_object_mut().unwrap().remove("floor_breach");
    assert!(format_portfolio_state_text(&old).contains("• Floor crossing: not reported (snapshot predates 2.9.0)\n• Stable Stack"));
}
