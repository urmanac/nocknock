//! Cost basis and the average-cost series (issue #98).
//!
//! The three fills below are the operator's real plan buys, read from Base with
//! eth_getTransactionReceipt and eth_getBlockByNumber on 2026-11-16 (see
//! ONCHAIN_INTEGRATION.md §12): each receipt shows exactly 20000000 raw USDC leaving the
//! wallet and the raw NOCK amount below entering it. The opening-basis figures used in
//! the whole-stack tests (10,000 NOCK for $200) are made up for the tests: round numbers
//! that are nobody's records.
use super::*;

fn t(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

/// (tx hash, block, block time, raw USDC out of the wallet, raw NOCK into the wallet)
const REAL_FILLS: [(&str, i64, &str, u128, u128); 3] = [
    (
        "0x5d0c02d9929f49f71526e0c8d91d1bd99ec19f116befc285d67f6bf9412a3f0e",
        57228848,
        "2026-11-14T06:02:25Z",
        20_000_000,
        15_979_340_475_636_968_138,
    ),
    (
        "0x4644edd99bb2770f369862357eb5647a68a7157ca9e91ecccfce79e6964c61e8",
        57605753,
        "2026-11-15T02:58:46Z",
        20_000_000,
        17_202_181_466_216_939_446,
    ),
    (
        "0x78376f3cc9e80f2eb124a945a452df40556649e3d9983be526b2bd97586e31b9",
        58040708,
        "2026-11-16T03:08:37Z",
        20_000_000,
        17_803_592_976_960_049_083,
    ),
];

/// The real fills as the record handler stores them (format_units of the raw amounts).
fn real_fills() -> Vec<BasisFill> {
    REAL_FILLS
        .iter()
        .map(|(tx, block, at, usdc_raw, nock_raw)| {
            BasisFill::parse("2026-11-reentry", tx, *block, t(at), &format_units(*usdc_raw, 6), &format_units(*nock_raw, 16)).unwrap()
        })
        .collect()
}

fn fill(plan: &str, tx: &str, at: &str, usdc: &str, nock: &str) -> BasisFill {
    BasisFill::parse(plan, tx, 1, t(at), usdc, nock).unwrap()
}

fn opening_cfg(nock: serde_json::Value, cost: serde_json::Value, as_of: serde_json::Value, source: serde_json::Value) -> OpeningBasisConfig {
    let some = |v: serde_json::Value| if v.is_null() { None } else { Some(v) };
    OpeningBasisConfig { nock_amount: some(nock), cost_usd: some(cost), as_of: some(as_of), source: some(source) }
}

const NOW: &str = "2026-11-16T04:00:00Z";

fn test_opening() -> OpeningBasisState {
    resolve_opening_basis(
        Some(&opening_cfg(json!(10000), json!(200), json!("2026-11-14T12:00:00Z"), json!("test figures"))),
        t(NOW),
    )
}

fn spot_point(at: &str, price: f64, source: &str, backfilled: bool) -> SpotPoint {
    SpotPoint { at: t(at), price_usd: price, source: source.to_string(), is_backfilled: backfilled }
}

fn invalid_reason(cfg: &OpeningBasisConfig) -> String {
    match resolve_opening_basis(Some(cfg), t(NOW)) {
        OpeningBasisState::Invalid(r) => r,
        other => panic!("expected a configuration error, got {other:?}"),
    }
}

// ---------------- Layer 1: plan buys ----------------

#[test]
fn test_real_fills_parse_to_the_ledgers_own_amounts() {
    let fills = real_fills();
    assert_eq!(fills[0].usdc_spent, "20");
    assert_eq!(fills[0].nock_received, "1597.9340475636968138");
    assert_eq!(fills[2].nock_received, "1780.3592976960049083");
    // The ledger's stored effective prices (GET /v1/accumulation/events on the deployment).
    let prices: Vec<String> = fills.iter().map(|f| format!("{:.8}", f.usdc / f.nock)).collect();
    assert_eq!(prices, ["0.01251616", "0.01162643", "0.01123369"]);
}

#[test]
fn test_plan_buys_average_is_spend_over_nock_and_difference_is_measured_at_spot() {
    let fills = real_fills();
    let cb = compute_cost_basis(Ok(&fills), &OpeningBasisState::NotConfigured, Some(0.01153), None);
    let p = &cb.plan_buys;
    assert_eq!(p.status, "ok");
    assert_eq!(p.fills, Some(3));
    assert_eq!(p.spent_usdc.as_deref(), Some("60.00"));
    assert_eq!(p.acquired_nock.as_deref(), Some("5098.5115"));
    // 60 / 5098.5114918813956667
    assert_eq!(p.avg_cost_usd.as_deref(), Some("0.011768"));
    assert_eq!(p.first_fill_at.as_deref(), Some("2026-11-14T06:02:25+00:00"));
    assert_eq!(p.last_fill_at.as_deref(), Some("2026-11-16T03:08:37+00:00"));
    assert_eq!(cb.spot_price_usd.as_deref(), Some("0.011530"));
    // 5098.5115 NOCK at $0.01153 = $58.79; $58.79 − $60.00 = −$1.21 = −2.02%.
    assert_eq!(p.value_at_spot_usd.as_deref(), Some("58.79"));
    assert_eq!(p.unrealized_usd.as_deref(), Some("-1.21"));
    assert_eq!(p.unrealized_pct.as_deref(), Some("-2.02"));
    assert_eq!(p.plans.len(), 1);
    assert_eq!(p.plans[0].plan_id, "2026-11-reentry");
    assert_eq!(p.plans[0].avg_cost_usd, "0.011768");
    assert!(p.includes[0].contains("receipt-verified buy"));
    assert!(p.excludes.iter().any(|e| e.contains("sells")));
    // With no opening basis the whole stack is not configured: no number at all.
    let w = &cb.whole_stack;
    assert_eq!(w.status, "not_configured");
    assert!(w.reason.as_deref().unwrap().contains("cost_basis.opening is not set"));
    assert_eq!((w.avg_cost_usd.as_ref(), w.basis_nock.as_ref(), w.unrealized_usd.as_ref()), (None, None, None));
}

#[test]
fn test_plan_buys_are_reported_per_plan_and_across_plans() {
    let fills = vec![
        fill("first", "0xa1", "2026-10-01T00:00:00Z", "100", "5000"),
        fill("first", "0xa2", "2026-10-02T00:00:00Z", "100", "10000"),
        fill("second", "0xb1", "2026-11-01T00:00:00Z", "50", "5000"),
    ];
    let cb = compute_cost_basis(Ok(&fills), &OpeningBasisState::NotConfigured, Some(0.02), None);
    let p = &cb.plan_buys;
    assert_eq!(p.avg_cost_usd.as_deref(), Some("0.012500")); // 250 / 20000
    assert_eq!(p.unrealized_usd.as_deref(), Some("150.00")); // 20000 × 0.02 − 250
    assert_eq!(p.unrealized_pct.as_deref(), Some("60.00"));
    let plans: Vec<(&str, i64, &str)> = p.plans.iter().map(|x| (x.plan_id.as_str(), x.fills, x.avg_cost_usd.as_str())).collect();
    assert_eq!(plans, [("first", 2, "0.013333"), ("second", 1, "0.010000")]);
}

#[test]
fn test_no_fills_and_an_unread_ledger_are_different_states_and_neither_is_zero() {
    let none = compute_cost_basis(Ok(&[]), &OpeningBasisState::NotConfigured, Some(0.01153), None);
    assert_eq!(none.plan_buys.status, "no_fills");
    assert_eq!(none.plan_buys.fills, Some(0), "the ledger was read and is empty: a real zero");
    assert_eq!((none.plan_buys.avg_cost_usd.as_ref(), none.plan_buys.spent_usdc.as_ref(), none.plan_buys.unrealized_usd.as_ref()), (None, None, None));

    let unread = compute_cost_basis(Err("the accumulation ledger could not be read this cycle"), &test_opening(), Some(0.01153), Some(100.0));
    assert_eq!(unread.plan_buys.status, "unavailable");
    assert_eq!(unread.plan_buys.reason.as_deref(), Some("the accumulation ledger could not be read this cycle"));
    assert_eq!(unread.plan_buys.avg_cost_usd, None);
    // The count was not read: null in the snapshot, never a 0 that reads as "no buys".
    assert_eq!(unread.plan_buys.fills, None);
    assert!(serde_json::to_value(&unread).unwrap()["plan_buys"]["fills"].is_null());
    // The blend needs the ledger too: unavailable, with the operator's figures still shown.
    assert_eq!(unread.whole_stack.status, "unavailable");
    assert_eq!(unread.whole_stack.avg_cost_usd, None);
    assert_eq!(unread.whole_stack.opening_nock.as_deref(), Some("10000.0000"));
    assert_eq!(unread.whole_stack.source.as_deref(), Some("test figures"));
}

#[test]
fn test_no_spot_means_no_comparison_never_a_zero_value() {
    for spot in [None, Some(0.0), Some(f64::NAN)] {
        let cb = compute_cost_basis(Ok(&real_fills()), &test_opening(), spot, None);
        assert_eq!(cb.spot_price_usd, None);
        assert_eq!(cb.plan_buys.avg_cost_usd.as_deref(), Some("0.011768"));
        assert_eq!((cb.plan_buys.value_at_spot_usd.as_ref(), cb.plan_buys.unrealized_usd.as_ref(), cb.plan_buys.unrealized_pct.as_ref()), (None, None, None));
        assert_eq!((cb.whole_stack.value_at_spot_usd.as_ref(), cb.whole_stack.unrealized_usd.as_ref()), (None, None));
    }
}

#[test]
fn test_a_ledger_row_that_does_not_parse_fails_the_read_instead_of_being_skipped() {
    for (usdc, nock) in [("", "100"), ("20", "abc"), ("0", "100"), ("20", "-5"), ("20", "0")] {
        let e = BasisFill::parse("p", "0xdead", 1, t(NOW), usdc, nock).unwrap_err();
        assert!(e.contains("0xdead") && e.contains("unreadable"), "{e}");
    }
}

// ---------------- Layer 2: operator-supplied opening basis ----------------

#[test]
fn test_opening_basis_unset_is_not_configured() {
    assert_eq!(resolve_opening_basis(None, t(NOW)), OpeningBasisState::NotConfigured);
    // An empty block (every key absent or null) supplies nothing: still not configured.
    assert_eq!(resolve_opening_basis(Some(&OpeningBasisConfig::default()), t(NOW)), OpeningBasisState::NotConfigured);
    let nulls = OpeningBasisConfig {
        nock_amount: Some(serde_json::Value::Null),
        cost_usd: Some(serde_json::Value::Null),
        as_of: Some(serde_json::Value::Null),
        source: Some(serde_json::Value::Null),
    };
    assert_eq!(resolve_opening_basis(Some(&nulls), t(NOW)), OpeningBasisState::NotConfigured);
}

#[test]
fn test_opening_basis_complete_block_is_accepted_as_entered() {
    let state = resolve_opening_basis(
        Some(&opening_cfg(json!(10000.5), json!("200.25"), json!("2026-11-14T16:30:00-05:00"), json!("my own records, read by hand"))),
        t(NOW),
    );
    assert_eq!(
        state,
        OpeningBasisState::Set(OpeningBasis {
            nock: 10000.5,
            cost_usd: 200.25,
            as_of: t("2026-11-14T21:30:00Z"),
            source: "my own records, read by hand".to_string(),
        })
    );
}

#[test]
fn test_partial_opening_basis_is_a_configuration_error_not_a_default() {
    // Amount without cost.
    let r = invalid_reason(&opening_cfg(json!(10000), json!(null), json!("2026-11-14T12:00:00Z"), json!("notes")));
    assert!(r.contains("cost_usd is missing"), "{r}");
    assert!(r.starts_with("cost_basis.opening cannot be used:"), "{r}");
    assert!(r.contains("only plan buys are reported"), "{r}");
    // Cost without amount, and nothing else: every missing field is named at once.
    let r = invalid_reason(&opening_cfg(json!(null), json!(200), json!(null), json!(null)));
    for needle in ["nock_amount is missing", "as_of is missing", "source is missing"] {
        assert!(r.contains(needle), "{needle} not in: {r}");
    }
    // A source note is required: no figure without a stated source.
    let r = invalid_reason(&opening_cfg(json!(10000), json!(200), json!("2026-11-14T12:00:00Z"), json!("   ")));
    assert!(r.contains("source is missing"), "{r}");
}

#[test]
fn test_zero_negative_or_non_numeric_amounts_are_configuration_errors() {
    let ok_time = json!("2026-11-14T12:00:00Z");
    let r = invalid_reason(&opening_cfg(json!(0), json!(200), ok_time.clone(), json!("notes")));
    assert!(r.contains("nock_amount must be greater than zero (it is 0)"), "{r}");
    let r = invalid_reason(&opening_cfg(json!(-5.5), json!(200), ok_time.clone(), json!("notes")));
    assert!(r.contains("nock_amount must be greater than zero (it is -5.5)"), "{r}");
    // A zero or negative cost is refused too: it would be shown as a real average.
    let r = invalid_reason(&opening_cfg(json!(10000), json!(0), ok_time.clone(), json!("notes")));
    assert!(r.contains("cost_usd must be greater than zero"), "{r}");
    let r = invalid_reason(&opening_cfg(json!(10000), json!(-1), ok_time.clone(), json!("notes")));
    assert!(r.contains("cost_usd must be greater than zero"), "{r}");
    // A placeholder word left in place is named, not parsed as 0.
    let r = invalid_reason(&opening_cfg(json!("<NOCK you held>"), json!(200), ok_time, json!("notes")));
    assert!(r.contains("nock_amount must be a number (it is \"<NOCK you held>\")"), "{r}");
}

#[test]
fn test_as_of_must_be_a_full_past_timestamp() {
    // In the future.
    let r = invalid_reason(&opening_cfg(json!(10000), json!(200), json!("2026-11-16T04:00:01Z"), json!("notes")));
    assert!(r.contains("as_of 2026-11-16T04:00:01Z is in the future"), "{r}");
    // Exactly now is not the future.
    assert!(matches!(
        resolve_opening_basis(Some(&opening_cfg(json!(10000), json!(200), json!(NOW), json!("notes"))), t(NOW)),
        OpeningBasisState::Set(_)
    ));
    // A date alone is ambiguous about that day's buys, so it is refused with the reason.
    let r = invalid_reason(&opening_cfg(json!(10000), json!(200), json!("2026-11-14"), json!("notes")));
    assert!(r.contains("as_of \"2026-11-14\" must be a full timestamp with a UTC offset"), "{r}");
    assert!(r.contains("a date alone does not say whether a buy made that day is already inside the figures"), "{r}");
    // No offset: also refused (whose midnight?).
    let r = invalid_reason(&opening_cfg(json!(10000), json!(200), json!("2026-11-14T12:00:00"), json!("notes")));
    assert!(r.contains("must be a full timestamp with a UTC offset"), "{r}");
}

#[test]
fn test_whole_stack_blends_the_opening_basis_with_buys_after_the_as_of_only() {
    // as_of 2026-11-14T12:00Z: the first real fill (06:02Z that day) is inside the opening
    // figures; the second and third are added.
    let cb = compute_cost_basis(Ok(&real_fills()), &test_opening(), Some(0.01153), Some(12000.0));
    let w = &cb.whole_stack;
    assert_eq!(w.status, "ok");
    assert_eq!(w.reason, None);
    assert_eq!(w.source.as_deref(), Some("test figures"));
    assert_eq!(w.as_of.as_deref(), Some("2026-11-14T12:00:00+00:00"));
    assert_eq!(w.opening_nock.as_deref(), Some("10000.0000"));
    assert_eq!(w.opening_cost_usd.as_deref(), Some("200.00"));
    assert_eq!(w.opening_avg_cost_usd.as_deref(), Some("0.020000"));
    assert_eq!(w.fills_since, Some(2));
    assert_eq!(w.bought_since_usdc.as_deref(), Some("40.00"));
    assert_eq!(w.bought_since_nock.as_deref(), Some("3500.5774"));
    // (200 + 40) / (10000 + 3500.5774443176988529)
    assert_eq!(w.basis_nock.as_deref(), Some("13500.5774"));
    assert_eq!(w.basis_cost_usd.as_deref(), Some("240.00"));
    assert_eq!(w.avg_cost_usd.as_deref(), Some("0.017777"));
    // 13500.5774 NOCK at $0.01153 = $155.66; $155.66 − $240.00 = −$84.34 = −35.14%.
    assert_eq!(w.value_at_spot_usd.as_deref(), Some("155.66"));
    assert_eq!(w.unrealized_usd.as_deref(), Some("-84.34"));
    assert_eq!(w.unrealized_pct.as_deref(), Some("-35.14"));
    // The cycle passes the snapshot's total_nock_amount, a whole number of NOCK.
    assert_eq!(w.held_nock.as_deref(), Some("12000"));
    // The statements name the source, the instant, and what is left out.
    assert!(w.includes[0].contains("10000.0000 NOCK for $200.00 as of 2026-11-14T12:00:00+00:00, source: test figures"), "{:?}", w.includes);
    assert!(w.includes[0].contains("NockNock did not read or check them"));
    assert!(w.includes[1].contains("2 so far"));
    let excluded = w.excludes.join(" ");
    for needle in ["at or before 2026-11-14T12:00:00+00:00", "sells", "LP ranges", "fees earned", "unrecorded buys", "Realized"] {
        assert!(excluded.contains(needle), "{needle} not in: {excluded}");
    }
    // The plan-buys layer is unchanged by the opening basis.
    assert_eq!(cb.plan_buys.avg_cost_usd.as_deref(), Some("0.011768"));
}

#[test]
fn test_a_fill_confirmed_exactly_at_the_as_of_is_inside_the_opening_figures() {
    let opening = resolve_opening_basis(
        Some(&opening_cfg(json!(1000), json!(20), json!("2026-11-15T02:58:46Z"), json!("test figures"))),
        t(NOW),
    );
    let cb = compute_cost_basis(Ok(&real_fills()), &opening, Some(0.01153), None);
    assert_eq!(cb.whole_stack.fills_since, Some(1), "only the 2026-11-16 fill is after the as-of");
    assert_eq!(cb.whole_stack.bought_since_usdc.as_deref(), Some("20.00"));
}

#[test]
fn test_an_invalid_opening_basis_reports_the_error_and_no_whole_stack_number() {
    let opening = resolve_opening_basis(Some(&opening_cfg(json!(10000), json!(null), json!(null), json!(null))), t(NOW));
    let cb = compute_cost_basis(Ok(&real_fills()), &opening, Some(0.01153), Some(12000.0));
    let w = &cb.whole_stack;
    assert_eq!(w.status, "invalid");
    assert!(w.reason.as_deref().unwrap().contains("cost_usd is missing"));
    assert_eq!((w.avg_cost_usd.as_ref(), w.opening_nock.as_ref(), w.basis_cost_usd.as_ref()), (None, None, None));
    // Plan buys still report.
    assert_eq!(cb.plan_buys.status, "ok");
}

// ---------------- Config ----------------

fn load_with(extra: &str) -> AppConfig {
    let dir = std::env::temp_dir().join(format!("nocknock-cost-basis-{}", Uuid::new_v4()));
    std::fs::create_dir_all(&dir).unwrap();
    let main = dir.join("config.yaml");
    std::fs::write(&main, extra).unwrap();
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/config.yaml.example");
    let cfg = load_app_config(example, Some(main.to_str().unwrap()), false).unwrap();
    let _ = std::fs::remove_file(&main);
    let _ = std::fs::remove_dir(&dir);
    cfg
}

#[test]
fn test_the_example_config_sets_no_opening_basis() {
    let cfg = load_with("port: 8080\n");
    assert!(cfg.opening_basis().is_none());
    // A bare key with everything under it still commented out loads, and means the same.
    for bare in ["cost_basis:\n", "cost_basis:\n  opening:\n"] {
        let cfg = load_with(bare);
        assert_eq!(resolve_opening_basis(cfg.opening_basis(), t(NOW)), OpeningBasisState::NotConfigured, "{bare:?}");
    }
    assert_eq!(resolve_opening_basis(cfg.opening_basis(), t(NOW)), OpeningBasisState::NotConfigured);
    // The documented block is there, commented out, with words rather than numbers.
    let example = include_str!("../config.yaml.example");
    assert!(example.contains("# cost_basis:"), "config.yaml.example must document cost_basis");
    for key in ["nock_amount", "cost_usd", "as_of", "source"] {
        let line = example
            .lines()
            .find(|l| l.trim_start().starts_with(&format!("#     {key}:")))
            .unwrap_or_else(|| panic!("{key} is not documented under # cost_basis: opening"));
        assert!(line.contains('<') && line.contains('>'), "{key} must carry a placeholder in words: {line}");
        assert!(!line.chars().any(|c| c.is_ascii_digit()), "{key} must not carry anyone's numbers: {line}");
    }
    // Uncommented as shipped, placeholders and all, the block still loads (the worker
    // starts) and is reported as a configuration error naming what to replace.
    let start = example.find("# cost_basis:").unwrap();
    let block: String = example[start..]
        .lines()
        .take_while(|l| l.starts_with('#'))
        .map(|l| format!("{}\n", l.strip_prefix("# ").unwrap_or("")))
        .collect();
    assert!(block.starts_with("cost_basis:\n  opening:\n    nock_amount:"), "{block}");
    let cfg = load_with(&block);
    let OpeningBasisState::Invalid(r) = resolve_opening_basis(cfg.opening_basis(), t(NOW)) else {
        panic!("the placeholder block must be a configuration error");
    };
    for needle in ["nock_amount must be a number", "cost_usd must be a number", "as_of \"<the instant", "must be a full timestamp"] {
        assert!(r.contains(needle), "{needle} not in: {r}");
    }
}

#[test]
fn test_opening_basis_loads_from_yaml_and_a_half_filled_block_does_not_stop_the_worker() {
    let cfg = load_with(
        "cost_basis:\n  opening:\n    nock_amount: 10000\n    cost_usd: 200.5\n    as_of: \"2026-11-14T21:30:00Z\"\n    source: \"my records\"\n",
    );
    assert_eq!(
        resolve_opening_basis(cfg.opening_basis(), t(NOW)),
        OpeningBasisState::Set(OpeningBasis { nock: 10000.0, cost_usd: 200.5, as_of: t("2026-11-14T21:30:00Z"), source: "my records".to_string() })
    );
    // The documented placeholders left in place, and an unquoted date: the config still
    // loads (the worker starts) and the block is reported as an error.
    let cfg = load_with(
        "cost_basis:\n  opening:\n    nock_amount: <NOCK you held>\n    as_of: 2026-11-14\n    source: my records\n",
    );
    let OpeningBasisState::Invalid(r) = resolve_opening_basis(cfg.opening_basis(), t(NOW)) else {
        panic!("expected a configuration error");
    };
    for needle in ["nock_amount must be a number", "cost_usd is missing", "must be a full timestamp"] {
        assert!(r.contains(needle), "{needle} not in: {r}");
    }
}

// ---------------- Series ----------------

#[test]
fn test_fill_series_carries_running_averages_per_plan_across_plans_and_blended() {
    let OpeningBasisState::Set(opening) = test_opening() else { panic!() };
    let spot = vec![
        spot_point("2026-11-14T06:00:00Z", 0.0124, "uniswap_rpc", false),
        spot_point("2026-11-14T06:05:00Z", 0.0125, "uniswap_rpc", false),
        // Nothing stored within 60 minutes of the second fill (02:58:46Z on the 15th).
        spot_point("2026-11-15T04:10:00Z", 0.0117, "geckoterminal", true),
        spot_point("2026-11-16T03:00:00Z", 0.0112, "geckoterminal", true),
    ];
    let rows = fill_series(&real_fills(), Some(&opening), &spot);
    assert_eq!(rows.len(), 3);
    let s = |i: usize, k: &str| rows[i][k].as_str().map(|x| x.to_string());
    assert_eq!(s(0, "tx_hash").as_deref(), Some(REAL_FILLS[0].0));
    assert_eq!(rows[0]["block_number"], json!(57228848));
    assert_eq!(s(0, "confirmed_at").as_deref(), Some("2026-11-14T06:02:25+00:00"));
    assert_eq!(s(0, "usdc_spent").as_deref(), Some("20"));
    assert_eq!(s(0, "nock_received").as_deref(), Some("1597.9340475636968138"));
    let col = |k: &str| (0..3).map(|i| s(i, k)).collect::<Vec<_>>();
    let some = |v: [&str; 3]| v.iter().map(|x| Some(x.to_string())).collect::<Vec<_>>();
    assert_eq!(col("effective_price_usd"), some(["0.01251616", "0.01162643", "0.01123369"]));
    assert_eq!(col("avg_cost_usd"), some(["0.01251616", "0.01205490", "0.01176814"]));
    assert_eq!(col("plan_avg_cost_usd"), col("avg_cost_usd"), "one plan: both averages agree");
    // The first fill predates the as-of: no blended value for it, never the opening average.
    assert_eq!(
        col("whole_stack_avg_cost_usd"),
        vec![None, Some("0.01877098".to_string()), Some("0.01777702".to_string())]
    );
    // Spot at matching times: the nearest stored point within 60 minutes, else null.
    assert_eq!(rows[0]["spot_at_fill"]["at"], json!("2026-11-14T06:00:00+00:00"));
    assert_eq!(rows[0]["spot_at_fill"]["price_usd"], json!("0.01240000"));
    assert_eq!(rows[0]["spot_at_fill"]["is_backfilled"], json!(false));
    assert!(rows[1]["spot_at_fill"].is_null(), "71 minutes away is not a match: {}", rows[1]["spot_at_fill"]);
    assert_eq!(rows[2]["spot_at_fill"]["source"], json!("geckoterminal"));
    assert_eq!(rows[2]["spot_at_fill"]["is_backfilled"], json!(true));

    // Without an opening basis there is no blended column value at all.
    let rows = fill_series(&real_fills(), None, &spot);
    assert!(rows.iter().all(|r| r["whole_stack_avg_cost_usd"].is_null()));

    // Two plans: the per-plan average restarts, the ledger-wide one does not.
    let two = vec![
        fill("first", "0xa1", "2026-10-01T00:00:00Z", "100", "5000"),
        fill("second", "0xb1", "2026-11-01T00:00:00Z", "50", "5000"),
    ];
    let rows = fill_series(&two, None, &[]);
    assert_eq!(rows[1]["plan_avg_cost_usd"], json!("0.01000000"));
    assert_eq!(rows[1]["avg_cost_usd"], json!("0.01500000"));
    assert!(rows[1]["spot_at_fill"].is_null());
}

#[test]
fn test_nearest_spot_respects_the_window_on_both_sides() {
    let pts = vec![
        spot_point("2026-11-14T00:00:00Z", 1.0, "a", false),
        spot_point("2026-11-14T02:00:00Z", 2.0, "a", false),
    ];
    let w = chrono::Duration::minutes(60);
    assert_eq!(nearest_spot(&pts, t("2026-11-14T00:20:00Z"), w).map(|p| p.price_usd), Some(1.0));
    assert_eq!(nearest_spot(&pts, t("2026-11-14T01:40:00Z"), w).map(|p| p.price_usd), Some(2.0));
    assert_eq!(nearest_spot(&pts, t("2026-11-14T03:00:00Z"), w).map(|p| p.price_usd), Some(2.0));
    assert_eq!(nearest_spot(&pts, t("2026-11-13T23:00:00Z"), w).map(|p| p.price_usd), Some(1.0));
    assert!(nearest_spot(&pts, t("2026-11-14T03:00:01Z"), w).is_none());
    assert!(nearest_spot(&pts, t("2026-11-13T22:59:59Z"), w).is_none());
    assert!(nearest_spot(&[], t("2026-11-14T00:00:00Z"), w).is_none());
}

#[test]
fn test_nearest_spot_prefers_the_workers_own_read_over_a_backfilled_point() {
    let w = chrono::Duration::minutes(60);
    let pts = vec![
        spot_point("2026-11-15T02:10:00Z", 0.0119, "uniswap_rpc", false),
        spot_point("2026-11-15T03:00:00Z", 0.0117, "geckoterminal", true),
        spot_point("2026-11-15T03:40:00Z", 0.0116, "uniswap_rpc", false),
    ];
    // The candle is 74 seconds away, but two of the worker's own reads are inside the
    // window: the nearer of those (41 minutes) is taken, not the third-party point.
    let hit = nearest_spot(&pts, t("2026-11-15T02:58:46Z"), w).unwrap();
    assert_eq!((hit.at, hit.is_backfilled), (t("2026-11-15T03:40:00Z"), false));
    // With no worker read inside the window, the backfilled point stands in.
    let hit = nearest_spot(&pts[1..2], t("2026-11-15T02:58:46Z"), w).unwrap();
    assert!(hit.is_backfilled);
    // A worker read outside the window is not preferred over a backfilled one inside it,
    // and nothing outside the window is ever returned.
    let far = vec![
        spot_point("2026-11-15T01:00:00Z", 0.0121, "uniswap_rpc", false),
        spot_point("2026-11-15T03:00:00Z", 0.0117, "geckoterminal", true),
    ];
    assert_eq!(nearest_spot(&far, t("2026-11-15T02:58:46Z"), w).map(|p| p.at), Some(t("2026-11-15T03:00:00Z")));
    assert!(nearest_spot(&far[..1], t("2026-11-15T02:58:46Z"), w).is_none());
}

#[test]
fn test_a_stored_candle_gives_its_open_at_its_open_time_never_its_close() {
    // market_price_history stores a third-party candle at its OPEN time with price_usd set
    // to its CLOSE (market.rs upsert_price_candles), so price_usd is the price one candle
    // period later. The prices below are made up for the test.
    //
    // Two hourly candles around the real second buy (2026-11-15T02:58:46Z):
    //   02:00 open 0.01190 close 0.01170   (the close is the price at 03:00)
    //   03:00 open 0.01170 close 0.01140   (the close is the price at 04:00)
    let stored = [
        ("2026-11-15T02:00:00Z", 0.01170, Some(0.01190)),
        ("2026-11-15T03:00:00Z", 0.01140, Some(0.01170)),
    ];
    let pts: Vec<SpotPoint> = stored
        .iter()
        .filter_map(|(at, close, open)| SpotPoint::from_stored(t(at), *close, *open, "geckoterminal", true))
        .collect();
    assert_eq!(pts.iter().map(|p| p.price_usd).collect::<Vec<_>>(), [0.01190, 0.01170], "each point is the candle's open");
    let rows = fill_series(&real_fills()[1..2], None, &pts);
    // The 03:00 row is 74 seconds after the buy, and what it carries is the price at
    // 03:00 (its open), not the price at 04:00 (its close).
    assert_eq!(rows[0]["spot_at_fill"]["at"], json!("2026-11-15T03:00:00+00:00"));
    assert_eq!(rows[0]["spot_at_fill"]["price_usd"], json!("0.01170000"));
    assert_eq!(rows[0]["spot_at_fill"]["is_backfilled"], json!(true));

    // A daily candle at 00:00 matched to a buy at 00:30 gives the price 30 minutes before
    // the buy (its open), never the close 23.5 hours after it.
    let daily = SpotPoint::from_stored(t("2026-11-15T00:00:00Z"), 0.0111, Some(0.0122), "geckoterminal", true).unwrap();
    let hit = nearest_spot(std::slice::from_ref(&daily), t("2026-11-15T00:30:00Z"), chrono::Duration::minutes(60)).unwrap();
    assert_eq!(hit.price_usd, 0.0122);

    // An observation has no OHLC columns and is stamped when it was taken: the worker's
    // read, and the single DEXScreener seed (backfilled, but not a candle). Its price_usd
    // is the price at its timestamp.
    let read = SpotPoint::from_stored(t("2026-11-15T02:55:00Z"), 0.01175, None, "uniswap_rpc", false).unwrap();
    assert_eq!((read.price_usd, read.is_backfilled), (0.01175, false));
    let seed = SpotPoint::from_stored(t("2026-11-15T02:55:00Z"), 0.01176, None, "dexscreener", true).unwrap();
    assert_eq!((seed.price_usd, seed.is_backfilled), (0.01176, true));

    // A candle whose open cannot be used yields no point: its close is not a substitute.
    for bad_open in [0.0, -1.0, f64::NAN] {
        assert_eq!(SpotPoint::from_stored(t("2026-11-15T02:00:00Z"), 0.0117, Some(bad_open), "geckoterminal", true), None);
    }
    // Nor does an observation without a positive price.
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        assert_eq!(SpotPoint::from_stored(t("2026-11-15T02:00:00Z"), bad, None, "uniswap_rpc", false), None);
    }
}

