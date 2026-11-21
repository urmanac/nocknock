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

fn params(pairs: &[(&str, &str)]) -> HashMap<String, String> {
    pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect()
}

/// The simulator, unsnapped, at exactly the operator's bottom (the price-source pool).
fn reserve_at_setting(env: &SnapshotEnvelope, usdc: &str, bottom: &str) -> Value {
    let req = LadderRequest::from_params(&params(&[("usdc", usdc), ("low", bottom), ("high", "spot")])).unwrap();
    simulate_ladder(env, &req, env.meta.generated_at, "ok").unwrap()
}

/// Move the v4 pool's price to `usd` (USD per NOCK) in the stored snapshot.
fn move_v4_price(env: &mut SnapshotEnvelope, usd: f64) {
    let raw = 1e10 / usd; // NOCK (16) per USDC (6), raw
    let v = raw.sqrt() * 79228162514264337593543950336.0;
    let two128 = 340282366920938463463374607431768211456.0f64;
    let hi = (v / two128).floor();
    let hex = format!("{:032x}{:032x}", hi as u128, (v - hi * two128) as u128);
    let tick = (raw.ln() / 1.0001f64.ln()).floor() as i32;
    let pool_id = env.data.positions.iter().find(|p| p.position_id == V4_ID).unwrap().pool_id.clone();
    for m in env.data.pool_mechanics.iter_mut().filter(|m| m.pool_id == pool_id) {
        m.sqrt_price_x96 = Some(hex.clone());
        m.tick_current = Some(tick);
    }
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
    // The reserve read at the operator's own $0.0100 (the lower of it and the snapped
    // bottom $0.01002405), not at the snapped tick: $765.86, not $766.32.
    assert_eq!(s(r, "/usdc/amount"), "765.86");
    assert_eq!(s(r, "/usdc/raw"), "765860000");
    assert_eq!(s(&l, "/gates/reserve/read_at_usd"), "0.01000000");
    assert_eq!(s(&l, "/gates/reserve/binding"), "reserve");
    assert_eq!(r["usdc"]["decimals"], json!(6));
    assert_eq!(s(r, "/pool/position_id"), V4_ID);
    assert_eq!(s(r, "/pool/label"), "the USDC/NOCK 3% v4 pool");
    assert_eq!(r["pool"]["new_position"], json!(true));
    assert_eq!(s(r, "/pool/pool_id"), "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13");
    assert_eq!(r["pool"]["fee_raw"], json!(30000));
    assert_eq!(r["pool"]["tick_spacing"], json!(300));
    assert_eq!(s(r, "/pool/token0/address"), "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913");
    assert_eq!(r["pool"]["token1"]["decimals"], json!(16));
    // Both forms of the bounds, NOCK per USDC from the ticks: the app shows 78.474965 and
    // 99.760036.
    assert_eq!(s(r, "/bounds_usd/low"), "0.01002405");
    assert_eq!(s(r, "/bounds_usd/high"), "0.01274292");
    assert_eq!(s(r, "/bounds_nock_per_usdc/low"), "78.4750");
    assert_eq!(s(r, "/bounds_nock_per_usdc/high"), "99.7600");
    assert_eq!(s(&l, "/gates/reserve/status"), "allows");
    assert_eq!(s(&l, "/gates/nock_budget/status"), "fits");
    assert_eq!(s(&l, "/gates/floor_breach_effect"), "none");
    assert_eq!(s(&l, "/minimum_usdc"), "1.00");
    assert_eq!(s(&l, "/headline"), "Ladder ready: up to $765.86 USDC in 1 deposit, 0 NOCK.");
    let nock_at_bottom = f(&l, "/numbers/nock_at_bottom");
    assert!((nock_at_bottom - 67_800.0).abs() < 200.0, "{nock_at_bottom}");
    // The simulator, unsnapped at exactly $0.01 with this amount: the reserve holds.
    let at = reserve_at_setting(&env, "765.86", "0.01");
    assert_eq!(s(&at, "/reserve_check/status"), "holds", "{:#}", at["reserve_check"]);
    assert!(f(&at, "/reserve_check/stable_at_bottom_usdc") >= 950.0);
    assert_eq!(s(&at, "/reserve_check/max_usdc_for_this_range"), "765.86");
    // The old snapped amount would breach it there.
    let old = reserve_at_setting(&env, "766.32", "0.01");
    assert_eq!(s(&old, "/reserve_check/status"), "breached");
    // The picture's reserve label is now true at $0.01.
    assert_eq!(s(&l, "/picture/reserve_label"), "$950 kept for below $0.01");
}

