//! Closed positions (issue #144): a withdrawn position reports as closed instead of
//! stalling ingestion.
//!
//! Every chain answer here is real, read on Base on 2026-11-17 through
//! https://base-rpc.publicnode.com (ONCHAIN_INTEGRATION.md §13):
//! - the `REAL_*` constants are the JSON-RPC responses for the withdrawn Aero CL NFT
//!   #2914490 and for a burned Uniswap v3 and Uniswap v4 NFT, as the node sent them;
//! - `testdata/closed/cycle_fixture.json` holds every read one ingestion cycle makes for
//!   the tracked wallet, pinned to one block after the withdrawal. It is recorded by
//!   `test_record_closed_cycle_fixture` (ignored; it needs the network), which runs the
//!   worker's own `fetch_raw_data_from` through a recording proxy, so the requests are
//!   the worker's, byte for byte. Two things in it are not a node's own bytes and are
//!   marked as such in the file: `alchemy_getTokenBalances` (an Alchemy method the
//!   public node does not serve) is assembled from two real `balanceOf` reads at the
//!   pinned block, and `eth_gasPrice` is the price at recording time (it has no block).
//!
//! The cycle tests replay that file from a local JSON-RPC server. A request the file
//! does not hold is answered with a JSON-RPC error, never an invented value, so a
//! missing read fails the test's cycle loudly. The negative tests swap ONE answer.
//! The file also holds that block's own header (`eth_getBlockByNumber`): a closure is
//! proven at one current block, so the replayed cycle runs on the clock of that block
//! (`read_clock`), and the stale-node tests move the clock, not the chain's answer.
//!
//! The config entries are those of config.yaml.example (ids, pools, ticks), plus the
//! commented Aero CL example entry, which is the one that was withdrawn. Synthetic, and
//! labelled where used: the test profile, the `-gone` and `-retired` entries' ids, and
//! the answers the negative tests substitute (an HTTP status, a JSON-RPC error, a
//! dropped connection).

use crate::*;
use super::tests_aero_rewards as before;
use std::collections::BTreeMap;

const WALLET: &str = "0xeE114cC97eC5adeB34bA5645e315B497D8487a45";
const CL_NPM: &str = "0x4857db576ee0e45f10525ec59527bfd10beec60b";
const CL_POOL: &str = "0x957256c98d61A714081DE2EE70ae3A96c9b40874";
const CL_GAUGE: &str = "0x3bf70c880adc8370917994ab0df68e763b7a60fd";
const CL_NFT: u64 = 2914490;
const V3_NPM: &str = "0x03a520b32C04BF3bEEf7BEb72E919cf822Ed34f1";
/// A Uniswap v3 NFT burned at block 58632803 (tx 0x64e76755…cf68f3), by another account.
const V3_BURNED_NFT: u64 = 6604113;
/// A Uniswap v4 NFT burned at block 58641149 (tx 0x7af0d60c…c63819), by another account.
const V4_BURNED_NFT: u64 = 3229273;
/// A Uniswap v4 NFT the tracked wallet still holds, with zero liquidity: NOT closed.
const V4_EMPTY_NFT: u64 = 3163431;
/// The legacy Slipstream position manager: its answers are not on the allow-list.
const LEGACY_CL_NPM: &str = "0x827922686190790b37229fd06084350E74485b72";

// ------------------------------------------------------------------
// The real answers (publicnode, 2026-11-17; mainnet.base.org gave the same bytes)
// ------------------------------------------------------------------

/// Aero CL position manager, `ownerOf(2914490)` after the burn in block 58637717.
const REAL_CL_OWNER_OF_BURNED: &str = r#"{"jsonrpc":"2.0","id":1,"error":{"code":3,"message":"execution reverted: ERC721: owner query for nonexistent token","data":"0x08c379a0000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000294552433732313a206f776e657220717565727920666f72206e6f6e6578697374656e7420746f6b656e0000000000000000000000000000000000000000000000"}}"#;
/// Aero CL position manager, `positions(2914490)` after the burn.
const REAL_CL_POSITIONS_BURNED: &str = r#"{"jsonrpc":"2.0","id":1,"error":{"code":3,"message":"execution reverted: ID","data":"0x08c379a0000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000024944000000000000000000000000000000000000000000000000000000000000"}}"#;
/// Aero CL position manager, `ownerOf(2914490)` at block 58637716, one block before the
/// burn: the gauge held it. The answer of a LIVE staked position.
const REAL_CL_OWNER_OF_BEFORE_BURN: &str =
    r#"{"jsonrpc":"2.0","id":1,"result":"0x0000000000000000000000003bf70c880adc8370917994ab0df68e763b7a60fd"}"#;
/// Uniswap v3 NPM, `positions(6604113)` after its burn.
const REAL_V3_POSITIONS_BURNED: &str = r#"{"jsonrpc":"2.0","id":1,"error":{"code":3,"message":"execution reverted: Invalid token ID","data":"0x08c379a000000000000000000000000000000000000000000000000000000000000000200000000000000000000000000000000000000000000000000000000000000010496e76616c696420746f6b656e20494400000000000000000000000000000000"}}"#;
/// Uniswap v4 PositionManager, `ownerOf(3229273)` after its burn.
const REAL_V4_OWNER_OF_BURNED: &str = r#"{"jsonrpc":"2.0","id":1,"error":{"code":3,"message":"execution reverted: NOT_MINTED","data":"0x08c379a00000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000a4e4f545f4d494e54454400000000000000000000000000000000000000000000"}}"#;
/// Uniswap v4 PositionManager, `getPoolAndPositionInfo(3229273)` after its burn: six
/// zero words, a successful call.
const REAL_V4_INFO_BURNED: &str = "0x000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000000";
/// Uniswap v4 PositionManager, `getPoolAndPositionInfo(3163431)`: the wallet's own
/// empty NFT. The pool key and the packed position info are stored.
const REAL_V4_INFO_EMPTY_BUT_EXISTING: &str = "0x000000000000000000000000833589fcd6edb6e08f4c7c32d4f71b54bda029130000000000000000000000009b5e262cf9bb04869ab40b19af91d2dc857617220000000000000000000000000000000000000000000000000000000000001d4c000000000000000000000000000000000000000000000000000000000000004b000000000000000000000000000000000000000000000000000000000000000091a9ef0a8dd231b99fa53a2a145a9e8bca7897e0d85852107a0422ca04163100";
/// CL gauge, `earned(wallet, [2914490])` after the withdrawal: Error("NA"). A real
/// revert with a reason that has nothing to do with whether the NFT exists.
const REAL_GAUGE_EARNED_NA: &str = r#"{"jsonrpc":"2.0","id":1,"error":{"code":3,"message":"execution reverted: NA","data":"0x08c379a0000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000024e41000000000000000000000000000000000000000000000000000000000000"}}"#;

fn outcome(response: &str) -> CallOutcome {
    classify_eth_call_outcome(&serde_json::from_str(response).unwrap()).unwrap()
}

fn returned(hex: &str) -> CallOutcome {
    CallOutcome::Returned(hex.trim_start_matches("0x").to_string())
}

// ------------------------------------------------------------------
// The RPC layer keeps the revert reason
// ------------------------------------------------------------------

#[test]
fn test_revert_data_is_kept_and_decoded_strictly() {
    let reason = |resp: &str| match outcome(resp) {
        CallOutcome::Reverted(data) => decode_revert_reason(data.as_deref()),
        CallOutcome::Returned(_) => panic!("not a revert"),
    };
    assert_eq!(reason(REAL_CL_OWNER_OF_BURNED), RevertReason::Message("ERC721: owner query for nonexistent token".to_string()));
    assert_eq!(reason(REAL_CL_POSITIONS_BURNED), RevertReason::Message("ID".to_string()));
    assert_eq!(reason(REAL_V3_POSITIONS_BURNED), RevertReason::Message("Invalid token ID".to_string()));
    assert_eq!(reason(REAL_V4_OWNER_OF_BURNED), RevertReason::Message("NOT_MINTED".to_string()));
    assert_eq!(reason(REAL_GAUGE_EARNED_NA), RevertReason::Message("NA".to_string()));
    assert_eq!(RevertReason::Message("ID".to_string()).describe(), "Error(\"ID\")");

    // The older reading of the same responses is unchanged: a revert is Ok(None).
    let v: serde_json::Value = serde_json::from_str(REAL_CL_POSITIONS_BURNED).unwrap();
    assert_eq!(classify_eth_call_response(&v).unwrap(), None);
    assert_eq!(
        classify_eth_call_response(&json!({"result": "0x01"})).unwrap().as_deref(),
        Some("01")
    );

    // Shapes that are not the contract's own reason (constructed from the JSON-RPC and
    // ABI rules; the real ones above are the only ones a test treats as proof).
    let rev = |data: serde_json::Value| {
        let mut err = json!({"code": 3, "message": "execution reverted"});
        if !data.is_null() {
            err["data"] = data;
        }
        match classify_eth_call_outcome(&json!({"jsonrpc": "2.0", "id": 1, "error": err})).unwrap() {
            CallOutcome::Reverted(d) => decode_revert_reason(d.as_deref()),
            CallOutcome::Returned(_) => unreachable!(),
        }
    };
    assert_eq!(rev(serde_json::Value::Null), RevertReason::NoData, "a node that sends no data");
    assert_eq!(rev(json!("0x")), RevertReason::Empty, "revert() with no data");
    assert_eq!(rev(json!({"nested": "0x08c379a0"})), RevertReason::NoData, "data that is not a hex string");
    assert_eq!(rev(json!("Reverted 0x08c379a0")), RevertReason::NoData, "text is not revert data");
    assert_eq!(rev(json!("0x1f2a2005")), RevertReason::Custom("0x1f2a2005".to_string()));
    assert_eq!(
        rev(json!("0x4e487b710000000000000000000000000000000000000000000000000000000000000011")),
        RevertReason::Panic("0x11".to_string())
    );
    // Error(string) that is truncated, padded with a non-zero byte, has a wrong offset
    // or a length that does not fit is Malformed, never a message.
    let id = "08c379a0000000000000000000000000000000000000000000000000000000000000002000000000000000000000000000000000000000000000000000000000000000024944000000000000000000000000000000000000000000000000000000000000";
    assert_eq!(decode_revert_reason(Some(id)), RevertReason::Message("ID".to_string()));
    assert_eq!(decode_revert_reason(Some(&id[..id.len() - 2])), RevertReason::Malformed);
    assert_eq!(decode_revert_reason(Some(&format!("{id}00"))), RevertReason::Malformed);
    assert_eq!(decode_revert_reason(Some(&id.replace("4944000000", "4944000001"))), RevertReason::Malformed);
    assert_eq!(
        decode_revert_reason(Some(&id.replacen("0000000000000000000000000000000000000000000000000000000000000020", "0000000000000000000000000000000000000000000000000000000000000040", 1))),
        RevertReason::Malformed
    );
    assert_eq!(decode_revert_reason(Some("08c379a0")), RevertReason::Malformed);
    assert_eq!(decode_revert_reason(Some("08c3")), RevertReason::Malformed);
    // A length word the data cannot hold is Malformed, whatever its size: the length is
    // bounded before any arithmetic, so 2^63 and above neither overflow nor slice out of
    // bounds (either would panic the ingestion task instead of failing the cycle).
    let head = format!("08c379a0{:064x}", 0x20);
    assert_eq!(decode_revert_reason(Some(&format!("{head}{:064x}4944", 3))), RevertReason::Malformed);
    assert_eq!(decode_revert_reason(Some(&format!("{head}{:064x}{}", 33, "49".repeat(32)))), RevertReason::Malformed);
    for len in [1u64 << 31, (1 << 63) - 1, 1 << 63, (1 << 63) + 1, u64::MAX - 31, u64::MAX] {
        for tail in ["", "4944", &"00".repeat(32), &"49".repeat(64)] {
            let data = format!("{head}{len:064x}{tail}");
            assert_eq!(decode_revert_reason(Some(&data)), RevertReason::Malformed, "length {len:#x} with {} bytes of text", tail.len() / 2);
            // The same payload as a node would send it, through both proofs: never closed.
            let out = CallOutcome::Reverted(Some(data));
            assert!(describe_outcome(&out).contains("does not decode"));
            assert!(matches!(nft_closure_verdict("aerodrome-cl", CL_NPM, &out, &outcome(REAL_CL_OWNER_OF_BURNED)), ClosureVerdict::NotProven(_)));
            assert!(matches!(nft_closure_verdict("aerodrome-cl", LEGACY_CL_NPM, &out, &out), ClosureVerdict::NotProven(_)));
            assert!(matches!(v4_closure_verdict(&out, &returned(REAL_V4_INFO_BURNED)), ClosureVerdict::NotProven(_)));
        }
    }
    // A length that fits is still read: the bound rejects nothing real.
    assert_eq!(decode_revert_reason(Some(&format!("{head}{:064x}", 0))), RevertReason::Message(String::new()));
    assert_eq!(decode_revert_reason(Some(&format!("{head}{:064x}{}", 32, "41".repeat(32)))), RevertReason::Message("A".repeat(32)));
    // None of these descriptions could be mistaken for a reason on the allow-list.
    for r in [RevertReason::NoData, RevertReason::Empty, RevertReason::Malformed, RevertReason::Custom("0x1f2a2005".into())] {
        assert!(!r.describe().contains("Error("), "{}", r.describe());
    }

    // A JSON-RPC error that is not an execution revert is no answer at all.
    for (code, message) in [(-32005, "rate limit exceeded"), (-32000, "header not found"), (429, "Too Many Requests"), (-32603, "internal error")] {
        let v = json!({"jsonrpc": "2.0", "id": 1, "error": {"code": code, "message": message}});
        let err = classify_eth_call_outcome(&v).unwrap_err().to_string();
        assert!(err.starts_with(&format!("RPC error {code}")), "{err}");
    }
    assert!(classify_eth_call_outcome(&json!({"jsonrpc": "2.0", "id": 1})).is_err());
}

// ------------------------------------------------------------------
// The proof: an allow-list, and two reads that must agree
// ------------------------------------------------------------------