/// The stored-history read against PostgreSQL. Skipped without DATABASE_URL, like the
/// other DB-gated tests. It writes three rows at a random instant in the 2000s (far from
/// any real price row), reads them back and deletes exactly those rows.
#[tokio::test]
async fn test_load_spot_points_reads_a_candle_as_its_open_and_an_observation_as_itself() {
    let Ok(url) = std::env::var("DATABASE_URL") else {
        eprintln!("DATABASE_URL not set; skipping DB integration test");
        return;
    };
    let pool = match PgPool::connect(&url).await {
        Ok(pool) => pool,
        Err(e) => {
            eprintln!("Postgres unreachable ({e}); skipping DB integration test");
            return;
        }
    };
    // The migration's own table (20261012000004), for a database that has not run it.
    let ddl = "CREATE TABLE IF NOT EXISTS market_price_history (
            timestamp      TIMESTAMPTZ NOT NULL PRIMARY KEY,
            price_usd      FLOAT8 NOT NULL,
            open_usd       FLOAT8,
            high_usd       FLOAT8,
            low_usd        FLOAT8,
            close_usd      FLOAT8,
            volume_usd     FLOAT8,
            source         VARCHAR NOT NULL,
            is_backfilled  BOOLEAN NOT NULL DEFAULT false,
            backfilled_at  TIMESTAMPTZ
        )";
    if sqlx::query(ddl).execute(&pool).await.is_err() {
        // A parallel test creating the same table can win the race; once more settles it.
        sqlx::query(ddl).execute(&pool).await.expect("create market_price_history");
    }
    let offset = (Uuid::new_v4().as_u128() % 250_000_000) as i64;
    let open = DateTime::<Utc>::from_timestamp(978_307_200 + offset, 0).unwrap(); // 2001-01-01 + offset
    let (read_at, seed_at) = (open + chrono::Duration::minutes(10), open + chrono::Duration::minutes(20));
    // As the worker stores them: a completed hourly candle (price_usd = its close), its
    // own read, and a DEXScreener seed (backfilled, no OHLC).
    let candle = PriceCandle { timestamp: open, open: 0.0119, high: 0.0120, low: 0.0116, close: 0.0117, volume: Some(5.0) };
    let added = upsert_price_candles(&pool, &[candle], PRICE_SOURCE_GECKOTERMINAL, chrono::Duration::hours(1), open + chrono::Duration::hours(2))
        .await
        .unwrap();
    assert_eq!(added, 1);
    insert_price_point(&pool, read_at, 0.01185, PRICE_SOURCE_RPC, false).await.unwrap();
    insert_price_point(&pool, seed_at, 0.0118, PRICE_SOURCE_DEXSCREENER, true).await.unwrap();

    let got = load_spot_points(&pool, open - chrono::Duration::minutes(1), open + chrono::Duration::minutes(30)).await;
    for at in [open, read_at, seed_at] {
        sqlx::query("DELETE FROM market_price_history WHERE timestamp = $1").bind(at).execute(&pool).await.unwrap();
    }
    assert_eq!(
        got.unwrap(),
        vec![
            // The candle's open at its open time: 0.0119, not the 0.0117 close stored in price_usd.
            SpotPoint { at: open, price_usd: 0.0119, source: "geckoterminal".to_string(), is_backfilled: true },
            SpotPoint { at: read_at, price_usd: 0.01185, source: "uniswap_rpc".to_string(), is_backfilled: false },
            SpotPoint { at: seed_at, price_usd: 0.0118, source: "dexscreener".to_string(), is_backfilled: true },
        ]
    );
}

