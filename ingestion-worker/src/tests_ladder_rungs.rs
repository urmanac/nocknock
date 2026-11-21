//! A ladder of rungs, the fee estimate and the operator's configured shape (extending the
//! ladder simulator #166). The fixture is the #166 one (tests_ladder.rs: spot $0.012944,
//! wallet USDC 1545.8357 and NOCK 90086.7891, floor 100,000 with 1,000 bps, so an LP NOCK
//! budget of 86.7891, a $950 reserve, the v4 range 262200..282300 and the Aero basic
//! pool, the pool's active liquidity 80173935548453779) plus the fee measurement of the
//! recorded chudmax history (tests_fee_yield.rs). The rung depths, amounts and splits
//! below are fixture inputs only, never presets.
//!
//! Every expected number was computed independently, by scripts that share no code with
//! the worker: /tmp/nn-v090/rungs_fixture/rungs_check.py (rung sizing, gates, fees, in-range
//! share and replay from the recorded rows) and /tmp/nn-v090/rungs_fixture/cap_check.py
//! (the bottle's cap: the band containing the tick, the budget and the near-USDC limit).
use super::*;
use crate::tests_fee_yield::{fee_fixture, V4_ID};
use crate::tests_ladder::{fixture, fixture_at_sqrt, fixture_with_rungs};
use serde_json::Value;

fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-11-20T12:05:00Z").unwrap().with_timezone(&Utc)
}

fn req(pairs: &[(&str, &str)]) -> Result<LadderRequest, LadderError> {
    LadderRequest::from_params(&pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect())
}

pub(crate) fn sim_on(env: &SnapshotEnvelope, pairs: &[(&str, &str)]) -> Result<Value, LadderError> {
    simulate_ladder(env, &req(pairs)?, now(), "ok")
}

fn sim(env: &SnapshotEnvelope, pairs: &[(&str, &str)]) -> Value {
    sim_on(env, pairs).unwrap_or_else(|e| panic!("not simulated: {e:?}"))
}

fn st<'a>(v: &'a Value, ptr: &str) -> &'a str {
    v.pointer(ptr).and_then(|x| x.as_str()).unwrap_or_else(|| panic!("{ptr} missing in {v}"))
}

fn f(v: &Value, ptr: &str) -> f64 {
    st(v, ptr).parse().unwrap()
}

fn rel(a: f64, b: f64) -> f64 {
    ((a - b) / b).abs()
}

/// r1 near $100 spot..$0.012 and r2 deep $500 $0.011..$0.01, snapped to the v4 pool.
const TWO: [(&str, &str); 9] = [
    ("position_id", V4_ID),
    ("r1_rung", "near"),
    ("r1_usdc", "100"),
    ("r1_high", "spot"),
    ("r1_low", "0.012"),
    ("r2_rung", "deep"),
    ("r2_usdc", "500"),
    ("r2_high", "0.011"),
    ("r2_low", "0.01"),
];

const FLAT_741: [(&str, &str); 4] = [("usdc", "741"), ("low", "0.01"), ("high", "spot"), ("position_id", V4_ID)];

fn with_bottom(mut env: SnapshotEnvelope, bottom: &str) -> SnapshotEnvelope {
    env.data.derived_metrics.global_status.ladder_bottom_usd = Some(bottom.to_string());
    env
}

fn bottle() -> Vec<LadderRungConfig> {
    vec![
        LadderRungConfig { rung: "cap".into(), share: None, low_usd: None },
        LadderRungConfig { rung: "near".into(), share: Some(0.15), low_usd: Some(0.0110) },
        LadderRungConfig { rung: "deep".into(), share: Some(0.85), low_usd: Some(0.01) },
    ]
}

/// The operator's settings on an envelope: ladder_bottom_usd 0.01 and a shape.
fn with_shape(mut env: SnapshotEnvelope, shape: &[LadderRungConfig]) -> SnapshotEnvelope {
    assert_eq!(validate_ladder_rungs(shape, Some(0.01)), Ok(()));
    let gs = &mut env.data.derived_metrics.global_status;
    gs.ladder_bottom_usd = Some("0.01000000".into());
    gs.ladder_rungs = Some(ladder_rung_settings(shape));
    env
}

/// sqrtPriceX96 at a fractional tick (USDC token0, NOCK token1).
fn sqrt_at_tick(t: f64) -> U256 {
    crate::tests_ladder::sqrt_x96_of_raw(1.0001f64.powf(t))
}

// ------------------------------------------------------------------
// The fee estimate
// ------------------------------------------------------------------

#[test]
fn test_flat_fee_estimate_from_the_recorded_history() {
    let b = sim(&fee_fixture(), &FLAT_741);
    let fe = &b["fee_estimate"];
    assert_eq!(st(fe, "/status"), "measured");
    assert_eq!(st(fe, "/measured_by"), V4_ID);
    assert_eq!(st(fe, "/measured_label"), "v4 USDC/NOCK 3.00% #3163450");
    assert_eq!(fe["measurement_age_seconds"], 900);
    assert_eq!(st(fe, "/measured_days"), "6.33");
    assert_eq!(fe["intervals"], 1836);
    // Valued at the current pool price: $6.0811 per 1e15 L per day at $0.01294400.
    assert_eq!(st(fe, "/rate/usd_per_1e15_liquidity_day"), "6.0811");
    assert_eq!(st(fe, "/rate/valued_at_pool_price_usd"), "0.01294400");
    // L 5805238794314994 times the rate; diluted by A/(A+L) with A 80173935548453779.
    assert!(rel(f(&b, "/sizing/liquidity_raw"), 5805238794314994.0) < 1e-9);
    assert_eq!(st(fe, "/per_day_in_range_usd"), "35.30");
    assert_eq!(st(fe, "/per_100_usd_per_day_in_range_usd"), "4.76");
    assert_eq!(st(fe, "/per_day_in_range_diluted_usd"), "32.92");
    assert_eq!(st(fe, "/dilution/share_of_active_pct"), "6.75");
    assert_eq!(st(fe, "/dilution/active_liquidity_raw"), "80173935548453779");
    // Beside it, always: the in-range share of stored history and the replay bounds.
    assert_eq!(st(fe, "/in_range_share_stored_history/share_pct"), "34.78");
    assert_eq!(st(fe, "/in_range_share_stored_history/seconds"), "190295");
    assert_eq!(st(fe, "/in_range_share_stored_history/edge_seconds"), "300");
    assert_eq!(st(fe, "/stored_history_replay/inside_upper_usd"), "60.27");
    assert_eq!(st(fe, "/stored_history_replay/edge_upper_usd"), "0.71");
    assert!(st(fe, "/stored_history_replay/note").starts_with("At most"));
    // Measured inside the range: $27.36 per day over 2.20 days, 22% below the estimate.
    assert_eq!(st(fe, "/band_measured/usd_per_day"), "27.36");
    assert_eq!(st(fe, "/band_measured/days"), "2.20");
    assert_eq!(fe["coverage"]["whole"], true);
    let assumptions: Vec<&str> = fe["assumptions"].as_array().unwrap().iter().map(|a| a.as_str().unwrap()).collect();
    assert!(assumptions[0].starts_with("Fee intensity per unit of liquidity is taken as uniform across ticks within the measuring range"));
    assert!(assumptions.iter().any(|a| a.contains("valued at the current pool price")));
    assert!(assumptions.iter().any(|a| a.contains("'latest'")));
    assert_eq!(fe["constants"]["band_min_seconds"], 72000.0);
    assert_eq!(st(fe, "/constants/note"), "display constants, not settings");
    // The text says it earns nothing until the price falls to its snapped top.
    let text = ladder_text::ladder_text(&b);
    for needle in [
        "• Fees while in range (an estimate): $35.30/day, $32.92 diluted (it would be 6.75% of the pool's active liquidity); $4.76 per $100 of its value at spot per day; measured on v4 USDC/NOCK 3.00% #3163450 over 6.33 days (1,836 intervals, measured 15 min ago)",
        "not a measurement inside it",
        "• It earns nothing until the price falls to $0.01274292 (-1.55%)",
        "• In range over stored history: 34.78% of the measured time (190,295 s of 547,205 s); replayed over it, at most $60.27 from intervals wholly inside and at most $0.71 more",
        "• Measured inside this range: $27.36/day over 2.20 days",
        "• Daily spread (5 UTC days with at least 72,000 s measured): min $17.98, median $30.72, max $46.38 per day; every day: 2026-11-14 $",
        "• Fee estimate assumptions: Fee intensity per unit of liquidity is taken as uniform",
    ] {
        assert!(text.contains(needle), "missing {needle:?} in\n{text}");
    }
}

#[test]
fn test_fee_estimate_unavailable_and_partial_cases_say_why() {
    // No measurement in the snapshot (a 2.9.0 body, or no cycle has measured yet).
    let b = sim(&fixture(), &FLAT_741);
    assert_eq!(st(&b, "/fee_estimate/status"), "unavailable");
    assert!(st(&b, "/fee_estimate/reason").contains("has no fee_yield (2.10.0)"));
    // No measuring range in this pool.
    let mut env = fee_fixture();
    let fy = env.data.derived_metrics.fee_yield.as_mut().unwrap();
    fy.pools.clear();
    fy.excluded.push(FeeYieldExcluded { position_id: V4_ID.into(), reason: "staked: swap fees on staked liquidity go to the gauge, not to the LP".into() });
    let b = sim(&env, &FLAT_741);
    assert!(st(&b, "/fee_estimate/reason").contains("fees are measured per pool; no tracked range in this pool earns LP fees (uniswap-v4:base:3163450: staked"));
    // Too little data, and liquidity changing at every row.
    let mut env = fixture();
    let rows = crate::tests_fee_yield::history_rows();
    env.data.derived_metrics.fee_yield = Some(fee_yield_from_rows(&env.data.positions, &rows[..1], crate::tests_fee_yield::computed_at()));
    let b = sim(&env, &FLAT_741);
    assert!(st(&b, "/fee_estimate/reason").contains("too little history"), "{}", b["fee_estimate"]);
    let mut changing: Vec<FeeYieldRow> = rows[200..210].to_vec();
    for (i, r) in changing.iter_mut().enumerate() {
        r.liquidity = Some(1_000_000_000 + i as u128);
    }
    env.data.derived_metrics.fee_yield = Some(fee_yield_from_rows(&env.data.positions, &changing, crate::tests_fee_yield::computed_at()));
    let b = sim(&env, &FLAT_741);
    assert!(st(&b, "/fee_estimate/reason").contains("none of the 9 intervals"), "{}", b["fee_estimate"]);
    // A tick spacing the caller states names no tracked pool.
    let b = sim(&fee_fixture(), &[("usdc", "741"), ("low", "0.01"), ("high", "spot"), ("tick_spacing", "300")]);
    assert!(st(&b, "/fee_estimate/reason").contains("a tick spacing you state names no tracked pool"));
    // Unsnapped: the rate applies; the band, share and replay are null with the reason.
    let b = sim(&fee_fixture(), &[("usdc", "741"), ("low", "0.01"), ("high", "spot")]);
    let fe = &b["fee_estimate"];
    assert_eq!(st(fe, "/status"), "measured");
    assert!(fe["per_day_in_range_usd"].is_string());
    for (k, why) in [("band_measured", "band_reason"), ("in_range_share_stored_history", "in_range_share_reason"), ("stored_history_replay", "replay_reason")] {
        assert!(fe[k].is_null(), "{k}");
        assert!(st(fe, &format!("/{why}")).starts_with("unsnapped"), "{why}");
    }
    // The pool's active liquidity not read: only the diluted figure is null.
    let mut env = fee_fixture();
    env.data.pool_mechanics[0].active_liquidity_raw = None;
    let b = sim(&env, &FLAT_741);
    assert_eq!(st(&b, "/fee_estimate/per_day_in_range_usd"), "35.30");
    assert!(b["fee_estimate"]["per_day_in_range_diluted_usd"].is_null());
    assert!(st(&b, "/fee_estimate/dilution_reason").contains("only the undiluted figure is stated"));
    // A range partly outside the measuring range: stated, and wholly outside: null.
    let b = sim(&fee_fixture(), &[("usdc", "741"), ("low", "0.004"), ("high", "spot"), ("position_id", V4_ID)]);
    assert_eq!(b["fee_estimate"]["coverage"]["whole"], false);
    assert!(st(&b, "/fee_estimate/coverage/note").contains("outside it no fees are measured"));
    let b = sim(&fee_fixture(), &[("usdc", "741"), ("low", "0.003"), ("high", "0.005"), ("position_id", V4_ID)]);
    assert_eq!(st(&b, "/fee_estimate/status"), "unavailable");
    assert!(st(&b, "/fee_estimate/reason").contains("wholly outside the measuring range's $0.00550148..$0.04105518"), "{}", b["fee_estimate"]["reason"]);
}