#[test]
fn test_aero_cl_real_revert_answers_prove_the_position_closed() {
    let verdict = nft_closure_verdict("aerodrome-cl", CL_NPM, &outcome(REAL_CL_POSITIONS_BURNED), &outcome(REAL_CL_OWNER_OF_BURNED));
    let ClosureVerdict::Closed(evidence) = verdict else {
        panic!("the two real answers prove it: {verdict:?}");
    };
    assert_eq!(evidence.len(), 2);
    assert_eq!(evidence[0].call, "positions(uint256)");
    assert_eq!(evidence[0].contract, CL_NPM);
    assert_eq!(evidence[0].outcome, "reverted");
    assert_eq!(evidence[0].answer, "Error(\"ID\")");
    assert!(evidence[0].raw.starts_with("0x08c379a0") && evidence[0].raw.ends_with("024944000000000000000000000000000000000000000000000000000000000000"));
    assert_eq!(evidence[1].call, "ownerOf(uint256)");
    assert_eq!(evidence[1].answer, "Error(\"ERC721: owner query for nonexistent token\")");
    // The raw revert data is published as the node sent it.
    let sent: serde_json::Value = serde_json::from_str(REAL_CL_OWNER_OF_BURNED).unwrap();
    assert_eq!(evidence[1].raw, sent["error"]["data"].as_str().unwrap());

    // The manager's address is matched without regard to case or the 0x prefix.
    assert!(nft_nonexistence_rule("aerodrome-cl", &CL_NPM.to_uppercase().replace("0X", "0x")).is_some());
    assert!(nft_nonexistence_rule("aerodrome-cl", CL_NPM.trim_start_matches("0x")).is_some());
}

#[test]
fn test_uniswap_v3_and_v4_real_answers_prove_a_burned_nft_closed() {
    // v3: positions() says "Invalid token ID" and ownerOf() the same ERC721 reason as Aero CL.
    let v3 = nft_closure_verdict("uniswap-v3", V3_NPM, &outcome(REAL_V3_POSITIONS_BURNED), &outcome(REAL_CL_OWNER_OF_BURNED));
    assert!(matches!(v3, ClosureVerdict::Closed(ref e) if e[0].answer == "Error(\"Invalid token ID\")"), "{v3:?}");

    // v4: ownerOf() reverts NOT_MINTED and getPoolAndPositionInfo() returns six zero words.
    let v4 = v4_closure_verdict(&outcome(REAL_V4_OWNER_OF_BURNED), &returned(REAL_V4_INFO_BURNED));
    let ClosureVerdict::Closed(evidence) = v4 else { panic!("{v4:?}") };
    assert_eq!(evidence[0].answer, "Error(\"NOT_MINTED\")");
    assert_eq!(evidence[1].call, "getPoolAndPositionInfo(uint256)");
    assert_eq!(evidence[1].outcome, "returned");
    assert_eq!(evidence[1].answer, "192 zero bytes: no pool and no position info is stored for this id");
    assert_eq!(evidence[1].raw, REAL_V4_INFO_BURNED);
    assert_eq!(evidence[1].contract, UNISWAP_V4_POSM_BASE);

    // An NFT that exists with zero liquidity is NOT closed (the wallet's own #3163431):
    // its ownerOf answers, and its pool key and position info are stored.
    let owner = returned(&format!("{:0>64}", WALLET.trim_start_matches("0x").to_lowercase()));
    assert!(matches!(v4_closure_verdict(&owner, &returned(REAL_V4_INFO_EMPTY_BUT_EXISTING)), ClosureVerdict::NotProven(_)));
    // The two v4 reads must agree: NOT_MINTED with stored info is not a closed position,
    assert!(matches!(
        v4_closure_verdict(&outcome(REAL_V4_OWNER_OF_BURNED), &returned(REAL_V4_INFO_EMPTY_BUT_EXISTING)),
        ClosureVerdict::NotProven(ref why) if why.contains("do not agree")
    ));
    // nor is an owner with empty info, nor a zero answer of another length, nor a revert.
    assert!(matches!(v4_closure_verdict(&owner, &returned(REAL_V4_INFO_BURNED)), ClosureVerdict::NotProven(_)));
    assert!(matches!(
        v4_closure_verdict(&outcome(REAL_V4_OWNER_OF_BURNED), &returned(&"0".repeat(64))),
        ClosureVerdict::NotProven(_)
    ));
    assert!(matches!(
        v4_closure_verdict(&outcome(REAL_V4_OWNER_OF_BURNED), &outcome(REAL_V4_OWNER_OF_BURNED)),
        ClosureVerdict::NotProven(_)
    ));
    // Another contract's "no such token" reason is not the PositionManager's.
    assert!(matches!(
        v4_closure_verdict(&outcome(REAL_CL_OWNER_OF_BURNED), &returned(REAL_V4_INFO_BURNED)),
        ClosureVerdict::NotProven(_)
    ));
}

#[test]
fn test_anything_short_of_two_agreeing_answers_is_not_closed() {
    let not_proven = |positions: &CallOutcome, owner_of: &CallOutcome| -> String {
        match nft_closure_verdict("aerodrome-cl", CL_NPM, positions, owner_of) {
            ClosureVerdict::NotProven(why) => why,
            ClosureVerdict::Closed(_) => panic!("must not be closed"),
        }
    };
    let (id, gone) = (outcome(REAL_CL_POSITIONS_BURNED), outcome(REAL_CL_OWNER_OF_BURNED));
    // Only ONE read says nonexistent: ownerOf still names the gauge (the real answer of
    // one block before the burn).
    let why = not_proven(&id, &outcome(REAL_CL_OWNER_OF_BEFORE_BURN));
    assert!(why.contains("do not agree"), "{why}");
    // Only the other one does.
    let why = not_proven(&outcome(REAL_GAUGE_EARNED_NA), &gone);
    assert!(why.contains("Error(\"NA\")") && why.contains("not this position manager's answer"), "{why}");
    // A revert with an unrelated reason (the gauge's real "NA"), on either read.
    assert!(not_proven(&id, &outcome(REAL_GAUGE_EARNED_NA)).contains("do not agree"));
    // Another protocol's real answer is not this manager's: v3's "Invalid token ID" and
    // v4's "NOT_MINTED" on the Aero CL manager.
    not_proven(&outcome(REAL_V3_POSITIONS_BURNED), &gone);
    not_proven(&id, &outcome(REAL_V4_OWNER_OF_BURNED));
    // A revert with empty data, with no data, with a custom error.
    for data in [Some(String::new()), None, Some("1f2a2005".to_string())] {
        let r = CallOutcome::Reverted(data);
        not_proven(&r, &gone);
        not_proven(&id, &r);
    }
    // positions() must have reverted at all.
    not_proven(&returned(&"0".repeat(768)), &gone);

    // The allow-list is per protocol AND per position manager.
    assert!(nft_nonexistence_rule("aerodrome-cl", LEGACY_CL_NPM).is_none(), "the legacy manager's answers were not verified for a burned id");
    assert!(nft_nonexistence_rule("aerodrome-cl", V3_NPM).is_none());
    assert!(nft_nonexistence_rule("uniswap-v3", CL_NPM).is_none());
    assert!(nft_nonexistence_rule("uniswap-v4", UNISWAP_V4_POSM_BASE).is_none(), "v4 has its own proof");
    assert!(nft_nonexistence_rule("aerodrome", CL_NPM).is_none(), "an Aero basic pool has no NFT");
    assert!(nft_nonexistence_rule("mystery", CL_NPM).is_none());
    let why = match nft_closure_verdict("aerodrome-cl", LEGACY_CL_NPM, &id, &gone) {
        ClosureVerdict::NotProven(why) => why,
        ClosureVerdict::Closed(_) => panic!("an unverified manager is never closed"),
    };
    assert!(why.contains("is not one whose answers for a nonexistent token id have been verified"), "{why}");
    // The right answers under the wrong protocol's rule are not proof either.
    assert!(matches!(nft_closure_verdict("uniswap-v3", V3_NPM, &id, &gone), ClosureVerdict::NotProven(_)));
}

// ------------------------------------------------------------------
// The recorded cycle and the local node that replays it
// ------------------------------------------------------------------

const FIXTURE: &str = include_str!("../testdata/closed/cycle_fixture.json");

fn fixture() -> serde_json::Value {
    serde_json::from_str(FIXTURE).expect("testdata/closed/cycle_fixture.json parses")
}

/// How a request is found in the fixture: the method and what it reads, lowercase.
/// A simulated call (one with `from`) is its own key.
fn call_key(req: &serde_json::Value) -> String {
    let method = req["method"].as_str().unwrap_or_default();
    let lower = |v: &serde_json::Value| v.as_str().unwrap_or_default().to_lowercase();
    match method {
        "eth_call" => {
            let tx = &req["params"][0];
            let from = if tx.get("from").is_some() { format!("|from={}", lower(&tx["from"])) } else { String::new() };
            format!("eth_call|{}|{}{}", lower(&tx["to"]), lower(&tx["data"]), from)
        }
        "eth_getBalance" | "alchemy_getTokenBalances" => format!("{method}|{}", lower(&req["params"][0])),
        other => other.to_string(),
    }
}

fn key_call(to: &str, data: &str) -> String {
    format!("eth_call|{}|{}", to.to_lowercase(), data.to_lowercase())
}

fn key_positions(npm: &str, nft: u64) -> String {
    key_call(npm, &format!("{V3_POSITIONS}{nft:064x}"))
}

fn key_owner_of(npm: &str, nft: u64) -> String {
    key_call(npm, &format!("{ERC721_OWNER_OF}{nft:064x}"))
}

/// The block parameter a request was made with ("latest" or a block number), for an
/// `eth_call`; empty for a method without one.
fn call_block(req: &serde_json::Value) -> String {
    match req["method"].as_str().unwrap_or_default() {
        "eth_call" => req["params"][1].as_str().unwrap_or_default().to_string(),
        _ => String::new(),
    }
}

/// A swap key for one call at one block only: `key_at(&key, "latest")`.
fn key_at(key: &str, block: &str) -> String {
    format!("{key}@{block}")
}

const KEY_LATEST_BLOCK: &str = "eth_getBlockByNumber";

/// The fixture's block as the proof reads name it.
fn proof_block_tag() -> String {
    fixture()["block_hex"].as_str().unwrap().to_string()
}

/// The clock of a replayed cycle: the fixture block's own time plus 3 seconds, which is
/// what a worker reading that block as `latest` would see.
fn read_clock() -> DateTime<Utc> {
    Utc.timestamp_opt(fixture()["block_timestamp"].as_i64().unwrap() + 3, 0).unwrap()
}

/// What the local node answers for one key instead of the recorded answer.
#[derive(Clone)]
enum Swap {
    /// The `result`/`error` member of a JSON-RPC response (the id is the request's).
    Json(serde_json::Value),
    /// An HTTP status with an empty body.
    Http(u16),
    /// An HTTP status with this body, sent as it is (it need not be JSON).
    Body(u16, String),
    /// The connection is dropped without a response: a transport failure.
    Drop,
}

/// A real response constant as a swap (its `result` or `error` member).
fn swap_real(response: &str) -> Swap {
    let mut v: serde_json::Value = serde_json::from_str(response).unwrap();
    let obj = v.as_object_mut().unwrap();
    obj.remove("jsonrpc");
    obj.remove("id");
    Swap::Json(v)
}

struct Node {
    url: String,
    seen: Arc<std::sync::Mutex<Vec<String>>>,
    /// Every request again, with the block it was made at: `key@latest`, `key@0x…`.
    seen_at: Arc<std::sync::Mutex<Vec<String>>>,
}

impl Node {
    fn count(&self, key: &str) -> usize {
        self.seen.lock().unwrap().iter().filter(|k| k.as_str() == key).count()
    }

    /// The blocks one call was made at, in order.
    fn blocks_of(&self, key: &str) -> Vec<String> {
        let prefix = format!("{key}@");
        self.seen_at.lock().unwrap().iter().filter_map(|k| k.strip_prefix(&prefix).map(str::to_string)).collect()
    }
}

/// A local JSON-RPC server that replays the recorded cycle. A request the fixture does
/// not hold is a JSON-RPC error (which fails the cycle), never an invented answer.
/// A swap keyed `key_at(key, block)` replaces the answer at that block only.
async fn replay_node(swaps: Vec<(String, Swap)>) -> Node {
    let responses = fixture()["responses"].clone();
    let seen: Arc<std::sync::Mutex<Vec<String>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let seen_at: Arc<std::sync::Mutex<Vec<String>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a local port");
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handler = {
        let (seen, seen_at) = (seen.clone(), seen_at.clone());
        move |Json(req): Json<serde_json::Value>| {
            let (seen, seen_at, responses, swaps) = (seen.clone(), seen_at.clone(), responses.clone(), swaps.clone());
            async move {
                let key = call_key(&req);
                let at = key_at(&key, &call_block(&req));
                seen.lock().unwrap().push(key.clone());
                seen_at.lock().unwrap().push(at.clone());
                let answer = match swaps.iter().find(|(k, _)| k == &key || k == &at) {
                    Some((_, Swap::Http(status))) => {
                        return StatusCode::from_u16(*status).unwrap().into_response();
                    }
                    Some((_, Swap::Body(status, body))) => {
                        return (StatusCode::from_u16(*status).unwrap(), [("content-type", "application/json")], body.clone()).into_response();
                    }
                    // Unwinding without the panic hook: the connection closes with no
                    // response and nothing is printed.
                    Some((_, Swap::Drop)) => std::panic::resume_unwind(Box::new("dropped connection (test)")),
                    Some((_, Swap::Json(v))) => v.clone(),
                    None => match responses.get(&key) {
                        Some(recorded) => {
                            let mut v = recorded.clone();
                            v.as_object_mut().unwrap().remove("note");
                            v
                        }
                        None => json!({ "error": { "code": -32000, "message": format!("no recorded answer for {key}") } }),
                    },
                };
                let mut body = json!({ "jsonrpc": "2.0", "id": req["id"] });
                for (k, v) in answer.as_object().unwrap() {
                    body[k] = v.clone();
                }
                Json(body).into_response()
            }
        }
    };
    let app = axum::Router::new().route("/", axum::routing::post(handler));
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    Node { url, seen, seen_at }
}

/// A URL nothing listens on: every request is a transport failure.
async fn dead_url() -> String {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a local port");
    let url = format!("http://{}", listener.local_addr().unwrap());
    drop(listener);
    url
}

// ---- configs: the tracked wallet's book on 2026-11-17 (public chain data) ----

const ENTRY_V4: &str = r#"
  - id: "uniswap-v4-nock-usdc-wide"
    protocol: "uniswap-v4"
    nft_id: 3163450
    pool_id: "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13"
    currency0: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
    currency1: "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722"
    tick_lower: 262200
    tick_upper: 282300
    is_price_source: true"#;
const ENTRY_V3: &str = r#"
  - id: "uniswap-v3-weth-nock"
    protocol: "uniswap-v3"
    nft_id: 6336662
    npm_address: "0x03a520b32C04BF3bEEf7BEb72E919cf822Ed34f1"
    pool_address: "0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a""#;
