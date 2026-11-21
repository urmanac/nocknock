//! The ladder suggestion (issue #168).
use super::*;
use serde_json::Value;

const V4_ID: &str = "uniswap-v4:base:3163450";
const ZERO_HOOKS: &str = "0x0000000000000000000000000000000000000000";

fn s<'a>(v: &'a Value, ptr: &str) -> &'a str {
    v.pointer(ptr).and_then(|x| x.as_str()).unwrap_or_else(|| panic!("{ptr} missing in {v:#}"))
}
fn f(v: &Value, ptr: &str) -> f64 {
    s(v, ptr).parse().unwrap()
}

/// The poolKeys facts a 2.9.0 cycle now stamps on the v4 position (#168), as the recorded
/// answer in testdata/closed/cycle_fixture.json states them.
fn with_pool_key(mut env: SnapshotEnvelope) -> SnapshotEnvelope {
    for p in env.data.positions.iter_mut().filter(|p| p.protocol == "uniswap-v4") {
        p.fee_raw = Some(30000);
        p.hooks = Some(ZERO_HOOKS.to_string());
    }
    env
}

/// The operator's live snapshot from chudmax (2026-11-21T04:04Z), before the cycle read
/// fee_raw and hooks; the pool key is added as the cycle now reads it.
fn live() -> SnapshotEnvelope {
    let env: SnapshotEnvelope =
        serde_json::from_str(include_str!("../testdata/ladder_suggest/live_snapshot_2026-11-21.json")).expect("live snapshot parses");
    with_pool_key(env)
}

/// The #166 simulator fixture (tests_ladder.rs), with the pool key.
fn sim_fixture() -> SnapshotEnvelope {
    with_pool_key(crate::tests_ladder::fixture())
}

fn suggest(env: &SnapshotEnvelope, bottom: Option<f64>) -> Value {
    ladder_suggestion(env, Ok(bottom))
}

fn text(env: &SnapshotEnvelope, l: &Value, health: Option<&str>) -> String {
    let mut data = serde_json::to_value(&env.data).unwrap();
    data["derived_metrics"]["ladder_suggestion"] = l.clone();
    crate::ladder_suggest_text::ladder_suggestion_lines(&data, health)
}

#[test]
fn test_ladder_suggest_live_fixture_is_ready() {
    let env = live();
    let l = suggest(&env, Some(0.01));
    assert_eq!(s(&l, "/status"), "ready", "{l:#}");
    assert_eq!(l["shown"], json!(true));
    assert_eq!(s(&l, "/id"), LADDER_SUGGEST_ID);
    assert_eq!(s(&l, "/tier"), "top");
    let r = &l["rungs"][0];
    assert_eq!(l["rungs"].as_array().unwrap().len(), 1);
    assert_eq!(r["tick_lower"], json!(273900));
    assert_eq!(r["tick_upper"], json!(276300));
    assert_eq!(s(r, "/top/price_usd"), "0.01274292");
    assert_eq!(s(r, "/bottom/price_usd"), "0.01002405");
    assert_eq!(s(r, "/nock/amount"), "0.0000");
    assert_eq!(s(r, "/nock/raw"), "0");
    // The reserve's ceiling at this snapshot: $766.32 (the lead's $766.84 is an earlier
    // cycle's figure; the existing ranges' USDC at the bottom moves with every cycle).
    assert_eq!(s(r, "/usdc/amount"), "766.32");
    assert_eq!(s(r, "/usdc/raw"), "766320000");
    assert_eq!(r["usdc"]["decimals"], json!(6));
    assert_eq!(s(r, "/pool/position_id"), V4_ID);
    assert_eq!(s(r, "/pool/pool_id"), "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13");
    assert_eq!(r["pool"]["fee_raw"], json!(30000));
    assert_eq!(r["pool"]["tick_spacing"], json!(300));
    assert_eq!(s(r, "/pool/token0/address"), "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913");
    assert_eq!(r["pool"]["token1"]["decimals"], json!(16));
    // Both forms of the bounds: USD per NOCK and NOCK per USDC (the app's form).
    assert_eq!(s(r, "/bounds_usd/low"), "0.01002405");
    assert_eq!(s(r, "/bounds_usd/high"), "0.01274292");
    assert_eq!(s(r, "/bounds_nock_per_usdc/low"), "78.4749");
    assert_eq!(s(r, "/bounds_nock_per_usdc/high"), "99.7601");
    // The gates, each stated.
    assert_eq!(s(&l, "/gates/reserve/status"), "allows");
    assert!(f(&l, "/gates/reserve/stable_at_bottom_usdc") >= 950.0);
    assert!(f(&l, "/gates/reserve/stable_at_bottom_usdc") <= 950.01);
    assert_eq!(s(&l, "/gates/nock_budget/status"), "fits");
    assert_eq!(s(&l, "/gates/floor_breach_effect"), "none");
    assert_eq!(s(&l, "/minimum_usdc"), "1.00");
    assert_eq!(s(&l, "/headline"), "Ladder ready: $766.32 of idle USDC in 1 deposit, 0 NOCK.");
    let nock_at_bottom = f(&l, "/numbers/nock_at_bottom");
    assert!((nock_at_bottom - 67_800.0).abs() < 200.0, "{nock_at_bottom}");
    // The plan budget and tax reserve are stated, not gates.
    assert!(s(&l, "/stated/note").contains("stated, not gates"));
}