#[test]
fn test_the_in_range_test_is_half_open_at_the_upper_tick() {
    // Spans placed by hand: one bin inside 273900..276300, one exactly at its upper tick
    // (276300, which is out of range: tickLower <= tick < tickUpper), one straddling it.
    let mut env = fee_fixture();
    let pool = &mut env.data.derived_metrics.fee_yield.as_mut().unwrap().pools[0];
    pool.measured_seconds = 10000.0;
    pool.spans = vec![
        FeeYieldSpan { bin_lo: 276000, bin_hi: 276000, seconds: 1000.0, usdc_per_liquidity: "1e-18".into(), nock_per_liquidity: "0e0".into() },
        FeeYieldSpan { bin_lo: 276300, bin_hi: 276300, seconds: 500.0, usdc_per_liquidity: "1e-18".into(), nock_per_liquidity: "0e0".into() },
        FeeYieldSpan { bin_lo: 276000, bin_hi: 276300, seconds: 200.0, usdc_per_liquidity: "1e-18".into(), nock_per_liquidity: "0e0".into() },
    ];
    let b = sim(&env, &FLAT_741);
    let sh = &b["fee_estimate"]["in_range_share_stored_history"];
    assert_eq!((st(sh, "/seconds"), st(sh, "/edge_seconds"), st(sh, "/share_pct")), ("1000", "200", "10.00"));
    // Below the band minimum: the dollars and seconds only, never a per-day figure.
    assert!(b["fee_estimate"]["band_measured"]["usd_per_day"].is_null());
    assert!(st(&b, "/fee_estimate/band_reason").contains("only 1000 s of stored intervals sat wholly inside"));
}

// ------------------------------------------------------------------
// A ladder of rungs
// ------------------------------------------------------------------

#[test]
fn test_two_rungs_share_one_reserve_gate_and_one_budget() {
    let b = sim(&fee_fixture(), &TWO);
    let r1 = &b["rungs"][0];
    let r2 = &b["rungs"][1];
    assert_eq!((r1["index"].as_u64(), st(r1, "/rung")), (Some(1), "near"));
    assert_eq!((r1["bounds"]["tick_lower"].as_i64(), r1["bounds"]["tick_upper"].as_i64()), (Some(273900), Some(274500)));
    assert_eq!((r2["bounds"]["tick_lower"].as_i64(), r2["bounds"]["tick_upper"].as_i64()), (Some(275400), Some(276300)));
    assert_eq!((st(r1, "/bounds/low_usd"), st(r1, "/bounds/high_usd")), ("0.01200086", "0.01274292"));
    assert_eq!((st(r2, "/bounds/low_usd"), st(r2, "/bounds/high_usd")), ("0.01002405", "0.01096801"));
    assert!(rel(f(r1, "/sizing/liquidity_raw"), 2997532031710744.0) < 1e-9);
    assert!(rel(f(r2, "/sizing/liquidity_raw"), 10850499376755608.0) < 1e-9);
    assert_eq!(st(r1, "/bottom/nock_held"), "8086.4763");
    assert_eq!(st(r1, "/bottom/avg_buy_price_usd"), "0.01236633");
    assert_eq!(st(r2, "/bottom/nock_held"), "47685.2789");
    assert_eq!(st(r2, "/bottom/avg_buy_price_usd"), "0.01048542");
    // Combined.
    let c = &b["combined"];
    assert_eq!((st(c, "/total_usdc"), st(c, "/total_nock_needed")), ("600.00", "0.0000"));
    assert_eq!(st(c, "/nock_held_at_deepest_bottom"), "55771.7551");
    assert_eq!(st(c, "/nock_bought_at_deepest_bottom"), "55771.7551");
    assert_eq!(st(c, "/avg_buy_price_usd"), "0.01075813");
    assert_eq!((st(c, "/nock_kept_if_deep_rungs_pulled"), st(c, "/avg_kept_price_usd")), ("47685.2789", "0.01048542"));
    assert_eq!(c["max_rungs"], 4);
    assert_eq!(st(c, "/max_rungs_note"), "display constants, not settings");
    // The shared reserve gate at the deepest bottom ($0.01002405, ladder_bottom_usd not
    // set): the same most as the one-rung ladder to that bottom.
    let rc = &b["reserve_check"];
    assert_eq!(st(rc, "/gate_price_usd"), "0.01002405");
    assert!(st(rc, "/gate_price_rule").starts_with("the deepest rung's bottom: operator_profile.ladder_bottom_usd is not set"));
    assert_eq!((st(rc, "/existing_ranges_usdc_at_bottom"), st(rc, "/stable_at_bottom_usdc"), st(rc, "/margin_usdc")), ("171.62", "1117.45", "167.45"));
    assert_eq!(st(rc, "/max_usdc_for_this_range"), "767.45");
    assert_eq!(st(c, "/max_usdc_total"), "767.45");
    let flat = sim(&fee_fixture(), &FLAT_741);
    assert_eq!(st(&flat, "/reserve_check/max_usdc_for_this_range"), "767.45");
    assert_eq!((st(r1, "/max_usdc_with_the_others_as_given"), st(r2, "/max_usdc_with_the_others_as_given")), ("267.45", "667.45"));
    // One budget: both tops at or below spot need no NOCK.
    assert_eq!((st(&b, "/floor_check/status"), st(&b, "/floor_check/nock_needed")), ("fits", "0.0000"));
    // Fees per rung, each beside its in-range share and replay bounds.
    let fe1 = &r1["fee_estimate"];
    assert_eq!((st(fe1, "/per_day_in_range_usd"), st(fe1, "/per_day_in_range_diluted_usd")), ("18.23", "17.57"));
    assert_eq!(st(fe1, "/in_range_share_stored_history/share_pct"), "0.11");
    assert_eq!((st(fe1, "/stored_history_replay/inside_upper_usd"), st(fe1, "/stored_history_replay/edge_upper_usd")), ("1.31", "4.70"));
    assert!(fe1["band_measured"]["usd_per_day"].is_null());
    assert!(st(fe1, "/band_reason").contains("only 600 s"));
    let fe2 = &r2["fee_estimate"];
    assert_eq!((st(fe2, "/per_day_in_range_usd"), st(fe2, "/per_day_in_range_diluted_usd")), ("65.98", "58.12"));
    assert_eq!(st(fe2, "/in_range_share_stored_history/share_pct"), "5.82");
    assert_eq!((st(fe2, "/stored_history_replay/inside_upper_usd"), st(fe2, "/stored_history_replay/edge_upper_usd")), ("25.00", "38.41"));
    // A grid row sums only the rungs in range at that price.
    let row = |pct: &str| b["grid"].as_array().unwrap().iter().find(|r| r["pct_of_spot"] == pct).unwrap().clone();
    assert_eq!(row("-5.00")["rungs_in_range"], json!([1]));
    assert_eq!((st(&row("-5.00"), "/est_fee_usd_per_day"), st(&row("-5.00"), "/est_fee_usd_per_day_diluted")), ("18.23", "17.57"));
    assert_eq!(row("-10.00")["rungs_in_range"], json!([]));
    assert_eq!(st(&row("-10.00"), "/est_fee_usd_per_day"), "0.00");
    assert_eq!(row("-20.00")["rungs_in_range"], json!([2]));
    assert_eq!((st(&row("-20.00"), "/est_fee_usd_per_day"), st(&row("-20.00"), "/est_fee_usd_per_day_diluted")), ("65.98", "58.12"));
    // Every existing top-level key is kept, and the sums are labelled.
    for k in ["snapshot", "inputs", "pool", "bounds", "top_neighbours", "sizing", "grid", "bottom", "top", "reserve_check", "floor_check", "assumptions", "statement"] {
        assert!(b.get(k).is_some(), "{k}");
    }
    assert!(st(&b, "/sizing/describes").starts_with("the sum over the rungs"));
    assert_eq!(st(&b, "/bottom/nock_held"), "55771.7551");
}

#[test]
fn test_the_gate_is_read_at_the_configured_ladder_bottom() {
    let near_only = [("position_id", V4_ID), ("r1_rung", "near"), ("r1_usdc", "300"), ("r1_high", "spot"), ("r1_low", "0.012")];
    // Without ladder_bottom_usd the gate is the rung's own bottom, and it says so.
    let b = sim(&fixture(), &near_only);
    assert_eq!((st(&b, "/reserve_check/gate_price_usd"), st(&b, "/reserve_check/margin_usdc")), ("0.01200086", "503.43"));
    assert_eq!(st(&b, "/combined/max_usdc_total"), "803.42");
    assert!(st(&b, "/combined/gate_price_rule").contains("is not set, so the gate falls back to it"));
    // With it, a ladder stopping at $0.012 is read at $0.01: no false margin.
    let b = sim(&with_bottom(fixture(), "0.01000000"), &near_only);
    assert_eq!((st(&b, "/reserve_check/gate_price_usd"), st(&b, "/reserve_check/margin_usdc")), ("0.01000000", "466.99"));
    assert_eq!(st(&b, "/combined/max_usdc_total"), "766.99");
    assert!(st(&b, "/combined/gate_price_rule").starts_with("your ladder bottom (operator_profile.ladder_bottom_usd $0.01000000)"));
    // A deeper rung than the setting keeps its own bottom.
    let b = sim(&with_bottom(fixture(), "0.01100000"), &TWO);
    assert_eq!(st(&b, "/reserve_check/gate_price_usd"), "0.01002405");
    assert!(st(&b, "/reserve_check/gate_price_rule").starts_with("the deepest rung's bottom, at or below your ladder bottom"));
    // The one-rung form: with the key set it is gated there too, and says so...
    let b = sim(&with_bottom(fixture(), "0.01000000"), &FLAT_741);
    assert_eq!((st(&b, "/reserve_check/gate_price_usd"), st(&b, "/reserve_check/margin_usdc")), ("0.01000000", "25.99"));
    assert_eq!(st(&b, "/reserve_check/max_usdc_for_this_range"), "766.99");
    assert!(st(&b, "/reserve_check/gate").contains("operator_profile.ladder_bottom_usd"));
    // ...and with it unset the body has no gate fields (the golden pins its bytes).
    let b = sim(&fixture(), &FLAT_741);
    assert!(b["reserve_check"].get("gate_price_usd").is_none());
    assert_eq!(st(&b, "/reserve_check/margin_usdc"), "26.45");
    // An invalid setting is stated and never applied.
    let mut env = fixture();
    env.data.derived_metrics.global_status.ladder_bottom_note = Some(LADDER_BOTTOM_INVALID_NOTE.into());
    let b = sim(&env, &near_only);
    assert!(st(&b, "/combined/gate_price_rule").contains("must be above 0 USD per NOCK; it is not applied"));
    assert_eq!(st(&b, "/reserve_check/gate_price_usd"), "0.01200086");
}

