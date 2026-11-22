//! Fill detection for the accumulation plan (issue #118).
//!
//! The fixtures under testdata/fills are real responses from Base mainnet, read on
//! 2026-11-15/16 for the wallet named in ONCHAIN_INTEGRATION.md (§10 records how each was
//! obtained). Three are plan buys, four are Uniswap fee claims, two are Aero liquidity
//! deposits that returned a NOCK refund, and one is a purchase from before the plan.

use super::*;
use std::collections::HashSet;

const WALLET: &str = "0xeE114cC97eC5adeB34bA5645e315B497D8487a45";

// The plan's three buys, oldest first.
const BUY_1: &str = "0x5d0c02d9929f49f71526e0c8d91d1bd99ec19f116befc285d67f6bf9412a3f0e";
const BUY_2: &str = "0x4644edd99bb2770f369862357eb5647a68a7157ca9e91ecccfce79e6964c61e8";
const BUY_3: &str = "0x78376f3cc9e80f2eb124a945a452df40556649e3d9983be526b2bd97586e31b9";
/// Block times of the three buys (ONCHAIN_INTEGRATION.md §10).
const BUY_1_AT: &str = "2026-11-14T06:02:25Z";
const BUY_2_AT: &str = "2026-11-15T02:58:46Z";
const BUY_3_AT: &str = "2026-11-16T03:08:37Z";
/// A real USDC→NOCK purchase from 2026-11-06, before the plan began.
const PRE_PLAN_BUY: &str = "0xa8cd45017a2f693fd7bb6c3129c4baffc73923fa3fe765393ca3c6d7249ce4dc";
const V3_CLAIMS: [&str; 2] = [
    "0x3127a1f419d3036d1f0b89dc67fd76e0af3841a25aa444587cb9c13278e49c9e",
    "0x1d5660018d280d320e6c07aa4edf292dcde0f6425627623499047f7857a374b1",
];
const V4_CLAIMS: [&str; 2] = [
    "0x929cb3e53fedfce94bc056558896f6f59b5e88fff55cdf68401efd36d7cc4fd0",
    "0x8f0b3cbb7669831181cabf9c3db8932780e0ac118a77f0834a86ee1756c78228",
];
/// Aero deposits (ONCHAIN_INTEGRATION.md §7): USDC and NOCK go in, a NOCK refund comes back.
const LP_DEPOSITS: [&str; 2] = [
    "0x127c4f4fd3d3d97f4711b4e193f2f26bc80877a2dd4408cf743c97fe9bf185f0",
    "0x80277e2bc5f351b66051967b44a47d22edd2704ad690cc5dc1f47e878cc93ddb",
];

