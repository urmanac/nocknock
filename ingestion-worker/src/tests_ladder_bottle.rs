//! The ladder suggestion built from the operator's shape (`operator_profile.ladder_rungs`):
//! the "bottle" of three deposits, cap | near | deep.
//!
//! The fixture is the #166 one with the recorded fee measurement (tests_ladder_rungs.rs:
//! spot $0.012944, wallet USDC 1545.8357 and NOCK 90086.7891, an LP NOCK budget of 86.7891,
//! a $950 reserve, the bottle near 15% down to $0.0110 and deep 85% down to $0.01). The
//! pinned mints (liquidity and raw amounts) are the ones tests_ladder_rungs.rs pins for
//! `derived_metrics.ladder_shape`, recomputed there by scripts that share no code with
//! the worker. The shape, the bottom and the reserve are fixture inputs, never presets.
use super::*;
use crate::tests_fee_yield::{fee_fixture, V4_ID};
use crate::tests_ladder::{fixture, fixture_at_sqrt, fixture_with_rungs, sqrt_x96_of_raw};
use crate::tests_ladder_suggest::{assert_measurement_wording, text, with_pool_key};
use serde_json::Value;

pub(crate) fn now() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-11-20T12:05:00Z").unwrap().with_timezone(&Utc)
}

pub(crate) fn st<'a>(v: &'a Value, ptr: &str) -> &'a str {
    v.pointer(ptr).and_then(|x| x.as_str()).unwrap_or_else(|| panic!("{ptr} missing in {v:#}"))
}

pub(crate) fn f(v: &Value, ptr: &str) -> f64 {
    st(v, ptr).parse().unwrap()
}

pub(crate) fn bottle() -> Vec<LadderRungConfig> {
    vec![
        LadderRungConfig { rung: "cap".into(), share: None, low_usd: None },
        LadderRungConfig { rung: "near".into(), share: Some(0.15), low_usd: Some(0.0110) },
        LadderRungConfig { rung: "deep".into(), share: Some(0.85), low_usd: Some(0.01) },
    ]
}

/// The operator's profile for these fixtures, as config.yaml would state it.
pub(crate) fn profile() -> OperatorProfile {
    OperatorProfile { ladder_bottom_usd: Some(0.01), cash_reserve_usdc: Some(950.0), ladder_rungs: Some(bottle()), ..OperatorProfile::default() }
}

/// An envelope as a cycle leaves it before the suggestion is attached: the operator's
/// settings in global_status, the v4 pool key, the recorded fee measurement, the tracked
/// ladders' readings and `derived_metrics.ladder_shape`.
pub(crate) fn prepared(env: SnapshotEnvelope) -> SnapshotEnvelope {
    assert_eq!(validate_ladder_rungs(&bottle(), Some(0.01)), Ok(()));
    let mut env = with_pool_key(env);
    let gs = &mut env.data.derived_metrics.global_status;
    gs.ladder_bottom_usd = Some("0.01000000".into());
    gs.ladder_rungs = Some(ladder_rung_settings(&bottle()));
    env.data.derived_metrics.fee_yield = fee_fixture().data.derived_metrics.fee_yield;
    env.data.ladders = ladder_readings(&stored_book(&env.data).raw);
    env.data.derived_metrics.ladder_shape = ladder_shape_block(&env.data, now());
    env
}

pub(crate) fn card(env: &SnapshotEnvelope) -> Value {
    ladder_suggestion(env, Ok(Some(0.01)))
}

/// The bottle with nothing deposited.
pub(crate) fn fresh() -> SnapshotEnvelope {
    prepared(fixture())
}

/// The liquidity of each rung of the fresh bottle: what a deposit "as stated" mints.
pub(crate) fn stated_liquidity() -> (u128, u128, u128) {
    let l = card(&fresh());
    let liq = |i: usize| -> u128 { st(&l["rungs"][i], "/mint/liquidity_raw").parse().unwrap() };
    (liq(0), liq(1), liq(2))
}

pub(crate) const CAP: (u64, Option<&str>, i32, i32) = (9000002, Some("cap"), 273600, 273900);
pub(crate) const NEAR: (u64, Option<&str>, i32, i32) = (9000003, Some("near"), 273900, 275400);
pub(crate) const DEEP: (u64, Option<&str>, i32, i32) = (9000001, Some("deep"), 275400, 276300);

pub(crate) fn deposited(rungs: &[((u64, Option<&'static str>, i32, i32), u128)]) -> SnapshotEnvelope {
    let rows: Vec<(u64, Option<&str>, i32, i32, u128)> = rungs.iter().map(|((id, role, tl, tu), l)| (*id, *role, *tl, *tu, *l)).collect();
    prepared(fixture_with_rungs(&rows))
}

pub(crate) fn deep_deployed() -> SnapshotEnvelope {
    deposited(&[(DEEP, stated_liquidity().2)])
}

pub(crate) fn cap_and_near_deployed() -> SnapshotEnvelope {
    let (c, n, _) = stated_liquidity();
    deposited(&[(CAP, c), (NEAR, n)])
}

pub(crate) fn all_deployed() -> SnapshotEnvelope {
    let (c, n, d) = stated_liquidity();
    deposited(&[(CAP, c), (NEAR, n), (DEEP, d)])
}

/// The pool half a tick into the band 275100..275400: below the near rung's range.
pub(crate) fn near_passed() -> SnapshotEnvelope {
    prepared(fixture_at_sqrt(sqrt_x96_of_raw(1.0001f64.powf(275100.5))))
}

/// The pool at the ladder's bottom tick: every rung is passed.
pub(crate) fn all_passed() -> SnapshotEnvelope {
    prepared(fixture_at_sqrt(sqrt_x96_of_raw(1.0001f64.powf(276300.0))))
}

pub(crate) fn deposit<'a>(l: &'a Value, role: &str) -> &'a Value {
    l["rungs"].as_array().unwrap().iter().find(|r| r["rung"] == role).unwrap_or_else(|| panic!("no {role} deposit in {:#}", l["rungs"]))
}

pub(crate) fn row<'a>(l: &'a Value, role: &str) -> &'a Value {
    l["shape"]["rungs"].as_array().unwrap().iter().find(|r| r["rung"] == role).unwrap_or_else(|| panic!("no {role} row in {:#}", l["shape"]))
}

// ------------------------------------------------------------------
// The bottle: three numbered deposits
// ------------------------------------------------------------------