#[test]
fn test_the_snapshot_states_the_ladder_bottom_from_config() {
    let profile = |v: Option<f64>| OperatorProfile { ladder_bottom_usd: v, ..OperatorProfile::default() };
    let gs = |p: &OperatorProfile| {
        let raw = RawOnChainData::default();
        compute_derived_metrics_with_plan(&raw, 0.0, p, compute_structural_channels_sync(&[]), &[], &LedgerAggregate::default(), &MarketRegime::default(), &PlanContext::none()).global_status
    };
    let set = gs(&profile(Some(0.01)));
    assert_eq!((set.ladder_bottom_usd.as_deref(), set.ladder_bottom_note.as_deref()), (Some("0.01000000"), None));
    let unset = gs(&profile(None));
    assert_eq!((unset.ladder_bottom_usd.as_deref(), unset.ladder_bottom_note.as_deref()), (None, None));
    let bad = gs(&profile(Some(0.0)));
    assert_eq!((bad.ladder_bottom_usd.as_deref(), bad.ladder_bottom_note.as_deref()), (None, Some(LADDER_BOTTOM_INVALID_NOTE)));
    // Parsed from YAML with no default; the example documents the keys, commented out.
    let p: OperatorProfile = serde_yaml::from_str("ladder_bottom_usd: 0.01\nladder_rungs:\n  - rung: cap\n  - rung: near\n    share: 0.15\n    low_usd: 0.0110\n  - rung: deep\n    share: 0.85\n    low_usd: 0.01\n").unwrap();
    assert_eq!(p.ladder_bottom_usd, Some(0.01));
    assert_eq!(p.ladder_rungs.as_deref(), Some(bottle().as_slice()));
    let none: OperatorProfile = serde_yaml::from_str("treasury_floor_nock: 1\n").unwrap();
    assert_eq!((none.ladder_bottom_usd, none.ladder_rungs), (None, None));
    let example = include_str!("../config.yaml.example");
    assert!(example.contains("# ladder_bottom_usd:") && example.contains("# ladder_rungs:"));
}

#[test]
fn test_the_shared_nock_budget_sums_every_rung() {
    // r1's top above spot needs NOCK; r2 needs none. The sum meets one budget.
    let pairs = [
        ("position_id", V4_ID),
        ("r1_rung", "near"),
        ("r1_usdc", "300"),
        ("r1_high_pct", "2"),
        ("r1_low", "0.012"),
        ("r2_rung", "deep"),
        ("r2_usdc", "300"),
        ("r2_high", "0.011"),
        ("r2_low", "0.01"),
    ];
    let b = sim(&fixture(), &pairs);
    let n1 = f(&b, "/rungs/0/sizing/nock_needed_at_spot");
    assert!(n1 > 86.7891);
    assert_eq!(st(&b, "/rungs/1/sizing/nock_needed_at_spot"), "0.0000");
    assert!((f(&b, "/floor_check/nock_needed") - n1).abs() < 1e-4);
    assert_eq!(st(&b, "/floor_check/status"), "exceeds");
    assert_eq!(b["floor_check"]["nock_needed_by_rung"].as_array().unwrap().len(), 2);
    let cap = (600.0 * 86.7891 / n1 * 100.0 + 1e-7).floor() / 100.0;
    assert_eq!(st(&b, "/floor_check/max_usdc_within_budget"), format!("{cap:.2}"));
    // Its floor-crossing effect counts both rungs: more NOCK out of the wallet lowers the
    // crossing at least as much as r1 alone.
    let one = sim(&fixture(), &pairs[..5]);
    let both_after = f(&b, "/floor_check/floor_breach_effect/after/crossing/multiple");
    let one_after = f(&one, "/floor_check/floor_breach_effect/after/crossing/multiple");
    let before = f(&b, "/floor_check/floor_breach_effect/before/crossing/multiple");
    assert!(both_after <= one_after && one_after < before, "{both_after} {one_after} {before}");
    assert_eq!(st(&b, "/floor_check/floor_breach_effect/status"), "lowers");
}

#[test]
fn test_overlapping_rungs_are_allowed_and_listed() {
    let pairs = [
        ("position_id", V4_ID),
        ("r1_rung", "near"),
        ("r1_usdc", "200"),
        ("r1_high", "spot"),
        ("r1_low", "0.0105"),
        ("r2_rung", "deep"),
        ("r2_usdc", "200"),
        ("r2_high", "0.011"),
        ("r2_low", "0.01"),
    ];
    let b = sim(&fee_fixture(), &pairs);
    let o = &b["combined"]["overlaps"][0];
    assert_eq!((o["a"].as_u64(), o["b"].as_u64()), (Some(1), Some(2)));
    assert_eq!(st(o, "/low_usd"), st(&b, "/rungs/0/bounds/low_usd"));
    assert_eq!(st(o, "/high_usd"), st(&b, "/rungs/1/bounds/high_usd"));
    // Each rung's dilution counts the other's liquidity too.
    let l1 = f(&b, "/rungs/0/sizing/liquidity_raw");
    let l2 = f(&b, "/rungs/1/sizing/liquidity_raw");
    assert!(rel(f(&b, "/rungs/0/fee_estimate/dilution/new_liquidity_raw"), l1 + l2) < 1e-9);
    let text = ladder_text::ladder_text(&b);
    assert!(text.contains("• Overlaps: r1 and r2 at $"), "{text}");
}

#[test]
fn test_a_near_rung_round_trip_hands_back_its_usdc_and_a_deep_rung_keeps_its_nock_if_pulled() {
    let b = sim(&fee_fixture(), &TWO);
    let rt = &b["rungs"][0]["round_trip"];
    assert_eq!((st(rt, "/usdc_start"), st(rt, "/usdc_back")), ("100.00", "100.00"));
    assert_eq!((st(rt, "/nock_start"), st(rt, "/nock_back")), ("0.0000", "0.0000"));
    assert!(st(rt, "/text").contains("the round trip's gain is the fees it earned in range"));
    assert!(b["rungs"][0]["if_pulled_at_fill"].is_null());
    let p = &b["rungs"][1]["if_pulled_at_fill"];
    assert_eq!(st(p, "/nock_kept"), st(&b, "/rungs/1/bottom/nock_held"));
    assert_eq!(st(p, "/avg_price_usd"), "0.01048542");
    assert!(st(p, "/note").contains("Pulling is a withdrawal you sign"));
    assert!(b["rungs"][1]["round_trip"].is_null());
    // A near rung whose top is above spot: back at the price now it holds what it started with.
    let b = sim(&fixture(), &[("position_id", V4_ID), ("r1_rung", "near"), ("r1_usdc", "250"), ("r1_high_pct", "3"), ("r1_low", "0.012")]);
    let rt = &b["rungs"][0]["round_trip"];
    assert_eq!(st(rt, "/usdc_back"), "250.00");
    assert_eq!(st(rt, "/nock_back"), st(rt, "/nock_start"));
}

#[test]
fn test_the_tradeoff_tables_are_measurements_in_order_with_no_ranking() {
    let b = sim(&fee_fixture(), &TWO);
    let t = &b["tradeoff"];
    assert_eq!(st(t, "/statement"), "Rows are measurements at your depths; NockNock does not choose the split or the depths.");
    let rows = t["split"]["rows"].as_array().unwrap();
    // 0..100 in steps of 10 (a display step, echoed) plus the operator's own 16.67%.
    assert_eq!(t["split"]["step_pct"], 10.0);
    assert_eq!(rows.len(), 12);
    let shares: Vec<f64> = rows.iter().map(|r| f(r, "/first_rung_share_pct")).collect();
    let mut sorted = shares.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    assert_eq!(shares, sorted);
    assert!(rows.iter().all(|r| r.get("yours").is_none() && r.get("rank").is_none()));
    // The 0% and 100% rows are each rung alone with the whole $600.
    let alone = |i: usize| {
        let p: Vec<(&str, &str)> = [("position_id", V4_ID)].into_iter().chain(TWO[1 + 4 * i..5 + 4 * i].iter().map(|(k, v)| (*k, *v))).collect();
        let p: Vec<(String, String)> = p.iter().map(|(k, v)| (k.replace(&format!("r{}_", i + 1), "r1_"), if k.ends_with("_usdc") { "600".to_string() } else { v.to_string() })).collect();
        let p: Vec<(&str, &str)> = p.iter().map(|(k, v)| (k.as_str(), v.as_str())).collect();
        sim(&fee_fixture(), &p)
    };
    let deep_alone = alone(1);
    assert_eq!(rows[0]["per_rung"][1]["fee_per_day_in_range_usd"], deep_alone["rungs"][0]["fee_estimate"]["per_day_in_range_usd"]);
    assert_eq!(rows[0]["nock_at_deepest_bottom"], deep_alone["combined"]["nock_held_at_deepest_bottom"]);
    assert_eq!(rows[0]["per_rung"][0]["note"], "no USDC in this rung in this row");
    let near_alone = alone(0);
    let last = rows.last().unwrap();
    assert_eq!(last["per_rung"][0]["fee_per_day_in_range_usd"], near_alone["rungs"][0]["fee_estimate"]["per_day_in_range_usd"]);
    assert_eq!(last["avg_buy_price_usd"], near_alone["combined"]["avg_buy_price_usd"]);
    // The operator's own split is the main body, unmarked.
    let own = rows.iter().find(|r| st(r, "/first_rung_share_pct") == "16.67").unwrap();
    assert_eq!(own["per_rung"][1]["fee_per_day_in_range_usd"], b["rungs"][1]["fee_estimate"]["per_day_in_range_usd"]);
    assert_eq!(own["nock_at_deepest_bottom"], b["combined"]["nock_held_at_deepest_bottom"]);
    // Every row shows the in-range share and the replay beside the fee per day in range.
    for r in rows {
        for x in r["per_rung"].as_array().unwrap() {
            for k in ["fee_per_day_in_range_usd", "fee_per_100_usd_per_day_in_range_usd", "in_range_share_pct", "replay_inside_upper_usd", "replay_edge_upper_usd"] {
                assert!(x.get(k).is_some(), "{k}");
            }
        }
    }
    // Depth tables: the near rung's bottom from one spacing below its top to the deep
    // rung's top (274200..275400: 5 ticks), the deep rung's top from the near rung's
    // bottom to one spacing above its own bottom (274500..276000: 6 ticks).
    let nd = t["near_depth"]["rows"].as_array().unwrap();
    assert_eq!(nd.len(), 5);
    assert_eq!(st(&nd[0], "/near_low_usd"), "0.01236633");
    assert_eq!(st(&nd[4], "/near_low_usd"), "0.01096801");
    let dt = t["deep_top"]["rows"].as_array().unwrap();
    assert_eq!(dt.len(), 6);
    assert_eq!(st(&dt[0], "/deep_high_usd"), "0.01200086");
    assert_eq!(t["near_depth"]["max_rows"], 12);
    // A larger split step from the caller: 0, 25, 50, 75, 100 and the operator's own.
    let mut p = TWO.to_vec();
    p.push(("split_step_pct", "25"));
    assert_eq!(sim(&fee_fixture(), &p)["tradeoff"]["split"]["rows"].as_array().unwrap().len(), 6);
    // Three rungs: no split table, and two deep rungs: no depth tables; each says why.
    let mut three = TWO.to_vec();
    three.extend([("r3_rung", "deep"), ("r3_usdc", "50"), ("r3_high", "0.0101"), ("r3_low", "0.0098")]);
    let b3 = sim(&fee_fixture(), &three);
    assert!(b3["tradeoff"]["split"].is_null());
    assert_eq!(st(&b3, "/tradeoff/split_reason"), "the split table is drawn for two rungs");
    let mut deeps = TWO.to_vec();
    deeps[1] = ("r1_rung", "deep");
    assert_eq!(st(&sim(&fee_fixture(), &deeps), "/tradeoff/near_depth_reason"), "the depth tables are drawn for one near and one deep rung");
    // No ranking words anywhere, in the body or the text.
    for body in [&b, &b3] {
        // The #166 floor check's rebuild_suggested_nock names the rebuild's own field; the
        // rungs, the combined block, the trade-off and the whole text are scanned.
        let all = format!("{}\n{}\n{}\n{}", body["rungs"], body["combined"], body["tradeoff"], ladder_text::ladder_text(body)).to_lowercase();
        for w in ["best", "should", "recommend", "suggest", "deposit"] {
            assert!(!all.contains(w), "{w}");
        }
    }
}