#[test]
fn test_downsampling_keeps_real_points_and_leaves_gaps_empty() {
    let pts = vec![
        spot_point("2026-11-14T00:01:00Z", 1.0, "uniswap_rpc", false),
        spot_point("2026-11-14T00:31:00Z", 2.0, "uniswap_rpc", false),
        spot_point("2026-11-14T00:59:00Z", 3.0, "uniswap_rpc", false),
        // Five hours with nothing stored.
        spot_point("2026-11-14T06:10:00Z", 4.0, "geckoterminal", true),
    ];
    let out = downsample_spot(&pts, 3600);
    // One point per hour bucket that has data: the last stored one, with its own time.
    assert_eq!(out, vec![pts[2].clone(), pts[3].clone()]);
    // No point is made up for 01:00 to 06:00, and no price is an average of others.
    assert!(out.iter().all(|p| pts.contains(p)));
    // A bucket no wider than the spacing changes nothing.
    assert_eq!(downsample_spot(&pts, 300), pts);
    assert!(downsample_spot(&[], 300).is_empty());
}

#[test]
fn test_series_bucket_and_default_range() {
    assert_eq!(series_bucket_seconds(86_400, 600), 300); // one day at the sync cadence
    assert_eq!(series_bucket_seconds(7 * 86_400, 600), 1800);
    assert_eq!(series_bucket_seconds(30 * 86_400, 600), 7200);
    assert_eq!(series_bucket_seconds(365 * 86_400, 600), 86_400);
    assert_eq!(series_bucket_seconds(0, 600), 300);
    let now = t(NOW);
    assert_eq!(default_series_days(None, now), 30);
    assert_eq!(default_series_days(Some(t("2026-11-14T06:02:25Z")), now), 3, "two days back, rounded up, plus a day");
    assert_eq!(default_series_days(Some(t("2026-11-16T03:59:00Z")), now), 2);
    assert_eq!(default_series_days(Some(now), now), 2, "never shorter than two days");
    assert_eq!(default_series_days(Some(t("2026-10-01T00:00:00Z")), now), 48);
    assert_eq!(default_series_days(Some(t("2020-01-01T00:00:00Z")), now), 365);
}