#[test]
fn test_ladder_suggest_sim_fixture_matches_the_simulator() {
    let env = sim_fixture();
    let l = suggest(&env, Some(0.01));
    assert_eq!(s(&l, "/status"), "ready", "{l:#}");
    let r = &l["rungs"][0];
    // The #166 test pins the same ceiling for this range (tests_ladder.rs).
    assert_eq!(s(r, "/usdc/amount"), "767.45");
    assert_eq!(s(r, "/top/pct_of_spot"), "-1.55");
    // The average buy price is the geometric mean of the bounds for a USDC-only range.
    let avg = f(r, "/at_bottom/avg_buy_price_usd");
    let gm = (f(r, "/bounds_usd/low") * f(r, "/bounds_usd/high")).sqrt();
    assert!((avg - gm).abs() < 1e-8, "{avg} vs {gm}");
    // Re-running the simulator with the suggested amount leaves the reserve held.
    let params: HashMap<String, String> = [("usdc", "767.45"), ("low", "0.01"), ("high", "spot"), ("position_id", V4_ID)]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
    let b = simulate_ladder(&env, &LadderRequest::from_params(&params).unwrap(), env.meta.generated_at, "ok").unwrap();
    assert_eq!(s(&b, "/reserve_check/status"), "holds");
    assert_eq!(s(&b, "/bottom/nock_held"), s(r, "/at_bottom/nock_held"));
}

