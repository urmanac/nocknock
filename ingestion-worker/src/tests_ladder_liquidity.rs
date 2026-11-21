//! The bottle followed deposit by deposit while the price moves (review of the bottle,
//! 2026-11-21). Each test reproduces a reviewed failure:
//! - each rung's target is a liquidity on its own ticks, so a deployed rung stays deployed
//!   whatever the price does (findings 3, 4, 12);
//! - the plan is frozen by the first near or deep rung deposited, never re-divided from the
//!   reserve gate each cycle (findings 3, 13);
//! - what is left is cut pro rata to what the reserve gate allows, with the shortfall
//!   stated; `no_headroom` only when nothing above the $1.00 minimum fits, with a true
//!   headline (findings 3, 8, 13);
//! - a remainder under the $1.00 minimum is never offered (findings 7, 16);
//! - a position found on-chain on a rung's exact ticks is not offered again, and position
//!   detection tracks a new mint with that rung's role (finding 5);
//! - the cap states the limit that binds and the field that keeps it (findings 6, 15);
//! - a deposit that needs NOCK the budget cannot give is held back with a true reason
//!   (findings 10, 14); and every figure the card adds is the sum of what it says to enter
//!   (findings 11, 18).
use super::*;
use crate::tests_ladder::fixture_with_rungs_moved;
use crate::tests_ladder_bottle::*;
use crate::tests_ladder_suggest::text;
use crate::tests_position_detect::{candidate, OPERATOR};
use serde_json::Value;