#[test]
fn test_ladder_suggest_sim_fixture_matches_the_simulator() {
    let env = sim_fixture();
    let l = suggest(&env, Some(0.01));
    assert_eq!(s(&l, "/status"), "ready", "{l:#}");
    let r = &l["rungs"][0];
    // tests_ladder.rs pins 766.99 as the unsnapped ceiling at exactly $0.01.
    assert_eq!(s(r, "/usdc/amount"), "766.99");
    assert_eq!(s(r, "/top/pct_of_spot"), "-1.55");
    // The average buy price is the geometric mean of the bounds for a USDC-only range.
    let avg = f(r, "/at_bottom/avg_buy_price_usd");
    let gm = (1.0001f64.powi(-273900) * 1e10 * 1.0001f64.powi(-276300) * 1e10).sqrt();
    assert!((avg - gm).abs() < 1e-8, "{avg} vs {gm}");
    // The snapped simulator at the amount: the NOCK held at the bottom agrees.
    let req = LadderRequest::from_params(&params(&[("usdc", "766.99"), ("low", "0.01"), ("high", "spot"), ("position_id", V4_ID)])).unwrap();
    let b = simulate_ladder(&env, &req, env.meta.generated_at, "ok").unwrap();
    assert_eq!(s(&b, "/reserve_check/status"), "holds");
    assert!((f(&b, "/bottom/nock_held") - f(r, "/at_bottom/nock_held")).abs() < 1e-3);
    assert_eq!(s(&reserve_at_setting(&env, "766.99", "0.01"), "/reserve_check/status"), "holds");
}

#[test]
fn test_ladder_suggest_reserve_gate_reads_the_lower_of_snapped_and_setting() {
    // 0.00988 snaps UP to the same tick 276300 ($0.01002405); the gate is read at $0.00988.
    let env = live();
    let l = suggest(&env, Some(0.00988));
    assert_eq!(l["rungs"][0]["tick_upper"], json!(276300));
    assert_eq!(s(&l, "/gates/reserve/read_at_usd"), "0.00988000");
    let usdc = s(&l, "/rungs/0/usdc/amount").to_string();
    assert!(f(&l, "/rungs/0/usdc/amount") < 765.86);
    assert_eq!(s(&reserve_at_setting(&env, &usdc, "0.00988"), "/reserve_check/status"), "holds");
    assert_eq!(s(&l, "/picture/reserve_label"), "$950 kept for below $0.00988");
    // A bottom that snaps DOWN reads the gate at the snapped bottom, the lower one.
    let l = suggest(&env, Some(0.01012));
    let snapped = f(&l, "/rungs/0/bottom/price_usd");
    assert!(snapped < 0.01012, "{snapped}");
    assert_eq!(f(&l, "/gates/reserve/read_at_usd"), snapped);
}

#[test]
fn test_ladder_suggest_price_at_or_below_the_bottom_is_its_own_status() {
    let env = sim_fixture();
    for bottom in [0.012944, 0.013, 0.02] {
        let l = suggest(&env, Some(bottom));
        assert_eq!(s(&l, "/status"), "price_at_or_below_bottom", "{bottom}: {l:#}");
        assert_eq!(s(&l, "/headline"), format!("Ladder: NOCK is at or below your ${bottom} ladder bottom, so none is measured above it."));
        assert!(s(&l, "/reason").starts_with(&format!("NOCK is at or below your ${bottom} ladder bottom (NOCK $0.01294 in the USDC/NOCK 3% v4 pool)")), "{}", s(&l, "/reason"));
        assert!(!s(&l, "/reason").contains("invalid"));
    }
    // The pool's price moved below the operator's $0.01.
    let mut env = live();
    move_v4_price(&mut env, 0.0095);
    let l = suggest(&env, Some(0.01));
    assert_eq!(s(&l, "/status"), "price_at_or_below_bottom", "{l:#}");
    assert!(s(&l, "/reason").contains("(NOCK $0.009500 in the USDC/NOCK 3% v4 pool)"), "{}", s(&l, "/reason"));
    // Within one tick spacing below spot: an empty range, said so.
    let l = suggest(&sim_fixture(), Some(0.0128));
    assert_eq!(s(&l, "/status"), "bottom_at_or_above_top");
    assert_eq!(s(&l, "/headline"), "Ladder: NOCK is within one tick spacing of your bottom, so the range would be empty.");
}