#[test]
fn test_ladder_suggest_outcomes_in_plain_words() {
    let l = suggest(&sim_fixture(), Some(0.01));
    let o = l["outcomes"].as_array().unwrap();
    let kinds: Vec<&str> = o.iter().map(|x| x["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["fall_inside", "fall_to_bottom", "rise"]);
    assert!(s(&o[0], "/text").starts_with("A fall to $"), "{}", s(&o[0], "/text"));
    assert!(s(&o[0], "/text").contains("NOCK for $"));
    assert!(s(&o[1], "/text").starts_with("A fall to the ladder bottom $0.01002405 (-22.56%) would buy about"));
    assert!(s(&o[1], "/text").contains("with all $767.45, an average of $"));
    assert_eq!(s(&o[2], "/text"), "A rise sells nothing: the ladder holds 0 NOCK, so its USDC stays USDC.");
    // The partial fall buys less, at a higher average, than the whole fall.
    assert!(f(&o[0], "/nock_bought") < f(&o[1], "/nock_bought"));
    assert!(f(&o[0], "/avg_buy_price_usd") > f(&o[1], "/avg_buy_price_usd"));
    // The picture's data: spot, one bar, the bottom line and the reserve zone.
    assert_eq!(s(&l, "/picture/reserve_label"), "$950 kept for below $0.01");
    assert_eq!(s(&l, "/picture/bars/0/label"), "$767.45");
    assert_eq!(s(&l, "/picture/bottom_usd"), "0.01002405");
    assert!(f(&l, "/picture/spot_usd") > f(&l, "/picture/bars/0/high_usd"));
}

#[test]
fn test_ladder_suggest_deep_link_format() {
    let l = suggest(&sim_fixture(), Some(0.01));
    let url = s(&l, "/rungs/0/deep_link/url");
    assert_eq!(
        url,
        "https://app.uniswap.org/positions/add/new?chain=base&protocolVersion=v4&currencyA=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913&currencyB=0x9B5E262cF9bb04869ab40b19AF91D2dc85761722&fee=%7B%22feeAmount%22%3A30000%2C%22isDynamic%22%3Afalse%2C%22tickSpacing%22%3A300%7D&priceRangeState=%7B%22fullRange%22%3Afalse%2C%22maxTick%22%3A276300%2C%22minTick%22%3A273900%2C%22priceInverted%22%3Afalse%7D&step=1"
    );
    // Decoded, the fields round-trip; no amount and no hook (the hooks address is zero).
    let u = reqwest::Url::parse(url).unwrap();
    let q: HashMap<String, String> = u.query_pairs().map(|(k, v)| (k.into_owned(), v.into_owned())).collect();
    let fee: Value = serde_json::from_str(&q["fee"]).unwrap();
    assert_eq!(fee, json!({ "feeAmount": 30000, "tickSpacing": 300, "isDynamic": false }));
    let pr: Value = serde_json::from_str(&q["priceRangeState"]).unwrap();
    assert_eq!(pr, json!({ "priceInverted": false, "fullRange": false, "minTick": 273900, "maxTick": 276300 }));
    assert!(!q.contains_key("hook") && !q.contains_key("depositState"), "{q:?}");
    assert_eq!(u.host_str(), Some("app.uniswap.org"));
    assert!(!url.contains("aerodrome.finance"));
    assert_eq!(l["rungs"][0]["deep_link"]["not_prefilled"], json!(["amount"]));
    // A hooked pool's address would be passed; an Aero CL pool gets its deposit page.
    let mut p = sim_fixture().data.positions.into_iter().find(|p| p.position_id == V4_ID).unwrap();
    p.hooks = Some("0x00000000000000000000000000000000000000aa".to_string());
    assert!(new_position_url(&p, 273900, 276300).unwrap().0.contains("hook=0x00000000000000000000000000000000000000aa"));
    p.protocol = "aerodrome-cl".to_string();
    p.pool_id = "0xpool".to_string();
    assert_eq!(new_position_url(&p, 1, 2).unwrap(), ("https://app.aero.xyz/deposit/8453-0xpool".to_string(), vec![]));
    p.protocol = "aerodrome".to_string();
    assert!(new_position_url(&p, 1, 2).unwrap_err().contains("full range"));
}

#[test]
fn test_ladder_suggest_tracked_ladder_means_no_card() {
    let mut env = sim_fixture();
    env.data.positions[0].role = Some(LADDER_ROLE.to_string());
    let l = suggest(&env, Some(0.01));
    assert_eq!(s(&l, "/status"), "ladder_tracked");
    assert_eq!(l["shown"], json!(false));
    assert!(l["rungs"].as_array().unwrap().is_empty());
    assert_eq!(s(&l, "/tracked/0/position_id"), V4_ID);
    assert_eq!(s(&l, "/headline"), "Ladder: tracked. No new one is suggested.");
    assert!(s(&l, "/reason").contains("A ladder is tracked"));
    let t = text(&env, &l, Some("ok"));
    assert!(t.contains("Ladder: tracked. No new one is suggested. (ladder_tracked) A ladder is tracked"), "{t}");
}

#[test]
fn test_ladder_suggest_no_bottom_shows_the_candidate_table() {
    let env = sim_fixture();
    let l = suggest(&env, None);
    assert_eq!(s(&l, "/status"), "needs_setting");
    assert_eq!(l["missing"], json!(["operator_profile.ladder_bottom_usd"]));
    assert_eq!(s(&l, "/headline"), "Ladder: set your bottom price (operator_profile.ladder_bottom_usd) to see one.");
    let c = &l["candidates"];
    assert_eq!((s(c, "/to_pct"), s(c, "/step_pct")), ("-40", "5"));
    let rows = c["rows"].as_array().unwrap();
    assert!(rows.len() >= 6, "{c:#}");
    for r in rows {
        assert_eq!(r["tick"].as_i64().unwrap() % 300, 0, "snapped to the 300-tick grid: {r}");
        assert!(r.get("selected").is_none() && r.get("picked").is_none(), "no row is picked: {r}");
        assert!(r["usdc_ceiling"].is_string(), "the reserve is set, so each row has a ceiling: {r}");
    }
    // Deeper bottoms buy at a lower average.
    let avgs: Vec<f64> = rows.iter().map(|r| r["avg_buy_price_usd"].as_str().unwrap().parse::<f64>().unwrap()).collect();
    assert!(avgs.windows(2).all(|w| w[1] < w[0]), "{avgs:?}");
    assert!(l["rungs"].as_array().unwrap().is_empty());
    // With no reserve either, both settings are named and no row has an amount.
    let mut env2 = sim_fixture();
    env2.data.derived_metrics.global_status.cash_reserve_usdc = None;
    let l2 = suggest(&env2, None);
    assert_eq!(l2["missing"], json!(["operator_profile.ladder_bottom_usd", "operator_profile.cash_reserve_usdc"]));
    for r in l2["candidates"]["rows"].as_array().unwrap() {
        assert!(r["usdc_ceiling"].is_null() && r["nock_at_bottom"].is_null(), "{r}");
        assert!(r["nock_per_100_usdc"].is_string());
    }
    let t = text(&env, &l, Some("ok"));
    assert!(t.contains("Candidate bottoms (a measurement, none picked; grid to -40% in 5% steps)"), "{t}");
}

#[test]
fn test_ladder_suggest_each_gate_states_its_reason() {
    // Bottom invalid.
    for bad in [0.0, -1.0, f64::NAN, f64::INFINITY] {
        let p = OperatorProfile { ladder_bottom_usd: Some(bad), ..OperatorProfile::default() };
        assert_eq!(ladder_bottom_setting(&p), Err(LADDER_BOTTOM_INVALID_NOTE));
        let l = ladder_suggestion(&sim_fixture(), ladder_bottom_setting(&p));
        assert_eq!((s(&l, "/status"), s(&l, "/reason")), ("invalid", LADDER_BOTTOM_INVALID_NOTE));
    }
    assert_eq!(ladder_bottom_setting(&OperatorProfile { ladder_bottom_usd: Some(0.01), ..OperatorProfile::default() }), Ok(Some(0.01)));
    // Reserve unset.
    let mut env = sim_fixture();
    env.data.derived_metrics.global_status.cash_reserve_usdc = None;
    let l = suggest(&env, Some(0.01));
    assert_eq!(s(&l, "/status"), "needs_setting");
    assert_eq!(l["missing"], json!(["operator_profile.cash_reserve_usdc"]));
    // Reserve invalid.
    env.data.derived_metrics.global_status.reserve_note = Some(RESERVE_INVALID_NOTE.to_string());
    let l = suggest(&env, Some(0.01));
    assert_eq!((s(&l, "/status"), s(&l, "/reason")), ("invalid", RESERVE_INVALID_NOTE));
    // No headroom: a reserve above what the stable balance holds at the bottom.
    let mut env = sim_fixture();
    env.data.derived_metrics.global_status.cash_reserve_usdc = Some("2000.00".to_string());
    let l = suggest(&env, Some(0.01));
    assert_eq!(s(&l, "/status"), "no_headroom");
    assert!(s(&l, "/reason").contains("the $2000.00 reserve leaves $0.00 for a ladder, below the $1.00 minimum"), "{}", s(&l, "/reason"));
    // The bottom at or above the top.
    let l = suggest(&sim_fixture(), Some(0.0128));
    assert_eq!(s(&l, "/status"), "bottom_at_or_above_top", "{l:#}");
    assert!(!s(&l, "/reason").is_empty());
    // No pool: frozen, hooked, dynamic-fee or unread pool key.
    let cases: [(&str, fn(&mut Position)); 4] = [
        ("frozen", |p| p.pool_active = Some(false)),
        ("hooks (0x", |p| p.hooks = Some("0x00000000000000000000000000000000000000aa".to_string())),
        ("dynamic-fee", |p| p.fee_raw = Some(0x800000)),
        ("hooks address was not read", |p| p.hooks = None),
    ];
    for (needle, edit) in cases {
        let mut env = sim_fixture();
        env.data.positions.iter_mut().filter(|p| p.position_id == V4_ID).for_each(edit);
        let l = suggest(&env, Some(0.01));
        assert_eq!(s(&l, "/status"), "no_target_pool", "{needle}");
        let reasons: Vec<&str> = l["excluded"].as_array().unwrap().iter().map(|e| e["reason"].as_str().unwrap()).collect();
        assert!(reasons.iter().any(|r| r.contains(needle)), "{needle}: {reasons:?}");
        assert!(reasons.iter().any(|r| r.contains("full range")), "the Aero basic pool is excluded too: {reasons:?}");
    }
    // Two eligible pools: NockNock does not choose.
    let mut env = sim_fixture();
    let mut twin = env.data.positions.iter().find(|p| p.position_id == V4_ID).unwrap().clone();
    twin.position_id = "uniswap-v4:base:1".to_string();
    twin.pool_id = "0xother".to_string();
    twin.is_price_source = false;
    env.data.positions.push(twin);
    assert_eq!(s(&suggest(&env, Some(0.01)), "/status"), "several_pools");
    // Every non-ready status shows nothing and says why, in the text too.
    for l in [suggest(&sim_fixture(), Some(0.0128)), suggest(&sim_fixture(), None)] {
        assert_eq!(l["shown"], json!(false));
        let t = text(&sim_fixture(), &l, Some("ok"));
        assert!(t.starts_with("• Ladder suggestion: Ladder: "), "{t}");
        assert!(!t.contains("Deposit 1 of"), "{t}");
    }
}

#[test]
fn test_ladder_suggest_stale_snapshot_is_not_shown() {
    let env = sim_fixture();
    let l = suggest(&env, Some(0.01));
    for health in [Some("degraded"), Some("starting"), None] {
        let t = text(&env, &l, health);
        assert!(t.starts_with("• Ladder suggestion: not shown: the snapshot is not fresh (worker health: "), "{t}");
        assert!(!t.contains("https://"), "{t}");
    }
    let fresh = text(&env, &l, Some("ok"));
    assert!(fresh.contains("Deposit 1 of 1: 767.45 USDC + 0.0000 NOCK into"), "{fresh}");
    assert!(fresh.contains("Deposit link (no amount prefilled): https://app.uniswap.org/positions/add/new?"), "{fresh}");
    assert!(fresh.contains("After you deposit (current fallback): "), "{fresh}");
    assert_eq!(s(&l, "/freshness_rule"), LADDER_FRESHNESS_RULE);
}

#[test]
fn test_ladder_suggest_two_rungs_render_as_a_numbered_list() {
    // The operator's chosen "bottle" shape (near 15% from the top down to $0.0110, deep
    // 85% from $0.0110 to $0.01), as a fixture object: the card only lays it out.
    let mut l = suggest(&sim_fixture(), Some(0.01));
    let mut near = l["rungs"][0].clone();
    let mut deep = near.clone();
    near["rung"] = json!("near");
    near["usdc"]["amount"] = json!("115.12");
    near["bounds_usd"] = json!({ "low": "0.01100000", "high": "0.01274292" });
    deep["index"] = json!(2);
    deep["rung"] = json!("deep");
    deep["usdc"]["amount"] = json!("652.33");
    deep["bounds_usd"] = json!({ "low": "0.01002405", "high": "0.01100000" });
    near["of"] = json!(2);
    deep["of"] = json!(2);
    l["rungs"] = json!([near, deep]);
    let t = text(&sim_fixture(), &l, Some("ok"));
    let a = t.find("Deposit 1 of 2 (near rung): 115.12 USDC").expect(&t);
    let b = t.find("Deposit 2 of 2 (deep rung): 652.33 USDC").expect(&t);
    assert!(a < b);
    assert!(t.contains("($0.01002405 to $0.01100000;"), "{t}");
    assert_eq!(t.matches("Deposit link (no amount prefilled)").count(), 2);
}

#[test]
fn test_ladder_suggest_after_deposit_is_one_isolated_fallback() {
    let l = suggest(&sim_fixture(), Some(0.01));
    let a = &l["after_deposit"];
    assert_eq!(s(a, "/kind"), "manual_config");
    assert!(s(a, "/is").contains("fallback"));
    let snip = s(a, "/config_snippets/0");
    for needle in ["protocol: \"uniswap-v4\"", "tick_lower: 273900", "tick_upper: 276300", "role: \"ladder\"", "deposit_tx:", "rung:"] {
        assert!(snip.contains(needle), "{needle}: {snip}");
    }
    // Nothing else in the card names config.yaml or deploy.sh.
    let mut rest = l.clone();
    rest.as_object_mut().unwrap().remove("after_deposit");
    let other = rest.to_string();
    assert!(!other.contains("config.yaml") && !other.contains("deploy.sh"), "{other}");
}

#[test]
fn test_ladder_suggest_wording_is_a_measurement() {
    let env = sim_fixture();
    for l in [suggest(&env, Some(0.01)), suggest(&env, None), suggest(&env, Some(0.0128))] {
        for t in [l.to_string(), text(&env, &l, Some("ok"))] {
            let low = t.to_lowercase();
            for bad in ["should", "recommend", "you must", "deposit $", "buy now"] {
                assert!(!low.contains(bad), "'{bad}' in {t}");
            }
        }
    }
}

#[test]
fn test_ladder_suggest_never_changes_the_directive() {
    let mut data = sim_fixture().data;
    let before = (data.derived_metrics.global_status.shot_advice_action.clone(), data.derived_metrics.global_status.shot_advice_details.clone());
    let profile = OperatorProfile { ladder_bottom_usd: Some(0.01), cash_reserve_usdc: Some(950.0), ..OperatorProfile::default() };
    attach_ladder_suggestion(&mut data, &profile, Utc::now());
    let l = data.derived_metrics.ladder_suggestion.as_ref().unwrap();
    assert_eq!(s(l, "/status"), "ready");
    let after = (data.derived_metrics.global_status.shot_advice_action.clone(), data.derived_metrics.global_status.shot_advice_details.clone());
    assert_eq!(before, after);
    // The cycle attaches it after capturing the directive strings the push is built from.
    let ingest = include_str!("ingest.rs");
    let capture = ingest.find("let next_directive_details").unwrap();
    let attach = ingest.find("attach_ladder_suggestion(&mut snapshot_data, &profile").unwrap();
    assert!(capture < attach);
}

#[test]
fn test_ladder_bottom_usd_parses_from_config() {
    let p: OperatorProfile = serde_yaml::from_str("treasury_floor_nock: 1\nladder_bottom_usd: 0.01\n").unwrap();
    assert_eq!(p.ladder_bottom_usd, Some(0.01));
    let p: OperatorProfile = serde_yaml::from_str("treasury_floor_nock: 1\n").unwrap();
    assert_eq!(p.ladder_bottom_usd, None, "no default");
}

#[test]
fn test_v4_pool_key_fee_raw_and_hooks_from_the_recorded_answers() {
    let fixture: Value = serde_json::from_str(include_str!("../testdata/closed/cycle_fixture.json")).unwrap();
    let mut seen = Vec::new();
    let mut walk = vec![&fixture];
    while let Some(v) = walk.pop() {
        match v {
            Value::String(x) => {
                let clean = x.trim_start_matches("0x");
                if clean.len() == 320 && clean.get(128..192).map_or(false, |w| w.trim_start_matches('0') == "7530") {
                    seen.push(v4_pool_key_fee_raw_and_hooks(clean));
                }
            }
            Value::Array(a) => walk.extend(a.iter()),
            Value::Object(o) => walk.extend(o.values()),
            _ => {}
        }
    }
    assert!(!seen.is_empty());
    assert!(seen.iter().all(|x| *x == (Some(30000), Some(ZERO_HOOKS.to_string()))), "{seen:?}");
    assert_eq!(v4_pool_key_fee_raw_and_hooks(&"0".repeat(160)), (None, None));
}

/// The golden text both MCP transports must print (mcp_bin_tests_ladder_suggest.rs reads it).
/// Regenerate only for an intended change: NOCKNOCK_BLESS_GOLDEN=1 cargo test test_golden_ladder_suggest
#[test]
fn test_golden_ladder_suggest_text() {
    let env = sim_fixture();
    let mut tracked = sim_fixture();
    tracked.data.positions[0].role = Some(LADDER_ROLE.to_string());
    let cases = [
        ("ready", &env, suggest(&env, Some(0.01)), Some("ok")),
        ("ready_stale", &env, suggest(&env, Some(0.01)), Some("degraded")),
        ("needs_setting", &env, suggest(&env, None), Some("ok")),
        ("bottom_at_or_above_top", &env, suggest(&env, Some(0.0128)), Some("ok")),
        ("tracked", &tracked, suggest(&tracked, Some(0.01)), Some("ok")),
    ];
    let mut doc = serde_json::Map::new();
    for (name, e, l, health) in cases {
        doc.insert(
            name.to_string(),
            json!({ "health": health, "ladder_suggestion": l, "text": text(e, &l, health) }),
        );
    }
    let actual = format!("{}\n", serde_json::to_string_pretty(&Value::Object(doc)).unwrap());
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/ladder_suggest/golden_ladder_suggest.json");
    if std::env::var("NOCKNOCK_BLESS_GOLDEN").is_ok() {
        std::fs::write(path, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(path).expect("testdata/ladder_suggest/golden_ladder_suggest.json is checked in");
    assert!(actual == expected, "ladder suggestion text changed; re-bless and review {path}");
}
