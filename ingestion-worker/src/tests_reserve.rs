//! The standing protected cash reserve (#162): `operator_profile.cash_reserve_usdc`.
//!
//! On 2026-11-21 the advice reported "Liquid dry powder: $1360.77" while the operator's
//! $950 reserve, configured only under the completed accumulation plan, protected
//! nothing. The reserve is now a standing setting: dry powder never exceeds the headroom
//! above it, no plan buy draws below it, and the snapshot states it. It is measured on
//! wallet USDC plus USDC inside NOCK-pair LP; gas-side ETH/USDC ranges are left out.
//! Nothing here touches the network or needs a database or a config.yaml.
use super::*;
use crate::reserve_text::reserve_line;
use crate::tests_gas_lp::{nock_only, profile, with_gas_ranges};
use chrono::TimeZone;

fn with_reserve(r: Option<f64>) -> OperatorProfile {
    OperatorProfile { cash_reserve_usdc: r, ..profile() }
}

fn metrics_for(raw: &RawOnChainData, p: &OperatorProfile, plan: &PlanContext) -> DerivedMetrics {
    compute_derived_metrics_with_plan(
        raw, 0.0, p, StructuralChannels { channels: vec![] }, &[],
        &LedgerAggregate::default(), &MarketRegime::default(), plan,
    )
}

fn num(s: &Option<String>) -> f64 {
    s.as_deref().expect("present").parse().expect("a number")
}

fn nock_lp_usdc(raw: &RawOnChainData) -> f64 {
    raw.positions.iter().filter(|p| is_nock_pair(p)).map(|p| {
        if is_usdc_token(&p.token0) { p.amount0 } else if is_usdc_token(&p.token1) { p.amount1 } else { 0.0 }
    }).sum()
}

fn plan(cash_reserve: Option<f64>) -> AccumulationPlanConfig {
    AccumulationPlanConfig {
        id: "reserve-test".to_string(),
        budget_usdc: 1000.0,
        tranches: 2,
        interval_hours: 24.0,
        starts_at: None,
        venue: None,
        max_price_usd: None,
        pause_when_overextended: true,
        respect_cash_target: false,
        cash_reserve_usdc: cash_reserve,
    }
}

fn now() -> DateTime<Utc> {
    Utc.with_ymd_and_hms(2026, 11, 21, 1, 52, 7).unwrap()
}

#[test]
fn test_the_operators_numbers_cap_dry_powder_at_the_headroom() {
    // 2026-11-21 01:52 UTC: wallet $1545.84, $222.34 in NOCK LP, cash target $185.06,
    // tax reserve $8.70. Before #162 the advice said $1360.77.
    let reserve = standing_reserve(Some(950.0), 1545.84, 222.34);
    assert_eq!(reserve.stable().map(|v| format!("{v:.2}")).as_deref(), Some("1768.18"));
    assert_eq!(reserve.headroom().map(|v| format!("{v:.2}")).as_deref(), Some("818.18"));
    let wallet_deployable: f64 = 1545.84 - 185.06;
    let deployable = wallet_deployable.min(reserve.headroom().unwrap());
    let base = format!(
        "Liquid dry powder: ${:.2} (wallet USDC $1545.84 less the larger of the $185.06 cash target and the $8.70 virtual tax reserve).",
        deployable
    );
    assert_eq!(
        reserve_dry_powder_sentence(base, &reserve, wallet_deployable),
        "Liquid dry powder: $818.18 (wallet USDC $1545.84 less the larger of the $185.06 cash target and the $8.70 virtual tax reserve, \
and no more than the $818.18 the stable balance holds above the reserve). The $950.00 protected cash reserve is measured on wallet \
USDC plus the $222.34 of USDC in NOCK LP: $1768.18, $818.18 above it."
    );
}