#[test]
fn test_ladder_suggest_names_the_limit_that_binds() {
    // Reserve $100 and wallet USDC $500: the wallet binds, not the reserve.
    let mut env = live();
    env.data.derived_metrics.global_status.cash_reserve_usdc = Some("100.00".to_string());
    for w in env.data.wallet_balances.iter_mut().filter(|w| w.token_symbol == "USDC") {
        w.amount = "500.000000".to_string();
    }
    let l = suggest(&env, Some(0.01));
    assert_eq!(s(&l, "/status"), "ready");
    assert_eq!(s(&l, "/gates/reserve/binding"), "wallet");
    assert_eq!(s(&l, "/rungs/0/usdc/amount"), "500.00");
    let sentence = s(&l, "/sentence");
    assert!(sentence.contains("all the wallet's USDC (the $100.00 reserve would allow $"), "{sentence}");
    assert!(!sentence.contains("leaves the stable balance at your"), "{sentence}");
    assert!(s(&l, "/what_changed").contains("the wallet's $500.00 USDC fits"));
    assert!(text(&env, &l, Some("ok")).contains("(wallet USDC binds)"));
    // Wallet USDC $0.50 with the same reserve: no_headroom names the wallet.
    for w in env.data.wallet_balances.iter_mut().filter(|w| w.token_symbol == "USDC") {
        w.amount = "0.500000".to_string();
    }
    let l = suggest(&env, Some(0.01));
    assert_eq!(s(&l, "/status"), "no_headroom");
    assert!(s(&l, "/reason").starts_with("Wallet USDC is $0.50: below the $1.00 minimum (the reserve alone would leave $"), "{}", s(&l, "/reason"));
    // The live reserve binds and the text says so.
    let live = live();
    let l = suggest(&live, Some(0.01));
    assert!(s(&l, "/sentence").contains("the most that leaves the stable balance at your $950.00 reserve at your $0.01 bottom"));
    assert!(text(&live, &l, Some("ok")).contains("(the reserve binds)"));
}

#[test]
fn test_ladder_suggest_states_the_lines_that_are_not_gates() {
    let mut env = live();
    if let Some(p) = env.data.derived_metrics.accumulation_plan.as_mut() {
        p.status = "active".to_string();
        p.remaining_usdc = "900.00".to_string();
    }
    let l = suggest(&env, Some(0.01));
    let lines: Vec<&str> = l["stated"]["lines"].as_array().unwrap().iter().map(|x| x.as_str().unwrap()).collect();
    assert!(lines.iter().any(|x| x.starts_with("Plan budget not yet spent: $900.00")), "{lines:?}");
    assert!(lines.iter().any(|x| x.starts_with("Tax reserve: $8.70")), "{lines:?}");
    assert!(lines.iter().any(|x| x.starts_with("Cash target: $") && x.contains("% of the portfolio")), "{lines:?}");
    assert!(lines.iter().any(|x| x.starts_with("Below the bottom the other NOCK ranges keep converting")), "{lines:?}");
    assert!(l["stated"]["cash_target_usd"].is_string());
    let t = text(&env, &l, Some("ok"));
    assert!(t.contains("Stated, not gates: Plan budget not yet spent: $900.00"), "{t}");
    assert!(t.contains("Cash target: $"), "{t}");
    assert!(t.contains("Assumptions: The amount is the most"), "{t}");
}