#[test]
fn test_rung_parameters_are_validated() {
    for (pairs, needle) in [
        (vec![("usdc", "100"), ("low", "0.01"), ("high", "spot"), ("r1_rung", "near"), ("r1_usdc", "1"), ("r1_low", "0.01"), ("r1_high", "spot")], "is the one-rung form"),
        (vec![("r1_rung", "near"), ("r1_usdc", "1"), ("r1_low", "0.01"), ("r1_high", "spot"), ("r3_rung", "deep"), ("r3_usdc", "1"), ("r3_low", "0.01"), ("r3_high", "0.011")], "r2 is missing"),
        (vec![("r1_usdc", "1"), ("r1_low", "0.01"), ("r1_high", "spot")], "r1_rung is required (cap, near or deep): NockNock assumes none"),
        (vec![("r1_rung", "middle"), ("r1_usdc", "1"), ("r1_low", "0.01"), ("r1_high", "spot")], "r1_rung must be cap, near or deep"),
        (vec![("r5_rung", "deep")], "at most 4 rungs"),
        (vec![("r1_rung", "near"), ("r1_bottom", "0.01")], "unknown rung parameter 'r1_bottom'"),
        (vec![("r1_rung", "near"), ("r1_usdc", "0"), ("r1_low", "0.01"), ("r1_high", "spot")], "r1_usdc must be above 0"),
        (vec![("r1_rung", "near"), ("r1_usdc", "1"), ("r1_high", "spot")], "r1_low or r1_low_pct is required"),
        (vec![("usdc", "1"), ("low", "0.01"), ("high", "spot"), ("split_step_pct", "10")], "split_step_pct applies to a ladder of rungs"),
        (vec![("r1_rung", "near"), ("r1_usdc", "1"), ("r1_low", "0.01"), ("r1_high", "spot"), ("split_step_pct", "3")], "split_step_pct must be from 5 to 50"),
        (vec![("shape", "configured"), ("usdc", "1")], "shape=configured draws the rungs from operator_profile.ladder_rungs, not both"),
        (vec![("shape", "configured"), ("r1_rung", "near")], "give it or indexed rungs (r1_..), not both"),
        (vec![("shape", "mine")], "shape must be 'configured'"),
        (vec![("shape_usdc", "10")], "shape_usdc applies only with shape=configured"),
    ] {
        let e = sim_on(&fixture(), &pairs).expect_err("refused");
        assert_eq!((e.status, e.error), (400, "INVALID_PARAMETER"), "{pairs:?}");
        assert!(e.detail.contains(needle), "{pairs:?}: {}", e.detail);
    }
    for (pairs, code, needle) in [
        (vec![("position_id", V4_ID), ("r1_rung", "near"), ("r1_usdc", "100"), ("r1_low", "0.0127"), ("r1_high", "0.0128")], "RUNG_TOO_NARROW", "r1 (near): both bounds snap to tick 273900"),
        (vec![("r1_rung", "near"), ("r1_usdc", "1000"), ("r1_low", "0.01"), ("r1_high", "spot"), ("r2_rung", "deep"), ("r2_usdc", "1000"), ("r2_low", "0.009"), ("r2_high", "0.01")], "RUNGS_EXCEED_WALLET", "(r1 $1000.00, r2 $1000.00)"),
        (vec![("r1_rung", "near"), ("r1_usdc", "1"), ("r1_low", "0.01"), ("r1_high", "spot"), ("r2_rung", "deep"), ("r2_usdc", "1"), ("r2_low", "0.013"), ("r2_high", "0.014")], "RANGE_ABOVE_SPOT", "r2 (deep): the bottom $0.01300000"),
        (vec![("r1_rung", "near"), ("r1_usdc", "100"), ("r1_low", "0.01"), ("r1_high", "spot"), ("r1_nock", "50")], "NOCK_TOO_MUCH", "r1 (near): this range at the pool price takes 0.0000 NOCK"),
        (vec![("shape", "configured")], "SHAPE_NOT_CONFIGURED", "operator_profile.ladder_rungs is not set"),
    ] {
        let e = sim_on(&fixture(), &pairs).expect_err("not simulated");
        assert_eq!((e.status, e.code), (422, Some(code)), "{pairs:?}: {}", e.detail);
        assert!(e.detail.contains(needle), "{pairs:?}: {}", e.detail);
    }
}

// ------------------------------------------------------------------
// The operator's shape: the bottle and its cap
// ------------------------------------------------------------------

#[test]
fn test_the_shape_setting_is_validated_and_never_completed() {
    let b = |v: &[(&str, Option<f64>, Option<f64>)]| v.iter().map(|(r, s, l)| LadderRungConfig { rung: r.to_string(), share: *s, low_usd: *l }).collect::<Vec<_>>();
    assert_eq!(validate_ladder_rungs(&bottle(), Some(0.01)), Ok(()));
    assert_eq!(validate_ladder_rungs(&bottle()[1..], None), Ok(()));
    // A shape may stop above the ladder bottom; the gate is still read there.
    assert_eq!(validate_ladder_rungs(&b(&[("near", Some(0.5), Some(0.012)), ("deep", Some(0.5), Some(0.011))]), Some(0.01)), Ok(()));
    for (shape, needle) in [
        (b(&[]), "it lists no rung"),
        (b(&[("near", Some(0.15), Some(0.011)), ("deep", Some(0.80), Some(0.01))]), "the near and deep shares sum to 0.9500000000000001, not 1"),
        (b(&[("near", Some(0.5), Some(0.01)), ("deep", Some(0.5), Some(0.011))]), "the bottoms must strictly decrease"),
        (b(&[("near", Some(0.5), Some(0.011)), ("deep", Some(0.5), Some(0.011))]), "the bottoms must strictly decrease"),
        (b(&[("near", Some(0.5), Some(0.011)), ("deep", Some(0.5), Some(0.009))]), "is below operator_profile.ladder_bottom_usd 0.01"),
        (b(&[("middle", Some(1.0), Some(0.011))]), "unknown rung 'middle'"),
        (b(&[("near", Some(0.0), Some(0.011)), ("deep", Some(1.0), Some(0.01))]), "share 0 is not above 0"),
        (b(&[("near", None, Some(0.011))]), "has no share"),
        (b(&[("near", Some(1.0), None)]), "has no low_usd"),
        (b(&[("near", Some(0.5), Some(0.011)), ("cap", None, None), ("deep", Some(0.5), Some(0.01))]), "a cap is the first rung, and there is at most one"),
        (b(&[("cap", None, None), ("cap", None, None), ("deep", Some(1.0), Some(0.01))]), "at most one"),
        (b(&[("cap", Some(0.1), None), ("deep", Some(1.0), Some(0.01))]), "the cap takes no share and no low_usd"),
        (b(&[("cap", None, None)]), "it lists only a cap"),
        (b(&[("near", Some(0.2), Some(0.02)), ("near", Some(0.2), Some(0.019)), ("near", Some(0.2), Some(0.018)), ("near", Some(0.2), Some(0.017)), ("deep", Some(0.2), Some(0.016))]), "draws at most 4"),
    ] {
        let e = validate_ladder_rungs(&shape, Some(0.01)).unwrap_err();
        assert!(e.starts_with("operator_profile.ladder_rungs is a configuration error: ") && e.ends_with("; it is not applied"), "{e}");
        assert!(e.contains(needle), "{needle}: {e}");
    }
    // In the snapshot: the setting as validated, or the error named, never both.
    let profile = |r: Vec<LadderRungConfig>| OperatorProfile { ladder_bottom_usd: Some(0.01), ladder_rungs: Some(r), ..OperatorProfile::default() };
    let gs = |p: &OperatorProfile| {
        compute_derived_metrics_with_plan(&RawOnChainData::default(), 0.0, p, compute_structural_channels_sync(&[]), &[], &LedgerAggregate::default(), &MarketRegime::default(), &PlanContext::none()).global_status
    };
    let ok = gs(&profile(bottle()));
    let rungs = ok.ladder_rungs.unwrap();
    assert_eq!(ok.ladder_rungs_note, None);
    assert_eq!(
        serde_json::to_value(&rungs).unwrap(),
        json!([
            {"rung": "cap", "share": null, "high": "band", "low_usd": null},
            {"rung": "near", "share": "0.15", "high": "spot", "low_usd": "0.01100000"},
            {"rung": "deep", "share": "0.85", "high": "0.01100000", "low_usd": "0.01000000"},
        ])
    );
    let bad = gs(&profile(b(&[("near", Some(0.5), Some(0.011))])));
    assert_eq!(bad.ladder_rungs, None);
    assert!(bad.ladder_rungs_note.unwrap().contains("sum to 0.5, not 1"));
}