#[test]
fn test_series_response_thins_spot_in_range_and_survives_a_failed_spot_read() {
    let (from, to) = (t("2026-11-14T00:00:00Z"), t(NOW));
    let spot = vec![
        spot_point("2026-11-13T23:00:00Z", 0.0130, "geckoterminal", true), // before the range
        spot_point("2026-11-14T06:00:00Z", 0.0124, "uniswap_rpc", false),
        spot_point("2026-11-14T06:04:00Z", 0.0125, "uniswap_rpc", false),
        spot_point("2026-11-16T03:10:00Z", 0.0112, "uniswap_rpc", false),
    ];
    let body = build_series_response(&real_fills(), &test_opening(), Ok(&spot), from, to);
    assert_eq!(body["from"], json!("2026-11-14T00:00:00+00:00"));
    assert_eq!(body["bucket_seconds"], json!(900), "52 hours at 5 minutes would be 624 points");
    assert_eq!(body["spot_match_window_minutes"], json!(60));
    let times: Vec<&str> = body["spot"].as_array().unwrap().iter().map(|p| p["at"].as_str().unwrap()).collect();
    assert_eq!(times, ["2026-11-14T06:04:00+00:00", "2026-11-16T03:10:00+00:00"], "06:00 and 06:04 share a bucket; 23:00 is out of range");
    assert!(body["spot_error"].is_null());
    assert_eq!(body["fills"].as_array().unwrap().len(), 3);
    assert_eq!(body["whole_stack"]["status"], json!("ok"));
    assert_eq!(body["whole_stack"]["opening"]["avg_cost_usd"], json!("0.02000000"));
    assert_eq!(body["whole_stack"]["opening"]["source"], json!("test figures"));

    // Spot history unreadable: spot is null with the reason, fills and averages remain.
    let body = build_series_response(&real_fills(), &OpeningBasisState::NotConfigured, Err("the stored price history could not be read"), from, to);
    assert!(body["spot"].is_null());
    assert_eq!(body["spot_error"], json!("the stored price history could not be read"));
    assert_eq!(body["fills"][2]["avg_cost_usd"], json!("0.01176814"));
    assert!(body["fills"][2]["spot_at_fill"].is_null());
    assert_eq!(body["whole_stack"], json!({ "status": "not_configured", "reason": null, "opening": null }));

    // No fills and no history: empty lists, not placeholders.
    let body = build_series_response(&[], &OpeningBasisState::Invalid("bad".to_string()), Ok(&[]), from, to);
    assert_eq!(body["spot"], json!([]));
    assert_eq!(body["fills"], json!([]));
    assert_eq!(body["whole_stack"]["status"], json!("invalid"));
    assert_eq!(body["whole_stack"]["reason"], json!("bad"));
}