#[test]
fn test_ladder_suggest_outcomes_in_plain_words() {
    let l = suggest(&live(), Some(0.01));
    let o = l["outcomes"].as_array().unwrap();
    let kinds: Vec<&str> = o.iter().map(|x| x["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["fall_inside", "fall_to_bottom", "rise"]);
    assert_eq!(
        s(&o[0], "/text"),
        "A fall to $0.01100 (-15%) would buy about 40,488 NOCK for $479.41 of the USDC, an average of $0.01184."
    );
    assert_eq!(
        s(&o[1], "/text"),
        "A fall to the ladder bottom $0.01002 (-23%) would buy about 67,763 NOCK with all $765.86, an average of $0.01130."
    );
    assert_eq!(s(&o[2], "/text"), "A rise sells nothing: the ladder holds 0 NOCK, so its USDC stays USDC.");
    // Exact, not from rounded amounts: the average is the geometric mean of the prices.
    let top = 1e10 / 1.0001f64.powi(273900);
    let p = f(&o[0], "/price_usd");
    assert!((f(&o[0], "/avg_buy_price_usd") - (top * p).sqrt()).abs() < 1e-8);
    assert!(f(&o[0], "/nock_bought") < f(&o[1], "/nock_bought"));
    // The picture's data.
    assert_eq!(s(&l, "/picture/bars/0/label"), "$765.86");
    assert_eq!(s(&l, "/picture/bottom_usd"), "0.01002405");
    assert!(f(&l, "/picture/spot_usd") > f(&l, "/picture/bars/0/high_usd"));
    // Plain numbers in the sentence: 4 significant digits, plain percents, whole NOCK.
    assert!(s(&l, "/sentence").contains("from $0.01274 (-1.6%) down to $0.01002 (-23%)"), "{}", s(&l, "/sentence"));
    assert!(s(&l, "/sentence").contains("about 67,763 NOCK, an average of $0.01130"), "{}", s(&l, "/sentence"));
}

#[test]
fn test_plain_number_formats() {
    assert_eq!(price4(0.0110), "0.01100");
    assert_eq!(price4(0.01002405), "0.01002");
    assert_eq!(price4(0.0129445), "0.01294");
    assert_eq!(price4(1.234567), "1.235");
    assert_eq!(pct_plain(-22.56), "-23%");
    assert_eq!(pct_plain(-1.553), "-1.6%");
    assert_eq!(pct_plain(-9.96), "-10%");
    assert_eq!(pct_plain(4.2), "+4.2%");
}

#[test]
fn test_ladder_suggest_deep_link_format() {
    let l = suggest(&live(), Some(0.01));
    let url = s(&l, "/rungs/0/deep_link/url");
    assert_eq!(
        url,
        "https://app.uniswap.org/positions/add/new?chain=base&protocolVersion=v4&currencyA=0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913&currencyB=0x9B5E262cF9bb04869ab40b19AF91D2dc85761722&fee=%7B%22feeAmount%22%3A30000%2C%22isDynamic%22%3Afalse%2C%22tickSpacing%22%3A300%7D&priceRangeState=%7B%22fullRange%22%3Afalse%2C%22maxTick%22%3A276300%2C%22minTick%22%3A273900%2C%22priceInverted%22%3Afalse%7D&step=1"
    );
    // Decoded, the fields round-trip. No amount: the app was observed on 2026-11-21 to
    // drop a depositState amount, so the amount is printed beside the link instead.
    let u = reqwest::Url::parse(url).unwrap();
    let q: HashMap<String, String> = u.query_pairs().map(|(k, v)| (k.into_owned(), v.into_owned())).collect();
    let fee: Value = serde_json::from_str(&q["fee"]).unwrap();
    assert_eq!(fee, json!({ "feeAmount": 30000, "tickSpacing": 300, "isDynamic": false }));
    let pr: Value = serde_json::from_str(&q["priceRangeState"]).unwrap();
    assert_eq!(pr, json!({ "priceInverted": false, "fullRange": false, "minTick": 273900, "maxTick": 276300 }));
    assert!(!q.contains_key("depositState") && !url.contains("765.86"), "{q:?}");
    assert!(!q.contains_key("hook"), "{q:?}");
    assert_eq!(u.host_str(), Some("app.uniswap.org"));
    assert!(!url.contains("aerodrome.finance"));
    assert_eq!(l["rungs"][0]["deep_link"]["prefilled"], json!(["pair", "fee and tick spacing", "range ticks"]));
    assert_eq!(l["rungs"][0]["deep_link"]["not_prefilled"], json!(["amount"]));
    assert_eq!(s(&l, "/rungs/0/deep_link/amount_to_enter"), "Amount to enter in the app: $765.86 USDC");
    assert_eq!(s(&l, "/rungs/0/deep_link/format_note"), LADDER_LINK_FORMAT_NOTE);
    // A hooked pool's address would be passed; with USDC as token1 the amount is TOKEN1.
    let mut p = sim_fixture().data.positions.into_iter().find(|p| p.position_id == V4_ID).unwrap();
    p.hooks = Some("0x00000000000000000000000000000000000000aa".to_string());
    let (hooked, pre) = new_position_url(&p, 273900, 276300, None).unwrap();
    assert!(hooked.contains("hook=0x00000000000000000000000000000000000000aa"));
    assert!(!hooked.contains("depositState") && !pre.contains(&"amount"));
    let mut flipped = p.clone();
    std::mem::swap(&mut flipped.token0, &mut flipped.token1);
    assert!(new_position_url(&flipped, 1, 2, Some("5.00")).unwrap().0.contains("%22exactField%22%3A%22TOKEN1%22"));
    // Aero CL: its deposit page, nothing prefilled, amount or not.
    p.protocol = "aerodrome-cl".to_string();
    p.pool_id = "0xpool".to_string();
    assert_eq!(new_position_url(&p, 1, 2, Some("5.00")).unwrap(), ("https://app.aero.xyz/deposit/8453-0xpool".to_string(), vec![]));
    p.protocol = "aerodrome".to_string();
    assert!(new_position_url(&p, 1, 2, Some("5.00")).unwrap_err().contains("full range"));
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
    assert_eq!(s(&l, "/headline"), "Ladder: needs your bottom price (operator_profile.ladder_bottom_usd) before one is measured.");
    let c = &l["candidates"];
    assert_eq!((s(c, "/to_pct"), s(c, "/step_pct")), ("-40", "5"));
    let rows = c["rows"].as_array().unwrap();
    assert!(rows.len() >= 6, "{c:#}");
    for r in rows {
        assert_eq!(r["tick"].as_i64().unwrap() % 300, 0, "snapped to the 300-tick grid: {r}");
        assert!(r.get("selected").is_none() && r.get("picked").is_none(), "no row is picked: {r}");
        assert!(r["usdc_ceiling"].is_string(), "the reserve is set, so each row has a ceiling: {r}");
    }
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
    }
    let t = text(&env, &l, Some("ok"));
    assert!(t.contains("Candidate bottoms (a measurement, none picked; grid to -40% in 5% steps)"), "{t}");
    // With the bottom set and only the reserve missing, no candidate bottoms.
    let l3 = suggest(&env2, Some(0.01));
    assert_eq!(l3["missing"], json!(["operator_profile.cash_reserve_usdc"]));
    assert!(l3["candidates"].is_null(), "{:#}", l3["candidates"]);
    assert_eq!(s(&l3, "/headline"), "Ladder: needs your protected reserve (operator_profile.cash_reserve_usdc) before one is measured.");
    assert!(!text(&env2, &l3, Some("ok")).contains("Candidate bottoms"));
}

#[test]
fn test_ladder_suggest_each_gate_states_its_reason() {
    // Bottom invalid (a number at or below 0; non-numbers stop the boot, see below).
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
    assert!(s(&l, "/reason").starts_with("At $0.01000 the stable balance would be $"), "{}", s(&l, "/reason"));
    assert!(s(&l, "/reason").contains("the $2000.00 reserve leaves $0.00 for a ladder, below the $1.00 minimum"), "{}", s(&l, "/reason"));
    // No pool: frozen, hooked, dynamic-fee or unread pool key; each reason is shown.
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
        let t = text(&env, &l, Some("ok"));
        assert!(t.contains(&format!("  - {V4_ID}: ")) && t.contains(needle), "{t}");
        assert!(!t.contains("see excluded"), "{t}");
    }
    // Two eligible pools: NockNock does not choose.
    let mut env = sim_fixture();
    let mut twin = env.data.positions.iter().find(|p| p.position_id == V4_ID).unwrap().clone();
    twin.position_id = "uniswap-v4:base:1".to_string();
    twin.pool_id = "0xother".to_string();
    twin.is_price_source = false;
    env.data.positions.push(twin);
    assert_eq!(s(&suggest(&env, Some(0.01)), "/status"), "several_pools");
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
    assert!(fresh.contains("Deposit 1 of 1: 766.99 USDC + 0.0000 NOCK as a new position in the USDC/NOCK 3% v4 pool"), "{fresh}");
    assert!(fresh.contains("Deposit link (pool and range prefilled): https://app.uniswap.org/positions/add/new?"), "{fresh}");
    assert!(fresh.contains("prefilled): https://app.uniswap.org/positions/add/new?") && fresh.contains("&step=1\n    Amount to enter in the app: $766.99 USDC\n"), "{fresh}");
    assert!(fresh.contains(LADDER_LINK_FORMAT_NOTE), "{fresh}");
    assert!(fresh.contains("After you deposit: After you deposit there is nothing to copy or configure."), "{fresh}");
    assert!(!fresh.contains("current fallback") && !fresh.contains("config.yaml"), "{fresh}");
    assert!(!fresh.contains("#3163450"), "the pool is named by itself, not by an NFT: {fresh}");
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
    near["usdc"]["amount"] = json!("115.05");
    near["bounds_usd"] = json!({ "low": "0.01100000", "high": "0.01274292" });
    deep["index"] = json!(2);
    deep["rung"] = json!("deep");
    deep["usdc"]["amount"] = json!("651.94");
    deep["bounds_usd"] = json!({ "low": "0.01002405", "high": "0.01100000" });
    near["of"] = json!(2);
    deep["of"] = json!(2);
    l["rungs"] = json!([near, deep]);
    let t = text(&sim_fixture(), &l, Some("ok"));
    let a = t.find("Deposit 1 of 2 (near rung): 115.05 USDC").expect(&t);
    let b = t.find("Deposit 2 of 2 (deep rung): 651.94 USDC").expect(&t);
    assert!(a < b);
    assert!(t.contains("($0.01002405 to $0.01100000;"), "{t}");
    assert_eq!(t.matches("Deposit link (pool and range prefilled)").count(), 2);
    assert_eq!(t.matches(LADDER_LINK_FORMAT_NOTE).count(), 1, "the format note once");
}