const ENTRY_BASIC: &str = r#"
  - id: "aero-basic-nock-usdc"
    protocol: "aerodrome"
    pool_address: "0x770fE8Fc9248c6382950a86e39a9391b41d59e0a"
    gauge_address: "0xc82782ee66d82c4d19db0429652091a085083526""#;
/// The entry that stalled ingestion on 2026-11-17: its NFT was burned in block 58637717.
const ENTRY_CL_WITHDRAWN: &str = r#"
  - id: "aero-cl200-weth-nock"
    protocol: "aerodrome-cl"
    nft_id: 2914490
    npm_address: "0x4857db576ee0e45f10525ec59527bfd10beec60b"
    pool_address: "0x957256c98d61A714081DE2EE70ae3A96c9b40874""#;
/// The wallet's own v4 NFT #3163431: it exists and holds no liquidity. Its pool id is
/// keccak256 of the pool key `getPoolAndPositionInfo` returns; the ticks are in its info.
const ENTRY_V4_EMPTY: &str = r#"
  - id: "uniswap-v4-nock-usdc-retired"
    protocol: "uniswap-v4"
    nft_id: 3163431
    pool_id: "0x91a9ef0a8dd231b99fa53a2a145a9e8bca7897e0d85852107abb997916f9f99c"
    currency0: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913"
    currency1: "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722"
    tick_lower: 267825
    tick_upper: 271050"#;

fn config(entries: &[&str]) -> AppConfig {
    let yaml = format!("alchemy_api_keys: [\"not-a-real-key\"]\ntracked_wallets: [\"{WALLET}\"]\npositions:{}\n", entries.concat());
    serde_yaml::from_str(&yaml).expect("the test config parses")
}

/// The live book with the withdrawn Aero CL entry still configured: what the worker saw
/// from 12:20 UTC on 2026-11-17.
fn book_with_withdrawn_cl() -> AppConfig {
    config(&[ENTRY_V4, ENTRY_V3, ENTRY_BASIC, ENTRY_CL_WITHDRAWN])
}

/// The same book without that entry: what the operator's hand edit left.
fn book_without_cl() -> AppConfig {
    config(&[ENTRY_V4, ENTRY_V3, ENTRY_BASIC])
}

/// The v4 entry pointed at a token id that does not exist (burned in another pool; the
/// reads that prove closure do not depend on the pool).
fn entry_v4_nonexistent(price_source: bool) -> String {
    ENTRY_V4
        .replace("uniswap-v4-nock-usdc-wide", "uniswap-v4-nock-usdc-gone")
        .replace("nft_id: 3163450", &format!("nft_id: {V4_BURNED_NFT}"))
        .replace("is_price_source: true", &format!("is_price_source: {price_source}"))
}

/// The v4 entry with `nft_id` left out: it reads as token id 0, which was never minted.
fn entry_v4_without_nft_id() -> String {
    let entry = ENTRY_V4.replace("uniswap-v4-nock-usdc-wide", "uniswap-v4-nock-usdc-no-id").replace("    nft_id: 3163450\n", "");
    assert!(!entry.contains("nft_id"));
    entry
}

fn entry_v3_nonexistent() -> String {
    ENTRY_V3.replace("uniswap-v3-weth-nock", "uniswap-v3-weth-nock-gone").replace("nft_id: 6336662", &format!("nft_id: {V3_BURNED_NFT}"))
}

/// A test profile (synthetic numbers, in the shape of config.yaml.example): a floor,
/// and an LP target the book is under, so rebuild suggestions are sized.
fn test_profile() -> OperatorProfile {
    OperatorProfile {
        treasury_floor_nock: 100_000.0,
        lp_floor_tolerance_bps: 0,
        tax_reserve_ratio: 0.40,
        target_weights_bps: TargetWeightsBps { treasury: 6500, uniswap_lp: 3000, cash: 500 },
        cash_reserve_usdc: None,
        ladder_bottom_usd: None,
        ladder_rungs: None,
    }
}

const NOW: &str = "2026-11-17T13:00:00+00:00";

/// One cycle's reads against the local node.
async fn read_cycle(node: &Node, cfg: &AppConfig) -> Result<RawOnChainData> {
    read_cycle_at(node, cfg, read_clock()).await
}

/// The same reads on another clock: `now` is the worker's time when it reads the
/// fixture block as the node's latest.
async fn read_cycle_at(node: &Node, cfg: &AppConfig, now: DateTime<Utc>) -> Result<RawOnChainData> {
    fetch_raw_data_from(&Client::new(), &node.url, cfg, now).await
}

/// The pure rest of the cycle, as run_ingestion_cycle applies it: the metrics, then
/// cycle_snapshot_data. Returns the snapshot and the closed-position sentences.
fn finish_cycle(raw: RawOnChainData, prev: Option<&SnapshotData>) -> (SnapshotData, Vec<String>) {
    let plan = PlanContext {
        cfg: None,
        fills: PlanFills::default(),
        ledger_unavailable: false,
        now: Utc.with_ymd_and_hms(2026, 11, 17, 13, 0, 0).unwrap(),
    };
    let derived = compute_derived_metrics_with_plan(
        &raw,
        0.0,
        &test_profile(),
        StructuralChannels { channels: vec![] },
        &[],
        &LedgerAggregate::default(),
        &MarketRegime::default(),
        &plan,
    );
    cycle_snapshot_data(raw, derived, prev, NOW)
}

/// The snapshot as a consumer reads it, without wall-clock fields.
fn stable_json(data: &SnapshotData) -> serde_json::Value {
    canonical_state_json(data)
}

/// No error text may carry the RPC URL or the key.
fn assert_no_url(text: &str, node_url: &str) {
    assert!(!text.contains(node_url) && !text.contains("127.0.0.1") && !text.contains("not-a-real-key") && !text.contains("http"), "{text}");
}

// ------------------------------------------------------------------
// Recording (ignored: needs the network)
// ------------------------------------------------------------------
//   NOCKNOCK_LIVE_RPC_URL=https://base-rpc.publicnode.com \
//     cargo test test_record_closed_cycle_fixture -- --ignored --nocapture
// Optional NOCKNOCK_RECORD_BLOCK=<decimal block> pins another block (the node must
// still hold its state). The recorder runs the worker's own reads through a proxy that
// pins every `latest` to that block, so the fixture's requests are the worker's own.
// Recording again at the block the file already holds keeps its `eth_gasPrice` answer
// (the price at first recording): that method has no block, and a new price would move
// every gas figure of the golden file for no reason.