// ---------------- Snapshot shape and MCP text ----------------

fn snapshot_with(cb: Option<CostBasis>) -> serde_json::Value {
    let dm = DerivedMetrics { cost_basis: cb, ..Default::default() };
    json!({ "derived_metrics": serde_json::to_value(&dm).unwrap() })
}

#[test]
fn test_a_stored_snapshot_without_cost_basis_still_parses_as_unknown() {
    let mut v = serde_json::to_value(DerivedMetrics::default()).unwrap();
    assert!(v.get("cost_basis").is_some_and(|c| c.is_null()), "the field is always present; null when not computed");
    v.as_object_mut().unwrap().remove("cost_basis");
    let old: DerivedMetrics = serde_json::from_value(v).unwrap();
    assert_eq!(old.cost_basis, None);
    // And a full block round-trips.
    let cb = compute_cost_basis(Ok(&real_fills()), &test_opening(), Some(0.01153), Some(12000.0));
    let back: CostBasis = serde_json::from_value(serde_json::to_value(&cb).unwrap()).unwrap();
    assert_eq!(back, cb);
}

#[test]
fn test_historical_rows_keep_the_cost_basis_figures_and_drop_the_repeated_prose() {
    let cb = compute_cost_basis(Ok(&real_fills()), &test_opening(), Some(0.01153), Some(12000.0));
    assert!(!cb.plan_buys.includes.is_empty() && !cb.whole_stack.excludes.is_empty());
    let mut data = SnapshotData {
        ladders: Vec::new(),
        closed_positions: vec![],
        wallet_balances: vec![],
        positions: vec![],
        pool_mechanics: vec![],
        concentrated_liquidity: vec![],
        contract_fees: vec![],
        derived_metrics: DerivedMetrics { cost_basis: Some(cb.clone()), ..Default::default() },
    };
    slim_historical_payload(&mut data);
    let slim = data.derived_metrics.cost_basis.unwrap();
    assert!(slim.plan_buys.includes.is_empty() && slim.plan_buys.excludes.is_empty());
    assert!(slim.whole_stack.includes.is_empty() && slim.whole_stack.excludes.is_empty());
    assert_eq!(slim.plan_buys.avg_cost_usd, cb.plan_buys.avg_cost_usd);
    assert_eq!(slim.whole_stack.avg_cost_usd, cb.whole_stack.avg_cost_usd);
    assert_eq!(slim.whole_stack.source, cb.whole_stack.source);
    // A row from before the field has nothing to slim.
    let mut old = SnapshotData {
        ladders: Vec::new(),
        closed_positions: vec![],
        wallet_balances: vec![],
        positions: vec![],
        pool_mechanics: vec![],
        concentrated_liquidity: vec![],
        contract_fees: vec![],
        derived_metrics: DerivedMetrics::default(),
    };
    slim_historical_payload(&mut old);
    assert_eq!(old.derived_metrics.cost_basis, None);

    // Issue #136: a row stored while a tranche was due drops its route quotes and the
    // offer lists; detection's status and check times stay, and so do the plan figures.
    let offer = FillOffer {
        tx_hash: "0x78376f3c0000000000000000000000000000000000000000000000000000006e31b9".to_string(),
        confirmed_at: "2026-11-16T03:08:37+00:00".to_string(),
        block_number: 58_000_000,
        usdc_spent: "20.000000".to_string(),
        nock_received: "1780.3600".to_string(),
        effective_price_usd: "0.011234".to_string(),
        timing: "early".to_string(),
        reference_spot_usd: Some("0.01128675".to_string()),
        reference_spot_source: Some("worker".to_string()),
        execution_cost_bps: Some(-47),
    };
    let mut plan = build_plan_status(
        &AccumulationPlanConfig {
            id: "2026-11-reentry".to_string(),
            budget_usdc: 100.0,
            tranches: 5,
            interval_hours: 24.0,
            starts_at: None,
            venue: None,
            max_price_usd: None,
            pause_when_overextended: true,
            respect_cash_target: true,
            cash_reserve_usdc: None,
        },
        &PlanFills::default(),
        &PlanDirective::TrancheDue { usdc: 20.0, limited_by: None },
    );
    plan.route_quotes = Some(route_quotes_unavailable("20.00", "no Alchemy RPC key is configured", Some(0.0112867512)));
    plan.fill_detection = Some(FillDetection {
        status: "ok".to_string(),
        detail: None,
        checked_back_to: Some("2026-11-14T00:00:00+00:00".to_string()),
        last_checked_at: Some("2026-11-16T04:00:00+00:00".to_string()),
        offers: vec![offer.clone(); 3],
        dismissed: vec![offer; 2],
    });
    let mut due = SnapshotData {
        ladders: Vec::new(),
        closed_positions: vec![],
        wallet_balances: vec![],
        positions: vec![],
        pool_mechanics: vec![],
        concentrated_liquidity: vec![],
        contract_fees: vec![],
        derived_metrics: DerivedMetrics { accumulation_plan: Some(plan.clone()), ..Default::default() },
    };
    let before = serde_json::to_vec(&due).unwrap().len();
    slim_historical_payload(&mut due);
    let slim = due.derived_metrics.accumulation_plan.as_ref().unwrap();
    assert_eq!(slim.route_quotes, None);
    let fd = slim.fill_detection.as_ref().unwrap();
    assert!(fd.offers.is_empty() && fd.dismissed.is_empty());
    assert_eq!((fd.status.as_str(), fd.last_checked_at.as_deref()), ("ok", Some("2026-11-16T04:00:00+00:00")));
    assert_eq!((slim.status.as_str(), slim.tranche_usdc.as_str(), slim.spent_usdc.as_str()), ("tranche_due", "20.00", "0.00"));
    assert!(serde_json::to_vec(&due).unwrap().len() < before);
}