#[test]
fn test_ladder_suggest_after_deposit_is_tracked_automatically() {
    let l = suggest(&sim_fixture(), Some(0.01));
    let a = &l["after_deposit"];
    assert_eq!(s(a, "/kind"), "tracked_automatically");
    assert_eq!(s(a, "/text"), LADDER_AFTER_DEPOSIT_TEXT);
    // No config entry, no hash and no NFT id is asked for anywhere on the card.
    let whole = l.to_string();
    for banned in ["config.yaml", "deploy.sh", "config_snippets", "manual_config", "transaction hash", "nft_id: <"] {
        assert!(!whole.contains(banned), "{banned}: {whole}");
    }
    // The wording promises no number of minutes: it says what the worker does.
    let t = s(a, "/text");
    assert!(t.contains("at the start of every cycle") && t.contains("normally found in the first cycle after it lands"), "{t}");
    assert!(!t.chars().any(|c| c.is_ascii_digit()), "no promised duration: {t}");
    // With no detection block (a fixture) nothing is claimed about this cycle's check.
    assert!(a["detection_status"].is_null() && a["detection_note"].is_null());

    // A cycle whose check could not run says so beside it, with the worker's own reason.
    let mut env = sim_fixture();
    env.data.derived_metrics.position_detection = Some(json!({
        "status": "unavailable",
        "reason": "New positions could not be checked this cycle (transfer logs: the request timed out): a deposit made since the last check would not be counted yet.",
        "found_not_counted": { "count": 0, "positions": [] },
    }));
    let l = suggest(&env, Some(0.01));
    assert_eq!(s(&l, "/status"), "ready", "a failed check does not hide the card");
    assert_eq!(s(&l, "/after_deposit/detection_status"), "unavailable");
    let note = s(&l, "/after_deposit/detection_note");
    assert!(note.starts_with("This cycle's check did not run: New positions could not be checked this cycle") && note.ends_with("A deposit made now is found once the check runs again."), "{note}");
    let t = text(&env, &l, Some("ok"));
    assert!(t.contains("until one tap says how to track it. This cycle's check did not run:"), "{t}");
}