#[test]
fn test_the_bottle_with_its_cap_on_the_fixture() {
    let env = with_shape(fee_fixture(), &bottle());
    let b = sim(&env, &[("shape", "configured"), ("position_id", V4_ID)]);
    let sh = &b["inputs"]["shape"];
    assert_eq!((st(sh, "/total_usdc"), st(sh, "/cap")), ("766.99", "sized"));
    assert!(st(sh, "/headroom_rule").starts_with("the shared reserve gate's most at $0.01000000"), "{}", st(sh, "/headroom_rule"));
    assert_eq!((st(sh, "/headroom_usdc"), st(sh, "/fit/status")), ("766.99", "fits"));
    // The cap: the band 273600..273900 containing tick 273743, holding the whole budget.
    let cap = &b["rungs"][0];
    assert_eq!((st(cap, "/rung"), cap["index"].as_u64()), ("cap", Some(1)));
    assert_eq!((cap["bounds"]["tick_lower"].as_i64(), cap["bounds"]["tick_upper"].as_i64()), (Some(273600), Some(273900)));
    assert_eq!((st(cap, "/bounds/low_usd"), st(cap, "/bounds/high_usd")), ("0.01274292", "0.01313098"));
    assert_eq!((st(cap, "/cap/bound_by"), st(cap, "/cap/nock"), st(cap, "/cap/usdc")), ("lp_nock_budget", "86.7891", "1.23"));
    assert!(st(cap, "/cap/band_rule").contains("the band 273600..273900 of the 300-tick grid contains the pool's tick 273743"));
    assert_eq!(st(cap, "/sizing/liquidity_human"), "1381.9212");
    assert_eq!(st(cap, "/sizing/nock_needed_at_spot"), "86.7891");
    assert!((f(cap, "/sizing/usdc_at_spot") - 1.225999).abs() < 0.006);
    // In range at spot: its fee per day while in range applies now.
    assert_eq!(cap["top"]["out_of_range_above_now"], false);
    assert_eq!(st(cap, "/fee_estimate/status"), "measured");
    // The near and deep rungs divide the USDC left after the cap, 15/85.
    let (near, deep) = (&b["rungs"][1], &b["rungs"][2]);
    assert_eq!((st(near, "/rung"), st(deep, "/rung")), ("near", "deep"));
    assert_eq!((near["bounds"]["tick_lower"].as_i64(), near["bounds"]["tick_upper"].as_i64()), (Some(273900), Some(275400)));
    assert_eq!((deep["bounds"]["tick_lower"].as_i64(), deep["bounds"]["tick_upper"].as_i64()), (Some(275400), Some(276300)));
    assert!((f(near, "/sizing/usdc_at_spot") - 114.8646).abs() < 0.006);
    assert!((f(deep, "/sizing/usdc_at_spot") - 650.8994).abs() < 0.006);
    assert!(f(near, "/sizing/usdc_at_spot") >= f(cap, "/sizing/usdc_at_spot"));
    // The three rungs add up to the gated total, and the reserve holds to the cent.
    let sum: f64 = (0..3).map(|i| f(&b["rungs"][i], "/sizing/usdc_at_spot")).sum();
    assert!((sum - 766.99).abs() < 0.011, "{sum}");
    assert_eq!(st(&b, "/combined/total_usdc"), "766.99");
    assert_eq!((st(&b, "/reserve_check/status"), st(&b, "/reserve_check/margin_usdc")), ("holds", "0.00"));
    assert_eq!(st(&b, "/reserve_check/gate_price_usd"), "0.01000000");
    // The budget gate sums the cap's NOCK; the others hold none at spot.
    assert_eq!((st(&b, "/floor_check/nock_needed"), st(&b, "/floor_check/status")), ("86.7891", "fits"));
    assert_eq!(st(&b, "/combined/nock_held_at_deepest_bottom"), "71974.8945");
    assert_eq!(st(&b, "/combined/nock_bought_at_deepest_bottom"), "71888.1054");
    assert_eq!(st(&b, "/combined/avg_buy_price_usd"), "0.01066922");
    // The cap's NOCK lowers the floor-crossing price slightly: a measurement.
    assert_eq!(st(&b, "/floor_check/floor_breach_effect/status"), "lowers");
    // The text names the shape as the operator's, and the cap's limit.
    let text = ladder_text::ladder_text(&b);
    assert!(text.contains("• Shape: your operator_profile.ladder_rungs (cap: the tick band containing spot; "), "{text}");
    assert!(text.contains("near 0.15 from spot down to $0.01100000; deep 0.85 from $0.01100000 down to $0.01000000), sized to $766.99 USDC"), "{text}");
    assert!(text.contains("• r1 cap: $0.01274292 – $0.01313098"), "{text}");
}

#[test]
fn test_the_cap_near_the_band_top_is_bound_by_the_near_rungs_usdc() {
    let env = with_shape(fixture_at_sqrt(sqrt_at_tick(273601.5)), &bottle());
    let b = sim(&env, &[("shape", "configured"), ("position_id", V4_ID)]);
    assert_eq!(st(&b, "/inputs/shape/total_usdc"), "766.30");
    let cap = &b["rungs"][0];
    assert_eq!(st(cap, "/cap/bound_by"), "near_usdc");
    assert!(st(cap, "/cap/note").contains("the budget's 86.7891 NOCK would pair with $225.08 USDC, more than the most that keeps it at or below the near rung's USDC, $99.95"), "{}", cap["cap"]["note"]);
    assert_eq!(st(cap, "/cap/nock"), "38.5414");
    assert!((f(cap, "/sizing/usdc_at_spot") - 99.952174).abs() < 0.006);
    let near = &b["rungs"][1];
    assert!((f(near, "/sizing/usdc_at_spot") - 99.952174).abs() < 0.006);
    let sum: f64 = (0..3).map(|i| f(&b["rungs"][i], "/sizing/usdc_at_spot")).sum();
    assert!((sum - 766.30).abs() < 0.011, "{sum}");
    assert_eq!(st(&b, "/combined/nock_held_at_deepest_bottom"), "70238.1762");
}

#[test]
fn test_the_cap_on_a_band_edge_says_which_band_and_why() {
    let env = with_shape(fixture_at_sqrt(sqrt_at_tick(273900.25)), &bottle());
    let b = sim(&env, &[("shape", "configured"), ("position_id", V4_ID)]);
    let cap = &b["rungs"][0];
    assert_eq!((cap["bounds"]["tick_lower"].as_i64(), cap["bounds"]["tick_upper"].as_i64()), (Some(273900), Some(274200)));
    assert!(st(cap, "/cap/band_rule").contains("exactly on a band edge: the band 273900..274200 is used, the one that contains it"));
    assert_eq!((st(cap, "/cap/bound_by"), st(cap, "/cap/nock")), ("near_usdc", "6.6037"));
    assert_eq!(st(&b, "/inputs/shape/total_usdc"), "767.76");
    assert_eq!(b["rungs"][1]["bounds"]["tick_lower"].as_i64(), Some(274200));
}

#[test]
fn test_no_cap_when_the_budget_is_zero_or_negative() {
    for budget in ["0.0000", "-12.5000"] {
        let mut env = with_shape(fixture(), &bottle());
        env.data.derived_metrics.global_status.lp_nock_budget = Some(budget.into());
        let b = sim(&env, &[("shape", "configured"), ("position_id", V4_ID)]);
        assert_eq!(st(&b, "/inputs/shape/cap"), "none");
        assert!(st(&b, "/inputs/shape/cap_reason").contains(&format!("(lp_nock_budget) is {budget}: there is no NOCK for a cap")));
        assert_eq!(b["rungs"].as_array().unwrap().len(), 2);
        assert_eq!((st(&b, "/rungs/0/rung"), st(&b, "/rungs/1/rung")), ("near", "deep"));
        assert!((f(&b, "/rungs/0/sizing/usdc_at_spot") - 115.0485).abs() < 0.006);
        assert!((f(&b, "/rungs/1/sizing/usdc_at_spot") - 651.9415).abs() < 0.006);
        assert_eq!(st(&b, "/combined/nock_held_at_deepest_bottom"), "71907.5864");
    }
}

#[test]
fn test_the_shape_without_a_cap_and_with_a_given_total() {
    let env = with_shape(fee_fixture(), &bottle()[1..]);
    let b = sim(&env, &[("shape", "configured"), ("position_id", V4_ID)]);
    assert_eq!((st(&b, "/inputs/shape/total_usdc"), st(&b, "/inputs/shape/cap")), ("766.99", "not in the shape"));
    assert_eq!(st(&b, "/combined/nock_held_at_deepest_bottom"), "71907.5864");
    assert_eq!(st(&b, "/combined/avg_buy_price_usd"), "0.01066633");
    assert_eq!(st(&b, "/combined/nock_kept_if_deep_rungs_pulled"), "62176.0245");
    let row = b["grid"].as_array().unwrap().iter().find(|r| r["pct_of_spot"] == "-10.00").unwrap().clone();
    assert_eq!(st(&row, "/ladder_nock"), "5732.0844");
    assert_eq!(st(&b, "/rungs/1/fee_estimate/per_day_in_range_usd"), "86.03");
    assert_eq!(st(&b, "/rungs/0/fee_estimate/band_measured/usd_per_day"), "3.53");
    // A total the caller gives.
    let b = sim(&env, &[("shape", "configured"), ("shape_usdc", "766.84"), ("position_id", V4_ID)]);
    assert!(st(&b, "/inputs/shape/headroom_rule").starts_with("given (shape_usdc)"));
    assert_eq!(st(&b, "/combined/nock_held_at_deepest_bottom"), "71893.5235");
    assert_eq!(st(&b, "/reserve_check/margin_usdc"), "0.15");
    // No reserve, no default total.
    let mut env2 = env.clone();
    env2.data.derived_metrics.global_status.cash_reserve_usdc = None;
    let e = sim_on(&env2, &[("shape", "configured"), ("position_id", V4_ID)]).unwrap_err();
    assert_eq!(e.code, Some("SHAPE_TOTAL_UNKNOWN"));
    // An invalid shape is named.
    let mut env3 = fixture();
    env3.data.derived_metrics.global_status.ladder_rungs_note = Some("operator_profile.ladder_rungs is a configuration error: the near and deep shares sum to 0.5, not 1; it is not applied".into());
    let e = sim_on(&env3, &[("shape", "configured"), ("position_id", V4_ID)]).unwrap_err();
    assert_eq!(e.code, Some("SHAPE_NOT_CONFIGURED"));
    assert!(e.detail.contains("sum to 0.5, not 1"));
}