#[tokio::test]
#[ignore = "needs NOCKNOCK_LIVE_RPC_URL (a Base mainnet JSON-RPC endpoint); rewrites testdata/closed/cycle_fixture.json"]
async fn test_record_closed_cycle_fixture() {
    let upstream = std::env::var("NOCKNOCK_LIVE_RPC_URL").expect("NOCKNOCK_LIVE_RPC_URL is set");
    let http = Client::new();
    let ask = |method: &'static str, params: serde_json::Value| {
        let (http, upstream) = (http.clone(), upstream.clone());
        async move {
            let body = json!({ "jsonrpc": "2.0", "id": 1, "method": method, "params": params });
            http.post(&upstream).json(&body).send().await.expect("upstream reachable").json::<serde_json::Value>().await.expect("upstream answers JSON")
        }
    };
    let block: u64 = match std::env::var("NOCKNOCK_RECORD_BLOCK").ok().and_then(|b| b.parse().ok()) {
        Some(b) => b,
        None => {
            let head = ask("eth_blockNumber", json!([])).await;
            u64::from_str_radix(head["result"].as_str().unwrap().trim_start_matches("0x"), 16).unwrap() - 20
        }
    };
    let block_hex = format!("0x{block:x}");
    let header = ask("eth_getBlockByNumber", json!([block_hex, false])).await;
    let timestamp = u64::from_str_radix(header["result"]["timestamp"].as_str().unwrap().trim_start_matches("0x"), 16).unwrap();
    let block_hash = header["result"]["hash"].as_str().unwrap().to_string();

    let recorded: Arc<std::sync::Mutex<BTreeMap<String, serde_json::Value>>> = Arc::new(std::sync::Mutex::new(BTreeMap::new()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let proxy_url = format!("http://{}", listener.local_addr().unwrap());
    // The gas price of the first recording at this block, if the file holds one.
    let kept_gas_price: Option<serde_json::Value> = Some(fixture())
        .filter(|old| old["block"].as_u64() == Some(block))
        .map(|old| old["responses"]["eth_gasPrice"].clone())
        .filter(|v| v.is_object());
    let handler = {
        let (recorded, http, upstream, block_hex) = (recorded.clone(), http.clone(), upstream.clone(), block_hex.clone());
        move |Json(req): Json<serde_json::Value>| {
            let (recorded, http, upstream, block_hex) = (recorded.clone(), http.clone(), upstream.clone(), block_hex.clone());
            let kept_gas_price = kept_gas_price.clone();
            async move {
                let key = call_key(&req);
                let forward = |body: serde_json::Value| {
                    let (http, upstream) = (http.clone(), upstream.clone());
                    async move {
                        let v = http.post(&upstream).json(&body).send().await.expect("upstream reachable").json::<serde_json::Value>().await.expect("upstream JSON");
                        let mut out = serde_json::Map::new();
                        for k in ["result", "error"] {
                            if let Some(x) = v.get(k) {
                                out.insert(k.to_string(), x.clone());
                            }
                        }
                        serde_json::Value::Object(out)
                    }
                };
                let method = req["method"].as_str().unwrap_or_default().to_string();
                let answer = match method.as_str() {
                    "eth_call" | "eth_getBalance" => {
                        let mut pinned = req.clone();
                        // A proving read names the block the worker was just given as
                        // the latest, which is this one; every other read says latest.
                        assert!(
                            pinned["params"][1] == "latest" || (method == "eth_call" && pinned["params"][1] == json!(block_hex)),
                            "the cycle reads at latest, or at the proof block: {}",
                            pinned["params"][1]
                        );
                        pinned["params"][1] = json!(block_hex);
                        forward(pinned).await
                    }
                    "eth_getBlockByNumber" => {
                        // The node's latest block, for a closure proof: the pinned block's
                        // own header, as the node sends it.
                        assert_eq!(req["params"], json!(["latest", false]), "the cycle asks for the latest header only");
                        let mut pinned = req.clone();
                        pinned["params"][0] = json!(block_hex);
                        forward(pinned).await
                    }
                    "eth_gasPrice" => match kept_gas_price {
                        Some(kept) => kept,
                        None => {
                            let mut v = forward(req.clone()).await;
                            v["note"] = json!("the gas price when the fixture was recorded; eth_gasPrice cannot be pinned to a block");
                            v
                        }
                    },
                    "alchemy_getTokenBalances" => {
                        // Not served by a public node. The response is assembled, in
                        // Alchemy's shape, from the two real balanceOf reads at the block.
                        let wallet = req["params"][0].as_str().unwrap().to_string();
                        let mut balances = Vec::new();
                        for token in req["params"][1].as_array().unwrap() {
                            let token = token.as_str().unwrap();
                            let call = json!({ "jsonrpc": "2.0", "id": 1, "method": "eth_call", "params": [
                                { "to": token, "data": erc20_balance_of_calldata(&wallet) }, block_hex
                            ]});
                            let v = forward(call).await;
                            balances.push(json!({ "contractAddress": token.to_lowercase(), "tokenBalance": v["result"] }));
                        }
                        json!({
                            "result": { "address": wallet.to_lowercase(), "tokenBalances": balances },
                            "note": "assembled in Alchemy's response shape from two real balanceOf eth_calls at the pinned block; the public node does not serve this method"
                        })
                    }
                    other => panic!("the cycle made an unexpected call: {other}"),
                };
                recorded.lock().unwrap().insert(key, answer.clone());
                let mut body = json!({ "jsonrpc": "2.0", "id": req["id"] });
                for (k, v) in answer.as_object().unwrap() {
                    if k != "note" {
                        body[k] = v.clone();
                    }
                }
                Json(body)
            }
        }
    };
    tokio::spawn(async move {
        let _ = axum::serve(listener, axum::Router::new().route("/", axum::routing::post(handler))).await;
    });

    // Every config a replay test uses is read once through the proxy.
    let v4_gone_source = entry_v4_nonexistent(true);
    let v4_gone = entry_v4_nonexistent(false);
    let v3_gone = entry_v3_nonexistent();
    let cl_legacy = ENTRY_CL_WITHDRAWN.replace(CL_NPM, LEGACY_CL_NPM).replace("2914490", "99999999999");
    let v4_no_id = entry_v4_without_nft_id();
    let runs: Vec<(&str, AppConfig)> = vec![
        ("the book with the withdrawn Aero CL entry", book_with_withdrawn_cl()),
        ("the book without it", book_without_cl()),
        ("a nonexistent v4 NFT as the price source", config(&[&v4_gone_source, ENTRY_V3, ENTRY_BASIC])),
        ("a nonexistent v4 and v3 NFT beside the live book", config(&[ENTRY_V4, ENTRY_V3, ENTRY_BASIC, &v4_gone, &v3_gone])),
        ("the wallet's empty v4 NFT", config(&[ENTRY_V4, ENTRY_V3, ENTRY_BASIC, ENTRY_V4_EMPTY])),
        ("an NFT on a position manager that is not on the allow-list", config(&[ENTRY_V4, &cl_legacy])),
        ("a v4 entry with no nft_id as the only entry", config(&[&v4_no_id])),
    ];
    // The worker's clock while it reads the pinned block as the latest one.
    let clock = Utc.timestamp_opt(timestamp as i64 + 3, 0).unwrap();
    for (what, cfg) in &runs {
        match fetch_raw_data_from(&http, &proxy_url, cfg, clock).await {
            Ok(raw) => println!("{what}: {} live, {} closed, spot {}", raw.positions.len(), raw.closed_positions.len(), raw.spot_price_nock_usd),
            Err(e) => println!("{what}: cycle error: {e}"),
        }
    }

    let responses = recorded.lock().unwrap().clone();
    let doc = json!({
        "about": "Every JSON-RPC answer one NockNock ingestion cycle reads for the tracked wallet, recorded by test_record_closed_cycle_fixture (tests_closed.rs) for issue #144. Keys are the method and what it reads. Entries with a `note` are not a node's own bytes; the note says what they are.",
        "recorded_via": upstream.split('?').next().unwrap_or_default(),
        "block": block,
        "block_hex": block_hex,
        "block_hash": block_hash,
        "block_timestamp": timestamp,
        "responses": responses,
    });
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/closed/cycle_fixture.json");
    std::fs::create_dir_all(concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/closed")).unwrap();
    std::fs::write(path, format!("{}\n", serde_json::to_string_pretty(&doc).unwrap())).unwrap();
    println!("wrote {} answers at block {block} to {path}", responses.len());
}

// ------------------------------------------------------------------
// (a) The withdrawn position is closed and the cycle goes on
// ------------------------------------------------------------------

#[tokio::test]
async fn test_the_withdrawn_aero_cl_position_is_closed_and_the_cycle_succeeds() {
    let node = replay_node(vec![]).await;
    // The fixture holds the two real answers that stalled ingestion on 2026-11-17.
    let fx = fixture();
    let sent = |response: &str| serde_json::from_str::<serde_json::Value>(response).unwrap()["error"].clone();
    assert_eq!(fx["responses"][key_positions(CL_NPM, CL_NFT)]["error"], sent(REAL_CL_POSITIONS_BURNED));
    assert_eq!(fx["responses"][key_owner_of(CL_NPM, CL_NFT)]["error"], sent(REAL_CL_OWNER_OF_BURNED));

    let raw = read_cycle(&node, &book_with_withdrawn_cl()).await.expect("a closed position does not fail the cycle");
    // positions() reverted at latest; then the proof: the node's latest block was read
    // once, and positions() and ownerOf() were each read once AT THAT BLOCK. Nothing
    // else was read from the position: not its pool, not its gauge.
    let block = proof_block_tag();
    assert_eq!(block, "0x37edf0d");
    assert_eq!(node.blocks_of(&key_positions(CL_NPM, CL_NFT)), vec!["latest".to_string(), block.clone()]);
    assert_eq!(node.blocks_of(&key_owner_of(CL_NPM, CL_NFT)), vec![block.clone()]);
    assert_eq!(node.count(KEY_LATEST_BLOCK), 1);
    let seen = node.seen.lock().unwrap().clone();
    assert!(!seen.iter().any(|k| k.contains(&CL_POOL.to_lowercase()) || k.contains(CL_GAUGE)), "{seen:?}");

    assert_eq!(raw.positions.iter().map(|p| p.position_id.as_str()).collect::<Vec<_>>(), vec![
        "uniswap-v4:base:3163450",
        "uniswap-v3:base:6336662",
        "aerodrome:base:0x770fe8fc9248c6382950a86e39a9391b41d59e0a",
    ]);
    assert_eq!(raw.closed_positions.len(), 1);
    assert!(!raw.rebuild_rules.contains_key("aerodrome-cl:base:2914490"));
    assert!(!raw.position_facts.contains_key("aerodrome-cl:base:2914490"));
    assert!(!raw.pool_liquidity.contains_key("aerodrome-cl:base:2914490"));
    assert!(raw.aero.unclaimed.as_ref().unwrap().iter().all(|u| u.position_id != "aerodrome-cl:base:2914490"));

    let (snap, _) = finish_cycle(raw, None);
    // Absent from positions[], present in closed_positions[].
    assert_eq!(snap.positions.len(), 3);
    assert!(snap.positions.iter().all(|p| p.nft_id != CL_NFT && p.protocol != AERO_CL_PROTOCOL));
    assert_eq!(snap.closed_positions.len(), 1);
    let c = &snap.closed_positions[0];
    assert_eq!(c.config_id, "aero-cl200-weth-nock");
    assert_eq!(c.position_id, "aerodrome-cl:base:2914490");
    assert_eq!(c.protocol, "aerodrome-cl");
    assert_eq!(c.nft_id, CL_NFT);
    assert_eq!(c.pool_id.as_deref(), Some("0x957256c98d61a714081de2ee70ae3a96c9b40874"));
    assert_eq!(c.position_manager, CL_NPM);
    assert_eq!(c.label, "Aero CL #2914490 (aero-cl200-weth-nock)");
    assert_eq!(c.status, "closed");
    assert_eq!(
        c.reason,
        "NFT #2914490 does not exist on position manager 0x4857db576ee0e45f10525ec59527bfd10beec60b: withdrawn and burned, or a mistyped nft_id"
    );
    assert_eq!(c.evidence.len(), 2);
    assert_eq!((c.evidence[0].call.as_str(), c.evidence[0].answer.as_str()), ("positions(uint256)", "Error(\"ID\")"));
    assert_eq!(c.evidence[0].raw, sent(REAL_CL_POSITIONS_BURNED)["data"].as_str().unwrap());
    assert_eq!(c.evidence[1].raw, sent(REAL_CL_OWNER_OF_BURNED)["data"].as_str().unwrap());
    assert_eq!(c.first_seen_closed_at.as_deref(), Some(NOW), "no previous snapshot listed it: this cycle is the first to record it");
    assert!(!c.supplies_spot_price);

    // The entry carries no amount, value or fee figure: nothing was read from it.
    let entry = serde_json::to_value(c).unwrap();
    let keys: Vec<&str> = entry.as_object().unwrap().keys().map(String::as_str).collect();
    for k in &keys {
        assert!(!k.contains("amount") && !k.contains("usd") && !k.contains("fee") && !k.contains("liquidity") && !k.contains("owed"), "{k}");
    }
    assert!(!keys.contains(&"supplies_spot_price"), "absent when false");

    // global_status says it, in one count and one sentence.
    let gs = &snap.derived_metrics.global_status;
    assert_eq!(gs.closed_positions_count, Some(1));
    assert_eq!(
        gs.closed_positions_note.as_deref(),
        Some("1 configured position is closed and is not counted in any total: Aero CL #2914490 (aero-cl200-weth-nock) (NFT #2914490 no longer exists: withdrawn and burned, or a mistyped nft_id). Its entry can be removed from config.yaml.")
    );
    // A cycle with a closed position is a successful cycle: /health reads ok.
    assert_eq!(health_status(true, 0, 300, false), "ok");
    // The stored payload round-trips, and an older payload without the field parses.
    let stored = serde_json::to_value(&snap).unwrap();
    let back: SnapshotData = serde_json::from_value(stored.clone()).unwrap();
    assert_eq!(back.closed_positions, snap.closed_positions);
    let mut old = stored;
    old.as_object_mut().unwrap().remove("closed_positions");
    assert!(serde_json::from_value::<SnapshotData>(old).unwrap().closed_positions.is_empty());
}

// ------------------------------------------------------------------
// (b) Anything that is not proof fails the cycle, exactly as before
// ------------------------------------------------------------------

#[tokio::test]
async fn test_no_answer_or_another_answer_fails_the_cycle_and_is_never_closed() {
    let positions_key = key_positions(CL_NPM, CL_NFT);
    let owner_key = key_owner_of(CL_NPM, CL_NFT);
    let positions_at_proof = key_at(&positions_key, &proof_block_tag());
    // The first 200 characters of a real response: the revert selector is in them and
    // the JSON is not closed.
    let cut_short = |response: &str| {
        let cut = response[..200].to_string();
        assert!(cut.contains("0x08c379a0") && serde_json::from_str::<serde_json::Value>(&cut).is_err());
        cut
    };
    let rpc_error = |code: i64, message: &str| Swap::Json(json!({ "error": { "code": code, "message": message } }));
    let revert = |data: Option<&str>| {
        let mut err = json!({ "code": 3, "message": "execution reverted" });
        if let Some(d) = data {
            err["data"] = json!(d);
        }
        Swap::Json(json!({ "error": err }))
    };
    // (the swapped key, its answer, what the cycle error must say)
    let cases: Vec<(&str, &String, Swap, &str)> = vec![
        ("transport failure on positions()", &positions_key, Swap::Drop, "positions() read failed: network error"),
        ("HTTP 429 on positions()", &positions_key, Swap::Http(429), "positions() read failed: HTTP 429"),
        ("HTTP 500 on positions()", &positions_key, Swap::Http(500), "positions() read failed: HTTP 500"),
        ("a JSON-RPC rate-limit error", &positions_key, rpc_error(-32005, "rate limit exceeded"), "positions() read failed: RPC error -32005"),
        ("a JSON-RPC node error", &positions_key, rpc_error(-32000, "header not found"), "positions() read failed: RPC error -32000"),
        ("a revert with an unrelated reason", &positions_key, swap_real(REAL_GAUGE_EARNED_NA), "positions() reverted with Error(\"NA\")"),
        ("another protocol's reason", &positions_key, swap_real(REAL_V3_POSITIONS_BURNED), "positions() reverted with Error(\"Invalid token ID\")"),
        ("a revert with empty data", &positions_key, revert(Some("0x")), "positions() reverted with empty revert data"),
        ("a revert with no data", &positions_key, revert(None), "positions() reverted with no revert data from the node"),
        ("a revert with a custom error", &positions_key, revert(Some("0x1f2a2005")), "positions() reverted with custom error 0x1f2a2005"),
        ("only positions() says nonexistent", &owner_key, swap_real(REAL_CL_OWNER_OF_BEFORE_BURN), "the two reads do not agree"),
        ("ownerOf() reverts with another reason", &owner_key, swap_real(REAL_GAUGE_EARNED_NA), "the two reads do not agree"),
        ("ownerOf() reverts with empty data", &owner_key, revert(Some("0x")), "the two reads do not agree"),
        ("transport failure on ownerOf()", &owner_key, Swap::Drop, "ownerOf() read failed: network error"),
        ("HTTP 500 on ownerOf()", &owner_key, Swap::Http(500), "ownerOf() read failed: HTTP 500"),
        ("a JSON-RPC error on ownerOf()", &owner_key, rpc_error(-32005, "rate limit exceeded"), "ownerOf() read failed: RPC error -32005"),
        // An error status is no answer, whatever its body says: here the body is the
        // real "nonexistent" revert, as a cache or a relay in front of the node could
        // send it. A 200 whose body is that revert cut short is no answer either.
        ("HTTP 429 carrying the real revert", &positions_key, Swap::Body(429, REAL_CL_POSITIONS_BURNED.to_string()), "positions() read failed: HTTP 429"),
        ("HTTP 500 carrying the real revert", &positions_key, Swap::Body(500, REAL_CL_POSITIONS_BURNED.to_string()), "positions() read failed: HTTP 500"),
        ("HTTP 200 with the revert cut short", &positions_key, Swap::Body(200, cut_short(REAL_CL_POSITIONS_BURNED)), "positions() read failed: unparseable response"),
        ("HTTP 429 carrying the real revert, at the proof block", &positions_at_proof, Swap::Body(429, REAL_CL_POSITIONS_BURNED.to_string()), "positions() read failed: HTTP 429"),
        ("HTTP 429 carrying the real revert on ownerOf()", &owner_key, Swap::Body(429, REAL_CL_OWNER_OF_BURNED.to_string()), "ownerOf() read failed: HTTP 429"),
        ("HTTP 503 carrying the real revert on ownerOf()", &owner_key, Swap::Body(503, REAL_CL_OWNER_OF_BURNED.to_string()), "ownerOf() read failed: HTTP 503"),
        ("HTTP 200 with the ownerOf() revert cut short", &owner_key, Swap::Body(200, cut_short(REAL_CL_OWNER_OF_BURNED)), "ownerOf() read failed: unparseable response"),
    ];
    for (what, key, swap, expect) in cases {
        let node = replay_node(vec![(key.clone(), swap)]).await;
        let err = match read_cycle(&node, &book_with_withdrawn_cl()).await {
            Err(e) => e.to_string(),
            Ok(raw) => panic!("{what}: the cycle must fail, but it listed {} closed", raw.closed_positions.len()),
        };
        assert!(err.starts_with("position aero-cl200-weth-nock: Aero CL "), "{what}: {err}");
        assert!(err.contains(expect), "{what}: {err}");
        assert_no_url(&err, &node.url);
    }

    // The position manager is not on the allow-list (the legacy Slipstream manager and a
    // never-minted id: both of its real answers are the "nonexistent" ones, and still
    // nothing is read as closed).
    let node_ok = replay_node(vec![]).await;
    let legacy = ENTRY_CL_WITHDRAWN.replace(CL_NPM, LEGACY_CL_NPM).replace("2914490", "99999999999");
    let err = read_cycle(&node_ok, &config(&[ENTRY_V4, &legacy])).await.unwrap_err().to_string();
    assert!(err.contains("is not one whose answers for a nonexistent token id have been verified"), "{err}");

    // No node at all.
    let dead = dead_url().await;
    let err = fetch_raw_data_from(&Client::new(), &dead, &book_with_withdrawn_cl(), read_clock()).await.unwrap_err().to_string();
    assert!(!err.contains(&dead) && !err.contains("127.0.0.1"), "{err}");
    let cfg = book_with_withdrawn_cl();
    let clock = ProofClock::for_cycle(read_clock(), cfg.sync_window_seconds);
    let err = fetch_aero_cl_position(&Client::new(), &dead, &cfg.positions[3], WALLET, clock).await.unwrap_err().to_string();
    assert!(err.starts_with("position aero-cl200-weth-nock: Aero CL positions() read failed: network error"), "{err}");
    assert!(!err.contains(&dead) && !err.contains("127.0.0.1"), "{err}");
    let err = eth_call_outcome(&Client::new(), &dead, &json!({"jsonrpc": "2.0", "id": 1, "method": "eth_call", "params": []})).await.unwrap_err().to_string();
    assert!(err.starts_with("network error") && !err.contains(&dead), "{err}");
}

#[tokio::test]
async fn test_uniswap_v3_and_v4_fail_the_cycle_unless_both_reads_prove_it() {
    let (v4_gone, v3_gone) = (entry_v4_nonexistent(false), entry_v3_nonexistent());
    let cfg = config(&[ENTRY_V4, ENTRY_V3, ENTRY_BASIC, &v4_gone, &v3_gone]);
    let v4_owner = key_owner_of(UNISWAP_V4_POSM_BASE, V4_BURNED_NFT);
    let v4_info = key_call(UNISWAP_V4_POSM_BASE, &format!("{V4_GET_POOL_AND_POSITION_INFO}{V4_BURNED_NFT:064x}"));
    let v3_positions = key_positions(V3_NPM, V3_BURNED_NFT);
    let v3_owner = key_owner_of(V3_NPM, V3_BURNED_NFT);

    // The real answers: both are closed, and the three live positions are untouched.
    let node_ok = replay_node(vec![]).await;
    let raw = read_cycle(&node_ok, &cfg).await.expect("two closed positions do not fail the cycle");
    assert_eq!(raw.positions.len(), 3);
    let closed: Vec<(&str, &str)> = raw.closed_positions.iter().map(|c| (c.config_id.as_str(), c.label.as_str())).collect();
    assert_eq!(closed, vec![
        ("uniswap-v4-nock-usdc-gone", "v4 USDC/NOCK #3229273 (uniswap-v4-nock-usdc-gone)"),
        ("uniswap-v3-weth-nock-gone", "v3 #6604113 (uniswap-v3-weth-nock-gone)"),
    ]);
    let v4 = &raw.closed_positions[0];
    assert_eq!(v4.position_manager, UNISWAP_V4_POSM_BASE);
    assert_eq!(v4.pool_id.as_deref(), Some("0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13"));
    assert_eq!(v4.evidence[0].raw, serde_json::from_str::<serde_json::Value>(REAL_V4_OWNER_OF_BURNED).unwrap()["error"]["data"].as_str().unwrap());
    assert_eq!(v4.evidence[1].raw, REAL_V4_INFO_BURNED);
    let v3 = &raw.closed_positions[1];
    assert_eq!(v3.position_manager, V3_NPM.to_lowercase());
    assert_eq!(v3.evidence[0].raw, serde_json::from_str::<serde_json::Value>(REAL_V3_POSITIONS_BURNED).unwrap()["error"]["data"].as_str().unwrap());
    assert_eq!(v3.evidence[1].answer, "Error(\"ERC721: owner query for nonexistent token\")");
    let (snap, _) = finish_cycle(raw, None);
    assert_eq!(snap.derived_metrics.global_status.closed_positions_count, Some(2));
    assert!(snap.derived_metrics.global_status.closed_positions_note.as_deref().unwrap().starts_with(
        "2 configured positions are closed and are not counted in any total: v4 USDC/NOCK #3229273 (uniswap-v4-nock-usdc-gone) (NFT #3229273 no longer exists: withdrawn and burned, or a mistyped nft_id); v3 #6604113"
    ));
    assert!(snap.derived_metrics.global_status.closed_positions_note.as_deref().unwrap().ends_with("Their entries can be removed from config.yaml."));
    // A live v4 position with liquidity makes no existence read at all.
    assert_eq!(node_ok.count(&key_owner_of(UNISWAP_V4_POSM_BASE, 3163450)), 0);

    let rpc_error = Swap::Json(json!({ "error": { "code": -32005, "message": "rate limit exceeded" } }));
    let cases: Vec<(&str, &String, Swap, &str)> = vec![
        ("v4: ownerOf() returns an owner while the info is empty", &v4_owner, swap_real(REAL_CL_OWNER_OF_BEFORE_BURN), ""),
        ("v4: ownerOf() reverts with another reason", &v4_owner, swap_real(REAL_GAUGE_EARNED_NA), "ownerOf() reverted with Error(\"NA\")"),
        ("v4: ownerOf() transport failure", &v4_owner, Swap::Drop, "POSM ownerOf() query failed: network error"),
        ("v4: ownerOf() HTTP 429", &v4_owner, Swap::Http(429), "POSM ownerOf() query failed: HTTP 429"),
        ("v4: ownerOf() JSON-RPC error", &v4_owner, rpc_error.clone(), "POSM ownerOf() query failed: RPC error -32005"),
        ("v4: ownerOf() HTTP 429 carrying the real revert", &v4_owner, Swap::Body(429, REAL_V4_OWNER_OF_BURNED.to_string()), "POSM ownerOf() query failed: HTTP 429"),
        ("v3: positions() HTTP 500 carrying the real revert", &v3_positions, Swap::Body(500, REAL_V3_POSITIONS_BURNED.to_string()), "NPM positions() query failed: HTTP 500"),
        ("v4: only ownerOf() says nonexistent", &v4_info, Swap::Json(json!({ "result": REAL_V4_INFO_EMPTY_BUT_EXISTING })), "the two reads do not agree"),
        ("v4: the info read reverts", &v4_info, swap_real(REAL_GAUGE_EARNED_NA), "the two reads do not agree"),
        ("v4: the info read fails", &v4_info, Swap::Http(500), "POSM getPoolAndPositionInfo() query failed: HTTP 500"),
        ("v3: positions() transport failure", &v3_positions, Swap::Drop, "NPM positions() query failed: network error"),
        ("v3: positions() HTTP 500", &v3_positions, Swap::Http(500), "NPM positions() query failed: HTTP 500"),
        ("v3: positions() JSON-RPC error", &v3_positions, rpc_error.clone(), "NPM positions() query failed: RPC error -32005"),
        ("v3: positions() reverts with another reason", &v3_positions, swap_real(REAL_CL_POSITIONS_BURNED), "positions() reverted with Error(\"ID\")"),
        ("v3: positions() reverts with empty data", &v3_positions, Swap::Json(json!({ "error": { "code": 3, "message": "execution reverted", "data": "0x" } })), "empty revert data"),
        ("v3: only positions() says nonexistent", &v3_owner, swap_real(REAL_CL_OWNER_OF_BEFORE_BURN), "the two reads do not agree"),
        ("v3: ownerOf() fails", &v3_owner, Swap::Http(429), "NPM ownerOf() query failed: HTTP 429"),
    ];
    for (what, key, swap, expect) in cases {
        let node = replay_node(vec![(key.clone(), swap)]).await;
        match read_cycle(&node, &cfg).await {
            Err(e) => {
                let err = e.to_string();
                assert!(err.contains(expect), "{what}: {err}");
                assert_no_url(&err, &node.url);
            }
            // An owner for the id means the NFT exists: the entry is then a LIVE, empty
            // position (zero liquidity), which is what the first case swaps in.
            Ok(raw) => {
                assert!(what.starts_with("v4: ownerOf() returns an owner"), "{what}: the cycle must fail");
                assert!(raw.closed_positions.iter().all(|c| c.config_id != "uniswap-v4-nock-usdc-gone"));
                assert!(raw.positions.iter().any(|p| p.nft_id == V4_BURNED_NFT && p.liquidity == 0));
            }
        }
    }
}

#[tokio::test]
async fn test_an_nft_that_exists_with_zero_liquidity_is_a_live_position_not_a_closed_one() {
    // The wallet's own v4 NFT #3163431: withdrawn to zero, never burned.
    let node = replay_node(vec![]).await;
    let raw = read_cycle(&node, &config(&[ENTRY_V4, ENTRY_V3, ENTRY_BASIC, ENTRY_V4_EMPTY])).await.unwrap();
    assert!(raw.closed_positions.is_empty());
    let empty = raw.positions.iter().find(|p| p.nft_id == V4_EMPTY_NFT).expect("it stays a live position");
    assert_eq!((empty.liquidity, empty.amount0, empty.amount1), (0, 0.0, 0.0));
    // Zero liquidity is followed by exactly one existence read, which names the wallet.
    assert_eq!(node.count(&key_owner_of(UNISWAP_V4_POSM_BASE, V4_EMPTY_NFT)), 1);
    let owner = fixture()["responses"][key_owner_of(UNISWAP_V4_POSM_BASE, V4_EMPTY_NFT)]["result"].as_str().unwrap().to_string();
    assert!(owner.ends_with(&WALLET.trim_start_matches("0x").to_lowercase()));
    let (snap, _) = finish_cycle(raw, None);
    assert_eq!(snap.positions.len(), 4);
    assert_eq!(snap.derived_metrics.global_status.closed_positions_count, None);
    assert!(!serde_json::to_value(&snap).unwrap().as_object().unwrap().contains_key("closed_positions"));

    // An Aero basic pool has no NFT and is never closed: a wallet with no LP in the pool
    // or its gauge still reads as a live position holding nothing (synthetic zeros
    // swapped in for the two real balances), and its gauge reward is still read.
    let pool = "0x770fE8Fc9248c6382950a86e39a9391b41d59e0a";
    let gauge = "0xc82782ee66d82c4d19db0429652091a085083526";
    let zero = Swap::Json(json!({ "result": format!("0x{}", "0".repeat(64)) }));
    let balance_of = erc20_balance_of_calldata(WALLET);
    let node = replay_node(vec![(key_call(pool, &balance_of), zero.clone()), (key_call(gauge, &balance_of), zero)]).await;
    let raw = read_cycle(&node, &book_without_cl()).await.unwrap();
    assert!(raw.closed_positions.is_empty());
    let basic = raw.positions.iter().find(|p| p.protocol == "aerodrome").unwrap();
    assert_eq!(basic.liquidity, 0);
    assert_eq!(node.count(&key_call(gauge, &gauge_earned_basic_calldata(WALLET))), 1, "earned AERO stays claimable after an unstake");
    assert!(raw.aero.unclaimed.as_ref().unwrap().iter().any(|u| u.position_id == basic.position_id && u.amount.is_some()));
}

// ------------------------------------------------------------------
// (b2) The proof is read at one current block
// ------------------------------------------------------------------
// A never-minted id answers byte for byte like a burned one, so a node whose state is
// older than an NFT's mint gives both "nonexistent" answers for a live position. The
// proof is therefore read at ONE block, and that block must be current.

#[test]
fn test_a_proof_block_must_be_current() {
    let time = Utc.timestamp_opt(fixture()["block_timestamp"].as_i64().unwrap(), 0).unwrap();
    let block = ProofBlock { number: fixture()["block"].as_i64().unwrap(), time };
    assert_eq!(block.tag(), "0x37edf0d");
    assert_eq!(block.tag(), proof_block_tag());
    let after = |seconds: i64| time + chrono::Duration::seconds(seconds);

    // The limit is 120 seconds and never longer than the cycle interval, so a proof
    // block is always newer than the previous cycle.
    assert_eq!(MAX_PROOF_BLOCK_AGE_SECONDS, 120);
    let limit = |sync_window: u64| ProofClock::for_cycle(time, sync_window).max_block_age_seconds;
    assert_eq!((limit(300), limit(120), limit(60), limit(0), limit(u64::MAX)), (120, 120, 60, 1, 120));
    // The clock is the cycle's start plus the time that has passed since: it never
    // runs behind its start, and it moves.
    let clock = ProofClock::for_cycle(after(3), 300);
    let (first, second) = (clock.now(), clock.now());
    assert!(first >= after(3) && second >= first && second < after(63), "{first} {second}");
    let example = load_app_config(concat!(env!("CARGO_MANIFEST_DIR"), "/config.yaml.example"), Some("/nonexistent/config.yaml"), false).unwrap();
    assert!(example.sync_window_seconds as i64 >= MAX_PROOF_BLOCK_AGE_SECONDS, "the example's interval keeps the full limit");

    for (seconds_after, sync_window, current) in [
        (0, 300, true),
        (3, 300, true),
        (120, 300, true),
        (121, 300, false),
        (3600, 300, false),
        (60, 60, true),
        (61, 60, false),
        // The block is ahead of the clock: fine within the limit (two clocks differ by
        // seconds), refused beyond it.
        (-120, 300, true),
        (-121, 300, false),
    ] {
        let verdict = proof_block_is_current(after(seconds_after), limit(sync_window), &block);
        assert_eq!(verdict.is_ok(), current, "{seconds_after}s after the block, window {sync_window}: {verdict:?}");
    }
    assert_eq!(
        proof_block_is_current(after(121), 120, &block).unwrap_err(),
        "the node's latest block 58646285 is 121 seconds old (limit 120): a node that is behind the chain cannot prove that a token id does not exist, because an id that was not minted yet answers the same way"
    );
    assert_eq!(
        proof_block_is_current(after(-121), 120, &block).unwrap_err(),
        "the node's latest block 58646285 is 121 seconds ahead of this host's clock (limit 120): its age cannot be judged, so it proves nothing about a token id"
    );
    // The adapters go through it, and the cycle's clock is the real one.
    for adapter in [include_str!("adapters/aero_cl.rs"), include_str!("adapters/uniswap_v3.rs"), include_str!("adapters/uniswap_v4.rs")] {
        assert!(adapter.contains("fetch_proof_block(http_client, alchemy_url, clock)"));
        assert!(include_str!("closed.rs").contains("proof_block_is_current(clock.now(), clock.max_block_age_seconds, &block)"));
        assert_eq!(adapter.matches("block.tag()").count(), 2, "both proving reads name the proof block");
    }
    assert!(include_str!("onchain.rs").contains("fetch_raw_data_merged(&state.http_client, &alchemy_url, &state.config, &merged, Utc::now())"));
}

#[tokio::test]
async fn test_a_node_behind_the_chain_fails_the_cycle_and_closes_nothing() {
    let block = proof_block_tag();
    let (v4_gone, v3_gone) = (entry_v4_nonexistent(false), entry_v3_nonexistent());
    let books: Vec<(&str, AppConfig, &str)> = vec![
        (
            "Aero CL",
            book_with_withdrawn_cl(),
            "position aero-cl200-weth-nock: Aero CL positions() read failed (NFT #2914490 on position manager 0x4857db576ee0e45f10525ec59527bfd10beec60b: positions() reverted with Error(\"ID\"), and the node's latest block 58646285 is ",
        ),
        (
            "Uniswap v4",
            config(&[ENTRY_V4, ENTRY_V3, ENTRY_BASIC, &v4_gone]),
            "position uniswap-v4-nock-usdc-gone: NFT #3229273 reads zero liquidity and whether it exists on PositionManager 0x7c5f5a4bbd8fd63184577525326123b519429bdc could not be established (ownerOf() reverted with Error(\"NOT_MINTED\"), and the node's latest block 58646285 is ",
        ),
        (
            "Uniswap v3",
            config(&[ENTRY_V4, ENTRY_V3, ENTRY_BASIC, &v3_gone]),
            "position uniswap-v3-weth-nock-gone: NPM positions() query failed (NFT #6604113 on position manager 0x03a520b32C04BF3bEEf7BEb72E919cf822Ed34f1: positions() reverted with Error(\"Invalid token ID\"), and the node's latest block 58646285 is ",
        ),
    ];
    let block_time = Utc.timestamp_opt(fixture()["block_timestamp"].as_i64().unwrap(), 0).unwrap();
    for (what, cfg, expect) in &books {
        // The node's latest block is the recorded one, and every answer is the real
        // "nonexistent" one. Only the worker's clock differs: 100 seconds after the
        // block the proof stands, 121 seconds after it the node is behind. (The exact
        // limit is pinned in test_a_proof_block_must_be_current; the cycle's clock also
        // counts the time the cycle itself takes, so these leave a margin.)
        let node = replay_node(vec![]).await;
        let raw = read_cycle_at(&node, cfg, block_time + chrono::Duration::seconds(100)).await.unwrap_or_else(|e| panic!("{what}: {e}"));
        assert_eq!(raw.closed_positions.len(), 1, "{what}");
        let node = replay_node(vec![]).await;
        let err = match read_cycle_at(&node, cfg, block_time + chrono::Duration::seconds(121)).await {
            Err(e) => e.to_string(),
            Ok(raw) => panic!("{what}: a node behind the chain must fail the cycle, but {} closed", raw.closed_positions.len()),
        };
        assert!(err.starts_with(expect), "{what}: {err}");
        assert!(err.ends_with(" seconds old (limit 120): a node that is behind the chain cannot prove that a token id does not exist, because an id that was not minted yet answers the same way)"), "{what}: {err}");
        assert_no_url(&err, &node.url);
        // The block was read once and no proving read was spent on it.
        assert_eq!(node.count(KEY_LATEST_BLOCK), 1, "{what}");
        assert!(node.seen_at.lock().unwrap().iter().all(|k| !k.ends_with(&format!("@{block}"))), "{what}");
        // A clock far behind the chain is refused as well.
        let node = replay_node(vec![]).await;
        let err = read_cycle_at(&node, cfg, block_time - chrono::Duration::seconds(180)).await.unwrap_err().to_string();
        assert!(err.contains(" seconds ahead of this host's clock (limit 120): its age cannot be judged"), "{what}: {err}");
        assert!(node.seen_at.lock().unwrap().iter().all(|k| !k.ends_with(&format!("@{block}"))), "{what}");
    }

    // The clock decides a proof and nothing else: a book with no closed entry, and one
    // with an NFT that exists and is empty, read the same on any clock and never ask
    // for the block.
    for cfg in [book_without_cl(), config(&[ENTRY_V4, ENTRY_V3, ENTRY_BASIC, ENTRY_V4_EMPTY])] {
        let node = replay_node(vec![]).await;
        let raw = read_cycle_at(&node, &cfg, block_time + chrono::Duration::days(365)).await.expect("no proof, no clock");
        assert!(raw.closed_positions.is_empty());
        assert_eq!(node.count(KEY_LATEST_BLOCK), 0);
    }

    // Both proving reads name the proof block, for every protocol.
    let node = replay_node(vec![]).await;
    read_cycle(&node, &config(&[ENTRY_V4, ENTRY_V3, ENTRY_BASIC, &v4_gone, &v3_gone])).await.unwrap();
    let v4_owner = key_owner_of(UNISWAP_V4_POSM_BASE, V4_BURNED_NFT);
    let v4_info = key_call(UNISWAP_V4_POSM_BASE, &format!("{V4_GET_POOL_AND_POSITION_INFO}{V4_BURNED_NFT:064x}"));
    assert_eq!(node.blocks_of(&v4_owner), vec!["latest".to_string(), block.clone()]);
    assert_eq!(node.blocks_of(&v4_info), vec![block.clone()]);
    assert_eq!(node.blocks_of(&key_positions(V3_NPM, V3_BURNED_NFT)), vec!["latest".to_string(), block.clone()]);
    assert_eq!(node.blocks_of(&key_owner_of(V3_NPM, V3_BURNED_NFT)), vec![block.clone()]);
    assert_eq!(node.count(KEY_LATEST_BLOCK), 2, "one block per proof");

    // What can go wrong between the trigger and the proof. Each fails the cycle.
    let cl_positions = key_positions(CL_NPM, CL_NFT);
    let cl_owner = key_owner_of(CL_NPM, CL_NFT);
    // (synthetic, by the JSON-RPC rules) what a backend answers for a block it does not hold
    let no_such_block = Swap::Json(json!({ "error": { "code": -32000, "message": "header not found" } }));
    let header = fixture()["responses"][KEY_LATEST_BLOCK]["result"].clone();
    assert_eq!(header["number"], json!(block));
    let mut header_without_time = header.clone();
    header_without_time.as_object_mut().unwrap().remove("timestamp");
    let wallet_owns = fixture()["responses"][key_owner_of(UNISWAP_V4_POSM_BASE, V4_EMPTY_NFT)].clone();
    assert!(wallet_owns["result"].is_string());
    let cases: Vec<(&str, AppConfig, String, Swap, &str)> = vec![
        ("the block read fails", book_with_withdrawn_cl(), KEY_LATEST_BLOCK.to_string(), Swap::Http(500), "and the node's latest block could not be read"),
        ("the block read is rate limited", book_with_withdrawn_cl(), KEY_LATEST_BLOCK.to_string(), Swap::Json(json!({ "error": { "code": -32005, "message": "rate limit exceeded" } })), "and the node's latest block could not be read"),
        ("the node knows no latest block", book_with_withdrawn_cl(), KEY_LATEST_BLOCK.to_string(), Swap::Json(json!({ "result": null })), "carries no readable number and time"),
        ("the header has no time", book_with_withdrawn_cl(), KEY_LATEST_BLOCK.to_string(), Swap::Json(json!({ "result": header_without_time })), "carries no readable number and time"),
        ("positions() lands on a backend without the proof block", book_with_withdrawn_cl(), key_at(&cl_positions, &block), no_such_block.clone(), "positions() read failed: RPC error -32000"),
        ("ownerOf() lands on a backend without the proof block", book_with_withdrawn_cl(), key_at(&cl_owner, &block), no_such_block.clone(), "ownerOf() read failed: RPC error -32000"),
        // The read at latest said "nonexistent" and the proof block says the NFT is
        // there (the real ownerOf answer of one block before the burn).
        ("the proof block names an owner", book_with_withdrawn_cl(), key_at(&cl_owner, &block), swap_real(REAL_CL_OWNER_OF_BEFORE_BURN), "at block 58646285: positions() reverted with Error(\"ID\") but ownerOf() returned 32 bytes of data: the two reads do not agree"),
        ("positions() answers another revert at the proof block", book_with_withdrawn_cl(), key_at(&cl_positions, &block), swap_real(REAL_GAUGE_EARNED_NA), "at block 58646285: positions() reverted with Error(\"NA\")"),
        ("v4: the proof block names an owner", config(&[ENTRY_V4, ENTRY_V3, ENTRY_BASIC, &v4_gone]), key_at(&v4_owner, &block), Swap::Json(wallet_owns), "at block 58646285: ownerOf() returned 32 bytes of data, which is not the PositionManager's answer for a token id that does not exist"),
        ("v4: ownerOf() lands on a backend without the proof block", config(&[ENTRY_V4, ENTRY_V3, ENTRY_BASIC, &v4_gone]), key_at(&v4_owner, &block), no_such_block.clone(), "POSM ownerOf() query failed: RPC error -32000"),
        ("v3: ownerOf() lands on a backend without the proof block", config(&[ENTRY_V4, ENTRY_V3, ENTRY_BASIC, &v3_gone]), key_at(&key_owner_of(V3_NPM, V3_BURNED_NFT), &block), no_such_block, "NPM ownerOf() query failed: RPC error -32000"),
    ];
    for (what, cfg, key, swap, expect) in cases {
        let node = replay_node(vec![(key, swap)]).await;
        let err = match read_cycle(&node, &cfg).await {
            Err(e) => e.to_string(),
            Ok(raw) => panic!("{what}: the cycle must fail, but it listed {} closed", raw.closed_positions.len()),
        };
        assert!(err.contains(expect), "{what}: {err}");
        assert_no_url(&err, &node.url);
    }
}

// ------------------------------------------------------------------
// (b3) A Uniswap v4 entry with no nft_id
// ------------------------------------------------------------------

#[tokio::test]
async fn test_a_v4_entry_without_nft_id_is_closed_and_is_not_a_position_holding_nothing() {
    let node = replay_node(vec![]).await;
    let no_id = entry_v4_without_nft_id();
    // The key stays optional: the config loads, and the id reads as 0.
    let alone = config(&[&no_id]);
    assert_eq!(alone.positions[0].nft_id, 0);
    // Token id 0 was never minted: the PositionManager's real answers at the recorded
    // block are the same bytes as for the burned id.
    let fx = fixture();
    assert_eq!(
        fx["responses"][key_owner_of(UNISWAP_V4_POSM_BASE, 0)]["error"],
        serde_json::from_str::<serde_json::Value>(REAL_V4_OWNER_OF_BURNED).unwrap()["error"]
    );
    assert_eq!(fx["responses"][key_call(UNISWAP_V4_POSM_BASE, &format!("{V4_GET_POOL_AND_POSITION_INFO}{:064x}", 0))]["result"], json!(REAL_V4_INFO_BURNED));

    // As the only entry (before #144 this completed, as one "position" holding $0 of an
    // NFT that does not exist): the cycle fails, like any config whose every position
    // is closed. A price-only setup is an `aerodrome` entry, which is never closed.
    let err = read_cycle(&node, &alone).await.unwrap_err().to_string();
    assert_eq!(
        err,
        "No live positions: every configured position is closed (uniswap-v4-nock-usdc-no-id: NFT #0 does not exist on position manager 0x7c5f5a4bbd8fd63184577525326123b519429bdc: withdrawn and burned, or a mistyped nft_id). Remove the closed entries and add your current positions under `positions:` in config.yaml (see ONCHAIN_INTEGRATION.md §5)"
    );

    // Beside live positions, as the price source: the cycle goes on, its pool supplies
    // the spot, and it is listed as closed, never among the positions.
    let live = read_cycle(&node, &book_without_cl()).await.unwrap();
    let raw = read_cycle(&node, &config(&[&no_id, ENTRY_V3, ENTRY_BASIC])).await.expect("the pool still prices NOCK");
    assert_eq!(raw.spot_price_nock_usd, live.spot_price_nock_usd);
    assert_eq!(raw.positions.len(), 2);
    assert!(raw.positions.iter().all(|p| p.protocol != "uniswap-v4"));
    let c = &raw.closed_positions[0];
    assert_eq!((c.nft_id, c.label.as_str(), c.supplies_spot_price), (0, "v4 USDC/NOCK #0 (uniswap-v4-nock-usdc-no-id)", true));

    // The example config says that a v4 entry needs the id of an NFT that exists, and
    // what a price-only setup uses instead.
    let example = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/config.yaml.example")).unwrap();
    assert!(example.contains("An entry with no nft_id reads as token id 0, which does not exist"), "config.yaml.example");
    assert!(example.contains("use an `aerodrome` entry on a NOCK/USDC pool with is_price_source: true"), "config.yaml.example");
}

// ------------------------------------------------------------------
// (c) A closed entry changes nothing else; (f) the arrays stay aligned
// ------------------------------------------------------------------

#[tokio::test]
async fn test_a_closed_entry_changes_no_figure_and_the_arrays_stay_aligned() {
    let node = replay_node(vec![]).await;
    let with_raw = read_cycle(&node, &book_with_withdrawn_cl()).await.unwrap();
    let without_raw = read_cycle(&node, &book_without_cl()).await.unwrap();
    // The reads themselves: spot, gas, rules, facts, AERO.
    assert_eq!(with_raw.spot_price_nock_usd, without_raw.spot_price_nock_usd);
    assert_eq!(with_raw.live_eth_price_usd, without_raw.live_eth_price_usd);
    assert_eq!(with_raw.rebuild_rules, without_raw.rebuild_rules);
    assert_eq!(with_raw.position_facts, without_raw.position_facts);
    assert_eq!(with_raw.pool_liquidity, without_raw.pool_liquidity);
    assert_eq!(with_raw.fees_not_lp_claimable, without_raw.fees_not_lp_claimable);
    assert_eq!(with_raw.aero, without_raw.aero);

    let (with, _) = finish_cycle(with_raw, None);
    let (without, _) = finish_cycle(without_raw, None);
    assert_eq!(with.closed_positions.len(), 1);
    assert!(without.closed_positions.is_empty());

    // Byte-equal: everything a consumer reads as holdings, totals, pillars, LP weights,
    // rebuild suggestions, fee and reward rows.
    let bytes = |v: &serde_json::Value| serde_json::to_string(v).unwrap();
    let (a, b) = (stable_json(&with), stable_json(&without));
    for key in ["wallet_balances", "positions", "pool_mechanics", "concentrated_liquidity", "contract_fees"] {
        assert_eq!(bytes(&a[key]), bytes(&b[key]), "{key}");
    }
    for key in [
        "asset_aggregation_series", "total_value_vs_baseline", "tick_bracket_hypervisor", "portfolio_deviation_matrix",
        "structural_channels", "strategy_counterfactual", "velocity_gas_sandbox", "sovereign_ledger", "lp_rebuild",
        "accumulation_plan", "cost_basis",
    ] {
        assert_eq!(bytes(&a["derived_metrics"][key]), bytes(&b["derived_metrics"][key]), "derived_metrics.{key}");
    }
    // global_status differs by the two closed fields and by nothing else, the
    // directive included.
    let mut gs = a["derived_metrics"]["global_status"].clone();
    assert_eq!(gs.as_object_mut().unwrap().remove("closed_positions_count"), Some(json!(1)));
    assert!(gs.as_object_mut().unwrap().remove("closed_positions_note").is_some());
    assert_eq!(bytes(&gs), bytes(&b["derived_metrics"]["global_status"]));
    // The whole snapshot differs by those two fields and the closed list.
    let mut whole = a.clone();
    whole.as_object_mut().unwrap().remove("closed_positions");
    whole["derived_metrics"]["global_status"] = gs;
    assert_eq!(bytes(&whole), bytes(&b));
    assert!(!bytes(&b).contains("closed"), "a book with no closed entry does not carry the word");
    // Nothing of the closed position is anywhere but in its own entry and the note.
    assert!(!bytes(&whole).contains("2914490") && !bytes(&whole).contains("aerodrome-cl"));

    // The figures are the live book's, and they add up without the withdrawn position.
    let gs = &with.derived_metrics.global_status;
    let lp: f64 = with.positions.iter().map(|p| p.amount_usd.parse::<f64>().unwrap()).sum();
    assert!(lp > 0.0);
    assert_eq!(with.derived_metrics.lp_rebuild.actual_usd, without.derived_metrics.lp_rebuild.actual_usd);
    let lp_reported: f64 = with.derived_metrics.lp_rebuild.actual_usd.parse().unwrap();
    assert!((lp - lp_reported).abs() < 0.02, "{lp} vs {lp_reported}");
    assert_eq!(gs.shot_advice_action, without.derived_metrics.global_status.shot_advice_action);
    assert!(!gs.shot_advice_details.contains("2914490") && !gs.shot_advice_details.to_lowercase().contains("closed"));
    // No LP weight, suggestion or exclusion names it.
    let lp_rebuild = serde_json::to_string(&with.derived_metrics.lp_rebuild).unwrap();
    assert!(!lp_rebuild.contains("2914490"), "{lp_rebuild}");

    // (f) index alignment: one pool_mechanics row, fee row, bracket and NOCK series per
    // live position, in the positions' order.
    let n = with.positions.len();
    assert_eq!(n, 3);
    assert_eq!(with.pool_mechanics.len(), n);
    assert_eq!(with.contract_fees.len(), n);
    assert_eq!(with.derived_metrics.tick_bracket_hypervisor.brackets.len(), n);
    assert_eq!(with.derived_metrics.asset_aggregation_series.positions_nock.len(), n);
    for (i, p) in with.positions.iter().enumerate() {
        assert_eq!(with.pool_mechanics[i].pool_id, p.pool_id, "pool_mechanics[{i}]");
        assert_eq!(with.pool_mechanics[i].liquidity_usd, p.amount_usd, "pool_mechanics[{i}]");
        assert_eq!(with.contract_fees[i].fee_token_symbol, fee_token_label(p), "contract_fees[{i}]");
        assert_eq!(with.derived_metrics.tick_bracket_hypervisor.brackets[i].liquidity_usd, p.amount_usd, "brackets[{i}]");
        assert_eq!(with.derived_metrics.asset_aggregation_series.positions_nock[i].position_id, p.position_id);
    }

    // Claim rows and reward rows: the claim payload builds actions for live entries only.
    let cfg = book_with_withdrawn_cl();
    let live: Vec<&str> = live_position_configs(&cfg.positions, &with.closed_positions).iter().map(|c| c.id.as_str()).collect();
    assert_eq!(live, vec!["uniswap-v4-nock-usdc-wide", "uniswap-v3-weth-nock", "aero-basic-nock-usdc"]);
    assert_eq!(live_position_configs(&cfg.positions, &[]).len(), 4, "a snapshot that lists none filters none");
    let claims = include_str!("claims.rs");
    // Since v0.9.0 the entries are config.yaml's and the database rows the snapshot holds.
    assert!(claims.contains("claim_position_configs(&state.config.positions, &tracked_rows, &snapshot.data)"));
    assert!(claims.contains("live_position_configs(&claim_configs, &snapshot.data.closed_positions)"));
    assert!(!claims.contains(".config\n        .positions\n        .iter()"), "claims.rs builds actions from the live entries only");
    assert_eq!(
        serde_json::to_string(&claim_reward_rows(&with.positions, &[])).unwrap(),
        serde_json::to_string(&claim_reward_rows(&without.positions, &[])).unwrap()
    );

    // The MCP text: the same text plus the closed block, on all four tools.
    let (full_with, full_without) = (serde_json::to_value(&with).unwrap(), serde_json::to_value(&without).unwrap());
    let block = closed_positions_lines(&full_with);
    assert_eq!(
        block,
        "• Closed positions (1), not counted in any total:\n   ✗ Aero CL #2914490 (aero-cl200-weth-nock): closed. NFT #2914490 does not exist on position manager 0x4857db576ee0e45f10525ec59527bfd10beec60b: withdrawn and burned, or a mistyped nft_id. Not counted in any total. You can remove its entry (aero-cl200-weth-nock) from config.yaml.\n"
    );
    assert_eq!(closed_positions_lines(&full_without), "");
    let state = format_portfolio_state_text(&full_with);
    assert!(state.contains(&block), "{state}");
    assert_eq!(state.replace(&block, ""), format_portfolio_state_text(&full_without));
    let claim = format_claim_simulation_text(&full_with);
    assert!(claim.ends_with(&block), "{claim}");
    assert_eq!(claim.replace(&block, ""), format_claim_simulation_text(&full_without));
    // The directive and the plan state the closing too: both list what the tracked
    // positions hold and are short of, and the fourth configured entry is in neither
    // list. Apart from the block, each text is what it is without a closed position:
    // the directive, its details and every rebuild line are unchanged.
    let advice = format_shot_advice_text(&full_with);
    assert!(advice.contains(&format!("{block}• Harvest rule: ")), "{advice}");
    assert_eq!(advice.replace(&block, ""), format_shot_advice_text(&full_without));
    let trailer = closed_positions_trailer(&full_with);
    assert_eq!(trailer, format!("\n{}", block.trim_end()));
    assert_eq!(closed_positions_trailer(&full_without), "");
    let plan = format_accumulation_plan_text(&full_with);
    assert!(plan.starts_with("🪜 Accumulation Plan\n====================\n• No accumulation_plan is configured.\n") && plan.ends_with(&trailer), "{plan}");
    assert_eq!(plan.replace(&trailer, ""), format_accumulation_plan_text(&full_without));
    assert!(plan.contains("LP rebuild bias: under_target") && !plan.replace(&trailer, "").contains("2914490"), "{plan}");
    // With a plan configured (a synthetic plan object, for the text's other branch).
    let planned = |full: &serde_json::Value| {
        let mut v = full.clone();
        v["derived_metrics"]["accumulation_plan"] = json!({ "plan_id": "synthetic-test-plan", "status": "waiting" });
        format_accumulation_plan_text(&v)
    };
    assert!(planned(&full_with).starts_with("🪜 Accumulation Plan \"synthetic-test-plan\"") && planned(&full_with).ends_with(&trailer));
    assert_eq!(planned(&full_with).replace(&trailer, ""), planned(&full_without));
    // No surface text advises anything about AERO because of it.
    for text in [&state, &claim, &advice, &plan] {
        let lower = text.to_lowercase();
        assert!(!lower.contains("claim aero") && !lower.contains("sell aero") && !lower.contains("aerodrome.finance"), "{text}");
    }
}

// ------------------------------------------------------------------
// (d) The price source
// ------------------------------------------------------------------

#[tokio::test]
async fn test_a_closed_price_source_never_guesses_a_price() {
    let node = replay_node(vec![]).await;
    let live = read_cycle(&node, &book_without_cl()).await.unwrap();

    // Uniswap v4: the pool goes on supplying the spot through the same two reads
    // (poolKeys and getSlot0), and the stale-pool guard still runs.
    let v4_gone_source = entry_v4_nonexistent(true);
    let cfg = config(&[&v4_gone_source, ENTRY_V3, ENTRY_BASIC]);
    node.seen.lock().unwrap().clear();
    let raw = read_cycle(&node, &cfg).await.expect("the pool still prices NOCK");
    assert_eq!(raw.spot_price_nock_usd, live.spot_price_nock_usd, "the same pool at the same block gives the same spot");
    assert_eq!(raw.positions.len(), 2);
    assert!(raw.positions.iter().all(|p| !p.is_price_source), "the closed entry's pool is never a position");
    let c = &raw.closed_positions[0];
    assert!(c.supplies_spot_price);
    let pool_id = "064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13";
    assert_eq!(node.count(&key_call(UNISWAP_V4_STATE_VIEW_BASE, &format!("{V4_STATE_VIEW_GET_SLOT0}{pool_id}"))), 1);
    assert_eq!(node.count(&key_call(UNISWAP_V4_STATE_VIEW_BASE, &format!("{V4_STATE_VIEW_GET_LIQUIDITY}{pool_id}"))), 1, "the stale-pool guard");
    let (snap, notes) = finish_cycle(raw, None);
    assert!(notes.is_empty());
    assert!(serde_json::to_value(&snap.closed_positions[0]).unwrap()["supplies_spot_price"] == json!(true));
    let note = snap.derived_metrics.global_status.closed_positions_note.clone().unwrap();
    assert_eq!(
        note,
        "1 configured position is closed and is not counted in any total: v4 USDC/NOCK #3229273 (uniswap-v4-nock-usdc-gone) (NFT #3229273 no longer exists: withdrawn and burned, or a mistyped nft_id). The pool of v4 USDC/NOCK #3229273 (uniswap-v4-nock-usdc-gone) still supplies the NOCK/USD spot: flag another position as is_price_source before removing that entry."
    );
    // Removing this entry first would leave no price source and fail every cycle, so
    // no text may say it "can be removed": not the note, and not the push.
    assert!(!note.contains("can be removed"), "{note}");
    let pushed = closed_position_notes(Some(&[snap.closed_positions[0].position_id.clone()]), &snap.closed_positions);
    assert_eq!(pushed, vec![
        "Position closed: v4 USDC/NOCK #3229273 (uniswap-v4-nock-usdc-gone). NFT #3229273 no longer exists on 0x7c5f5a4bbd8fd63184577525326123b519429bdc (withdrawn and burned, or a mistyped nft_id). It is no longer counted in any total. Its pool still supplies the NOCK/USD spot: flag another position as is_price_source before removing its entry (uniswap-v4-nock-usdc-gone) from config.yaml.".to_string()
    ]);
    assert!(!pushed[0].contains("can be removed") && !pushed[0].contains("AERO"));
    assert!(pushed[0].chars().count() < PUSHOVER_MESSAGE_LIMIT / 2);
    // Beside a closed entry that does not supply the price: only that one is removable.
    let raw_two = read_cycle(&node, &config(&[&v4_gone_source, ENTRY_V3, ENTRY_BASIC, ENTRY_CL_WITHDRAWN])).await.unwrap();
    let (snap_two, _) = finish_cycle(raw_two, None);
    let note_two = snap_two.derived_metrics.global_status.closed_positions_note.clone().unwrap();
    assert!(note_two.starts_with("2 configured positions are closed and are not counted in any total: "), "{note_two}");
    assert!(
        note_two.ends_with("or a mistyped nft_id). The entry of Aero CL #2914490 (aero-cl200-weth-nock) can be removed from config.yaml. The pool of v4 USDC/NOCK #3229273 (uniswap-v4-nock-usdc-gone) still supplies the NOCK/USD spot: flag another position as is_price_source before removing that entry."),
        "{note_two}"
    );
    assert!(!note_two.contains("Their entries") && note_two.matches("can be removed").count() == 1, "{note_two}");
    let both_live_before: Vec<String> = snap_two.closed_positions.iter().map(|c| c.position_id.clone()).collect();
    let pushed_two = closed_position_notes(Some(&both_live_before), &snap_two.closed_positions);
    assert_eq!(pushed_two.iter().filter(|t| t.contains("can be removed")).count(), 1);
    assert!(pushed_two.iter().any(|t| t.contains("Aero CL #2914490") && t.ends_with("Its entry (aero-cl200-weth-nock) can be removed from config.yaml.")));
    let text = closed_positions_lines(&serde_json::to_value(&snap).unwrap());
    assert!(text.contains("Its pool still supplies the NOCK/USD spot: flag another position as is_price_source before removing its entry from config.yaml."), "{text}");
    assert!(!text.contains("You can remove its entry"), "{text}");
    // No range-exit reading comes from a position that does not exist.
    assert!(snap.positions.iter().all(|p| p.nft_id != V4_BURNED_NFT));

    // A frozen pool is still refused: zero in-range liquidity fails the cycle.
    let frozen = Swap::Json(json!({ "result": format!("0x{}", "0".repeat(64)) }));
    let node_frozen = replay_node(vec![(key_call(UNISWAP_V4_STATE_VIEW_BASE, &format!("{V4_STATE_VIEW_GET_LIQUIDITY}{pool_id}")), frozen)]).await;
    let err = read_cycle(&node_frozen, &cfg).await.unwrap_err().to_string();
    assert!(err.starts_with("Stale NOCK/USD spot"), "{err}");
    // A failed price read fails the cycle: nothing is carried over.
    let node_down = replay_node(vec![(key_call(UNISWAP_V4_STATE_VIEW_BASE, &format!("{V4_STATE_VIEW_GET_SLOT0}{pool_id}")), Swap::Http(500))]).await;
    let err = read_cycle(&node_down, &cfg).await.unwrap_err().to_string();
    assert!(err.contains("StateView getSlot0 query failed"), "{err}");

    // Aero CL and Uniswap v3: the pool was identified through the NFT, so the cycle
    // fails with a configuration message that names the fix.
    let cl_source = format!("{ENTRY_CL_WITHDRAWN}\n    is_price_source: true");
    let v4_not_source = ENTRY_V4.replace("is_price_source: true", "is_price_source: false");
    let err = read_cycle(&node, &config(&[&v4_not_source, ENTRY_V3, ENTRY_BASIC, &cl_source])).await.unwrap_err().to_string();
    assert_eq!(
        err,
        "Configuration error: position aero-cl200-weth-nock is the NOCK/USD price source (is_price_source: true) and is closed: NFT #2914490 does not exist on position manager 0x4857db576ee0e45f10525ec59527bfd10beec60b: withdrawn and burned, or a mistyped nft_id. The pool of an entry with protocol aerodrome-cl is identified through its NFT, so no price is read from it and none is guessed. Flag another position as is_price_source: true in config.yaml"
    );
    let v3_source = format!("{}\n    is_price_source: true", entry_v3_nonexistent());
    let err = read_cycle(&node, &config(&[&v4_not_source, ENTRY_BASIC, &v3_source])).await.unwrap_err().to_string();
    assert!(err.starts_with("Configuration error: position uniswap-v3-weth-nock-gone is the NOCK/USD price source") && err.ends_with("Flag another position as is_price_source: true in config.yaml"), "{err}");
    assert!(!closed_price_source_keeps_pricing("aerodrome-cl") && !closed_price_source_keeps_pricing("uniswap-v3"));
    assert!(closed_price_source_keeps_pricing("uniswap-v4") && !closed_price_source_keeps_pricing("aerodrome") && !closed_price_source_keeps_pricing("x"));

    // A live flagged position always supplies the spot: a closed entry that is also
    // flagged is listed, and neither fails the cycle nor claims to supply the price.
    let raw = read_cycle(&node, &config(&[ENTRY_V4, ENTRY_V3, ENTRY_BASIC, &cl_source])).await.unwrap();
    assert_eq!(raw.spot_price_nock_usd, live.spot_price_nock_usd);
    assert!(!raw.closed_positions[0].supplies_spot_price);

    // Every configured position closed: the cycle fails, as with none configured (#119).
    let err = read_cycle(&node, &config(&[&v4_gone_source, ENTRY_CL_WITHDRAWN])).await.unwrap_err().to_string();
    assert!(err.starts_with("No live positions: every configured position is closed (uniswap-v4-nock-usdc-gone: NFT #3229273 does not exist"), "{err}");
    assert!(err.contains("aero-cl200-weth-nock: NFT #2914490 does not exist"), "{err}");
    let err_none = read_cycle(&node, &config(&[])).await.unwrap_err().to_string();
    assert!(err_none.starts_with("No positions configured"), "{err_none}");
}

// ------------------------------------------------------------------
// (e) One notification, at the transition
// ------------------------------------------------------------------

#[tokio::test]
async fn test_the_closing_is_pushed_once_at_the_live_to_closed_transition() {
    let node = replay_node(vec![]).await;
    // The previous snapshot: the real cycle of 2026-11-16 (block 58077673), in which
    // NFT #2914490 was a live, staked position.
    let prev = before::snapshot_of(before::real_cycle(), &before::example_profile(), 0.0);
    assert!(prev.positions.iter().any(|p| p.position_id == "aerodrome-cl:base:2914490"));

    let raw = read_cycle(&node, &book_with_withdrawn_cl()).await.unwrap();
    let (first, notes) = finish_cycle(raw, Some(&prev));
    assert_eq!(notes, vec![
        "Position closed: Aero CL #2914490 (aero-cl200-weth-nock). NFT #2914490 no longer exists on 0x4857db576ee0e45f10525ec59527bfd10beec60b (withdrawn and burned, or a mistyped nft_id). It is no longer counted in any total. Its entry (aero-cl200-weth-nock) can be removed from config.yaml.".to_string()
    ]);
    let action = first.derived_metrics.global_status.shot_advice_action.clone();
    let details = first.derived_metrics.global_status.shot_advice_details.clone();
    // The directive did not change: the closing notifies on its own.
    let text = cycle_notification_with_closed(Some(&action), &action, &details, None, None, None, &notes, Some("http://chudmax:3000/?view=lite")).unwrap();
    assert_eq!(text, format!("🛡️ NockNock: {}\nDirective unchanged: {}\n→ http://chudmax:3000/?view=lite", notes[0], action));
    // With a directive change the sentence comes before the details.
    let text = cycle_notification_with_closed(Some("HOLD & ACCRETE (Dual Growth Engine Active)"), "REBUILD LP TRANCHES (No Net Cash Change)", "details", None, None, None, &notes, None).unwrap();
    assert_eq!(text, format!("🛡️ NockNock directive change: REBUILD LP TRANCHES (No Net Cash Change)\n{}\ndetails", notes[0]));
    // It fits a Pushover message whole, and says nothing about AERO.
    assert!(notes[0].chars().count() < PUSHOVER_MESSAGE_LIMIT / 2);
    assert!(!notes[0].contains("AERO"));

    // The next cycle: the previous snapshot already lists it as closed. Nothing is
    // pushed, and the first-seen time is carried, not restamped.
    let raw = read_cycle(&node, &book_with_withdrawn_cl()).await.unwrap();
    let plan_later = "2026-11-17T13:05:00+00:00";
    let derived = first.derived_metrics.clone();
    let (second, notes) = cycle_snapshot_data(raw, derived, Some(&first), plan_later);
    assert!(notes.is_empty());
    assert_eq!(second.closed_positions[0].first_seen_closed_at.as_deref(), Some(NOW));
    assert_eq!(cycle_notification_with_closed(Some(&action), &action, &details, None, None, None, &notes, None), None);
    // Nothing changed on-chain, so the state hash is the same and no new row is stored.
    assert_eq!(compute_state_hash(&second), compute_state_hash(&first));

    // A restart: the worker reloads its last snapshot from the database (JSON), which
    // lists the position as closed. Still nothing.
    let reloaded: SnapshotData = serde_json::from_value(serde_json::to_value(&first).unwrap()).unwrap();
    let raw = read_cycle(&node, &book_with_withdrawn_cl()).await.unwrap();
    let (third, notes) = finish_cycle(raw, Some(&reloaded));
    assert!(notes.is_empty());
    assert_eq!(third.closed_positions[0].first_seen_closed_at.as_deref(), Some(NOW));

    // A first cycle with no previous snapshot stays silent, like every notification.
    let raw = read_cycle(&node, &book_with_withdrawn_cl()).await.unwrap();
    assert!(finish_cycle(raw, None).1.is_empty());
    // An entry that was never a tracked position (a new one with a mistyped id): the
    // previous snapshot held neither a live nor a closed row for it. Listed, not pushed.
    let raw = read_cycle(&node, &book_without_cl()).await.unwrap();
    let (three_live, _) = finish_cycle(raw, None);
    let raw = read_cycle(&node, &book_with_withdrawn_cl()).await.unwrap();
    let (snap, notes) = finish_cycle(raw, Some(&three_live));
    assert_eq!(snap.closed_positions.len(), 1);
    assert!(notes.is_empty());

    // No closed sentence: the notification is exactly what it was before #144.
    for (prev_action, next_action) in [(Some("HOLD & ACCRETE (LP Empty)"), "BATCH HARVEST SHOT"), (Some("A"), "A"), (None, "A")] {
        assert_eq!(
            cycle_notification_with_closed(prev_action, next_action, "d", None, None, Some("range"), &[], Some("u")),
            cycle_notification(prev_action, next_action, "d", None, None, Some("range"), Some("u"))
        );
    }
    // The pure rule, stated directly.
    let closed = first.closed_positions.clone();
    let live = vec!["aerodrome-cl:base:2914490".to_string()];
    assert_eq!(closed_position_notes(Some(&live), &closed).len(), 1);
    assert!(closed_position_notes(Some(&[]), &closed).is_empty());
    assert!(closed_position_notes(None, &closed).is_empty());
    assert!(closed_position_notes(Some(&live), &[]).is_empty());
    // The status endpoint says when the worker notifies.
    let status = notification_status_body(&NotificationConfig::default());
    assert!(status["fires_on"].as_str().unwrap().contains("a tracked position closing"));
    // run_ingestion_cycle goes through the same two functions.
    let ingest = include_str!("ingest.rs");
    assert!(ingest.contains("cycle_snapshot_data(raw_data, derived_metrics, prev.as_ref().map(|s| &s.data)"));
    assert!(ingest.contains("cycle_notification_with_closed("));
    assert!(ingest.contains("&closed_notes,"));
}

// ------------------------------------------------------------------
// Config, spec and the golden file
// ------------------------------------------------------------------

#[test]
fn test_closed_positions_need_no_new_config_key_and_are_in_the_spec() {
    // The shipped example still loads, and the test configs use only existing keys.
    let example = concat!(env!("CARGO_MANIFEST_DIR"), "/config.yaml.example");
    load_app_config(example, Some("/nonexistent/config.yaml"), false).expect("example config loads");
    assert_eq!(book_with_withdrawn_cl().positions.len(), 4);
    let text = std::fs::read_to_string(example).unwrap();
    assert!(text.contains("reported as closed"), "the example config says what happens to a withdrawn position");
    // The live entries of these tests are the example's own.
    let example_cfg = load_app_config(example, Some("/nonexistent/config.yaml"), false).unwrap();
    let ids = |c: &AppConfig| c.positions.iter().map(|p| (p.id.clone(), p.nft_id, position_key(p))).collect::<Vec<_>>();
    assert_eq!(ids(&example_cfg), ids(&book_without_cl()));

    // 2.8.0 brought closed positions; the v0.8.1 burn-down moved it to 2.8.1, and
    // TickBracket.position_id (#152) to 2.8.2, the standing reserve (#162) to 2.8.3 and
    // the floor-crossing price (#165) to 2.9.0, and the fee yield and the operator's
    // ladder shape (#166 rungs) to 2.10.0.
    assert_eq!(SNAPSHOT_SPEC_VERSION, "2.10.0");
    let spec = include_str!("../../specs/core/snapshot.yaml");
    assert!(spec.contains("  version: 2.10.0"));
    for field in [
        "closed_positions:", "ClosedPosition:", "ClosedEvidence:", "config_id:", "position_manager:", "first_seen_closed_at:",
        "supplies_spot_price:", "closed_positions_count:", "closed_positions_note:",
    ] {
        assert!(spec.contains(field), "specs/core/snapshot.yaml documents {field}");
    }
    assert!(spec.matches("(#144)").count() >= 15);
    let parsed: serde_yaml::Value = serde_yaml::from_str(spec).expect("the snapshot spec parses as YAML");
    let schemas = &parsed["components"]["schemas"];
    assert!(schemas["ClosedPosition"]["properties"]["evidence"].is_mapping());
    assert!(schemas["SnapshotData"]["properties"]["closed_positions"].is_mapping());
    // closed_positions is optional: a snapshot from before 2.8.0 is still valid.
    let required: Vec<&str> = schemas["SnapshotData"]["required"].as_sequence().unwrap().iter().filter_map(|v| v.as_str()).collect();
    assert!(!required.contains(&"closed_positions"));
    // Every field the worker serializes for a closed position is in the spec.
    let entry = serde_json::to_value(closed_position_entry(
        &book_with_withdrawn_cl().positions[3],
        ClosedProof { position_manager: CL_NPM.to_string(), evidence: vec![] },
        true,
    ))
    .unwrap();
    for key in entry.as_object().unwrap().keys() {
        assert!(schemas["ClosedPosition"]["properties"][key.as_str()].is_mapping(), "ClosedPosition.{key} is in the spec");
    }
    // The module that decides closure is where the comments say it is.
    assert!(include_str!("adapters/mod.rs").contains("pub(crate) fn nft_nonexistence_rule("));
}

// Regenerate only for an intended change:
//   NOCKNOCK_BLESS_GOLDEN=1 cargo test test_golden_snapshot
#[tokio::test]
async fn test_golden_snapshot_closed_is_byte_identical() {
    let node = replay_node(vec![]).await;
    let case = |data: &SnapshotData| {
        let full = serde_json::to_value(data).unwrap();
        json!({
            "state_hash": compute_state_hash(data),
            "snapshot": canonical_state_json(data),
            "mcp_text": {
                "get_portfolio_state": format_portfolio_state_text(&full),
                "get_shot_advice": format_shot_advice_text(&full),
                "simulate_claim_fees": format_claim_simulation_text(&full),
                "get_accumulation_plan": format_accumulation_plan_text(&full),
            },
        })
    };
    let (withdrawn, _) = finish_cycle(read_cycle(&node, &book_with_withdrawn_cl()).await.unwrap(), None);
    let v4_gone_source = entry_v4_nonexistent(true);
    let (price_pool, _) = finish_cycle(read_cycle(&node, &config(&[&v4_gone_source, ENTRY_V3, ENTRY_BASIC])).await.unwrap(), None);
    let doc = json!({
        "aero_cl_withdrawn_block_58646285": case(&withdrawn),
        "v4_price_source_closed_block_58646285": case(&price_pool),
    });
    let actual = format!("{}\n", serde_json::to_string_pretty(&doc).unwrap());
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/golden_snapshot_closed.json");
    if std::env::var("NOCKNOCK_BLESS_GOLDEN").is_ok() {
        std::fs::write(path, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(path).expect("testdata/golden_snapshot_closed.json is checked in");
    assert!(
        actual == expected,
        "snapshot bytes changed; compare {path} with a fresh NOCKNOCK_BLESS_GOLDEN=1 run and `git diff` it"
    );
}

// ------------------------------------------------------------------
// Ladders (#167) through the whole recorded cycle
// ------------------------------------------------------------------

/// The wallet's own emptied v4 NFT #3163431 (USDC/NOCK, ticks 267825..271050) configured
/// as a ladder, with a deposit_tx the replay node holds no receipt for.
fn entry_v4_empty_as_ladder() -> String {
    format!(
        "{ENTRY_V4_EMPTY}\n    role: ladder\n    rung: deep\n    deposit_tx: \"0x{}\"",
        "ab".repeat(32)
    )
}

#[tokio::test]
async fn test_a_ladder_is_read_measured_and_its_unread_receipt_is_unverified_not_zero() {
    let node = replay_node(vec![]).await;
    let raw = read_cycle(&node, &config(&[ENTRY_V4, ENTRY_V3, ENTRY_BASIC, &entry_v4_empty_as_ladder()])).await.expect("a ladder does not fail the cycle");
    let id = format!("uniswap-v4:base:{V4_EMPTY_NFT}");
    assert_eq!(raw.ladders.get(&id), Some(&LadderTag { config_id: "uniswap-v4-nock-usdc-retired".to_string(), rung: Some("deep".to_string()), deposit_tx: Some(format!("0x{}", "ab".repeat(32))) }));
    // The receipt read failed (the node holds no answer for it): unverified, not cached.
    assert_eq!(node.count("eth_getTransactionReceipt"), 3, "one read, retried twice: {:?}", node.seen.lock().unwrap());
    assert!(matches!(raw.ladder_baselines.get(&id), Some(LadderBaselineRead::Unverified { .. })), "{:?}", raw.ladder_baselines);
    let (snap, _) = finish_cycle(raw, None);
    let p = snap.positions.iter().find(|p| p.position_id == id).unwrap();
    assert_eq!((p.role.as_deref(), p.rung.as_deref()), (Some("ladder"), Some("deep")));
    assert_eq!(snap.ladders.len(), 1);
    let l = &snap.ladders[0];
    assert_eq!((l.state.as_str(), l.since_deposit.status.as_str()), ("empty", "unverified"));
    assert!(l.since_deposit.reason.as_deref().unwrap().starts_with("the receipt could not be read this cycle (the RPC did not answer)"), "{:?}", l.since_deposit.reason);
    assert_eq!(l.since_deposit.net_nock_bought, None);
    // Not NOCK LP: no rebuild share, no exclusion.
    assert!(snap.derived_metrics.lp_rebuild.suggestions.iter().all(|s| s.position_id != id));
    assert!(snap.derived_metrics.lp_rebuild.excluded.iter().all(|e| e.position_id != id));
    // The other three positions are exactly what they are without the role.
    let plain = read_cycle(&replay_node(vec![]).await, &config(&[ENTRY_V4, ENTRY_V3, ENTRY_BASIC])).await.unwrap();
    let (plain_snap, _) = finish_cycle(plain, None);
    assert_eq!(serde_json::to_value(&snap.positions[..3]).unwrap(), serde_json::to_value(&plain_snap.positions[..]).unwrap());
}

#[tokio::test]
async fn test_a_ladder_on_a_weth_nock_pair_fails_the_cycle_after_the_read() {
    let node = replay_node(vec![]).await;
    let ladder_v3 = format!("{ENTRY_V3}\n    role: ladder");
    let e = read_cycle(&node, &config(&[ENTRY_V4, &ladder_v3, ENTRY_BASIC])).await.unwrap_err().to_string();
    assert_eq!(e, "Configuration error: position 'uniswap-v3-weth-nock' is role: ladder but is a WETH/NOCK pair. A ladder is a USDC/NOCK range; AERO and ETH ladders are not supported yet (#170). Correct `positions:` in config.yaml");
    assert_no_url(&e, &node.url);
}

#[tokio::test]
async fn test_an_unknown_role_fails_the_cycle_before_any_read() {
    let node = replay_node(vec![]).await;
    let bad = format!("{ENTRY_V3}\n    role: Ladder");
    let e = read_cycle(&node, &config(&[ENTRY_V4, &bad])).await.unwrap_err().to_string();
    assert!(e.contains("position 'uniswap-v3-weth-nock' has unknown role 'Ladder'"), "{e}");
    assert!(node.seen.lock().unwrap().is_empty(), "nothing is read");
}