#[test]
fn test_an_emptied_ladder_is_not_a_deployed_rung() {
    // A tracked ladder that holds something: no new one is suggested, as on main.
    let mut env = sim_fixture();
    env.data.positions[0].role = Some(LADDER_ROLE.to_string());
    env.data.positions[0].source = Some(POSITION_SOURCE_TRACKED.to_string());
    let l = suggest(&env, Some(0.01));
    assert_eq!(s(&l, "/status"), "ladder_tracked");
    assert_eq!(s(&l, "/tracked/0/source"), "tracked");
    assert!(s(&l, "/reason").contains("uniswap-v4 #3163450, found on-chain"), "{}", s(&l, "/reason"));

    // The same ladder emptied (NFT still in the wallet): listed, and a ladder is measured.
    let empty = LadderReading { position_id: V4_ID.to_string(), label: "v4 USDC/NOCK 3.00% #3163450".to_string(), state: "empty".to_string(), ..Default::default() };
    env.data.ladders = vec![empty];
    let l = suggest(&env, Some(0.01));
    assert_ne!(s(&l, "/status"), "ladder_tracked", "an emptied rung must not read as deployed for ever");
    assert_eq!(s(&l, "/tracked/0/state"), "empty");
    assert_eq!(s(&l, "/gates/ladder_tracked/reason"), "every tracked ladder is empty (#167)");
    assert!(s(&l, "/sentence").starts_with("No tracked ladder holds anything (v4 USDC/NOCK 3.00% #3163450 is empty: emptied, its NFT still in the wallet, so not a deployed rung). A new USDC-only position"), "{}", s(&l, "/sentence"));
    assert!(s(&l, "/what_changed").contains("so not a deployed rung). Now the"), "{}", s(&l, "/what_changed"));
}

