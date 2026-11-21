//! Fee yield (snapshot 2.10.0): fees per unit of liquidity measured on our own tracked
//! range from stored snapshot rows. The recorded fixture is every stored row of the v4
//! USDC/NOCK range #3163450 on chudmax from 2026-11-14 to 2026-11-21
//! (testdata/fee_yield/history_v4_3163450.json). Every expected number below was
//! recomputed independently by /tmp/nn-v090/rungs_fixture/recompute.py (Python, its own
//! skip rules written from the spec text, true UTC days, the NOCK leg valued at the
//! current pool price), which agrees with the critique's re-derivation ($6.08 per 1e15 L
//! per day, not the design's accrual-priced $6.32).
use super::*;
use crate::tests_ladder::fixture;
use serde_json::Value;

pub(crate) const V4_ID: &str = "uniswap-v4:base:3163450";

fn t(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

/// The recorded rows as the cycle's query returns them.
pub(crate) fn history_rows() -> Vec<FeeYieldRow> {
    let doc: Value = serde_json::from_str(include_str!("../testdata/fee_yield/history_v4_3163450.json")).unwrap();
    doc["rows"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| FeeYieldRow {
            at: t(r[0].as_str().unwrap()),
            position_id: V4_ID.to_string(),
            liquidity: r[1].as_str().unwrap().parse().ok(),
            owed0: r[2].as_str().unwrap().parse().ok(),
            owed1: r[3].as_str().unwrap().parse().ok(),
            in_range: r[4].as_bool(),
            pool_active: r[5].as_bool(),
            tick: r[6].as_i64().map(|v| v as i32),
            staked: false,
        })
        .collect()
}

/// When the fixture's measurement was made: 10 minutes before its snapshot.
pub(crate) fn computed_at() -> DateTime<Utc> {
    t("2026-11-20T11:50:00Z")
}

/// The #166 ladder fixture with the recorded measurement attached, priced at its own
/// pool price ($0.012944), as a 2.10.0 cycle would store it.
pub(crate) fn fee_fixture() -> SnapshotEnvelope {
    let mut env = fixture();
    let mut fy = fee_yield_from_rows(&env.data.positions, &history_rows(), computed_at());
    price_fee_yield(&mut fy, &env.data.positions, &env.data.pool_mechanics);
    env.data.derived_metrics.fee_yield = Some(fy);
    env
}

/// The recorded pool at the history's own last price (tick 273743, $0.01294450).
fn measured_at_history_price() -> FeeYield {
    let env = fixture();
    let doc: Value = serde_json::from_str(include_str!("../testdata/fee_yield/history_v4_3163450.json")).unwrap();
    let mut pools = env.data.pool_mechanics.clone();
    pools[0].sqrt_price_x96 = Some(doc["last_sqrt_price_x96"].as_str().unwrap().to_string());
    let mut fy = fee_yield_from_rows(&env.data.positions, &history_rows(), computed_at());
    price_fee_yield(&mut fy, &env.data.positions, &pools);
    fy
}

fn rel(a: f64, b: f64) -> f64 {
    ((a - b) / b).abs()
}

fn f(s: &Option<String>) -> f64 {
    s.as_deref().expect("present").parse().unwrap()
}

#[test]
fn test_fee_yield_recorded_history_pins_the_measured_rate() {
    let fy = measured_at_history_price();
    assert_eq!(fy.status, "measured");
    assert_eq!(fy.window_days, 30);
    assert_eq!((fy.max_gap_seconds, fy.day_min_seconds, fy.refresh_seconds), (7200, 72000, 3600));
    assert_eq!(fy.pools.len(), 1);
    let p = &fy.pools[0];
    assert_eq!(p.measured_by, V4_ID);
    assert_eq!(p.label, "v4 USDC/NOCK 3.00% #3163450");
    assert_eq!((p.covered_tick_lower, p.covered_tick_upper, p.tick_spacing), (262200, 282300, Some(300)));
    assert_eq!(p.rows, 1985);
    assert_eq!(p.intervals, 1836);
    assert_eq!(p.zero_fee_gap_intervals, 0);
    // The skipped intervals, each in its own bucket, with their seconds.
    assert_eq!(
        p.skipped,
        FeeYieldSkips { liquidity_changed: 4, zero_liquidity: 137, fees_fell: 6, pool_inactive: 0, out_of_range: 0, gap: 1, unreadable: 0, staked: 0 }
    );
    assert!((p.skipped_seconds.zero_liquidity - 38413.578).abs() < 0.01);
    assert!((p.skipped_seconds.liquidity_changed - 1202.240).abs() < 0.01);
    assert!((p.skipped_seconds.fees_fell - 1776.479).abs() < 0.01);
    assert!((p.skipped_seconds.gap - 14179.252).abs() < 0.01);
    assert!((p.measured_seconds - 547205.136).abs() < 0.01);
    // Per raw L per day: 3.213286e-15 USDC and 2.215582e-13 NOCK.
    assert!(rel(f(&p.usdc_per_liquidity_day), 3.213286e-15) < 1e-6);
    assert!(rel(f(&p.nock_per_liquidity_day), 2.215582e-13) < 1e-6);
    // Valued at the current pool price, $6.0812 per 1e15 L per day (the USDC leg 52.8%).
    assert_eq!(p.pool_price_usd.as_deref(), Some("0.01294450"));
    assert_eq!(p.usd_per_1e15_liquidity_day.as_deref(), Some("6.0812"));
    let usd = f(&p.usd_per_liquidity_day);
    assert!((f(&p.usdc_per_liquidity_day) / usd - 0.5284).abs() < 1e-4);
    assert_eq!((p.from.as_deref(), p.to.as_deref()), (Some("2026-11-14T04:38:29Z"), Some("2026-11-21T04:04:46Z")));
}

#[test]
fn test_fee_yield_by_day_is_utc_and_the_median_is_true() {
    let fy = measured_at_history_price();
    let p = &fy.pools[0];
    let days: Vec<(&str, i64, String)> = p
        .by_day
        .iter()
        .map(|d| (d.date.as_str(), d.seconds.round() as i64, format!("{:.4}", d.usd_per_liquidity_day.as_deref().unwrap().parse::<f64>().unwrap() * 1e15)))
        .collect();
    // True UTC dates (an interval counts on the date it starts): the design's 19:00-UTC
    // cut and its $14.54 day are gone.
    assert_eq!(
        days,
        vec![
            ("2026-11-14", 30699, "3.8394".to_string()),
            ("2026-11-15", 86488, "5.0097".to_string()),
            ("2026-11-16", 86007, "7.1922".to_string()),
            ("2026-11-17", 71106, "9.7357".to_string()),
            ("2026-11-18", 86146, "3.0965".to_string()),
            ("2026-11-19", 85799, "7.9900".to_string()),
            ("2026-11-20", 86400, "5.2919".to_string()),
            ("2026-11-21", 14559, "3.8596".to_string()),
        ]
    );
    // The simulator's spread over the 5 days with >= 72000 s: an odd count, so the median
    // is the middle day; an even count takes the mean of the two middle days.
    let env = crate::tests_fee_yield::fee_fixture();
    let body = crate::tests_ladder_rungs::sim_on(&env, &[("usdc", "741"), ("low", "0.01"), ("high", "spot"), ("position_id", V4_ID)]).unwrap();
    let sp = &body["fee_estimate"]["spread"];
    assert_eq!(sp["days_counted"], 5);
    assert_eq!((sp["min_usd_per_day"].as_str(), sp["median_usd_per_day"].as_str(), sp["max_usd_per_day"].as_str()), (Some("17.98"), Some("30.72"), Some("46.38")));
    assert_eq!(sp["days"].as_array().unwrap().len(), 8);
    let mut env2 = env.clone();
    let pool = &mut env2.data.derived_metrics.fee_yield.as_mut().unwrap().pools[0];
    pool.by_day.retain(|d| d.date != "2026-11-20");
    let body = crate::tests_ladder_rungs::sim_on(&env2, &[("usdc", "741"), ("low", "0.01"), ("high", "spot"), ("position_id", V4_ID)]).unwrap();
    let sp = &body["fee_estimate"]["spread"];
    assert_eq!(sp["days_counted"], 4);
    // Days 5.0097, 7.1922, 3.0965, 7.9900 per 1e15 at $0.01294450, here at the fixture's
    // $0.012944 and its L: the median is the mean of the 2nd and 3rd.
    let vals: Vec<f64> = sp["days"].as_array().unwrap().iter().filter(|d| d["counted"] == true).map(|d| d["usd_per_day"].as_str().unwrap().parse().unwrap()).collect();
    let mut sorted = vals.clone();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let median: f64 = sp["median_usd_per_day"].as_str().unwrap().parse().unwrap();
    assert!((median - (sorted[1] + sorted[2]) / 2.0).abs() <= 0.006, "{median} {sorted:?}");
}

fn row(at: &str, l: u128, o0: f64, o1: f64, tick: i32) -> FeeYieldRow {
    FeeYieldRow { at: t(at), position_id: V4_ID.to_string(), liquidity: Some(l), owed0: Some(o0), owed1: Some(o1), in_range: Some(true), pool_active: Some(true), tick: Some(tick), staked: false }
}

fn measure(rows: &[FeeYieldRow]) -> FeeYieldPool {
    let env = fixture();
    measure_pool(&env.data.positions[0], rows)
}

#[test]
fn test_fee_yield_skip_rules_each_count_in_their_own_bucket() {
    let l = 1_000_000_000_000_000u128;
    let mut rows = vec![
        row("2026-11-20T00:00:00Z", l, 0.0, 0.0, 274000),
        row("2026-11-20T00:05:00Z", l, 0.010, 1.0, 274050), // counted: 300 s
        row("2026-11-20T00:10:00Z", l + 1, 0.011, 1.1, 274050), // liquidity changed
        row("2026-11-20T00:15:00Z", l + 1, 0.001, 1.2, 274050), // fees fell (a collect)
        row("2026-11-20T03:15:00Z", l + 1, 0.002, 1.3, 274100), // a 3 h gap with fees: gap
        row("2026-11-20T06:15:00Z", l + 1, 0.002, 1.3, 274100), // a 3 h gap, nothing changed: zero-fee
    ];
    let mut oor = row("2026-11-20T06:20:00Z", l + 1, 0.003, 1.4, 274100);
    oor.in_range = Some(false);
    rows.push(oor); // out of range at one end
    let mut back = row("2026-11-20T06:25:00Z", l + 1, 0.004, 1.5, 274100);
    back.pool_active = None;
    rows.push(back.clone()); // out_of_range (the previous end), then pool_inactive next
    rows.push(row("2026-11-20T06:30:00Z", l + 1, 0.005, 1.6, 274100)); // pool inactive at one end
    rows.push(row("2026-11-20T06:35:00Z", 0, 0.005, 1.6, 274100)); // liquidity changed to 0
    rows.push(row("2026-11-20T06:40:00Z", 0, 0.005, 1.6, 274100)); // zero liquidity
    let mut unread = row("2026-11-20T06:45:00Z", 0, 0.005, 1.6, 274100);
    unread.owed0 = None;
    rows.push(unread); // unreadable
    // Out of order on input: the rows are sorted by time first.
    rows.reverse();
    let p = measure(&rows);
    assert_eq!(
        p.skipped,
        FeeYieldSkips { liquidity_changed: 2, zero_liquidity: 1, fees_fell: 1, pool_inactive: 2, out_of_range: 1, gap: 1, unreadable: 1, staked: 0 }
    );
    assert_eq!(p.intervals, 2);
    assert_eq!(p.zero_fee_gap_intervals, 1);
    // 300 s with fees plus 10,800 s of zero-fee gap; the rate is over both.
    assert_eq!(p.measured_seconds, 11100.0);
    assert!((p.skipped_seconds.gap - 10800.0).abs() < 1e-9);
    let u: f64 = p.usdc_per_liquidity_day.as_deref().unwrap().parse().unwrap();
    assert!(rel(u, 0.010 / l as f64 / (11100.0 / 86400.0)) < 1e-6);
    let n: f64 = p.nock_per_liquidity_day.as_deref().unwrap().parse().unwrap();
    assert!(rel(n, 1.0 / l as f64 / (11100.0 / 86400.0)) < 1e-6);
    assert_eq!(p.status, "measured");
}

#[test]
fn test_fee_yield_spans_place_each_interval_on_the_tick_grid() {
    let l = 1_000_000_000_000_000u128;
    let rows = vec![
        row("2026-11-20T00:00:00Z", l, 0.0, 0.0, 274050),
        row("2026-11-20T00:05:00Z", l, 0.01, 1.0, 274150), // both ends in the 273900 bin
        row("2026-11-20T00:10:00Z", l, 0.02, 2.0, 274199), // still the 273900 bin
        row("2026-11-20T00:15:00Z", l, 0.03, 3.0, 274200), // 273900 to 274200: one span across two bins
        row("2026-11-20T00:20:00Z", l, 0.04, 4.0, 274499), // both ends in the 274200 bin
        row("2026-11-20T00:25:00Z", l, 0.05, 5.0, -150), // a negative tick bins down, to -300
        row("2026-11-20T00:30:00Z", l, 0.06, 6.0, -1),
    ];
    let p = measure(&rows);
    let spans: Vec<(i32, i32, f64)> = p.spans.iter().map(|s| (s.bin_lo, s.bin_hi, s.seconds)).collect();
    assert_eq!(
        spans,
        vec![(-300, -300, 300.0), (-300, 274200, 300.0), (273900, 273900, 600.0), (273900, 274200, 300.0), (274200, 274200, 300.0)]
    );
    assert!(rel(p.spans[3].usdc_per_liquidity.parse::<f64>().unwrap(), 0.01 / l as f64) < 1e-6);
    // A pool whose spacing was not read keeps the rate and says why it has no spans.
    let mut env = fixture();
    env.data.positions[0].tick_spacing = None;
    let p = measure_pool(&env.data.positions[0], &rows);
    assert!(p.spans.is_empty());
    assert!(p.spans_reason.as_deref().unwrap().contains("tick spacing was not read"));
    assert_eq!(p.intervals, 6);
}

#[test]
fn test_fee_yield_with_too_little_history_is_no_intervals_with_a_reason() {
    let env = fixture();
    let fy = fee_yield_from_rows(&env.data.positions, &[], computed_at());
    assert_eq!(fy.status, "measured");
    let p = &fy.pools[0];
    assert_eq!(p.status, "no_intervals");
    assert!(p.reason.as_deref().unwrap().contains("0 stored rows"), "{:?}", p.reason);
    assert_eq!((p.usdc_per_liquidity_day.as_ref(), p.nock_per_liquidity_day.as_ref()), (None, None));
    // Liquidity changing at every row: nothing can be counted, and the reason says so.
    let l = 1_000_000_000_000_000u128;
    let rows: Vec<FeeYieldRow> = (0..4).map(|i| row(&format!("2026-11-20T00:0{i}:00Z"), l + i as u128, 0.0, 0.0, 274000)).collect();
    let p = measure(&rows);
    assert_eq!(p.status, "no_intervals");
    assert_eq!(p.skipped.liquidity_changed, 3);
    assert!(p.reason.as_deref().unwrap().contains("none of the 3 intervals"), "{:?}", p.reason);
}

#[test]
fn test_fee_yield_chooses_its_measuring_positions_deliberately() {
    let mut env = fixture();
    // The fixture: the v4 range measures; the basic Aero pool is excluded; the two
    // gas-side WETH/USDC ranges have no NOCK leg and are not listed at all.
    let (chosen, excluded) = measuring_positions(&env.data.positions);
    assert_eq!(chosen.iter().map(|p| p.position_id.as_str()).collect::<Vec<_>>(), vec![V4_ID]);
    assert_eq!(excluded.len(), 1);
    assert!(excluded[0].position_id.starts_with("aerodrome:base:"));
    assert!(excluded[0].reason.contains("full-range"));
    // A second, narrower range in the same pool, a tracked ladder, a staked Aero CL range
    // and a WETH/NOCK range.
    let base = env.data.positions[0].clone();
    let narrow = Position { position_id: "uniswap-v4:base:1".into(), nft_id: 1, lower_tick: 273900, upper_tick: 276300, ..base.clone() };
    let ladder = Position { position_id: "uniswap-v4:base:2".into(), nft_id: 2, role: Some("ladder".into()), rung: Some("deep".into()), ..base.clone() };
    let staked = Position { position_id: "aerodrome-cl:base:3".into(), protocol: "aerodrome-cl".into(), nft_id: 3, pool_id: "0xaero-cl".into(), staked_in_gauge: Some("0xgauge".into()), ..base.clone() };
    let mut weth = base.clone();
    weth.position_id = "uniswap-v3:base:4".into();
    weth.pool_id = "0xweth-nock".into();
    weth.token0.symbol = "WETH".into();
    weth.token0.address = "0x4200000000000000000000000000000000000006".into();
    env.data.positions.extend([narrow, ladder, staked, weth]);
    let (chosen, excluded) = measuring_positions(&env.data.positions);
    assert_eq!(chosen.iter().map(|p| p.position_id.as_str()).collect::<Vec<_>>(), vec![V4_ID]);
    let reason = |id: &str| excluded.iter().find(|e| e.position_id == id).map(|e| e.reason.clone()).unwrap_or_default();
    assert!(reason("uniswap-v4:base:1").contains("another tracked range measures this pool: uniswap-v4:base:3163450, the widest"));
    assert!(reason("uniswap-v4:base:2").contains("a tracked ladder"));
    assert!(reason("aerodrome-cl:base:3").contains("staked: swap fees on staked liquidity go to the gauge"));
    assert!(reason("uniswap-v3:base:4").contains("WETH/NOCK pool"));
    // Two positions measuring at once would double the seconds: only the widest is read,
    // so measured seconds stay wall-clock seconds.
    let l = 1_000_000_000_000_000u128;
    let mut rows = vec![row("2026-11-20T00:00:00Z", l, 0.0, 0.0, 274000), row("2026-11-20T00:05:00Z", l, 0.01, 1.0, 274000)];
    for r in rows.clone() {
        rows.push(FeeYieldRow { position_id: "uniswap-v4:base:1".into(), ..r });
    }
    let fy = fee_yield_from_rows(&env.data.positions, &rows, computed_at());
    assert_eq!(fy.pools.len(), 1);
    assert_eq!(fy.pools[0].measured_seconds, 300.0);
    assert_eq!(fy.pools[0].by_day[0].seconds, 300.0);
    // With no USDC/NOCK concentrated range at all: unavailable, with the reason.
    let only_aero: Vec<Position> = fixture().data.positions.into_iter().filter(|p| p.full_range).collect();
    let fy = fee_yield_from_rows(&only_aero, &rows, computed_at());
    assert_eq!(fy.status, "unavailable");
    assert!(fy.reason.as_deref().unwrap().contains("nothing measures them"));
    assert!(fy.pools.is_empty());
}

#[test]
fn test_fee_yield_is_measured_at_most_hourly_and_carried_forward() {
    let ids = vec![V4_ID.to_string()];
    let at = computed_at();
    assert!(fee_yield_is_fresh(at, &ids, &ids, at + chrono::Duration::minutes(59)));
    assert!(!fee_yield_is_fresh(at, &ids, &ids, at + chrono::Duration::minutes(60)));
    // A different measuring set is measured again at once.
    assert!(!fee_yield_is_fresh(at, &ids, &["uniswap-v4:base:1".to_string()], at + chrono::Duration::minutes(1)));
    // A clock that went backwards is not "fresh".
    assert!(!fee_yield_is_fresh(at, &ids, &ids, at - chrono::Duration::minutes(1)));
}

#[test]
fn test_fee_yield_and_active_liquidity_leave_the_state_hash_and_dedup_unchanged() {
    let env = fixture();
    let mut bare = env.data.clone();
    bare.derived_metrics.fee_yield = None;
    for m in &mut bare.pool_mechanics {
        m.active_liquidity_raw = None;
    }
    let with = fee_fixture().data;
    assert!(with.derived_metrics.fee_yield.is_some());
    assert!(with.pool_mechanics[0].active_liquidity_raw.is_some());
    // The same wallet and price hash the same with or without the measurement: an
    // unchanged cycle still stores no new row.
    assert_eq!(compute_state_hash(&with), compute_state_hash(&bare));
    // A different measurement (it changes every hour) does not change it either.
    let mut later = with.clone();
    later.derived_metrics.fee_yield.as_mut().unwrap().computed_at = Some("2026-11-20T12:50:00Z".into());
    later.pool_mechanics[0].active_liquidity_raw = Some("1".into());
    assert_eq!(compute_state_hash(&later), compute_state_hash(&bare));
    // It is still in the stored payload.
    let stored = serde_json::to_value(&with).unwrap();
    assert!(stored["derived_metrics"]["fee_yield"]["pools"][0]["spans"].as_array().unwrap().len() > 10);
    assert_eq!(stored["pool_mechanics"][0]["active_liquidity_raw"], "80173935548453779");
}

#[test]
fn test_active_liquidity_is_stamped_only_when_read() {
    let env = fixture();
    let mut pools = env.data.pool_mechanics.clone();
    for m in &mut pools {
        m.active_liquidity_raw = None;
    }
    let book = crate::ladder::stored_book(&env.data);
    let mut reads: HashMap<String, String> = HashMap::new();
    reads.insert(V4_ID.to_string(), "195500000000000000".to_string());
    // The stored book holds only the NOCK-pair positions, in snapshot order: the v4
    // range and the Aero pool, the first two rows.
    let mut two = pools[..2].to_vec();
    stamp_active_liquidity(&mut two, &book.raw.positions, &reads);
    assert_eq!(two[0].active_liquidity_raw.as_deref(), Some("195500000000000000"));
    // A failed read (no key) stays absent: unknown, never 0.
    assert_eq!(two[1].active_liquidity_raw, None);
}

#[test]
fn test_fee_yield_spec_documents_the_block() {
    let snap: serde_yaml::Value = serde_yaml::from_str(include_str!("../../specs/core/snapshot.yaml")).unwrap();
    let schemas = &snap["components"]["schemas"];
    let fy = serde_json::to_value(fee_fixture().data.derived_metrics.fee_yield.unwrap()).unwrap();
    for key in fy.as_object().unwrap().keys() {
        assert!(schemas["FeeYield"]["properties"][key.as_str()].is_mapping(), "FeeYield.{key}");
    }
    for key in fy["pools"][0].as_object().unwrap().keys() {
        assert!(schemas["FeeYieldPool"]["properties"][key.as_str()].is_mapping(), "FeeYieldPool.{key}");
    }
    assert!(schemas["DerivedMetrics"]["properties"]["fee_yield"].is_mapping());
    assert!(schemas["PoolMechanic"]["properties"]["active_liquidity_raw"].is_mapping());
    for k in ["ladder_bottom_usd", "ladder_bottom_note", "ladder_rungs", "ladder_rungs_note"] {
        assert!(schemas["GlobalStatus"]["properties"][k].is_mapping(), "GlobalStatus.{k}");
    }
    let text = include_str!("../../specs/core/snapshot.yaml");
    assert!(text.contains("a UTC date"));
    assert!(text.contains("never used"));
}

// Regenerate only for an intended change:
//   NOCKNOCK_BLESS_GOLDEN=1 cargo test test_golden_fee_yield
// The dashboard (frontend/src/feeYield.test.ts) reads this block, so its helpers are
// pinned to the worker's own output.
#[test]
fn test_golden_fee_yield_block_is_byte_identical() {
    let fy = fee_fixture().data.derived_metrics.fee_yield.unwrap();
    let doc = json!({
        "about": "derived_metrics.fee_yield of the #166 fixture with the recorded chudmax history of #3163450 (tests_fee_yield.rs), priced at the fixture's pool price $0.012944.",
        "fee_yield": fy,
    });
    let actual = format!("{}\n", serde_json::to_string_pretty(&doc).unwrap());
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/fee_yield/golden_fee_yield.json");
    if std::env::var("NOCKNOCK_BLESS_GOLDEN").is_ok() {
        std::fs::write(path, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(path).expect("testdata/fee_yield/golden_fee_yield.json is checked in");
    assert!(actual == expected, "fee_yield bytes changed; compare {path} with a fresh NOCKNOCK_BLESS_GOLDEN=1 run and `git diff` it");
}

#[test]
fn test_fee_yield_never_measures_a_staked_period() {
    // 20 days staked (owed carried as 0, constant liquidity, in range), then 5 days
    // unstaked earning fees: only the unstaked days are measured, and the staked seconds
    // are stated (review of #166 rungs, finding 7).
    let l = 1_000_000_000_000_000u128;
    let start = t("2026-10-01T00:00:00Z");
    let mut rows = Vec::new();
    let step = chrono::Duration::minutes(5);
    let mut owed = 0.0;
    for i in 0..(25 * 288) {
        let staked = i < 20 * 288;
        if !staked {
            owed += 1e-6;
        }
        let mut r = row("2026-10-01T00:00:00Z", l, if staked { 0.0 } else { owed }, 0.0, 274000);
        r.at = start + step * i;
        r.staked = staked;
        rows.push(r);
    }
    let p = measure(&rows);
    // The interval from the last staked row to the first unstaked one is staked too.
    assert_eq!(p.skipped.staked, 20 * 288);
    assert!((p.skipped_seconds.staked - 20.0 * 86400.0).abs() < 1e-6);
    assert_eq!(p.intervals as usize, 5 * 288 - 1);
    let u: f64 = p.usdc_per_liquidity_day.as_deref().unwrap().parse().unwrap();
    // 1e-6 USDC per 5 minutes on 1e15 L: 288e-6 / 1e15 per day.
    assert!(rel(u, 288e-6 / l as f64) < 1e-6, "{u}");
}
