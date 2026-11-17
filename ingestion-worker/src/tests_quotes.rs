//! Route quotes for a due tranche (issue #115).
//!
//! The fixtures under testdata/quotes are real `eth_call` requests and their verbatim
//! answers from Base mainnet, read on 2026-11-16 (ONCHAIN_INTEGRATION.md §11 records how
//! each was obtained). `quotes_block_58081910.json` holds every route of the table, the
//! failing calls and the fee reads, all at one block, for 20 USDC in.

use super::*;

const QUOTE_BLOCK: i64 = 58081910;
/// 20 USDC, the plan's tranche on the day the fixtures were read.
const TRANCHE_RAW: u128 = 20_000_000;

fn fixture(raw: &str) -> serde_json::Value {
    serde_json::from_str(raw).expect("fixture is JSON")
}

fn quotes_fixture() -> serde_json::Value {
    fixture(include_str!("../testdata/quotes/quotes_block_58081910.json"))
}

fn exhausted_fixture() -> serde_json::Value {
    fixture(include_str!("../testdata/quotes/exhausted_block_58081910.json"))
}

/// The fixture key holding the live call for a route of the table.
fn fixture_key(route_id: &str) -> &'static str {
    match route_id {
        "aero-basic-direct" => "aero_basic_usdc_nock",
        "aero-cl50-weth" => "aero_cl50_weth_cl200_nock",
        "aero-cl100-weth" => "aero_cl100_weth_cl200_nock",
        "aero-usdt-weth" => "aero_usdt_weth_nock",
        "uniswap-v4-direct" => "uni_v4_usdc_nock_3pct",
        "uniswap-v3-weth" => "uni_v3_usdc_weth_nock",
        "uniswap-v4-eth" => "uni_v4_usdc_eth_nock",
        other => panic!("no fixture for route {other}"),
    }
}

fn call<'a>(fx: &'a serde_json::Value, key: &str) -> &'a serde_json::Value {
    fx.pointer(&format!("/calls/{key}")).unwrap_or_else(|| panic!("fixture has no call {key}"))
}

/// What the worker's own classifier makes of a captured response.
fn classified(fx: &serde_json::Value, key: &str) -> std::result::Result<String, QuoteCallFailure> {
    classify_quote_call(&call(fx, key)["response"])
}

fn spec(route_id: &str) -> RouteSpec {
    route_table().into_iter().find(|r| r.id == route_id).unwrap_or_else(|| panic!("no route {route_id}"))
}

/// Every route of the table with the outcome its real answer gives.
fn real_results() -> Vec<RouteResult> {
    let fx = quotes_fixture();
    route_table()
        .into_iter()
        .map(|spec| {
            let outcome = outcome_from_call(&spec, classified(&fx, fixture_key(spec.id)));
            RouteResult { spec, outcome }
        })
        .collect()
}

/// The reference spot at the quote block: USDC per NOCK from the v4 3% pool's slot0,
/// the pool the worker's `is_price_source` position names.
fn real_spot() -> f64 {
    let fx = quotes_fixture();
    let hex = classified(&fx, "stateview_slot0_usdc_nock_3pct").unwrap();
    let sqrt_price = U256::from_hex_be(&hex[..64]).unwrap();
    1.0 / sqrt_price_x96_to_raw(sqrt_price) * 1e10
}

fn fee_word(fx: &serde_json::Value, key: &str) -> u128 {
    decode_small_word(&classified(fx, key).unwrap()).unwrap()
}

/// The fees of a route's hops as the fee reads in the fixture give them.
fn real_fees(route_id: &str) -> Vec<Option<String>> {
    let fx = quotes_fixture();
    spec(route_id)
        .hops
        .iter()
        .map(|h| match h.fee {
            HopFee::Fixed(m) => Some(fee_pct_from_millionths(m as u128)),
            HopFee::AeroBasic => Some(fee_pct_from_bps(aero_basic_fee_bps(
                fee_word(&fx, "fee_basic_custom"),
                fee_word(&fx, "fee_basic_default"),
            ))),
            HopFee::AeroConcentrated => {
                let key = match h.pool {
                    AERO_CL50_WETH_USDC => "fee_cl50",
                    AERO_CL100_WETH_USDC => "fee_cl100",
                    AERO_CL200_WETH_NOCK => "fee_cl200_nock",
                    AERO_CL1_USDC_USDT => "fee_usdc_usdt",
                    AERO_CL100_WETH_USDT => "fee_weth_usdt",
                    other => panic!("no fee fixture for pool {other}"),
                };
                Some(fee_pct_from_millionths(fee_word(&fx, key)))
            }
        })
        .collect()
}

fn block_time() -> DateTime<Utc> {
    let fx = quotes_fixture();
    let (number, time) = parse_block_header(&fx["header"]["response"]).expect("fixture header parses");
    assert_eq!(number, QUOTE_BLOCK);
    time
}

/// The comparison the worker would have shown at block 58081910.
fn real_quotes() -> RouteQuotes {
    let results = real_results();
    let spot = Some(real_spot());
    let venues = QUOTE_VENUES
        .iter()
        .map(|venue| {
            let fees = best_route_index(&results, venue).map(|i| real_fees(results[i].spec.id)).unwrap_or_default();
            build_venue_quote(venue, &results, &fees, TRANCHE_RAW, spot)
        })
        .collect();
    assemble_route_quotes("20.00", Some((QUOTE_BLOCK, block_time())), spot, venues)
}

fn plan_cfg() -> AccumulationPlanConfig {
    AccumulationPlanConfig {
        id: "2026-11-reentry".to_string(),
        budget_usdc: 100.0,
        tranches: 5,
        interval_hours: 24.0,
        starts_at: None,
        venue: "Aero".to_string(),
        max_price_usd: None,
        pause_when_overextended: true,
        respect_cash_target: true,
        cash_reserve_usdc: None,
    }
}

fn plan_status(directive: &PlanDirective) -> AccumulationPlanStatus {
    build_plan_status(&plan_cfg(), &PlanFills::default(), directive)
}

fn due(usdc: f64) -> AccumulationPlanStatus {
    plan_status(&PlanDirective::TrancheDue { usdc, limited_by: None })
}

// ---------------- Calldata: what the worker sends is what was verified ----------------

#[test]
fn test_every_route_sends_exactly_the_call_that_was_verified_live() {
    let fx = quotes_fixture();
    assert_eq!(fx["amount_usdc_raw"].as_str(), Some("20000000"));
    let table = route_table();
    assert_eq!(table.len(), 7);
    for spec in &table {
        let live = call(&fx, fixture_key(spec.id));
        let (to, data) = quote_request(spec, TRANCHE_RAW);
        assert!(to.eq_ignore_ascii_case(live["to"].as_str().unwrap()), "{}: quoter {to}", spec.id);
        assert_eq!(data, live["data"].as_str().unwrap(), "{}: calldata", spec.id);
        // Every call in the fixture was made at the one block.
        assert_eq!(live["block"].as_str(), Some("0x3764276"), "{}", spec.id);
    }
    assert_eq!(i64::from_str_radix("3764276", 16).unwrap(), QUOTE_BLOCK);
}

#[test]
fn test_route_table_is_usdc_in_nock_out_on_the_two_venues_only() {
    let table = route_table();
    let mut ids: Vec<&str> = table.iter().map(|r| r.id).collect();
    ids.sort();
    ids.dedup();
    assert_eq!(ids.len(), table.len(), "route ids are unique");
    for r in &table {
        assert!(QUOTE_VENUES.contains(&r.venue), "{}", r.id);
        assert_eq!(r.hops.first().unwrap().token_in, "USDC", "{}", r.id);
        assert_eq!(r.hops.last().unwrap().token_out, "NOCK", "{}", r.id);
        // Each hop starts where the one before it ended.
        for pair in r.hops.windows(2) {
            assert_eq!(pair[0].token_out, pair[1].token_in, "{}", r.id);
        }
        // An Aero route is asked of the Aero quoter, a Uniswap route of a Uniswap one.
        let (to, _) = quote_request(r, TRANCHE_RAW);
        match r.venue {
            VENUE_AERO => assert_eq!(to, AERO_METAQUOTER),
            _ => assert!(to == UNISWAP_V3_QUOTER_V2 || to == UNISWAP_V4_QUOTER),
        }
    }
    assert_eq!(spec("aero-usdt-weth").label(), "USDC → USDT → WETH → NOCK");
    assert_eq!(spec("uniswap-v4-eth").label(), "USDC → ETH → NOCK");
    // The legacy Aerodrome vAMM is not a route: app.aero.xyz cannot trade it.
    let legacy = "0x85f1aa3a70fedd1c52705c15baed143e675cd626";
    assert!(table.iter().all(|r| r.hops.iter().all(|h| !h.pool.eq_ignore_ascii_case(legacy))));
}

#[test]
fn test_calldata_scales_with_the_amount_and_nothing_else() {
    // The amount is one word; the rest of the request is the route.
    let s = spec("uniswap-v3-weth");
    let (_, twenty) = quote_request(&s, 20_000_000);
    let (_, fifty) = quote_request(&s, 50_000_000);
    assert_eq!(twenty.len(), fifty.len());
    assert_eq!(&twenty[10 + 64..10 + 128], &format!("{:064x}", 20_000_000u128));
    assert_eq!(&fifty[10 + 64..10 + 128], &format!("{:064x}", 50_000_000u128));
    assert_eq!(twenty.replace(&format!("{:064x}", 20_000_000u128), ""), fifty.replace(&format!("{:064x}", 50_000_000u128), ""));
    // The v3 path is token, fee, token, fee, token packed: 20 + 3 + 20 + 3 + 20 bytes.
    assert_eq!(&twenty[10 + 128..10 + 192], &format!("{:064x}", 66u128));
    assert!(twenty.contains(&format!(
        "{}0001f4{}002710{}",
        USDC_BASE.trim_start_matches("0x").to_lowercase(),
        WETH_BASE.trim_start_matches("0x").to_lowercase(),
        NOCK_BASE.trim_start_matches("0x").to_lowercase()
    )));
}

// ---------------- Real answers ----------------

#[test]
fn test_real_quoter_answers_decode_to_the_amounts_read_on_chain() {
    let results = real_results();
    let out = |id: &str| match &results.iter().find(|r| r.spec.id == id).unwrap().outcome {
        QuoteOutcome::Quoted(o) => *o,
        QuoteOutcome::Unavailable(why) => panic!("{id}: {why}"),
    };
    assert_eq!(out("aero-basic-direct"), 17_422_508_413_591_799_568);
    assert_eq!(out("aero-cl50-weth"), 17_725_259_725_360_589_554);
    assert_eq!(out("aero-cl100-weth"), 17_722_004_408_122_825_130);
    assert_eq!(out("aero-usdt-weth"), 17_730_768_618_618_146_014);
    assert_eq!(out("uniswap-v4-direct"), 17_168_660_568_388_805_278);
    assert_eq!(out("uniswap-v3-weth"), 17_722_085_413_951_150_619);
    assert_eq!(out("uniswap-v4-eth"), 16_927_635_343_644_656_443);

    // The Metaquoter's answer for the basic pool is the pool's own getAmountOut, and
    // that is x*y=k on the reserves read at the same block with the 0.30% fee.
    let fx = quotes_fixture();
    let own = decode_amount_out(&classified(&fx, "basic_pool_getAmountOut").unwrap()).unwrap();
    assert_eq!(own, out("aero-basic-direct"));
    let reserves = classified(&fx, "basic_pool_getReserves").unwrap();
    let (r0, r1) = (decode_small_word(&reserves[..64]).unwrap(), decode_small_word(&reserves[64..128]).unwrap());
    let fee_bps = aero_basic_fee_bps(fee_word(&fx, "fee_basic_custom"), fee_word(&fx, "fee_basic_default"));
    assert_eq!(fee_bps, 30);
    let net = TRANCHE_RAW - TRANCHE_RAW * fee_bps / 10_000;
    assert_eq!(net * r1 / (r0 + net), own);
}