#[test]
fn test_a_found_position_waiting_for_a_tap_stops_a_new_suggestion() {
    let mut env = sim_fixture();
    let found = |choices: Value| {
        json!({
            "status": "ok",
            "found_not_counted": { "count": 1, "value_usd": "115.03", "positions": [{
                "nft_id": 3243807, "pool_label": "v4 USDC/NOCK 3.00%", "value_usd": "115.03", "choices": choices,
            }] },
        })
    };
    env.data.derived_metrics.position_detection = Some(found(json!(["ladder:near", "ladder:deep", "lp", "dismiss"])));
    let l = suggest(&env, Some(0.01));
    assert_eq!(s(&l, "/status"), "found_untracked");
    assert_eq!(l["shown"], json!(false));
    assert!(l["rungs"].as_array().unwrap().is_empty(), "no deposit is offered beside an uncounted one");
    assert_eq!(s(&l, "/headline"), "Ladder: a position found on-chain is waiting for one tap.");
    assert_eq!(
        s(&l, "/reason"),
        "A USDC/NOCK position the wallet holds was found on-chain and is not tracked or counted yet (v4 USDC/NOCK 3.00% #3243807, worth $115.03). No new ladder is measured until it is tracked or set aside (one tap, under Found on-chain)."
    );
    assert_eq!(l["found_untracked"][0]["nft_id"], json!(3243807));
    // A found position that cannot be a ladder (a WETH/USDC range) does not stop the card.
    env.data.derived_metrics.position_detection = Some(found(json!(["lp", "dismiss"])));
    assert_eq!(s(&suggest(&env, Some(0.01)), "/status"), "ready");
}

/// Every status the card can take, for the wording checks.
fn every_status() -> Vec<(SnapshotEnvelope, Value)> {
    let mut out: Vec<(SnapshotEnvelope, Value)> = Vec::new();
    let base = sim_fixture();
    for b in [Some(0.01), None, Some(0.0128), Some(0.02)] {
        out.push((base.clone(), suggest(&base, b)));
    }
    let bad = OperatorProfile { ladder_bottom_usd: Some(-1.0), ..OperatorProfile::default() };
    out.push((base.clone(), ladder_suggestion(&base, ladder_bottom_setting(&bad))));
    let mut tracked = sim_fixture();
    tracked.data.positions[0].role = Some(LADDER_ROLE.to_string());
    out.push((tracked.clone(), suggest(&tracked, Some(0.01))));
    let mut frozen = sim_fixture();
    frozen.data.positions.iter_mut().filter(|p| p.position_id == V4_ID).for_each(|p| p.pool_active = Some(false));
    out.push((frozen.clone(), suggest(&frozen, Some(0.01))));
    let mut rich = sim_fixture();
    rich.data.derived_metrics.global_status.cash_reserve_usdc = Some("2000.00".to_string());
    out.push((rich.clone(), suggest(&rich, Some(0.01))));
    let mut twin_env = sim_fixture();
    let mut twin = twin_env.data.positions.iter().find(|p| p.position_id == V4_ID).unwrap().clone();
    twin.position_id = "uniswap-v4:base:1".to_string();
    twin.pool_id = "0xother".to_string();
    twin_env.data.positions.push(twin);
    out.push((twin_env.clone(), suggest(&twin_env, Some(0.01))));
    let mut noreserve = sim_fixture();
    noreserve.data.derived_metrics.global_status.cash_reserve_usdc = None;
    out.push((noreserve.clone(), suggest(&noreserve, Some(0.01))));
    out
}