#[test]
fn test_a_reserve_that_does_not_bind_is_still_stated() {
    let reserve = standing_reserve(Some(100.0), 300.0, 0.0);
    let base = "Liquid dry powder: $150.00 (wallet USDC $300.00 less the $150.00 cash target).".to_string();
    assert_eq!(
        reserve_dry_powder_sentence(base, &reserve, 150.0),
        "Liquid dry powder: $150.00 (wallet USDC $300.00 less the $150.00 cash target). The $100.00 protected cash reserve is \
measured on wallet USDC plus the $0.00 of USDC in NOCK LP: $300.00, $200.00 above it."
    );
}

#[test]
fn test_below_the_reserve_there_is_no_dry_powder() {
    let reserve = standing_reserve(Some(950.0), 700.0, 100.0);
    assert_eq!(reserve.headroom(), Some(-150.0));
    let base = "Liquid dry powder: none (wallet USDC $700.00 less the $100.00 cash target).".to_string();
    assert_eq!(
        reserve_dry_powder_sentence(base, &reserve, 600.0),
        "Liquid dry powder: none (wallet USDC $700.00 less the $100.00 cash target, and nothing while the stable balance is \
below the reserve). The $950.00 protected cash reserve is measured on wallet USDC plus the $100.00 of USDC in NOCK LP: \
$800.00, $150.00 below it."
    );
}

#[test]
fn test_an_unset_reserve_is_no_rule_and_an_invalid_one_is_named() {
    assert_eq!(standing_reserve(None, 1.0, 1.0), StandingReserve::Unset);
    assert_eq!(reserve_dry_powder_sentence("base.".to_string(), &StandingReserve::Unset, 1.0), "base.");
    for bad in [-1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(standing_reserve(Some(bad), 1.0, 1.0), StandingReserve::Invalid, "{bad}");
    }
    assert_eq!(standing_reserve(Some(0.0), 5.0, 0.0).headroom(), Some(5.0));
    assert_eq!(
        reserve_dry_powder_sentence("base.".to_string(), &StandingReserve::Invalid, 1.0),
        "Liquid dry powder: not reported: operator_profile.cash_reserve_usdc must be zero or positive."
    );
}

#[test]
fn test_the_reserve_parses_from_the_operator_profile() {
    let p: OperatorProfile = serde_yaml::from_str("treasury_floor_nock: 100000\ncash_reserve_usdc: 950\n").unwrap();
    assert_eq!(p.cash_reserve_usdc, Some(950.0));
    let none: OperatorProfile = serde_yaml::from_str("treasury_floor_nock: 100000\n").unwrap();
    assert_eq!(none.cash_reserve_usdc, None);
    assert_eq!(OperatorProfile::default().cash_reserve_usdc, None);
}

#[test]
fn test_the_snapshot_states_the_reserve_and_leaves_gas_range_usdc_out() {
    let raw = with_gas_ranges();
    let lp = nock_lp_usdc(&raw);
    assert!(lp > 0.0);
    let m = metrics_for(&raw, &with_reserve(Some(1000.0)), &PlanContext::none());
    let g = &m.global_status;
    assert_eq!(g.cash_reserve_usdc.as_deref(), Some("1000.00"));
    // Wallet $1053 + NOCK-pair USDC; the WETH/USDC ranges' USDC is not in it.
    assert!((num(&g.reserve_stable_usdc) - (1053.0 + lp)).abs() < 0.006, "{:?}", g.reserve_stable_usdc);
    assert!((num(&g.reserve_headroom_usdc) - (53.0 + lp)).abs() < 0.006);
    // The total USDC pillar still counts the gas ranges: it is what is held.
    assert!(num(&Some(g.total_stable_usd.clone())) > num(&g.reserve_stable_usdc) + 100.0);
    // Dry powder is the headroom: below what the wallet side alone allows.
    assert_eq!(g.dry_powder_usdc, g.reserve_headroom_usdc);
    let d = &g.shot_advice_details;
    assert!(d.contains(&format!("Liquid dry powder: ${}", g.dry_powder_usdc.as_deref().unwrap())), "{d}");
    assert!(d.contains("The $1000.00 protected cash reserve is measured on wallet USDC plus the "), "{d}");

    // Without a reserve: no reserve fields, and dry powder is the wallet side.
    let free = metrics_for(&raw, &with_reserve(None), &PlanContext::none());
    let f = &free.global_status;
    assert_eq!((&f.cash_reserve_usdc, &f.reserve_stable_usdc, &f.reserve_headroom_usdc), (&None, &None, &None));
    assert!(num(&f.dry_powder_usdc) > num(&g.dry_powder_usdc));
    assert!(!f.shot_advice_details.contains("protected cash reserve"));
}