#[test]
fn test_replay_of_the_first_plan_buy_matches_its_receipt_to_the_wei() {
    // Issue #115's replay check: quote 20 USDC along fill 1's recorded route at the
    // block before fill 1, and compare with what the wallet received.
    let replay = fixture(include_str!("../testdata/quotes/replay_fill1_block_57228847.json"));
    assert_eq!(replay["block"].as_str(), Some("0x3693e2f"));
    assert_eq!(i64::from_str_radix("3693e2f", 16).unwrap(), 57228848 - 1);
    // The same request the worker builds for that route today.
    let (to, data) = quote_request(&spec("aero-usdt-weth"), TRANCHE_RAW);
    assert!(to.eq_ignore_ascii_case(replay["to"].as_str().unwrap()));
    assert_eq!(data, replay["data"].as_str().unwrap());
    let quoted = match outcome_from_call(&spec("aero-usdt-weth"), classify_quote_call(&replay["response"])) {
        QuoteOutcome::Quoted(o) => o,
        QuoteOutcome::Unavailable(why) => panic!("{why}"),
    };

    let receipt = fixture(include_str!("../testdata/fills/receipt_0x5d0c02d992.json"));
    let receipt = receipt.get("result").cloned().unwrap_or(receipt);
    let (usdc, nock) = parse_accumulation_swap(&receipt, "0xeE114cC97eC5adeB34bA5645e315B497D8487a45").expect("fill 1 is a purchase");
    assert_eq!(usdc, TRANCHE_RAW);
    assert_eq!(quoted, nock, "the quote one block earlier is what the swap returned");
    assert_eq!(nock, 15_979_340_475_636_968_138);
}

#[test]
fn test_block_header_gives_the_quote_block_and_time() {
    assert_eq!(block_time().to_rfc3339(), "2026-11-16T05:25:57+00:00");
    // A header without a number or a timestamp is no block, never "now".
    assert_eq!(parse_block_header(&json!({ "result": { "timestamp": "0x6afa93e5" } })), None);
    assert_eq!(parse_block_header(&json!({ "result": { "number": "0x3764276" } })), None);
    assert_eq!(parse_block_header(&json!({ "result": null })), None);
    assert_eq!(parse_block_header(&json!({ "error": { "code": -32000, "message": "header not found" } })), None);
}

// ---------------- The comparison ----------------

#[test]
fn test_each_venue_shows_its_best_verified_route_with_hops_and_fees() {
    let q = real_quotes();
    assert_eq!(q.status, "ok");
    assert_eq!(q.detail, None);
    assert_eq!(q.amount_usdc, "20.00");
    assert_eq!(q.block_number, Some(QUOTE_BLOCK));
    assert_eq!(q.quoted_at.as_deref(), Some("2026-11-16T05:25:57+00:00"));
    assert_eq!(q.reference_spot_usd.as_deref(), Some("0.01128675"));
    assert_eq!(q.venues.iter().map(|v| v.venue.as_str()).collect::<Vec<_>>(), vec!["Aero", "Uniswap"]);

    let aero = &q.venues[0];
    assert_eq!(aero.status, "quoted");
    assert_eq!(aero.route.as_deref(), Some("USDC → USDT → WETH → NOCK"));
    assert_eq!(aero.nock_out.as_deref(), Some("1773.0768618618146014"));
    assert_eq!(aero.effective_price_usd.as_deref(), Some("0.01127983"));
    // Below the reference spot: the reference is one pool's mid price, not the market.
    assert_eq!(aero.cost_over_spot_pct.as_deref(), Some("-0.06"));
    assert_eq!(
        aero.hops.iter().map(|h| (h.token_in.as_str(), h.token_out.as_str(), h.pool.as_str(), h.fee_pct.as_deref())).collect::<Vec<_>>(),
        vec![
            ("USDC", "USDT", "0xd448990883e3bf9cdd8ebca368a3ecf9d629aaf7", Some("0.0008")),
            ("USDT", "WETH", "0x2dbe0e264c7c246e05c6614794899937f264cf7b", Some("0.0468")),
            ("WETH", "NOCK", "0x957256c98d61a714081de2ee70ae3a96c9b40874", Some("0.97")),
        ]
    );
    assert!(aero.hops.iter().all(|h| h.pool_kind == "Aero concentrated"));
    // Every candidate is listed with its own amount, not only the winner.
    assert_eq!(
        aero.routes_tried.iter().map(|r| (r.route_id.as_str(), r.nock_out.as_deref())).collect::<Vec<_>>(),
        vec![
            ("aero-basic-direct", Some("1742.2508413591799568")),
            ("aero-cl50-weth", Some("1772.5259725360589554")),
            ("aero-cl100-weth", Some("1772.200440812282513")),
            ("aero-usdt-weth", Some("1773.0768618618146014")),
        ]
    );

    let uni = &q.venues[1];
    assert_eq!(uni.status, "quoted");
    assert_eq!(uni.route.as_deref(), Some("USDC → WETH → NOCK"));
    assert_eq!(uni.nock_out.as_deref(), Some("1772.2085413951150619"));
    assert_eq!(uni.effective_price_usd.as_deref(), Some("0.01128535"));
    assert_eq!(uni.cost_over_spot_pct.as_deref(), Some("-0.01"));
    assert_eq!(
        uni.hops.iter().map(|h| (h.pool_kind.as_str(), h.pool.as_str(), h.fee_pct.as_deref())).collect::<Vec<_>>(),
        vec![
            ("Uniswap v3", "0xd0b53d9277642d899df5c87a3966a349a798f224", Some("0.05")),
            ("Uniswap v3", "0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a", Some("1")),
        ]
    );
    // The direct v4 3% pool pays its 3% fee: about 3.2% over the same spot.
    let direct = uni.routes_tried.iter().find(|r| r.route_id == "uniswap-v4-direct").unwrap();
    assert_eq!(direct.nock_out.as_deref(), Some("1716.8660568388805278"));
    let direct_price = quote_effective_price(TRANCHE_RAW, 17_168_660_568_388_805_278).unwrap();
    assert_eq!(format!("{:.2}", cost_over_spot_pct(direct_price, Some(real_spot())).unwrap()), "3.21");
}

#[test]
fn test_comparison_names_the_venue_that_returns_more_nock_as_a_fact() {
    let q = real_quotes();
    assert_eq!(q.more_nock_venue.as_deref(), Some("Aero"));
    assert_eq!(q.more_nock_by_pct.as_deref(), Some("0.05"));
    assert_eq!(
        q.comparison,
        "Aero returns more NOCK by 0.05% for $20.00 USDC at block 58081910: 1773.0768 NOCK against 1772.2085 on Uniswap."
    );
    // A statement of two reads, never advice.
    let lower = q.comparison.to_lowercase();
    for word in ["recommend", "should", "best", "cheaper", "buy on"] {
        assert!(!lower.contains(word), "{word:?} in {:?}", q.comparison);
    }
    assert_eq!(best_venue_quote(&q).map(|v| v.venue.as_str()), Some("Aero"));
}

#[test]
fn test_swap_links_go_to_each_venues_own_swap_page_and_prefill_nothing() {
    let q = real_quotes();
    assert_eq!(q.venues[0].swap_url, "https://app.aero.xyz/swap");
    assert_eq!(q.venues[1].swap_url, "https://app.uniswap.org/swap");
    for v in &q.venues {
        let host = v.swap_url.trim_start_matches("https://").split('/').next().unwrap();
        assert!(host == "app.aero.xyz" || host == "app.uniswap.org", "{host}");
        assert!(!v.swap_url.contains('?') && !v.swap_url.contains("0x"), "nothing is prefilled: {}", v.swap_url);
    }
    // Unavailable venues keep their link; the retired app is never linked.
    let down = route_quotes_unavailable("20.00", "no Alchemy RPC key is configured", None);
    assert_eq!(down.venues[0].swap_url, "https://app.aero.xyz/swap");
    assert!(!serde_json::to_string(&(q, down)).unwrap().contains("aerodrome.finance"));
}

#[test]
fn test_one_quote_or_none_marks_no_venue() {
    let aero = real_quotes().venues[0].clone();
    let uni_down = VenueQuote {
        venue: "Uniswap".to_string(),
        status: "unavailable".to_string(),
        reason: Some("the quote request timed out".to_string()),
        swap_url: venue_swap_url("Uniswap").to_string(),
        ..Default::default()
    };
    let one = assemble_route_quotes("20.00", Some((QUOTE_BLOCK, block_time())), Some(real_spot()), vec![aero.clone(), uni_down.clone()]);
    assert_eq!(one.status, "partial");
    assert_eq!(one.detail.as_deref(), Some("Uniswap: the quote request timed out"));
    assert_eq!((one.more_nock_venue.clone(), one.more_nock_by_pct.clone()), (None, None));
    assert_eq!(one.comparison, "Only Aero could be quoted at block 58081910; with one quote there is nothing to compare.");
    // The one quote is still the stored reference for a later fill.
    assert_eq!(best_venue_quote(&one).map(|v| v.venue.as_str()), Some("Aero"));

    let none = route_quotes_unavailable("20.00", "the latest block could not be read, so no quote was requested", None);
    assert_eq!(none.status, "unavailable");
    assert_eq!(none.detail.as_deref(), Some("the latest block could not be read, so no quote was requested"));
    assert_eq!((none.block_number, none.quoted_at.clone(), none.reference_spot_usd.clone()), (None, None, None));
    assert_eq!(none.comparison, "Neither venue could be quoted, so there is nothing to compare.");
    assert_eq!((none.more_nock_venue.clone(), none.more_nock_by_pct.clone()), (None, None));
    assert_eq!(best_venue_quote(&none), None);
    for v in &none.venues {
        // Unavailable means no number at all: not a zero, not an estimate.
        assert_eq!(v.status, "unavailable");
        assert_eq!((v.nock_out.clone(), v.effective_price_usd.clone(), v.cost_over_spot_pct.clone(), v.route.clone()), (None, None, None, None));
        assert!(v.hops.is_empty());
    }

    // Equal amounts name nobody; a hair's difference is not rounded up to a figure.
    let mut same = aero.clone();
    same.venue = "Uniswap".to_string();
    let (venue, pct, text) = compare_venues(&[aero.clone(), same.clone()], "20.00", Some(QUOTE_BLOCK));
    assert_eq!((venue, pct), (None, None));
    assert_eq!(text, "Aero and Uniswap return the same NOCK for $20.00 USDC at block 58081910: 1773.0768.");
    same.nock_out = Some("1773.0768618618146015".to_string());
    let (venue, pct, text) = compare_venues(&[aero, same], "20.00", Some(QUOTE_BLOCK));
    assert_eq!((venue.as_deref(), pct.as_deref()), (Some("Uniswap"), Some("0.00")));
    assert!(text.starts_with("Uniswap returns more NOCK by less than 0.01% for $20.00 USDC"), "{text}");
}