/// Words that would turn a measurement into advice (lore: "Why not buy more now?").
const ADVICE_WORDS: [&str; 12] = [
    "buy more", "average down", "averaging down", "should", "opportunity", "cheap", "discount",
    "bargain", "accumulate", "recommend", "consider", "underwater",
];

/// Claims about things NockNock does not read (sells, LP activity).
const UNREAD_CLAIMS: [&str; 3] = ["nothing was sold", "not sold", "never sold"];

fn assert_no_advice(text: &str) {
    let lower = text.to_lowercase();
    for w in ADVICE_WORDS.iter().chain(UNREAD_CLAIMS.iter()) {
        assert!(!lower.contains(w), "\"{w}\" in: {text}");
    }
}

#[test]
fn test_mcp_cost_basis_lines_state_facts_for_every_state() {
    // Plan buys only.
    let cb = compute_cost_basis(Ok(&real_fills()), &OpeningBasisState::NotConfigured, Some(0.01153), None);
    let text = cost_basis_lines(&snapshot_with(Some(cb)));
    assert_eq!(
        text,
        "• Plan buys average cost: $0.011768 per NOCK over 3 verified buy(s) ($60.00 for 5098.5115 NOCK). At spot $0.011530 that NOCK is worth $58.79: $1.21 below cost (-2.02%).\n\
         • Whole-stack average cost: not configured (cost_basis.opening is unset; NockNock does not derive it from the chain)"
    );
    assert_no_advice(&text);

    // With an opening basis: labelled with the source, the instant and what is not tracked.
    let cb = compute_cost_basis(Ok(&real_fills()), &test_opening(), Some(0.02), Some(12000.0));
    let text = cost_basis_lines(&snapshot_with(Some(cb)));
    let whole = text.lines().nth(1).unwrap();
    assert_eq!(
        whole,
        "• Whole-stack average cost: $0.017777 per NOCK on 13500.5774 NOCK ($240.00). Operator-entered opening basis: 10000.0000 NOCK for $200.00 as of 2026-11-14T12:00:00+00:00, source: test figures; plus 2 verified buy(s) since. At spot $0.020000 the NOCK this basis covers is worth $270.01: $30.01 above cost (+12.50%). NockNock reads 12000 NOCK held now (wallet plus tracked LP). Not tracked after the as-of: sells, LP rebalancing, fees earned, unrecorded buys."
    );
    assert!(text.lines().next().unwrap().contains("$41.97 above cost (+69.95%)"), "{text}");
    assert_no_advice(&text);
    // The value at spot is of the NOCK the basis covers; without the NOCK read now there
    // is nothing to compare it with, and the line says so instead of staying silent.
    let cb = compute_cost_basis(Ok(&real_fills()), &test_opening(), Some(0.02), None);
    let text = cost_basis_lines(&snapshot_with(Some(cb)));
    assert!(text.contains("the NOCK this basis covers is worth $270.01: $30.01 above cost (+12.50%). The NOCK held now is not reported by this snapshot. Not tracked"), "{text}");

    // No buys yet; ledger unread; bad config; no spot; a snapshot from before the field.
    let cb = compute_cost_basis(Ok(&[]), &OpeningBasisState::NotConfigured, Some(0.01153), None);
    assert!(cost_basis_lines(&snapshot_with(Some(cb))).starts_with("• Plan buys average cost: no verified buys recorded yet\n"));
    let cb = compute_cost_basis(Err("the accumulation ledger could not be read this cycle"), &OpeningBasisState::NotConfigured, Some(0.01153), None);
    assert!(cost_basis_lines(&snapshot_with(Some(cb)))
        .starts_with("• Plan buys average cost: unavailable (the accumulation ledger could not be read this cycle)\n"));
    let bad = resolve_opening_basis(Some(&opening_cfg(json!(10000), json!(null), json!(null), json!(null))), t(NOW));
    let cb = compute_cost_basis(Ok(&real_fills()), &bad, Some(0.01153), None);
    let text = cost_basis_lines(&snapshot_with(Some(cb)));
    assert!(text.contains("• Whole-stack average cost: configuration error. cost_basis.opening cannot be used: cost_usd is missing"), "{text}");
    let cb = compute_cost_basis(Ok(&real_fills()), &OpeningBasisState::NotConfigured, None, None);
    let text = cost_basis_lines(&snapshot_with(Some(cb)));
    assert!(text.contains("No spot price in this snapshot, so no comparison with cost."), "{text}");
    assert!(!text.contains("$0.00"), "{text}");
    assert_eq!(cost_basis_lines(&snapshot_with(None)), "• Cost basis: not reported by this snapshot");
    assert_eq!(cost_basis_lines(&json!({})), "• Cost basis: not reported by this snapshot");
}