#[test]
fn test_an_invalid_reserve_reports_no_dry_powder_and_blocks_the_plan() {
    let raw = nock_only();
    let ctx = PlanContext { cfg: Some(plan(None)), fills: PlanFills::default(), ledger_unavailable: false, now: now() };
    let m = metrics_for(&raw, &with_reserve(Some(-5.0)), &ctx);
    // Unknown, never a made-up 0.00 (#162 review), and named on every surface.
    assert_eq!(m.global_status.dry_powder_usdc, None);
    assert_eq!(m.global_status.cash_reserve_usdc, None);
    assert_eq!(m.global_status.reserve_note.as_deref(), Some("operator_profile.cash_reserve_usdc must be zero or positive"));
    let text = format_portfolio_state_text(&serde_json::json!({ "derived_metrics": &m }));
    assert!(text.contains("• Protected cash reserve: not applied: operator_profile.cash_reserve_usdc must be zero or positive\n"), "{text}");
    let p = m.accumulation_plan.expect("plan");
    assert_eq!(p.status, "invalid");
    let advice = format!("{} {}", m.global_status.shot_advice_action, m.global_status.shot_advice_details);
    assert!(advice.contains("operator_profile.cash_reserve_usdc must be zero or positive"), "{advice}");
}

#[test]
fn test_the_reserve_outlives_a_completed_plan() {
    // The defect: the plan's $950 protected nothing once the plan was complete.
    let raw = nock_only();
    let lp = nock_lp_usdc(&raw);
    let done = PlanFills { count: 2, spent_usdc: 1000.0, acquired_nock: 80_000.0, last_at: Some(now()), first_price: Some(0.0125) };
    let ctx = PlanContext { cfg: Some(plan(Some(950.0))), fills: done, ledger_unavailable: false, now: now() };
    // Before #162: only the plan's own reserve, and nothing stated once complete.
    let before = metrics_for(&raw, &with_reserve(None), &ctx);
    assert_eq!(before.accumulation_plan.as_ref().unwrap().status, "complete");
    assert_eq!(before.global_status.cash_reserve_usdc, None);
    // The standing reserve: dry powder is capped and stated whatever the plan's state.
    let m = metrics_for(&raw, &with_reserve(Some(950.0)), &ctx);
    assert_eq!(m.accumulation_plan.as_ref().unwrap().status, "complete");
    let headroom = 1053.0 + lp - 950.0;
    assert!((num(&m.global_status.dry_powder_usdc) - headroom.max(0.0).min(num(&before.global_status.dry_powder_usdc))).abs() < 0.006);
    assert!(m.global_status.shot_advice_details.contains("The $950.00 protected cash reserve"), "{}", m.global_status.shot_advice_details);
}

#[test]
fn test_a_plan_honours_the_larger_of_its_own_and_the_standing_reserve() {
    let raw = nock_only();
    let lp = nock_lp_usdc(&raw);
    let fresh = |own: Option<f64>| PlanContext { cfg: Some(plan(own)), fills: PlanFills::default(), ledger_unavailable: false, now: now() };

    // The standing reserve binds when the plan has none.
    let m = metrics_for(&raw, &with_reserve(Some(1000.0)), &fresh(None));
    let p = m.accumulation_plan.expect("plan");
    assert_eq!(p.cash_reserve_usdc.as_deref(), Some("1000.00"));
    let c = p.binding_constraint.expect("the reserve binds");
    assert!(c.contains("less the standing $1000.00 protected cash reserve"), "{c}");
    assert!((p.tranche_usdc.parse::<f64>().unwrap() - (53.0 + lp)).abs() < 0.006, "{}", p.tranche_usdc);
    assert!(m.global_status.shot_advice_details.contains("never spends the stable balance (wallet + NOCK LP USDC) below $1000.00"));

    // The plan's own reserve binds when it is the larger; its wording is unchanged.
    let m = metrics_for(&raw, &with_reserve(Some(900.0)), &fresh(Some(1000.0)));
    let p = m.accumulation_plan.expect("plan");
    assert_eq!(p.cash_reserve_usdc.as_deref(), Some("1000.00"));
    let c = p.binding_constraint.clone().expect("the reserve binds");
    assert!(c.contains("less the $1000.00 protected cash reserve"), "{c}");

    // Without a standing reserve the plan is exactly as before.
    let old = metrics_for(&raw, &with_reserve(None), &fresh(Some(1000.0))).accumulation_plan.unwrap();
    assert_eq!((old.tranche_usdc, old.binding_constraint, old.cash_reserve_usdc), (p.tranche_usdc, p.binding_constraint, p.cash_reserve_usdc));
}