fn t(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

/// The plan anchor used throughout: the day the plan's first buy was made.
fn buy_time(tx_hash: &str) -> DateTime<Utc> {
    match tx_hash {
        BUY_1 => t(BUY_1_AT),
        BUY_2 => t(BUY_2_AT),
        BUY_3 => t(BUY_3_AT),
        other => panic!("not one of the plan's buys: {other}"),
    }
}

fn anchor() -> DateTime<Utc> {
    t("2026-11-14T00:00:00Z")
}

fn receipt(tx_hash: &str) -> serde_json::Value {
    let raw = match &tx_hash[..12] {
        "0x5d0c02d992" => include_str!("../testdata/fills/receipt_0x5d0c02d992.json"),
        "0x4644edd99b" => include_str!("../testdata/fills/receipt_0x4644edd99b.json"),
        "0x78376f3cc9" => include_str!("../testdata/fills/receipt_0x78376f3cc9.json"),
        "0xa8cd45017a" => include_str!("../testdata/fills/receipt_0xa8cd45017a.json"),
        "0x3127a1f419" => include_str!("../testdata/fills/receipt_0x3127a1f419.json"),
        "0x1d5660018d" => include_str!("../testdata/fills/receipt_0x1d5660018d.json"),
        "0x929cb3e53f" => include_str!("../testdata/fills/receipt_0x929cb3e53f.json"),
        "0x8f0b3cbb76" => include_str!("../testdata/fills/receipt_0x8f0b3cbb76.json"),
        "0x127c4f4fd3" => include_str!("../testdata/fills/receipt_0x127c4f4fd3.json"),
        "0x80277e2bc5" => include_str!("../testdata/fills/receipt_0x80277e2bc5.json"),
        other => panic!("no receipt fixture for {other}"),
    };
    let r: serde_json::Value = serde_json::from_str(raw).unwrap();
    assert!(r["transactionHash"].as_str().unwrap().eq_ignore_ascii_case(tx_hash), "fixture is the named transaction");
    r
}

/// `eth_getBlockByNumber` response for a fixture block (trimmed to identity + timestamp).
fn block_header(block_number: i64) -> serde_json::Value {
    let all: serde_json::Value = serde_json::from_str(include_str!("../testdata/fills/block_headers.json")).unwrap();
    all.get(block_number.to_string()).cloned().unwrap_or(serde_json::Value::Null)
}

fn transfers_response() -> serde_json::Value {
    serde_json::from_str(include_str!("../testdata/fills/alchemy_getAssetTransfers_nock_in.json")).unwrap()
}

fn transfer_log(token: &str, from: &str, to: &str, amount: u128) -> serde_json::Value {
    let pad = |a: &str| format!("0x000000000000000000000000{}", a.trim_start_matches("0x").to_lowercase());
    json!({ "address": token, "topics": [TRANSFER_EVENT_TOPIC, pad(from), pad(to)], "data": format!("0x{:064x}", amount) })
}

// ---------------- Qualification: the record endpoint's rules ----------------

#[test]
fn test_real_plan_buys_qualify_with_the_amounts_the_swap_parser_returns() {
    // (hash, block, NOCK received raw, effective price). Each spent exactly 20 USDC.
    let cases = [
        (BUY_1, 57_228_848i64, 15_979_340_475_636_968_138u128, "0.01251616"),
        (BUY_2, 57_605_753, 17_202_181_466_216_939_446, "0.01162643"),
        (BUY_3, 58_040_708, 17_803_592_976_960_049_083, "0.01123369"),
    ];
    for (hash, block, nock_raw, price) in cases {
        let r = receipt(hash);
        let swap = verify_fill_receipt(&r, WALLET).unwrap_or_else(|e| panic!("{hash} must qualify: {e:?}"));
        assert_eq!(Some((swap.usdc_raw, swap.nock_raw)), parse_accumulation_swap(&r, WALLET));
        assert_eq!((swap.usdc_raw, swap.nock_raw), (20_000_000, nock_raw), "{hash}");
        assert_eq!(swap.usdc_spent, "20");
        assert_eq!(swap.nock_received, format_units(nock_raw, 16));
        assert_eq!(swap.effective_price_usd, price);
        assert_eq!(swap.block_number, block);

        let block_time = block_time_from_response(&block_header(block)).expect("header fixture has a timestamp");
        let fill = verify_fill(&r, WALLET, block_time, anchor()).unwrap();
        assert_eq!(fill.swap, swap);
        assert_eq!(fill.confirmed_at, block_time);
    }
    // BUY_1 is the transaction ONCHAIN_INTEGRATION.md §6 records at $0.01251616.
}

#[test]
fn test_real_fee_claims_and_lp_deposits_do_not_qualify() {
    for hash in V3_CLAIMS.iter().chain(V4_CLAIMS.iter()).chain(LP_DEPOSITS.iter()) {
        let r = receipt(hash);
        assert_eq!(r["status"], "0x1", "{hash} succeeded on-chain");
        assert_eq!(verify_fill_receipt(&r, WALLET), Err(FillRejection::NotANockPurchase), "{hash}");
        assert_eq!(
            classify_candidate(Some(&r), WALLET, Some(t("2026-11-15T00:00:00Z")), anchor()),
            CandidateVerdict::NotAPurchase { reason: "NOT_A_NOCK_PURCHASE" },
            "{hash}"
        );
    }
    // The deposit that paid USDC and got NOCK back is the false positive #118 warns
    // about. It does not qualify, because the wallet's net NOCK flow is outward.
    let deposit = receipt(LP_DEPOSITS[0]);
    assert_eq!(parse_accumulation_swap(&deposit, WALLET), None);
}

#[test]
fn test_each_rejection_is_named_and_the_handler_codes_are_unchanged() {
    let buy = receipt(BUY_3);
    let block_time = block_time_from_response(&block_header(58_040_708)).unwrap();

    // No receipt.
    let missing = verify_fill_receipt(&serde_json::Value::Null, WALLET).unwrap_err();
    assert_eq!(missing, FillRejection::ReceiptNotFound);
    assert_eq!((missing.http_status(), missing.code()), (StatusCode::NOT_FOUND, "RECEIPT_NOT_FOUND"));
    assert_eq!(
        missing.detail("p"),
        "Transaction not found on-chain. If the wallet shows it approved, it may still be pending broadcast; retry once it lands."
    );

    // A reverted transaction (the real buy with its status flipped).
    let mut reverted = buy.clone();
    reverted["status"] = json!("0x0");
    let r = verify_fill_receipt(&reverted, WALLET).unwrap_err();
    assert_eq!(r, FillRejection::TxReverted);
    assert_eq!((r.http_status(), r.code()), (StatusCode::UNPROCESSABLE_ENTITY, "TX_REVERTED"));
    assert_eq!(r.detail("p"), "Receipt status is not success; nothing recorded");

    // A sale, and someone else's swap (the real buy read for another wallet).
    let pool = "0x85f1aa3a70fedd1c52705c15baed143e675cd626";
    let sell = json!({ "status": "0x1", "blockNumber": "0x1", "logs": [
        transfer_log(NOCK_BASE, WALLET, pool, 80_000_000_000_000_000_000),
        transfer_log(USDC_BASE, pool, WALLET, 99_000_000),
    ]});
    let other_wallet = "0x1111111111111111111111111111111111111111";
    for r in [verify_fill_receipt(&sell, WALLET).unwrap_err(), verify_fill_receipt(&buy, other_wallet).unwrap_err()] {
        assert_eq!(r, FillRejection::NotANockPurchase);
        assert_eq!((r.http_status(), r.code()), (StatusCode::UNPROCESSABLE_ENTITY, "NOT_A_NOCK_PURCHASE"));
        assert_eq!(r.detail("p"), "Receipt does not show USDC leaving and NOCK entering the sovereign wallet");
    }
    let dust = FillRejection::AmountsNotPositive;
    assert_eq!((dust.code(), dust.detail("p").as_str()), ("NOT_A_NOCK_PURCHASE", "Parsed amounts are not positive"));

    // A real purchase from before the plan: rejected for the plan, with the hint.
    let old = receipt(PRE_PLAN_BUY);
    let old_time = block_time_from_response(&block_header(53_725_121)).unwrap();
    assert_eq!(old_time, t("2026-11-06T03:23:19Z"));
    let r = verify_fill(&old, WALLET, old_time, anchor()).unwrap_err();
    assert_eq!((r.http_status(), r.code()), (StatusCode::UNPROCESSABLE_ENTITY, "BEFORE_PLAN_START"));
    assert_eq!(
        r.detail("2026-11-reentry"),
        "This purchase was confirmed at 2026-11-06T03:23:19+00:00, before plan \"2026-11-reentry\" began (2026-11-14T00:00:00+00:00). To count earlier buys, set accumulation_plan.starts_at to an earlier time."
    );
    match r {
        FillRejection::BeforePlanStart { fill, .. } => {
            assert_eq!((fill.swap.usdc_raw, fill.swap.nock_raw), (105_471_820, 73_701_997_952_200_334_761));
        }
        other => panic!("expected BeforePlanStart, got {other:?}"),
    }
    // With an earlier anchor the same purchase counts; at the anchor instant it counts too.
    assert!(verify_fill(&old, WALLET, old_time, t("2026-11-01T00:00:00Z")).is_ok());
    assert!(verify_fill(&buy, WALLET, block_time, block_time).is_ok());
    assert!(fill_is_before_anchor(block_time - chrono::Duration::seconds(1), block_time));
}

#[test]
fn test_block_time_comes_from_the_header_or_not_at_all() {
    // The header times equal the transfer list's own block timestamps (two sources).
    assert_eq!(block_time_from_response(&block_header(58_040_708)), Some(t("2026-11-16T03:08:37Z")));
    assert_eq!(block_time_from_response(&block_header(57_605_753)), Some(t("2026-11-15T02:58:46Z")));
    assert_eq!(block_time_from_response(&block_header(57_228_848)), Some(t("2026-11-14T06:02:25Z")));
    assert_eq!(block_time_from_response(&json!({ "result": null })), None);
    assert_eq!(block_time_from_response(&json!({ "result": { "timestamp": "soon" } })), None);
    assert_eq!(block_time_from_response(&json!({})), None);
}

// ---------------- Candidate extraction from the transfer history ----------------

#[test]
fn test_transfer_history_fixture_parses_to_lowercase_hints() {
    let page = parse_asset_transfers(&transfers_response()).unwrap();
    assert_eq!(page.transfers.len(), 20);
    assert_eq!(page.page_key.as_deref(), Some("e73927e8-c0bf-4c78-a0eb-5279ce640947"));
    let first = &page.transfers[0];
    assert_eq!(first.tx_hash, V3_CLAIMS[0]);
    assert_eq!(first.block_number, 58_043_697);
    assert_eq!(first.block_time, t("2026-11-16T03:18:35Z"));
    for h in &page.transfers {
        assert_eq!(h.tx_hash, h.tx_hash.to_lowercase());
        assert!(is_valid_tx_hash(&h.tx_hash));
    }
    // An upper-case hash in a response is stored lower-case.
    let mut shouting = transfers_response();
    let upper = format!("0x{}", BUY_3.trim_start_matches("0x").to_uppercase());
    shouting["result"]["transfers"][2]["hash"] = json!(upper);
    assert_eq!(parse_asset_transfers(&shouting).unwrap().transfers[2].tx_hash, BUY_3);
}

#[test]
fn test_malformed_transfer_history_is_an_error_not_an_empty_list() {
    for bad in [
        json!({}),
        json!({ "jsonrpc": "2.0", "id": 1, "error": { "code": -32602, "message": "invalid params" } }),
        json!({ "result": {} }),
        json!({ "result": { "transfers": "none" } }),
        json!({ "result": { "transfers": [{ "blockNum": "0x1", "metadata": { "blockTimestamp": "2026-11-16T03:18:35.000Z" } }] } }),
        json!({ "result": { "transfers": [{ "hash": "0x1234", "blockNum": "0x1", "metadata": { "blockTimestamp": "2026-11-16T03:18:35.000Z" } }] } }),
        json!({ "result": { "transfers": [{ "hash": BUY_3, "metadata": { "blockTimestamp": "2026-11-16T03:18:35.000Z" } }] } }),
        json!({ "result": { "transfers": [{ "hash": BUY_3, "blockNum": "0x1" }] } }),
        json!({ "result": { "transfers": [{ "hash": BUY_3, "blockNum": "0x1", "metadata": { "blockTimestamp": "yesterday" } }] } }),
    ] {
        assert!(parse_asset_transfers(&bad).is_err(), "must be an error: {bad}");
    }
    // A genuinely empty history is a successful read of nothing.
    let empty = parse_asset_transfers(&json!({ "result": { "transfers": [] } })).unwrap();
    assert!(empty.transfers.is_empty() && empty.page_key.is_none());
}

#[test]
fn test_scan_stops_at_the_plan_anchor_and_collapses_to_distinct_hashes() {
    let page = parse_asset_transfers(&transfers_response()).unwrap();
    let mut scan = TransferScan::new(anchor());
    // The fixture reaches back to October, so one page is enough: no second call.
    assert_eq!(scan.absorb(page), None);
    assert!(scan.complete);
    assert_eq!(scan.pages, 1);
    assert_eq!(scan.checked_back_to(), Some(anchor()));
    let hashes: Vec<&str> = scan.hints.iter().map(|h| h.tx_hash.as_str()).collect();
    assert_eq!(
        hashes,
        vec![V3_CLAIMS[0], V4_CLAIMS[0], BUY_3, BUY_2, V3_CLAIMS[1], V4_CLAIMS[1], LP_DEPOSITS[1], LP_DEPOSITS[0], BUY_1],
        "everything since the anchor, newest first; nothing from before it"
    );
    assert!(!hashes.contains(&PRE_PLAN_BUY));

    // Two NOCK transfers in one transaction give one hash.
    let mut doubled = transfers_response();
    let mut second_leg = doubled["result"]["transfers"][2].clone();
    second_leg["uniqueId"] = json!(format!("{BUY_3}:log:11"));
    doubled["result"]["transfers"].as_array_mut().unwrap().insert(3, second_leg);
    let mut scan = TransferScan::new(anchor());
    scan.absorb(parse_asset_transfers(&doubled).unwrap());
    assert_eq!(scan.hints.len(), 9);
    assert_eq!(scan.hints.iter().filter(|h| h.tx_hash == BUY_3).count(), 1);
}

#[test]
fn test_scan_follows_a_second_page_until_the_anchor_or_the_end() {
    // The real response split in two: the first three transfers with its page key, then
    // the rest. An anchor before both pages forces the second read.
    let full = transfers_response();
    let all = full["result"]["transfers"].as_array().unwrap().clone();
    let page1 = json!({ "result": { "transfers": all[..3].to_vec(), "pageKey": "e73927e8-c0bf-4c78-a0eb-5279ce640947" } });
    let page2 = json!({ "result": { "transfers": all[3..].to_vec() } });

    let mut scan = TransferScan::new(anchor());
    let next = scan.absorb(parse_asset_transfers(&page1).unwrap());
    assert_eq!(next.as_deref(), Some("e73927e8-c0bf-4c78-a0eb-5279ce640947"));
    assert!(!scan.complete);
    // Cut short here, the scan says how far back it got, not "back to the anchor".
    assert_eq!(scan.checked_back_to(), Some(t("2026-11-16T03:08:37Z")));
    assert_eq!(scan.absorb(parse_asset_transfers(&page2).unwrap()), None);
    assert!(scan.complete);
    assert_eq!(scan.hints.len(), 9);
    assert_eq!(scan.pages, 2);

    // An anchor older than everything: complete only when the history ends.
    let mut old = TransferScan::new(t("2026-01-01T00:00:00Z"));
    assert!(old.absorb(parse_asset_transfers(&full).unwrap()).is_some());
    assert!(!old.complete);
    assert_eq!(old.absorb(TransferPage { transfers: vec![], page_key: None }), None);
    assert!(old.complete);
    assert_eq!(old.hints.len(), 20);

    // The request asks for the wallet's incoming NOCK, newest first, with timestamps.
    let req = asset_transfers_request(WALLET, Some("abc"));
    assert_eq!(req["method"], "alchemy_getAssetTransfers");
    let p = &req["params"][0];
    assert_eq!(p["toAddress"], WALLET);
    assert_eq!(p["contractAddresses"], json!([NOCK_BASE]));
    assert_eq!(p["category"], json!(["erc20"]));
    assert_eq!((p["order"].as_str(), p["withMetadata"].as_bool()), (Some("desc"), Some(true)));
    assert_eq!(p["maxCount"], "0x64");
    assert_eq!(p["pageKey"], "abc");
    assert!(asset_transfers_request(WALLET, None)["params"][0].get("pageKey").is_none());
}

#[test]
fn test_eth_get_logs_agrees_with_the_transfer_history() {
    // The public RPC's eth_getLogs over blocks 57,995,000..58,045,000 (NOCK Transfer,
    // to = wallet) returns the same three transactions the Alchemy list starts with.
    let logs: serde_json::Value =
        serde_json::from_str(include_str!("../testdata/fills/eth_getLogs_nock_in_57995000_58045000.json")).unwrap();
    let mut from_logs: Vec<(String, i64)> = logs["result"]
        .as_array()
        .unwrap()
        .iter()
        .map(|l| {
            assert!(l["topics"][0].as_str().unwrap().eq_ignore_ascii_case(TRANSFER_EVENT_TOPIC));
            assert!(l["address"].as_str().unwrap().eq_ignore_ascii_case(NOCK_BASE));
            assert!(l["topics"][2].as_str().unwrap().ends_with(&WALLET.trim_start_matches("0x").to_lowercase()));
            (
                l["transactionHash"].as_str().unwrap().to_lowercase(),
                i64::from_str_radix(l["blockNumber"].as_str().unwrap().trim_start_matches("0x"), 16).unwrap(),
            )
        })
        .collect();
    from_logs.sort_by(|a, b| b.1.cmp(&a.1));
    let page = parse_asset_transfers(&transfers_response()).unwrap();
    let from_alchemy: Vec<(String, i64)> = page
        .transfers
        .iter()
        .filter(|h| h.block_number >= 57_995_000)
        .map(|h| (h.tx_hash.clone(), h.block_number))
        .collect();
    assert_eq!(from_logs, from_alchemy);
    assert_eq!(from_logs.len(), 3);
}

// ---------------- Classification ----------------

#[test]
fn test_an_unread_receipt_is_never_classified() {
    let buy = receipt(BUY_3);
    let when = Some(t("2026-11-16T03:08:37Z"));
    // The node did not answer, or answered "no such receipt".
    assert_eq!(classify_candidate(None, WALLET, when, anchor()), CandidateVerdict::Unread);
    assert_eq!(classify_candidate(Some(&serde_json::Value::Null), WALLET, when, anchor()), CandidateVerdict::Unread);
    // The receipt is a purchase but its block header could not be read.
    assert_eq!(classify_candidate(Some(&buy), WALLET, None, anchor()), CandidateVerdict::Unread);
    // A fee claim needs no block header to be ruled out.
    assert_eq!(
        classify_candidate(Some(&receipt(V4_CLAIMS[0])), WALLET, None, anchor()),
        CandidateVerdict::NotAPurchase { reason: "NOT_A_NOCK_PURCHASE" }
    );
    let mut reverted = buy.clone();
    reverted["status"] = json!("0x0");
    assert_eq!(
        classify_candidate(Some(&reverted), WALLET, when, anchor()),
        CandidateVerdict::NotAPurchase { reason: "TX_REVERTED" }
    );
}

/// The whole pure path on real data: transfer history → distinct hashes → receipts and
/// headers → verdicts → candidate rows.
fn candidates_from_fixtures(anchor: DateTime<Utc>) -> Vec<FillCandidate> {
    let mut scan = TransferScan::new(anchor);
    scan.absorb(parse_asset_transfers(&transfers_response()).unwrap());
    let mut rows = Vec::new();
    for hint in &scan.hints {
        let r = receipt(&hint.tx_hash);
        let block_time = match verify_fill_receipt(&r, WALLET) {
            Ok(swap) => block_time_from_response(&block_header(swap.block_number)),
            Err(_) => None,
        };
        rows.push(match classify_candidate(Some(&r), WALLET, block_time, anchor) {
            CandidateVerdict::Purchase(fill) => {
                // The header time and the transfer list's own timestamp agree.
                assert_eq!(fill.confirmed_at, hint.block_time, "{}", hint.tx_hash);
                FillCandidate {
                    tx_hash: hint.tx_hash.clone(),
                    state: "offered".to_string(),
                    block_number: Some(fill.swap.block_number),
                    confirmed_at: Some(fill.confirmed_at),
                    usdc_spent: Some(fill.swap.usdc_spent),
                    nock_received: Some(fill.swap.nock_received),
                    effective_price_usd: Some(fill.swap.effective_price_usd),
                    ..Default::default()
                }
            }
            CandidateVerdict::NotAPurchase { .. } => FillCandidate {
                tx_hash: hint.tx_hash.clone(),
                state: "not_a_purchase".to_string(),
                ..Default::default()
            },
            CandidateVerdict::Unread => panic!("every fixture receipt is readable: {}", hint.tx_hash),
        });
    }
    rows
}

#[test]
fn test_real_history_offers_exactly_the_plan_buys() {
    let rows = candidates_from_fixtures(anchor());
    assert_eq!(rows.len(), 9);
    assert_eq!(rows.iter().filter(|c| c.state == "not_a_purchase").count(), 6, "4 fee claims + 2 deposits");

    // Nothing recorded yet: all three buys are offered, oldest first.
    let (offers, dismissed) = build_fill_offers(&rows, &HashSet::new(), anchor(), &[], 24.0);
    assert!(dismissed.is_empty());
    assert_eq!(offers.iter().map(|o| o.tx_hash.as_str()).collect::<Vec<_>>(), vec![BUY_1, BUY_2, BUY_3]);
    // The labels are already the ones ONCHAIN_INTEGRATION.md §10 gives: the second buy
    // came 20 h 56 min after the first (early on a 24 h plan), the third 24 h 10 min
    // after the second. They do not wait for the first buy to be confirmed.
    assert_eq!(
        offers.iter().map(|o| o.timing.as_str()).collect::<Vec<_>>(),
        vec!["on_schedule", "early", "on_schedule"]
    );
    let first = &offers[0];
    assert_eq!(first.confirmed_at, "2026-11-14T06:02:25+00:00");
    assert_eq!(first.block_number, 57_228_848);
    assert_eq!(first.usdc_spent, "20");
    assert_eq!(first.nock_received, "1597.9340475636968138");
    assert_eq!(first.effective_price_usd, "0.01251616");
    assert_eq!(first.timing, "on_schedule");
    // No stored price was looked up here, so no cost figure is shown: never a substitute.
    assert_eq!((first.reference_spot_usd.as_ref(), first.execution_cost_bps), (None, None));

    // After the first buy is recorded (24 h plan) the labels are the same: the second
    // is early against the recorded fill instead of against the open offer.
    let cfg = plan_cfg();
    let fills = PlanFills { count: 1, spent_usdc: 20.0, acquired_nock: 1597.93, last_at: Some(t(BUY_1_AT)), first_price: Some(0.01251616) };
    assert_eq!(next_tranche_due_at(&cfg, &fills), Some(t("2026-11-15T06:02:25Z")));
    // The ledger stores the hash as the operator pasted it; case must not matter.
    let recorded: HashSet<String> = [BUY_1.to_uppercase().replace("0X", "0x")].into_iter().collect();
    let (offers, _) = build_fill_offers(&rows, &recorded, anchor(), &[t(BUY_1_AT)], cfg.interval_hours);
    assert_eq!(
        offers.iter().map(|o| (o.tx_hash.as_str(), o.timing.as_str())).collect::<Vec<_>>(),
        vec![(BUY_2, "early"), (BUY_3, "on_schedule")]
    );

    // An anchor that reaches back to 2026-11-01 also finds the earlier purchase.
    let earlier = candidates_from_fixtures(t("2026-11-01T00:00:00Z"));
    let (offers, _) = build_fill_offers(&earlier, &HashSet::new(), t("2026-11-01T00:00:00Z"), &[], 24.0);
    assert_eq!(offers.iter().map(|o| o.tx_hash.as_str()).collect::<Vec<_>>(), vec![PRE_PLAN_BUY, BUY_1, BUY_2, BUY_3]);
    // The same rows under the later anchor: the early purchase is not offered.
    let (offers, _) = build_fill_offers(&earlier, &HashSet::new(), anchor(), &[], 24.0);
    assert_eq!(offers.len(), 3);
}

/// Review finding on #118: the label used to compare every offer with the plan's LATEST
/// recorded fill plus the interval, so an offer older than that fill was always "early",
/// the plan's first buy included. Each offer is judged against the buy before it.
#[test]
fn test_timing_is_judged_against_the_buy_before_each_offer_not_the_latest_fill() {
    let rows = candidates_from_fixtures(anchor());
    let timing = |recorded: &[&str], dismissed: &[&str]| -> Vec<(String, String)> {
        let mut rows = rows.clone();
        for c in rows.iter_mut().filter(|c| dismissed.contains(&c.tx_hash.as_str())) {
            c.state = "dismissed".to_string();
        }
        let hashes: HashSet<String> = recorded.iter().map(|h| h.to_string()).collect();
        let times: Vec<DateTime<Utc>> = recorded.iter().map(|h| buy_time(h)).collect();
        let (offers, gone) = build_fill_offers(&rows, &hashes, anchor(), &times, 24.0);
        offers.into_iter().chain(gone).map(|o| (o.tx_hash, o.timing)).collect()
    };
    let label = |h: &str, l: &str| (h.to_string(), l.to_string());

    // Only the newest buy is recorded (pasted by hash, or confirmed first during a
    // recovery). The first buy is not early: nothing came before it. The second is
    // early because of the first, not because of the newer recorded fill.
    assert_eq!(timing(&[BUY_3], &[]), vec![label(BUY_1, "on_schedule"), label(BUY_2, "early")]);
    // The order of confirmation does not move a label.
    assert_eq!(timing(&[BUY_2], &[]), vec![label(BUY_1, "on_schedule"), label(BUY_3, "on_schedule")]);
    assert_eq!(timing(&[BUY_1, BUY_3], &[]), vec![label(BUY_2, "early")]);
    assert_eq!(timing(&[BUY_1, BUY_2], &[]), vec![label(BUY_3, "on_schedule")]);
    // A swap marked "not part of the plan" is nobody's previous buy: with the first
    // dismissed, the second is the plan's first and is not early. The dismissed row
    // keeps its own label (listed after the open offers here).
    assert_eq!(
        timing(&[], &[BUY_1]),
        vec![label(BUY_2, "on_schedule"), label(BUY_3, "on_schedule"), label(BUY_1, "on_schedule")]
    );
    // A dismissed swap is still judged against what came before it.
    assert_eq!(
        timing(&[BUY_1], &[BUY_2]),
        vec![label(BUY_3, "on_schedule"), label(BUY_2, "early")]
    );

    // Another plan's fills set no cadence for this one: they hide their own hash only.
    let other_plan: HashSet<String> = [BUY_1.to_string()].into_iter().collect();
    let (offers, _) = build_fill_offers(&rows, &other_plan, anchor(), &[], 24.0);
    assert_eq!(
        offers.iter().map(|o| (o.tx_hash.as_str(), o.timing.as_str())).collect::<Vec<_>>(),
        vec![(BUY_2, "on_schedule"), (BUY_3, "on_schedule")]
    );
    // The interval is the plan's own: on a 12 h plan none of the three is early.
    let (offers, _) = build_fill_offers(&rows, &HashSet::new(), anchor(), &[], 12.0);
    assert!(offers.iter().all(|o| o.timing == "on_schedule"), "{offers:?}");
}

// ---------------- The offer list ----------------

fn candidate(tx_hash: &str, state: &str, at: &str) -> FillCandidate {
    FillCandidate {
        tx_hash: tx_hash.to_string(),
        state: state.to_string(),
        block_number: Some(1),
        confirmed_at: Some(t(at)),
        usdc_spent: Some("20".to_string()),
        nock_received: Some("1600".to_string()),
        effective_price_usd: Some("0.01250000".to_string()),
        ..Default::default()
    }
}

#[test]
fn test_offer_list_excludes_recorded_dismissed_non_purchases_and_pre_anchor_rows() {
    let h = |n: u8| format!("0x{}", format!("{n:02x}").repeat(32));
    let rows = vec![
        candidate(&h(1), "offered", "2026-11-15T10:00:00Z"),
        candidate(&h(2), "offered", "2026-11-15T09:00:00Z"),
        candidate(&h(3), "dismissed", "2026-11-15T08:00:00Z"),
        candidate(&h(4), "not_a_purchase", "2026-11-15T07:00:00Z"),
        candidate(&h(5), "offered", "2026-11-13T23:59:59Z"), // before the anchor
        candidate(&h(6), "offered", "2026-11-15T06:00:00Z"), // recorded (any plan id)
        candidate(&h(1).to_uppercase().replace("0X", "0x"), "offered", "2026-11-15T10:00:00Z"), // same hash again
        FillCandidate { tx_hash: h(7), state: "offered".to_string(), ..Default::default() }, // no amounts
        candidate(&h(8), "dismissed", "2026-11-13T00:00:00Z"), // dismissed, before the anchor
        candidate(&h(9), "surprise", "2026-11-15T11:00:00Z"),  // unknown state
    ];
    let recorded: HashSet<String> = [h(6).to_uppercase().replace("0X", "0x")].into_iter().collect();
    let (offers, dismissed) = build_fill_offers(&rows, &recorded, anchor(), &[], 24.0);
    assert_eq!(offers.iter().map(|o| o.tx_hash.clone()).collect::<Vec<_>>(), vec![h(2), h(1)], "oldest first, once each");
    assert_eq!(dismissed.iter().map(|o| o.tx_hash.clone()).collect::<Vec<_>>(), vec![h(3)]);

    // Restoring is the reverse of dismissing: the row moves lists, nothing else changes.
    let mut restored = rows.clone();
    restored[2].state = "offered".to_string();
    let (offers, dismissed) = build_fill_offers(&restored, &recorded, anchor(), &[], 24.0);
    assert_eq!(offers.iter().map(|o| o.tx_hash.clone()).collect::<Vec<_>>(), vec![h(3), h(2), h(1)]);
    assert!(dismissed.is_empty());

    // Recording a dismissed swap by hand removes it from the dismissed list too.
    let both: HashSet<String> = [h(6), h(3)].into_iter().collect();
    assert!(build_fill_offers(&rows, &both, anchor(), &[], 24.0).1.is_empty());

    // A cost figure is shown only together with its reference.
    let mut priced = candidate(&h(10), "offered", "2026-11-15T12:00:00Z");
    priced.reference_spot_usd = Some("0.01230000".to_string());
    priced.reference_spot_source = Some("market_price_history uniswap_rpc at 2026-11-15T11:58:00+00:00".to_string());
    priced.execution_cost_bps = Some(163);
    let mut half = candidate(&h(11), "offered", "2026-11-15T13:00:00Z");
    half.execution_cost_bps = Some(40); // a cost with no reference is not shown
    let (offers, _) = build_fill_offers(&[priced, half], &HashSet::new(), anchor(), &[], 24.0);
    assert_eq!(offers[0].execution_cost_bps, Some(163));
    assert_eq!(offers[0].reference_spot_usd.as_deref(), Some("0.01230000"));
    assert_eq!((offers[1].execution_cost_bps, offers[1].reference_spot_usd.clone()), (None, None));
}

#[test]
fn test_timing_labels_a_swap_less_than_one_interval_after_the_buy_before_it_as_early() {
    let previous = Some(t(BUY_1_AT));
    assert_eq!(fill_timing(t(BUY_2_AT), previous, 24.0), "early");
    // Exactly one interval later is the moment the tranche comes due: not early.
    assert_eq!(fill_timing(t("2026-11-15T06:02:24Z"), previous, 24.0), "early");
    assert_eq!(fill_timing(t("2026-11-15T06:02:25Z"), previous, 24.0), "on_schedule");
    assert_eq!(fill_timing(t(BUY_3_AT), previous, 24.0), "on_schedule");
    // The boundary is the directive's own due time, to the second.
    let fills = PlanFills { last_at: previous, ..PlanFills::default() };
    assert_eq!(next_tranche_due_at(&plan_cfg(), &fills), Some(t("2026-11-15T06:02:25Z")));
    // With no buy before it the first tranche is due from the start: nothing is early.
    assert_eq!(fill_timing(t(BUY_1_AT), None, 24.0), "on_schedule");
    assert_eq!(next_tranche_due_at(&plan_cfg(), &PlanFills::default()), None);
    // An interval that is not a number labels nothing early, and nothing panics.
    assert_eq!(fill_timing(t(BUY_2_AT), previous, f64::NAN), "on_schedule");
    assert_eq!(fill_timing(t(BUY_2_AT), previous, 1e300), "early");
}

/// Review finding on #118: three 20 s attempts outlasted the 60 s cap on the whole
/// check, so a stalled endpoint could never report its own reason.
#[test]
fn test_a_stalled_history_call_fails_with_its_own_reason_inside_the_cap() {
    let worst = transfer_history_worst_case_ms();
    assert_eq!(worst, 20_150, "two 10 s attempts and one 150 ms pause");
    assert_eq!(
        worst,
        u64::from(FILL_SCAN_REQUEST_ATTEMPTS) * FILL_SCAN_REQUEST_TIMEOUT_SECONDS * 1000 + 150
    );
    assert!(
        worst * 2 <= FILL_SCAN_TIMEOUT_SECONDS * 1000,
        "a stalled history page must fail by itself with at least half the cap left for receipts"
    );
}

// ---------------- Execution cost over the reference spot ----------------

#[test]
fn test_execution_cost_over_spot() {
    assert_eq!(execution_cost_bps(0.0127, Some(0.0125)), Some(160));
    assert_eq!(execution_cost_bps(0.0125, Some(0.0125)), Some(0));
    assert_eq!(execution_cost_bps(0.0124, Some(0.0125)), Some(-80));
    // The first plan buy against the engine spot noted at the time (+1.8%).
    assert_eq!(execution_cost_bps(0.01251616, Some(0.01229)), Some(184));
    // No reference, a zero or a non-finite one: no figure, never a substitute.
    for bad in [None, Some(0.0), Some(-0.01), Some(f64::NAN), Some(f64::INFINITY)] {
        assert_eq!(execution_cost_bps(0.0127, bad), None, "{bad:?}");
    }
    assert_eq!(execution_cost_bps(f64::NAN, Some(0.0125)), None);
    assert_eq!(execution_cost_bps(0.0, Some(0.0125)), None);

    let found = fill_reference("0.01270000", Some((0.0125, "market_price_history uniswap_rpc at 2026-11-15T02:55:00+00:00".to_string())));
    assert_eq!(found.spot_usd.as_deref(), Some("0.01250000"));
    assert_eq!(found.cost_bps, Some(160));
    assert!(found.source.unwrap().contains("uniswap_rpc at 2026-11-15T02:55:00"));
    // No stored price near the block: all three stay empty together.
    assert_eq!(fill_reference("0.01270000", None), FillReference::default());
    assert_eq!(fill_reference("0.01270000", Some((0.0, "x".to_string()))), FillReference::default());
    assert_eq!(fill_reference("not a price", Some((0.0125, "x".to_string()))), FillReference::default());
}

// ---------------- When detection runs, and what it reports ----------------

fn plan_cfg() -> AccumulationPlanConfig {
    AccumulationPlanConfig {
        id: "2026-11-reentry".to_string(),
        budget_usdc: 100.0,
        tranches: 5,
        interval_hours: 24.0,
        starts_at: None,
        venue: Some("Aero".to_string()),
        max_price_usd: None,
        pause_when_overextended: true,
        respect_cash_target: true,
        cash_reserve_usdc: None,
    }
}

#[test]
fn test_detection_is_skipped_only_for_stated_reasons() {
    let now = t("2026-11-16T04:00:00Z");
    let none = PlanFills::default();
    assert_eq!(detection_gate(&plan_cfg(), &none, false, now), DetectionGate::Run);
    // Waiting between tranches is still an active plan: an early buy must be seen.
    let one = PlanFills { count: 1, spent_usdc: 20.0, acquired_nock: 1600.0, last_at: Some(now), first_price: Some(0.0125) };
    assert_eq!(detection_gate(&plan_cfg(), &one, false, now), DetectionGate::Run);

    let skip = |g: DetectionGate| match g {
        DetectionGate::Skip { reason, list_known } => (reason, list_known),
        DetectionGate::Run => panic!("expected a skip"),
    };
    let (reason, list_known) = skip(detection_gate(&plan_cfg(), &none, true, now));
    assert!(reason.contains("ledger could not be read") && !list_known, "{reason}");

    let mut invalid = plan_cfg();
    invalid.tranches = 0;
    let (reason, list_known) = skip(detection_gate(&invalid, &none, false, now));
    assert!(reason.contains("invalid") && !list_known);

    let mut future = plan_cfg();
    future.starts_at = Some(t("2026-12-01T00:00:00Z"));
    let (reason, _) = skip(detection_gate(&future, &none, false, now));
    assert!(reason.contains("has not started") && reason.contains("2026-12-01T00:00:00+00:00"), "{reason}");

    // Budget spent: nothing new is looked for, but a swap still waiting stays listed.
    let spent = PlanFills { count: 5, spent_usdc: 100.0, acquired_nock: 8000.0, last_at: Some(now), first_price: Some(0.0125) };
    let (reason, list_known) = skip(detection_gate(&plan_cfg(), &spent, false, now));
    assert!(reason.contains("budget is spent") && list_known);
}

#[test]
fn test_scan_status_is_ok_only_for_a_complete_check() {
    let complete = ScanSummary { complete: true, checked_back_to: Some(anchor()), unread: 0, deferred: 0 };
    assert_eq!(scan_status(&complete, anchor()), ("ok", None));

    let unread = ScanSummary { unread: 2, ..complete.clone() };
    let (status, detail) = scan_status(&unread, anchor());
    assert_eq!(status, "partial");
    assert!(detail.unwrap().contains("2 transaction(s) could not be read and will be tried again"));

    let deferred = ScanSummary { deferred: 7, ..complete.clone() };
    assert!(scan_status(&deferred, anchor()).1.unwrap().contains("7 more transfer(s) are still to be examined"));

    // Cut short: the text says how far back was checked.
    let cut = ScanSummary { complete: false, checked_back_to: Some(t("2026-11-15T00:00:00Z")), unread: 0, deferred: 0 };
    let (status, detail) = scan_status(&cut, anchor());
    assert_eq!(status, "partial");
    let detail = detail.unwrap();
    assert!(detail.contains("read back to 2026-11-15T00:00:00+00:00, not as far as the plan anchor 2026-11-14T00:00:00+00:00"), "{detail}");

    assert_eq!(FillDetection::unavailable("x").status, "unavailable");
    assert_eq!(FillDetection::not_run("y").detail.as_deref(), Some("y"));
    assert!(FillDetection::unavailable("x").offers.is_empty());
}

#[test]
fn test_rpc_error_reason_names_the_error_and_nothing_else() {
    // What Alchemy returns for a bad request, and what a node without the method returns.
    let bad = json!({ "jsonrpc": "2.0", "id": 44, "error": { "code": -32602, "message": "invalid params" } });
    assert_eq!(rpc_error_reason(&bad).as_deref(), Some("the RPC answered with error -32602: invalid params"));
    let missing = json!({ "jsonrpc": "2.0", "id": 44, "error": { "code": -32601, "message": "the method alchemy_getAssetTransfers does not exist/is not available" } });
    assert!(rpc_error_reason(&missing).unwrap().contains("-32601: the method alchemy_getAssetTransfers does not exist"));
    // The public RPC's real answer to a 200,000-block eth_getLogs (ONCHAIN_INTEGRATION.md §10).
    let range = json!({ "jsonrpc": "2.0", "error": { "code": -32701, "message": "exceed maximum block range: 50000" }, "id": 1 });
    assert_eq!(rpc_error_reason(&range).as_deref(), Some("the RPC answered with error -32701: exceed maximum block range: 50000"));
    // A long message is cut, an odd error still reads as an error, an answer is not one.
    let long = json!({ "error": { "message": "x".repeat(500) } });
    assert_eq!(rpc_error_reason(&long).unwrap().len(), "the RPC answered with an error: ".len() + 160);
    assert_eq!(rpc_error_reason(&json!({ "error": {} })).as_deref(), Some("the RPC answered with an error: no message"));
    assert_eq!(rpc_error_reason(&transfers_response()), None);
}

#[test]
fn test_shipped_example_config_leaves_detection_on_and_adds_no_numbers() {
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/config.yaml.example");
    let cfg = load_app_config(example, Some("/nonexistent/nocknock-config.yaml"), false).unwrap();
    assert!(cfg.fill_detection.enabled);
    // The example documents the switch as a comment; detection has no other setting.
    let text = std::fs::read_to_string(example).unwrap();
    assert!(text.contains("# fill_detection:\n#   enabled: true"));
}

#[test]
fn test_fill_detection_config_is_on_by_default_and_can_be_switched_off() {
    let default: AppConfig = serde_yaml::from_str("port: 8080\n").unwrap();
    assert!(default.fill_detection.enabled);
    let off: AppConfig = serde_yaml::from_str("fill_detection:\n  enabled: false\n").unwrap();
    assert!(!off.fill_detection.enabled);
    let empty: AppConfig = serde_yaml::from_str("fill_detection: {}\n").unwrap();
    assert!(empty.fill_detection.enabled);
}

// ---------------- Dismiss and restore ----------------

#[test]
fn test_dismiss_and_restore_only_move_a_detected_purchase_between_two_states() {
    use CandidateAction::{Dismiss, Restore};
    assert_eq!(candidate_transition(Dismiss, Some("offered"), false), Ok(("dismissed", true)));
    assert_eq!(candidate_transition(Restore, Some("dismissed"), false), Ok(("offered", true)));
    // Idempotent.
    assert_eq!(candidate_transition(Dismiss, Some("dismissed"), false), Ok(("dismissed", false)));
    assert_eq!(candidate_transition(Restore, Some("offered"), false), Ok(("offered", false)));

    let code = |r: std::result::Result<(&'static str, bool), (StatusCode, &'static str, &'static str)>| {
        let (status, code, _) = r.unwrap_err();
        (status, code)
    };
    // A hash NockNock never detected cannot be turned into a candidate here.
    assert_eq!(code(candidate_transition(Dismiss, None, false)), (StatusCode::NOT_FOUND, "CANDIDATE_NOT_FOUND"));
    assert_eq!(code(candidate_transition(Restore, None, true)), (StatusCode::NOT_FOUND, "CANDIDATE_NOT_FOUND"));
    // A fee claim cannot be restored into an offer.
    for action in [Dismiss, Restore] {
        assert_eq!(code(candidate_transition(action, Some("not_a_purchase"), false)), (StatusCode::CONFLICT, "NOT_AN_OFFER"));
        // A recorded buy is not an offer any more.
        assert_eq!(code(candidate_transition(action, Some("offered"), true)), (StatusCode::CONFLICT, "ALREADY_RECORDED"));
        assert_eq!(code(candidate_transition(action, Some("dismissed"), true)), (StatusCode::CONFLICT, "ALREADY_RECORDED"));
    }
}

// ---------------- Plan status, advice and state hash ----------------

/// A funded wallet with no LP: 90,000 NOCK and $1,053 USDC.
fn funded_raw() -> RawOnChainData {
    let mut raw = RawOnChainData::default();
    raw.spot_price_nock_usd = 0.01153;
    raw.live_eth_price_usd = 3000.0;
    raw.live_gas_cost_usd = 0.004;
    raw.wallet_eth = Some(0.01);
    raw.wallet_weth = Some(0.0);
    let row = |sym: &str, addr: &str, amount: f64| WalletBalance {
        wallet_address: WALLET.to_string(),
        token_symbol: sym.to_string(),
        token_address: addr.to_string(),
        amount: format!("{:.4}", amount),
        amount_usd: "0".to_string(),
        updated_at: "now".to_string(),
    };
    raw.wallet_balances = vec![row("NOCK", NOCK_BASE, 90_000.0), row("USDC", USDC_BASE, 1053.0)];
    raw
}

fn metrics(profile: &OperatorProfile, ctx: &PlanContext, detection: Option<&FillDetection>) -> DerivedMetrics {
    compute_derived_metrics_with_fills(
        &funded_raw(), 0.0, profile, StructuralChannels { channels: vec![] }, &[],
        &LedgerAggregate::default(), &MarketRegime::default(), ctx, detection,
    )
}

/// The open offers when `recorded` (some of the plan's three buys) are in the ledger.
fn real_offers(recorded: &[&str]) -> Vec<FillOffer> {
    let times: Vec<DateTime<Utc>> = recorded.iter().map(|h| buy_time(h)).collect();
    let recorded: HashSet<String> = recorded.iter().map(|h| h.to_string()).collect();
    build_fill_offers(&candidates_from_fixtures(anchor()), &recorded, anchor(), &times, plan_cfg().interval_hours).0
}

fn detection_ok(offers: Vec<FillOffer>, checked: &str) -> FillDetection {
    FillDetection {
        status: "ok".to_string(),
        detail: None,
        checked_back_to: Some(anchor().to_rfc3339()),
        last_checked_at: Some(checked.to_string()),
        offers,
        dismissed: vec![],
    }
}

#[test]
fn test_open_offers_are_not_progress_and_lead_the_advice_in_every_plan_branch() {
    let due_ctx = PlanContext { cfg: Some(plan_cfg()), fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-16T04:00:00Z") };
    let one_offer = detection_ok(real_offers(&[BUY_1, BUY_2]), "2026-11-16T04:00:00+00:00");
    assert_eq!(one_offer.offers.len(), 1);
    let no_offer = detection_ok(vec![], "2026-11-16T04:00:00+00:00");
    let found = "A swap that looks like a plan buy was found on-chain and is not counted yet: $20.00 USDC for 1780.36 NOCK at 2026-11-16T03:08:37+00:00 (0x78376f3c…6e31b9).";

    // Branch 1: a due tranche leads (plan_leads).
    let with = metrics(&OperatorProfile::default(), &due_ctx, Some(&one_offer));
    let without = metrics(&OperatorProfile::default(), &due_ctx, Some(&no_offer));
    assert_eq!(with.global_status.shot_advice_action, "DCA TRANCHE DUE (1 of 5)");
    assert_eq!(
        with.global_status.shot_advice_action, without.global_status.shot_advice_action,
        "the action title is what the notifier compares: an offer must not change it"
    );
    let d = &with.global_status.shot_advice_details;
    assert!(d.starts_with(found), "{d}");
    assert!(d.contains("In the dashboard, confirm it or mark it as not part of the plan before buying again."), "{d}");
    assert!(d.contains("Buy $20.00 of NOCK (spot $0.01153; your plan's venue label: Aero)"), "the tranche advice itself is still there: {d}");
    assert!(!without.global_status.shot_advice_details.contains("was found on-chain"));
    // An offer is not a fill: nothing is counted until it is recorded.
    let ps = with.accumulation_plan.as_ref().unwrap();
    assert_eq!((ps.fills, ps.spent_usdc.as_str(), ps.acquired_nock.as_str()), (0, "0.00", "0.0000"));
    assert_eq!((ps.status.as_str(), ps.tranche_index, ps.avg_price_usd.clone()), ("tranche_due", 1, None));
    assert_eq!(ps.fill_detection.as_ref().unwrap().offers[0].tx_hash, BUY_3);

    // Branch 2: below the floor, the plan is the recovery path.
    let floor = OperatorProfile { treasury_floor_nock: 100_000.0, ..OperatorProfile::default() };
    let with = metrics(&floor, &due_ctx, Some(&one_offer));
    let without = metrics(&floor, &due_ctx, Some(&no_offer));
    assert_eq!(with.global_status.shot_advice_action, "DEFENSIVE LOCK (Floor Recovery) · DCA TRANCHE DUE (1 of 5)");
    assert_eq!(with.global_status.shot_advice_action, without.global_status.shot_advice_action);
    assert!(with.global_status.shot_advice_details.contains(found), "{}", with.global_status.shot_advice_details);

    // Branch 3: nothing due (waiting), the plan still speaks.
    let fills = PlanFills { count: 2, spent_usdc: 40.0, acquired_nock: 3318.15, last_at: Some(t("2026-11-15T02:58:46Z")), first_price: Some(0.01251616) };
    let waiting_ctx = PlanContext { cfg: Some(plan_cfg()), fills, ledger_unavailable: false, now: t("2026-11-15T12:00:00Z") };
    let with = metrics(&OperatorProfile::default(), &waiting_ctx, Some(&one_offer));
    let without = metrics(&OperatorProfile::default(), &waiting_ctx, Some(&no_offer));
    assert_eq!(with.global_status.shot_advice_action, "DCA ON SCHEDULE (3 of 5 next)");
    assert_eq!(with.global_status.shot_advice_action, without.global_status.shot_advice_action);
    let d = &with.global_status.shot_advice_details;
    assert!(d.starts_with(found), "{d}");
    assert!(d.contains("mark it as not part of the plan."), "nothing is due, so no 'before buying again': {d}");
    assert!(!d.contains("before buying again"));
    assert_eq!(with.accumulation_plan.unwrap().fills, 2);

    // Several offers are counted, not listed one by one.
    let three = detection_ok(real_offers(&[]), "2026-11-16T04:00:00+00:00");
    let m = metrics(&OperatorProfile::default(), &due_ctx, Some(&three));
    assert!(
        m.global_status.shot_advice_details.starts_with("3 swaps that look like plan buys were found on-chain and are not counted yet ($60.00 USDC in total)."),
        "{}", m.global_status.shot_advice_details
    );
    assert_eq!(m.global_status.shot_advice_action, "DCA TRANCHE DUE (1 of 5)");
    assert_eq!(m.accumulation_plan.unwrap().spent_usdc, "0.00");
}

/// Review finding on #118: the notice was written only by the plan's own advice arms, so
/// an arm that outranks the plan (overextension, a due harvest) or follows it (a
/// finished plan) carried offers in the snapshot and said nothing about them.
#[test]
fn test_open_offers_lead_the_advice_when_another_arm_outranks_or_follows_the_plan() {
    let run = |raw: &RawOnChainData, regime: &MarketRegime, ctx: &PlanContext, detection: &FillDetection| {
        compute_derived_metrics_with_fills(
            raw, 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[],
            &LedgerAggregate::default(), regime, ctx, Some(detection),
        )
    };
    let one_offer = detection_ok(real_offers(&[BUY_1, BUY_2]), "2026-11-16T04:00:00+00:00");
    let no_offer = detection_ok(vec![], "2026-11-16T04:00:00+00:00");
    let found = "A swap that looks like a plan buy was found on-chain and is not counted yet: $20.00 USDC for 1780.36 NOCK at 2026-11-16T03:08:37+00:00 (0x78376f3c…6e31b9). In the dashboard, confirm it or mark it as not part of the plan.";
    // In every arm: the title is the one the arm chose (the notifier compares titles),
    // the notice leads, it is said once, and the arm's own text follows unchanged.
    let check = |raw: &RawOnChainData, regime: &MarketRegime, ctx: &PlanContext, title: &str, plan_state: &str| {
        let with = run(raw, regime, ctx, &one_offer);
        let without = run(raw, regime, ctx, &no_offer);
        assert_eq!(with.global_status.shot_advice_action, title);
        assert_eq!(without.global_status.shot_advice_action, title, "an offer must not change the title");
        let (d, plain) = (&with.global_status.shot_advice_details, &without.global_status.shot_advice_details);
        assert_eq!(d, &format!("{found} {plain}"), "{title}");
        assert_eq!(d.matches("was found on-chain").count(), 1, "{d}");
        assert!(!plain.contains("was found on-chain"), "{plain}");
        let ps = with.accumulation_plan.as_ref().unwrap();
        assert_eq!(ps.status, plan_state);
        assert_eq!(ps.fill_detection.as_ref().unwrap().offers.len(), 1);
    };

    // Overextended: the plan pauses and the cooldown arm speaks.
    let two = PlanFills { count: 2, spent_usdc: 40.0, acquired_nock: 3318.15, last_at: Some(t(BUY_2_AT)), first_price: Some(0.01251616) };
    let due_ctx = PlanContext { cfg: Some(plan_cfg()), fills: two, ledger_unavailable: false, now: t("2026-11-16T04:00:00Z") };
    let hot = MarketRegime { rsi_5m: None, rsi_1d: Some(75.0), overextended: true };
    check(&funded_raw(), &hot, &due_ctx, "OVEREXTENDED (Daily RSI ≥ 70, No Buy Advised)", "paused");

    // A due harvest outranks a plan with nothing due.
    let mut with_fees = funded_raw();
    with_fees.positions.push(RawPosition {
        position_id: "uniswap-v4:base:3163431".to_string(),
        network: "base".to_string(),
        protocol: "uniswap-v4".to_string(),
        wallet: WALLET.to_lowercase(),
        nft_id: 3163431,
        pool_id: "0x91a9ef0a8dd231b99fa53a2a145a9e8bca7897e0d85852107abb997916f9f99c".to_string(),
        npm_address: None,
        fee_tier_bps: 75,
        token0: PositionTokenInfo { address: USDC_BASE.to_string(), symbol: "USDC".to_string(), decimals: 6 },
        token1: PositionTokenInfo { address: NOCK_BASE.to_string(), symbol: "NOCK".to_string(), decimals: 16 },
        // An emptied range that still has $6 of USDC fees to collect.
        liquidity: 0,
        sqrt_price_x96: U256::from_hex_be("0x0a9598f7fe4938c0312958557374bf").unwrap(),
        tick: 269008,
        tick_lower: 267825,
        tick_upper: 271050,
        amount0: 0.0,
        amount1: 0.0,
        fee0: 6.0,
        fee1: 0.0,
        in_range: true,
        is_price_source: false,
    });
    let waiting_ctx = PlanContext { cfg: Some(plan_cfg()), fills: two, ledger_unavailable: false, now: t("2026-11-15T12:00:00Z") };
    check(&with_fees, &MarketRegime::default(), &waiting_ctx, "BATCH HARVEST SHOT (Execute Claim)", "waiting");

    // The plan is complete: detection keeps a waiting swap listed, and the advice says so.
    let spent = PlanFills { count: 5, spent_usdc: 100.0, acquired_nock: 8000.0, last_at: Some(t("2026-11-16T03:30:00Z")), first_price: Some(0.0125) };
    let done_ctx = PlanContext { cfg: Some(plan_cfg()), fills: spent, ledger_unavailable: false, now: t("2026-11-16T04:00:00Z") };
    check(&funded_raw(), &MarketRegime::default(), &done_ctx, "HOLD & ACCRETE (No NOCK LP Tracked)", "complete");

    // The plan's own arms already carry the notice: it is never said twice, including
    // below the floor, where it sits inside the recovery sentence.
    let fresh_ctx = PlanContext { cfg: Some(plan_cfg()), fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-16T04:00:00Z") };
    let floor = OperatorProfile { treasury_floor_nock: 100_000.0, ..OperatorProfile::default() };
    for profile in [OperatorProfile::default(), floor] {
        let d = metrics(&profile, &fresh_ctx, Some(&one_offer)).global_status.shot_advice_details;
        assert_eq!(d.matches("was found on-chain").count(), 1, "{d}");
    }

    // The helper by itself: nothing to say without a plan, without detection, or without offers.
    let text = "Some advice.".to_string();
    assert_eq!(lead_with_open_offer_notice(None, text.clone()), text);
    let mut ps = metrics(&OperatorProfile::default(), &done_ctx, Some(&no_offer)).accumulation_plan.unwrap();
    assert_eq!(lead_with_open_offer_notice(Some(&ps), text.clone()), text);
    ps.fill_detection = None;
    assert_eq!(lead_with_open_offer_notice(Some(&ps), text.clone()), text);
    ps.fill_detection = Some(one_offer.clone());
    assert_eq!(lead_with_open_offer_notice(Some(&ps), text.clone()), format!("{found} {text}"));
    // Already there: left alone.
    let said = format!("{found} {text}");
    assert_eq!(lead_with_open_offer_notice(Some(&ps), said.clone()), said);
}

/// Review finding on #118: the offer's cost was looked up at detection and the fill's
/// again at recording, so one swap could show two figures. A fill confirmed from an
/// offer keeps the offer's figure; anything else is looked up afresh.
#[test]
fn test_a_recorded_fill_keeps_the_reference_its_offer_showed() {
    let source = "market_price_history uniswap_rpc at 2026-11-16T03:05:00+00:00";
    let mut c = candidate(BUY_3, "offered", BUY_3_AT);
    c.effective_price_usd = Some("0.01123369".to_string());
    c.reference_spot_usd = Some("0.01120000".to_string());
    c.reference_spot_source = Some(source.to_string());
    c.execution_cost_bps = Some(30);
    assert_eq!(
        offered_reference(&c, "0.01123369"),
        Some(FillReference {
            spot_usd: Some("0.01120000".to_string()),
            source: Some(source.to_string()),
            cost_bps: Some(30),
        })
    );
    // The stored figure is the one fill_reference computes from the same two prices.
    assert_eq!(
        offered_reference(&c, "0.01123369").unwrap(),
        fill_reference("0.01123369", Some((0.0112, source.to_string())))
    );
    // Dismissed and then recorded by hand: still the figure the operator saw.
    c.state = "dismissed".to_string();
    assert!(offered_reference(&c, "0.01123369").is_some());
    // A different effective price is not this fill's cost: look it up afresh.
    assert_eq!(offered_reference(&c, "0.01200000"), None);
    // No reference on the offer (no stored price was near the block): nothing to reuse.
    let bare = candidate(BUY_3, "offered", BUY_3_AT);
    assert_eq!(offered_reference(&bare, "0.01250000"), None);
    // Half a reference is no reference.
    let mut half = candidate(BUY_3, "offered", BUY_3_AT);
    half.execution_cost_bps = Some(30);
    assert_eq!(offered_reference(&half, "0.01250000"), None);
    // A row that is not a purchase carries no price at all.
    let other = FillCandidate { tx_hash: BUY_3.to_string(), state: "not_a_purchase".to_string(), ..Default::default() };
    assert_eq!(offered_reference(&other, "0.01123369"), None);
}

#[test]
fn test_after_buying_text_follows_the_detection_state() {
    let ctx = PlanContext { cfg: Some(plan_cfg()), fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-16T04:00:00Z") };
    let legacy = "After buying, record the transaction hash so the plan advances.";

    // No detection result (callers that did not run it): the text is as it was.
    let plain = compute_derived_metrics_with_plan(
        &funded_raw(), 0.0, &OperatorProfile::default(), StructuralChannels { channels: vec![] }, &[],
        &LedgerAggregate::default(), &MarketRegime::default(), &ctx,
    );
    assert!(plain.global_status.shot_advice_details.ends_with(legacy));
    assert_eq!(plain.accumulation_plan.unwrap().fill_detection, None);

    let ok = metrics(&OperatorProfile::default(), &ctx, Some(&detection_ok(vec![], "2026-11-16T04:00:00+00:00")));
    let d = &ok.global_status.shot_advice_details;
    assert!(d.contains("NockNock looks for the swap on-chain and offers it for confirmation"), "{d}");
    assert!(d.contains("or record the transaction hash yourself"), "the manual path is always named: {d}");

    // Could not check: say so, and fall back to the hash.
    let down = FillDetection::unavailable("the wallet's transfer history could not be read");
    let m = metrics(&OperatorProfile::default(), &ctx, Some(&down));
    let d = &m.global_status.shot_advice_details;
    assert!(d.ends_with("Swap detection could not check this cycle, so after buying record the transaction hash so the plan advances."), "{d}");
    assert_eq!(m.global_status.shot_advice_action, "DCA TRANCHE DUE (1 of 5)");
    assert_eq!(m.accumulation_plan.unwrap().fill_detection.unwrap().status, "unavailable");

    let off = metrics(&OperatorProfile::default(), &ctx, Some(&FillDetection::not_run("swap detection is turned off")));
    assert!(off.global_status.shot_advice_details.ends_with(legacy));
}

#[test]
fn test_check_time_does_not_enter_the_state_hash() {
    let ctx = PlanContext { cfg: Some(plan_cfg()), fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-16T04:00:00Z") };
    let data = |detection: &FillDetection| SnapshotData {
        ladders: Vec::new(),
        closed_positions: vec![],
        wallet_balances: vec![],
        positions: vec![],
        pool_mechanics: vec![],
        concentrated_liquidity: vec![],
        contract_fees: vec![],
        derived_metrics: metrics(&OperatorProfile::default(), &ctx, Some(detection)),
    };
    let offers = real_offers(&[BUY_1, BUY_2]);
    let a = data(&detection_ok(offers.clone(), "2026-11-16T04:00:00+00:00"));
    let b = data(&detection_ok(offers.clone(), "2026-11-16T04:05:00+00:00"));
    assert_ne!(
        serde_json::to_value(&a).unwrap(), serde_json::to_value(&b).unwrap(),
        "the two snapshots do differ, in the check time"
    );
    assert_eq!(compute_state_hash(&a), compute_state_hash(&b), "a new check time alone must not store a new row");

    // Everything else about detection is state: a new offer, or a failed check, changes the hash.
    let none = data(&detection_ok(vec![], "2026-11-16T04:00:00+00:00"));
    assert_ne!(compute_state_hash(&a), compute_state_hash(&none));
    let mut down = detection_ok(offers, "2026-11-16T04:00:00+00:00");
    down.status = "unavailable".to_string();
    down.detail = Some("the wallet's transfer history could not be read".to_string());
    assert_ne!(compute_state_hash(&a), compute_state_hash(&data(&down)));
    // The offer's own block time is a fact about the swap and stays in the hash input.
    let canonical = canonical_state_json(&a);
    let fd = &canonical["derived_metrics"]["accumulation_plan"]["fill_detection"];
    assert!(fd.get("last_checked_at").is_none());
    assert_eq!(fd["offers"][0]["confirmed_at"], "2026-11-16T03:08:37+00:00");
}

#[test]
fn test_snapshot_carries_offers_and_older_snapshots_still_parse() {
    let ctx = PlanContext { cfg: Some(plan_cfg()), fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-16T04:00:00Z") };
    let mut detection = detection_ok(real_offers(&[BUY_1, BUY_2]), "2026-11-16T04:00:00+00:00");
    detection.offers[0].reference_spot_usd = Some("0.01120000".to_string());
    detection.offers[0].reference_spot_source = Some("market_price_history uniswap_rpc at 2026-11-16T03:05:00+00:00".to_string());
    detection.offers[0].execution_cost_bps = Some(30);
    let m = metrics(&OperatorProfile::default(), &ctx, Some(&detection));
    let plan = serde_json::to_value(&m).unwrap()["accumulation_plan"].clone();
    assert_eq!(
        plan["fill_detection"],
        json!({
            "status": "ok",
            "detail": null,
            "checked_back_to": "2026-11-14T00:00:00+00:00",
            "last_checked_at": "2026-11-16T04:00:00+00:00",
            "offers": [{
                "tx_hash": BUY_3,
                "confirmed_at": "2026-11-16T03:08:37+00:00",
                "block_number": 58040708,
                "usdc_spent": "20",
                "nock_received": "1780.3592976960049083",
                "effective_price_usd": "0.01123369",
                "timing": "on_schedule",
                "reference_spot_usd": "0.01120000",
                "reference_spot_source": "market_price_history uniswap_rpc at 2026-11-16T03:05:00+00:00",
                "execution_cost_bps": 30
            }],
            "dismissed": []
        })
    );
    // A stored snapshot from before v0.6.0 has no fill_detection: it reads as "not reported".
    let mut old = plan.clone();
    old.as_object_mut().unwrap().remove("fill_detection");
    let parsed: AccumulationPlanStatus = serde_json::from_value(old).unwrap();
    assert_eq!(parsed.fill_detection, None);
    let round_trip: AccumulationPlanStatus = serde_json::from_value(plan).unwrap();
    assert_eq!(round_trip.fill_detection, Some(detection));
}

// ---------------- MCP text (read-only) ----------------

fn plan_text(detection: Option<&FillDetection>) -> String {
    let ctx = PlanContext { cfg: Some(plan_cfg()), fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-16T04:00:00Z") };
    let m = metrics(&OperatorProfile::default(), &ctx, detection);
    format_accumulation_plan_text(&json!({ "derived_metrics": serde_json::to_value(&m).unwrap() }))
}

#[test]
fn test_mcp_plan_text_lists_offers_and_never_confuses_could_not_check_with_none_found() {
    // Checked, nothing found.
    let none = plan_text(Some(&detection_ok(vec![], "2026-11-16T04:00:00+00:00")));
    assert!(
        none.contains("• Swap detection: checked 2026-11-16T04:00:00+00:00 back to 2026-11-14T00:00:00+00:00; no uncounted swap found\n"),
        "{none}"
    );

    // Could not check: never "none found".
    let mut down = FillDetection::unavailable("the wallet's transfer history could not be read (the request timed out)");
    down.last_checked_at = Some("2026-11-16T03:55:00+00:00".to_string());
    let text = plan_text(Some(&down));
    assert!(text.contains("• Swap detection: could not check this cycle (the wallet's transfer history could not be read"), "{text}");
    assert!(text.contains("last successful check: 2026-11-16T03:55:00+00:00"), "{text}");
    assert!(!text.contains("no uncounted swap found"), "{text}");
    let never = plan_text(Some(&FillDetection::unavailable("no Alchemy RPC key is configured")));
    assert!(never.contains("last successful check: none on record"), "{never}");

    // Offers: the full hash, the amounts, the price, the cost when known, the timing.
    let mut offers = real_offers(&[BUY_1]);
    offers[1].reference_spot_usd = Some("0.01120000".to_string());
    offers[1].reference_spot_source = Some("market_price_history uniswap_rpc at 2026-11-16T03:05:00+00:00".to_string());
    offers[1].execution_cost_bps = Some(30);
    let mut detection = detection_ok(offers, "2026-11-16T04:00:00+00:00");
    detection.dismissed = real_offers(&[BUY_2, BUY_3]);
    let text = plan_text(Some(&detection));
    assert!(text.contains("• 2 detected swap(s) not counted yet."), "{text}");
    assert!(text.contains("(MCP is read-only)"), "{text}");
    assert!(
        text.contains(&format!(
            "   ? {BUY_2} · 2026-11-15T02:58:46+00:00 · $20 USDC → 1720.2181466216939446 NOCK at $0.01162643 · cost over spot unavailable · early (less than one plan interval after the buy before it)\n"
        )),
        "{text}"
    );
    // The cost names its reference and the reference's own time, in the text itself.
    assert!(
        text.contains(&format!(
            "   ? {BUY_3} · 2026-11-16T03:08:37+00:00 · $20 USDC → 1780.3592976960049083 NOCK at $0.01123369 · +30 bps against the stored price $0.01120000 (market_price_history uniswap_rpc at 2026-11-16T03:05:00+00:00) · on schedule\n"
        )),
        "{text}"
    );
    // A figure whose source was not stored says so rather than passing as a bare fact.
    let mut bare = detection.clone();
    bare.offers[1].reference_spot_source = None;
    assert!(
        plan_text(Some(&bare)).contains("+30 bps against the stored price $0.01120000 (reference source not recorded) · on schedule\n"),
        "{}", plan_text(Some(&bare))
    );
    assert!(text.contains(&format!("   ✗ not part of the plan (dismissed, restorable): {BUY_1} · ")), "{text}");
    // Progress is still zero: the offers are listed, not counted.
    assert!(text.contains("Progress: $0.00 of $100.00 spent over 0 verified buy(s)"), "{text}");

    // Partial, not run, and a snapshot that reports no detection at all.
    let mut partial = detection_ok(vec![], "2026-11-16T04:00:00+00:00");
    partial.status = "partial".to_string();
    partial.detail = Some("1 transaction(s) could not be read and will be tried again next cycle".to_string());
    let text = plan_text(Some(&partial));
    assert!(text.contains("incompletely: 1 transaction(s) could not be read"), "{text}");
    assert!(!text.contains("no uncounted swap found"), "{text}");
    let off = plan_text(Some(&FillDetection::not_run("swap detection is turned off (fill_detection.enabled is false)")));
    assert!(off.contains("• Swap detection: not run (swap detection is turned off"), "{off}");
    assert!(!plan_text(None).contains("Swap detection"));
    assert_eq!(fill_detection_lines(&json!({ "plan_id": "p" })), "");
    assert_eq!(fill_detection_lines(&json!({ "fill_detection": null })), "");
}

#[test]
fn test_mcp_gains_no_write_tool() {
    // The worker's MCP surface stays four argument-less, read-only tools.
    let src = include_str!("mcp_http.rs");
    assert_eq!(src.matches("\"inputSchema\"").count(), 4, "tools listed by /v1/mcp");
    assert_eq!(src.matches("\"required\": []").count(), 4, "none of them takes an argument");
    for banned in ["confirm", "dismiss", "restore", "record_"] {
        assert!(!src.to_lowercase().contains(&format!("\"name\": \"{banned}")), "{banned}");
    }
}

// ---------------- SQL, against a scratch database ----------------
// The queries in fill_detection.rs are runtime strings (no compile-time check), so this
// test runs them for real against a scratch PostgreSQL. It is #[ignore]d, so a normal
// `cargo test` reports it as ignored rather than as a pass it did not earn:
//   NOCKNOCK_SCRATCH_DATABASE_URL=postgres://… cargo test scratch_database -- --ignored
// It refuses any database whose name does not end in "_scratch": it must never touch a
// deployment's database. The schema must already be migrated (the migrations
// directory, applied in order).

async fn body_json(resp: Response) -> (StatusCode, serde_json::Value) {
    let status = resp.status();
    let bytes = axum::body::to_bytes(resp.into_body(), 1 << 20).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap())
}

#[tokio::test]
#[ignore = "needs NOCKNOCK_SCRATCH_DATABASE_URL (a migrated scratch PostgreSQL)"]
async fn test_fill_detection_sql_against_a_scratch_database() {
    let url = std::env::var("NOCKNOCK_SCRATCH_DATABASE_URL").expect("NOCKNOCK_SCRATCH_DATABASE_URL is set");
    let db = PgPool::connect(&url).await.expect("scratch database reachable");
    let name: String = sqlx::query("SELECT current_database() AS n").fetch_one(&db).await.unwrap().get("n");
    assert!(name.ends_with("_scratch"), "refusing to run against database {name:?}: its name must end in _scratch");
    // Each test migrates its own database, so it passes alone and on an empty one.
    sqlx::migrate!("./migrations").run(&db).await.expect("every migration applies");

    let wallet = WALLET.to_lowercase();
    let plan_id = "scratch-fill-detection";
    let price_points = [
        (t("2026-11-16T03:05:00Z"), 0.0112f64),
        (t("2026-11-14T06:00:00Z"), 0.01229f64),
        // Stored later, nearer the third buy than the price its offer was shown with.
        (t("2026-11-16T03:08:00Z"), 0.0113f64),
    ];
    let cleanup = |db: PgPool| async move {
        sqlx::query("DELETE FROM accumulation_candidates WHERE wallet = $1").bind(WALLET.to_lowercase()).execute(&db).await.unwrap();
        sqlx::query("DELETE FROM accumulation_events WHERE plan_id = $1").bind(plan_id).execute(&db).await.unwrap();
        sqlx::query("DELETE FROM accumulation_plans WHERE plan_id = $1").bind(plan_id).execute(&db).await.unwrap();
        for (ts, _) in price_points {
            sqlx::query("DELETE FROM market_price_history WHERE timestamp = $1").bind(ts).execute(&db).await.unwrap();
        }
    };
    cleanup(db.clone()).await;

    // One stored price three minutes before the third buy; none near the other two yet.
    insert_price_point(&db, price_points[0].0, price_points[0].1, PRICE_SOURCE_RPC, false).await.unwrap();

    // Classify the real history and store every verdict, twice: a hash is stored once.
    let mut scan = TransferScan::new(anchor());
    scan.absorb(parse_asset_transfers(&transfers_response()).unwrap());
    for _ in 0..2 {
        for hint in &scan.hints {
            let r = receipt(&hint.tx_hash);
            let block_time = verify_fill_receipt(&r, &wallet).ok().and_then(|s| block_time_from_response(&block_header(s.block_number)));
            let verdict = classify_candidate(Some(&r), &wallet, block_time, anchor());
            store_verdict(&db, &wallet, &hint.tx_hash, &verdict).await.unwrap();
        }
    }
    // An unread receipt stores nothing.
    let unread = format!("0x{}", "ab".repeat(32));
    store_verdict(&db, &wallet, &unread, &CandidateVerdict::Unread).await.unwrap();
    let known = known_candidate_hashes(&db, &wallet).await.unwrap();
    assert_eq!(known.len(), 9);
    assert!(!known.contains(&unread));

    let mut cfg = plan_cfg();
    cfg.id = plan_id.to_string();
    cfg.starts_at = Some(anchor());
    let state_with = |yaml: &str| -> Arc<AppState> {
        let mut config: AppConfig = serde_yaml::from_str(yaml).unwrap();
        config.tracked_wallets = vec![WALLET.to_string()];
        config.accumulation_plan = Some(cfg.clone());
        Arc::new(AppState {
            db: db.clone(),
            http_client: Client::new(),
            config: Arc::new(config),
            latest_snapshot: Mutex::new(None),
            last_sync_duration_ms: std::sync::atomic::AtomicU64::new(0),
            last_ingest_error: std::sync::Mutex::new(None),
        cycle_gate: CycleGate::default(),
        quote_refresh_at: std::sync::Mutex::new(None),
        })
    };
    // No Alchemy key: the transfer history cannot be read. No network call is made.
    let state = state_with("alchemy_api_keys: [\"YOUR_ALCHEMY_API_KEY\"]\n");
    let ctx = PlanContext { cfg: Some(cfg.clone()), fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-16T04:00:00Z") };

    // Could not check, said so; the swaps an earlier check found are still offered.
    let d = detect_fills(&state, &ctx).await.expect("a plan is configured");
    assert_eq!(d.status, "unavailable");
    assert_eq!(d.detail.as_deref(), Some("no Alchemy RPC key is configured"));
    assert_eq!((d.last_checked_at.clone(), d.checked_back_to.clone()), (None, None));
    assert_eq!(d.offers.iter().map(|o| o.tx_hash.as_str()).collect::<Vec<_>>(), vec![BUY_1, BUY_2, BUY_3]);
    assert!(d.dismissed.is_empty());
    // Cost over spot only where a stored price was near the block.
    let third = &d.offers[2];
    assert_eq!(third.reference_spot_usd.as_deref(), Some("0.01120000"));
    assert_eq!(third.execution_cost_bps, Some(30));
    assert!(third.reference_spot_source.as_ref().unwrap().contains("uniswap_rpc at 2026-11-16T03:05:00"), "{third:?}");
    assert_eq!((d.offers[0].reference_spot_usd.clone(), d.offers[0].execution_cost_bps), (None, None));

    // Dismiss, idempotently; the swap moves to the dismissed list and stays there.
    let act = |hash: &str| Json(serde_json::from_value::<CandidateActionRequest>(json!({ "tx_hash": hash })).unwrap());
    let (status, body) = body_json(dismiss_candidate_handler(Extension(state.clone()), act(BUY_2)).await.into_response()).await;
    assert_eq!((status, body["state"].as_str(), body["changed"].as_bool()), (StatusCode::OK, Some("dismissed"), Some(true)));
    let upper = format!("0x{}", BUY_2.trim_start_matches("0x").to_uppercase());
    let (status, body) = body_json(dismiss_candidate_handler(Extension(state.clone()), act(&upper)).await.into_response()).await;
    assert_eq!((status, body["changed"].as_bool(), body["tx_hash"].as_str()), (StatusCode::OK, Some(false), Some(BUY_2)));
    let d = detect_fills(&state, &ctx).await.unwrap();
    assert_eq!(d.offers.iter().map(|o| o.tx_hash.as_str()).collect::<Vec<_>>(), vec![BUY_1, BUY_3]);
    assert_eq!(d.dismissed.iter().map(|o| o.tx_hash.as_str()).collect::<Vec<_>>(), vec![BUY_2]);
    // Restore puts it back on offer.
    let (status, body) = body_json(restore_candidate_handler(Extension(state.clone()), act(BUY_2)).await.into_response()).await;
    assert_eq!((status, body["state"].as_str(), body["changed"].as_bool()), (StatusCode::OK, Some("offered"), Some(true)));
    assert_eq!(detect_fills(&state, &ctx).await.unwrap().offers.len(), 3);

    // Refusals: not a hash, never detected, a fee claim.
    let (status, body) = body_json(dismiss_candidate_handler(Extension(state.clone()), act("0x1234")).await.into_response()).await;
    assert_eq!((status, body["error"].as_str()), (StatusCode::UNPROCESSABLE_ENTITY, Some("INVALID_TX_HASH")));
    let (status, body) = body_json(dismiss_candidate_handler(Extension(state.clone()), act(&unread)).await.into_response()).await;
    assert_eq!((status, body["error"].as_str()), (StatusCode::NOT_FOUND, Some("CANDIDATE_NOT_FOUND")));
    let (status, body) = body_json(restore_candidate_handler(Extension(state.clone()), act(V4_CLAIMS[0])).await.into_response()).await;
    assert_eq!((status, body["error"].as_str()), (StatusCode::CONFLICT, Some("NOT_AN_OFFER")));

    // The first buy is recorded (as the record endpoint would write it, before v0.6.0:
    // no reference yet). It is no longer offered and can no longer be dismissed.
    sqlx::query(
        "INSERT INTO accumulation_events (plan_id, tx_hash, venue, usdc_spent_raw, nock_received_raw, usdc_spent, nock_received, effective_price_usd, block_number, confirmed_at) \
         VALUES ($1, $2, 'Aero', '20000000', '15979340475636968138', '20', '1597.9340475636968138', '0.01251616', 57228848, $3)",
    )
    .bind(plan_id)
    .bind(BUY_1)
    .bind(t("2026-11-14T06:02:25Z"))
    .execute(&db)
    .await
    .unwrap();
    assert!(recorded_fill_hashes(&db).await.unwrap().contains(BUY_1));
    // The same ledger read with the plan's own fill times, which place each offer's
    // early / on-schedule label. Another plan id sees the hash but no cadence.
    let (hashes, times) = recorded_fills(&db, plan_id).await.unwrap();
    assert!(hashes.contains(BUY_1));
    assert_eq!(times, vec![t(BUY_1_AT)]);
    let (hashes, times) = recorded_fills(&db, "some-other-plan").await.unwrap();
    assert!(hashes.contains(BUY_1));
    assert!(times.is_empty());
    let fills = PlanFills { count: 1, spent_usdc: 20.0, acquired_nock: 1597.93, last_at: Some(t("2026-11-14T06:02:25Z")), first_price: Some(0.01251616) };
    let after = PlanContext { cfg: Some(cfg.clone()), fills, ledger_unavailable: false, now: t("2026-11-16T04:00:00Z") };
    let d = detect_fills(&state, &after).await.unwrap();
    assert_eq!(
        d.offers.iter().map(|o| (o.tx_hash.as_str(), o.timing.as_str())).collect::<Vec<_>>(),
        vec![(BUY_2, "early"), (BUY_3, "on_schedule")]
    );
    let (status, body) = body_json(dismiss_candidate_handler(Extension(state.clone()), act(BUY_1)).await.into_response()).await;
    assert_eq!((status, body["error"].as_str()), (StatusCode::CONFLICT, Some("ALREADY_RECORDED")));

    // The recorded fill has no cost figure until a price near its block is stored; then
    // the next cycle fills it in, once.
    let cost = |db: PgPool| async move {
        let r = sqlx::query("SELECT reference_spot_usd, reference_spot_source, execution_cost_bps FROM accumulation_events WHERE tx_hash = $1")
            .bind(BUY_1)
            .fetch_one(&db)
            .await
            .unwrap();
        (r.get::<Option<String>, _>("reference_spot_usd"), r.get::<Option<i32>, _>("execution_cost_bps"))
    };
    assert_eq!(cost(db.clone()).await, (None, None));
    insert_price_point(&db, price_points[1].0, price_points[1].1, PRICE_SOURCE_RPC, false).await.unwrap();
    detect_fills(&state, &after).await.unwrap();
    assert_eq!(cost(db.clone()).await, (Some("0.01229000".to_string()), Some(184)));

    // The events and candidates endpoints report it.
    let (status, body) = body_json(accumulation_events_handler(Extension(state.clone())).await.into_response()).await;
    assert_eq!(status, StatusCode::OK);
    let mine: Vec<&serde_json::Value> = body["events"].as_array().unwrap().iter().filter(|e| e["plan_id"] == plan_id).collect();
    assert_eq!(mine.len(), 1);
    assert_eq!((mine[0]["execution_cost_bps"].as_i64(), mine[0]["reference_spot_usd"].as_str()), (Some(184), Some("0.01229000")));
    let (status, body) = body_json(accumulation_candidates_handler(Extension(state.clone())).await.into_response()).await;
    assert_eq!(status, StatusCode::OK);
    let rows = body["candidates"].as_array().unwrap();
    assert_eq!(rows.len(), 9);
    assert_eq!(rows[0]["tx_hash"], BUY_3, "newest purchase first; rows without a time last");
    let first_buy = rows.iter().find(|r| r["tx_hash"] == BUY_1).unwrap();
    assert_eq!((first_buy["recorded"].as_bool(), first_buy["state"].as_str()), (Some(true), Some("offered")));
    assert_eq!(rows.iter().filter(|r| r["state"] == "not_a_purchase" && r["reason"] == "NOT_A_NOCK_PURCHASE").count(), 6);

    // A fill recorded from an offer keeps the figure the offer showed, even when a
    // nearer price has been stored since. Wallet and hash are matched without case.
    insert_price_point(&db, price_points[2].0, price_points[2].1, PRICE_SOURCE_RPC, false).await.unwrap();
    let afresh = reference_for_fill(&db, "0.01123369", t(BUY_3_AT)).await;
    assert_eq!((afresh.spot_usd.as_deref(), afresh.cost_bps), (Some("0.01130000"), Some(-59)));
    let upper_3 = format!("0x{}", BUY_3.trim_start_matches("0x").to_uppercase());
    let kept = reference_for_recorded_fill(&db, WALLET, &upper_3, "0.01123369", t(BUY_3_AT)).await;
    assert_eq!((kept.spot_usd.as_deref(), kept.cost_bps), (Some("0.01120000"), Some(30)));
    assert!(kept.source.as_ref().unwrap().contains("uniswap_rpc at 2026-11-16T03:05:00"), "{kept:?}");
    // The offer itself is unchanged: a row is written once.
    let d = detect_fills(&state, &after).await.unwrap();
    assert_eq!(d.offers.last().map(|o| (o.tx_hash.as_str(), o.execution_cost_bps)), Some((BUY_3, Some(30))));
    // A hash that was never offered, or an offer shown without a figure, is looked up.
    assert_eq!(reference_for_recorded_fill(&db, WALLET, &unread, "0.01123369", t(BUY_3_AT)).await, afresh);
    let none_near = reference_for_recorded_fill(&db, WALLET, BUY_2, "0.01162643", t(BUY_2_AT)).await;
    assert_eq!(none_near, FillReference::default());

    // Budget spent: not run, but a swap still waiting stays listed. Ledger unreadable or
    // detection switched off: not run, nothing listed.
    let spent = PlanFills { count: 5, spent_usdc: 100.0, acquired_nock: 8000.0, last_at: Some(t("2026-11-16T03:30:00Z")), first_price: Some(0.0125) };
    let done = PlanContext { cfg: Some(cfg.clone()), fills: spent, ledger_unavailable: false, now: t("2026-11-16T04:00:00Z") };
    let d = detect_fills(&state, &done).await.unwrap();
    assert_eq!((d.status.as_str(), d.offers.len()), ("not_run", 2));
    assert!(d.detail.unwrap().contains("budget is spent"));
    let blind = PlanContext { ledger_unavailable: true, ..ctx.clone() };
    let d = detect_fills(&state, &blind).await.unwrap();
    assert_eq!((d.status.as_str(), d.offers.len()), ("not_run", 0));
    let off = state_with("alchemy_api_keys: [\"YOUR_ALCHEMY_API_KEY\"]\nfill_detection:\n  enabled: false\n");
    let d = detect_fills(&off, &ctx).await.unwrap();
    assert_eq!((d.status.as_str(), d.offers.len()), ("not_run", 0));
    assert!(d.detail.unwrap().contains("turned off"));
    // No plan, no detection result at all.
    assert_eq!(detect_fills(&state, &PlanContext::none()).await, None);

    cleanup(db.clone()).await;
}

// ---------------- the stored price that stands for a time (#129) ----------------

/// Always runs: the statement must read a candle as its open and prefer the worker's own
/// read. The scratch-database test below runs it for real.
#[test]
fn test_stored_price_statement_reads_a_candle_as_its_open_and_prefers_the_workers_read() {
    let sql = NOCK_PRICE_NEAR_SQL;
    assert!(sql.contains("COALESCE(open_usd, price_usd) AS price_usd"), "{sql}");
    assert!(sql.contains("COALESCE(open_usd, price_usd) > 0"), "{sql}");
    let order = sql.split("ORDER BY").nth(1).expect("an ORDER BY");
    assert!(order.trim_start().starts_with("is_backfilled ASC,"), "the worker's own read must come first: {order}");
    assert!(!sql.contains("close_usd"), "{sql}");
}

#[tokio::test]
#[ignore = "needs NOCKNOCK_SCRATCH_DATABASE_URL (an empty, migrated scratch PostgreSQL)"]
async fn test_stored_price_lookup_against_a_scratch_database() {
    let url = std::env::var("NOCKNOCK_SCRATCH_DATABASE_URL").expect("NOCKNOCK_SCRATCH_DATABASE_URL is set");
    let db = PgPool::connect(&url).await.expect("scratch database reachable");
    let name: String = sqlx::query("SELECT current_database() AS n").fetch_one(&db).await.unwrap().get("n");
    assert!(name.ends_with("_scratch"), "refusing to run against database {name:?}: its name must end in _scratch");
    // Each test migrates its own database, so it passes alone and on an empty one.
    sqlx::migrate!("./migrations").run(&db).await.expect("every migration applies");

    // Far from any real price row: a day in 2001.
    let open_at = t("2001-03-04T03:00:00Z");
    let own_read_at = t("2001-03-04T03:40:00Z");
    let swap_at = t("2001-03-04T02:58:46Z");
    let cleanup = |db: PgPool| async move {
        sqlx::query("DELETE FROM market_price_history WHERE timestamp BETWEEN $1 AND $2")
            .bind(t("2001-03-04T00:00:00Z"))
            .bind(t("2001-03-05T00:00:00Z"))
            .execute(&db)
            .await
            .unwrap();
    };
    cleanup(db.clone()).await;

    // An hourly candle opening 74 seconds after the swap: open 0.0100, close 0.0200.
    let candle = PriceCandle { timestamp: open_at, open: 0.0100, high: 0.0210, low: 0.0095, close: 0.0200, volume: None };
    let added = upsert_price_candles(&db, &[candle], "geckoterminal", chrono::Duration::hours(1), t("2001-03-05T00:00:00Z")).await.unwrap();
    assert_eq!(added, 1);

    // The candle stands for its open time with its OPEN, never its close.
    let (price, source) = nock_price_near(&db, swap_at).await.expect("a stored price within the window");
    assert_eq!(price, 0.0100, "{source}");
    assert!(source.contains("geckoterminal at 2001-03-04T03:00:00"), "{source}");

    // The same rule reaches a fill's reference spot and execution cost.
    let reference = reference_for_fill(&db, "0.0101", swap_at).await;
    assert_eq!(reference.spot_usd.as_deref(), Some("0.01000000"));
    assert_eq!(reference.cost_bps, Some(100));
    assert!(reference.source.unwrap().contains("03:00:00"));

    // The worker's own read inside the window wins, even though the candle is nearer.
    insert_price_point(&db, own_read_at, 0.0150, PRICE_SOURCE_RPC, false).await.unwrap();
    let (price, source) = nock_price_near(&db, swap_at).await.unwrap();
    assert_eq!(price, 0.0150, "{source}");
    assert!(source.contains("2001-03-04T03:40:00"), "{source}");

    // A candle with no positive open gives no price rather than its close.
    cleanup(db.clone()).await;
    let bad = PriceCandle { timestamp: open_at, open: 0.0, high: 0.02, low: 0.0, close: 0.0200, volume: None };
    upsert_price_candles(&db, &[bad], "geckoterminal", chrono::Duration::hours(1), t("2001-03-05T00:00:00Z")).await.unwrap();
    assert!(nock_price_near(&db, swap_at).await.is_none());

    cleanup(db).await;
}