// ---------------- Failures are "unavailable", never a number ----------------

#[test]
fn test_a_reverting_quoter_is_unavailable_with_its_reason() {
    let fx = quotes_fixture();
    // Uniswap QuoterV2 on the v3 USDC/NOCK 1% pool, which holds no liquidity.
    assert_eq!(
        classified(&fx, "uni_v3_usdc_nock_1pct_no_liquidity"),
        Err(QuoteCallFailure::Reverted(Some("Unexpected error".to_string())))
    );
    // V4Quoter on a pool key that was never initialized: a bare revert with error data.
    assert_eq!(classified(&fx, "uni_v4_usdc_nock_uninitialized"), Err(QuoteCallFailure::Reverted(None)));
    // Aero Metaquoter asked to put USDC into a pool that does not hold it.
    assert_eq!(classified(&fx, "aero_wrong_token_in"), Err(QuoteCallFailure::Reverted(Some("TI".to_string()))));

    let v3 = outcome_from_call(&spec("uniswap-v3-weth"), classified(&fx, "uni_v3_usdc_nock_1pct_no_liquidity"));
    assert_eq!(
        v3,
        QuoteOutcome::Unavailable("the quoter reverted (Unexpected error): this route cannot fill the amount at this block".to_string())
    );
    let v4 = outcome_from_call(&spec("uniswap-v4-direct"), classified(&fx, "uni_v4_usdc_nock_uninitialized"));
    assert_eq!(v4, QuoteOutcome::Unavailable("the quoter reverted: this route cannot fill the amount at this block".to_string()));

    // A venue whose every route reverted is unavailable; the other venue is unaffected.
    let mut results = real_results();
    for r in results.iter_mut().filter(|r| r.spec.venue == VENUE_UNISWAP) {
        r.outcome = v4.clone();
    }
    let uni = build_venue_quote(VENUE_UNISWAP, &results, &[], TRANCHE_RAW, Some(real_spot()));
    assert_eq!(uni.status, "unavailable");
    assert_eq!(
        uni.reason.as_deref(),
        Some("none of its 3 verified route(s) could be quoted: the quoter reverted: this route cannot fill the amount at this block")
    );
    assert_eq!((uni.nock_out.clone(), uni.effective_price_usd.clone(), uni.cost_over_spot_pct.clone()), (None, None, None));
    assert_eq!(uni.routes_tried.len(), 3);
    assert!(uni.routes_tried.iter().all(|r| r.nock_out.is_none() && r.reason.is_some()));
    let aero = build_venue_quote(VENUE_AERO, &results, &real_fees("aero-usdt-weth"), TRANCHE_RAW, Some(real_spot()));
    assert_eq!(aero.status, "quoted");

    // One failed route does not take the venue down: the best of the rest stands.
    let mut results = real_results();
    results.iter_mut().find(|r| r.spec.id == "aero-usdt-weth").unwrap().outcome =
        QuoteOutcome::Unavailable("the quote request timed out".to_string());
    let best = best_route_index(&results, VENUE_AERO).unwrap();
    assert_eq!(results[best].spec.id, "aero-cl50-weth");
    let aero = build_venue_quote(VENUE_AERO, &results, &real_fees("aero-cl50-weth"), TRANCHE_RAW, Some(real_spot()));
    assert_eq!(aero.nock_out.as_deref(), Some("1772.5259725360589554"));
    let failed = aero.routes_tried.iter().find(|r| r.route_id == "aero-usdt-weth").unwrap();
    assert_eq!((failed.nock_out.clone(), failed.reason.as_deref()), (None, Some("the quote request timed out")));

    // A venue with no route in the table has no verified quoter.
    let unknown = build_venue_quote("Elsewhere", &results, &[], TRANCHE_RAW, None);
    assert_eq!((unknown.status.as_str(), unknown.reason.as_deref()), ("unavailable", Some("quoter not verified")));
}

#[test]
fn test_a_pool_with_no_liquidity_never_yields_a_quote() {
    let fx = quotes_fixture();
    // The v3 USDC/NOCK 1% pool exists (the factory resolves it) and is empty.
    assert_eq!(decode_small_word(&classified(&fx, "v3_usdc_nock_1pct_liquidity").unwrap()), Some(0));
    assert!(classified(&fx, "uni_v3_usdc_nock_1pct_no_liquidity").is_err());

    // A pool that runs dry part-way: 5,000,000 USDC through the v3 WETH/NOCK pool.
    // QuoterV2 answers with the NOCK for the part that did swap and says nothing else;
    // only the post-swap price, pinned one step inside the range's end, shows it.
    let ex = exhausted_fixture();
    let v3_hex = classified(&ex, "uni_v3_usdc_weth_nock_exhausted").unwrap();
    assert!(decode_amount_out(&v3_hex).is_ok(), "the raw answer does carry an amount");
    assert_eq!(pool_exhausted(&v3_hex), Some(true));
    assert_eq!(
        outcome_from_call(&spec("uniswap-v3-weth"), Ok(v3_hex)),
        QuoteOutcome::Unavailable("a pool on this route ran out of liquidity before the whole amount was swapped".to_string())
    );
    // The V4Quoter reverts in the same situation.
    assert_eq!(classified(&ex, "uni_v4_usdc_nock_3pct_exhausted"), Err(QuoteCallFailure::Reverted(None)));
    // The Aero pool holds full-range liquidity: it fills any size at a collapsing
    // price. That is a real quote, and the real quote for 20 USDC is not flagged.
    assert_eq!(pool_exhausted(&classified(&ex, "aero_cl50_weth_cl200_nock_exhausted").unwrap()), Some(false));
    for id in ["aero-basic-direct", "aero-cl50-weth", "aero-cl100-weth", "aero-usdt-weth", "uniswap-v3-weth"] {
        assert_eq!(pool_exhausted(&classified(&fx, fixture_key(id)).unwrap()), Some(false), "{id}");
    }
    // A basic-pool hop reports a zero price: skipped, not mistaken for the range's end.
    let basic = classified(&fx, "aero_basic_usdc_nock").unwrap();
    assert_eq!(&basic[4 * 64..5 * 64], &"0".repeat(64));
}

#[test]
fn test_answers_that_are_not_an_amount_are_errors_not_zero() {
    assert_eq!(decode_amount_out(""), Err("the quoter's answer was too short to hold an amount".to_string()));
    assert_eq!(decode_amount_out("0x"), Err("the quoter's answer was too short to hold an amount".to_string()));
    assert_eq!(decode_amount_out(&"0".repeat(64)), Err("the quoter returned zero NOCK".to_string()));
    assert_eq!(decode_amount_out(&format!("{}{}", "f".repeat(32), "0".repeat(32))), Err("the quoter's answer does not fit an amount".to_string()));
    assert_eq!(decode_amount_out(&format!("{:064x}", 7u128)), Ok(7));
    // Not hex at all (a multi-byte character where a digit should be): an error, no panic.
    assert_eq!(decode_amount_out(&format!("{}é", "0".repeat(63))), Err("the quoter's answer was not hex".to_string()));
    assert_eq!(decode_small_word(&format!("{}é", "0".repeat(63))), None);
    assert_eq!(pool_exhausted(&format!("{}é", "0".repeat(127))), None);
    // A two-word V4Quoter answer is never read as a price list.
    let fx = quotes_fixture();
    let v4 = classified(&fx, "uni_v4_usdc_nock_3pct").unwrap();
    assert_eq!(v4.len(), 128);
    assert_eq!(pool_exhausted(&v4), None);
    assert_eq!(outcome_from_call(&spec("uniswap-v4-direct"), Ok(v4)), QuoteOutcome::Quoted(17_168_660_568_388_805_278));
    // The same bytes offered as an Aero answer are refused rather than half-read.
    assert_eq!(
        outcome_from_call(&spec("aero-cl50-weth"), Ok(format!("{:064x}{:064x}", 7u128, 0xa848u128))),
        QuoteOutcome::Unavailable("the quoter's answer was not understood".to_string())
    );

    // RPC failures are not reverts, and their text never comes from a transport error.
    assert_eq!(
        classify_quote_call(&json!({ "error": { "code": -32000, "message": "header not found" } })),
        Err(QuoteCallFailure::NoAnswer("the RPC answered with error -32000: header not found".to_string()))
    );
    assert_eq!(
        classify_quote_call(&json!({ "id": 1 })),
        Err(QuoteCallFailure::NoAnswer("the RPC's answer had neither a result nor an error".to_string()))
    );
    assert_eq!(
        outcome_from_call(&spec("aero-cl50-weth"), Err(QuoteCallFailure::NoAnswer("the quote request timed out".to_string()))),
        QuoteOutcome::Unavailable("the quote request timed out".to_string())
    );
}

// ---------------- Small pure helpers ----------------

#[test]
fn test_amounts_and_fees_are_exact_decimals() {
    assert_eq!(units_to_raw("20.00", 6), Some(20_000_000));
    assert_eq!(units_to_raw("20", 6), Some(20_000_000));
    assert_eq!(units_to_raw("17.5", 6), Some(17_500_000));
    assert_eq!(units_to_raw("0.000001", 6), Some(1));
    assert_eq!(units_to_raw("1773.0768618618146014", 16), Some(17_730_768_618_618_146_014));
    for bad in ["", ".", "-20.00", "20.0000001", "1e3", "20,00", "abc", "+5"] {
        assert_eq!(units_to_raw(bad, 6), None, "{bad:?}");
    }
    assert_eq!(nock_4dp(17_730_768_618_618_146_014), "1773.0768");
    assert_eq!(nock_4dp(10u128.pow(16)), "1.0000");
    assert_eq!(nock_4dp(123), "0.0000");

    // Aero concentrated pools and Uniswap report millionths; the basic pool, basis points.
    assert_eq!(fee_pct_from_millionths(9700), "0.97");
    assert_eq!(fee_pct_from_millionths(8), "0.0008");
    assert_eq!(fee_pct_from_millionths(500), "0.05");
    assert_eq!(fee_pct_from_millionths(10_000), "1");
    assert_eq!(fee_pct_from_millionths(30_000), "3");
    assert_eq!(fee_pct_from_bps(30), "0.3");
    assert_eq!(aero_basic_fee_bps(0, 30), 30);
    assert_eq!(aero_basic_fee_bps(100, 30), 100);
    assert_eq!(aero_basic_fee_bps(AERO_ZERO_FEE_INDICATOR, 30), 0);
    let fx = quotes_fixture();
    assert_eq!(fee_word(&fx, "fee_basic_zero_indicator"), AERO_ZERO_FEE_INDICATOR);
    assert_eq!(fee_word(&fx, "fee_cl200_nock"), 9700);

    assert_eq!(quote_effective_price(0, 5), None);
    assert_eq!(quote_effective_price(5, 0), None);
    assert_eq!(cost_over_spot_pct(0.0113, None), None);
    assert_eq!(cost_over_spot_pct(0.0113, Some(0.0)), None);
    assert_eq!(cost_over_spot_pct(0.0113, Some(f64::NAN)), None);
}