#[test]
fn test_the_bottle_is_three_numbered_deposits_with_the_pinned_mints() {
    let env = fresh();
    let l = card(&env);
    assert_eq!((st(&l, "/status"), &l["shown"]), ("ready", &json!(true)), "{l:#}");
    // The headline adds what the deposits state, cut to cents: $1.22 + $114.86 + $650.89.
    assert_eq!(st(&l, "/headline"), "Ladder: 3 of 3 deposits left, up to $766.97 USDC and about 87 NOCK.");
    assert_eq!((l["shape"]["of"].as_u64(), l["shape"]["left"].as_u64()), (Some(3), Some(3)));
    let rungs = l["rungs"].as_array().unwrap();
    assert_eq!(rungs.len(), 3);
    // (role, ticks, liquidity, raw USDC, raw NOCK): the mints tests_ladder_rungs.rs pins.
    let pinned = [
        ("cap", 273600, 273900, "138192109253769", "1225999", "867890958577132038"),
        ("near", 273900, 275400, "1408301615002237", "114864600", "0"),
        ("deep", 275400, 276300, "14125167068081104", "650899400", "0"),
    ];
    for (i, (role, tl, tu, liquidity, usdc_raw, nock_raw)) in pinned.iter().enumerate() {
        let r = &rungs[i];
        assert_eq!((st(r, "/rung"), r["index"].as_u64(), r["of"].as_u64()), (*role, Some(i as u64 + 1), Some(3)));
        assert_eq!((r["tick_lower"].as_i64(), r["tick_upper"].as_i64()), (Some(*tl), Some(*tu)), "{role}");
        assert_eq!(st(r, "/mint/liquidity_raw"), *liquidity, "{role}");
        assert_eq!((st(r, "/usdc/raw"), st(r, "/nock/raw")), (*usdc_raw, *nock_raw), "{role}");
        // USDC is token0 and NOCK token1 in this pool: the mint pulls exactly these.
        assert_eq!((st(r, "/mint/amount0_raw"), st(r, "/mint/amount1_raw")), (*usdc_raw, *nock_raw), "{role}");
        assert_eq!(st(r, "/status"), "to_deposit");
        assert_eq!((st(r, "/pool/position_id"), st(r, "/pool/label")), (V4_ID, "the USDC/NOCK 3% v4 pool"));
        assert_eq!((r["pool"]["fee_raw"].as_i64(), r["pool"]["tick_spacing"].as_i64()), (Some(30000), Some(300)));
    }
    assert!(st(&l, "/mint_note").contains("rounded down (LiquidityAmounts)"));
    // The amounts: exact, and cut (never rounded up) to what a person types.
    let (cap, near, deep) = (&rungs[0], &rungs[1], &rungs[2]);
    assert_eq!((st(cap, "/usdc/amount"), st(cap, "/nock/amount")), ("1.225999", "86.7890958577132038"));
    assert_eq!((st(near, "/usdc/amount"), st(near, "/usdc/to_enter")), ("114.864600", "114.86"));
    assert_eq!((st(deep, "/usdc/amount"), st(deep, "/usdc/to_enter")), ("650.899400", "650.89"));
    assert_eq!(st(cap, "/nock/to_enter"), "86.7890");
    // The bounds in USD per NOCK and, from the ticks, in NOCK per USDC.
    assert_eq!((st(cap, "/bounds_usd/low"), st(cap, "/bounds_usd/high")), ("0.01274292", "0.01313098"));
    assert_eq!((st(near, "/bounds_usd/low"), st(near, "/bounds_usd/high")), ("0.01096801", "0.01274292"));
    assert_eq!((st(deep, "/bounds_usd/low"), st(deep, "/bounds_usd/high")), ("0.01002405", "0.01096801"));
    assert_eq!((st(cap, "/bounds_nock_per_usdc/low"), st(cap, "/bounds_nock_per_usdc/high")), ("76.1558", "78.4750"));
    assert_eq!((st(near, "/bounds_nock_per_usdc/low"), st(near, "/bounds_nock_per_usdc/high")), ("78.4750", "91.1742"));
    assert_eq!((st(deep, "/bounds_nock_per_usdc/low"), st(deep, "/bounds_nock_per_usdc/high")), ("91.1742", "99.7600"));
    for r in rungs {
        let lo = 1e10 / 1.0001f64.powi(r["tick_upper"].as_i64().unwrap() as i32);
        assert!((f(r, "/bounds_usd/low") - lo).abs() < 1e-8, "the bound is the tick's own price");
        assert!((f(r, "/bounds_nock_per_usdc/high") - 1.0 / lo).abs() < 1e-4);
    }
    // The summaries: the cap in range with both tokens, the others USDC only.
    assert_eq!(st(cap, "/summary"), "about 86.79 NOCK + $1.22 USDC, in range now");
    assert_eq!(st(near, "/summary"), "$114.86 USDC, no NOCK, below the price now");
    assert_eq!(st(deep, "/summary"), "$650.89 USDC, no NOCK, below the price now");
    assert_eq!((&cap["in_range_now"], &near["in_range_now"]), (&json!(true), &json!(false)));
    // What is stated adds up to the headline, and never to more than the gate allows.
    let typed: f64 = f(near, "/usdc/to_enter") + f(deep, "/usdc/to_enter") + f(cap, "/usdc/to_enter");
    assert!((typed - f(&l, "/numbers/usdc_max")).abs() < 1e-9, "{typed}");
    assert!(f(near, "/usdc/amount") + f(deep, "/usdc/amount") + f(cap, "/usdc/amount") <= f(&l, "/numbers/headroom_usdc") + 1e-9);
    assert_eq!((st(&l, "/numbers/usdc_max"), st(&l, "/numbers/nock_now"), st(&l, "/numbers/headroom_usdc")), ("766.97", "86.7891", "766.99"));
    assert_eq!((st(&l, "/gates/reserve/status"), st(&l, "/gates/reserve/read_at_usd"), st(&l, "/gates/nock_budget/status")), ("allows", "0.01000000", "fits"));
    // The text both transports print.
    let t = text(&env, &l, Some("ok"));
    assert!(t.contains("  Deposit 1 of 3 (cap): about 86.79 NOCK + $1.22 USDC, in range now. A new position in the USDC/NOCK 3% v4 pool, ticks 273600 to 273900 ($0.01274292 to $0.01313098; 76.1558 to 78.4750 NOCK per USDC).\n"), "{t}");
    assert!(t.contains("    Exact mint: liquidity 138192109253769, 1225999 raw USDC and 867890958577132038 raw NOCK.\n"), "{t}");
    assert!(t.contains("  Deposit 2 of 3 (near rung): $114.86 USDC, no NOCK, below the price now. A new position in the USDC/NOCK 3% v4 pool, ticks 273900 to 275400 ($0.01096801 to $0.01274292; 78.4750 to 91.1742 NOCK per USDC).\n"), "{t}");
    assert!(t.contains("  Deposit 3 of 3 (deep rung): $650.89 USDC, no NOCK, below the price now. A new position in the USDC/NOCK 3% v4 pool, ticks 275400 to 276300 ($0.01002405 to $0.01096801; 91.1742 to 99.7600 NOCK per USDC).\n"), "{t}");
    assert_eq!(t.matches(LADDER_LINK_FORMAT_NOTE).count(), 1, "the format note once");
}

#[test]
fn test_each_deposit_has_its_own_link_with_pool_and_range_and_never_an_amount() {
    let l = card(&fresh());
    for (r, (min, max)) in l["rungs"].as_array().unwrap().iter().zip([(273600, 273900), (273900, 275400), (275400, 276300)]) {
        let url = st(r, "/deep_link/url");
        let u = reqwest::Url::parse(url).unwrap();
        assert_eq!(u.host_str(), Some("app.uniswap.org"));
        let q: HashMap<String, String> = u.query_pairs().map(|(k, v)| (k.into_owned(), v.into_owned())).collect();
        let pr: Value = serde_json::from_str(&q["priceRangeState"]).unwrap();
        assert_eq!(pr, json!({ "priceInverted": false, "fullRange": false, "minTick": min, "maxTick": max }));
        assert_eq!(serde_json::from_str::<Value>(&q["fee"]).unwrap(), json!({ "feeAmount": 30000, "tickSpacing": 300, "isDynamic": false }));
        // No amount in any link: the app was observed to drop one, so it is printed.
        assert!(!q.contains_key("depositState"), "{q:?}");
        assert_eq!(r["deep_link"]["prefilled"], json!(["pair", "fee and tick spacing", "range ticks"]));
        assert_eq!(r["deep_link"]["not_prefilled"], json!(["amount"]));
        assert!(st(r, "/deep_link/amount_to_enter").starts_with("Amount to enter in the app: "));
    }
    assert_eq!(st(deposit(&l, "near"), "/deep_link/amount_to_enter"), "Amount to enter in the app: $114.86 USDC");
    assert_eq!(st(deposit(&l, "deep"), "/deep_link/amount_to_enter"), "Amount to enter in the app: $650.89 USDC");
    // After a deposit nothing is typed: the worker finds the new position on-chain and
    // matches it to its rung by pool and ticks (position_detect.rs), so no config snippet.
    assert_eq!(l["after_deposit"]["kind"], "tracked_automatically");
    assert!(l["after_deposit"].get("config_snippets").is_none());
}