#[test]
fn test_the_ladder_shape_block_in_the_snapshot() {
    let env = with_shape(fee_fixture(), &bottle());
    let at = now();
    let block = ladder_shape_block(&env.data, at).expect("a shape is configured");
    assert_eq!(st(&block, "/status"), "computed");
    assert_eq!(st(&block, "/source"), "operator_profile.ladder_rungs");
    assert_eq!(st(&block, "/computed_at"), "2026-11-20T12:05:00Z");
    assert_eq!((st(&block, "/pool/position_id"), block["pool"]["pool_tick"].as_i64()), (V4_ID, Some(273743)));
    assert_eq!(st(&block, "/total_usdc"), "766.99");
    let cap = &block["rungs"][0];
    assert_eq!((st(cap, "/rung"), cap["tick_lower"].as_i64(), cap["tick_upper"].as_i64()), ("cap", Some(273600), Some(273900)));
    assert_eq!((st(cap, "/low_usd"), st(cap, "/high_usd")), ("0.01274292", "0.01313098"));
    assert_eq!((st(cap, "/nock_per_usdc_at_low"), st(cap, "/nock_per_usdc_at_high")), ("78.4749", "76.1558"));
    assert_eq!(cap["in_range_now"], true);
    // The exact mint (review finding 4), recomputed independently with Uniswap's integer
    // formulas by /tmp/nn-v090/rungs_fixture/exact_mint.py: the cap's USDC (truncated to
    // 1225999 raw) binds, so it pulls a little less NOCK than the 86.7891 budget.
    assert_eq!(st(cap, "/liquidity_raw"), "138192109253769");
    assert_eq!((st(cap, "/amount0_raw"), st(cap, "/amount1_raw")), ("1225999", "867890958577132038"));
    assert_eq!((st(cap, "/usdc_raw"), st(cap, "/nock_raw")), ("1225999", "867890958577132038"));
    assert_eq!((st(cap, "/nock"), st(cap, "/usdc")), ("86.7890958577132038", "1.225999"));
    assert_eq!(st(cap, "/liquidity_human"), "1381.9212");
    assert_eq!((st(&block["rungs"][1], "/liquidity_raw"), st(&block["rungs"][1], "/usdc_raw")), ("1408301615002237", "114864600"));
    assert_eq!((st(&block["rungs"][2], "/liquidity_raw"), st(&block["rungs"][2], "/usdc_raw")), ("14125167068081104", "650899400"));
    assert_eq!(st(cap, "/status"), "to_deposit");
    assert_eq!(st(cap, "/cap/bound_by"), "lp_nock_budget");
    assert_eq!(st(cap, "/fee_estimate/status"), "measured");
    let near = &block["rungs"][1];
    assert_eq!((st(near, "/rung"), st(near, "/share")), ("near", "0.15"));
    assert_eq!(near["in_range_now"], false);
    assert_eq!(st(near, "/nock_bought_at_bottom"), "9716.0065");
    assert_eq!(st(&block["rungs"][2], "/avg_buy_price_usd"), "0.01048542");
    assert_eq!(st(&block, "/combined/nock_bought_at_deepest_bottom"), "71888.1054");
    assert_eq!((st(&block, "/gate/price_usd"), st(&block, "/gate/status"), st(&block, "/gate/max_usdc_total")), ("0.01000000", "holds", "766.99"));
    assert_eq!((st(&block, "/nock_budget/nock_needed"), st(&block, "/nock_budget/status")), ("86.7891", "fits"));
    assert_eq!(st(&block, "/floor_breach_effect/status"), "lowers");
    // No shape configured: absent. An invalid one: stated.
    assert_eq!(ladder_shape_block(&fee_fixture().data, at), None);
    let mut bad = fixture();
    bad.data.derived_metrics.global_status.ladder_rungs_note = Some("operator_profile.ladder_rungs is a configuration error: it lists no rung; it is not applied".into());
    let block = ladder_shape_block(&bad.data, at).unwrap();
    assert_eq!((st(&block, "/status"), st(&block, "/reason")), ("unavailable", "operator_profile.ladder_rungs is a configuration error: it lists no rung; it is not applied"));
    // It is not part of the state hash.
    let mut data = env.data.clone();
    let hash = compute_state_hash(&data);
    data.derived_metrics.ladder_shape = ladder_shape_block(&env.data, at);
    assert_eq!(compute_state_hash(&data), hash);
}

#[test]
fn test_the_cap_reads_as_a_tracked_rung() {
    let text = crate::ladder_track_text::ladder_statement(&json!({"label": "v4 USDC/NOCK 3.00% #1", "rung": "cap", "low_price_usd": "0.01274292", "high_price_usd": "0.01313098"}));
    assert!(text.contains("(rung cap)"), "{text}");
}

// ------------------------------------------------------------------
// MCP: both transports
// ------------------------------------------------------------------

#[test]
fn test_the_rungs_argument_normalises_to_the_indexed_query() {
    let args = json!({
        "position_id": V4_ID,
        "rungs": [
            {"rung": "near", "usdc": 100, "high_at_spot": true, "low": 0.012},
            {"rung": "deep", "usdc": "500", "high": 0.011, "low": 0.01},
        ],
    });
    let q = ladder_text::ladder_query_from_args(Some(&args)).unwrap();
    let mut expected: Vec<(String, String)> = TWO[1..].iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
    expected.push(("position_id".into(), V4_ID.into()));
    let mut got = q.clone();
    got.sort();
    expected.sort();
    assert_eq!(got, expected);
    // The HTTP transport prints the text of the same body.
    let env = fee_fixture();
    let (text, is_error) = simulate_ladder_mcp(Some(&args), &env, now(), "ok");
    assert!(!is_error, "{text}");
    assert_eq!(text, ladder_text::ladder_text(&sim(&env, &TWO)));
    // The shape.
    let env = with_shape(fee_fixture(), &bottle());
    let (text, is_error) = simulate_ladder_mcp(Some(&json!({"shape": "configured", "position_id": V4_ID})), &env, now(), "ok");
    assert!(!is_error, "{text}");
    assert!(text.contains("• r1 cap: "), "{text}");
    // Refusals.
    for (a, needle) in [
        (json!({"usdc": 1, "rungs": [{"rung": "near", "usdc": 1}]}), "is the one-rung form"),
        (json!({"rungs": []}), "rungs is empty"),
        (json!({"rungs": [{}, {}, {}, {}, {}]}), "at most 4 rungs"),
        (json!({"rungs": [5]}), "rungs[0] must be an object"),
        (json!({"rungs": [{"rung": "near", "bottom": 1}]}), "unknown argument 'bottom' in rungs[0]"),
        (json!({"rungs": [{"rung": "near", "high": 1, "high_at_spot": true}]}), "rungs[0]: give high or high_at_spot, not both"),
        (json!({"rungs": {"rung": "near"}}), "rungs must be an array"),
    ] {
        let e = ladder_text::ladder_query_from_args(Some(&a)).unwrap_err();
        assert!(e["detail"].as_str().unwrap().contains(needle), "{needle}: {e}");
    }
    let tool = ladder_text::simulate_ladder_tool();
    assert_eq!(tool["inputSchema"]["properties"]["rungs"]["type"], "array");
    assert_eq!(tool["inputSchema"]["properties"]["rungs"]["items"]["properties"]["rung"]["enum"], json!(["cap", "near", "deep"]));
    assert!(tool["description"].as_str().unwrap().contains("estimated fees per day while in range, measured from your own tracked range"));
}

// Regenerate only for an intended change:
//   NOCKNOCK_BLESS_GOLDEN=1 cargo test test_golden_ladder
// The stdio MCP (mcp_bin_tests_ladder.rs) and the dashboard (frontend/src/ladderRungs.test.ts)
// read these bodies and texts.
#[test]
fn test_golden_ladder_rungs_is_byte_identical() {
    let fee = fee_fixture();
    let shaped = with_shape(fee_fixture(), &bottle());
    let plan = block_of(&with_shape(fixture(), &bottle()));
    let ls: Vec<u128> = ["cap", "near", "deep"].iter().map(|r| liq(shape_rung(&plan, r))).collect();
    let deep_done = with_shape(fixture_with_rungs(&[(9000001, Some("deep"), 275400, 276300, ls[2])]), &bottle());
    let all_done = with_shape(
        fixture_with_rungs(&[(9000002, Some("cap"), 273600, 273900, ls[0]), (9000003, Some("near"), 273900, 275400, ls[1]), (9000001, Some("deep"), 275400, 276300, ls[2])]),
        &bottle(),
    );
    let text = ladder_text::ladder_text(&sim(&deep_done, &[("shape", "configured"), ("position_id", V4_ID)]));
    assert!(text.contains("  – r3 deep: deployed; target $650.90, deployed $650.90, left to deposit $0.00, tracked: uniswap-v4:base:9000001"), "{text}");
    let text = ladder_text::ladder_text(&sim(&all_done, &[("shape", "configured"), ("position_id", V4_ID)]));
    assert!(text.contains("nothing is left to deposit"), "{text}");
    let ok = |env: &SnapshotEnvelope, args: Value| {
        let q: HashMap<String, String> = ladder_text::ladder_query_from_args(Some(&args)).unwrap().into_iter().collect();
        let body = simulate_ladder(env, &LadderRequest::from_params(&q).unwrap(), now(), "ok").unwrap();
        let (text, is_error) = simulate_ladder_mcp(Some(&args), env, now(), "ok");
        assert!(!is_error, "{text}");
        assert_eq!(text, ladder_text::ladder_text(&body));
        json!({ "arguments": args, "status": 200, "body": body, "mcp_text": text })
    };
    let bad = |env: &SnapshotEnvelope, args: Value| {
        let q: HashMap<String, String> = ladder_text::ladder_query_from_args(Some(&args)).unwrap().into_iter().collect();
        let e = LadderRequest::from_params(&q).and_then(|r| simulate_ladder(env, &r, now(), "ok")).unwrap_err();
        let (text, is_error) = simulate_ladder_mcp(Some(&args), env, now(), "ok");
        assert!(is_error);
        json!({ "arguments": args, "status": e.status, "body": e.body(), "mcp_text": text })
    };
    let two = json!({"position_id": V4_ID, "rungs": [
        {"rung": "near", "usdc": 100, "high_at_spot": true, "low": 0.012},
        {"rung": "deep", "usdc": 500, "high": 0.011, "low": 0.01},
    ]});
    let doc = json!({
        "about": "GET /v1/simulate/ladder bodies and the simulate_ladder MCP text for ladders of rungs and the configured shape, over the #166 fixture with the recorded fee measurement (tests_ladder_rungs.rs). Depths, amounts and splits are fixture inputs only.",
        "flat_741_measured": ok(&fee, json!({"usdc": 741, "low": 0.01, "high_at_spot": true, "position_id": V4_ID})),
        "two_rungs_measured": ok(&fee, two),
        "the_bottle_configured": ok(&shaped, json!({"shape": "configured", "position_id": V4_ID})),
        "the_bottle_deep_deployed": ok(&deep_done, json!({"shape": "configured", "position_id": V4_ID})),
        "the_bottle_all_deployed": ok(&all_done, json!({"shape": "configured", "position_id": V4_ID})),
        "rung_too_narrow": bad(&fee, json!({"position_id": V4_ID, "rungs": [{"rung": "near", "usdc": 100, "low": 0.0127, "high": 0.0128}]})),
        "rungs_exceed_wallet": bad(&fee, json!({"rungs": [{"rung": "near", "usdc": 1000, "low": 0.01, "high_at_spot": true}, {"rung": "deep", "usdc": 1000, "low": 0.009, "high": 0.01}]})),
        "missing_role": bad(&fee, json!({"rungs": [{"usdc": 100, "low": 0.01, "high_at_spot": true}]})),
    });
    let actual = format!("{}\n", serde_json::to_string_pretty(&doc).unwrap());
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/ladder/golden_ladder_rungs.json");
    if std::env::var("NOCKNOCK_BLESS_GOLDEN").is_ok() {
        std::fs::write(path, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(path).expect("testdata/ladder/golden_ladder_rungs.json is checked in");
    assert!(actual == expected, "ladder rung bytes changed; compare {path} with a fresh NOCKNOCK_BLESS_GOLDEN=1 run and `git diff` it");
}

#[test]
fn test_the_specs_document_rungs_shape_and_fees() {
    let api: serde_yaml::Value = serde_yaml::from_str(include_str!("../../specs/core/api.yaml")).unwrap();
    let get = &api["paths"]["/simulate/ladder"]["get"];
    let names: Vec<&str> = get["parameters"].as_sequence().unwrap().iter().filter_map(|p| p["name"].as_str()).collect();
    for n in ["r1_rung", "r1_usdc", "r1_nock", "r1_low", "r1_low_pct", "r1_high", "r1_high_pct", "split_step_pct", "shape", "shape_usdc"] {
        assert!(names.contains(&n), "parameter {n}");
    }
    let schemas = &api["components"]["schemas"];
    let b = sim(&with_shape(fee_fixture(), &bottle()), &[("shape", "configured"), ("position_id", V4_ID)]);
    for key in b.as_object().unwrap().keys() {
        assert!(schemas["LadderSimulation"]["properties"][key.as_str()].is_mapping(), "LadderSimulation.{key}");
    }
    for key in b["rungs"][0].as_object().unwrap().keys() {
        assert!(schemas["LadderRung"]["properties"][key.as_str()].is_mapping(), "LadderRung.{key}");
    }
    for key in b["combined"].as_object().unwrap().keys() {
        assert!(schemas["LadderCombined"]["properties"][key.as_str()].is_mapping(), "LadderCombined.{key}");
    }
    for key in b["rungs"][0]["fee_estimate"].as_object().unwrap().keys() {
        assert!(schemas["LadderFeeEstimate"]["properties"][key.as_str()].is_mapping(), "LadderFeeEstimate.{key}");
    }
    for key in b["grid"][0].as_object().unwrap().keys() {
        assert!(schemas["LadderGridRow"]["properties"][key.as_str()].is_mapping(), "LadderGridRow.{key}");
    }
    let two = sim(&fee_fixture(), &TWO);
    for key in two["tradeoff"].as_object().unwrap().keys() {
        assert!(schemas["LadderTradeoff"]["properties"][key.as_str()].is_mapping(), "LadderTradeoff.{key}");
    }
    let text = include_str!("../../specs/core/api.yaml");
    for code in ["RUNG_TOO_NARROW", "RUNGS_EXCEED_WALLET", "SHAPE_NOT_CONFIGURED", "SHAPE_TOTAL_UNKNOWN", "SHAPE_NO_ROOM", "SHAPE_NEEDS_TICK_SPACING"] {
        assert!(text.contains(code), "{code}");
    }
    let snap: serde_yaml::Value = serde_yaml::from_str(include_str!("../../specs/core/snapshot.yaml")).unwrap();
    let shape = ladder_shape_block(&with_shape(fee_fixture(), &bottle()).data, now()).unwrap();
    let ss = &snap["components"]["schemas"];
    for key in shape.as_object().unwrap().keys() {
        assert!(ss["LadderShape"]["properties"][key.as_str()].is_mapping(), "LadderShape.{key}");
    }
    for key in shape["rungs"][0].as_object().unwrap().keys() {
        assert!(ss["LadderShapeRung"]["properties"][key.as_str()].is_mapping(), "LadderShapeRung.{key}");
    }
}

// ------------------------------------------------------------------
// Review of #166 rungs: the plan-stable shape, passed rungs, exact mints, honest grids
// ------------------------------------------------------------------

fn block_of(env: &SnapshotEnvelope) -> Value {
    ladder_shape_block(&env.data, now()).expect("a shape is configured")
}

fn shape_rung<'a>(b: &'a Value, role: &str) -> &'a Value {
    b["rungs"].as_array().unwrap().iter().find(|r| r["rung"] == role).unwrap_or_else(|| panic!("no {role} rung in {b}"))
}