#[test]
fn test_fee_unit_of_aero_concentrated_pools_is_millionths() {
    // A tiny WETH→NOCK quote on the Aero pool against its slot0 price: out equals
    // in × price × (1 − fee()/1e6) to within the price impact of 1e12 wei.
    let fx = fixture(include_str!("../testdata/quotes/cl_fee_unit_check_block_58081910.json"));
    let fee = fx["fee"].as_u64().unwrap();
    assert_eq!(fee, 9700);
    let slot0 = fx["slot0"]["result"].as_str().unwrap().trim_start_matches("0x");
    let price = sqrt_price_x96_to_raw(U256::from_hex_be(&slot0[..64]).unwrap());
    let out = decode_amount_out(fx["single_v3_call"]["response"]["result"].as_str().unwrap()).unwrap() as f64;
    let expected = |denominator: f64| 1e12 * price * (1.0 - fee as f64 / denominator);
    assert!((out / expected(1e6) - 1.0).abs() < 1e-6, "millionths: {}", out / expected(1e6));
    assert!((out / expected(1e4) - 1.0).abs() > 1.0, "not basis points");
}

// ---------------- Only the plan's own due tranche is ever quoted ----------------

#[test]
fn test_quotes_are_requested_only_for_a_due_tranche_of_the_plans_own_size() {
    let now = Utc::now();
    assert_eq!(tranche_to_quote(true, Some(&due(20.0))), Some(("20.00".to_string(), 20_000_000)));
    // A tranche the guardrails shrank is quoted at the shrunk size: the amount advised.
    assert_eq!(tranche_to_quote(true, Some(&due(13.37))), Some(("13.37".to_string(), 13_370_000)));
    // Switched off, or no plan: nothing.
    assert_eq!(tranche_to_quote(false, Some(&due(20.0))), None);
    assert_eq!(tranche_to_quote(true, None), None);
    // Every other status: nothing, whatever the nominal tranche is.
    for directive in [
        PlanDirective::Waiting { next_due_at: now },
        PlanDirective::Pending { starts_at: now },
        PlanDirective::Paused { reason: "overextended".to_string() },
        PlanDirective::Blocked { reason: "nothing spendable".to_string() },
        PlanDirective::Invalid { reason: "bad".to_string() },
        PlanDirective::Complete,
    ] {
        let ps = plan_status(&directive);
        assert_eq!(ps.nominal_tranche_usdc, "20.00");
        assert_eq!(tranche_to_quote(true, Some(&ps)), None, "{}", ps.status);
    }
    // A due status without a positive amount is not quoted either.
    let mut odd = due(20.0);
    odd.tranche_usdc = "0.00".to_string();
    assert_eq!(tranche_to_quote(true, Some(&odd)), None);
    odd.tranche_usdc = "n/a".to_string();
    assert_eq!(tranche_to_quote(true, Some(&odd)), None);
    // The status field decides, not the advice text (a floor breach prefixes the action).
    let mut waiting = plan_status(&PlanDirective::Waiting { next_due_at: now });
    waiting.tranche_usdc = "20.00".to_string();
    assert_eq!(tranche_to_quote(true, Some(&waiting)), None);

    // A fresh plan status carries no quotes: they are attached only by the cycle.
    assert_eq!(due(20.0).route_quotes, None);
}

#[test]
fn test_refresh_is_refused_unless_a_tranche_is_due() {
    let code = |r: Option<(StatusCode, &'static str, &'static str)>| r.map(|(status, error, _)| (status, error));
    let d = due(20.0);
    assert_eq!(code(refresh_refusal(true, true, true, Some(&d))), None);
    assert_eq!(code(refresh_refusal(false, true, true, Some(&d))), Some((StatusCode::CONFLICT, "NO_PLAN")));
    assert_eq!(code(refresh_refusal(true, false, true, Some(&d))), Some((StatusCode::CONFLICT, "QUOTES_DISABLED")));
    assert_eq!(code(refresh_refusal(true, true, false, None)), Some((StatusCode::SERVICE_UNAVAILABLE, "NO_SNAPSHOT")));
    assert_eq!(code(refresh_refusal(true, true, true, None)), Some((StatusCode::CONFLICT, "NO_TRANCHE_DUE")));
    let waiting = plan_status(&PlanDirective::Waiting { next_due_at: Utc::now() });
    let refused = refresh_refusal(true, true, true, Some(&waiting)).unwrap();
    assert_eq!((refused.0, refused.1), (StatusCode::CONFLICT, "NO_TRANCHE_DUE"));
    assert!(refused.2.contains("never another size"));
}

#[tokio::test]
async fn test_no_call_is_made_without_a_due_tranche_or_an_rpc_key() {
    // The pool is lazy and the key is the placeholder: any database or network use
    // would fail the test rather than pass silently.
    let state_with = |yaml: &str| -> Arc<AppState> {
        let mut config: AppConfig = serde_yaml::from_str(yaml).unwrap();
        config.accumulation_plan = Some(plan_cfg());
        Arc::new(AppState {
            db: PgPool::connect_lazy("postgres://nobody@127.0.0.1:1/none").unwrap(),
            http_client: Client::new(),
            config: Arc::new(config),
            latest_snapshot: Mutex::new(None),
            last_sync_duration_ms: std::sync::atomic::AtomicU64::new(0),
            last_ingest_error: std::sync::Mutex::new(None),
        cycle_gate: CycleGate::default(),
        quote_refresh_at: std::sync::Mutex::new(None),
        })
    };
    let state = state_with("alchemy_api_keys: [\"YOUR_ALCHEMY_API_KEY\"]\n");
    assert!(state.config.route_quotes.enabled, "on by default");

    // Not due: None, nothing asked.
    let waiting = plan_status(&PlanDirective::Waiting { next_due_at: Utc::now() });
    assert_eq!(quote_due_tranche(&state, &waiting, Some(0.0113)).await, None);

    // Due, but no RPC key: both venues unavailable with the reason, no numbers.
    let q = quote_due_tranche(&state, &due(20.0), Some(0.0113)).await.expect("a tranche is due");
    assert_eq!(q.status, "unavailable");
    assert_eq!(q.detail.as_deref(), Some("no Alchemy RPC key is configured"));
    assert_eq!(q.amount_usdc, "20.00");
    assert_eq!((q.block_number, q.more_nock_venue.clone()), (None, None));
    assert!(q.venues.iter().all(|v| v.status == "unavailable" && v.nock_out.is_none()));

    // Switched off: None even though a tranche is due.
    let off = state_with("alchemy_api_keys: [\"YOUR_ALCHEMY_API_KEY\"]\nroute_quotes:\n  enabled: false\n");
    assert!(!off.config.route_quotes.enabled);
    assert_eq!(quote_due_tranche(&off, &due(20.0), Some(0.0113)).await, None);

    // The refresh endpoint refuses before any read: no snapshot here.
    let resp = refresh_route_quotes_handler(Extension(state.clone())).await;
    assert_eq!(resp.status(), StatusCode::SERVICE_UNAVAILABLE);
    let resp = refresh_route_quotes_handler(Extension(off)).await;
    assert_eq!(resp.status(), StatusCode::CONFLICT);
}

#[tokio::test]
async fn test_refresh_quotes_the_snapshots_tranche_and_attaches_the_answer() {
    async fn body_json(resp: Response) -> (StatusCode, serde_json::Value) {
        let status = resp.status();
        let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20).await.unwrap();
        (status, serde_json::from_slice(&bytes).unwrap())
    }
    let snapshot_with = |plan: AccumulationPlanStatus| SnapshotEnvelope {
        meta: SnapshotMeta {
            generated_at: Utc::now(),
            state_hash: "test".to_string(),
            version: SNAPSHOT_SPEC_VERSION.to_string(),
            snapshot_id: Uuid::new_v4(),
        },
        data: SnapshotData {
            closed_positions: vec![],
            wallet_balances: vec![],
            positions: vec![],
            pool_mechanics: vec![],
            concentrated_liquidity: vec![],
            contract_fees: vec![],
            derived_metrics: DerivedMetrics { accumulation_plan: Some(plan), ..Default::default() },
        },
    };
    // No RPC key and a lazy pool: the handler runs end to end without a network or a
    // database, and the answer is "unavailable", which is still an answer.
    let mut config: AppConfig = serde_yaml::from_str("alchemy_api_keys: [\"YOUR_ALCHEMY_API_KEY\"]\n").unwrap();
    config.accumulation_plan = Some(plan_cfg());
    let mut stale = due(20.0);
    stale.route_quotes = Some(real_quotes());
    let state = Arc::new(AppState {
        db: PgPool::connect_lazy("postgres://nobody@127.0.0.1:1/none").unwrap(),
        http_client: Client::new(),
        config: Arc::new(config),
        latest_snapshot: Mutex::new(Some(snapshot_with(stale))),
        last_sync_duration_ms: std::sync::atomic::AtomicU64::new(0),
        last_ingest_error: std::sync::Mutex::new(None),
        cycle_gate: CycleGate::default(),
        quote_refresh_at: std::sync::Mutex::new(None),
    });

    let (status, body) = body_json(refresh_route_quotes_handler(Extension(state.clone())).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["plan_id"].as_str(), Some("2026-11-reentry"));
    assert_eq!(body["tranche_index"].as_u64(), Some(1));
    assert_eq!(body["route_quotes"]["amount_usdc"].as_str(), Some("20.00"));
    assert_eq!(body["route_quotes"]["status"].as_str(), Some("unavailable"));
    assert_eq!(body["route_quotes"]["detail"].as_str(), Some("no Alchemy RPC key is configured"));
    // The snapshot being served now says so too: the earlier quotes are not left
    // standing as if they were current.
    let attached = state.latest_snapshot.lock().await.as_ref().unwrap().data.derived_metrics.accumulation_plan.clone().unwrap();
    let q = attached.route_quotes.expect("the refresh's answer is attached");
    assert_eq!((q.status.as_str(), q.block_number), ("unavailable", None));
    assert!(q.venues.iter().all(|v| v.nock_out.is_none()));

    // Once the plan is no longer due there is nothing to quote, whatever was attached.
    let mut waiting = plan_status(&PlanDirective::Waiting { next_due_at: Utc::now() });
    waiting.route_quotes = Some(real_quotes());
    *state.latest_snapshot.lock().await = Some(snapshot_with(waiting));
    let (status, body) = body_json(refresh_route_quotes_handler(Extension(state.clone())).await).await;
    assert_eq!((status, body["error"].as_str()), (StatusCode::CONFLICT, Some("NO_TRANCHE_DUE")));
}

// ---------------- Snapshot behaviour ----------------

#[test]
fn test_quotes_never_force_a_new_snapshot_row() {
    let mut with = due(20.0);
    with.route_quotes = Some(real_quotes());
    let mut later = with.clone();
    let q = later.route_quotes.as_mut().unwrap();
    q.block_number = Some(QUOTE_BLOCK + 1500);
    q.quoted_at = Some("2026-11-16T05:30:57+00:00".to_string());
    q.venues[0].nock_out = Some("1771.5".to_string());
    q.comparison = "Uniswap returns more NOCK".to_string();
    let canonical = |ps: &AccumulationPlanStatus| {
        let mut v = json!({ "derived_metrics": { "accumulation_plan": ps } });
        strip_volatile_time_fields(&mut v);
        v
    };
    assert_ne!(with, later);
    assert_eq!(canonical(&with), canonical(&later), "quotes are left out of the state hash");
    assert_eq!(canonical(&with), canonical(&due(20.0)));
    // The rest of the plan status still counts.
    assert_ne!(canonical(&with), canonical(&due(19.0)));

    // A stored snapshot from before v0.7.0 has no such field and still parses.
    let mut old = serde_json::to_value(due(20.0)).unwrap();
    old.as_object_mut().unwrap().remove("route_quotes");
    assert_eq!(serde_json::from_value::<AccumulationPlanStatus>(old).unwrap().route_quotes, None);
    // And a snapshot with quotes round-trips.
    let round: AccumulationPlanStatus = serde_json::from_value(serde_json::to_value(&with).unwrap()).unwrap();
    assert_eq!(round, with);
}