#[test]
fn test_the_cap_states_both_amounts_the_nock_limit_and_the_field_that_keeps_it() {
    let env = fresh();
    let l = card(&env);
    let cap = deposit(&l, "cap");
    // The app derives the other side from the field typed: typing USDC lets the NOCK side
    // exceed the budget (critique of the deploy design), so the NOCK field is the one named.
    assert_eq!(
        st(cap, "/deep_link/amount_to_enter"),
        "Amount to enter in the app: 86.7890 NOCK, in the NOCK field; the app then works out the USDC side, about $1.22 at this reading."
    );
    // Both limits, the one that binds now, and the field that keeps it (review of the
    // bottle, findings 6 and 15): typing NOCK leaves the USDC side to the price.
    assert_eq!(
        st(cap, "/nock_limit/text"),
        "The NOCK side must not exceed 86.7891 NOCK, your LP NOCK budget, which binds now; the USDC side must not exceed $100.04, the most that keeps it at or below the near rung's USDC. The limits hold when the NOCK field is the one typed, at 86.7890 NOCK or less, and the app shows at most $100.04 of USDC: about $1.22 at this reading, more if the price rises inside the band before you deposit. When the USDC field is typed instead, the app works out the NOCK side from the price at that moment, and it can come out above the limit; neither the link nor the app applies a NOCK limit of its own."
    );
    assert_eq!((st(cap, "/nock_limit/nock"), st(cap, "/nock_limit/is"), st(cap, "/nock_limit/field_to_type")), ("86.7891", "lp_nock_budget", "NOCK"));
    // What is typed is at most the exact mint, which is at most the budget.
    let (typed, exact, budget) = (f(cap, "/nock_limit/nock_to_enter"), f(cap, "/nock/amount"), f(cap, "/nock_limit/nock"));
    assert!(typed <= exact && exact <= budget, "{typed} {exact} {budget}");
    assert_eq!(st(&l, "/numbers/lp_nock_budget"), "86.7891");
    assert_eq!(st(cap, "/cap/bound_by"), "lp_nock_budget");
    assert!(st(cap, "/would_sign").contains("holding up to 86.7890 NOCK and about $1.22 USDC"));
    // A USDC-only rung has no NOCK limit to state.
    for role in ["near", "deep"] {
        assert!(deposit(&l, role)["nock_limit"].is_null(), "{role}");
        assert!(deposit(&l, role)["cap"].is_null(), "{role}");
    }
    let t = text(&env, &l, Some("ok"));
    assert!(t.contains("    The NOCK side must not exceed 86.7891 NOCK, your LP NOCK budget, which binds now;"), "{t}");
    assert_eq!(t.matches("must not exceed").count(), 2, "both sides of the one deposit that holds NOCK");
    assert_eq!(cut_dp("86.7890958577132038", 4), "86.7890");
    assert_eq!(cut_dp("650.899400", 2), "650.89");
    assert_eq!(cut_dp("7", 2), "7.00");
    assert_eq!(cut_dp("7.5", 0), "7");
}

#[test]
fn test_the_card_states_what_the_deposits_leave_in_the_wallet() {
    let env = fresh();
    let l = card(&env);
    let w = &l["wallet_after"];
    assert_eq!(
        st(w, "/text"),
        "After these deposits your wallet holds about $779 USDC; the $950 reserve counts the $224 of USDC inside your existing NOCK ranges. At $0.01 with every rung converted, the stable balance reads $950.02: the wallet's $778.87 plus $171.15 still inside those ranges."
    );
    // Wallet 1545.8357 less the three deposits as the card states them.
    let deposits: f64 = l["rungs"].as_array().unwrap().iter().map(|r| f(r, "/usdc/to_enter")).sum();
    assert!((f(w, "/wallet_usdc_after") - (1545.8357 - deposits)).abs() < 0.006, "{w:#}");
    assert_eq!((st(w, "/wallet_usdc_now"), st(w, "/deposits_usdc"), st(w, "/wallet_usdc_after")), ("1545.84", "766.97", "778.87"));
    // The two parts add up to the total as printed; the shape's own gate reads the exact
    // mints, which take the two cents the cut amounts leave in the wallet.
    assert!((f(w, "/wallet_usdc_after") + f(w, "/existing_ranges_usdc_at_gate") - f(w, "/stable_at_gate_usdc")).abs() < 1e-9);
    let shape = env.data.derived_metrics.ladder_shape.as_ref().unwrap();
    assert_eq!(st(shape, "/gate/stable_at_gate_usdc"), "950.00");
    assert_eq!((st(w, "/gate_price_usd"), st(w, "/cash_reserve_usdc")), ("0.01000000", "950.00"));
    // The USDC inside the existing NOCK ranges now is the snapshot's own reserve reading
    // less the wallet's USDC.
    let gs = &env.data.derived_metrics.global_status;
    let stable_now: f64 = gs.reserve_stable_usdc.as_deref().unwrap().parse().unwrap();
    assert!((f(w, "/existing_ranges_usdc_now") - (stable_now - 1545.8357)).abs() < 0.011, "{} vs {stable_now}", st(w, "/existing_ranges_usdc_now"));
    // The wallet alone ends below the reserve: stated, not hidden.
    assert_eq!(w["wallet_alone_below_reserve"], json!(true));
    assert_eq!(st(&l, "/numbers/wallet_usdc_after"), "778.87");
    let t = text(&env, &l, Some("ok"));
    assert!(t.contains(&format!("  {}\n", st(w, "/text"))), "{t}");
}

#[test]
fn test_the_bottle_outcomes_in_plain_words() {
    let env = fresh();
    let l = card(&env);
    let o = l["outcomes"].as_array().unwrap();
    let kinds: Vec<&str> = o.iter().map(|x| x["kind"].as_str().unwrap()).collect();
    assert_eq!(kinds, ["fall_inside", "fall_to_bottom", "rise"]);
    assert_eq!(st(&o[0], "/text"), "A fall to $0.01097 (-15%), the near rung's bottom, would buy about 9,811 NOCK for $116.09 of the USDC, an average of $0.01183.");
    assert_eq!(st(&o[1], "/text"), "A fall to the ladder bottom $0.01002 (-23%) would buy about 71,888 NOCK in total with all $766.97, an average of $0.01067.");
    assert_eq!(
        st(&o[2], "/text"),
        "A rise to $0.01313 (+1.4%) or above sells the cap's NOCK, about 87, for about $1.13 of USDC, and sells nothing else: every other rung holds 0 NOCK."
    );
    // Against the shape's own figures (the simulator, tests_ladder_rungs.rs): at the near
    // rung's bottom the cap and the near rung are fully NOCK and the deep rung untouched.
    let shape = env.data.derived_metrics.ladder_shape.as_ref().unwrap();
    let bought = |i: usize| f(&shape["rungs"][i], "/nock_bought_at_bottom");
    assert!((f(&o[0], "/nock_bought") - (bought(0) + bought(1))).abs() < 1.0, "{}", o[0]);
    assert!((f(&o[0], "/usdc_spent") - (1.225999 + 114.8646)).abs() < 0.006);
    assert_eq!(st(shape, "/combined/nock_bought_at_deepest_bottom"), "71888.1054");
    assert_eq!((st(&o[1], "/nock_bought"), st(&o[1], "/avg_buy_price_usd")), ("71888", "0.01066922"));
    // The rise: the cap's NOCK sold between the price now and its top, at their geometric mean.
    let (p0, top) = (0.012944f64, 1e10 / 1.0001f64.powi(273600));
    assert!((f(&o[2], "/usdc_received") - 86.7891 * (p0 * top).sqrt()).abs() < 0.006, "{}", o[2]);
    assert_eq!(st(&o[2], "/nock_sold"), "87");
    assert!(st(&l, "/sentence").contains("3 of 3 deposits are left, holding up to $766.97 USDC and 86.79 NOCK now"), "{}", st(&l, "/sentence"));
    assert!(st(&l, "/sentence").contains("about 71,975 NOCK, about 71,888 of it bought, at an average of $0.01067"), "{}", st(&l, "/sentence"));
}