fn liq(r: &Value) -> u128 {
    st(r, "/liquidity_raw").parse().unwrap()
}

#[test]
fn test_depositing_the_deep_rung_as_stated_leaves_near_and_cap_unchanged() {
    let before = block_of(&with_shape(fixture(), &bottle()));
    let deep = shape_rung(&before, "deep");
    let env = with_shape(fixture_with_rungs(&[(9000001, Some("deep"), 275400, 276300, liq(deep))]), &bottle());
    let after = block_of(&env);
    assert_eq!(st(&after, "/status"), "computed");
    assert_eq!(st(&after, "/total_usdc"), st(&before, "/total_usdc"));
    let d = shape_rung(&after, "deep");
    assert_eq!(st(d, "/status"), "deployed");
    assert_eq!(d["counterparts"], json!(["uniswap-v4:base:9000001"]));
    assert!(f(d, "/remaining_usdc") < LADDER_SHAPE_MIN_USDC);
    assert!(d["liquidity_raw"].is_null(), "nothing is minted for a deployed rung");
    for role in ["near", "cap"] {
        let (a, b) = (shape_rung(&after, role), shape_rung(&before, role));
        assert_eq!(st(a, "/status"), "to_deposit", "{role}");
        assert_eq!(fx(f(a, "/target_usdc"), 2), fx(f(b, "/target_usdc"), 2), "{role} target to the cent");
        assert_eq!(fx(f(a, "/remaining_usdc"), 2), fx(f(b, "/remaining_usdc"), 2), "{role} remaining to the cent");
        assert_eq!(st(a, "/usdc_raw"), st(b, "/usdc_raw"), "{role} mint");
    }
    // No second deep rung is offered, and the gate still holds to the cent.
    assert_eq!(after["rungs"].as_array().unwrap().iter().filter(|r| r["rung"] == "deep").count(), 1);
    assert_eq!(st(&after, "/gate/status"), "holds");
    assert!(f(&after, "/gate/margin_usdc").abs() < 0.02, "{}", after["gate"]);
}

#[test]
fn test_depositing_cap_and_near_first_leaves_deep_unchanged() {
    let before = block_of(&with_shape(fixture(), &bottle()));
    let (cap, near) = (shape_rung(&before, "cap"), shape_rung(&before, "near"));
    let env = with_shape(fixture_with_rungs(&[(9000002, Some("cap"), 273600, 273900, liq(cap)), (9000003, Some("near"), 273900, 275400, liq(near))]), &bottle());
    let after = block_of(&env);
    for role in ["cap", "near"] {
        assert_eq!(st(shape_rung(&after, role), "/status"), "deployed", "{role}: {}", shape_rung(&after, role));
    }
    // A tracked cap is the cap: its liquidity is its target, and it is not sized again.
    let cap = shape_rung(&after, "cap");
    assert!(cap["cap"].is_null());
    assert_eq!(cap["tracked_liquidity"], cap["target_liquidity"]);
    let (a, b) = (shape_rung(&after, "deep"), shape_rung(&before, "deep"));
    assert_eq!(st(a, "/status"), "to_deposit");
    assert_eq!(fx(f(a, "/target_usdc"), 2), fx(f(b, "/target_usdc"), 2));
    assert_eq!(st(a, "/usdc_raw"), st(b, "/usdc_raw"));
}

#[test]
fn test_with_every_rung_deposited_nothing_is_left_to_deposit() {
    let before = block_of(&with_shape(fixture(), &bottle()));
    let ls: Vec<u128> = ["cap", "near", "deep"].iter().map(|r| liq(shape_rung(&before, r))).collect();
    let env = with_shape(
        fixture_with_rungs(&[(9000002, Some("cap"), 273600, 273900, ls[0]), (9000003, Some("near"), 273900, 275400, ls[1]), (9000001, Some("deep"), 275400, 276300, ls[2])]),
        &bottle(),
    );
    let after = block_of(&env);
    assert_eq!(st(&after, "/status"), "deployed");
    assert!(st(&after, "/reason").contains("nothing is left to deposit"));
    for r in after["rungs"].as_array().unwrap() {
        assert_eq!(r["status"], "deployed", "{r}");
        assert!(r["usdc_raw"].is_null());
    }
    // The API answers 200 with the status, never a parameter error.
    let b = sim(&env, &[("shape", "configured"), ("position_id", V4_ID)]);
    assert_eq!(st(&b, "/shape_status"), "deployed");
}

#[test]
fn test_a_tracked_rung_at_other_ticks_keeps_its_ticks_for_the_remainder() {
    // The near rung as stated, and a deep rung deposited on 275700..276300 with a third of
    // the deep target's liquidity there: the near rung froze the plan, so the deep rung has
    // a remainder.
    let env0 = with_shape(fixture(), &bottle());
    let (near0, deep0) = (shape_rung(&block_of(&env0), "near").clone(), shape_rung(&block_of(&env0), "deep").clone());
    let env = with_shape(fixture_with_rungs(&[(9000003, Some("near"), 273900, 275400, liq(&near0)), (9000004, Some("deep"), 275700, 276300, liq(&deep0) / 3)]), &bottle());
    let after = block_of(&env);
    let d = shape_rung(&after, "deep");
    assert_eq!(d["counterparts"], json!(["uniswap-v4:base:9000004"]));
    assert_eq!(st(d, "/status"), "to_deposit");
    // The remainder is drawn on the tracked rung's own ticks.
    assert_eq!((d["tick_lower"].as_i64(), d["tick_upper"].as_i64()), (Some(275700), Some(276300)));
    let remaining = f(d, "/remaining_usdc");
    assert!((f(d, "/target_usdc") - f(d, "/deployed_usdc") - remaining).abs() < 1e-5);
    assert!(f(d, "/usdc") <= remaining + 1e-9);
}

#[test]
fn test_an_unmatched_tracked_ladder_is_listed_and_the_gate_counts_it_like_any_range() {
    let before = block_of(&with_shape(fixture(), &bottle()));
    let env = with_shape(fixture_with_rungs(&[(9000005, None, 275400, 276300, 1_000_000_000_000_000)]), &bottle());
    let after = block_of(&env);
    let u = &after["unmatched"][0];
    assert_eq!(st(u, "/position_id"), "uniswap-v4:base:9000005");
    assert!(st(u, "/reason").contains("its rung is not set"));
    assert_eq!(st(u, "/note"), "it fills no configured rung; the reserve gate counts what it holds like any other range");
    // Its USDC left the wallet and fills no rung: the reserve gate allows that much less
    // for the shape, and what is left of it still fits (review of the bottle, finding 8).
    assert!(f(&after, "/headroom_usdc") < f(&before, "/headroom_usdc") - 40.0);
    assert_eq!(st(shape_rung(&after, "deep"), "/status"), "to_deposit");
    assert_eq!(st(&after, "/gate/status"), "holds");
    assert!(st(&after, "/tracked_note").contains("matched to configured rungs by role, then by ticks"));
}