#[test]
fn test_advice_points_to_the_quotes_without_a_number() {
    const POINTER: &str =
        " Read-only route quotes for this amount on Aero and on Uniswap, or why a venue could not be quoted, are listed with the plan.";
    let q = real_quotes();
    let details = "Buy $20.00 of NOCK on Aero (spot $0.01129).".to_string();
    let pointed = with_route_quote_pointer("DCA TRANCHE DUE (3 of 5)", details.clone(), Some(&q));
    assert_eq!(pointed, format!("{details}{POINTER}"));
    // Idempotent, and prefixed actions (a floor breach) still match.
    assert_eq!(with_route_quote_pointer("DEFENSIVE LOCK · DCA TRANCHE DUE (3 of 5)", pointed.clone(), Some(&q)), pointed);
    // The advice text is part of the state hash, so it must not depend on what the
    // quoters answered: one venue, none, or both give the same sentence. A quote
    // coming or going then never stores a snapshot row.
    let down = route_quotes_unavailable("20.00", "no Alchemy RPC key is configured", None);
    assert_eq!(with_route_quote_pointer("DCA TRANCHE DUE (3 of 5)", details.clone(), Some(&down)), pointed);
    let mut one = real_quotes();
    one.venues[1].status = "unavailable".to_string();
    assert_eq!(with_route_quote_pointer("DCA TRANCHE DUE (3 of 5)", details.clone(), Some(&one)), pointed);
    // Quotes not asked for (nothing due, or switched off), or another directive in
    // charge: the text is untouched.
    assert_eq!(with_route_quote_pointer("DCA TRANCHE DUE (3 of 5)", details.clone(), None), details);
    assert_eq!(with_route_quote_pointer("BATCH HARVEST", details.clone(), Some(&q)), details);
    // The sentence carries no number and is true when a venue could not be quoted.
    assert!(!POINTER.chars().any(|c| c.is_ascii_digit()));
    assert!(POINTER.contains("or why a venue could not be quoted"));
}