#[test]
fn test_the_picture_has_one_bar_per_rung_and_the_cap_straddles_spot() {
    let l = card(&fresh());
    let p = &l["picture"];
    let bars = p["bars"].as_array().unwrap();
    assert_eq!(bars.len(), 3);
    let labels: Vec<&str> = bars.iter().map(|b| b["label"].as_str().unwrap()).collect();
    assert_eq!(labels, ["$1.22 + 87 NOCK", "$114.86", "$650.89"]);
    assert!(bars.iter().all(|b| b["deployed"] == json!(false)));
    let spot = f(p, "/spot_usd");
    assert!(f(&bars[0], "/low_usd") < spot && spot < f(&bars[0], "/high_usd"), "the cap straddles spot");
    assert!(f(&bars[1], "/high_usd") < spot && f(&bars[2], "/high_usd") <= f(&bars[1], "/low_usd"));
    assert_eq!((st(p, "/bottom_usd"), st(p, "/bottom_requested_usd")), ("0.01002405", "0.01"));
    assert_eq!(st(p, "/reserve_label"), "$950 kept for below $0.01");
    // A deployed rung keeps its bar, marked.
    let l = card(&deep_deployed());
    let bars = l["picture"]["bars"].as_array().unwrap();
    assert_eq!(bars.len(), 3);
    let flags: Vec<(&str, bool)> = bars.iter().map(|b| (b["rung"].as_str().unwrap(), b["deployed"].as_bool().unwrap())).collect();
    assert_eq!(flags, [("cap", false), ("near", false), ("deep", true)]);
    assert_eq!((st(&bars[2], "/label"), st(&bars[2], "/status")), ("$650.90", "deployed"));
    // A deployed cap's label carries its NOCK too.
    let l = card(&cap_and_near_deployed());
    assert_eq!(st(&l["picture"]["bars"][0], "/label"), "$1.23 + 87 NOCK", "a deployed rung's bar is its reading now");
    assert_eq!(l["picture"]["bars"][0]["deployed"], json!(true));
}

#[test]
fn test_each_rung_states_its_fee_estimate_with_its_sample_share_and_replay_bound() {
    let env = fresh();
    let l = card(&env);
    let t = text(&env, &l, Some("ok"));
    for line in [
        "    Fees while in range (an estimate): $0.84/day undiluted, $0.84/day diluted (it would be 0.17% of the pool's active liquidity), measured over 6.33 days (1,836 intervals) on v4 USDC/NOCK 3.00% #3163450. In range 1.95% of the measured time. Replayed over it: at most $0.02 inside and $0.15 more at its edges (no lower bound).\n",
        "    Fees while in range (an estimate): $8.56/day undiluted, $8.42/day diluted (it would be 1.73% of the pool's active liquidity), measured over 6.33 days (1,836 intervals) on v4 USDC/NOCK 3.00% #3163450. In range 28.63% of the measured time. Replayed over it: at most $6.39 inside and $5.16 more at its edges (no lower bound).\n",
        "    Fees while in range (an estimate): $85.90/day undiluted, $73.03/day diluted (it would be 14.98% of the pool's active liquidity), measured over 6.33 days (1,836 intervals) on v4 USDC/NOCK 3.00% #3163450. In range 5.82% of the measured time. Replayed over it: at most $32.55 inside and $50.00 more at its edges (no lower bound).\n",
    ] {
        assert!(t.contains(line), "{line} missing in {t}");
    }
    assert_eq!(t.matches("Fees while in range").count(), 3, "one line per rung");
    // The figures are the shape's own, never recomputed.
    let shape = env.data.derived_metrics.ladder_shape.as_ref().unwrap();
    for (i, r) in l["rungs"].as_array().unwrap().iter().enumerate() {
        let fe = &shape["rungs"][i]["fee_estimate"];
        assert_eq!(r["fee_estimate"]["per_day_in_range_usd"], fe["per_day_in_range_usd"]);
        assert_eq!(r["fee_estimate"]["in_range_share_stored_history"]["share_pct"], fe["in_range_share_stored_history"]["share_pct"]);
        assert_eq!(r["fee_estimate"]["stored_history_replay"]["inside_upper_usd"], fe["stored_history_replay"]["inside_upper_usd"]);
        assert_eq!(r["fee_estimate"]["intervals"], json!(1836));
    }
    // No ranking, anywhere.
    let low = t.to_lowercase();
    for word in ["best", "highest", "better", "winner", "rank"] {
        assert!(!low.contains(word), "{word} in {t}");
    }
    // Never a figure without its companions: each missing part prints its reason.
    let line = crate::ladder_suggest_text::fee_estimate_line;
    assert_eq!(
        line(Some(&json!({ "status": "measured", "per_day_in_range_usd": "1.00", "in_range_share_reason": "no stored history", "replay_reason": "no stored history" }))),
        "Fees while in range (an estimate): $1.00/day undiluted; diluted not stated (no reason given), measured sample size not stated. In range over stored history: not stated (no stored history). Replay: not stated (no stored history)."
    );
    // Not measured: null with the reason, never a zero.
    let mut env = prepared(fixture());
    env.data.derived_metrics.fee_yield = None;
    env.data.derived_metrics.ladder_shape = ladder_shape_block(&env.data, now());
    let l = card(&env);
    assert_eq!(st(&l, "/status"), "ready");
    for r in l["rungs"].as_array().unwrap() {
        assert_eq!(st(r, "/fee_estimate/status"), "unavailable");
        assert!(r["fee_estimate"]["per_day_in_range_usd"].is_null());
        let said = line(r.get("fee_estimate"));
        assert!(said.starts_with("Fees while in range: not estimated: ") && said.len() > 40, "{said}");
        assert!(!said.contains("$0"), "{said}");
    }
    assert_eq!(line(None), "Fees while in range: not stated in this body.");
}