#[test]
fn test_both_mcp_transports_state_the_reserve() {
    let raw = with_gas_ranges();
    let m = metrics_for(&raw, &with_reserve(Some(1000.0)), &PlanContext::none());
    let data = serde_json::json!({ "derived_metrics": m });
    let text = format_portfolio_state_text(&data);
    let g = &m.global_status;
    let line = format!(
        "• Protected cash reserve: $1000.00 (wallet + NOCK LP USDC ${}, ${} above it; liquid dry powder ${}); kept for buying lower, never drawn by a plan buy\n",
        g.reserve_stable_usdc.as_deref().unwrap(),
        g.reserve_headroom_usdc.as_deref().unwrap(),
        g.dry_powder_usdc.as_deref().unwrap()
    );
    assert!(text.contains(&format!("• Stable Stack: {} USDC\n{line}• Gas held", g.total_usdc_amount)), "{text}");

    // Below the reserve, and with none set.
    let below = serde_json::json!({ "cash_reserve_usdc": "950.00", "reserve_stable_usdc": "800.00", "reserve_headroom_usdc": "-150.00", "dry_powder_usdc": "0.00" });
    assert_eq!(
        reserve_line(Some(&below)),
        "• Protected cash reserve: $950.00 (wallet + NOCK LP USDC $800.00, $150.00 below it; liquid dry powder $0.00); kept for buying lower, never drawn by a plan buy\n"
    );
    let none = metrics_for(&raw, &with_reserve(None), &PlanContext::none());
    let text = format_portfolio_state_text(&serde_json::json!({ "derived_metrics": none }));
    assert!(!text.contains("Protected cash reserve"), "{text}");
}

#[test]
fn test_a_fraction_of_a_cent_is_never_minus_zero() {
    // #162 review: -0.004 printed "-0.00", read "below" by the advice and "above" by the tile.
    let r = standing_reserve(Some(950.0), 949.996, 0.0);
    assert_eq!(format!("{:.2}", r.headroom().unwrap()), "0.00");
    assert!(!r.headroom().unwrap().is_sign_negative());
    let base = "Liquid dry powder: none (wallet USDC $949.996 less the $0.00 cash target).".to_string();
    assert!(reserve_dry_powder_sentence(base, &r, 0.0).ends_with("$950.00, $0.00 above it."));
    let under = standing_reserve(Some(950.0), 949.994, 0.0);
    assert_eq!(format!("{:.2}", under.headroom().unwrap()), "-0.01");
    assert_eq!(standing_reserve(Some(950.0), 1768.181, 0.0).headroom(), Some(818.18));
}

#[test]
fn test_an_invalid_plan_reserve_is_not_replaced_by_the_standing_one() {
    let raw = nock_only();
    let ctx = PlanContext { cfg: Some(plan(Some(-5.0))), fills: PlanFills::default(), ledger_unavailable: false, now: now() };
    let p = metrics_for(&raw, &with_reserve(Some(950.0)), &ctx).accumulation_plan.expect("plan");
    assert_eq!(p.status, "invalid");
    assert_eq!(p.cash_reserve_usdc.as_deref(), Some("-5.00"));
}