#[test]
fn test_route_quotes_payload_is_pinned() {
    // The golden snapshot leaves `route_quotes` out (it is not part of the state hash),
    // so its shape is pinned here: what the worker would have put in the snapshot at
    // block 58081910, byte for byte. The dashboard's tests use the same payload.
    // Regenerate only for an intended change:
    //   NOCKNOCK_BLESS_QUOTES=1 cargo test test_route_quotes_payload_is_pinned
    let actual = format!("{}\n", serde_json::to_string_pretty(&real_quotes()).unwrap());
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/quotes/route_quotes_payload_block_58081910.json");
    if std::env::var("NOCKNOCK_BLESS_QUOTES").is_ok() {
        std::fs::write(path, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(path).expect("the pinned payload is checked in");
    assert!(actual == expected, "route_quotes payload changed; compare {path} with a NOCKNOCK_BLESS_QUOTES=1 run");
}

// ---------------- A fill's cost over its tranche's quote ----------------

/// The block time of the pinned quotes: 2026-11-16T05:25:57Z.
fn quote_time() -> DateTime<Utc> {
    DateTime::parse_from_rfc3339("2026-11-16T05:25:57Z").unwrap().with_timezone(&Utc)
}

/// A stored quote read for the plan's first tranche at the pinned quote time.
fn stored(block: i64, price: &str) -> StoredQuote {
    StoredQuote {
        tranche_index: 1,
        venue: "Aero".to_string(),
        route: "USDC → USDT → WETH → NOCK".to_string(),
        amount_usdc: "20.00".to_string(),
        nock_out: "1773.0768618618146014".to_string(),
        effective_price_usd: price.to_string(),
        block_number: block,
        quoted_at: quote_time(),
    }
}

/// A fill `minutes` after the pinned quote time.
fn fill_at(block: i64, minutes: i64, previous_buy_block: Option<i64>, earlier_fills: i64) -> FillPosition {
    FillPosition { block, time: quote_time() + chrono::Duration::minutes(minutes), previous_buy_block, earlier_fills }
}

#[test]
fn test_a_fill_is_measured_only_against_a_quote_from_before_it_in_its_own_tranche() {
    let quote = |block: i64, tranche_index: i32| StoredQuote { tranche_index, ..stored(block, "0.01127983") };
    // Read before the fill's block, for its tranche: applies.
    assert!(quote_applies_to_fill(&quote(100, 1), &fill_at(101, 0, None, 0)));
    assert!(quote_applies_to_fill(&quote(100, 2), &fill_at(5000, 5, Some(50), 1)));
    // Read at the fill's own block: that state already includes the fill.
    assert!(!quote_applies_to_fill(&quote(101, 1), &fill_at(101, 0, None, 0)));
    // A later quote is never used.
    assert!(!quote_applies_to_fill(&quote(102, 1), &fill_at(101, 0, None, 0)));
    // Read before the plan's previous fill: it belonged to the tranche before.
    assert!(!quote_applies_to_fill(&quote(100, 1), &fill_at(5000, 5, Some(200), 1)));
    assert!(!quote_applies_to_fill(&quote(100, 2), &fill_at(5000, 5, Some(200), 1)));

    // The plan keeps saying tranche 1 is due after buy 1 lands at block 200, until
    // that buy is recorded, and keeps storing tranche-1 quotes (here at blocks 200 and
    // 260). Once buy 1 is recorded they never serve buy 2, however recent they are.
    for leftover in [200, 260] {
        assert!(!quote_applies_to_fill(&quote(leftover, 1), &fill_at(5000, 5, Some(200), 1)), "{leftover}");
    }
    // A quote the plan read for tranche 2, after it knew of buy 1, does.
    assert!(quote_applies_to_fill(&quote(260, 2), &fill_at(5000, 5, Some(200), 1)));
    // A tranche the ledger has not reached is not this fill's either.
    assert!(!quote_applies_to_fill(&quote(260, 3), &fill_at(5000, 5, Some(200), 1)));
    // An earlier swap still on offer is a buy before this one, but not a recorded
    // fill: a quote read between the two, while the plan still said tranche 1, is the
    // one that was on screen before this buy.
    assert!(quote_applies_to_fill(&quote(260, 1), &fill_at(5000, 5, Some(200), 0)));
    assert!(!quote_applies_to_fill(&quote(100, 1), &fill_at(5000, 5, Some(200), 0)));

    // Age: a quote more than an hour older than the fill is price movement, not cost.
    assert_eq!(FILL_QUOTE_MAX_AGE_MINUTES, 60);
    assert!(quote_applies_to_fill(&quote(100, 1), &fill_at(5000, 60, None, 0)));
    assert!(!quote_applies_to_fill(&quote(100, 1), &fill_at(5000, 61, None, 0)));
    assert!(!quote_applies_to_fill(&quote(100, 1), &fill_at(5000, 3 * 24 * 60, None, 0)));
}

#[test]
fn test_cost_over_quote_is_all_or_nothing() {
    // The fill paid 0.5% more per NOCK than the quote said.
    let r = fill_quote_reference("0.01133623", Some(stored(QUOTE_BLOCK, "0.01127983")));
    assert_eq!(r.cost_bps, Some(50));
    assert_eq!(r.venue.as_deref(), Some("Aero"));
    assert_eq!(r.route.as_deref(), Some("USDC → USDT → WETH → NOCK"));
    assert_eq!((r.usdc.as_deref(), r.block_number), (Some("20.00"), Some(QUOTE_BLOCK)));
    assert_eq!(r.effective_price_usd.as_deref(), Some("0.01127983"));
    assert!(r.quoted_at.is_some());
    // A fill that did better than the quote is a negative figure, not hidden.
    assert_eq!(fill_quote_reference("0.01122343", Some(stored(QUOTE_BLOCK, "0.01127983"))).cost_bps, Some(-50));
    // No quote on record, or one that cannot be used: every field is None.
    assert_eq!(fill_quote_reference("0.01133623", None), FillQuoteReference::default());
    assert_eq!(fill_quote_reference("0.01133623", Some(stored(QUOTE_BLOCK, "0"))), FillQuoteReference::default());
    assert_eq!(fill_quote_reference("0.01133623", Some(stored(QUOTE_BLOCK, "n/a"))), FillQuoteReference::default());
    assert_eq!(fill_quote_reference("not a price", Some(stored(QUOTE_BLOCK, "0.01127983"))), FillQuoteReference::default());

    // Replayed on the plan's first buy: quoted one block earlier along the route it
    // took, the fill cost nothing over its quote.
    let replay = fixture(include_str!("../testdata/quotes/replay_fill1_block_57228847.json"));
    let quoted = decode_amount_out(replay["response"]["result"].as_str().unwrap()).unwrap();
    let quote_price = format!("{:.8}", quote_effective_price(TRANCHE_RAW, quoted).unwrap());
    assert_eq!(quote_price, "0.01251616");
    assert_eq!(fill_quote_reference("0.01251616", Some(stored(57228847, &quote_price))).cost_bps, Some(0));
}

// ---------------- Text: both MCP transports ----------------

fn snapshot_with_quotes(quotes: Option<RouteQuotes>) -> serde_json::Value {
    let mut ps = due(20.0);
    ps.route_quotes = quotes;
    json!({ "derived_metrics": { "accumulation_plan": ps, "lp_rebuild": { "status": "on_target" } } })
}

#[test]
fn test_plan_text_shows_both_venues_and_says_what_a_quote_is() {
    let data = snapshot_with_quotes(Some(real_quotes()));
    let text = format_accumulation_plan_text(&data);
    let lines = route_quote_lines(&data["derived_metrics"]["accumulation_plan"]);
    assert!(text.contains(&lines), "the worker's plan text carries the shared lines verbatim");
    assert_eq!(
        lines,
        "• Route quotes for $20.00 USDC → NOCK at block 58081910 (2026-11-16T05:25:57+00:00), reference spot $0.01128675:\n   \
         Aero: 1773.0768 NOCK at $0.01127983 (0.06% under the reference spot) via USDC → USDT → WETH → NOCK\n      \
         USDC→USDT Aero concentrated 0xd448990883e3bf9cdd8ebca368a3ecf9d629aaf7 fee 0.0008%\n      \
         USDT→WETH Aero concentrated 0x2dbe0e264c7c246e05c6614794899937f264cf7b fee 0.0468%\n      \
         WETH→NOCK Aero concentrated 0x957256c98d61a714081de2ee70ae3a96c9b40874 fee 0.97%\n      \
         swap page: https://app.aero.xyz/swap (enter $20.00 USDC → NOCK yourself; nothing is prefilled)\n   \
         Uniswap: 1772.2085 NOCK at $0.01128535 (0.01% under the reference spot) via USDC → WETH → NOCK\n      \
         USDC→WETH Uniswap v3 0xd0b53d9277642d899df5c87a3966a349a798f224 fee 0.05%\n      \
         WETH→NOCK Uniswap v3 0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a fee 1%\n      \
         swap page: https://app.uniswap.org/swap (enter $20.00 USDC → NOCK yourself; nothing is prefilled)\n   \
         Aero returns more NOCK by 0.05% for $20.00 USDC at block 58081910: 1773.0768 NOCK against 1772.2085 on Uniswap.\n   \
         Quotes are contract reads at the stated block: estimates as of that time, not a promise. The wallet app's own router may pick a different route. This compares execution routes for a buy your plan already sized; it is not a recommendation to buy.\n"
    );
    assert!(!text.contains("aerodrome.finance"));
}

#[test]
fn test_plan_text_says_unavailable_with_the_reason_and_nothing_when_not_due() {
    let down = snapshot_with_quotes(Some(route_quotes_unavailable("20.00", "no Alchemy RPC key is configured", None)));
    let lines = route_quote_lines(&down["derived_metrics"]["accumulation_plan"]);
    assert!(lines.contains("at no block (nothing was read), no reference spot:"), "{lines}");
    assert!(lines.contains("   Aero: unavailable (no Alchemy RPC key is configured)\n"), "{lines}");
    assert!(lines.contains("   Uniswap: unavailable (no Alchemy RPC key is configured)\n"), "{lines}");
    assert!(lines.contains("Neither venue could be quoted, so there is nothing to compare."), "{lines}");
    assert!(!lines.contains(" NOCK at $"), "no figure for an unavailable venue: {lines}");

    // A hop whose fee read failed says so; the quote stands.
    let mut q = real_quotes();
    q.venues[0].hops[1].fee_pct = None;
    q.venues[1].cost_over_spot_pct = None;
    let lines = route_quote_lines(&snapshot_with_quotes(Some(q))["derived_metrics"]["accumulation_plan"]);
    assert!(lines.contains("USDT→WETH Aero concentrated 0x2dbe0e264c7c246e05c6614794899937f264cf7b fee not read\n"), "{lines}");
    assert!(lines.contains("Uniswap: 1772.2085 NOCK at $0.01128535 (cost over spot unavailable)"), "{lines}");

    // The cost reads in words: over, under, or at the reference spot.
    let mut q = real_quotes();
    q.venues[0].cost_over_spot_pct = Some("3.21".to_string());
    q.venues[1].cost_over_spot_pct = Some("-0.00".to_string());
    let lines = route_quote_lines(&snapshot_with_quotes(Some(q))["derived_metrics"]["accumulation_plan"]);
    assert!(lines.contains("Aero: 1773.0768 NOCK at $0.01127983 (3.21% over the reference spot)"), "{lines}");
    assert!(lines.contains("Uniswap: 1772.2085 NOCK at $0.01128535 (at the reference spot)"), "{lines}");

    // No quotes in the snapshot (nothing due, switched off, or an older worker): no line.
    for none in [snapshot_with_quotes(None), json!({ "derived_metrics": { "accumulation_plan": { "status": "waiting" } } })] {
        assert_eq!(route_quote_lines(&none["derived_metrics"]["accumulation_plan"]), "");
        assert!(!format_accumulation_plan_text(&none).contains("Route quotes"));
    }
}

#[test]
fn test_route_quotes_config_is_on_by_default_and_has_one_switch() {
    let default: AppConfig = serde_yaml::from_str("alchemy_api_keys: []\n").unwrap();
    assert!(default.route_quotes.enabled);
    let off: AppConfig = serde_yaml::from_str("route_quotes:\n  enabled: false\n").unwrap();
    assert!(!off.route_quotes.enabled);
    let empty: AppConfig = serde_yaml::from_str("route_quotes: {}\n").unwrap();
    assert!(empty.route_quotes.enabled);
    // No amount, venue or route can be configured: the only key is the switch.
    assert_eq!(serde_json::to_value(RouteQuotesConfig::default()).unwrap(), json!({ "enabled": true }));
}

// ---------------- SQL, against a scratch database ----------------
// The queries in route_quotes.rs and the new ledger columns are runtime strings, so this
// test runs them for real. It is #[ignore]d like the fill-detection one, and refuses any
// database whose name does not end in "_scratch":
//   NOCKNOCK_SCRATCH_DATABASE_URL=postgres://… cargo test scratch_database -- --ignored

#[tokio::test]
#[ignore = "needs NOCKNOCK_SCRATCH_DATABASE_URL (a migrated scratch PostgreSQL)"]
async fn test_route_quote_sql_against_a_scratch_database() {
    let url = std::env::var("NOCKNOCK_SCRATCH_DATABASE_URL").expect("NOCKNOCK_SCRATCH_DATABASE_URL is set");
    let db = PgPool::connect(&url).await.expect("scratch database reachable");
    let name: String = sqlx::query("SELECT current_database() AS n").fetch_one(&db).await.unwrap().get("n");
    assert!(name.ends_with("_scratch"), "refusing to run against database {name:?}: its name must end in _scratch");

    let plan_id = "scratch-route-quotes";
    // A wallet of this test's own, so no other test's candidates are touched.
    const SCRATCH_WALLET: &str = "0x00000000000000000000000000000000000115aa";
    let cleanup = |db: PgPool| async move {
        sqlx::query("DELETE FROM accumulation_quotes WHERE plan_id = $1").bind(plan_id).execute(&db).await.unwrap();
        sqlx::query("DELETE FROM accumulation_events WHERE plan_id = $1").bind(plan_id).execute(&db).await.unwrap();
        sqlx::query("DELETE FROM accumulation_candidates WHERE wallet = $1").bind(SCRATCH_WALLET).execute(&db).await.unwrap();
    };
    cleanup(db.clone()).await;

    // The pinned quotes re-dated: `blocks` after the quote block, `minutes` after its time.
    let quotes_at = |blocks: i64, minutes: i64| {
        let mut q = real_quotes();
        q.block_number = Some(QUOTE_BLOCK + blocks);
        q.quoted_at = Some((quote_time() + chrono::Duration::minutes(minutes)).to_rfc3339());
        q
    };
    let at = |minutes: i64| quote_time() + chrono::Duration::minutes(minutes);
    let lookup = |blocks: i64, minutes: i64| {
        let db = db.clone();
        async move { quote_for_fill(&db, plan_id, SCRATCH_WALLET, QUOTE_BLOCK + blocks, at(minutes)).await }
    };

    // Nothing stored: no quote on record.
    assert_eq!(lookup(10, 1).await, None);

    // A failed comparison stores nothing; a real one stores one row, once.
    let down = route_quotes_unavailable("20.00", "no Alchemy RPC key is configured", Some(real_spot()));
    assert!(!store_route_quotes(&db, plan_id, 1, &down).await.unwrap());
    let q = real_quotes();
    assert!(store_route_quotes(&db, plan_id, 1, &q).await.unwrap());
    assert!(!store_route_quotes(&db, plan_id, 1, &q).await.unwrap(), "one row per plan and block");
    let kept: serde_json::Value =
        sqlx::query("SELECT quotes FROM accumulation_quotes WHERE plan_id = $1").bind(plan_id).fetch_one(&db).await.unwrap().get("quotes");
    assert_eq!(serde_json::from_value::<RouteQuotes>(kept).unwrap(), q, "the whole comparison is kept as shown");

    // A fill after the quote finds it; a fill at or before the quote's block does not.
    let found = lookup(10, 1).await.expect("quote before the fill");
    assert_eq!((found.venue.as_str(), found.block_number, found.tranche_index), ("Aero", QUOTE_BLOCK, 1));
    assert_eq!(found.route, "USDC → USDT → WETH → NOCK");
    assert_eq!((found.amount_usdc.as_str(), found.nock_out.as_str()), ("20.00", "1773.0768618618146014"));
    assert_eq!(found.effective_price_usd, "0.01127983");
    assert_eq!(found.quoted_at.to_rfc3339(), "2026-11-16T05:25:57+00:00");
    assert_eq!(lookup(0, 1).await, None);
    assert_eq!(lookup(-1, 0).await, None);
    assert_eq!(quote_for_fill(&db, "another-plan", SCRATCH_WALLET, QUOTE_BLOCK + 10, at(1)).await, None);
    // A quote more than an hour older than the fill is not its reference.
    assert!(lookup(10, 60).await.is_some());
    assert_eq!(lookup(10, 61).await, None);

    // Record a fill with its quote columns, as the record endpoint writes them.
    let insert_fill = |tx: &'static str, block: i64, r: FillQuoteReference| {
        let db = db.clone();
        async move {
            sqlx::query(
                "INSERT INTO accumulation_events \
                    (plan_id, tx_hash, venue, usdc_spent_raw, nock_received_raw, usdc_spent, nock_received, effective_price_usd, \
                     block_number, confirmed_at, quote_venue, quote_route, quote_usdc, quote_nock_out, \
                     quote_effective_price_usd, quote_block_number, quoted_at, quote_cost_bps) \
                 VALUES ($1, $2, 'Aero', '20000000', '17642000000000000000', '20', '1764.2', '0.01133623', $3, NOW(), \
                         $4, $5, $6, $7, $8, $9, $10, $11)",
            )
            .bind(plan_id)
            .bind(tx)
            .bind(block)
            .bind(&r.venue)
            .bind(&r.route)
            .bind(&r.usdc)
            .bind(&r.nock_out)
            .bind(&r.effective_price_usd)
            .bind(r.block_number)
            .bind(r.quoted_at)
            .bind(r.cost_bps)
            .execute(&db)
            .await
            .unwrap();
        }
    };

    // Buy 1 lands at +10. Until it is recorded the plan still says tranche 1 is due
    // and the cycle keeps storing tranche-1 quotes, here at +300 (ten minutes on).
    assert!(store_route_quotes(&db, plan_id, 1, &quotes_at(300, 10)).await.unwrap());
    // Buy 1 itself is measured against the quote read before it, not the later one.
    let first = fill_quote_reference("0.01133623", lookup(10, 1).await);
    assert_eq!((first.cost_bps, first.block_number), (Some(50), Some(QUOTE_BLOCK)));
    insert_fill("0xscratchquotefill1", QUOTE_BLOCK + 10, first).await;
    let row = sqlx::query(
        "SELECT quote_venue, quote_route, quote_usdc, quote_nock_out, quote_effective_price_usd, quote_block_number, \
                quoted_at, quote_cost_bps FROM accumulation_events WHERE tx_hash = '0xscratchquotefill1'",
    )
    .fetch_one(&db)
    .await
    .unwrap();
    assert_eq!(row.get::<Option<String>, _>("quote_venue").as_deref(), Some("Aero"));
    assert_eq!(row.get::<Option<i64>, _>("quote_block_number"), Some(QUOTE_BLOCK));
    assert_eq!(row.get::<Option<i32>, _>("quote_cost_bps"), Some(50));
    assert!(row.get::<Option<DateTime<Utc>>, _>("quoted_at").is_some());

    // Buy 2 is made early at +600, twenty minutes after the first quote: the plan was
    // waiting, so nothing was quoted for tranche 2. The latest stored quote (+300) is
    // recent and was read after buy 1, but it was read for tranche 1. It is not buy
    // 2's reference: "no quote on record", never the leftover figure.
    assert_eq!(lookup(600, 20).await, None);
    // The same buy days later (the old row is also far too old).
    assert_eq!(lookup(500_000, 3 * 24 * 60).await, None);
    let none = fill_quote_reference("0.01133623", None);
    insert_fill("0xscratchquotefill2", QUOTE_BLOCK + 500_000, none).await;
    let row = sqlx::query("SELECT quote_venue, quote_cost_bps FROM accumulation_events WHERE tx_hash = '0xscratchquotefill2'")
        .fetch_one(&db)
        .await
        .unwrap();
    assert_eq!((row.get::<Option<String>, _>("quote_venue"), row.get::<Option<i32>, _>("quote_cost_bps")), (None, None));
    sqlx::query("DELETE FROM accumulation_events WHERE tx_hash = '0xscratchquotefill2'").execute(&db).await.unwrap();
    // Had tranche 2 come due and been quoted (+550, fifteen minutes on), buy 2 at +600
    // takes that quote.
    assert!(store_route_quotes(&db, plan_id, 2, &quotes_at(550, 15)).await.unwrap());
    let second = lookup(600, 20).await.expect("the quote read for tranche 2");
    assert_eq!((second.block_number, second.tranche_index), (QUOTE_BLOCK + 550, 2));

    // An earlier swap still on offer is a buy before this one too: a fill recorded
    // ahead of it does not take the quote that belonged to the offered swap's tranche.
    sqlx::query("DELETE FROM accumulation_events WHERE plan_id = $1").bind(plan_id).execute(&db).await.unwrap();
    sqlx::query("DELETE FROM accumulation_quotes WHERE plan_id = $1 AND block_number > $2")
        .bind(plan_id)
        .bind(QUOTE_BLOCK)
        .execute(&db)
        .await
        .unwrap();
    assert!(lookup(900, 5).await.is_some());
    sqlx::query(
        "INSERT INTO accumulation_candidates (wallet, tx_hash, state, block_number, confirmed_at) \
         VALUES ($1, '0xscratchquoteoffer', 'offered', $2, NOW())",
    )
    .bind(SCRATCH_WALLET)
    .bind(QUOTE_BLOCK + 10)
    .execute(&db)
    .await
    .unwrap();
    assert_eq!(lookup(900, 5).await, None);
    // The offered swap itself still finds its quote, and another wallet's offers do not count.
    assert!(lookup(10, 1).await.is_some());
    assert!(quote_for_fill(&db, plan_id, "0x00000000000000000000000000000000000115bb", QUOTE_BLOCK + 900, at(5)).await.is_some());

    // The events endpoint's own query reads the new columns.
    sqlx::query(
        "SELECT plan_id, tx_hash, venue, usdc_spent, nock_received, effective_price_usd, block_number, confirmed_at, \
                reference_spot_usd, reference_spot_source, execution_cost_bps, \
                quote_venue, quote_route, quote_usdc, quote_nock_out, quote_effective_price_usd, quote_block_number, \
                quoted_at, quote_cost_bps \
         FROM accumulation_events ORDER BY confirmed_at ASC, id ASC",
    )
    .fetch_all(&db)
    .await
    .unwrap();

    cleanup(db).await;
}

// ---------------- Live, against a real RPC ----------------
// The whole read path (latest block, seven quoter calls at that block, fee reads) for
// the amount given, printed as the plan text would show it. Read-only. #[ignore]d: it
// needs the network and its numbers change every block.
//   NOCKNOCK_LIVE_RPC_URL=https://base-rpc.publicnode.com cargo test live_route_quotes -- --ignored --nocapture

#[tokio::test]
#[ignore = "needs NOCKNOCK_LIVE_RPC_URL (a Base mainnet JSON-RPC endpoint)"]
async fn test_live_route_quotes_against_a_real_rpc() {
    let url = std::env::var("NOCKNOCK_LIVE_RPC_URL").expect("NOCKNOCK_LIVE_RPC_URL is set");
    let spot = std::env::var("NOCKNOCK_LIVE_SPOT_USD").ok().and_then(|s| s.parse::<f64>().ok());
    let q = fetch_route_quotes(&Client::new(), &url, "20.00", TRANCHE_RAW, spot).await;
    println!("{}", route_quote_lines(&json!({ "route_quotes": q })));
    println!("{}", serde_json::to_string_pretty(&q).unwrap());
    assert!(q.block_number.is_some(), "{:?}", q.detail);
    assert_eq!(q.venues.len(), 2);
    for v in &q.venues {
        assert_eq!(v.routes_tried.len(), if v.venue == "Aero" { 4 } else { 3 });
        if v.status == "quoted" {
            // 20 USDC of NOCK at a price between $0.001 and $0.1: a sanity band for a
            // live read, not a rule the worker applies.
            let out: f64 = v.nock_out.as_ref().unwrap().parse().unwrap();
            assert!(out > 200.0 && out < 20_000.0, "{out}");
            assert!(v.hops.iter().all(|h| h.fee_pct.is_some()), "{:?}", v.hops);
        } else {
            assert!(v.reason.is_some() && v.nock_out.is_none());
        }
    }
}

// ---------------- The read path, against the captured answers ----------------
// A local JSON-RPC server that answers each request with the response captured on Base
// at block 58081910 (the same fixture the pure tests use), optionally slowly. Nothing
// leaves the machine. Unknown calls get an RPC error, never an invented answer.

#[derive(Clone, Copy, Default)]
struct MockDelays {
    block: std::time::Duration,
    fees: std::time::Duration,
}

type SeenCalls = Arc<std::sync::Mutex<Vec<(String, String)>>>;

/// Start the server; returns its URL and the (method, block-or-selector) of each request.
async fn captured_rpc(delays: MockDelays) -> (String, SeenCalls) {
    let fx = Arc::new(quotes_fixture());
    let seen: SeenCalls = Arc::new(std::sync::Mutex::new(Vec::new()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a local port");
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handler = {
        let seen = seen.clone();
        move |Json(req): Json<serde_json::Value>| {
            let (fx, seen) = (fx.clone(), seen.clone());
            async move {
                let method = req["method"].as_str().unwrap_or_default().to_string();
                if method == "eth_getBlockByNumber" {
                    seen.lock().unwrap().push((method, req["params"][0].as_str().unwrap_or_default().to_string()));
                    tokio::time::sleep(delays.block).await;
                    return Json(fx["header"]["response"].clone());
                }
                let to = req["params"][0]["to"].as_str().unwrap_or_default().to_lowercase();
                let data = req["params"][0]["data"].as_str().unwrap_or_default().to_lowercase();
                seen.lock().unwrap().push((method, req["params"][1].as_str().unwrap_or_default().to_string()));
                let is_fee_read = [POOL_FEE, AERO_CUSTOM_FEE, AERO_DEFAULT_FEE].iter().any(|s| data.starts_with(s));
                if is_fee_read {
                    tokio::time::sleep(delays.fees).await;
                }
                let captured = fx["calls"].as_object().unwrap().values().find(|c| {
                    c["to"].as_str().map(|t| t.to_lowercase()) == Some(to.clone())
                        && c["data"].as_str().map(|d| d.to_lowercase()) == Some(data.clone())
                });
                Json(match captured {
                    Some(c) => c["response"].clone(),
                    None => json!({ "jsonrpc": "2.0", "id": 1, "error": { "code": -32000, "message": "no captured answer for this call" } }),
                })
            }
        }
    };
    let app = axum::Router::new().route("/", axum::routing::post(handler));
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    (url, seen)
}

#[tokio::test]
async fn test_the_read_path_on_captured_answers_gives_the_pinned_comparison() {
    let (url, seen) = captured_rpc(MockDelays::default()).await;
    let q = fetch_route_quotes(&Client::new(), &url, "20.00", TRANCHE_RAW, Some(real_spot())).await;
    // Block header, seven quoter calls, the fee reads: the same object the pure
    // assembly gives, which is the pinned payload.
    assert_eq!(q, real_quotes());
    let seen = seen.lock().unwrap().clone();
    assert_eq!(seen.iter().filter(|(m, _)| m == "eth_getBlockByNumber").count(), 1);
    assert_eq!(seen[0], ("eth_getBlockByNumber".to_string(), "latest".to_string()));
    // 7 routes + the 3 dynamic fees of Aero's best route (Uniswap's best has fixed
    // fees), every one an eth_call pinned to the block that was read.
    let calls: Vec<_> = seen.iter().filter(|(m, _)| m == "eth_call").collect();
    assert_eq!(calls.len(), 10, "{seen:?}");
    assert!(calls.iter().all(|(_, block)| block == "0x3764276"), "{seen:?}");
    assert_eq!(seen.len(), 11, "nothing but the block read and eth_call: {seen:?}");
}

#[tokio::test]
async fn test_slow_fee_reads_never_discard_quotes_that_were_read() {
    // The fee reads answer after 3 s; their budget here is 200 ms.
    let slow = MockDelays { fees: std::time::Duration::from_secs(3), ..Default::default() };
    let (url, _) = captured_rpc(slow).await;
    let budgets = QuoteBudgets { fees: std::time::Duration::from_millis(200), ..Default::default() };
    let started = std::time::Instant::now();
    let q = fetch_route_quotes_within(&Client::new(), &url, "20.00", TRANCHE_RAW, Some(real_spot()), &budgets).await;
    assert!(started.elapsed() < slow.fees, "the fee reads were not waited for: {:?}", started.elapsed());

    // Every quote stands, exactly as read; only the fees that were not read are missing.
    let real = real_quotes();
    assert_eq!((q.status.as_str(), q.block_number), ("ok", Some(QUOTE_BLOCK)));
    assert_eq!(q.comparison, real.comparison);
    assert_eq!(q.more_nock_venue.as_deref(), Some("Aero"));
    for (got, want) in q.venues.iter().zip(&real.venues) {
        assert_eq!((got.status.as_str(), &got.nock_out, &got.effective_price_usd), ("quoted", &want.nock_out, &want.effective_price_usd));
        assert_eq!(got.cost_over_spot_pct, want.cost_over_spot_pct);
        assert_eq!(got.routes_tried, want.routes_tried);
        assert_eq!(got.hops.len(), want.hops.len());
    }
    // Aero's best route has three dynamic fees: none was read, none is invented.
    assert!(q.venues[0].hops.iter().all(|h| h.fee_pct.is_none()), "{:?}", q.venues[0].hops);
    // Uniswap's fees are fixed by the pools' identity and need no read.
    assert_eq!(q.venues[1].hops, real.venues[1].hops);
    assert!(route_quote_lines(&json!({ "route_quotes": q })).contains("fee not read"));

    // The budgets fit inside the cap that abandons a check, with the block read before
    // and the fee reads after the slowest possible quote call.
    let d = QuoteBudgets::default();
    assert_eq!(d.attempts, QUOTE_REQUEST_ATTEMPTS);
    let quotes_worst_ms = (d.request.as_millis() as u64) * (d.attempts as u64) + 200 + 400;
    assert!(d.block_read.as_millis() as u64 + quotes_worst_ms + (d.fees.as_millis() as u64) < QUOTE_TIMEOUT_SECONDS * 1000);
}

#[tokio::test]
async fn test_fee_reads_cover_every_kind_of_hop_and_a_failed_one_is_not_a_default() {
    let (url, seen) = captured_rpc(MockDelays::default()).await;
    // Only the basic pool was quoted on Aero and only the v4 pool on Uniswap: the fees
    // read are the basic pool's (fee module and factory, two reads) and a fixed one.
    let mut results = real_results();
    for r in results.iter_mut() {
        if r.spec.id != "aero-basic-direct" && r.spec.id != "uniswap-v4-direct" {
            r.outcome = QuoteOutcome::Unavailable("the quote request timed out".to_string());
        }
    }
    let fees = read_best_route_fees(&Client::new(), &url, &results, "0x3764276", &QuoteBudgets::default()).await;
    assert_eq!(fees, vec![real_fees("aero-basic-direct"), real_fees("uniswap-v4-direct")]);
    assert_eq!(fees, vec![vec![Some("0.3".to_string())], vec![Some("3".to_string())]]);
    assert_eq!(seen.lock().unwrap().len(), 2, "a fixed fee needs no read");

    // Nothing quoted on a venue: no hops, no reads.
    for r in results.iter_mut() {
        r.outcome = QuoteOutcome::Unavailable("the quote request timed out".to_string());
    }
    let none = read_best_route_fees(&Client::new(), &url, &results, "0x3764276", &QuoteBudgets::default()).await;
    assert_eq!(none, vec![Vec::<Option<String>>::new(), Vec::new()]);

    // An endpoint that does not answer: the dynamic fees are None, tried once each,
    // and the fixed ones are still stated.
    let dead = "http://127.0.0.1:1";
    let budgets = QuoteBudgets { fees: std::time::Duration::from_millis(500), ..Default::default() };
    let fees = read_best_route_fees(&Client::new(), dead, &real_results(), "0x3764276", &budgets).await;
    assert_eq!(fees[0], vec![None, None, None]);
    assert_eq!(fees[1], real_fees("uniswap-v3-weth"));
}

#[tokio::test]
async fn test_a_block_read_that_stalls_gives_no_quote_and_keeps_the_cycles_spot() {
    let (url, seen) = captured_rpc(MockDelays { block: std::time::Duration::from_secs(3), ..Default::default() }).await;
    let budgets = QuoteBudgets { block_read: std::time::Duration::from_millis(150), ..Default::default() };
    let q = fetch_route_quotes_within(&Client::new(), &url, "20.00", TRANCHE_RAW, Some(real_spot()), &budgets).await;
    assert_eq!(q.status, "unavailable");
    assert_eq!(q.detail.as_deref(), Some("the latest block could not be read, so no quote was requested"));
    assert_eq!((q.block_number, q.quoted_at.clone()), (None, None));
    assert!(q.venues.iter().all(|v| v.status == "unavailable" && v.nock_out.is_none() && v.cost_over_spot_pct.is_none()));
    // Without a block no quoter is asked.
    assert!(seen.lock().unwrap().iter().all(|(m, _)| m == "eth_getBlockByNumber"));
    // The cycle's own spot is carried at full precision for a later refresh.
    assert_eq!(q.reference_spot_usd.as_deref(), Some("0.01128675"));
}

// ---------------- Refresh reference, the latest endpoint, a restart ----------------

fn envelope_with(plan: Option<AccumulationPlanStatus>, displayed_price: &str) -> SnapshotEnvelope {
    let mut derived_metrics = DerivedMetrics { accumulation_plan: plan, ..Default::default() };
    derived_metrics.tick_bracket_hypervisor.current_price = displayed_price.to_string();
    SnapshotEnvelope {
        meta: SnapshotMeta {
            generated_at: Utc::now(),
            state_hash: "test".to_string(),
            version: SNAPSHOT_SPEC_VERSION.to_string(),
            snapshot_id: Uuid::new_v4(),
        },
        data: SnapshotData {
            closed_positions: vec![],
            wallet_balances: vec![],
            positions: vec![],
            pool_mechanics: vec![],
            concentrated_liquidity: vec![],
            contract_fees: vec![],
            derived_metrics,
        },
    }
}

/// A worker with a plan, no RPC key and a database that is never reached.
fn keyless_state(snapshot: Option<SnapshotEnvelope>) -> Arc<AppState> {
    let mut config: AppConfig = serde_yaml::from_str("alchemy_api_keys: [\"YOUR_ALCHEMY_API_KEY\"]\n").unwrap();
    config.accumulation_plan = Some(plan_cfg());
    Arc::new(AppState {
        db: PgPool::connect_lazy("postgres://nobody@127.0.0.1:1/none").unwrap(),
        http_client: Client::new(),
        config: Arc::new(config),
        latest_snapshot: Mutex::new(snapshot),
        last_sync_duration_ms: std::sync::atomic::AtomicU64::new(0),
        last_ingest_error: std::sync::Mutex::new(None),
        cycle_gate: CycleGate::default(),
        quote_refresh_at: std::sync::Mutex::new(None),
    })
}

async fn response_json(resp: Response) -> (StatusCode, HeaderMap, serde_json::Value) {
    let (status, headers) = (resp.status(), resp.headers().clone());
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    (status, headers, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
async fn test_a_refresh_measures_against_the_cycles_spot_or_against_nothing() {
    // The cycle's quotes carry its spot even when no venue could be quoted.
    let timed_out = route_quotes_unavailable("20.00", "the quotes took longer than 30 seconds and were abandoned", Some(0.0112867512));
    assert_eq!(timed_out.reference_spot_usd.as_deref(), Some("0.01128675"));
    assert!(timed_out.venues.iter().all(|v| v.nock_out.is_none() && v.cost_over_spot_pct.is_none()));
    let mut plan = due(20.0);
    assert_eq!(refresh_reference_spot(&plan), None, "no quotes from a cycle, no reference");
    plan.route_quotes = Some(timed_out);
    assert_eq!(refresh_reference_spot(&plan), Some(0.01128675));
    plan.route_quotes = Some(real_quotes());
    assert_eq!(refresh_reference_spot(&plan), Some(0.01128675));
    plan.route_quotes = Some(route_quotes_unavailable("20.00", "no Alchemy RPC key is configured", None));
    assert_eq!(refresh_reference_spot(&plan), None);

    // End to end: the snapshot displays $0.01129 (five decimals). A refresh keeps the
    // cycle's eight-decimal figure and never falls back to the displayed one.
    plan.route_quotes = Some(route_quotes_unavailable("20.00", "stalled", Some(0.0112867512)));
    let state = keyless_state(Some(envelope_with(Some(plan), "0.01129")));
    let (status, _, body) = response_json(refresh_route_quotes_handler(Extension(state)).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(body["route_quotes"]["reference_spot_usd"].as_str(), Some("0.01128675"));
    // With no quotes from a cycle (just after a restart) there is no reference at all.
    let state = keyless_state(Some(envelope_with(Some(due(20.0)), "0.01129")));
    let (status, _, body) = response_json(refresh_route_quotes_handler(Extension(state)).await).await;
    assert_eq!(status, StatusCode::OK);
    assert!(body["route_quotes"]["reference_spot_usd"].is_null(), "{}", body["route_quotes"]);
}

#[tokio::test]
async fn test_latest_quotes_are_read_from_memory_outside_the_state_hash() {
    // No snapshot, or no plan: every field is null, and it is still a 200.
    for state in [keyless_state(None), keyless_state(Some(envelope_with(None, "0.01129")))] {
        let (status, _, body) = response_json(latest_route_quotes_handler(Extension(state)).await).await;
        assert_eq!(status, StatusCode::OK);
        assert_eq!(body, json!({ "plan_id": null, "status": null, "tranche_index": null, "tranche_usdc": null, "route_quotes": null }));
    }

    let mut plan = due(20.0);
    plan.route_quotes = Some(real_quotes());
    let state = keyless_state(Some(envelope_with(Some(plan), "0.01129")));
    let (status, headers, body) = response_json(latest_route_quotes_handler(Extension(state.clone())).await).await;
    assert_eq!(status, StatusCode::OK);
    assert_eq!(headers.get("cache-control").and_then(|v| v.to_str().ok()), Some("no-store"));
    assert!(headers.get("etag").is_none(), "no conditional answer: the state hash does not cover the quotes");
    assert_eq!((body["plan_id"].as_str(), body["status"].as_str()), (Some("2026-11-reentry"), Some("tranche_due")));
    assert_eq!((body["tranche_index"].as_u64(), body["tranche_usdc"].as_str()), (Some(1), Some("20.00")));
    assert_eq!(serde_json::from_value::<RouteQuotes>(body["route_quotes"].clone()).unwrap(), real_quotes());

    // A refresh changes the quotes and not the state hash; the endpoint answers with
    // the new ones at once (here "unavailable": this worker has no RPC key).
    let hash_before = state.latest_snapshot.lock().await.as_ref().unwrap().meta.state_hash.clone();
    let (status, _, _) = response_json(refresh_route_quotes_handler(Extension(state.clone())).await).await;
    assert_eq!(status, StatusCode::OK);
    let (_, _, body) = response_json(latest_route_quotes_handler(Extension(state.clone())).await).await;
    assert_eq!(body["route_quotes"]["status"].as_str(), Some("unavailable"));
    assert_eq!(body["route_quotes"]["detail"].as_str(), Some("no Alchemy RPC key is configured"));
    assert_eq!(state.latest_snapshot.lock().await.as_ref().unwrap().meta.state_hash, hash_before);

    // The pure body: nothing due means no quotes, whatever the plan is.
    let waiting = plan_status(&PlanDirective::Waiting { next_due_at: Utc::now() });
    let body = latest_route_quotes_body(Some(&waiting));
    assert_eq!(body["status"].as_str(), Some("waiting"));
    assert!(body["route_quotes"].is_null());
}

#[test]
fn test_a_snapshot_read_back_from_the_database_carries_no_quotes() {
    let mut plan = due(20.0);
    plan.route_quotes = Some(real_quotes());
    let mut data = envelope_with(Some(plan), "0.01129").data;
    let hash = compute_state_hash(&data);
    forget_stored_route_quotes(&mut data);
    let plan = data.derived_metrics.accumulation_plan.as_ref().unwrap();
    assert_eq!(plan.route_quotes, None);
    // The plan itself is untouched, and so is the state hash the row was stored under.
    assert_eq!((plan.status.as_str(), plan.tranche_usdc.as_str()), ("tranche_due", "20.00"));
    assert_eq!(compute_state_hash(&data), hash);
    // No plan: nothing to do.
    let mut none = envelope_with(None, "0.01129").data;
    forget_stored_route_quotes(&mut none);
    assert!(none.derived_metrics.accumulation_plan.is_none());
}

#[test]
fn test_both_mcp_transports_describe_the_quotes_in_the_plan_tool() {
    // The stdio binary cannot share code with the worker, so the two tool lists are
    // separate literals. The plan tool must say the same on both, quotes included.
    let clause = "while a tranche is due, read-only route quotes for that amount on Aero and Uniswap (a comparison, not a recommendation)";
    let plan_tool_line = |src: &str| {
        let at = src.find("\"name\": \"get_accumulation_plan\"").expect("the plan tool is listed");
        let rest = &src[at..];
        let line = rest.lines().find(|l| l.contains("\"description\"")).expect("it has a description");
        line.trim().to_string()
    };
    let http = plan_tool_line(include_str!("mcp_http.rs"));
    let stdio = plan_tool_line(include_str!("bin/mcp.rs"));
    assert!(http.contains(clause), "{http}");
    assert_eq!(http, stdio);
}