#[test]
fn test_the_bottle_on_the_live_snapshot_of_2026_11_21() {
    // The operator's live snapshot from chudmax (tests_ladder_suggest.rs), with their shape:
    // the same $765.86 the single rung of #168 was sized to, now as three deposits.
    let mut env = crate::tests_ladder_suggest::live();
    let gs = &mut env.data.derived_metrics.global_status;
    gs.ladder_bottom_usd = Some("0.01000000".into());
    gs.ladder_rungs = Some(ladder_rung_settings(&bottle()));
    env.data.derived_metrics.ladder_shape = ladder_shape_block(&env.data, env.meta.generated_at);
    let l = card(&env);
    assert_eq!(st(&l, "/status"), "ready", "{l:#}");
    assert_eq!(st(&l, "/headline"), "Ladder: 3 of 3 deposits left, up to $765.85 USDC and about 87 NOCK.");
    assert_eq!(
        st(&l, "/wallet_after/text"),
        "After these deposits your wallet holds about $780 USDC; the $950 reserve counts the $222 of USDC inside your existing NOCK ranges. At $0.01 with every rung converted, the stable balance reads $950.02: the wallet's $779.99 plus $170.03 still inside those ranges."
    );
    let summaries: Vec<&str> = l["rungs"].as_array().unwrap().iter().map(|r| r["summary"].as_str().unwrap()).collect();
    assert_eq!(summaries, ["about 86.79 NOCK + $1.23 USDC, in range now", "$114.69 USDC, no NOCK, below the price now", "$649.93 USDC, no NOCK, below the price now"]);
    assert!(f(&l, "/wallet_after/stable_at_gate_usdc") >= 950.0);
    // This stored snapshot predates the fee measurement: each rung says so, with no figure.
    let t = text(&env, &l, Some("ok"));
    assert_eq!(t.matches("    Fees while in range: not estimated: this snapshot has no fee_yield (2.10.0): fees are not measured in it.\n").count(), 3, "{t}");
    assert!(t.contains("Stated, not gates: Plan budget not yet spent: $0.00 (stated, not a gate). Tax reserve: $8.70 (stated, not a gate)."), "{t}");
    assert_measurement_wording(&env, &l);
}

// ------------------------------------------------------------------
// Plan-stable states
// ------------------------------------------------------------------

#[test]
fn test_a_deployed_deep_rung_is_done_and_the_other_two_are_unchanged() {
    let before = card(&fresh());
    let env = deep_deployed();
    let l = card(&env);
    // A tracked ladder no longer hides the card: the shape counts it.
    assert_eq!((st(&l, "/status"), &l["shown"]), ("ready", &json!(true)), "{l:#}");
    assert_eq!(st(&l, "/tracked/0/position_id"), "uniswap-v4:base:9000001");
    assert_eq!(st(&l, "/gates/ladder_tracked/status"), "counted");
    assert_eq!(st(&l, "/headline"), "Ladder: 2 of 3 deposits left, up to $116.08 USDC and about 87 NOCK.");
    assert_eq!((l["shape"]["left"].as_u64(), l["shape"]["deployed"].as_u64()), (Some(2), Some(1)));
    // The deep rung is done, says what it holds, and is not offered again.
    let rungs = l["rungs"].as_array().unwrap();
    assert_eq!(rungs.iter().map(|r| r["rung"].as_str().unwrap()).collect::<Vec<_>>(), ["cap", "near"]);
    let done = row(&l, "deep");
    assert_eq!(st(done, "/status"), "deployed");
    assert_eq!(st(done, "/text"), "Deposit 3 of 3 (deep rung): done. It is deployed in v4 USDC/NOCK 3.00% #9000001, which holds $650.90 USDC and 0 NOCK now.");
    assert_eq!((st(done, "/holds_now/usdc"), done["counterparts"].clone()), ("650.90", json!(["uniswap-v4:base:9000001"])));
    // The cap and the near rung keep their numbers, their ticks and their exact mints.
    for role in ["cap", "near"] {
        let (a, b) = (deposit(&l, role), deposit(&before, role));
        for ptr in ["/index", "/of", "/tick_lower", "/tick_upper", "/mint", "/usdc", "/nock", "/summary", "/deep_link"] {
            assert_eq!(a.pointer(ptr), b.pointer(ptr), "{role} {ptr}");
        }
    }
    let t = text(&env, &l, Some("ok"));
    let (a, b, c) = (t.find("  Deposit 1 of 3 (cap): about 86.79 NOCK").expect(&t), t.find("  Deposit 2 of 3 (near rung): $114.86 USDC").expect(&t), t.find("  Deposit 3 of 3 (deep rung): done.").expect(&t));
    assert!(a < b && b < c);
    assert_eq!(t.matches("Deposit link (pool and range prefilled)").count(), 2);
    // The outcomes are for what is left, and say so.
    assert_eq!(
        st(&l, "/outcomes/0/text"),
        "A fall to $0.01097 (-15%), the bottom of what is left to deposit, would buy about 9,811 NOCK in total with all $116.08, an average of $0.01183."
    );
    assert_eq!(l["outcomes"].as_array().unwrap().len(), 2);
    // The wallet line counts the deployed rung's USDC as inside a NOCK range now.
    assert_eq!((st(&l, "/wallet_after/wallet_usdc_after"), st(&l, "/wallet_after/stable_at_gate_usdc")), ("778.86", "950.01"));
    assert!(f(&l, "/wallet_after/existing_ranges_usdc_now") > 650.0);
}

#[test]
fn test_with_the_cap_and_near_deployed_only_the_deep_rung_is_left() {
    let before = card(&fresh());
    let env = cap_and_near_deployed();
    let l = card(&env);
    assert_eq!(st(&l, "/status"), "ready", "{l:#}");
    assert_eq!(st(&l, "/headline"), "Ladder: 1 of 3 deposits left, up to $650.89 USDC.");
    let rungs = l["rungs"].as_array().unwrap();
    assert_eq!(rungs.len(), 1);
    let (a, b) = (&rungs[0], deposit(&before, "deep"));
    assert_eq!((a["index"].as_u64(), a["of"].as_u64(), st(a, "/rung")), (Some(3), Some(3), "deep"));
    for ptr in ["/mint", "/usdc", "/tick_lower", "/tick_upper", "/summary"] {
        assert_eq!(a.pointer(ptr), b.pointer(ptr), "{ptr}");
    }
    assert_eq!(st(row(&l, "cap"), "/text"), "Deposit 1 of 3 (cap): done. It is deployed in v4 USDC/NOCK 3.00% #9000002, which holds $1.23 USDC and 87 NOCK now.");
    assert_eq!(st(row(&l, "near"), "/text"), "Deposit 2 of 3 (near rung): done. It is deployed in v4 USDC/NOCK 3.00% #9000003, which holds $114.86 USDC and 0 NOCK now.");
    // Nothing left holds NOCK: a rise sells nothing, and no NOCK limit is stated.
    assert_eq!(st(l["outcomes"].as_array().unwrap().last().unwrap(), "/text"), "A rise sells nothing: these deposits hold 0 NOCK, so their USDC stays USDC.");
    assert!(a["nock_limit"].is_null());
    assert!(st(&l, "/sentence").contains("1 of 3 deposits is left (2 deployed)"), "{}", st(&l, "/sentence"));
    let t = text(&env, &l, Some("ok"));
    assert!(!t.contains("must not exceed"), "{t}");
    assert!(t.contains("  Deposit 3 of 3 (deep rung): $650.89 USDC, no NOCK, below the price now."), "{t}");
}

#[test]
fn test_with_every_rung_deployed_the_card_says_the_ladder_is_deployed() {
    let env = all_deployed();
    let l = card(&env);
    assert_eq!((st(&l, "/status"), &l["shown"]), ("ladder_deployed", &json!(false)), "{l:#}");
    assert_eq!(st(&l, "/headline"), "Ladder deployed: all 3 deposits are done.");
    assert_eq!(st(&l, "/reason"), "Nothing is left to deposit: of the 3 rungs of your shape, 3 deployed. Their readings are in the Ladders card.");
    assert!(l["rungs"].as_array().unwrap().is_empty());
    assert!(l["picture"].is_null() && l["wallet_after"].is_null());
    assert_eq!((l["shape"]["left"].as_u64(), l["shape"]["deployed"].as_u64()), (Some(0), Some(3)));
    let t = text(&env, &l, Some("ok"));
    assert!(t.starts_with("• Ladder suggestion: Ladder deployed: all 3 deposits are done. (ladder_deployed) Nothing is left to deposit"), "{t}");
    for role in ["cap", "near rung", "deep rung"] {
        assert!(t.contains(&format!("({role}): done. It is deployed in v4 USDC/NOCK 3.00% #")), "{role}: {t}");
    }
    assert!(!t.contains("https://"), "{t}");
}