#[test]
fn test_ladder_suggest_wording_is_a_measurement() {
    let statuses: std::collections::BTreeSet<String> = every_status().iter().map(|(_, l)| s(l, "/status").to_string()).collect();
    for want in ["ready", "needs_setting", "bottom_at_or_above_top", "price_at_or_below_bottom", "invalid", "ladder_tracked", "no_target_pool", "no_headroom", "several_pools"] {
        assert!(statuses.contains(want), "{want} not covered: {statuses:?}");
    }
    let banned = ["should", "recommend", "you must", "you need to", "buy now", "deposit $", "please", "we suggest", "make sure"];
    // An imperative opens a sentence: "Set your …", "Deposit 765 USDC …". "Deposit 1 of 2"
    // is a list label, and NockNock's own steps ("NockNock signs …") are statements.
    let imperatives = ["set", "add", "put", "buy", "sell", "deposit", "move", "withdraw", "open", "go", "use", "type", "enter", "click", "tap", "edit", "run", "consider"];
    for (env, l) in every_status() {
        // The fields a person reads (status words like "set" are data, not prose).
        let mut prose: Vec<String> = ["/headline", "/reason", "/sentence", "/what_changed", "/statement", "/after_deposit/text"]
            .iter()
            .filter_map(|p| l.pointer(p).and_then(|v| v.as_str()).map(|v| v.to_string()))
            .collect();
        for arr in ["/outcomes", "/stated/lines", "/assumptions", "/rungs", "/excluded"] {
            for x in l.pointer(arr).and_then(|v| v.as_array()).into_iter().flatten() {
                for p in ["/text", "/would_sign", "/reason", ""] {
                    if let Some(v) = (if p.is_empty() { Some(x) } else { x.pointer(p) }).and_then(|v| v.as_str()) {
                        prose.push(v.to_string());
                    }
                }
            }
        }
        for t in [prose.join("\n"), text(&env, &l, Some("ok"))] {
            let low = t.to_lowercase();
            for bad in banned {
                assert!(!low.contains(bad), "'{bad}' in {} text: {t}", s(&l, "/status"));
            }
            for sentence in low.split(|c| c == '.' || c == '\n' || c == ':' || c == '"').map(|x| x.trim_start_matches(|c: char| c == ' ' || c == '-' || c == '•')) {
                let mut words = sentence.split_whitespace();
                let (Some(first), second) = (words.next(), words.next()) else { continue };
                let list_label = first == "deposit" && second.map_or(false, |w| w == "link" || w.chars().all(|c| c.is_ascii_digit()));
                assert!(list_label || !imperatives.contains(&first), "imperative '{sentence}' in {} text", s(&l, "/status"));
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
    let ingest = include_str!("ingest.rs");
    let capture = ingest.find("let next_directive_details").unwrap();
    let attach = ingest.find("attach_ladder_suggestion(&mut snapshot_data, &profile").unwrap();
    assert!(capture < attach);
}

#[test]
fn test_ladder_bottom_usd_through_the_real_config_loader() {
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/config.yaml.example");
    let dir = std::env::temp_dir().join(format!("nocknock-ladder-bottom-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let load = |yaml: &str| {
        let path = dir.join("config.yaml");
        std::fs::write(&path, format!("operator_profile:\n  treasury_floor_nock: 1\n{yaml}")).unwrap();
        load_app_config(example, Some(path.to_str().unwrap()), false)
    };
    // The example leaves it unset: no default.
    let cfg = load_app_config(example, Some("/nonexistent/config.yaml"), false).unwrap();
    assert_eq!(cfg.profile().ladder_bottom_usd, None);
    assert_eq!(load("").unwrap().profile().ladder_bottom_usd, None);
    assert_eq!(load("  ladder_bottom_usd: 0.01\n").unwrap().profile().ladder_bottom_usd, Some(0.01));
    // A number at or below 0 loads and is reported as invalid on the card.
    for v in ["0", "-0.01"] {
        let p = load(&format!("  ladder_bottom_usd: {v}\n")).unwrap().profile();
        assert_eq!(ladder_bottom_setting(&p), Err(LADDER_BOTTOM_INVALID_NOTE), "{v}");
    }
    // A value that is not a number stops the boot with a config error, like every
    // numeric key (documented in config.yaml.example).
    for v in ["\"$0.01\"", "abc", "\"0,01\"", ".nan", ".inf"] {
        let e = load(&format!("  ladder_bottom_usd: {v}\n"));
        assert!(e.is_err(), "{v} should not load");
    }
    std::fs::remove_dir_all(&dir).ok();
    let doc = std::fs::read_to_string(example).unwrap();
    assert!(doc.contains("stops the worker at startup with a config error"));
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
    let live = live();
    let mut tracked = sim_fixture();
    tracked.data.positions[0].role = Some(LADDER_ROLE.to_string());
    let mut frozen = sim_fixture();
    frozen.data.positions.iter_mut().filter(|p| p.position_id == V4_ID).for_each(|p| p.pool_active = Some(false));
    let cases = [
        ("ready", &env, suggest(&env, Some(0.01)), Some("ok")),
        ("ready_live", &live, suggest(&live, Some(0.01)), Some("ok")),
        ("ready_stale", &env, suggest(&env, Some(0.01)), Some("degraded")),
        ("needs_setting", &env, suggest(&env, None), Some("ok")),
        ("bottom_at_or_above_top", &env, suggest(&env, Some(0.0128)), Some("ok")),
        ("price_at_or_below_bottom", &env, suggest(&env, Some(0.02)), Some("ok")),
        ("no_target_pool", &frozen, suggest(&frozen, Some(0.01)), Some("ok")),
        ("tracked", &tracked, suggest(&tracked, Some(0.01)), Some("ok")),
    ];
    let mut doc = serde_json::Map::new();
    for (name, e, l, health) in cases {
        doc.insert(name.to_string(), json!({ "health": health, "ladder_suggestion": l, "text": text(e, &l, health) }));
    }
    let actual = format!("{}\n", serde_json::to_string_pretty(&Value::Object(doc)).unwrap());
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/ladder_suggest/golden_ladder_suggest.json");
    if std::env::var("NOCKNOCK_BLESS_GOLDEN").is_ok() {
        std::fs::write(path, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(path).expect("testdata/ladder_suggest/golden_ladder_suggest.json is checked in");
    assert!(actual == expected, "ladder suggestion text changed; re-bless and review {path}");
}