#[test]
fn test_portfolio_text_carries_the_cost_basis_lines_before_the_directive() {
    let cb = compute_cost_basis(Ok(&real_fills()), &test_opening(), Some(0.01153), Some(12000.0));
    let data = snapshot_with(Some(cb));
    let text = format_portfolio_state_text(&data);
    let lines = cost_basis_lines(&data);
    assert!(text.contains(&lines), "{text}");
    let (basis_at, directive_at) = (text.find("• Plan buys average cost").unwrap(), text.find("• Tactical Directive").unwrap());
    assert!(basis_at < directive_at);
    assert_no_advice(&lines);
}

#[test]
fn test_both_mcp_transports_say_the_portfolio_tool_carries_cost_basis() {
    let clause = "the average cost of the plan's verified NOCK buys against spot (plus the whole-stack average when an opening basis is configured)";
    for (name, source) in [("mcp_http.rs", include_str!("mcp_http.rs")), ("bin/mcp.rs", include_str!("bin/mcp.rs"))] {
        let line = source
            .lines()
            .find(|l| l.contains("\"description\": \"Returns current sovereign portfolio balances"))
            .unwrap_or_else(|| panic!("{name}: get_portfolio_state description not found"));
        assert!(line.contains(clause), "{name}: {line}");
    }
}

#[test]
fn test_the_specs_document_the_cost_basis_fields_and_the_series_endpoint() {
    let snapshot = include_str!("../../specs/core/snapshot.yaml");
    for needle in ["CostBasis:", "PlanBuysBasis:", "WholeStackBasis:", "cost_basis:", "unrealized_usd", "opening_cost_usd", "not_configured"] {
        assert!(snapshot.contains(needle), "specs/core/snapshot.yaml lacks {needle}");
    }
    let api = include_str!("../../specs/core/api.yaml");
    for needle in ["/cost-basis/series:", "whole_stack_avg_cost_usd", "spot_at_fill", "bucket_seconds"] {
        assert!(api.contains(needle), "specs/core/api.yaml lacks {needle}");
    }
}