#[test]
fn test_a_passed_rung_states_its_plain_reason_and_the_rest_is_still_listed() {
    let env = near_passed();
    let l = card(&env);
    assert_eq!(st(&l, "/status"), "ready", "{l:#}");
    assert!(st(&l, "/headline").starts_with("Ladder: 2 of 3 deposits left, up to $"), "{}", st(&l, "/headline"));
    assert_eq!(l["shape"]["passed"].as_u64(), Some(1));
    assert_eq!(st(row(&l, "near"), "/status"), "passed");
    // The price is above the rung's bottom: the cap's band covers what is left of it.
    assert_eq!(st(row(&l, "near"), "/text"), "Deposit 2 of 3 (near rung): passed. The price $0.01130146 is less than one tick spacing above this rung's bottom $0.01096801: the cap's band, the one containing the price, covers all that is left of it, so it has no range of its own.");
    assert_eq!(l["rungs"].as_array().unwrap().iter().map(|r| (r["index"].as_u64().unwrap(), r["rung"].as_str().unwrap())).collect::<Vec<_>>(), [(1, "cap"), (3, "deep")]);
    assert_eq!(l["picture"]["bars"].as_array().unwrap().len(), 2, "a passed rung has no bar");
    // The cap is in the band that holds the price now, and its limit is still the budget.
    let cap = deposit(&l, "cap");
    assert_eq!((cap["tick_lower"].as_i64(), cap["tick_upper"].as_i64()), (Some(275100), Some(275400)));
    // Half a tick under the band's top the near rung's USDC binds: type the USDC field.
    assert!(f(cap, "/nock/amount") <= 86.7891 && st(cap, "/nock_limit/text").starts_with("The USDC side must not exceed $100.94, the most that keeps it at or below the near rung's USDC, which binds now"), "{}", st(cap, "/nock_limit/text"));
    // The outcomes follow the shape's own totals for what is left.
    let shape = env.data.derived_metrics.ladder_shape.as_ref().unwrap();
    let fall = l["outcomes"].as_array().unwrap().iter().find(|o| o["kind"] == "fall_to_bottom").unwrap();
    assert!((f(fall, "/nock_bought") - f(shape, "/combined/nock_bought_at_deepest_bottom")).abs() < 0.51);
    let t = text(&env, &l, Some("ok"));
    assert!(t.contains("  Deposit 2 of 3 (near rung): passed. The price $0.01130146 is less than one tick spacing above"), "{t}");
    // Every rung passed: its own status, with each rung's reason.
    let env = all_passed();
    let l = card(&env);
    assert_eq!((st(&l, "/status"), &l["shown"]), ("price_at_or_below_bottom", &json!(false)), "{l:#}");
    assert_eq!(st(&l, "/headline"), "Ladder: NOCK is at or below the bottom of your ladder shape, so no deposit is measured above it.");
    assert!(l["rungs"].as_array().unwrap().is_empty());
    let t = text(&env, &l, Some("ok"));
    assert_eq!(t.matches("is at or below this rung's bottom").count(), 2, "{t}");
    assert!(t.contains("  Deposit 1 of 3 (cap): passed. Every rung below the cap is passed"), "{t}");
}

#[test]
fn test_a_first_rung_deposited_larger_than_stated_sets_the_plan_and_the_reserve_is_stated() {
    // The deep rung alone, a fifth more than stated: it is the first rung deposited, so it
    // is its own target (deployed, never "over"), and the plan follows it. The reserve then
    // leaves no room for the rest: a true headline, each rung still stated.
    let (_, _, d) = stated_liquidity();
    let env = deposited(&[(DEEP, d + d / 5)]);
    let l = card(&env);
    assert_eq!(st(row(&l, "deep"), "/status"), "deployed");
    assert_eq!((st(&l, "/status"), st(&l, "/headline")), ("no_headroom", "Ladder: 2 of 3 deposits left, but your reserve leaves under $1.00 for them."));
    assert_eq!(st(row(&l, "near"), "/text"), "Deposit 2 of 3 (near rung): left to deposit, $137.83 USDC.");
    assert!(text(&env, &l, Some("ok")).contains("(deep rung): done."));
    // An over rung needs the deposit order: tests_ladder_liquidity.rs.
    let l = card(&crate::tests_ladder_liquidity::near_over());
    assert_eq!((st(row(&l, "near"), "/status"), l["shape"]["over"].as_u64()), ("over", Some(1)));
    assert!(l["rungs"].as_array().unwrap().iter().all(|r| r["rung"] != "near"));
}