#[test]
fn test_rungs_above_the_price_are_passed_and_the_shape_is_still_drawn() {
    for (tick, near, deep, cap) in [
        (275100.5, "passed", "to_deposit", "to_deposit"),
        // One tick above the band's bottom the budget's NOCK is worth $0.95: under the
        // $1.00 minimum, so the cap is not offered (review of the bottle, finding 16).
        (275399.0, "passed", "to_deposit", "below_minimum"),
        // Exactly on a band edge: the band at the pool's tick holds one token at this
        // price, so there is no cap, with the reason.
        (275700.0, "passed", "to_deposit", "none"),
        (276300.0, "passed", "passed", "passed"),
    ] {
        let env = with_shape(fixture_at_sqrt(sqrt_at_tick(tick)), &bottle());
        // shape=configured answers 200 with the block's status, never INVALID_PARAMETER.
        let b = sim_on(&env, &[("shape", "configured"), ("position_id", V4_ID)]).unwrap_or_else(|e| panic!("{tick}: {e:?}"));
        let block = block_of(&env);
        assert_eq!(st(shape_rung(&block, "near"), "/status"), near, "{tick}");
        // Each passed rung says why in its own terms (review of the bottle, finding 9).
        assert!(st(shape_rung(&block, "near"), "/reason").contains("this rung's bottom $0.01096801"), "{tick}: {}", shape_rung(&block, "near"));
        assert_eq!(st(shape_rung(&block, "deep"), "/status"), deep, "{tick}");
        assert_eq!(st(shape_rung(&block, "cap"), "/status"), cap, "{tick}");
        if cap == "none" {
            assert!(st(shape_rung(&block, "cap"), "/reason").contains("holds only one token"), "{tick}");
        }
        if deep == "to_deposit" {
            assert_eq!(st(&block, "/status"), "computed");
            // The first drawable rung starts at the usable tick at or below spot.
            let spot_tick = ((tick / 300.0).ceil() * 300.0) as i64;
            assert_eq!(shape_rung(&block, "deep")["tick_lower"].as_i64(), Some(spot_tick), "{tick}");
            assert_eq!(shape_rung(&block, "deep")["tick_upper"].as_i64(), Some(276300));
            assert_eq!(st(&b, "/shape_status"), "computed");
        } else {
            assert_eq!(st(&block, "/status"), "price_at_or_below_bottom");
            assert_eq!(st(&b, "/shape_status"), "price_at_or_below_bottom");
        }
    }
}

#[test]
fn test_two_rungs_of_one_role_state_their_own_shares() {
    let shape = vec![
        LadderRungConfig { rung: "cap".into(), share: None, low_usd: None },
        LadderRungConfig { rung: "near".into(), share: Some(0.15), low_usd: Some(0.0115) },
        LadderRungConfig { rung: "deep".into(), share: Some(0.35), low_usd: Some(0.0108) },
        LadderRungConfig { rung: "deep".into(), share: Some(0.50), low_usd: Some(0.01) },
    ];
    let block = block_of(&with_shape(fixture(), &shape));
    let shares: Vec<&str> = block["rungs"].as_array().unwrap().iter().map(|r| r["share"].as_str().unwrap_or("-")).collect();
    assert_eq!(shares, vec!["-", "0.15", "0.35", "0.5"]);
    let rest = f(&block, "/rest_usdc");
    assert!((f(&block["rungs"][3], "/target_usdc") - 0.5 * rest).abs() < 0.01);
    assert!((f(&block["rungs"][2], "/target_usdc") - 0.35 * rest).abs() < 0.01);
    // The cap's limit is the first near rung's target, stated.
    assert_eq!(
        st(&block["rungs"][0], "/cap/limit_rule"),
        "never more USDC than the first near rung's target; once a near or deep rung is deposited, never more than the reserve leaves for it beside the other deposits listed"
    );
}

#[test]
fn test_grid_fees_are_null_with_a_reason_where_nothing_is_measured() {
    // A rung partly below the measuring range ($0.00550148): rows below it have no figure.
    let pairs = [("position_id", V4_ID), ("r1_rung", "deep"), ("r1_usdc", "50"), ("r1_high", "0.007"), ("r1_low", "0.004"), ("grid_to_pct", "-70"), ("grid_step_pct", "5")];
    let b = sim(&fee_fixture(), &pairs);
    let row = |pct: &str| b["grid"].as_array().unwrap().iter().find(|r| r["pct_of_spot"] == pct).unwrap().clone();
    let r = row("-60.00");
    assert_eq!(r["rungs_in_range"], json!([1]));
    assert!(r["est_fee_usd_per_day"].is_null() && r["est_fee_usd_per_day_diluted"].is_null());
    assert!(st(&r, "/est_fee_reason").contains("outside the measuring range: no fees are measured here"));
    assert!(row("-50.00")["est_fee_usd_per_day"].is_string());
    // A rung wholly outside: null with its reason, never $0.00, and the text says why.
    let pairs = [("position_id", V4_ID), ("r1_rung", "deep"), ("r1_usdc", "50"), ("r1_high", "0.0054"), ("r1_low", "0.0045"), ("grid_to_pct", "-65"), ("grid_step_pct", "5")];
    let b = sim(&fee_fixture(), &pairs);
    let r = b["grid"].as_array().unwrap().iter().find(|r| r["rungs_in_range"] == json!([1])).unwrap().clone();
    assert!(r["est_fee_usd_per_day"].is_null());
    assert!(st(&r, "/est_fee_reason").contains("r1 lies wholly outside the measuring range"));
    let text = ladder_text::ladder_text(&b);
    assert!(text.contains("r1 in range, fees not estimated (r1 lies wholly outside the measuring range"), "{text}");
    assert!(!text.contains("fees $0.00/day"), "{text}");
}

#[test]
fn test_every_split_row_is_gated_at_the_same_price() {
    let b = sim(&fee_fixture(), &TWO);
    let rows = b["tradeoff"]["split"]["rows"].as_array().unwrap();
    for r in rows {
        assert_eq!(st(r, "/gate_price_usd"), "0.01002405", "{r}");
    }
    // The near-only (100%) row no longer reads a false margin at the near rung's bottom.
    let last = rows.last().unwrap();
    assert_eq!(st(last, "/reserve_margin_usdc"), st(&rows[0], "/reserve_margin_usdc"));
    // Where a fee per day in range is stated, its in-range share and replay are too.
    for r in rows {
        for x in r["per_rung"].as_array().unwrap() {
            if !x["fee_per_day_in_range_usd"].is_null() {
                for k in ["in_range_share_pct", "replay_inside_upper_usd", "replay_edge_upper_usd", "coverage_whole"] {
                    assert!(!x[k].is_null(), "{k} null beside a fee figure: {x}");
                }
            }
        }
    }
}

#[test]
fn test_the_one_rung_text_names_the_gate_price_rule_and_an_invalid_bottom() {
    let pairs = [("usdc", "300"), ("low", "0.012"), ("high", "spot"), ("position_id", V4_ID)];
    let text = ladder_text::ladder_text(&sim(&with_bottom(fixture(), "0.01000000"), &pairs));
    assert!(
        text.contains("• Reserve: stable $1416.99 at the gate $0.01000000 (your ladder bottom (operator_profile.ladder_bottom_usd $0.01000000), below the deepest rung's bottom) against the $950.00 reserve: holds by $466.99"),
        "{text}"
    );
    let mut env = fixture();
    env.data.derived_metrics.global_status.ladder_bottom_note = Some(LADDER_BOTTOM_INVALID_NOTE.into());
    let b = sim(&env, &pairs);
    let text = ladder_text::ladder_text(&b);
    assert!(text.contains("at the gate $0.01200086 (the deepest rung's bottom: operator_profile.ladder_bottom_usd must be above 0 USD per NOCK; it is not applied)"), "{text}");
    assert!(text.contains("is set but invalid and not applied: the gate falls back to the stable balance at the ladder's own bottom"), "{text}");
    assert!(!text.contains("the lower of the ladder's deepest bottom and your ladder bottom"), "{text}");
    // With the setting unset the text is the #166 one.
    let text = ladder_text::ladder_text(&sim(&fixture(), &pairs));
    assert!(text.contains("• Reserve: stable $1453.43 at the bottom against"), "{text}");
}

#[test]
fn test_the_rung_mint_is_exact_and_never_exceeds_the_amounts_stated() {
    use crate::mint_math::*;
    let b = sim(&with_shape(fixture(), &bottle()), &[("shape", "configured"), ("position_id", V4_ID)]);
    let sqrt_p = Big::from_hex("0d69693142532bf9792f0b505c97d4").unwrap();
    for r in b["rungs"].as_array().unwrap() {
        let m = &r["mint"];
        assert_eq!(st(m, "/status"), "exact");
        let (tl, tu) = (r["bounds"]["tick_lower"].as_i64().unwrap() as i32, r["bounds"]["tick_upper"].as_i64().unwrap() as i32);
        let l = Big::from_hex(&format!("{:x}", st(m, "/liquidity_raw").parse::<u128>().unwrap())).unwrap();
        // What a mint of liquidity_raw pulls, from Uniswap's integer formulas.
        let (a0, a1) = amounts_for_liquidity(273743, &sqrt_p, tl, tu, &l).unwrap();
        assert_eq!((a0.to_dec(), a1.to_dec()), (st(m, "/amount0_raw").to_string(), st(m, "/amount1_raw").to_string()));
        let max0: u128 = st(m, "/usdc_max_raw").parse().unwrap();
        let max1: u128 = st(m, "/nock_max_raw").parse().unwrap();
        assert!(a0.to_u128().unwrap() <= max0 && a1.to_u128().unwrap() <= max1);
        // One unit more of liquidity would need more than the maxima.
        let (b0, b1) = amounts_for_liquidity(273743, &sqrt_p, tl, tu, &l.add(&Big::from_u128(1))).unwrap();
        assert!(b0.to_u128().unwrap() > max0 || b1.to_u128().unwrap() > max1, "{}", r["index"]);
        // Never more than the USDC the rung states.
        assert!(a0.to_u128().unwrap() as f64 / 1e6 <= f(r, "/sizing/usdc_at_spot") + 0.005);
    }
}

#[test]
fn test_a_stored_row_keeps_only_the_summary_and_a_read_back_row_says_so() {
    let mut env = with_shape(fee_fixture(), &bottle());
    env.data.derived_metrics.ladder_shape = ladder_shape_block(&env.data, now());
    let full = serde_json::to_value(&env.data).unwrap();
    let size = |v: &Value, p: &str| serde_json::to_string(v.pointer(p).unwrap()).unwrap().len();
    let (fy_full, shape_full) = (size(&full, "/derived_metrics/fee_yield"), size(&full, "/derived_metrics/ladder_shape"));
    let mut stored = full.clone();
    slim_stored_data(&mut stored);
    let (fy_stored, shape_stored) = (size(&stored, "/derived_metrics/fee_yield"), size(&stored, "/derived_metrics/ladder_shape"));
    // About 30 KB per row before (review finding 8); a few KB now.
    assert!(fy_full + shape_full > 25_000, "{fy_full} + {shape_full}");
    assert!(fy_stored < 2_500 && shape_stored < 8_000, "{fy_stored} + {shape_stored}");
    assert!(stored["derived_metrics"]["fee_yield"]["pools"][0]["detail_note"].as_str().unwrap().contains("left out of the stored row"));
    assert!(stored["derived_metrics"]["ladder_shape"]["grid"].is_null());
    assert_eq!(stored["derived_metrics"]["ladder_shape"]["rungs"][2]["fee_estimate"]["per_day_in_range_usd"], full["derived_metrics"]["ladder_shape"]["rungs"][2]["fee_estimate"]["per_day_in_range_usd"]);
    // A row read back from the database: the rate still applies; the detail it lacks is
    // stated, never read as a measured zero.
    let back: SnapshotData = serde_json::from_value(stored).unwrap();
    let env2 = SnapshotEnvelope { meta: env.meta.clone(), data: back };
    let b = sim(&env2, &FLAT_741);
    let fe = &b["fee_estimate"];
    assert_eq!(st(fe, "/per_day_in_range_usd"), "35.30");
    for (k, why) in [("in_range_share_stored_history", "in_range_share_reason"), ("stored_history_replay", "replay_reason"), ("band_measured", "band_reason"), ("spread", "spread_reason")] {
        assert!(fe[k].is_null(), "{k}");
        assert!(st(fe, &format!("/{why}")).contains("read back from the database"), "{why}");
    }
}