pub(crate) type Rung = ((u64, Option<&'static str>, i32, i32), u128);

/// The bottle with `rungs` minted at the fixture's spot, then the pool moved `ticks_up`
/// (the Aero pool along with it), and the wallet's USDC moved by `wallet_delta`.
pub(crate) fn moved_with(rungs: &[Rung], ticks_up: f64, wallet_delta: f64) -> SnapshotEnvelope {
    let rows: Vec<(u64, Option<&str>, i32, i32, u128)> = rungs.iter().map(|((id, role, tl, tu), l)| (*id, *role, *tl, *tu, *l)).collect();
    let mut env = fixture_with_rungs_moved(&rows, ticks_up);
    if wallet_delta != 0.0 {
        for w in env.data.wallet_balances.iter_mut().filter(|w| w.token_symbol == "USDC") {
            w.amount = format!("{:.6}", w.amount.parse::<f64>().unwrap() + wallet_delta);
        }
    }
    prepared(env)
}

pub(crate) fn moved(rungs: &[Rung], ticks_up: f64) -> SnapshotEnvelope {
    moved_with(rungs, ticks_up, 0.0)
}

/// Re-plan the shape after changing the envelope's readings or settings.
pub(crate) fn replanned(mut env: SnapshotEnvelope) -> SnapshotEnvelope {
    env.data.derived_metrics.ladder_shape = ladder_shape_block(&env.data, now());
    env
}

/// The USDC each deposit states, cut to cents, and their sum as the headline states it.
fn stated_sum(l: &Value) -> f64 {
    let sum: f64 = l["rungs"].as_array().unwrap().iter().map(|r| (f(r, "/usdc/amount") * 100.0 + 1e-7).floor() / 100.0).sum();
    (sum * 100.0).round() / 100.0
}

/// The headline's "up to $X" is the sum of the amounts the deposits state, to the cent,
/// and so are the outcome, the wallet line and the numbers (findings 11, 18).
fn assert_one_figure_per_deposit(l: &Value) {
    let sum = format!("{:.2}", stated_sum(l));
    assert!(st(l, "/headline").contains(&format!("up to ${sum} USDC")), "{} vs {sum}", st(l, "/headline"));
    assert_eq!(st(l, "/numbers/usdc_max"), sum);
    assert_eq!(st(l, "/wallet_after/deposits_usdc"), sum);
    let fall = l["outcomes"].as_array().unwrap().iter().find(|o| o["kind"] == "fall_to_bottom").unwrap();
    assert!(st(fall, "/text").contains(&format!("with all ${sum}")), "{}", st(fall, "/text"));
    for r in l["rungs"].as_array().unwrap() {
        let cut = format!("{:.2}", (f(r, "/usdc/amount") * 100.0 + 1e-7).floor() / 100.0);
        assert!(st(r, "/summary").contains(&format!("${cut} USDC")), "{} vs {cut}", st(r, "/summary"));
    }
}

/// Never the old all-or-nothing: a card with deposits left lists them.
fn assert_listed(l: &Value, roles: &[&str]) {
    assert_eq!((st(l, "/status"), &l["shown"]), ("ready", &json!(true)), "{l:#}");
    let got: Vec<&str> = l["rungs"].as_array().unwrap().iter().map(|r| r["rung"].as_str().unwrap()).collect();
    assert_eq!(got, roles);
    assert!(!st(l, "/headline").contains("no USDC is free"));
}

#[test]
fn test_deep_deposited_then_the_price_moves_keeps_the_rest_listed() {
    let (_, _, d) = stated_liquidity();
    let fresh_near = st(deposit(&card(&fresh()), "near"), "/usdc/to_enter").to_string();
    assert_eq!(fresh_near, "114.86");
    for ticks in [0.0, 1.0, 8.0, 100.0, -107.0] {
        let env = moved(&[(DEEP, d)], ticks);
        let l = card(&env);
        assert_listed(&l, &["cap", "near"]);
        assert!(st(&l, "/headline").starts_with("Ladder: 2 of 3 deposits left, up to $"), "{ticks}: {}", st(&l, "/headline"));
        // The deep rung is deployed by liquidity: never "over", never offered again.
        let deep = row(&l, "deep");
        assert_eq!(st(deep, "/status"), "deployed", "{ticks}");
        assert_eq!(deep["tracked_liquidity"], deep["target_liquidity"], "{ticks}: the first rung deposited is its own target");
        // The plan is frozen by it: the near rung's target does not move with the price.
        assert_eq!(st(row(&l, "near"), "/target_usdc"), "114.86", "{ticks}");
        // The near rung's remainder is exactly what it was: the price moves the cap only.
        assert_eq!(st(deposit(&l, "near"), "/usdc/to_enter"), "114.86", "{ticks}");
        assert_eq!(st(&l, "/fit/status"), "fits", "{ticks}");
        // Up the band the budget's NOCK pairs with more USDC than the reserve leaves beside
        // the near rung: the cap holds less NOCK, and its row and limit say so.
        let cap = deposit(&l, "cap");
        assert!(f(cap, "/nock/amount") <= 86.7891);
        if st(cap, "/cap/bound_by") == "reserve" {
            assert!(st(cap, "/nock_limit/text").contains("what your reserve leaves for it beside the other deposits listed, which binds now, so the cap holds about"), "{}", st(cap, "/nock_limit/text"));
            assert_eq!(st(cap, "/nock_limit/field_to_type"), "USDC");
        }
        assert_one_figure_per_deposit(&l);
    }
    // One tick up: the cents it costs are absorbed or stated, never a hidden list.
    let l = card(&moved(&[(DEEP, d)], 1.0));
    assert_eq!(st(deposit(&l, "near"), "/usdc/to_enter"), "114.86");
    // Eight ticks up: the reviewed case. The text both transports print.
    let env = moved(&[(DEEP, d)], 8.0);
    let l = card(&env);
    let t = text(&env, &l, Some("ok"));
    assert!(t.contains("  Deposit 3 of 3 (deep rung): done. It is deployed in v4 USDC/NOCK 3.00% #9000001, which holds $650.90 USDC and 0 NOCK now.\n"), "{t}");
    assert!(!t.contains("no deposit is listed"), "{t}");
}

#[test]
fn test_the_cap_deposited_then_the_price_leaves_its_band_reads_deployed() {
    let (c, _, _) = stated_liquidity();
    // Above the band (+1.6%: the cap is all USDC) and below it (-2%: all NOCK).
    for ticks in [0.0, 160.0, 400.0, -200.0] {
        let env = moved(&[(CAP, c)], ticks);
        let l = card(&env);
        assert_listed(&l, &["near", "deep"]);
        let cap = row(&l, "cap");
        assert_eq!(st(cap, "/status"), "deployed", "{ticks}: a tracked cap is the cap, whatever its band holds now");
        assert!(st(cap, "/text").starts_with("Deposit 1 of 3 (cap): done. It is deployed in v4 USDC/NOCK 3.00% #9000002, which holds $"), "{}", st(cap, "/text"));
        assert_eq!(l["shape"]["over"].as_u64(), Some(0));
        // Nothing near or deep is deposited, so the reserve gate sizes them; the cap's USDC
        // has left the wallet, and the gate does not move with the price.
        assert_eq!((st(deposit(&l, "near"), "/usdc/to_enter"), st(deposit(&l, "deep"), "/usdc/to_enter")), ("114.86", "650.89"), "{ticks}");
        assert_eq!(st(&l, "/headline"), "Ladder: 2 of 3 deposits left, up to $765.75 USDC.");
        assert_one_figure_per_deposit(&l);
    }
}

#[test]
fn test_every_rung_deposited_reads_ladder_deployed_at_every_price() {
    let (c, n, d) = stated_liquidity();
    for ticks in [0.0, 8.0, 160.0, 300.0, -200.0, -1000.0, -2000.0, -2700.0] {
        let env = moved(&[(CAP, c), (NEAR, n), (DEEP, d)], ticks);
        let l = card(&env);
        assert_eq!((st(&l, "/status"), &l["shown"]), ("ladder_deployed", &json!(false)), "{ticks}: {l:#}");
        assert_eq!(st(&l, "/headline"), "Ladder deployed: all 3 deposits are done.", "{ticks}");
        assert!(l["rungs"].as_array().unwrap().is_empty());
    }
    // The operator's $5 harvest lands in the wallet: no sub-dollar top-up returns the card
    // to the top of the page (finding 16), and neither does a dip in the gated total.
    for delta in [5.0, -0.70, 50.0] {
        let l = card(&moved_with(&[(CAP, c), (NEAR, n), (DEEP, d)], 0.0, delta));
        assert_eq!((st(&l, "/status"), st(&l, "/headline")), ("ladder_deployed", "Ladder deployed: all 3 deposits are done."), "{delta}");
    }
    // A rung deposited a little short (the app rounds, the operator types cents): its
    // remainder under $1.00 is stated on its row and never offered (finding 7).
    let env = moved(&[(CAP, c), (NEAR, n), (DEEP, d - d / 1000)], 0.0);
    let l = card(&env);
    assert_eq!(st(&l, "/headline"), "Ladder deployed: all 3 deposits are done.");
    let deep = row(&l, "deep");
    assert!(st(deep, "/text").ends_with("of its target is not deposited: under the $1.00 minimum, so it is not offered."), "{}", st(deep, "/text"));
}

#[test]
fn test_cap_and_near_deposited_then_the_price_moves_leaves_the_deep_rung_unchanged() {
    let (c, n, _) = stated_liquidity();
    for ticks in [0.0, 8.0, 100.0, 300.0, -200.0, -1000.0] {
        let env = moved(&[(CAP, c), (NEAR, n)], ticks);
        let l = card(&env);
        assert_listed(&l, &["deep"]);
        assert_eq!(st(&l, "/headline"), "Ladder: 1 of 3 deposits left, up to $650.89 USDC.", "{ticks}");
        assert_eq!(st(deposit(&l, "deep"), "/usdc/to_enter"), "650.89", "{ticks}");
        assert!(st(&l, "/shape/rest_rule").starts_with("frozen by the near rung already deposited"), "{}", st(&l, "/shape/rest_rule"));
        assert_eq!((st(row(&l, "cap"), "/status"), st(row(&l, "near"), "/status")), ("deployed", "deployed"));
        assert_one_figure_per_deposit(&l);
    }
}

/// The bottle with the near rung minted through the link and found on-chain this cycle,
/// not tracked yet (its receipt not read, say): it is in no position list, and position
/// detection lists it as found and not counted, with its ticks and liquidity.
pub(crate) fn near_found(tl: i32, tu: i32) -> SnapshotEnvelope {
    near_found_moved(tl, tu, 0.0)
}

/// The near rung minted at the fixture's spot through the link, then the price moved
/// `ticks_up`, and the mint found on-chain, not yet tracked.
pub(crate) fn near_found_moved(tl: i32, tu: i32, ticks_up: f64) -> SnapshotEnvelope {
    let (_, n, _) = stated_liquidity();
    let env = fixture_with_rungs_moved(&[(9000003, Some("untracked"), tl, tu, n)], ticks_up);
    // Minted by the wallet itself, its receipt read (the second review of the bottle: only
    // such a position stands for a rung).
    let wallet = env.data.positions[0].wallet.to_lowercase();
    prepared(with_found(env, 9000003, tl, tu, n, Some(&wallet), None))
}

/// `env` with one position found on-chain and not counted, in the ladder's pool. A
/// stranger's position has `minted_by` another address, or arrived by transfer
/// (`received_from`, then no mint receipt is read and `minted_by` is null).
pub(crate) fn with_found(mut env: SnapshotEnvelope, nft: u64, tl: i32, tu: i32, l: u128, minted_by: Option<&str>, received_from: Option<&str>) -> SnapshotEnvelope {
    let why: Vec<String> = match (minted_by, received_from) {
        (_, Some(from)) => vec![format!("it reached the wallet by transfer from {from}, it was not minted to it")],
        (Some(by), None) if by != env.data.positions[0].wallet.to_lowercase() => vec![format!("its mint was sent by {by}, not by your wallet")],
        (Some(_), None) => vec!["it matches a suggested rung, and tracking it did not go through this cycle; it is tried again next cycle".to_string()],
        (None, None) => vec!["its mint transaction has not been read yet".to_string()],
    };
    env.data.derived_metrics.position_detection = Some(json!({
        "status": "ok",
        "reason": null,
        "found_not_counted": {
            "count": 1,
            "value_usd": null,
            "value_note": null,
            "positions": [{
                "nft_id": nft,
                "position_id": format!("uniswap-v4:base:{nft}"),
                "protocol": "uniswap-v4",
                "pool_id": "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13",
                "pool_label": "v4 USDC/NOCK 3.00%",
                "tick_lower": tl,
                "tick_upper": tu,
                "holds": { "liquidity": l.to_string() },
                "value_usd": null,
                "mint_tx": minted_by.map(|_| "0x6cdabada1ff062b6d3b4ba87bfe04158dd8b14a91e93cabe11dc3e35b587e89e"),
                "mint_block": minted_by.map(|_| 60240576),
                "minted_by": minted_by,
                "received_from": received_from,
                "why_not_automatic": why,
                "choices": ["ladder:cap", "ladder:near", "ladder:deep", "lp", "dismiss"],
            }],
        },
    }));
    env
}

#[test]
fn test_a_mint_found_on_chain_and_not_yet_tracked_is_not_offered_again() {
    let env = near_found(273900, 275400);
    let l = card(&env);
    assert_listed(&l, &["cap", "deep"]);
    let near = row(&l, "near");
    assert_eq!(st(near, "/status"), "found");
    assert_eq!(
        st(near, "/text"),
        "Deposit 2 of 3 (near rung): found on-chain, being tracked. The position uniswap-v4:base:9000003, which your wallet minted, is this rung in this pool; NockNock tracks it by itself and counts it from then, so it is not offered again."
    );
    // Its USDC left the wallet: it counts for its rung, so the deep rung is not re-sized.
    assert_eq!(st(deposit(&l, "deep"), "/usdc/to_enter"), "650.89");
    assert!(st(&l, "/headline").starts_with("Ladder: 2 of 3 deposits left"), "{}", st(&l, "/headline"));
    assert!(st(&l, "/sentence").contains("(1 found on-chain, being tracked)"), "{}", st(&l, "/sentence"));
    assert!(l["found_note"].is_null());
    // A stranger's mint on the rung's exact ticks never stands for the rung: offered again.
    let mut env = near_found(273900, 275400);
    env.data.derived_metrics.position_detection.as_mut().unwrap()["found_not_counted"]["positions"][0]["minted_by"] = json!("0x4598b3ba7d6714091b18da5d110b64f56e94e826");
    let l = card(&replanned(env));
    assert_listed(&l, &["cap", "near", "deep"]);
    // A range found on other ticks matches no rung: stated, and not counted until a tap.
    // (Its top is two bands from the near rung's top, so the drift rule does not match it.)
    let env = near_found(274500, 275400);
    let l = card(&env);
    assert_listed(&l, &["cap", "near", "deep"]);
    assert_eq!(
        st(&l, "/found_note"),
        "Found on-chain in this pool and not counted yet: NFT #9000003 on ticks 274500..275400 (it matches a suggested rung, and tracking it did not go through this cycle; it is tried again next cycle). It stands for no rung of your shape and is not tracked by itself; one tap under Found on-chain tracks it or sets it aside. The amounts here do not count it."
    );
    assert!(text(&env, &l, Some("ok")).contains(&format!("  {}\n", st(&l, "/found_note"))));
}

#[test]
fn test_a_rung_just_minted_is_tracked_with_its_rung_from_the_shape() {
    // The snapshot the card was read from is the one detection matches against.
    let mut env = fresh();
    env.data.derived_metrics.ladder_suggestion = Some(card(&env));
    let prev = serde_json::to_value(&env.data).unwrap();
    let suggested = crate::position_detect::suggested_rungs(&prev);
    for (role, tl, tu) in [("cap", 273600, 273900), ("near", 273900, 275400), ("deep", 275400, 276300)] {
        assert!(suggested.iter().any(|s| s.rung.as_deref() == Some(role) && s.tick_lower == tl && s.tick_upper == tu), "{role}: {suggested:?}");
        let minted = CandidateRow { tick_lower: Some(tl), tick_upper: Some(tu), ..candidate(9000010) };
        assert_eq!(crate::position_detect::decide(&minted, OPERATOR, &suggested, &[]), Decision::Track { rung: Some(role.to_string()) }, "{role}");
    }
    // After a deposit the next card still names every rung's ticks, deployed ones too, so
    // a second mint on a deployed rung's ticks is never tracked as a new rung by itself.
    let mut env = deep_deployed();
    env.data.derived_metrics.ladder_suggestion = Some(card(&env));
    let prev = serde_json::to_value(&env.data).unwrap();
    let suggested = crate::position_detect::suggested_rungs(&prev);
    assert!(suggested.iter().any(|s| s.rung.as_deref() == Some("deep") && s.tick_lower == 275400));
    let tracked = crate::position_detect::tracked_ranges_in_snapshot(&prev);
    let again = CandidateRow { tick_lower: Some(275400), tick_upper: Some(276300), ..candidate(9000011) };
    assert!(matches!(crate::position_detect::decide(&again, OPERATOR, &suggested, &tracked), Decision::Offer { .. }));
    // The stored row (read back after a restart) still names the rungs and their ticks.
    let mut env = fresh();
    env.data.derived_metrics.ladder_suggestion = Some(card(&env));
    let mut stored = serde_json::to_value(&env.data).unwrap();
    slim_stored_data(&mut stored);
    assert_eq!(crate::position_detect::suggested_rungs(&stored).len(), 3);
}

#[test]
fn test_a_cents_level_shortfall_cuts_pro_rata_and_never_hides_the_list() {
    let (c, _, d) = stated_liquidity();
    // Fifty cents less in the wallet after the cap and the deep rung: the near rung alone
    // needs more than the reserve allows. It is cut, the shortfall is stated, and it is
    // still listed.
    let env = moved_with(&[(CAP, c), (DEEP, d)], 0.0, -0.50);
    let l = card(&env);
    assert_listed(&l, &["near"]);
    assert_eq!((st(&l, "/fit/status"), st(&l, "/fit/shortfall_usdc")), ("reduced", "0.50"));
    assert_eq!(st(&l, "/gates/reserve/status"), "reduces");
    assert_eq!(
        st(&l, "/fit/text"),
        "Your $950 reserve, kept for below $0.01, allows $114.36 for what is left of your shape, $0.50 short of the $114.86 it would take: each deposit is cut by the same fraction, so the shape keeps its proportions."
    );
    assert!(st(&l, "/sentence").contains("cut to what your $950 reserve at $0.01 allows"), "{}", st(&l, "/sentence"));
    assert_eq!(st(deposit(&l, "near"), "/usdc/to_enter"), "114.36");
    assert!(st(deposit(&l, "near"), "/summary").ends_with("after the reserve gate's cut"));
    assert!(f(&l, "/wallet_after/stable_at_gate_usdc") >= 950.0);
    assert_one_figure_per_deposit(&l);
    // With the cap still to deposit, the cap takes the cut: the frozen near rung does not.
    let l = card(&moved_with(&[(DEEP, d)], 0.0, -0.50));
    assert_listed(&l, &["cap", "near"]);
    assert_eq!(st(&l, "/fit/status"), "fits");
    assert_eq!(st(deposit(&l, "near"), "/usdc/to_enter"), "114.86");
    let cap = deposit(&l, "cap");
    assert_eq!(st(cap, "/cap/bound_by"), "reserve");
    assert!(f(cap, "/usdc/amount") < 1.0 && f(cap, "/nock/amount") < 86.0, "{cap:#}");
    // Under a cent: absorbed, nothing to state.
    let l = card(&moved_with(&[(CAP, c), (DEEP, d)], 0.0, -0.004));
    assert_listed(&l, &["near"]);
    assert_eq!(st(&l, "/fit/status"), "fits");
    assert!(l["fit"]["text"].is_null());
    // Nothing above the minimum fits: a true headline, the rows still state what is left.
    let mut rich = crate::tests_ladder::fixture();
    rich.data.derived_metrics.global_status.cash_reserve_usdc = Some("5000.00".to_string());
    let env = prepared(rich);
    let l = card(&env);
    assert_eq!(st(&l, "/status"), "no_headroom");
    assert_eq!(st(&l, "/headline"), "Ladder: 3 of 3 deposits left, but your reserve leaves under $1.00 for them.");
    assert!(st(&l, "/reason").starts_with("With your $5000 reserve kept for below $0.01, the reserve gate allows $0.00 for what is left of your shape"), "{}", st(&l, "/reason"));
}

#[test]
fn test_the_cap_text_states_the_limit_that_binds_and_the_field_that_keeps_it() {
    // The LP NOCK budget binds: type the NOCK field; the USDC limit is stated too.
    let env = fresh();
    let l = card(&env);
    let cap = deposit(&l, "cap");
    assert_eq!(st(cap, "/cap/bound_by"), "lp_nock_budget");
    // Nothing is deposited yet, so the rule is the most that keeps the cap at or below the
    // near rung's USDC: $100.04 of $766.99 (the near rung is 15% of what the cap leaves).
    assert_eq!((st(cap, "/nock_limit/binding"), st(cap, "/nock_limit/field_to_type"), st(cap, "/nock_limit/usdc")), ("nock", "NOCK", "100.04"));
    assert_eq!(cap["nock_limit"]["holds_less_than_budget"], json!(false));
    assert_eq!(
        st(cap, "/deep_link/amount_to_enter"),
        "Amount to enter in the app: 86.7890 NOCK, in the NOCK field; the app then works out the USDC side, about $1.22 at this reading."
    );
    assert!(st(cap, "/nock_limit/text").starts_with(
        "The NOCK side must not exceed 86.7891 NOCK, your LP NOCK budget, which binds now; the USDC side must not exceed $100.04, the most that keeps it at or below the near rung's USDC. The limits hold when the NOCK field is the one typed, at 86.7890 NOCK or less, and the app shows at most $100.04 of USDC"
    ));
    // The near rung's USDC binds (the price near the top of the cap's band): type the USDC
    // field, and the card says the cap holds less than the budget.
    let env = near_passed();
    let l = card(&env);
    let cap = deposit(&l, "cap");
    assert_eq!(st(cap, "/cap/bound_by"), "near_usdc");
    assert_eq!((st(cap, "/nock_limit/binding"), st(cap, "/nock_limit/field_to_type")), ("usdc", "USDC"));
    assert_eq!(cap["nock_limit"]["holds_less_than_budget"], json!(true));
    let usdc = st(cap, "/usdc/to_enter");
    assert_eq!(
        st(cap, "/deep_link/amount_to_enter"),
        format!("Amount to enter in the app: ${usdc} USDC, in the USDC field; the app then works out the NOCK side, about {} NOCK at this reading.", fx4(f(cap, "/nock/amount")))
    );
    let limit = st(cap, "/nock_limit/text");
    assert!(limit.contains("less than your 86.7891 NOCK LP budget"), "{limit}");
    assert!(limit.contains(&format!("The limit holds when the USDC field is the one typed, at ${usdc} or less.")), "{limit}");
    assert!(limit.contains(LADDER_USDC_LIMIT_WHY));
    let t = text(&env, &l, Some("ok"));
    assert!(t.contains(&format!("    {limit}\n")), "{t}");
}

fn fx4(v: f64) -> String {
    format!("{:.4}", (v * 1e4).round() / 1e4)
}

/// The deep rung first (block 100), as stated; the near rung later (block 200), half as
/// much again as stated, with $60 that came into the wallet after it.
pub(crate) fn near_over() -> SnapshotEnvelope {
    let (_, n, d) = stated_liquidity();
    let mut env = moved_with(&[(DEEP, d), (NEAR, n + n / 2)], 0.0, 60.0);
    for r in env.data.ladders.iter_mut() {
        r.since_deposit.deposit_block = Some(if r.position_id.ends_with("9000001") { 100 } else { 200 });
    }
    replanned(env)
}

/// Cap and near deposited, half the deep rung, the price fallen into the deep rung, and a
/// floor + tolerance budget of one NOCK: what is left of the deep rung needs more NOCK.
pub(crate) fn deep_held() -> SnapshotEnvelope {
    let (c, n, d) = stated_liquidity();
    let mut env = moved(&[(CAP, c), (NEAR, n), (DEEP, d / 2)], -2000.0);
    for r in env.data.ladders.iter_mut() {
        r.since_deposit.deposit_block = Some(if r.position_id.ends_with("9000003") { 100 } else { 200 });
    }
    env.data.derived_metrics.global_status.lp_nock_budget = Some("1.0000".to_string());
    replanned(env)
}

#[test]
fn test_an_over_rung_is_stated_by_deposit_order_and_changes_nothing_else() {
    let (_, n, d) = stated_liquidity();
    let env = near_over();
    let l = card(&env);
    assert_listed(&l, &["cap"]);
    let near = row(&l, "near");
    assert_eq!(st(near, "/status"), "over");
    assert!(st(near, "/text").starts_with("Deposit 2 of 3 (near rung): over its target; it is stated and changes nothing else. The tracked rung holds $172.30 of USDC at its top, $57.43 more than its target."), "{}", st(near, "/text"));
    assert_eq!(st(row(&l, "deep"), "/status"), "deployed");
    assert!(st(&l, "/shape/rest_rule").contains("the first deposited, by its deposit block"));
    // Without that USDC the over rung took the reserve's room: a true headline, every row
    // still stated.
    let mut env = moved(&[(DEEP, d), (NEAR, n + n / 2)], 0.0);
    for r in env.data.ladders.iter_mut() {
        r.since_deposit.deposit_block = Some(if r.position_id.ends_with("9000001") { 100 } else { 200 });
    }
    let l = card(&replanned(env));
    // The cap is the last rung and the reserve has no room for it: nothing is left to
    // deposit, and the card says why (second review of the bottle, item 3).
    assert_eq!((st(&l, "/status"), &l["shown"]), ("ladder_deployed", &json!(false)));
    assert_eq!(st(&l, "/headline"), "Ladder: nothing is left to deposit (1 deployed, 1 over target, 1 not deposited (the reserve leaves no room)).");
    assert!(st(row(&l, "cap"), "/text").starts_with("Deposit 1 of 3 (cap): not deposited: the reserve leaves $"), "{}", st(row(&l, "cap"), "/text"));
    // Without the deposit order, the rung implying the most freezes the plan: the deep rung
    // then has a remainder to deposit rather than the near rung reading over.
    let l = card(&moved_with(&[(DEEP, d), (NEAR, n + n / 2)], 0.0, 200.0));
    assert_eq!(st(row(&l, "near"), "/status"), "deployed");
    assert_eq!(st(row(&l, "deep"), "/status"), "to_deposit");
}

#[test]
fn test_a_remainder_that_needs_nock_the_budget_cannot_give_is_held_with_a_true_reason() {
    let env = deep_held();
    let l = card(&env);
    assert_eq!(st(&l, "/status"), "nock_exceeds", "{l:#}");
    let reason = st(&l, "/reason");
    assert!(reason.starts_with("Deposit 3 (deep rung): it would need "), "{reason}");
    assert!(reason.contains(" NOCK now, and the floor + tolerance budget has 1.0000 left for it."), "{reason}");
    // The need and the allowance are different figures, and the cap is not blamed.
    assert!(!reason.contains("(cap)"));
    assert_eq!(st(row(&l, "deep"), "/status"), "held");
}

#[test]
fn test_a_shape_without_its_bottom_needs_the_setting() {
    let mut env = fresh();
    env.data.derived_metrics.global_status.ladder_bottom_usd = None;
    let env = replanned(env);
    let l = ladder_suggestion(&env, Ok(None));
    assert_eq!((st(&l, "/status"), l["missing"].clone(), &l["shown"]), ("needs_setting", json!([LADDER_BOTTOM_KEY]), &json!(false)));
    assert_eq!(st(&l, "/headline"), "Ladder: needs your ladder bottom (operator_profile.ladder_bottom_usd) before your shape is measured.");
    assert!(st(&l, "/reason").contains("operator_profile.ladder_bottom_usd (the price the reserve gate is read at; it has no default)"));
    assert!(!l["candidates"]["rows"].as_array().unwrap().is_empty(), "candidate bottoms, none picked");
    assert!(l["rungs"].as_array().unwrap().is_empty());
}

#[test]
fn test_the_stored_row_keeps_the_card_small_and_says_what_it_left_out() {
    let mut env = fresh();
    env.data.derived_metrics.ladder_suggestion = Some(card(&env));
    let full = serde_json::to_string(&env.data.derived_metrics.ladder_suggestion).unwrap().len();
    let mut stored = serde_json::to_value(&env.data).unwrap();
    slim_stored_data(&mut stored);
    let l = &stored["derived_metrics"]["ladder_suggestion"];
    let slim = serde_json::to_string(l).unwrap().len();
    assert!(full > 15_000 && slim < 6_000, "{full} -> {slim}");
    assert_eq!((&l["shown"], st(l, "/reason")), (&json!(false), STORED_LADDER_NOTE));
    assert!(st(l, "/detail_note").contains("left out of the stored row"));
    for r in l["rungs"].as_array().unwrap() {
        assert!(r["mint"]["liquidity_raw"].is_string() && r["tick_lower"].is_i64() && r["pool"]["pool_id"].is_string());
        assert!(r.get("deep_link").is_none() && r.get("nock_limit").is_none());
    }
    // The state hash is the whole card's, never the slimmed row's. Slimming changes what a
    // hash would read (so the order matters)...
    let full = crate::ingest::canonical_state_json(&env.data);
    let mut slimmed = stored.clone();
    crate::ingest::strip_volatile_time_fields(&mut slimmed);
    assert_ne!(full["derived_metrics"]["ladder_suggestion"], slimmed["derived_metrics"]["ladder_suggestion"]);
    // ...and the cycle hashes the assembled data, while store_snapshot stores the
    // envelope's hash as it is and slims only the JSON it writes.
    let ingest = include_str!("ingest.rs");
    let store_fn = &ingest[ingest.find("pub(crate) async fn store_snapshot(").unwrap()..];
    let store_fn = &store_fn[..store_fn.find("\n}\n").unwrap()];
    assert!(store_fn.contains("slim_stored_data(&mut data_json)") && store_fn.contains("envelope.meta.state_hash"));
    assert!(!store_fn.contains("compute_state_hash"), "the stored row's hash is never recomputed from the slimmed JSON");
    let (hash_at, store_at) = (ingest.find("let state_hash = compute_state_hash(&snapshot_data);").unwrap(), ingest.find("store_snapshot(&state.db, &envelope).await?").unwrap());
    assert!(hash_at < store_at);
    assert_eq!(crate::ingest::compute_state_hash(&env.data), {
        let mut h = sha2::Sha256::new();
        sha2::Digest::update(&mut h, serde_json::to_vec(&full).unwrap());
        format!("{:x}", sha2::Digest::finalize(h))
    });
}

#[test]
fn test_each_fee_line_states_the_diluted_figure_beside_the_undiluted_one() {
    let env = fresh();
    let l = card(&env);
    let deep = deposit(&l, "deep");
    let (undiluted, diluted, share) = (
        st(deep, "/fee_estimate/per_day_in_range_usd"),
        st(deep, "/fee_estimate/per_day_in_range_diluted_usd"),
        st(deep, "/fee_estimate/dilution/share_of_active_pct"),
    );
    assert_eq!((undiluted, diluted), ("85.90", "73.03"));
    let t = text(&env, &l, Some("ok"));
    assert!(t.contains(&format!("    Fees while in range (an estimate): ${undiluted}/day undiluted, ${diluted}/day diluted (it would be {share}% of the pool's active liquidity), measured over 6.33 days")), "{t}");
    // No dilution figure: the reason is stated in its place, never the undiluted alone.
    let line = crate::ladder_suggest_text::fee_estimate_line(Some(&json!({ "status": "measured", "per_day_in_range_usd": "1.00", "dilution_reason": "active liquidity not read" })));
    assert!(line.starts_with("Fees while in range (an estimate): $1.00/day undiluted; diluted not stated (active liquidity not read)"), "{line}");
}

#[test]
fn test_each_passed_rung_states_its_own_true_reason() {
    // Spot half a tick into the band above the near rung's bottom: the cap's band covers it.
    let l = card(&near_passed());
    assert_eq!(
        st(row(&l, "near"), "/text"),
        "Deposit 2 of 3 (near rung): passed. The price $0.01130146 is less than one tick spacing above this rung's bottom $0.01096801: the cap's band, the one containing the price, covers all that is left of it, so it has no range of its own."
    );
    // Every rung passed: the cap says why there is no cap, and the card names the rungs.
    let l = card(&all_passed());
    assert_eq!(st(row(&l, "cap"), "/text"), "Deposit 1 of 3 (cap): passed. Every rung below the cap is passed (the price is at or below the ladder's bottom), so no cap is drawn.");
    assert!(st(row(&l, "deep"), "/text").contains("is at or below this rung's bottom"));
    assert_eq!(
        st(&l, "/reason"),
        "NOCK at $0.01002405 is at or below the bottom of your ladder shape: every rung is passed (cap, near rung, deep rung), so no USDC-only deposit is possible in any of them."
    );
}

// ------------------------------------------------------------------
// Second review of the bottle (HEAD da29b33)
// ------------------------------------------------------------------

const STRANGER: &str = "0x4598b3ba7d6714091b18da5d110b64f56e94e826";

/// Set what each tracked rung was deposited with (the verified mint's liquidity) and its
/// deposit block, as the readings state them, then re-plan.
fn with_deposits(mut env: SnapshotEnvelope, deposits: &[(u64, u128, i64)]) -> SnapshotEnvelope {
    for (nft, l, block) in deposits {
        let r = env.data.ladders.iter_mut().find(|r| r.position_id == format!("uniswap-v4:base:{nft}")).unwrap_or_else(|| panic!("no reading for {nft}"));
        r.since_deposit.deposit_liquidity = Some(l.to_string());
        r.since_deposit.deposit_block = Some(*block);
    }
    replanned(env)
}

#[test]
fn test_a_strangers_position_never_stands_for_a_rung() {
    let (_, _, d) = stated_liquidity();
    // About $1.50 of USDC at its top, on the deep rung's exact ticks.
    let dust = d * 150 / 65_090;
    let fresh_card = card(&fresh());
    for (by, from) in [(None, Some(STRANGER)), (Some(STRANGER), None), (None, None)] {
        let env = prepared(with_found(crate::tests_ladder::fixture(), 9000020, 275400, 276300, dust, by, from));
        let l = card(&env);
        assert_listed(&l, &["cap", "near", "deep"]);
        assert_eq!(st(&l, "/headline"), st(&fresh_card, "/headline"), "{by:?} {from:?}: the plan is not frozen by it");
        assert_eq!(st(deposit(&l, "deep"), "/usdc/to_enter"), "650.89");
        assert_ne!(st(row(&l, "deep"), "/status"), "found");
        assert!(!serde_json::to_string(&l).unwrap().contains("being tracked"));
        assert!(st(&l, "/found_note").contains("NFT #9000020 on ticks 275400..276300 ("), "{}", l["found_note"]);
        assert!(st(&l, "/found_note").contains("It stands for no rung of your shape"), "{}", l["found_note"]);
    }
    // The same on the near rung's ticks, and on the cap's band (public: it contains spot).
    for (tl, tu) in [(273900, 275400), (273600, 273900)] {
        let l = card(&prepared(with_found(crate::tests_ladder::fixture(), 9000021, tl, tu, dust, None, Some(STRANGER))));
        assert_listed(&l, &["cap", "near", "deep"]);
        assert_eq!(st(&l, "/headline"), st(&fresh_card, "/headline"));
    }
}

#[test]
fn test_a_withdrawal_reads_withdrawn_and_never_shrinks_the_plan() {
    let (c, n, d) = stated_liquidity();
    // All three deposited (cap first, then near, then deep); then half the near withdrawn.
    let env = with_deposits(moved(&[(CAP, c), (NEAR, n / 2), (DEEP, d)], 0.0), &[(9000002, c, 50), (9000003, n, 100), (9000001, d, 200)]);
    let l = card(&env);
    assert_eq!((st(&l, "/status"), &l["shown"]), ("ladder_deployed", &json!(false)), "{l:#}");
    assert_eq!(st(&l, "/headline"), "Ladder: nothing is left to deposit (2 deployed, 1 partly withdrawn).");
    assert_eq!(
        st(row(&l, "near"), "/text"),
        "Deposit 2 of 3 (near rung): partly withdrawn. 50% of the liquidity it was deposited with was withdrawn: about $57.43 of its $114.86 at its top; it is not offered again (re-laddering is #169's)."
    );
    assert_eq!(st(row(&l, "deep"), "/status"), "deployed", "the deep rung holds what was planned: never 'over'");
    assert_eq!(st(row(&l, "cap"), "/status"), "deployed");
    // All of the near withdrawn: withdrawn, and still nothing is offered.
    let env = with_deposits(moved(&[(CAP, c), (NEAR, 0), (DEEP, d)], 0.0), &[(9000002, c, 50), (9000003, n, 100), (9000001, d, 200)]);
    let l = card(&env);
    assert_eq!(st(&l, "/status"), "ladder_deployed");
    assert_eq!(st(row(&l, "near"), "/text"), "Deposit 2 of 3 (near rung): withdrawn. It holds no liquidity now; it is not offered again (re-laddering is #169's).");
    assert_eq!(st(row(&l, "deep"), "/status"), "deployed");
    // The deep rung deposited first, filled, then pulled ("pull the reserve at the right
    // time"): withdrawn; no new deep deposit is offered, and the near rung keeps the plan
    // the deep rung froze.
    let env = with_deposits(moved(&[(DEEP, 0)], -2000.0), &[(9000001, d, 100)]);
    let l = card(&env);
    assert_eq!(st(row(&l, "deep"), "/status"), "withdrawn");
    assert!(l["rungs"].as_array().unwrap().iter().all(|r| r["rung"] != "deep"), "no new deep deposit");
    assert_eq!(st(row(&l, "near"), "/target_usdc"), "114.86", "frozen by what the deep rung was deposited with");
    // Burned: no longer read as a position; detection states it closed, and it stands for
    // its rung as withdrawn.
    let mut env = moved(&[], 0.0);
    env.data.derived_metrics.position_detection = Some(json!({ "status": "ok", "tracked": [{
        "position_id": "uniswap-v4:base:9000001", "nft_id": 9000001, "role": "ladder", "rung": "deep",
        "tick_lower": 275400, "tick_upper": 276300, "state": "closed", "mint_block": 100,
    }] }));
    let l = card(&replanned(prepared(env)));
    assert_eq!(st(row(&l, "deep"), "/text"), "Deposit 3 of 3 (deep rung): withdrawn. Its NFT is burned or no longer in the wallet; it is not offered again (re-laddering is #169's).");
    assert!(l["rungs"].as_array().unwrap().iter().all(|r| r["rung"] != "deep"));
}

#[test]
fn test_the_cap_is_the_last_rung_and_the_ladder_reads_deployed_when_the_reserve_has_no_room_for_it() {
    let (_, n, d) = stated_liquidity();
    // Near and deep deposited as stated; then the wallet holds $1.10 less: the reserve
    // leaves the cap less than the $1.00 minimum. Nothing is left to deposit.
    let env = with_deposits(moved_with(&[(NEAR, n), (DEEP, d)], 0.0, -1.10), &[(9000003, n, 100), (9000001, d, 200)]);
    let l = card(&env);
    assert_eq!((st(&l, "/status"), &l["shown"]), ("ladder_deployed", &json!(false)), "{l:#}");
    assert_eq!(st(&l, "/headline"), "Ladder deployed: 2 of 3 deposits are done, and the reserve leaves no room for the cap.");
    assert!(st(row(&l, "cap"), "/text").starts_with("Deposit 1 of 3 (cap): not deposited: the reserve leaves $0.13 for it beside the other rungs"), "{}", st(row(&l, "cap"), "/text"));
    // The same rule while room remains: the near rung deposited, then the price up 1% in
    // the cap's band. The cap is offered with what the reserve leaves (about 16 NOCK),
    // typed in the USDC field, and the card says it holds less than the budget.
    let l = card(&with_deposits(moved(&[(NEAR, n)], 100.0), &[(9000003, n, 100)]));
    let cap = deposit(&l, "cap");
    assert_eq!((st(cap, "/cap/bound_by"), st(cap, "/nock_limit/field_to_type")), ("reserve", "USDC"));
    assert_eq!(cap["nock_limit"]["holds_less_than_budget"], json!(true));
    assert!(f(cap, "/nock/amount") < 86.0);
    assert_eq!(st(deposit(&l, "deep"), "/usdc/to_enter"), "650.89", "the deep rung keeps the frozen plan");
}

#[test]
fn test_a_near_rung_minted_a_band_ago_is_matched_by_its_bottom_tick_and_tracked() {
    for ticks in [-200.0, 200.0] {
        let env = near_found_moved(273900, 275400, ticks);
        let l = card(&env);
        assert_listed(&l, &["cap", "deep"]);
        assert_eq!(st(row(&l, "near"), "/status"), "found", "{ticks}");
        assert_eq!(st(deposit(&l, "deep"), "/usdc/to_enter"), "650.89", "{ticks}: the deep rung is not re-sized");
        assert!(l["found_note"].is_null());
        // Detection tracks it with that role from the card it was read against.
        let mut prev_env = moved(&[], ticks);
        prev_env.data.derived_metrics.ladder_suggestion = Some(card(&prev_env));
        let prev = serde_json::to_value(&prev_env.data).unwrap();
        let suggested = crate::position_detect::suggested_rungs(&prev);
        assert!(!suggested.iter().any(|s| s.rung.as_deref() == Some("near") && s.tick_lower == 273900), "{ticks}: the near rung's top moved");
        let minted = CandidateRow { tick_lower: Some(273900), tick_upper: Some(275400), ..candidate(9000003) };
        assert_eq!(crate::position_detect::decide(&minted, OPERATOR, &suggested, &[]), Decision::Track { rung: Some("near".to_string()) }, "{ticks}");
        // Never a second near rung beside one that is tracked, and never two bands away.
        let tracked = vec![crate::position_detect::TrackedRange { position_id: "uniswap-v4:base:1".into(), pool_id: crate::tests_position_detect::POOL_3PCT.into(), tick_lower: 274200, tick_upper: 275400 }];
        assert!(matches!(crate::position_detect::decide(&minted, OPERATOR, &suggested, &tracked), Decision::Offer { .. }));
        let near_top = suggested.iter().find(|s| s.rung.as_deref() == Some("near")).unwrap().tick_lower;
        let far = CandidateRow { tick_lower: Some(near_top - 600), tick_upper: Some(275400), ..candidate(9000004) };
        assert!(matches!(crate::position_detect::decide(&far, OPERATOR, &suggested, &[]), Decision::Offer { .. }), "{ticks}");
    }
}

#[test]
fn test_after_a_restart_both_transports_say_the_stored_card_is_not_available() {
    let mut env = fresh();
    env.data.derived_metrics.ladder_suggestion = Some(card(&env));
    let mut stored = serde_json::to_value(&env.data).unwrap();
    slim_stored_data(&mut stored);
    // The worker's /v1/mcp builds get_shot_advice this way (mcp_http.rs); the stdio binary
    // runs the same text module on the same row (mcp_bin_tests_ladder_suggest.rs).
    let advice = crate::mcp_text::format_shot_advice_text(&stored);
    let text = crate::ladder_suggest_text::with_ladder_suggestion(&advice, &stored, Some("ok"));
    assert!(text.contains("• Ladder suggestion: not available until the first cycle after a restart. The stored row read \"Ladder: 3 of 3 deposits left, up to $766.97 USDC and about 87 NOCK.\". This is the stored row of the ladder card"), "{text}");
    for banned in ["$?", "Deposit link: none (not built)", "Deposit 1 of 3", "Amount to enter", "NOCK budget: unknown"] {
        assert!(!text.contains(banned), "{banned} in {text}");
    }
    // The live card in memory still prints in full.
    let live = crate::ladder_suggest_text::with_ladder_suggestion(&advice, &serde_json::to_value(&env.data).unwrap(), Some("ok"));
    assert!(live.contains("Deposit 1 of 3 (cap)") && live.contains("Amount to enter in the app"));
}

// ------------------------------------------------------------------
// Third review of the bottle (HEAD 54ee7a9): what a rung stands for after it is gone
// ------------------------------------------------------------------

/// Detection's `tracked[]` rows, as the database remembers each rung.
fn with_tracked_rows(mut env: SnapshotEnvelope, rows: Vec<Value>) -> SnapshotEnvelope {
    env.data.derived_metrics.position_detection = Some(json!({ "status": "ok", "tracked": rows }));
    env
}

fn tracked_row(nft: u64, rung: &str, tl: i32, tu: i32, state: &str, deposit: Option<u128>, max_seen: Option<u128>, block: i64) -> Value {
    json!({
        "position_id": format!("uniswap-v4:base:{nft}"), "nft_id": nft, "role": "ladder", "rung": rung,
        "tick_lower": tl, "tick_upper": tu, "state": state, "mint_block": block,
        "deposit_liquidity": deposit.map(|l| l.to_string()), "max_liquidity_seen": max_seen.map(|l| l.to_string()),
    })
}

#[test]
fn test_a_burned_or_sent_away_anchor_keeps_standing_for_its_deposit() {
    let (c, n, d) = stated_liquidity();
    let fresh_card = card(&fresh());
    for state in ["closed", "not_owned"] {
        // The deep deposited first, filled, pulled with its NFT burned (or sent away); the
        // price back at spot. Before: every other target fell to $0.
        let env = replanned(prepared(with_tracked_rows(moved(&[], 0.0), vec![tracked_row(9000001, "deep", 275400, 276300, state, Some(d), Some(d), 100)])));
        let l = card(&env);
        assert_listed(&l, &["near"]);
        assert_eq!(st(deposit(&l, "near"), "/usdc/to_enter"), st(deposit(&fresh_card, "near"), "/usdc/to_enter"), "{state}: frozen by what the deep rung was deposited with");
        assert_eq!(st(row(&l, "deep"), "/status"), "withdrawn");
        assert_eq!(st(row(&l, "cap"), "/text"), "Deposit 1 of 3 (cap): not deposited: not offered after a rung was pulled; re-laddering is #169's.");
        // The near and the cap deposited after it: done, never "over".
        let env = replanned(prepared(with_tracked_rows(
            moved(&[(CAP, c), (NEAR, n)], 0.0),
            vec![tracked_row(9000001, "deep", 275400, 276300, state, Some(d), None, 100)],
        )));
        let l = card(&env);
        assert_eq!(st(row(&l, "near"), "/status"), "deployed", "{state}");
        assert_eq!(st(&l, "/status"), "ladder_deployed");
    }
    // Nothing remembered of a burned rung (it was burned before a cycle read it): it reads
    // withdrawn and never freezes the plan, so the others follow the reserve gate.
    let env = replanned(prepared(with_tracked_rows(moved(&[], 0.0), vec![tracked_row(9000001, "deep", 275400, 276300, "closed", None, None, 100)])));
    let l = card(&env);
    assert_eq!(st(row(&l, "deep"), "/status"), "withdrawn");
    assert!(st(&l, "/shape/rest_rule").starts_with("what the reserve gate allows now"), "{}", st(&l, "/shape/rest_rule"));
}

#[test]
fn test_the_database_remembers_a_deposit_when_the_receipt_is_not_read_and_the_most_it_held() {
    let (_, n, d) = stated_liquidity();
    // Half the near withdrawn, its receipt not read this cycle (a restart, a failed read):
    // the database's copy of its deposit says it was pulled. Before: offered again.
    let mut env = moved(&[(NEAR, n / 2), (DEEP, d)], 0.0);
    env = with_tracked_rows(env, vec![tracked_row(9000003, "near", 273900, 275400, "counted", Some(n), Some(n), 100), tracked_row(9000001, "deep", 275400, 276300, "counted", Some(d), Some(d), 200)]);
    let mut env = prepared(env);
    for r in env.data.ladders.iter_mut() {
        r.since_deposit.deposit_liquidity = None; // the receipt was not read
    }
    let l = card(&replanned(env));
    assert_eq!(st(row(&l, "near"), "/status"), "partly_withdrawn");
    assert!(l["rungs"].as_array().unwrap().iter().all(|r| r["rung"] != "near"));
    // Minted at half, topped up to the whole, then 30% withdrawn: it stands for the most
    // it held. Before: the plan shrank by 30% and the deep read "over".
    let mut env = moved(&[(NEAR, n * 7 / 10), (DEEP, d)], 0.0);
    env = with_tracked_rows(env, vec![tracked_row(9000003, "near", 273900, 275400, "counted", Some(n / 2), Some(n), 100), tracked_row(9000001, "deep", 275400, 276300, "counted", Some(d), Some(d), 200)]);
    let mut env = prepared(env);
    for r in env.data.ladders.iter_mut().filter(|r| r.position_id.ends_with("9000003")) {
        r.since_deposit.deposit_liquidity = Some((n / 2).to_string());
        r.since_deposit.deposit_block = Some(100);
    }
    let l = card(&replanned(env));
    assert!(st(row(&l, "near"), "/text").starts_with("Deposit 2 of 3 (near rung): partly withdrawn. 30% of the liquidity"), "{}", st(row(&l, "near"), "/text"));
    assert_eq!(st(row(&l, "deep"), "/status"), "deployed", "never over");
}

#[test]
fn test_a_rung_whose_deposit_is_not_known_never_freezes_the_plan_and_the_card_says_so() {
    let (_, _, d) = stated_liquidity();
    // A config.yaml ladder with no deposit_tx: nothing says what it was deposited with.
    let mut env = moved(&[(DEEP, d)], 0.0);
    env = prepared(env);
    for r in env.data.ladders.iter_mut() {
        r.since_deposit.deposit_liquidity = None;
    }
    let l = card(&replanned(env));
    assert!(st(&l, "/shape/rest_rule").contains("the deep rung is tracked, but what it was deposited with is not known"), "{}", st(&l, "/shape/rest_rule"));
    assert!(st(row(&l, "deep"), "/text").ends_with("What it was deposited with is not known (no deposit_tx in config.yaml, or its mint receipt not read yet), so it does not fix the plan and a withdrawal from it cannot be told."), "{}", st(row(&l, "deep"), "/text"));
}

#[test]
fn test_after_the_deep_is_pulled_the_cap_is_not_offered() {
    let (_, _, d) = stated_liquidity();
    // The deep deposited at spot, the price down 2000 ticks, the deep emptied: its cheap
    // NOCK is back in the wallet and the LP budget jumps. Before: a cap of about 1,844 NOCK.
    let mut env = with_deposits(moved(&[(DEEP, 0)], -2000.0), &[(9000001, d, 100)]);
    env.data.derived_metrics.global_status.lp_nock_budget = Some("23445.2018".to_string());
    let l = card(&replanned(env));
    assert_eq!(st(row(&l, "cap"), "/status"), "not_deposited");
    assert!(l["rungs"].as_array().unwrap().iter().all(|r| r["rung"] != "cap"), "{:#}", l["rungs"]);
    assert_eq!(st(row(&l, "deep"), "/status"), "withdrawn");
    assert_ne!((st(&l, "/status"), &l["shown"]), ("ready", &json!(true)), "the card does not go back to the top");
}

#[test]
fn test_the_cycle_remembers_each_rungs_deposit_and_the_most_it_held() {
    let (_, n, _) = stated_liquidity();
    let env = moved(&[(NEAR, n / 2)], 0.0);
    let mut raw = stored_book(&env.data).raw;
    let row = TrackedRow {
        wallet: env.data.positions[0].wallet.to_lowercase(),
        position_key: "uniswap-v4:base:9000003".to_string(),
        protocol: "uniswap-v4".to_string(),
        network: "base".to_string(),
        nft_id: 9000003,
        pool_id: "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13".to_string(),
        currency0: "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913".to_string(),
        currency1: "0x9b5e262cf9bb04869ab40b19af91d2dc85761722".to_string(),
        tick_lower: 273900,
        tick_upper: 275400,
        tick_spacing: Some(300),
        role: Some(LADDER_ROLE.to_string()),
        rung: Some("near".to_string()),
        deposit_tx: Some("0x01".to_string()),
        mint_block: Some(100),
        source: TRACKED_SOURCE_DETECTED.to_string(),
        tracked_at: None,
        deposit_liquidity: None,
        max_liquidity_seen: Some(n),
    };
    raw.tracked_report = vec![TrackedReport { row: row.clone(), fate: TrackedFate::Merged }];
    raw.ladder_baselines.insert(
        "uniswap-v4:base:9000003".to_string(),
        LadderBaselineRead::Verified(LadderDeposit { tx: "0x01".into(), block: 100, at: None, liquidity: n, amount0_raw: 0, amount1_raw: 0 }),
    );
    // The receipt's liquidity is remembered; the most it held stays n though it holds n/2.
    assert_eq!(liquidity_updates(&raw), vec![("uniswap-v4:base:9000003".to_string(), Some(n), n)]);
    // Remembered already: nothing to write.
    raw.tracked_report[0].row.deposit_liquidity = Some(n);
    assert!(liquidity_updates(&raw).is_empty());
    // A rung not read this cycle (burned): what was remembered stays.
    raw.positions.retain(|p| p.position_id != "uniswap-v4:base:9000003");
    assert!(liquidity_updates(&raw).is_empty());
    // The store keeps a known deposit and takes the new most.
    let rt = tokio::runtime::Runtime::new().unwrap();
    rt.block_on(async {
        let store = PositionStore::mem();
        store.insert_tracked_if_absent(&TrackedRow { deposit_liquidity: None, max_liquidity_seen: None, ..row.clone() }).await.unwrap();
        assert!(store.record_liquidity(&row.wallet, &row.position_key, Some(n), n).await.unwrap());
        assert!(store.record_liquidity(&row.wallet, &row.position_key, Some(1), n + 5).await.unwrap());
        let got = store.tracked(&row.wallet).await.unwrap();
        assert_eq!((got[0].deposit_liquidity, got[0].max_liquidity_seen), (Some(n), Some(n + 5)));
    });
}