#[test]
fn test_a_shape_that_cannot_be_drawn_says_why() {
    // A configuration error: named, and nothing is measured.
    let mut bad = fixture();
    let note = "operator_profile.ladder_rungs is a configuration error: it lists no rung; it is not applied";
    bad.data.derived_metrics.global_status.ladder_rungs_note = Some(note.into());
    bad.data.derived_metrics.ladder_shape = ladder_shape_block(&bad.data, now());
    let l = card(&bad);
    assert_eq!((st(&l, "/status"), st(&l, "/reason"), &l["shown"]), ("invalid", note, &json!(false)));
    assert_eq!(st(&l, "/headline"), "Ladder: a setting is invalid, so none is measured.");
    assert!(l["rungs"].as_array().unwrap().is_empty() && l["shape"].is_null());
    // The reserve leaves no room: the shape's own reason.
    let mut rich = fixture();
    rich.data.derived_metrics.global_status.cash_reserve_usdc = Some("5000.00".to_string());
    let env = prepared(rich);
    let l = card(&env);
    assert_eq!(st(&l, "/status"), "no_headroom", "{l:#}");
    assert_eq!(
        st(&l, "/reason"),
        "With your $5000 reserve kept for below $0.01, the reserve gate allows $0.00 for what is left of your shape: nothing above the $1.00 minimum fits, so no deposit is listed."
    );
    assert_eq!(st(&l, "/headline"), "Ladder: 3 of 3 deposits left, but your reserve leaves under $1.00 for them.");
    // No price-source pool: unavailable, with the reason.
    let mut blind = prepared(fixture());
    blind.data.positions.iter_mut().for_each(|p| p.is_price_source = false);
    blind.data.derived_metrics.ladder_shape = ladder_shape_block(&blind.data, now());
    let l = card(&blind);
    assert_eq!(st(&l, "/status"), "unavailable", "{l:#}");
    assert!(st(&l, "/reason").ends_with("no live price-source position is in this snapshot: there is no pool to draw the shape in"), "{}", st(&l, "/reason"));
    assert_eq!(st(&l, "/headline"), "Ladder: not measured this cycle.");
    // No reserve set: the setting is named; the shape needs no ladder_bottom_usd.
    let mut noreserve = fixture();
    noreserve.data.derived_metrics.global_status.cash_reserve_usdc = None;
    let env = prepared(noreserve);
    let l = ladder_suggestion(&env, Ok(Some(0.01)));
    assert_eq!((st(&l, "/status"), l["missing"].clone()), ("needs_setting", json!([LADDER_RESERVE_KEY])));
    assert!(l["candidates"].is_null(), "with the bottom set, no candidate bottoms");
    // Neither set: both are named (no default for either), with the candidate bottoms.
    let l = ladder_suggestion(&env, Ok(None));
    assert_eq!((st(&l, "/status"), l["missing"].clone()), ("needs_setting", json!([LADDER_BOTTOM_KEY, LADDER_RESERVE_KEY])));
    // An invalid ladder_bottom_usd.
    let l = ladder_suggestion(&fresh(), Err(LADDER_SUGGEST_BOTTOM_INVALID_NOTE));
    assert_eq!((st(&l, "/status"), st(&l, "/reason")), ("invalid", LADDER_SUGGEST_BOTTOM_INVALID_NOTE));
    // The pool cannot hold a ladder this cycle: its reason, and no deposit.
    let mut frozen = fresh();
    frozen.data.positions.iter_mut().filter(|p| p.position_id == V4_ID).for_each(|p| p.pool_active = Some(false));
    let l = card(&frozen);
    assert_eq!(st(&l, "/status"), "no_target_pool");
    assert_eq!(st(&l, "/excluded/0/reason"), "this pool has no in-range liquidity, so its price is frozen");
    assert!(l["rungs"].as_array().unwrap().is_empty());
    // A tracked ladder that matches no rung fills none: the reserve gate counts what it
    // holds like any other range, and the rest of the shape is listed in what is left
    // (review of the bottle, finding 8: never "no USDC is free" beside $720 of deposits).
    let env = prepared(fixture_with_rungs(&[(9000005, None, 275400, 276300, 1_000_000_000_000_000)]));
    let l = card(&env);
    assert_eq!(st(&l, "/status"), "ready", "{l:#}");
    assert_eq!(st(&l, "/headline"), "Ladder: 3 of 3 deposits left, up to $720.90 USDC and about 87 NOCK.");
    assert_eq!(st(&l["shape"]["unmatched"][0], "/position_id"), "uniswap-v4:base:9000005");
    assert_eq!(st(&l["shape"]["unmatched"][0], "/reason"), "its rung is not set, so it matches no configured rung");
    // A block sized for more USDC than the wallet now holds: nothing is listed.
    let mut spent = fresh();
    spent.data.wallet_balances.iter_mut().filter(|w| w.token_symbol == "USDC").for_each(|w| w.amount = "100.000000".to_string());
    let l = card(&spent);
    assert_eq!(st(&l, "/status"), "no_headroom");
    assert_eq!(st(&l, "/reason"), "The wallet holds $100.00 USDC, less than the $766.97 what is left of your shape would take: no deposit is listed.");
    assert!(l["rungs"].as_array().unwrap().is_empty() && l["picture"].is_null() && l["wallet_after"].is_null());
    // A block without its exact mints (a stored row's): nothing is listed, rather than a
    // deposit without its exact amounts.
    let mut slim = fresh();
    for r in slim.data.derived_metrics.ladder_shape.as_mut().unwrap()["rungs"].as_array_mut().unwrap() {
        r["liquidity_raw"] = Value::Null;
    }
    let l = card(&slim);
    assert_eq!(st(&l, "/status"), "unavailable");
    assert!(st(&l, "/reason").starts_with("the exact mint of deposit 1 (cap) was not worked out this cycle"), "{}", st(&l, "/reason"));
    assert!(l["rungs"].as_array().unwrap().is_empty());
}

// ------------------------------------------------------------------
// No shape configured: the single rung of #168, unchanged
// ------------------------------------------------------------------

#[test]
fn test_without_a_shape_the_card_is_the_single_rung_of_168() {
    // The bytes for every #168 status are pinned by testdata/ladder_suggest/
    // golden_ladder_suggest.json (tests_ladder_suggest.rs), a file this change leaves
    // identical to main's. Here: none of the shape's fields appears, the fee measurement
    // changes nothing, and a tracked ladder still means no card.
    let env = with_pool_key(fixture());
    let l = card(&env);
    assert_eq!(st(&l, "/status"), "ready");
    assert_eq!(st(&l, "/headline"), "Ladder ready: up to $766.99 USDC in 1 deposit, 0 NOCK.");
    for key in ["shape", "wallet_after", "mint_note"] {
        assert!(l.get(key).is_none(), "{key} belongs to a configured shape only");
    }
    let r = &l["rungs"][0];
    for key in ["summary", "mint", "nock_limit", "fee_estimate", "status"] {
        assert!(r.get(key).is_none(), "{key}");
    }
    assert!(r["rung"].is_null());
    let mut with_fees = env.clone();
    with_fees.data.derived_metrics.fee_yield = fee_fixture().data.derived_metrics.fee_yield;
    assert_eq!(card(&with_fees), l);
    assert_eq!(text(&with_fees, &card(&with_fees), Some("ok")), text(&env, &l, Some("ok")));
    assert!(!text(&env, &l, Some("ok")).contains("Fees while in range"));
    let mut tracked = env.clone();
    tracked.data.positions[0].role = Some(LADDER_ROLE.to_string());
    assert_eq!(st(&card(&tracked), "/status"), "ladder_tracked");
    // The golden file's own bodies, rebuilt here from the same fixture.
    let golden: Value = serde_json::from_str(include_str!("../testdata/ladder_suggest/golden_ladder_suggest.json")).unwrap();
    assert_eq!(golden["ready"]["ladder_suggestion"], l);
    assert_eq!(golden["ready"]["text"].as_str().unwrap(), text(&env, &l, Some("ok")));
}

// ------------------------------------------------------------------
// It measures: wording, the directive, the state hash, freshness
// ------------------------------------------------------------------

pub(crate) fn every_bottle_state() -> Vec<(&'static str, SnapshotEnvelope, Value)> {
    use crate::tests_ladder_liquidity::{deep_held, moved, moved_with, near_found, near_over};
    let mut rich = fixture();
    rich.data.derived_metrics.global_status.cash_reserve_usdc = Some("5000.00".to_string());
    let (c, n, d) = stated_liquidity();
    let mut out = Vec::new();
    for (name, env) in [
        ("bottle", fresh()),
        ("deep_deployed", deep_deployed()),
        ("cap_and_near_deployed", cap_and_near_deployed()),
        ("all_deployed", all_deployed()),
        ("near_passed", near_passed()),
        ("all_passed", all_passed()),
        ("near_over", near_over()),
        ("no_room", prepared(rich)),
        ("unmatched", prepared(fixture_with_rungs(&[(9000005, None, 275400, 276300, 1_000_000_000_000_000)]))),
        // Deposit by deposit while the price moves (review of the bottle, 2026-11-21).
        ("deep_deployed_up_8_ticks", moved(&[(DEEP, d)], 8.0)),
        ("cap_deployed_above_its_band", moved(&[(CAP, c)], 160.0)),
        ("cap_deployed_below_its_band", moved(&[(CAP, c)], -200.0)),
        ("cap_and_near_deployed_down_200_ticks", moved(&[(CAP, c), (NEAR, n)], -200.0)),
        ("all_deployed_up_300_ticks", moved(&[(CAP, c), (NEAR, n), (DEEP, d)], 300.0)),
        ("all_deployed_down_1000_ticks", moved(&[(CAP, c), (NEAR, n), (DEEP, d)], -1000.0)),
        ("near_found_untracked", near_found(273900, 275400)),
        ("near_cut_by_the_reserve", moved_with(&[(CAP, c), (DEEP, d)], 0.0, -0.50)),
        ("cap_bound_by_the_reserve", moved_with(&[(DEEP, d)], 0.0, -0.50)),
        ("deep_held_for_nock", deep_held()),
    ] {
        let l = card(&env);
        out.push((name, env, l));
    }
    out
}

#[test]
fn test_the_bottle_wording_is_a_measurement_in_every_state() {
    let states = every_bottle_state();
    let seen: std::collections::BTreeSet<&str> = states.iter().map(|(_, _, l)| st(l, "/status")).collect();
    for want in ["ready", "ladder_deployed", "price_at_or_below_bottom", "no_headroom", "nock_exceeds"] {
        assert!(seen.contains(want), "{want} not covered: {seen:?}");
    }
    let rows: std::collections::BTreeSet<String> =
        states.iter().flat_map(|(_, _, l)| l["shape"]["rungs"].as_array().cloned().unwrap_or_default()).map(|r| r["status"].as_str().unwrap().to_string()).collect();
    for want in ["to_deposit", "deployed", "passed", "over", "found", "held", "below_minimum"] {
        assert!(rows.contains(want), "{want} row not covered: {rows:?}");
    }
    for (_, env, l) in &states {
        assert_measurement_wording(env, l);
    }
    // The settings errors too.
    let mut bad = fixture();
    bad.data.derived_metrics.global_status.ladder_rungs_note = Some("operator_profile.ladder_rungs is a configuration error: it lists no rung; it is not applied".into());
    assert_measurement_wording(&bad, &card(&bad));
    // And the cap with no room in its band (a `none` row): the reason is the shape's.
    let none = prepared(fixture_at_sqrt(sqrt_x96_of_raw(1.0001f64.powf(275700.0))));
    let l = card(&none);
    assert_eq!(st(row(&l, "cap"), "/status"), "none", "{:#}", l["shape"]);
    assert!(st(row(&l, "cap"), "/text").starts_with("Deposit 1 of 3 (cap): no cap this cycle. "), "{}", st(row(&l, "cap"), "/text"));
    assert_measurement_wording(&none, &l);
}

#[test]
fn test_the_bottle_is_shown_only_on_a_fresh_snapshot() {
    let env = fresh();
    let l = card(&env);
    for health in [Some("degraded"), Some("starting"), None] {
        let t = text(&env, &l, health);
        assert!(t.starts_with("• Ladder suggestion: not shown: the snapshot is not fresh (worker health: "), "{t}");
        assert!(!t.contains("https://") && !t.contains("Deposit 1 of 3"), "{t}");
    }
    assert_eq!(st(&l, "/freshness_rule"), LADDER_FRESHNESS_RULE);
}

#[test]
fn test_the_bottle_never_changes_the_directive_and_its_fees_stay_out_of_the_state_hash() {
    // Through the cycle's own entry point, with the operator's profile.
    let mut data = fresh().data;
    data.derived_metrics.ladder_suggestion = None;
    let before = (data.derived_metrics.global_status.shot_advice_action.clone(), data.derived_metrics.global_status.shot_advice_details.clone());
    attach_ladder_suggestion(&mut data, &profile(), now());
    let l = data.derived_metrics.ladder_suggestion.clone().unwrap();
    assert_eq!(st(&l, "/status"), "ready");
    assert_eq!(l["rungs"].as_array().unwrap().len(), 3);
    assert_eq!(before, (data.derived_metrics.global_status.shot_advice_action.clone(), data.derived_metrics.global_status.shot_advice_details.clone()));
    // The cycle works the shape out before the card reads it, after the directive is captured.
    let ingest = include_str!("ingest.rs");
    let capture = ingest.find("let next_directive_details").unwrap();
    let shape = ingest.find("snapshot_data.derived_metrics.ladder_shape = ladder_shape_block").unwrap();
    let attach = ingest.find("attach_ladder_suggestion(&mut snapshot_data, &profile").unwrap();
    assert!(capture < shape && shape < attach);
    // Without a stored block the card works the shape out itself, to the same body.
    let mut bare = fresh();
    bare.data.derived_metrics.ladder_shape = None;
    bare.meta.generated_at = now();
    assert_eq!(card(&bare), card(&fresh()));
    // An hourly re-measure of the fees changes the fee lines and not the state hash.
    let hash = compute_state_hash(&data);
    let mut later = data.clone();
    for r in later.derived_metrics.ladder_suggestion.as_mut().unwrap()["rungs"].as_array_mut().unwrap() {
        r["fee_estimate"]["per_day_in_range_usd"] = json!("999.99");
    }
    assert_eq!(compute_state_hash(&later), hash);
    // The amounts are state: a different mint is a different hash.
    let mut other = data.clone();
    other.derived_metrics.ladder_suggestion.as_mut().unwrap()["rungs"][1]["usdc"]["raw"] = json!("1");
    assert_ne!(compute_state_hash(&other), hash);
    // A stored row says the fee estimate is left out, rather than keep a stale one.
    let mut stored = serde_json::to_value(&data).unwrap();
    slim_stored_data(&mut stored);
    for r in stored["derived_metrics"]["ladder_suggestion"]["rungs"].as_array().unwrap() {
        assert_eq!(st(r, "/fee_estimate/status"), "unavailable");
        assert!(st(r, "/fee_estimate/reason").contains("left out of the stored row"));
        assert!(!r["mint"]["liquidity_raw"].is_null(), "the mint stays");
    }
}

#[test]
fn test_the_spec_documents_the_shape_on_the_card() {
    assert_eq!(SNAPSHOT_SPEC_VERSION, "2.10.0");
    let text = include_str!("../../specs/core/snapshot.yaml");
    let spec: serde_yaml::Value = serde_yaml::from_str(text).unwrap();
    assert_eq!(spec["info"]["version"].as_str(), Some("2.10.0"));
    for needle in ["ladder_deployed", "shape.rungs", "wallet_after", "nock_limit", "golden_ladder_bottle.json"] {
        assert!(text.contains(needle), "snapshot.yaml does not mention {needle}");
    }
    let example = include_str!("../config.yaml.example");
    assert!(example.contains("one\n  # numbered deposit per rung still to deposit"), "config.yaml.example describes the card with a shape");
}

// ------------------------------------------------------------------
// The bytes both MCP transports print
// ------------------------------------------------------------------

/// The golden bodies and text for a configured shape (mcp_bin_tests_ladder_suggest.rs and
/// the dashboard's fixture read it). Regenerate only for an intended change:
/// NOCKNOCK_BLESS_GOLDEN=1 cargo test test_golden_ladder_bottle
#[test]
fn test_golden_ladder_bottle_text() {
    let mut doc = serde_json::Map::new();
    for (name, env, l) in every_bottle_state() {
        doc.insert(name.to_string(), json!({ "health": "ok", "ladder_suggestion": l, "text": text(&env, &l, Some("ok")) }));
    }
    let env = fresh();
    let l = card(&env);
    doc.insert("bottle_stale".to_string(), json!({ "health": "degraded", "ladder_suggestion": l, "text": text(&env, &l, Some("degraded")) }));
    // The stored row read back after a restart, with a healthy worker: both transports
    // say the card is not available until the first cycle (second review of the bottle).
    let mut stored_env = fresh();
    stored_env.data.derived_metrics.ladder_suggestion = Some(card(&stored_env));
    let mut stored = serde_json::to_value(&stored_env.data).unwrap();
    slim_stored_data(&mut stored);
    let sl = stored["derived_metrics"]["ladder_suggestion"].clone();
    let st_text = crate::ladder_suggest_text::ladder_suggestion_lines(&json!({ "derived_metrics": { "ladder_suggestion": sl } }), Some("ok"));
    doc.insert("bottle_stored_row".to_string(), json!({ "health": "ok", "ladder_suggestion": sl, "text": st_text }));
    let actual = format!("{}\n", serde_json::to_string_pretty(&Value::Object(doc)).unwrap());
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/ladder_suggest/golden_ladder_bottle.json");
    if std::env::var("NOCKNOCK_BLESS_GOLDEN").is_ok() {
        std::fs::write(path, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(path).expect("testdata/ladder_suggest/golden_ladder_bottle.json is checked in");
    assert!(actual == expected, "the bottle's card changed; re-bless and review {path}");
}

