//! Positions found on-chain (position_detect.rs; v0.9.0, #145 for Uniswap v4).
//!
//! The chain answers are real. `testdata/position_detect/chain_fixture.json` was recorded
//! read-only from https://base-rpc.publicnode.com on 2026-11-21 and holds two scenarios:
//! - `operator`: the tracked wallet at block 58646285 (the block of the #144 cycle
//!   fixture). Its 43 Uniswap v4 position NFTs: `getPoolAndPositionInfo` and `ownerOf` for
//!   each, `getPositionLiquidity` for the six in whitelisted pools, the wallet's
//!   `balanceOf`, the mint receipt of the one that holds liquidity (#3163450), and the
//!   node's answers to the transfer-log requests: a refusal for the whole history and for
//!   a range older than it serves, and the chunks around the wallet's newest mints.
//!   `wide_logs` is the same request answered by an RPC without a range limit (the
//!   design's explorer, same day); three of its 43 logs are also in a public-node chunk.
//! - `minter`: the wallet that minted NFT #3240369 (the #167 fixture receipt), at the
//!   block of that mint. A position its own wallet minted, found from the chain alone.
//!
//! Three things are NOT a node's own bytes, and each says so where it is used:
//! - the block headers the local node serves are rebuilt from the recorded number and
//!   time (the scan reads `POSITION_HEAD_LAG_BLOCKS` behind the node's latest, so the
//!   local node reports the recorded block plus that lag);
//! - the `alchemy_getAssetTransfers` ERC-721 page is ASSEMBLED in Alchemy's documented
//!   shape from the real logs: that method needs an Alchemy key, which no test has;
//! - `testdata/position_detect/simulated_rung_mints.json` is SIMULATED (eth_simulateV1),
//!   not mined: the near and deep rung mints the operator has not sent yet.
//!
//! A request the fixture does not hold is answered with a JSON-RPC error, never an
//! invented value.
use super::*;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

const CHAIN: &str = include_str!("../testdata/position_detect/chain_fixture.json");
const CLOSED_CYCLE: &str = include_str!("../testdata/closed/cycle_fixture.json");
const SIMULATED: &str = include_str!("../testdata/position_detect/simulated_rung_mints.json");
const MINT_AFE4: &str = include_str!("../testdata/ladder/receipt_v4_mint_0xafe42fe2.json");

pub(crate) const OPERATOR: &str = "0xee114cc97ec5adeb34ba5645e315b497d8487a45";
const MINTER: &str = "0x4598b3ba7d6714091b18da5d110b64f56e94e826";
pub(crate) const POOL_3PCT: &str = "0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13";
const POOL_075PCT: &str = "0x91a9ef0a8dd231b99fa53a2a145a9e8bca7897e0d85852107abb997916f9f99c";
const USDC: &str = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913";
const NOCK: &str = "0x9b5e262cf9bb04869ab40b19af91d2dc85761722";
const WETH: &str = "0x4200000000000000000000000000000000000006";
/// The operator wallet's own mint of NFT #3163450 (block 53008241).
pub(crate) const MINT_6CDA: &str = "0x6cdabada1ff062b6d3b4ba87bfe04158dd8b14a91e93cabe11dc3e35b587e89e";
const MINT_AFE4_HASH: &str = "0xafe42fe29de67337d5b489226dbfdf8a8b315e2deabaf6c744b47c2dd9ff7a58";

fn chain() -> Value {
    serde_json::from_str(CHAIN).expect("testdata/position_detect/chain_fixture.json parses")
}

pub(crate) fn scenario(name: &str) -> Value {
    chain()[name].clone()
}

fn recorded(name: &str, key: &str) -> Value {
    scenario(name)["responses"].get(key).cloned().unwrap_or_else(|| panic!("the fixture holds no {key}"))
}

fn call_key(to: &str, data: &str) -> String {
    format!("eth_call|{}|{}", to.to_lowercase(), data.to_lowercase())
}

fn key_info(nft: u64) -> String {
    call_key(UNISWAP_V4_POSM_BASE, &format!("{V4_GET_POOL_AND_POSITION_INFO}{nft:064x}"))
}

pub(crate) fn key_owner(nft: u64) -> String {
    call_key(UNISWAP_V4_POSM_BASE, &format!("{ERC721_OWNER_OF}{nft:064x}"))
}

pub(crate) fn key_liquidity(nft: u64) -> String {
    call_key(UNISWAP_V4_POSM_BASE, &format!("{V4_GET_POSITION_LIQUIDITY}{nft:064x}"))
}

fn key_logs(wallet: &str, from: i64, to: i64) -> String {
    format!("eth_getLogs|{}|0x{from:x}|0x{to:x}", wallet.to_lowercase())
}

fn key_receipt(tx: &str) -> String {
    format!("eth_getTransactionReceipt|{}", tx.to_lowercase())
}

/// How a request is found in the fixtures: the method and what it reads, lowercase.
fn request_key(req: &Value) -> String {
    let method = req["method"].as_str().unwrap_or_default();
    let lower = |v: &Value| v.as_str().unwrap_or_default().to_lowercase();
    match method {
        "eth_call" => {
            let tx = &req["params"][0];
            let from = if tx.get("from").is_some() { format!("|from={}", lower(&tx["from"])) } else { String::new() };
            format!("eth_call|{}|{}{}", lower(&tx["to"]), lower(&tx["data"]), from)
        }
        "eth_getLogs" => {
            let f = &req["params"][0];
            let to_topic = lower(&f["topics"][2]);
            format!("eth_getLogs|0x{}|{}|{}", &to_topic[to_topic.len().saturating_sub(40)..], lower(&f["fromBlock"]), lower(&f["toBlock"]))
        }
        "eth_getTransactionReceipt" => format!("{method}|{}", lower(&req["params"][0])),
        "eth_getBalance" | "alchemy_getTokenBalances" => format!("{method}|{}", lower(&req["params"][0])),
        other => other.to_string(),
    }
}

/// What the local node answers for one key instead of the recorded answer.
#[derive(Clone)]
pub(crate) enum Swap {
    /// The `result` / `error` member of a JSON-RPC response; `http_status` sends it with
    /// that HTTP status, as the fixture records a refusal.
    Json(Value),
    /// The connection is dropped without a response: a transport failure.
    Drop,
}

pub(crate) struct Node {
    pub(crate) url: String,
    /// Every request: (key, the block an eth_call named).
    seen: Arc<std::sync::Mutex<Vec<(String, String)>>>,
}

impl Node {
    fn keys(&self) -> Vec<String> {
        self.seen.lock().unwrap().iter().map(|(k, _)| k.clone()).collect()
    }

    pub(crate) fn count(&self, prefix: &str) -> usize {
        self.keys().iter().filter(|k| k.starts_with(prefix)).count()
    }

    fn total(&self) -> usize {
        self.seen.lock().unwrap().len()
    }

    fn forget(&self) {
        self.seen.lock().unwrap().clear();
    }

    /// The block tags the eth_calls to `to` named.
    fn call_blocks(&self, to: &str) -> HashSet<String> {
        let prefix = format!("eth_call|{}|", to.to_lowercase());
        self.seen.lock().unwrap().iter().filter(|(k, _)| k.starts_with(&prefix)).map(|(_, b)| b.clone()).collect()
    }
}

/// The scan block of a scenario: the recorded block.
fn scan_block(name: &str) -> i64 {
    scenario(name)["number"].as_i64().unwrap()
}

/// The clock of a replayed cycle: a few seconds after the block the local node reports.
pub(crate) fn clock(name: &str) -> DateTime<Utc> {
    Utc.timestamp_opt(scenario(name)["block_timestamp"].as_i64().unwrap() + 4, 0).unwrap()
}

/// A local JSON-RPC server replaying one scenario, then the #144 cycle fixture (the reads
/// of a whole ingestion cycle at the same block as `operator`).
/// - `eth_getBlockByNumber("latest")` is REBUILT: the recorded block plus the scan's lag,
///   one second later, so the scan's own block is the recorded one.
/// - `eth_getBlockByNumber(<n>)` is REBUILT from the block time a recorded receipt's logs
///   carry for that block; a block no receipt names is an error.
pub(crate) async fn node(name: &'static str, swaps: Vec<(String, Swap)>) -> Node {
    let scn = scenario(name);
    let closed: Value = serde_json::from_str(CLOSED_CYCLE).unwrap();
    let seen: Arc<std::sync::Mutex<Vec<(String, String)>>> = Arc::new(std::sync::Mutex::new(Vec::new()));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a local port");
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handler = {
        let seen = seen.clone();
        move |Json(req): Json<Value>| {
            let (seen, scn, closed, swaps) = (seen.clone(), scn.clone(), closed.clone(), swaps.clone());
            async move {
                let key = request_key(&req);
                let method = req["method"].as_str().unwrap_or_default().to_string();
                let block = if method == "eth_call" { req["params"][1].as_str().unwrap_or_default().to_string() } else { String::new() };
                seen.lock().unwrap().push((key.clone(), block));
                let header = |number: i64, time: i64| json!({ "result": { "number": format!("0x{number:x}"), "timestamp": format!("0x{time:x}") } });
                let answer: Value = match swaps.iter().find(|(k, _)| k == &key) {
                    Some((_, Swap::Drop)) => std::panic::resume_unwind(Box::new("dropped connection (test)")),
                    Some((_, Swap::Json(v))) => v.clone(),
                    None if method == "eth_getBlockByNumber" => {
                        let tag = req["params"][0].as_str().unwrap_or_default().to_lowercase();
                        if tag == "latest" {
                            header(scn["number"].as_i64().unwrap() + POSITION_HEAD_LAG_BLOCKS, scn["block_timestamp"].as_i64().unwrap() + 1)
                        } else {
                            // The time a recorded receipt's own logs state for that block.
                            let time = scn["responses"].as_object().unwrap().iter().filter(|(k, _)| k.starts_with("eth_getTransactionReceipt|")).find_map(|(_, r)| {
                                r["result"]["logs"].as_array()?.iter().find(|l| l["blockNumber"].as_str().map(str::to_lowercase).as_deref() == Some(tag.as_str()))?["blockTimestamp"]
                                    .as_str()
                                    .and_then(|t| i64::from_str_radix(t.trim_start_matches("0x"), 16).ok())
                            });
                            match time {
                                Some(t) => header(i64::from_str_radix(tag.trim_start_matches("0x"), 16).unwrap(), t),
                                None => json!({ "error": { "code": -32000, "message": format!("no recorded header for block {tag}") } }),
                            }
                        }
                    }
                    None => match scn["responses"].get(&key).or_else(|| closed["responses"].get(&key)) {
                        Some(recorded) => {
                            let mut v = recorded.clone();
                            v.as_object_mut().unwrap().remove("note");
                            v
                        }
                        None => json!({ "error": { "code": -32000, "message": format!("no recorded answer for {}", &key[..key.len().min(60)]) } }),
                    },
                };
                let status = answer.get("http_status").and_then(|s| s.as_u64()).unwrap_or(200) as u16;
                let mut body = json!({ "jsonrpc": "2.0", "id": req["id"] });
                for (k, v) in answer.as_object().unwrap() {
                    if k != "http_status" {
                        body[k] = v.clone();
                    }
                }
                (StatusCode::from_u16(status).unwrap(), Json(body)).into_response()
            }
        }
    };
    let app = axum::Router::new().route("/", axum::routing::post(handler));
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    forget_log_limits(&url);
    Node { url, seen }
}

/// The whole-history log request answered as an RPC without a range limit answers it.
pub(crate) fn wide_logs_swap(name: &str) -> (String, Swap) {
    let scn = scenario(name);
    (key_logs(scn["wallet"].as_str().unwrap(), 0, scan_block(name)), Swap::Json(json!({ "result": scn["wide_logs"] })))
}

// ---- configs: the tracked wallet's book (public chain data, as in tests_closed.rs) ----

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

pub(crate) fn config_for(wallet: &str, entries: &[&str]) -> AppConfig {
    let yaml = format!("alchemy_api_keys: [\"not-a-real-key\"]\ntracked_wallets: [\"{wallet}\"]\npositions:{}\n", entries.concat());
    serde_yaml::from_str(&yaml).expect("the test config parses")
}

/// The operator's book as config.yaml lists it: the v4 range is the price source.
pub(crate) fn book() -> AppConfig {
    config_for("0xeE114cC97eC5adeB34bA5645e315B497D8487a45", &[ENTRY_V4, ENTRY_V3, ENTRY_BASIC])
}

/// The same book with the v4 range NOT in config.yaml, as if the operator had deposited
/// it in the Uniswap app and never edited the file. The Aero basic pool supplies the spot.
pub(crate) fn book_without_v4() -> AppConfig {
    let basic = format!("{ENTRY_BASIC}\n    is_price_source: true");
    config_for("0xeE114cC97eC5adeB34bA5645e315B497D8487a45", &[ENTRY_V3, &basic])
}

/// A previous snapshot's `data` that suggests these rungs in the 3% pool, in the shape
/// `ladder_suggestion.rungs[]` has on main.
pub(crate) fn prev_suggesting(rungs: &[(i32, i32, Option<&str>)]) -> Value {
    let rungs: Vec<Value> = rungs
        .iter()
        .map(|(lo, hi, rung)| json!({ "rung": rung, "pool": { "protocol": "uniswap-v4", "pool_id": POOL_3PCT }, "tick_lower": lo, "tick_upper": hi }))
        .collect();
    json!({ "positions": [], "derived_metrics": { "ladder_suggestion": { "status": "ready", "rungs": rungs } } })
}

pub(crate) async fn detect(store: &PositionStore, n: &Node, cfg: &AppConfig, prev: Option<&Value>, now: DateTime<Utc>) -> DetectionOutcome {
    detect_positions(&DetectionInputs {
        store,
        http: &Client::new(),
        rpc_url: Some(&n.url),
        wallet: cfg.tracked_wallets.first().map(String::as_str),
        config_positions: &cfg.positions,
        prev,
        now,
        read_budget: None,
    })
    .await
}

/// No text may carry the RPC URL or the key.
fn assert_no_url(text: &str, node_url: &str) {
    assert!(!text.contains(node_url) && !text.contains("127.0.0.1") && !text.contains("not-a-real-key") && !text.contains("http://"), "{text}");
}

pub(crate) fn candidate(nft: u64) -> CandidateRow {
    CandidateRow {
        wallet: OPERATOR.to_string(),
        protocol: TRACKED_PROTOCOL_V4.to_string(),
        network: TRACKED_NETWORK_BASE.to_string(),
        nft_id: nft,
        verdict: "eligible".to_string(),
        choice: None,
        owned: true,
        has_liquidity: Some(true),
        pool_id: Some(POOL_3PCT.to_string()),
        pool_id_prefix: Some(POOL_3PCT[2..52].to_string()),
        currency0: Some(USDC.to_string()),
        currency1: Some(NOCK.to_string()),
        fee_raw: Some(30000),
        tick_spacing: Some(300),
        hooks: Some("0x0000000000000000000000000000000000000000".to_string()),
        tick_lower: Some(273900),
        tick_upper: Some(275400),
        received_tx: Some(format!("0x{}", "ab".repeat(32))),
        received_block: Some(60_240_576),
        received_from: Some("0x0000000000000000000000000000000000000000".to_string()),
        mint_tx: Some(format!("0x{}", "ab".repeat(32))),
        mint_block: Some(60_240_576),
        minted_by: Some(OPERATOR.to_string()),
        deposit_check: Some("ok".to_string()),
    }
}

fn rung(lo: i32, hi: i32, name: Option<&str>) -> SuggestedRung {
    SuggestedRung { protocol: TRACKED_PROTOCOL_V4.to_string(), pool_id: POOL_3PCT.to_string(), tick_lower: lo, tick_upper: hi, rung: name.map(str::to_string) }
}

// ------------------------------------------------------------------
// Pure: decoding, the whitelist, the windows, the decision
// ------------------------------------------------------------------

#[test]
fn test_the_recorded_pool_and_position_info_decodes_to_the_nfts_pool_and_ticks() {
    let hex = recorded("operator", &key_info(3163450))["result"].as_str().unwrap().to_string();
    let NftInfoRead::Info(i) = decode_pool_and_position_info(&hex).unwrap() else { panic!("an NFT that exists") };
    assert_eq!((i.currency0.as_str(), i.currency1.as_str()), (USDC, NOCK));
    assert_eq!((i.fee_raw, i.tick_spacing), (30000, 300));
    assert_eq!(i.hooks, "0x0000000000000000000000000000000000000000");
    assert_eq!((i.tick_lower, i.tick_upper), (262200, 282300));
    // The 25 bytes the position manager stores are the first 25 of the pool id.
    assert_eq!(i.pool_id_prefix, &POOL_3PCT[2..52]);
    assert_eq!(i.pool_id_prefix.len(), 50);

    // The wallet's empty NFT #3163431 in the 0.75% pool, also recorded.
    let hex = recorded("operator", &key_info(3163431))["result"].as_str().unwrap().to_string();
    let NftInfoRead::Info(i) = decode_pool_and_position_info(&hex).unwrap() else { panic!() };
    assert_eq!((i.fee_raw, i.tick_spacing, i.tick_lower, i.tick_upper), (7500, 75, 267825, 271050));
    assert!(POOL_075PCT[2..].starts_with(&i.pool_id_prefix));

    // Six zero words are "no such NFT"; anything else malformed is an error, never "gone".
    assert_eq!(decode_pool_and_position_info(&"0".repeat(384)), Ok(NftInfoRead::Gone));
    assert!(decode_pool_and_position_info("0x").is_err());
    assert!(decode_pool_and_position_info(&"0".repeat(320)).is_err());
    assert!(decode_pool_and_position_info(&hex[..hex.len() - 2]).is_err());
    // Negative ticks are sign-extended from 24 bits: the packed word of ticks -600..-300.
    let mut words = hex.trim_start_matches("0x").to_string();
    let packed = format!("{}{}{}00", &POOL_3PCT[2..52], "fffed4", "fffda8");
    words.replace_range(320..384, &packed);
    let NftInfoRead::Info(neg) = decode_pool_and_position_info(&words).unwrap() else { panic!() };
    assert_eq!((neg.tick_lower, neg.tick_upper), (-600, -300));
}

#[test]
fn test_the_whitelist_is_the_token_registry_and_dust_is_never_eligible() {
    let zero = "0x0000000000000000000000000000000000000000";
    assert_eq!(pool_key_verdict(USDC, NOCK, zero), "eligible");
    assert_eq!(pool_key_verdict(WETH, USDC, zero), "eligible");
    assert_eq!(pool_key_verdict(zero, NOCK, zero), "eligible", "native ETH / NOCK");
    // Case does not matter in an address.
    assert_eq!(pool_key_verdict(&USDC.to_uppercase().replace("0X", "0x"), NOCK, zero), "eligible");
    // AERO is on the whitelist and cannot be read as a position leg yet.
    assert_eq!(pool_key_verdict(AERO_BASE, USDC, zero), "unsupported");
    // A token outside it: dust, whatever the other token and whatever the hooks.
    let dust = "0xba0dda8762c24da9487f5fa026a9b64b695a07ea";
    assert_eq!(pool_key_verdict(USDC, dust, zero), "not_whitelisted");
    assert_eq!(pool_key_verdict(dust, NOCK, "0x00000000000000000000000000000000000000ff"), "not_whitelisted");
    // Whitelisted tokens in a hooked pool.
    assert_eq!(pool_key_verdict(USDC, NOCK, "0x00000000000000000000000000000000000000ff"), "hooks");

    // The wallet's 43 NFTs by their recorded pool keys: 6 eligible, 37 dust.
    let scn = scenario("operator");
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for l in scn["wide_logs"].as_array().unwrap() {
        let id = u64::from_str_radix(&l["topics"][3].as_str().unwrap()[50..], 16).unwrap();
        let NftInfoRead::Info(i) = decode_pool_and_position_info(recorded("operator", &key_info(id))["result"].as_str().unwrap()).unwrap() else { panic!() };
        *counts.entry(pool_key_verdict(&i.currency0, &i.currency1, &i.hooks)).or_insert(0) += 1;
    }
    assert_eq!(counts, HashMap::from([("eligible", 6), ("not_whitelisted", 37)]));
}

#[test]
fn test_transfer_logs_parse_strictly_and_a_malformed_answer_is_never_an_empty_list() {
    let scn = scenario("operator");
    let resp = json!({ "result": scn["wide_logs"] });
    let seen = parse_transfer_logs(&resp, OPERATOR).unwrap();
    assert_eq!(seen.len(), 43);
    assert!(seen.iter().all(|t| t.from == "0x0000000000000000000000000000000000000000"), "all 43 are mints");
    let newest = seen.iter().max_by_key(|t| t.block).unwrap();
    assert_eq!((newest.nft_id, newest.block, newest.tx_hash.as_str()), (3163450, 53008241, MINT_6CDA));
    assert_eq!(seen.iter().map(|t| t.nft_id).min(), Some(2149));
    // The wallet's address in mixed case is the same wallet.
    assert_eq!(parse_transfer_logs(&resp, "0xeE114cC97eC5adeB34bA5645e315B497D8487a45").unwrap().len(), 43);
    // The public node's own chunk answers.
    let chunk = recorded("operator", &key_logs(OPERATOR, 52976286, 53021285));
    let ids: Vec<u64> = parse_transfer_logs(&chunk, OPERATOR).unwrap().iter().map(|t| t.nft_id).collect();
    assert_eq!(ids, vec![3163431, 3163432, 3163450]);
    assert_eq!(parse_transfer_logs(&recorded("operator", &key_logs(OPERATOR, 58601286, 58646285)), OPERATOR).unwrap(), vec![]);

    // Not an answer: never read as "nothing found".
    assert!(parse_transfer_logs(&json!({}), OPERATOR).is_err());
    assert!(parse_transfer_logs(&json!({ "result": null }), OPERATOR).is_err());
    assert!(parse_transfer_logs(&json!({ "result": "0x" }), OPERATOR).is_err());
    let broken = |change: &dyn Fn(&mut Value)| {
        let mut log = scn["wide_logs"][0].clone();
        change(&mut log);
        parse_transfer_logs(&json!({ "result": [log] }), OPERATOR).unwrap_err()
    };
    assert_eq!(broken(&|l| l["address"] = json!(USDC)), "log 0 is not from the position manager");
    assert_eq!(broken(&|l| l["topics"] = json!(["0xddf252ad1be2c89b69c2b068fc378daa952ba7f163c4a11628f55a4df523b3ef"])), "log 0 is not an ERC-721 Transfer to the wallet");
    assert_eq!(broken(&|l| l["topics"][2] = json!(format!("0x{:0>64}", &MINTER[2..]))), "log 0 is not an ERC-721 Transfer to the wallet");
    assert_eq!(broken(&|l| l["topics"][3] = json!(format!("0x{}", "f".repeat(64)))), "log 0 has no usable token id");
    assert_eq!(broken(&|l| l["removed"] = json!(true)), "log 0 was removed by a chain reorganisation");
    assert_eq!(broken(&|l| { l.as_object_mut().unwrap().remove("blockNumber"); }), "log 0 has no block number");
    assert_eq!(broken(&|l| l["transactionHash"] = json!("0x12")), "log 0 has no transaction hash");
    // Another wallet's transfers are not this wallet's.
    assert!(parse_transfer_logs(&resp, MINTER).is_err());
}

/// One page of `alchemy_getAssetTransfers` (category erc721) ASSEMBLED from real logs in
/// the shape Alchemy documents and the design's explorer read on the deployed key: `hash`,
/// `blockNum`, `from`, `to`, `tokenId` (a 32-byte hex word), `erc721TokenId`,
/// `rawContract.address`. It is not a recorded page: recording one needs an Alchemy key.
fn assembled_history_page(logs: &[Value], page_key: Option<&str>) -> Value {
    let transfers: Vec<Value> = logs
        .iter()
        .map(|l| {
            json!({
                "blockNum": l["blockNumber"],
                "uniqueId": format!("{}:log:{}", l["transactionHash"].as_str().unwrap(), l["logIndex"].as_str().unwrap()),
                "hash": l["transactionHash"],
                "from": format!("0x{}", &l["topics"][1].as_str().unwrap()[26..]),
                "to": format!("0x{}", &l["topics"][2].as_str().unwrap()[26..]),
                "value": null,
                "erc721TokenId": l["topics"][3],
                "erc1155Metadata": null,
                "tokenId": l["topics"][3],
                "asset": "UNI-V4-POSM",
                "category": "erc721",
                "rawContract": { "value": null, "address": UNISWAP_V4_POSM_BASE, "decimal": null },
            })
        })
        .collect();
    let mut result = json!({ "transfers": transfers });
    if let Some(k) = page_key {
        result["pageKey"] = json!(k);
    }
    json!({ "result": result })
}

#[test]
fn test_the_transfer_index_page_parses_strictly() {
    let scn = scenario("operator");
    let logs = scn["wide_logs"].as_array().unwrap();
    let (seen, next) = parse_nft_history(&assembled_history_page(logs, None)).unwrap();
    assert_eq!((seen.len(), next), (43, None));
    assert_eq!(seen, parse_transfer_logs(&json!({ "result": logs }), OPERATOR).unwrap(), "the index and the logs name the same transfers");
    let (_, next) = parse_nft_history(&assembled_history_page(&logs[..2], Some("abc"))).unwrap();
    assert_eq!(next.as_deref(), Some("abc"));
    // The request names the position manager, the wallet and ERC-721 only.
    let req = nft_history_request(OPERATOR, Some("abc"));
    assert_eq!(req["method"], "alchemy_getAssetTransfers");
    assert_eq!(req["params"][0]["contractAddresses"], json!([UNISWAP_V4_POSM_BASE]));
    assert_eq!(req["params"][0]["category"], json!(["erc721"]));
    assert_eq!(req["params"][0]["toAddress"], json!(OPERATOR));
    assert_eq!(req["params"][0]["pageKey"], json!("abc"));
    // Malformed pages are errors, never empty lists.
    assert!(parse_nft_history(&json!({})).is_err());
    assert!(parse_nft_history(&json!({ "result": {} })).is_err());
    let broken = |change: &dyn Fn(&mut Value)| {
        let mut page = assembled_history_page(&logs[..1], None);
        change(&mut page["result"]["transfers"][0]);
        parse_nft_history(&page).unwrap_err()
    };
    assert_eq!(broken(&|t| t["rawContract"]["address"] = json!(USDC)), "transfer 0 is not of the position manager's NFT");
    assert_eq!(broken(&|t| { t["tokenId"] = json!(null); t["erc721TokenId"] = json!(null); }), "transfer 0 has no usable token id");
    assert_eq!(broken(&|t| t["hash"] = json!("0x")), "transfer 0 has no transaction hash");
    assert_eq!(broken(&|t| t["blockNum"] = json!("latest")), "transfer 0 has no block number");
    assert_eq!(broken(&|t| t["from"] = json!(null)), "transfer 0 has no sender");
    // The public node's real answer to the method is not a page.
    assert!(recorded("operator", "alchemy_getAssetTransfers").get("error").is_some());
}

#[test]
fn test_short_forms_never_cut_a_character_in_half() {
    assert_eq!(short_addr(OPERATOR), "0xee11…7a45");
    assert_eq!(short_addr("0x12"), "0x12");
    // A node could answer anything in a `from` field: it is printed whole, never sliced.
    assert_eq!(short_addr("ééééééééééééé"), "ééééééééééééé");
    assert_eq!(crate::position_detect_text::choice_words("ladder:near"), "TRACK AS NEAR RUNG");
    assert_eq!(crate::position_detect_text::choice_words("untrack"), "UNTRACK");
    assert_eq!(crate::position_detect_text::choice_words("else"), "ELSE");
}

#[test]
fn test_newest_transfer_wins_per_nft() {
    let t = |id: u64, block: i64, tx: &str| TransferSeen { nft_id: id, block, tx_hash: tx.to_string(), from: "0x0".to_string() };
    let mut seen = HashMap::new();
    absorb_transfers(&mut seen, vec![t(7, 100, "a"), t(8, 50, "b")]);
    absorb_transfers(&mut seen, vec![t(7, 90, "old"), t(8, 60, "new")]);
    assert_eq!((seen[&7].tx_hash.as_str(), seen[&8].tx_hash.as_str()), ("a", "new"));
}

#[test]
fn test_log_windows_go_forward_first_then_back_within_the_range_limit() {
    let c = POSITION_LOG_CHUNK_BLOCKS;
    let w = |from, to| Some(LogWindow { from, to });
    let head = 60_000_000;
    // Nothing read yet: the whole history in one request where the RPC allows it,
    // otherwise the newest chunk first.
    assert_eq!(next_log_window(None, head, true, 0), w(0, head));
    assert_eq!(next_log_window(None, head, false, 0), w(head - c + 1, head));
    assert_eq!(next_log_window(None, head, false, 0).unwrap().blocks(), c);
    // A chain younger than one chunk.
    assert_eq!(next_log_window(None, 1000, false, 0), w(0, 1000));
    // Forward from where the last cycle stopped, one chunk at most when limited.
    assert_eq!(next_log_window(Some((0, head - 1500)), head, true, 0), w(head - 1499, head));
    assert_eq!(next_log_window(Some((0, head - 1500)), head, false, 0), w(head - 1499, head));
    assert_eq!(next_log_window(Some((0, head - 3 * c)), head, false, 0), w(head - 3 * c + 1, head - 2 * c));
    assert_eq!(next_log_window(Some((0, head - 3 * c)), head, true, 0), w(head - 3 * c + 1, head));
    // Caught up: backward toward block 0.
    assert_eq!(next_log_window(Some((head - c + 1, head)), head, false, 0), w(head - 2 * c + 1, head - c));
    assert_eq!(next_log_window(Some((head - c + 1, head)), head, true, 0), w(0, head - c));
    assert_eq!(next_log_window(Some((100, head)), head, false, 0), w(0, 99));
    // Covered: nothing more. A node that reports an older head than was already read
    // (load-balanced nodes one block apart) asks for nothing forward.
    assert_eq!(next_log_window(Some((0, head)), head, false, 0), None);
    assert_eq!(next_log_window(Some((0, head + 3)), head, true, 0), None);
    // A floor the RPC refuses to go below ends the backfill there.
    assert_eq!(next_log_window(Some((52_000_000, head)), head, false, 52_000_000), None);
    assert_eq!(next_log_window(Some((52_010_000, head)), head, false, 52_000_000), w(52_000_000, 52_009_999));
    // Each window read extends the one covered range, never leaves a gap.
    let mut covered = None;
    for _ in 0..5 {
        let win = next_log_window(covered, head, false, 0).unwrap();
        if let Some((lo, hi)) = covered {
            assert!(win.to + 1 == lo || win.from == hi + 1, "{win:?} touches {covered:?}");
        }
        covered = Some(cover(covered, win));
    }
    assert_eq!(covered, Some((head - 5 * c + 1, head)));
    // The request names the position manager, the Transfer topic and the wallet as `to`.
    let req = transfer_logs_request(OPERATOR, 10, 20);
    assert_eq!(req["params"][0]["address"], json!(UNISWAP_V4_POSM_BASE));
    assert_eq!(req["params"][0]["topics"], json!([TRANSFER_EVENT_TOPIC, null, format!("0x{:0>64}", &OPERATOR[2..])]));
    assert_eq!((req["params"][0]["fromBlock"].as_str(), req["params"][0]["toBlock"].as_str()), (Some("0xa"), Some("0x14")));
}

#[test]
fn test_the_real_mint_receipts_state_who_minted_what_into_which_pool() {
    let receipt = recorded("operator", &key_receipt(MINT_6CDA))["result"].clone();
    let facts = mint_facts(&receipt, OPERATOR, 3163450).unwrap();
    assert_eq!(facts, MintFacts { block: 53008241, sender: OPERATOR.to_string(), minted_to_wallet: true, pool_id: Some(POOL_3PCT.to_string()) });
    // The same receipt read for another wallet or another NFT proves nothing about them.
    assert!(!mint_facts(&receipt, MINTER, 3163450).unwrap().minted_to_wallet);
    let other = mint_facts(&receipt, OPERATOR, 3163449).unwrap();
    assert_eq!((other.minted_to_wallet, other.pool_id), (false, None));
    // A failed transaction minted nothing.
    let mut failed = receipt.clone();
    failed["status"] = json!("0x0");
    assert_eq!(mint_facts(&failed, OPERATOR, 3163450).unwrap_err(), "the transaction failed (status 0x0)");

    // The #167 fixture: NFT #3240369, minted by the wallet that owns it.
    let afe4: Value = serde_json::from_str(MINT_AFE4).unwrap();
    let afe4 = afe4.get("result").cloned().unwrap_or(afe4);
    assert_eq!(afe4, recorded("minter", &key_receipt(MINT_AFE4_HASH))["result"], "the receipt recorded here is the #167 fixture's");
    let facts = mint_facts(&afe4, MINTER, 3240369).unwrap();
    assert_eq!((facts.minted_to_wallet, facts.sender.as_str(), facts.pool_id.as_deref(), facts.block), (true, MINTER, Some(POOL_3PCT), 59660587));
}

#[test]
fn test_suggested_rungs_are_read_from_the_suggestion_or_from_the_configured_shape() {
    // The #168 suggestion as the worker's golden file states it: one rung, no rung name.
    let golden: Value = serde_json::from_str(include_str!("../testdata/ladder_suggest/golden_ladder_suggest.json")).unwrap();
    let data = json!({ "positions": [], "derived_metrics": { "ladder_suggestion": golden["ready"]["ladder_suggestion"] } });
    assert_eq!(suggested_rungs(&data), vec![rung(273900, 276300, None)]);
    // A suggestion that shows no ladder suggests no rung.
    let none = json!({ "derived_metrics": { "ladder_suggestion": golden["tracked"]["ladder_suggestion"] } });
    assert_eq!(suggested_rungs(&none), vec![]);
    assert_eq!(suggested_rungs(&json!({})), vec![]);

    // The configured shape of a later release (`ladder_shape`): its pool is the position
    // it names, and its rungs carry cap | near | deep. Only names this release knows pass.
    let shape = json!({
        "positions": [{ "position_id": "uniswap-v4:base:3163450", "protocol": "uniswap-v4", "pool_id": POOL_3PCT.to_uppercase().replace("0X", "0x") }],
        "derived_metrics": { "ladder_shape": {
            "pool": { "position_id": "uniswap-v4:base:3163450" },
            "rungs": [
                { "rung": "cap", "tick_lower": 273600, "tick_upper": 273900 },
                { "rung": "near", "tick_lower": 273900, "tick_upper": 275400 },
                { "rung": "deep", "tick_lower": 275400, "tick_upper": 276300 },
                { "rung": "deep", "tick_lower": null, "tick_upper": 276300 },
                { "rung": "near", "tick_lower": 275400, "tick_upper": 275400 },
            ],
        } },
    });
    let rungs = suggested_rungs(&shape);
    let known = |name: &str| LADDER_RUNGS.contains(&name).then(|| name.to_string());
    assert_eq!(
        rungs,
        vec![
            SuggestedRung { rung: known("cap"), ..rung(273600, 273900, None) },
            rung(273900, 275400, Some("near")),
            rung(275400, 276300, Some("deep")),
        ],
        "a rung without both ticks, or with an empty range, is left out; a rung name this release does not know reads as unset"
    );
    // Both present: each range once, the suggestion's first.
    let mut both = shape.clone();
    both["derived_metrics"]["ladder_suggestion"] = json!({ "rungs": [{ "rung": null, "pool": { "protocol": "uniswap-v4", "pool_id": POOL_3PCT }, "tick_lower": 273900, "tick_upper": 275400 }] });
    let rungs = suggested_rungs(&both);
    assert_eq!(rungs.len(), 3);
    assert_eq!(rungs[0], rung(273900, 275400, None));
    // A shape whose pool position is not in the snapshot names no pool: no rung.
    let mut orphan = shape.clone();
    orphan["positions"] = json!([]);
    assert_eq!(suggested_rungs(&orphan), vec![]);
}

#[test]
fn test_a_position_is_tracked_by_itself_only_when_every_condition_holds() {
    let near = [rung(273900, 275400, Some("near")), rung(275400, 276300, Some("deep"))];
    let c = candidate(3243807);
    // The exact rung, minted by the wallet, a plain deposit, nothing tracked there yet.
    assert_eq!(decide(&c, OPERATOR, &near, &[]), Decision::Track { rung: Some("near".to_string()) });
    let deep = CandidateRow { tick_lower: Some(275400), tick_upper: Some(276300), ..c.clone() };
    assert_eq!(decide(&deep, OPERATOR, &near, &[]), Decision::Track { rung: Some("deep".to_string()) });
    // The single #168 rung names no rung: tracked with the rung unset, never a guess.
    assert_eq!(decide(&c, OPERATOR, &[rung(273900, 275400, None)], &[]), Decision::Track { rung: None });
    // The wallet's address in another case is the same wallet.
    assert_eq!(decide(&c, "0xeE114cC97eC5adeB34bA5645e315B497D8487a45", &near, &[]), Decision::Track { rung: Some("near".to_string()) });

    let all = vec!["ladder:cap".to_string(), "ladder:near".to_string(), "ladder:deep".to_string(), "lp".to_string(), "dismiss".to_string()];
    let offered = |c: &CandidateRow, suggested: &[SuggestedRung], tracked: &[TrackedRange]| match decide(c, OPERATOR, suggested, tracked) {
        Decision::Offer { why_not_automatic, choices } => (why_not_automatic, choices),
        other => panic!("expected an offer, got {other:?}"),
    };
    let cases: Vec<(&str, CandidateRow, Vec<SuggestedRung>, Vec<TrackedRange>, &str)> = vec![
        // One band off at the top is the near rung's drift rule (asserted below); two is not.
        ("two tick spacings off at the top", CandidateRow { tick_lower: Some(273300), ..c.clone() }, near.to_vec(), vec![], "its ticks 273300..275400 are not a suggested rung's (273900..275400, 275400..276300)"),
        ("one tick spacing off at the bottom", CandidateRow { tick_upper: Some(275700), ..c.clone() }, near.to_vec(), vec![], "its ticks 273900..275700 are not a suggested rung's (273900..275400, 275400..276300)"),
        ("the same ticks in another pool", CandidateRow { pool_id: Some(POOL_075PCT.to_string()), ..c.clone() }, near.to_vec(), vec![], "it is not in the ladder suggestion's pool"),
        ("nothing is suggested", c.clone(), vec![], vec![], "no ladder rung is suggested now to match it against"),
        ("a stranger minted it to the wallet", CandidateRow { minted_by: Some(MINTER.to_string()), ..c.clone() }, near.to_vec(), vec![], "its mint was sent by 0x4598…e826, not by your wallet"),
        (
            "it arrived by transfer",
            CandidateRow { received_from: Some(MINTER.to_string()), mint_tx: None, mint_block: None, minted_by: None, deposit_check: None, ..c.clone() },
            near.to_vec(),
            vec![],
            "it reached the wallet by transfer from 0x4598…e826, it was not minted to it",
        ),
        ("its mint was not read yet", CandidateRow { mint_tx: None, mint_block: None, minted_by: None, deposit_check: None, ..c.clone() }, near.to_vec(), vec![], "its mint transaction has not been read yet"),
        (
            "its mint also swaps",
            CandidateRow { deposit_check: Some("the transaction also swaps, so its token flows are not the deposit alone".to_string()), ..c.clone() },
            near.to_vec(),
            vec![],
            "its mint is not a plain deposit into it: the transaction also swaps, so its token flows are not the deposit alone",
        ),
        (
            "that range is already a tracked ladder",
            c.clone(),
            near.to_vec(),
            vec![TrackedRange { position_id: "uniswap-v4:base:3243000".to_string(), pool_id: POOL_3PCT.to_string(), tick_lower: 273900, tick_upper: 275400 }],
            "that range is already tracked as uniswap-v4:base:3243000",
        ),
    ];
    // The near rung's top follows the price until it is deposited: the wallet's own mint
    // with the rung's bottom tick and its top one band away is that rung (second review of
    // the bottle), every other condition unchanged.
    for top in [273600, 274200] {
        let drifted = CandidateRow { tick_lower: Some(top), ..c.clone() };
        assert_eq!(decide(&drifted, OPERATOR, &near, &[]), Decision::Track { rung: near[0].rung.clone() }, "{top}");
        let stranger = CandidateRow { minted_by: Some(MINTER.to_string()), ..drifted.clone() };
        assert!(matches!(decide(&stranger, OPERATOR, &near, &[]), Decision::Offer { .. }), "{top}: a stranger's mint never");
    }
    for (name, row, suggested, tracked, expect) in cases {
        let (why, choices) = offered(&row, &suggested, &tracked);
        assert_eq!(why, vec![expect.to_string()], "{name}");
        assert_eq!(choices, all, "{name}: a USDC/NOCK range with a known pool has every choice");
    }
    // A receipt that was read and is not a mint of it: one reason, the receipt's.
    let no_mint = CandidateRow { mint_tx: None, mint_block: None, minted_by: None, deposit_check: Some("the transaction that brought it to the wallet does not mint it to the wallet".to_string()), ..c.clone() };
    assert_eq!(offered(&no_mint, &near, &[]).0, vec!["its mint is not a plain deposit into it: the transaction that brought it to the wallet does not mint it to the wallet".to_string()]);
    // Several failed conditions are all stated.
    let stranger_off = CandidateRow { minted_by: Some(MINTER.to_string()), tick_lower: Some(273300), ..c.clone() };
    assert_eq!(offered(&stranger_off, &near, &[]).0.len(), 2);
    // A tracked ladder in another range does not stand in the way.
    let elsewhere = [TrackedRange { position_id: "x".to_string(), pool_id: POOL_3PCT.to_string(), tick_lower: 275400, tick_upper: 276300 }];
    assert_eq!(decide(&c, OPERATOR, &near, &elsewhere), Decision::Track { rung: Some("near".to_string()) });

    // A pool that could not be identified: nothing can be read from it, so only NOT NOW.
    let (why, choices) = offered(&CandidateRow { pool_id: None, ..c.clone() }, &near, &[]);
    assert!(why[0].starts_with("its pool could not be identified yet"), "{why:?}");
    assert_eq!(choices, vec!["dismiss".to_string()]);
    // A WETH/USDC range is never a ladder: LP or not now.
    let weth = CandidateRow { currency0: Some(WETH.to_string()), currency1: Some(USDC.to_string()), ..c.clone() };
    assert_eq!(track_choices(&weth), vec!["lp".to_string(), "dismiss".to_string()]);
    assert!(!can_be_ladder(&weth));
    // A full-range USDC/NOCK position is not a ladder either.
    let full = CandidateRow { tick_lower: Some(-887100), tick_upper: Some(887100), ..c.clone() };
    assert_eq!(track_choices(&full), vec!["lp".to_string(), "dismiss".to_string()]);
    // One bound at the pool's usable extreme is open-ended, not full range: the #167
    // fixture's NFT (270900..887100) can be a ladder.
    assert!(can_be_ladder(&CandidateRow { tick_lower: Some(270900), tick_upper: Some(887100), ..c.clone() }));
}

#[test]
fn test_the_simulated_rung_mints_are_plain_deposits_that_match_their_rungs() {
    // SIMULATED receipts (eth_simulateV1, nothing mined): the near and deep rung mints
    // the operator would send, with the logs the chain would emit. The receipt fields
    // the simulation does not have (status, sender, block, hash) are filled in from the
    // simulation's own inputs.
    let sim: Value = serde_json::from_str(SIMULATED).unwrap();
    assert!(sim["note"].as_str().unwrap().starts_with("SIMULATED, not mined"));
    let shape = [rung(273900, 275400, Some("near")), rung(275400, 276300, Some("deep"))];
    for (name, nft) in [("near", 3243807u64), ("deep", 3243808u64)] {
        let step = &sim["steps"][name];
        assert_eq!(step["sim_status"], "0x1");
        let tx = format!("0x{}", if name == "near" { "11" } else { "22" }.repeat(32));
        let receipt = json!({
            "status": step["sim_status"],
            "from": OPERATOR,
            "blockNumber": format!("0x{:x}", sim["block"].as_i64().unwrap()),
            "transactionHash": tx,
            "logs": step["sim_logs"],
        });
        let facts = mint_facts(&receipt, OPERATOR, nft).unwrap();
        assert_eq!((facts.minted_to_wallet, facts.sender.as_str(), facts.pool_id.as_deref()), (true, OPERATOR, Some(POOL_3PCT)), "{name}");
        let (lo, hi) = (step["tick_lower"].as_i64().unwrap() as i32, step["tick_upper"].as_i64().unwrap() as i32);
        let mut row = CandidateRow { nft_id: nft, tick_lower: Some(lo), tick_upper: Some(hi), mint_tx: Some(tx.clone()), received_tx: Some(tx), deposit_check: None, ..candidate(nft) };
        // The #167 verifier, unchanged, accepts it for this rung's position...
        let p = candidate_raw_position(&row, POOL_3PCT).unwrap();
        let deposit = parse_ladder_deposit(&receipt, OPERATOR, &p).unwrap();
        assert_eq!(deposit.amount0_raw.to_string(), step["usdc_raw"].as_str().unwrap(), "{name}: exactly the rung's USDC");
        assert_eq!(deposit.amount1_raw, 0, "{name}: a rung takes no NOCK");
        assert_eq!(deposit.liquidity.to_string(), step["liquidity"].as_str().unwrap());
        // ...and refuses it for the other rung's ticks.
        let other = if name == "near" { (275400, 276300) } else { (273900, 275400) };
        let wrong = candidate_raw_position(&CandidateRow { tick_lower: Some(other.0), tick_upper: Some(other.1), ..row.clone() }, POOL_3PCT).unwrap();
        assert!(parse_ladder_deposit(&receipt, OPERATOR, &wrong).unwrap_err().contains("not this position's"));
        row.deposit_check = Some("ok".to_string());
        assert_eq!(decide(&row, OPERATOR, &shape, &[]), Decision::Track { rung: Some(name.to_string()) });
        let tracked = tracked_row_for(&row, Some(LADDER_ROLE), Some(name), TRACKED_SOURCE_DETECTED).unwrap();
        assert_eq!(validate_tracked_row(&tracked), Ok(()));
        assert_eq!(tracked.deposit_tx, row.mint_tx, "the mint is the #167 baseline: the operator supplies no hash");
        let cfg = tracked_position_config(&tracked);
        assert_eq!((cfg.role.as_deref(), cfg.rung.as_deref(), cfg.deposit_tx), (Some("ladder"), Some(name), row.mint_tx.clone()));
    }
}

#[test]
fn test_the_status_says_exactly_what_is_missing() {
    let head = 60_000_000;
    let logs = |covered: Option<(i64, i64)>, status: &str, reason: Option<&str>| LogsReport { status: status.to_string(), covered, calls: 1, reason: reason.map(str::to_string), refused_below: None };
    let history = |status: &str, reason: Option<&str>| HistoryReport { status: status.to_string(), pages: 1, reason: reason.map(str::to_string) };
    let run = |l: LogsReport, h: HistoryReport| ScanRun { block: Some(head), logs: l, history: h, held: Some(43), known_owned: 43, ..Default::default() };

    // Everything read: ok, no reason, and the coverage is stated.
    let (status, reason, scan) = detection_status(&run(logs(Some((0, head)), "used", None), history("not_used", None)));
    assert_eq!((status, reason), ("ok", None));
    assert_eq!(scan["coverage"], "complete");
    assert_eq!(scan["logs_from_block"], 0);
    assert_eq!(scan["logs_behind_blocks"], 0);
    assert_eq!(scan["text"], "Every Uniswap v4 position NFT sent to the wallet was read from the transfer logs, from the first block to this cycle's block.");

    // Logs limited to a recent window, the index read to its end: ok, and it says which.
    let (status, reason, scan) = detection_status(&run(logs(Some((head - 45_000, head)), "used", None), history("complete", None)));
    assert_eq!((status, reason), ("ok", None));
    assert_eq!(scan["coverage"], "complete");
    assert!(scan["text"].as_str().unwrap().contains("from the RPC's transfer index, which can trail the chain by a short time. The transfer logs, which have no delay, cover block 59955000 onward."), "{scan}");

    // The backfill is not finished and there is no index: partial, naming the block.
    let (status, reason, scan) = detection_status(&run(logs(Some((59_000_000, head)), "used", None), history("failed", Some("the RPC answered with error -32601: Method not found"))));
    assert_eq!(status, "partial");
    assert_eq!(scan["coverage"], "partial");
    assert_eq!(scan["logs_from_block"], 59_000_000);
    let reason = reason.unwrap();
    assert!(reason.contains("transfers before block 59000000 are not read yet (the backfill reads up to 540000 blocks more each cycle; transfer index: the RPC answered with error -32601: Method not found)"), "{reason}");
    assert!(scan["text"].as_str().unwrap().contains("A position that reached the wallet before block 59000000 is not found yet"));

    // The RPC refuses older blocks: the same gap, with the RPC's own reason.
    let mut refused = logs(Some((52_931_286, head)), "used", None);
    refused.refused_below = Some((52_931_286, "the RPC answered with error -32602: Archive requests require a personal token".to_string()));
    let (status, _, scan) = detection_status(&run(refused, history("failed", Some("Method not found"))));
    assert_eq!(status, "partial");
    assert!(scan["text"].as_str().unwrap().contains("this RPC does not serve older blocks (the RPC answered with error -32602: Archive requests require a personal token)"), "{scan}");

    // Each other gap is named, and several are all named.
    let complete = || run(logs(Some((0, head)), "used", None), history("not_used", None));
    let gap = |change: &dyn Fn(&mut ScanRun)| {
        let mut r = complete();
        change(&mut r);
        let (status, reason, _) = detection_status(&r);
        assert_eq!(status, "partial");
        reason.unwrap()
    };
    assert_eq!(gap(&|r| r.unread = 2), "2 position NFT(s) could not be read and will be tried again next cycle");
    assert_eq!(gap(&|r| r.deferred = 5), "5 more position NFT(s) are still to be read (a per-cycle limit)");
    assert_eq!(gap(&|r| r.held = Some(44)), "the wallet holds 44 Uniswap v4 position NFTs on-chain and 43 are known: 1 not found yet");
    assert_eq!(gap(&|r| r.held = None), "the wallet's NFT count on the position manager could not be read, so the scan was not cross-checked");
    assert_eq!(gap(&|r| { r.unread = 1; r.held = Some(45); }).matches("; ").count(), 1);
    // Fewer held than known is not a gap: an NFT left the wallet (its owner is re-read).
    let mut fewer = complete();
    fewer.held = Some(42);
    assert_eq!(detection_status(&fewer).0, "ok");

    // This cycle read no new transfers at all: unavailable, with how far it had read.
    let (status, reason, scan) = detection_status(&run(logs(Some((0, head - 1500)), "failed", Some("the request timed out")), history("failed", Some("the request timed out"))));
    assert_eq!(status, "unavailable");
    assert_eq!(
        reason.unwrap(),
        "New positions could not be checked this cycle (transfer logs: the request timed out; transfer index: the request timed out): a deposit made since the last check would not be counted yet. The transfer logs were last read through block 59998500."
    );
    assert_eq!((scan["coverage"].as_str(), scan["logs_behind_blocks"].as_i64()), (Some("none"), Some(1500)));
    // Nothing was ever read.
    let (status, reason, _) = detection_status(&run(logs(None, "refused", Some("range too wide")), history("failed", Some("Method not found"))));
    assert_eq!(status, "unavailable");
    assert!(reason.unwrap().ends_with("No transfers have been read yet."));
    // The scan stopped before any read.
    let stopped = ScanRun { failed: Some("the node's latest block could not be read".to_string()), previously_checked_at: Some(Utc.timestamp_opt(1_795_000_000, 0).unwrap()), ..Default::default() };
    let (status, reason, scan) = detection_status(&stopped);
    assert_eq!(status, "unavailable");
    assert_eq!(
        reason.unwrap(),
        "New positions could not be checked this cycle (the node's latest block could not be read): a deposit made since the last check would not be counted yet. The transfers were last read at 2026-11-18T11:06:40+00:00."
    );
    assert_eq!(scan["coverage"], "none");
}

// ------------------------------------------------------------------
// One cycle against the recorded chain
// ------------------------------------------------------------------

/// A store whose `tracked_at` is a fixed time, so a block built from it is reproducible.
pub(crate) fn store_at(now: DateTime<Utc>) -> PositionStore {
    let store = PositionStore::mem();
    if let PositionStore::Mem(m) = &store {
        m.lock().unwrap().now = Some(now);
    }
    store
}

/// The rest of the cycle after detection: the database rows merged, the positions read.
pub(crate) async fn read_cycle(store: &PositionStore, n: &Node, cfg: &AppConfig) -> Result<RawOnChainData> {
    let http = Client::new();
    let merged = load_cycle_positions(store, &http, &n.url, cfg).await?;
    fetch_raw_data_merged(&http, &n.url, cfg, &merged, clock("operator")).await
}

#[tokio::test]
async fn test_the_wallets_whole_history_is_one_request_and_a_steady_cycle_is_a_handful() {
    let n = node("operator", vec![wide_logs_swap("operator")]).await;
    let store = store_at(clock("operator"));
    let out = detect(&store, &n, &book(), None, clock("operator")).await;
    assert_eq!((out.status.as_str(), out.reason.clone()), ("ok", None));
    assert_eq!(out.scan["coverage"], "complete");
    assert_eq!(out.scan["sources"], json!({ "transfer_logs": "used", "transfer_index": "not_used" }));
    assert_eq!((out.run.held, out.run.known_owned), (Some(43), 43), "balanceOf on the position manager agrees with the NFTs found");
    // 37 are dust, 5 are empty, and the one that holds liquidity is config.yaml's own.
    assert_eq!(out.ignored, BTreeMap::from([("empty".to_string(), 5), ("in_config".to_string(), 1), ("not_whitelisted".to_string(), 37)]));
    assert!(out.found.is_empty() && out.newly_tracked.is_empty());
    assert_eq!(out.last_checked_at, Some(clock("operator")));

    // What the first cycle after a database loss costs for this wallet: one header, ONE
    // log request for the whole history, two reads per NFT (43), the liquidity of the six
    // in whitelisted pools, the one mint receipt, and the wallet's NFT count.
    assert_eq!(n.count("eth_getBlockByNumber"), 1);
    assert_eq!(n.count("eth_getLogs"), 1);
    assert_eq!(n.count("alchemy_getAssetTransfers"), 0, "the index is not read when the logs cover the history");
    assert_eq!(n.count("eth_getTransactionReceipt"), 1);
    assert_eq!(n.count("eth_call"), 43 * 2 + 6 + 1);
    assert_eq!(n.total(), 96);
    // Every chain read of the scan names ONE block: the recorded one.
    assert_eq!(n.call_blocks(UNISWAP_V4_POSM_BASE), HashSet::from([scenario("operator")["block_hex"].as_str().unwrap().to_string()]));
    // The cursor is stored, so the next cycle reads only what is new.
    let state = store.scan_state(OPERATOR, TRACKED_PROTOCOL_V4, TRACKED_NETWORK_BASE).await.unwrap();
    assert_eq!((state.logs_from_block, state.logs_to_block), (Some(0), Some(58646285)));
    assert_eq!(state.last_checked_at, Some(clock("operator")));
    assert_eq!(store.candidates(OPERATOR).await.unwrap().len(), 43);
    assert!(store.tracked(OPERATOR).await.unwrap().is_empty(), "nothing is tracked without a match or a tap");

    // A steady cycle: the header, the wallet's NFT count, and the liquidity of the five
    // empty whitelisted NFTs (one can be refilled with no transfer). On a moving chain
    // one log request for the new blocks comes on top; here the head has not moved.
    n.forget();
    let again = detect(&store, &n, &book(), None, clock("operator")).await;
    assert_eq!(again.status, "ok");
    assert_eq!((n.count("eth_getBlockByNumber"), n.count("eth_getLogs"), n.count("eth_getTransactionReceipt"), n.count("eth_call")), (1, 0, 0, 6));
    assert_eq!(n.total(), 7);
    assert_eq!(again.ignored, out.ignored);
}

#[tokio::test]
async fn test_a_position_the_wallet_holds_is_found_valued_and_stated_as_not_counted() {
    // The wallet's v4 range is NOT in config.yaml here: found on-chain, and nothing is
    // suggested to match it against, so it is offered, never tracked silently.
    let n = node("operator", vec![wide_logs_swap("operator")]).await;
    let store = store_at(clock("operator"));
    let cfg = book_without_v4();
    let out = detect(&store, &n, &cfg, None, clock("operator")).await;
    assert_eq!(out.status, "ok");
    assert_eq!(out.ignored, BTreeMap::from([("empty".to_string(), 5), ("not_whitelisted".to_string(), 37)]));
    assert_eq!(out.found.len(), 1);
    let f = &out.found[0];
    assert_eq!((f.row.nft_id, f.dismissed), (3163450, false));
    assert_eq!(f.why_not_automatic, vec!["no ladder rung is suggested now to match it against".to_string()]);
    assert_eq!(f.choices, vec!["ladder:cap", "ladder:near", "ladder:deep", "lp", "dismiss"]);
    // Every fact was read on-chain: its pool key and ticks, its owner, its liquidity,
    // and its mint receipt (sent by the wallet; the #167 verifier accepts it unchanged).
    assert_eq!(f.row.pool_id.as_deref(), Some(POOL_3PCT));
    assert_eq!((f.row.tick_lower, f.row.tick_upper), (Some(262200), Some(282300)));
    assert_eq!((f.row.mint_tx.as_deref(), f.row.mint_block, f.row.minted_by.as_deref()), (Some(MINT_6CDA), Some(53008241), Some(OPERATOR)));
    assert_eq!(f.row.deposit_check.as_deref(), Some("ok"));
    assert!(store.tracked(OPERATOR).await.unwrap().is_empty());
    // While it waits for a tap, each cycle reads it again with three calls on top of the
    // steady seven: its liquidity, its owner, and its pool's price. Its pool key and
    // ticks never change and are not asked for again, and neither is its receipt.
    n.forget();
    let again = detect(&store, &n, &cfg, None, clock("operator")).await;
    assert_eq!((n.total(), n.count("eth_getLogs"), n.count("eth_getTransactionReceipt")), (10, 0, 0));
    assert_eq!((n.count(&key_info(3163450)), n.count(&key_owner(3163450)), n.count(&key_liquidity(3163450))), (0, 1, 1));
    assert_eq!(again.found, out.found);

    // The cycle does not read it: it is in no position list and no total.
    let raw = read_cycle(&store, &n, &cfg).await.unwrap();
    assert!(raw.positions.iter().all(|p| p.nft_id != 3163450));
    assert!(raw.tracked_report.is_empty() && raw.position_sources.is_empty());
    // And the snapshot says so, with what it holds at this cycle's prices.
    let block = position_detection_block(&out, &raw);
    assert_eq!(block["status"], "ok");
    assert_eq!(block["checked_at_block"], 58646285);
    assert_eq!(block["onchain_count"], json!({ "held": 43, "known": 43 }));
    assert_eq!(block["found_not_counted"]["count"], 1);
    assert_eq!(block["found_not_counted"]["value_usd"], "240.61");
    let p = &block["found_not_counted"]["positions"][0];
    assert_eq!(p["position_id"], "uniswap-v4:base:3163450");
    assert_eq!(p["pool_label"], "v4 USDC/NOCK 3.00%");
    assert_eq!(p["holds"], json!({ "liquidity": "249864729574523", "token0": { "symbol": "USDC", "amount": "120.02" }, "token1": { "symbol": "NOCK", "amount": "8114.3626" } }));
    assert_eq!((p["value_usd"].as_str(), p["value_note"].is_null(), p["in_range"].as_bool()), (Some("240.61"), true, Some(true)));
    assert_eq!(p["bounds_usd"], json!({ "low": "0.00550148", "high": "0.04105518" }));
    assert_eq!(p["mint_tx"], MINT_6CDA);
    assert_eq!(block["dismissed"], json!({ "count": 0, "value_usd": null, "value_note": null, "positions": [] }));
    assert!(block["tracked"].as_array().unwrap().is_empty());
    // The value is the same arithmetic a tracked position gets: read as a ladder next.
    let prev = prev_suggesting(&[(262200, 282300, None)]);
    detect(&store, &n, &cfg, Some(&prev), clock("operator")).await;
    let tracked = read_cycle(&store, &n, &cfg).await.unwrap();
    let position = snapshot_positions(&tracked).into_iter().find(|p| p.nft_id == 3163450).unwrap();
    assert_eq!(position.amount_usd, "240.61", "found-not-counted states the value the position has once it is counted");

    // NOT NOW sets it aside: still stated, with its value, under its own heading.
    let n2 = node("operator", vec![wide_logs_swap("operator")]).await;
    let store2 = store_at(clock("operator"));
    detect(&store2, &n2, &cfg, None, clock("operator")).await;
    assert_eq!(set_found_choice(&store2, OPERATOR, 3163450, true).await.unwrap()["status"], "dismissed");
    // Even an exact match is never tracked against a dismissal.
    let out = detect(&store2, &n2, &cfg, Some(&prev), clock("operator")).await;
    assert!(out.newly_tracked.is_empty() && store2.tracked(OPERATOR).await.unwrap().is_empty());
    assert_eq!((out.found.len(), out.found[0].dismissed), (1, true));
    assert_eq!(out.found[0].why_not_automatic, vec!["you set it aside (NOT NOW or UNTRACK)".to_string()]);
    let block = position_detection_block(&out, &read_cycle(&store2, &n2, &cfg).await.unwrap());
    assert_eq!((block["found_not_counted"]["count"].as_u64(), block["dismissed"]["count"].as_u64()), (Some(0), Some(1)));
    assert_eq!(block["dismissed"]["value_usd"], "240.61");
    // RESTORE puts it back, and the next cycle tracks the exact match.
    assert_eq!(set_found_choice(&store2, OPERATOR, 3163450, false).await.unwrap()["status"], "offered");
    let out = detect(&store2, &n2, &cfg, Some(&prev), clock("operator")).await;
    assert_eq!(out.newly_tracked, vec!["uniswap-v4:base:3163450".to_string()]);
}

#[tokio::test]
async fn test_an_exact_rung_match_is_tracked_with_its_mint_as_the_baseline_in_the_same_cycle() {
    let n = node("operator", vec![wide_logs_swap("operator")]).await;
    let store = store_at(clock("operator"));
    let cfg = book_without_v4();
    // One tick spacing off the suggested rung: offered, with what differs.
    let near_miss = prev_suggesting(&[(262200, 282000, None)]);
    let out = detect(&store, &n, &cfg, Some(&near_miss), clock("operator")).await;
    assert!(out.newly_tracked.is_empty());
    assert_eq!(out.found[0].why_not_automatic, vec!["its ticks 262200..282300 are not a suggested rung's (262200..282000)".to_string()]);

    // The suggestion's pool and ticks exactly: tracked by itself.
    let prev = prev_suggesting(&[(262200, 282300, None)]);
    let out = detect(&store, &n, &cfg, Some(&prev), clock("operator")).await;
    assert_eq!(out.newly_tracked, vec!["uniswap-v4:base:3163450".to_string()]);
    assert!(out.found.is_empty(), "a tracked position is not also listed as not counted");
    let rows = store.tracked(OPERATOR).await.unwrap();
    assert_eq!(
        rows,
        vec![TrackedRow {
            wallet: OPERATOR.to_string(),
            position_key: "uniswap-v4:base:3163450".to_string(),
            protocol: "uniswap-v4".to_string(),
            network: "base".to_string(),
            nft_id: 3163450,
            pool_id: POOL_3PCT.to_string(),
            currency0: USDC.to_string(),
            currency1: NOCK.to_string(),
            tick_lower: 262200,
            tick_upper: 282300,
            tick_spacing: Some(300),
            role: Some("ladder".to_string()),
            // The #168 suggestion names no rung: unset, never a guessed "near".
            rung: None,
            // The mint detection found: the operator supplied no hash.
            deposit_tx: Some(MINT_6CDA.to_string()),
            mint_block: Some(53008241),
            source: "detected".to_string(),
            tracked_at: Some(clock("operator")),
            // Recorded by the cycle after its position reads, not by detection.
            deposit_liquidity: None,
            max_liquidity_seen: None,
        }]
    );

    // The same cycle's reads count it: detection ran before them.
    let raw = read_cycle(&store, &n, &cfg).await.unwrap();
    assert_eq!(raw.position_sources, HashMap::from([("uniswap-v4:base:3163450".to_string(), "tracked".to_string())]));
    let positions = snapshot_positions(&raw);
    let p = positions.iter().find(|p| p.nft_id == 3163450).unwrap();
    assert_eq!((p.source.as_deref(), p.role.as_deref(), p.rung.as_deref(), p.is_price_source), (Some("tracked"), Some("ladder"), None, false));
    assert!(positions.iter().filter(|p| p.nft_id != 3163450).all(|p| p.source.is_none()), "a config entry carries no source");
    // It is a ladder: measured, never NOCK LP, never a rebuild template.
    assert!(is_ladder(&raw, raw.positions.iter().find(|p| p.nft_id == 3163450).unwrap()));
    assert!(raw.rebuild_rules["uniswap-v4:base:3163450"].deposit_url.is_none());
    // Its #167 reading uses the mint as the baseline, checked by parse_ladder_deposit
    // as it is. This NFT was added to after its mint, which the reading says.
    let ladders = ladder_readings(&raw);
    assert_eq!(ladders.len(), 1);
    let l = &ladders[0];
    assert_eq!((l.source.as_deref(), l.config_id.as_str(), l.state.as_str()), (Some("tracked"), "tracked-v4-3163450", "in_range"));
    assert_eq!(l.rung_note.as_deref(), Some(RUNG_UNSET_NOTE_TRACKED));
    assert!(!l.rung_note.as_deref().unwrap().contains("config.yaml"));
    let sd = &l.since_deposit;
    assert_eq!((sd.status.as_str(), sd.deposit_tx.as_deref(), sd.deposit_block), ("liquidity_changed", Some(MINT_6CDA), Some(53008241)));
    assert_eq!((sd.deposited_usdc.as_deref(), sd.deposited_nock.as_deref(), sd.deposit_liquidity.as_deref()), (Some("33.60"), Some("976.1141"), Some("48295502692597")));
    assert_eq!(sd.deposited_at.as_deref(), Some("2026-10-24T01:37:09+00:00"));

    let block = position_detection_block(&out, &raw);
    assert_eq!(block["found_not_counted"]["count"], 0);
    assert_eq!(
        block["tracked"],
        json!([{
            "position_id": "uniswap-v4:base:3163450", "nft_id": 3163450, "config_id": "tracked-v4-3163450",
            "role": "ladder", "rung": null, "rung_note": RUNG_UNSET_NOTE_TRACKED,
            "source": "detected", "deposit_tx": MINT_6CDA, "mint_block": 53008241,
            "tracked_at": "2026-11-17T12:47:16+00:00", "tick_lower": 262200, "tick_upper": 282300, "deposit_liquidity": null, "max_liquidity_seen": null,
            "state": "counted", "note": null,
            "choices": ["ladder:cap", "ladder:near", "ladder:deep", "untrack"],
        }])
    );

    // The suggestion then says a ladder is tracked, and where it came from.
    let mut data = cycle_data(raw);
    data.derived_metrics.position_detection = Some(block);
    let env = SnapshotEnvelope {
        meta: SnapshotMeta { generated_at: clock("operator"), state_hash: String::new(), version: SNAPSHOT_SPEC_VERSION.to_string(), snapshot_id: Uuid::nil() },
        data,
    };
    let l = ladder_suggestion(&env, Ok(Some(0.01)));
    assert_eq!(l["status"], "ladder_tracked");
    assert_eq!(l["reason"], "A ladder is tracked (v4 USDC/NOCK 3.00% #3163450, found on-chain): no new one is suggested. Its readings are in the Ladders card.");

    // A later cycle leaves the row exactly as it is.
    detect(&store, &n, &cfg, Some(&prev), clock("operator")).await;
    assert_eq!(store.tracked(OPERATOR).await.unwrap(), rows);
}

/// The pure rest of a cycle, as run_ingestion_cycle applies it, with no plan.
pub(crate) fn cycle_data(raw: RawOnChainData) -> SnapshotData {
    let plan = PlanContext { cfg: None, fills: PlanFills::default(), ledger_unavailable: false, now: clock("operator") };
    let profile = OperatorProfile { cash_reserve_usdc: Some(950.0), ladder_bottom_usd: Some(0.01), ..OperatorProfile::default() };
    let derived = compute_derived_metrics_with_plan(&raw, 0.0, &profile, StructuralChannels { channels: vec![] }, &[], &LedgerAggregate::default(), &MarketRegime::default(), &plan);
    cycle_snapshot_data(raw, derived, None, "2026-11-17T12:47:16+00:00").0
}

#[tokio::test]
async fn test_after_a_database_loss_the_positions_are_found_again_from_the_chain() {
    let cfg = book_without_v4();
    let prev = prev_suggesting(&[(262200, 282300, None)]);
    let n = node("operator", vec![wide_logs_swap("operator")]).await;
    let before = store_at(clock("operator"));
    detect(&before, &n, &cfg, Some(&prev), clock("operator")).await;
    let rows_before = before.tracked(OPERATOR).await.unwrap();
    assert_eq!(rows_before.len(), 1);
    let raw_before = read_cycle(&before, &n, &cfg).await.unwrap();

    // The database is lost: every table is empty. The same chain, the same suggestion.
    let after = store_at(clock("operator"));
    let out = detect(&after, &n, &cfg, Some(&prev), clock("operator")).await;
    assert_eq!(out.status, "ok");
    assert_eq!(after.tracked(OPERATOR).await.unwrap(), rows_before, "the same row, with the same mint as its baseline, from the chain alone");
    // The populated database and the rebuilt one give the same positions and ladders.
    let raw_after = read_cycle(&after, &n, &cfg).await.unwrap();
    let json = |raw: &RawOnChainData| (serde_json::to_value(snapshot_positions(raw)).unwrap(), serde_json::to_value(ladder_readings(raw)).unwrap());
    assert_eq!(json(&raw_before), json(&raw_after));

    // What is lost with the database is the snapshot history too. On the first cycle of
    // a fresh start there is no previous suggestion to match against, so the position is
    // found and STATED as not counted, with its value; it is not missing, and it is not
    // tracked on a guess. It is tracked from the cycle in which a suggestion names its
    // exact ticks again, or by one tap.
    let fresh = store_at(clock("operator"));
    let out = detect(&fresh, &n, &cfg, None, clock("operator")).await;
    assert!(fresh.tracked(OPERATOR).await.unwrap().is_empty());
    assert_eq!(out.found.len(), 1);
    let block = position_detection_block(&out, &read_cycle(&fresh, &n, &cfg).await.unwrap());
    assert_eq!(block["found_not_counted"]["value_usd"], "240.61");
    let tap = track_position(&fresh, &Client::new(), Some(&n.url), OPERATOR, &cfg.positions, 3163450, "ladder:deep").await.unwrap();
    assert_eq!((tap["status"].as_str(), tap["rung"].as_str(), tap["deposit_tx"].as_str()), (Some("tracked"), Some("deep"), Some(MINT_6CDA)));
}

#[tokio::test]
async fn test_a_range_limited_rpc_is_read_in_chunks_and_the_snapshot_says_how_far_back() {
    // The public node's own answers. It refuses the whole history; the chunks it serves
    // are read one request each, backward, and a chunk it refuses ends the backfill
    // there. Here the backfill has already reached block 53021286 (a stored cursor).
    let archive_refusal = recorded("operator", &key_logs(OPERATOR, 25_800_001, 25_845_000));
    assert_eq!(archive_refusal["http_status"], 403);
    let n = node(
        "operator",
        vec![
            // The real refusal of a too-old range, for the two requests that reach below
            // what this node serves (recorded for 0x189ad41..0x18a5d08 and for 0x0..head).
            (key_logs(OPERATOR, 0, 53_021_285), Swap::Json(archive_refusal.clone())),
            (key_logs(OPERATOR, 52_886_286, 52_931_285), Swap::Json(archive_refusal.clone())),
        ],
    )
    .await;
    let store = store_at(clock("operator"));
    let cursor = ScanState { logs_from_block: Some(53_021_286), logs_to_block: Some(58_646_285), ..Default::default() };
    store.save_scan_state(OPERATOR, TRACKED_PROTOCOL_V4, TRACKED_NETWORK_BASE, &cursor).await.unwrap();
    let cfg = book_without_v4();
    let out = detect(&store, &n, &cfg, None, clock("operator")).await;

    // Four log requests: the wide one (refused), two chunks (served), one chunk (refused).
    assert_eq!(n.count("eth_getLogs"), 4);
    let asked: Vec<String> = n.keys().into_iter().filter(|k| k.starts_with("eth_getLogs")).collect();
    assert_eq!(
        asked,
        vec![
            key_logs(OPERATOR, 0, 53_021_285),
            key_logs(OPERATOR, 52_976_286, 53_021_285),
            key_logs(OPERATOR, 52_931_286, 52_976_285),
            key_logs(OPERATOR, 52_886_286, 52_931_285),
        ]
    );
    assert_eq!((52_976_286i64..=53_021_285).count() as i64, POSITION_LOG_CHUNK_BLOCKS);
    // The index is tried because the logs do not cover the history; this node has none.
    assert_eq!(n.count("alchemy_getAssetTransfers"), 1);
    assert_eq!(out.run.history.status, "failed");
    assert_eq!(out.run.history.reason.as_deref(), Some("the RPC answered with error -32601: Method not found"));

    // The three NFTs in the chunk were found and read; one holds liquidity.
    let ids: Vec<u64> = store.candidates(OPERATOR).await.unwrap().iter().map(|c| c.nft_id).collect();
    assert_eq!(ids, vec![3163431, 3163432, 3163450]);
    assert_eq!(out.found.len(), 1);
    assert_eq!(out.found[0].row.nft_id, 3163450);

    // Partial, and it says exactly what is missing: how far back the logs were read,
    // why no further, and that the wallet holds more NFTs than were found.
    assert_eq!(out.status, "partial");
    let reason = out.reason.clone().unwrap();
    assert!(reason.contains("transfers before block 52931286 are not read yet (this RPC does not serve older blocks (the RPC answered with error -32602: Archive requests require a personal token."), "{reason}");
    assert!(reason.contains("transfer index: the RPC answered with error -32601: Method not found"), "{reason}");
    assert!(reason.ends_with("the wallet holds 43 Uniswap v4 position NFTs on-chain and 3 are known: 40 not found yet"), "{reason}");
    assert_eq!(out.scan["coverage"], "partial");
    assert_eq!(out.scan["logs_from_block"], 52_931_286);
    assert_eq!(out.scan["logs_behind_blocks"], 0);
    assert!(out.scan["text"].as_str().unwrap().starts_with("The wallet's position-NFT transfers were read from block 52931286 to this cycle's block. A position that reached the wallet before block 52931286 is not found yet:"));
    let state = store.scan_state(OPERATOR, TRACKED_PROTOCOL_V4, TRACKED_NETWORK_BASE).await.unwrap();
    assert_eq!((state.logs_from_block, state.logs_to_block), (Some(52_931_286), Some(58_646_285)), "the cursor is stored, so the scan resumes");
    assert_no_url(&serde_json::to_string(&out.scan).unwrap(), &n.url);
    assert_no_url(&reason, &n.url);

    // The refusals are remembered for this RPC: the next cycle asks for no range it was
    // refused (the index is tried again, in case the RPC has one after all).
    n.forget();
    let again = detect(&store, &n, &cfg, None, clock("operator")).await;
    assert_eq!((n.count("eth_getLogs"), n.count("alchemy_getAssetTransfers")), (0, 1));
    assert_eq!(again.status, "partial");
    assert_eq!(again.reason, out.reason);

    // From nothing, a range-limited RPC is read newest chunk first.
    let n = node("operator", vec![]).await;
    let fresh = store_at(clock("operator"));
    detect(&fresh, &n, &cfg, None, clock("operator")).await;
    let asked: Vec<String> = n.keys().into_iter().filter(|k| k.starts_with("eth_getLogs")).take(2).collect();
    assert_eq!(asked, vec![key_logs(OPERATOR, 0, 58_646_285), key_logs(OPERATOR, 58_601_286, 58_646_285)], "the whole range is tried once, then the newest chunk");
}

#[tokio::test]
async fn test_the_transfer_index_covers_the_history_when_the_logs_cannot() {
    // An RPC that refuses every log request (a ten-block limit, say) and serves the
    // transfer index: the page is ASSEMBLED from the real logs (see the module note).
    let scn = scenario("operator");
    let logs = scn["wide_logs"].as_array().unwrap().clone();
    let refusal = json!({ "error": { "code": -32600, "message": "You can make eth_getLogs requests with up to a 10 block range" } });
    let n = node(
        "operator",
        vec![
            (key_logs(OPERATOR, 0, 58_646_285), Swap::Json(refusal.clone())),
            (key_logs(OPERATOR, 58_601_286, 58_646_285), Swap::Json(refusal)),
            ("alchemy_getAssetTransfers".to_string(), Swap::Json(assembled_history_page(&logs, None))),
        ],
    )
    .await;
    let store = store_at(clock("operator"));
    let out = detect(&store, &n, &book(), None, clock("operator")).await;
    assert_eq!((n.count("eth_getLogs"), n.count("alchemy_getAssetTransfers")), (2, 1));
    assert_eq!(out.run.logs.status, "refused");
    assert_eq!(out.run.history.status, "complete");
    assert_eq!(out.status, "ok", "{:?}", out.reason);
    assert_eq!(out.scan["coverage"], "complete");
    assert_eq!(out.scan["sources"], json!({ "transfer_logs": "refused", "transfer_index": "complete" }));
    assert_eq!(out.scan["text"], "Every Uniswap v4 position NFT sent to the wallet was read from the RPC's transfer index, which can trail the chain by a short time.");
    assert_eq!(store.candidates(OPERATOR).await.unwrap().len(), 43);
    assert_eq!((out.run.held, out.run.known_owned), (Some(43), 43));
    // The log refusal is remembered: the next cycle goes straight to the index, one page.
    n.forget();
    detect(&store, &n, &book(), None, clock("operator")).await;
    assert_eq!((n.count("eth_getLogs"), n.count("alchemy_getAssetTransfers")), (0, 1));
}

#[tokio::test]
async fn test_a_position_minted_by_its_wallet_is_tracked_from_the_chain_alone() {
    // The #167 fixture's NFT #3240369, seen from the wallet that minted it, at the block
    // of its mint. Every answer is the public node's. That wallet holds 285 v4 NFTs and
    // the node serves only recent chunks, so the scan is partial and says so, while the
    // new mint is found at once.
    let n = node("minter", vec![]).await;
    let store = store_at(clock("minter"));
    let cfg = config_for(MINTER, &[ENTRY_V3]);
    let prev = prev_suggesting(&[(270900, 887100, None)]);
    let out = detect(&store, &n, &cfg, Some(&prev), clock("minter")).await;
    assert_eq!(out.newly_tracked, vec!["uniswap-v4:base:3240369".to_string()]);
    let rows = store.tracked(MINTER).await.unwrap();
    assert_eq!(rows.len(), 1);
    let row = &rows[0];
    assert_eq!((row.nft_id, row.pool_id.as_str(), row.tick_lower, row.tick_upper), (3240369, POOL_3PCT, 270900, 887100));
    assert_eq!((row.role.as_deref(), row.rung.as_deref(), row.source.as_str()), (Some("ladder"), None, "detected"));
    assert_eq!((row.deposit_tx.as_deref(), row.mint_block), (Some(MINT_AFE4_HASH), Some(59660587)));
    assert_eq!(validate_tracked_row(row), Ok(()));
    let c = store.candidates(MINTER).await.unwrap().into_iter().find(|c| c.nft_id == 3240369).unwrap();
    assert_eq!((c.minted_by.as_deref(), c.deposit_check.as_deref(), c.owned, c.has_liquidity), (Some(MINTER), Some("ok"), true, Some(true)));
    // The pool was not one NockNock already read: it came from the mint receipt, checked
    // against the 25 bytes the position manager stores.
    assert_eq!(c.pool_id.as_deref(), Some(POOL_3PCT));
    // The baseline the cycle would read from that row is the #167 fixture's verified one.
    let receipt = recorded("minter", &key_receipt(MINT_AFE4_HASH))["result"].clone();
    let deposit = parse_ladder_deposit(&receipt, MINTER, &candidate_raw_position(&c, POOL_3PCT).unwrap()).unwrap();
    assert_eq!((deposit.tx.as_str(), deposit.block), (MINT_AFE4_HASH, 59660587));

    assert_eq!(out.status, "partial");
    let reason = out.reason.unwrap();
    assert!(reason.contains("the wallet holds 285 Uniswap v4 position NFTs on-chain and 1 are known: 284 not found yet"), "{reason}");
    assert_eq!(out.run.held, Some(285));
    assert_eq!(n.call_blocks(UNISWAP_V4_POSM_BASE), HashSet::from(["0x38e592b".to_string()]));

    // The same NFT seen from ANOTHER wallet's scan cannot be tracked for it: the log
    // filter names the wallet, and this wallet received nothing.
    let n = node("minter", vec![(key_logs(OPERATOR, 0, 59_660_587), Swap::Json(json!({ "result": [] })))]).await;
    let other = store_at(clock("minter"));
    let out = detect(&other, &n, &config_for(OPERATOR, &[ENTRY_V3]), Some(&prev), clock("minter")).await;
    assert!(out.newly_tracked.is_empty() && other.candidates(OPERATOR).await.unwrap().is_empty());
}

#[tokio::test]
async fn test_a_stranger_cannot_mark_a_rung_deployed_and_dust_is_never_offered() {
    // The operator wallet's real NFT, with its mint receipt re-attributed to a stranger
    // (the one field changed is the receipt's sender): same pool, same exact ticks.
    let mut receipt = recorded("operator", &key_receipt(MINT_6CDA));
    receipt["result"]["from"] = json!(MINTER);
    let n = node("operator", vec![wide_logs_swap("operator"), (key_receipt(MINT_6CDA), Swap::Json(receipt))]).await;
    let store = store_at(clock("operator"));
    let cfg = book_without_v4();
    let prev = prev_suggesting(&[(262200, 282300, None)]);
    let out = detect(&store, &n, &cfg, Some(&prev), clock("operator")).await;
    assert!(out.newly_tracked.is_empty() && store.tracked(OPERATOR).await.unwrap().is_empty());
    assert_eq!(out.found.len(), 1);
    let why = &out.found[0].why_not_automatic;
    assert_eq!(why[0], "its mint was sent by 0x4598…e826, not by your wallet");
    assert!(why[1].starts_with("its mint is not a plain deposit into it: the transaction was sent by 0x4598b3ba7d6714091b18da5d110b64f56e94e826, not by the tracked wallet"), "{why:?}");
    // It is still offered: one tap tracks it. As a ladder its baseline then reads refused.
    assert_eq!(out.found[0].choices, vec!["ladder:cap", "ladder:near", "ladder:deep", "lp", "dismiss"]);

    // The 37 NFTs in pools with a token outside the whitelist are counted, never listed,
    // and nothing is read from them beyond the pool key and the owner.
    assert_eq!(out.ignored["not_whitelisted"], 37);
    let dust: Vec<CandidateRow> = store.candidates(OPERATOR).await.unwrap().into_iter().filter(|c| c.verdict == "not_whitelisted").collect();
    assert_eq!(dust.len(), 37);
    assert!(dust.iter().all(|c| c.has_liquidity.is_none() && c.mint_tx.is_none() && c.pool_id.is_none()));
    for d in dust.iter().take(3) {
        assert_eq!(n.count(&key_liquidity(d.nft_id)), 0);
        let refused = track_position(&store, &Client::new(), Some(&n.url), OPERATOR, &cfg.positions, d.nft_id, "lp").await.unwrap_err();
        assert_eq!((refused.0, refused.1), (StatusCode::UNPROCESSABLE_ENTITY, "NOT_ELIGIBLE"));
        assert_eq!(set_found_choice(&store, OPERATOR, d.nft_id, true).await.unwrap_err().1, "NOT_AN_OFFER");
    }
}

#[tokio::test]
async fn test_an_emptied_nft_that_is_refilled_is_found_again_without_a_transfer() {
    let n = node("operator", vec![wide_logs_swap("operator")]).await;
    let store = store_at(clock("operator"));
    let cfg = book();
    detect(&store, &n, &cfg, None, clock("operator")).await;
    let row = store.candidates(OPERATOR).await.unwrap().into_iter().find(|c| c.nft_id == 3163431).unwrap();
    assert_eq!((row.verdict.as_str(), row.owned, row.has_liquidity), ("eligible", true, Some(false)), "the wallet's own empty NFT in the 0.75% pool");

    // Liquidity is added to it in the app: no Transfer is emitted. The answer swapped in
    // is the recorded liquidity of the wallet's other NFT.
    let refilled = recorded("operator", &key_liquidity(3163450));
    // The 0.75% pool's price does not answer in this cycle (the connection drops).
    let slot0 = call_key(UNISWAP_V4_STATE_VIEW_BASE, &format!("{V4_STATE_VIEW_GET_SLOT0}{}", &POOL_075PCT[2..]));
    let n = node("operator", vec![wide_logs_swap("operator"), (key_liquidity(3163431), Swap::Json(refilled)), (slot0, Swap::Drop)]).await;
    let out = detect(&store, &n, &cfg, None, clock("operator")).await;
    assert_eq!(n.count("eth_getLogs"), 0, "the head has not moved: nothing new in the logs");
    assert_eq!(out.found.len(), 1);
    let f = &out.found[0];
    assert_eq!((f.row.nft_id, f.row.has_liquidity), (3163431, Some(true)));
    // Its pool is not one config.yaml reads: it came from the NFT's own mint receipt.
    assert_eq!(f.row.pool_id.as_deref(), Some(POOL_075PCT));
    assert_eq!(f.row.minted_by.as_deref(), Some(OPERATOR));
    assert_eq!(out.ignored["empty"], 4);
    // Its pool's price was not read, so its value is not stated, and the list total is
    // null with the reason: never a partial sum, never a zero.
    let block = position_detection_block(&out, &read_cycle(&store, &n, &cfg).await.unwrap());
    let p = &block["found_not_counted"]["positions"][0];
    assert!(p["value_usd"].is_null());
    assert_eq!(p["value_note"], "its pool's price could not be read this cycle, so what it holds and its value are not stated");
    assert_eq!(block["found_not_counted"]["value_usd"], Value::Null);
    assert_eq!(block["found_not_counted"]["value_note"], "1 of 1 could not be valued this cycle, so no total is stated");
}

#[tokio::test]
async fn test_an_nft_that_left_the_wallet_is_noticed_by_the_count_and_its_owner_is_read_again() {
    let n = node("operator", vec![wide_logs_swap("operator")]).await;
    let store = store_at(clock("operator"));
    detect(&store, &n, &book(), None, clock("operator")).await;
    // The wallet now holds 42: NFT #2149 (a dust one) was sent to another address.
    let held_42 = json!({ "result": format!("0x{:064x}", 42) });
    let new_owner = json!({ "result": format!("0x{:0>64}", &MINTER[2..]) });
    let balance_key = call_key(UNISWAP_V4_POSM_BASE, &format!("{ERC721_BALANCE_OF}{:0>64}", &OPERATOR[2..]));
    let n = node("operator", vec![wide_logs_swap("operator"), (balance_key, Swap::Json(held_42)), (key_owner(2149), Swap::Json(new_owner))]).await;
    let out = detect(&store, &n, &book(), None, clock("operator")).await;
    assert_eq!((out.run.held, out.run.known_owned), (Some(42), 42));
    assert_eq!(out.status, "ok");
    let row = store.candidates(OPERATOR).await.unwrap().into_iter().find(|c| c.nft_id == 2149).unwrap();
    assert!(!row.owned);
    // The owners of the NFTs not read this cycle were read again: 43 less the 5 empties.
    assert_eq!(n.keys().iter().filter(|k| k.starts_with(&call_key(UNISWAP_V4_POSM_BASE, ERC721_OWNER_OF))).count(), 38);
}

#[tokio::test]
async fn test_when_it_cannot_check_it_says_so_keeps_what_it_found_and_never_prints_the_url() {
    let cfg = book_without_v4();
    // A previous cycle found the position.
    let n = node("operator", vec![wide_logs_swap("operator")]).await;
    let store = store_at(clock("operator"));
    detect(&store, &n, &cfg, None, clock("operator")).await;
    let raw = read_cycle(&store, &n, &cfg).await.unwrap();

    // The RPC stops answering. Its URL carries the key.
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let dead = format!("http://{}/v2/not-a-real-key", listener.local_addr().unwrap());
    drop(listener);
    let prev = prev_suggesting(&[(262200, 282300, None)]);
    let later = clock("operator") + chrono::Duration::seconds(300);
    let out = detect_positions(&DetectionInputs {
        store: &store,
        http: &Client::new(),
        rpc_url: Some(&dead),
        wallet: Some(OPERATOR),
        config_positions: &cfg.positions,
        prev: Some(&prev),
        now: later,
        read_budget: None,
    })
    .await;
    assert_eq!(out.status, "unavailable");
    assert_eq!(
        out.reason.as_deref(),
        Some("New positions could not be checked this cycle (the node's latest block could not be read): a deposit made since the last check would not be counted yet. The transfers were last read at 2026-11-17T12:47:16+00:00.")
    );
    assert_eq!(out.last_checked_at, Some(clock("operator")), "the time of the last check that ran, not of this one");
    // What was found stays listed, without a value it did not read this cycle, and it is
    // not tracked on an old reading even though the suggestion now names its ticks.
    assert_eq!(out.found.len(), 1);
    assert!(out.found[0].read.is_none());
    assert_eq!(out.found[0].why_not_automatic, vec!["it was not read this cycle, so it is not tracked on an old reading".to_string()]);
    assert!(store.tracked(OPERATOR).await.unwrap().is_empty());
    let block = position_detection_block(&out, &raw);
    let p = &block["found_not_counted"]["positions"][0];
    assert_eq!((p["nft_id"].as_u64(), p["value_usd"].is_null()), (Some(3163450), true));
    assert_eq!(p["value_note"], "it was not read this cycle, so what it holds and its value are not stated");
    assert!(block["checked_at_block"].is_null() && block["onchain_count"]["held"].is_null() && block["onchain_count"]["known"].is_null());
    let text = crate::position_detect_text::position_detection_lines(&json!({ "derived_metrics": { "position_detection": block } }));
    assert!(text.starts_with("• Positions found on-chain (unavailable): New positions could not be checked this cycle"), "{text}");
    for surface in [serde_json::to_string(&block).unwrap(), text] {
        assert!(!surface.contains("not-a-real-key") && !surface.contains("127.0.0.1") && !surface.contains("/v2/"), "{surface}");
    }

    // No key, no wallet, no database: each says which.
    let no_key = detect_positions(&DetectionInputs { store: &store, http: &Client::new(), rpc_url: None, wallet: Some(OPERATOR), config_positions: &cfg.positions, prev: None, now: later, read_budget: None }).await;
    assert_eq!(no_key.status, "unavailable");
    assert!(no_key.reason.unwrap().starts_with("New positions could not be checked this cycle (no Alchemy RPC key is configured)"));
    let no_wallet = detect_positions(&DetectionInputs { store: &store, http: &Client::new(), rpc_url: Some(&dead), wallet: None, config_positions: &cfg.positions, prev: None, now: later, read_budget: None }).await;
    assert_eq!((no_wallet.status.as_str(), no_wallet.reason.as_deref()), ("not_run", Some("no tracked wallet is configured")));
    if let PositionStore::Mem(m) = &store {
        m.lock().unwrap().unreadable = true;
    }
    let n = node("operator", vec![wide_logs_swap("operator")]).await;
    let no_db = detect(&store, &n, &cfg, None, later).await;
    assert_eq!(no_db.status, "unavailable");
    assert_eq!(
        no_db.reason.as_deref(),
        Some("Positions found on-chain could not be listed this cycle (the detection tables could not be read): one that is found and not counted is not shown.")
    );
    assert!(no_db.found.is_empty());
}

// ------------------------------------------------------------------
// The one-tap writes
// ------------------------------------------------------------------

#[tokio::test]
async fn test_one_tap_tracks_untracks_dismisses_and_restores() {
    let n = node("operator", vec![wide_logs_swap("operator")]).await;
    let store = store_at(clock("operator"));
    let cfg = book_without_v4();
    let http = Client::new();
    detect(&store, &n, &cfg, None, clock("operator")).await;
    let track = |nft: u64, choice: &'static str| track_position(&store, &http, Some(&n.url), OPERATOR, &cfg.positions, nft, choice);

    // Unknown choices, unknown NFTs and choices not offered change nothing.
    assert_eq!(parse_track_as("lp").unwrap(), (None, None));
    assert_eq!(parse_track_as("ladder:near").unwrap(), (Some("ladder"), Some("near".to_string())));
    for bad in ["", "ladder", "ladder:", "ladder:middle", "LP", "dismiss", "ladder:near "] {
        let e = track(3163450, bad).await.unwrap_err();
        assert_eq!((e.0, e.1), (StatusCode::BAD_REQUEST, "UNKNOWN_CHOICE"), "{bad}");
        assert!(e.2.ends_with("Nothing changed."));
    }
    assert_eq!(track(999, "lp").await.unwrap_err().1, "NOT_FOUND");
    // An empty NFT: there is nothing to track.
    assert_eq!(track(3163431, "lp").await.unwrap_err().1, "CHOICE_NOT_OFFERED", "its pool was never identified: it has always been empty");
    // A WETH/USDC range (the wallet's empty #2179) can never be a ladder.
    assert_eq!(track(2179, "ladder:near").await.unwrap_err().1, "CHOICE_NOT_OFFERED");
    assert!(store.tracked(OPERATOR).await.unwrap().is_empty());

    // TRACK AS LP: counted, classified by its pair, and never a rebuild target.
    let done = track(3163450, "lp").await.unwrap();
    assert_eq!((done["status"].as_str(), done["role"].is_null(), done["position_id"].as_str()), (Some("tracked"), true, Some("uniswap-v4:base:3163450")));
    assert!(done["note"].as_str().unwrap().contains("never a rebuild target"));
    let row = store.tracked(OPERATOR).await.unwrap().remove(0);
    assert_eq!((row.role.clone(), row.rung.clone(), row.source.as_str()), (None, None, "confirmed"));
    assert_eq!(tracked_position_config(&row).rebuild_weight_bps, Some(0));
    let raw = read_cycle(&store, &n, &cfg).await.unwrap();
    assert!(!is_ladder(&raw, raw.positions.iter().find(|p| p.nft_id == 3163450).unwrap()));
    assert_eq!(raw.rebuild_rules["uniswap-v4:base:3163450"].weight_bps, Some(0));
    let (suggestions, excluded) = lp_rebuild_suggestions(&raw, 10_000.0, RebuildFunds { nock: 1e9, floor_tolerance_nock: 0.0, floor_set: false, usdc: 1e9 });
    assert!(suggestions.iter().all(|s| s.position_id != "uniswap-v4:base:3163450"), "a position tracked from the chain is never a rebuild template");
    let why = excluded.iter().find(|x| x.position_id == "uniswap-v4:base:3163450").unwrap();
    assert_eq!(why.reason, "tracked from the chain, not from config.yaml: counted, never a rebuild target");

    // TRACK AS NEAR RUNG on the tracked row changes how it is tracked, keeping its mint.
    let done = track(3163450, "ladder:near").await.unwrap();
    assert_eq!((done["role"].as_str(), done["rung"].as_str(), done["deposit_tx"].as_str()), (Some("ladder"), Some("near"), Some(MINT_6CDA)));
    let row = store.tracked(OPERATOR).await.unwrap().remove(0);
    assert_eq!((row.role.as_deref(), row.rung.as_deref(), row.deposit_tx.as_deref(), row.tracked_at), (Some("ladder"), Some("near"), Some(MINT_6CDA), Some(clock("operator"))));
    // A tracked position is not an offer: NOT NOW is refused, UNTRACK is the way.
    assert_eq!(set_found_choice(&store, OPERATOR, 3163450, true).await.unwrap_err().1, "TRACKED");

    // UNTRACK: the row goes, the NFT is set aside so it is not tracked again by itself.
    let gone = untrack_position(&store, OPERATOR, &cfg.positions, "uniswap-v4:base:3163450").await.unwrap();
    assert_eq!(gone["status"], "untracked");
    assert_eq!(gone["note"], "NFT #3163450 is still in your wallet and is no longer counted. It is listed as set aside and can be tracked again.");
    assert!(store.tracked(OPERATOR).await.unwrap().is_empty());
    let prev = prev_suggesting(&[(262200, 282300, None)]);
    let out = detect(&store, &n, &cfg, Some(&prev), clock("operator")).await;
    assert!(out.newly_tracked.is_empty());
    assert_eq!((out.found.len(), out.found[0].dismissed), (1, true));
    assert_eq!(untrack_position(&store, OPERATOR, &cfg.positions, "uniswap-v4:base:3163450").await.unwrap_err().1, "NOT_TRACKED");
    // A config.yaml entry is the operator's file: never removed from here.
    let e = untrack_position(&store, OPERATOR, &cfg.positions, "uniswap-v3:base:6336662").await.unwrap_err();
    assert_eq!((e.0, e.1), (StatusCode::UNPROCESSABLE_ENTITY, "CONFIG_POSITION"));
    // And one config.yaml already lists cannot be tracked from the chain as well.
    let with_v4 = book();
    assert_eq!(track_position(&store, &http, Some(&n.url), OPERATOR, &with_v4.positions, 3163450, "lp").await.unwrap_err().1, "CONFIG_POSITION");

    // Dismiss and restore are idempotent and say whether anything changed.
    assert_eq!(set_found_choice(&store, OPERATOR, 3163450, true).await.unwrap()["changed"], false);
    assert_eq!(set_found_choice(&store, OPERATOR, 3163450, false).await.unwrap()["changed"], true);
    assert_eq!(set_found_choice(&store, OPERATOR, 3163450, false).await.unwrap()["changed"], false);
    assert_eq!(set_found_choice(&store, OPERATOR, 999, true).await.unwrap_err().1, "NOT_FOUND");
    // Tracking a set-aside position clears the dismissal.
    set_found_choice(&store, OPERATOR, 3163450, true).await.unwrap();
    track(3163450, "ladder:deep").await.unwrap();
    assert_eq!(store.candidates(OPERATOR).await.unwrap().iter().find(|c| c.nft_id == 3163450).unwrap().choice, None);
}

#[tokio::test]
async fn test_a_tap_reads_the_chain_again_and_refuses_what_is_no_longer_there() {
    let cfg = book_without_v4();
    let http = Client::new();
    let seed = node("operator", vec![wide_logs_swap("operator")]).await;
    let store = store_at(clock("operator"));
    detect(&store, &seed, &cfg, None, clock("operator")).await;

    // Since it was found, the NFT was sent away.
    let n = node("operator", vec![(key_owner(3163450), Swap::Json(json!({ "result": format!("0x{:0>64}", &MINTER[2..]) })))]).await;
    let e = track_position(&store, &http, Some(&n.url), OPERATOR, &cfg.positions, 3163450, "lp").await.unwrap_err();
    assert_eq!((e.0, e.1, e.2.as_str()), (StatusCode::UNPROCESSABLE_ENTITY, "NOT_OWNED", "NFT #3163450 is owned by 0x4598…e826, not by the tracked wallet. Nothing changed."));
    // Or burned (the position manager's own answer for a burned id).
    let burned: Value = json!({ "error": { "code": 3, "message": "execution reverted: NOT_MINTED", "data": "0x08c379a00000000000000000000000000000000000000000000000000000000000000020000000000000000000000000000000000000000000000000000000000000000a4e4f545f4d494e54454400000000000000000000000000000000000000000000" } });
    let n = node("operator", vec![(key_owner(3163450), Swap::Json(burned))]).await;
    assert_eq!(track_position(&store, &http, Some(&n.url), OPERATOR, &cfg.positions, 3163450, "lp").await.unwrap_err().2, "NFT #3163450 no longer exists. Nothing changed.");
    // Or emptied.
    let n = node("operator", vec![(key_liquidity(3163450), Swap::Json(json!({ "result": format!("0x{:064x}", 0) })))]).await;
    assert_eq!(track_position(&store, &http, Some(&n.url), OPERATOR, &cfg.positions, 3163450, "lp").await.unwrap_err().1, "EMPTY");
    // The node does not answer: nothing is written on an unread chain.
    let n = node("operator", vec![(key_owner(3163450), Swap::Drop)]).await;
    let e = track_position(&store, &http, Some(&n.url), OPERATOR, &cfg.positions, 3163450, "lp").await.unwrap_err();
    assert_eq!((e.0, e.1), (StatusCode::SERVICE_UNAVAILABLE, "CHAIN_UNAVAILABLE"));
    assert_no_url(&e.2, &n.url);
    assert_eq!(track_position(&store, &http, None, OPERATOR, &cfg.positions, 3163450, "lp").await.unwrap_err().1, "NO_RPC");
    assert!(store.tracked(OPERATOR).await.unwrap().is_empty(), "every refusal left the table as it was");
    // Both reads are made at `latest`, not at a remembered block.
    let n = node("operator", vec![]).await;
    track_position(&store, &http, Some(&n.url), OPERATOR, &cfg.positions, 3163450, "lp").await.unwrap();
    assert_eq!(n.call_blocks(UNISWAP_V4_POSM_BASE), HashSet::from(["latest".to_string()]));
    assert_eq!(n.total(), 2);
    // An unreadable database is a stated error, not a silent success.
    if let PositionStore::Mem(m) = &store {
        m.lock().unwrap().unreadable = true;
    }
    let e = untrack_position(&store, OPERATOR, &cfg.positions, "uniswap-v4:base:3163450").await.unwrap_err();
    assert_eq!((e.0, e.1), (StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR"));
}

/// A worker with config.yaml's book, no RPC key and a database that is never reached.
fn offline_state() -> Arc<AppState> {
    let mut config = book();
    config.alchemy_api_keys = vec!["YOUR_ALCHEMY_API_KEY".to_string()];
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
}

#[tokio::test]
async fn test_every_position_write_sits_behind_the_cross_site_guard_and_takes_a_json_body() {
    let state = offline_state();
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    tokio::spawn(async move { axum::serve(listener, api_router(state)).await.unwrap() });
    let base = format!("http://{addr}");
    let client = Client::new();
    let writes = [
        ("/v1/positions/track", json!({ "nft_id": 3163450, "as": "ladder:middle" })),
        ("/v1/positions/untrack", json!({ "position_id": "" })),
        ("/v1/positions/dismiss", json!({ "nft_id": 3163450 })),
        ("/v1/positions/restore", json!({ "nft_id": 3163450 })),
    ];
    for (path, body) in &writes {
        // REFUSED: a page elsewhere, with or without Sec-Fetch-Site, before any handler.
        for headers in [vec![("Origin", "https://evil.example"), ("Sec-Fetch-Site", "cross-site")], vec![("Origin", "https://evil.example")], vec![("Sec-Fetch-Site", "cross-site")]] {
            let mut req = client.post(format!("{base}{path}")).json(body);
            for (k, v) in headers {
                req = req.header(k, v);
            }
            let r = req.send().await.unwrap();
            assert_eq!(r.status(), reqwest::StatusCode::FORBIDDEN, "{path}");
            let b: Value = r.json().await.unwrap();
            assert_eq!(b["error"], "CROSS_SITE_REFUSED", "{path}");
        }
        // A form post (no JSON content type) from the same host is not a JSON body: the
        // handler never runs. A cross-site page can only send such a body without a preflight.
        let r = client.post(format!("{base}{path}")).header("Content-Type", "text/plain").body(body.to_string()).send().await.unwrap();
        assert_eq!(r.status(), reqwest::StatusCode::UNSUPPORTED_MEDIA_TYPE, "{path}");
        let r = client.post(format!("{base}{path}")).send().await.unwrap();
        assert_eq!(r.status(), reqwest::StatusCode::UNSUPPORTED_MEDIA_TYPE, "{path}: a body-less POST");
    }
    // ALLOWED: the dashboard's own request (same host, through nginx) reaches the handler.
    // track answers from its own validation, before any database or chain read.
    let r = client
        .post(format!("{base}/v1/positions/track"))
        .header("Origin", format!("http://{}", addr.ip()))
        .header("Sec-Fetch-Site", "same-origin")
        .json(&writes[0].1)
        .send()
        .await
        .unwrap();
    assert_eq!(r.status(), reqwest::StatusCode::BAD_REQUEST);
    let b: Value = r.json().await.unwrap();
    assert_eq!(b["error"], "UNKNOWN_CHOICE");
    // A script with neither header (curl, an MCP client) passes the guard as well.
    let r = client.post(format!("{base}/v1/positions/track")).json(&json!({ "nft_id": 1, "as": "nope" })).send().await.unwrap();
    assert_eq!(r.status(), reqwest::StatusCode::BAD_REQUEST);
    // The read is a GET and is never refused as cross-site (it answers 503 here: the
    // database is unreachable, and it says so rather than list nothing).
    let router = include_str!("api.rs");
    for route in ["/v1/positions/found", "/v1/positions/track", "/v1/positions/untrack", "/v1/positions/dismiss", "/v1/positions/restore"] {
        let at = router.find(&format!("\"{route}\"")).unwrap_or_else(|| panic!("{route} is routed"));
        assert!(at < router.find(".layer(axum::middleware::from_fn(refuse_cross_site_writes))").unwrap(), "{route} is inside the guard");
    }
    assert_eq!(router.matches("axum::routing::post(track_position_handler)").count(), 1);
    // No handler takes a typed transaction hash or signs anything.
    let src = include_str!("position_detect.rs");
    for banned in ["eth_sendTransaction", "eth_sendRawTransaction", "eth_sign", "personal_sign", "private_key"] {
        assert!(!src.contains(banned) && !include_str!("tracked.rs").contains(banned), "{banned}");
    }
}

// ------------------------------------------------------------------
// The text both MCP transports print, and the dashboard's fixture
// ------------------------------------------------------------------

/// Blocks from the recorded chain, with the lines both MCP transports print for each
/// (mcp_bin_tests_position_detect.rs reads the file; the frontend's fixture is a copy).
/// Regenerate only for an intended change:
///   NOCKNOCK_BLESS_GOLDEN=1 cargo test test_golden_position_detection
#[tokio::test]
async fn test_golden_position_detection_blocks_and_text() {
    let lines = |block: &Value| crate::position_detect_text::position_detection_lines(&json!({ "derived_metrics": { "position_detection": block } }));
    let mut doc = serde_json::Map::new();
    let mut add = |name: &str, block: Value| {
        doc.insert(name.to_string(), json!({ "position_detection": block, "text": lines(&block) }));
    };
    let without_v4 = book_without_v4();
    let prev = prev_suggesting(&[(262200, 282300, None)]);

    // Everything read, nothing that is not already counted.
    let n = node("operator", vec![wide_logs_swap("operator")]).await;
    let store = store_at(clock("operator"));
    let out = detect(&store, &n, &book(), None, clock("operator")).await;
    add("nothing_found", position_detection_block(&out, &read_cycle(&store, &n, &book()).await.unwrap()));

    // One position found and not counted.
    let store = store_at(clock("operator"));
    let out = detect(&store, &n, &without_v4, None, clock("operator")).await;
    let raw = read_cycle(&store, &n, &without_v4).await.unwrap();
    add("found_not_counted", position_detection_block(&out, &raw));

    // The same position set aside.
    set_found_choice(&store, OPERATOR, 3163450, true).await.unwrap();
    let out = detect(&store, &n, &without_v4, None, clock("operator")).await;
    add("dismissed", position_detection_block(&out, &read_cycle(&store, &n, &without_v4).await.unwrap()));

    // The same position matching a suggested rung: tracked by itself, counted.
    let store = store_at(clock("operator"));
    let out = detect(&store, &n, &without_v4, Some(&prev), clock("operator")).await;
    add("tracked_by_itself", position_detection_block(&out, &read_cycle(&store, &n, &without_v4).await.unwrap()));

    // Tracked by a tap as a deep rung.
    let store = store_at(clock("operator"));
    detect(&store, &n, &without_v4, None, clock("operator")).await;
    track_position(&store, &Client::new(), Some(&n.url), OPERATOR, &without_v4.positions, 3163450, "ladder:deep").await.unwrap();
    let out = detect(&store, &n, &without_v4, None, clock("operator")).await;
    add("tracked_by_tap", position_detection_block(&out, &read_cycle(&store, &n, &without_v4).await.unwrap()));

    // A range-limited RPC part-way through its backfill.
    let archive_refusal = recorded("operator", &key_logs(OPERATOR, 25_800_001, 25_845_000));
    let limited = node("operator", vec![(key_logs(OPERATOR, 0, 53_021_285), Swap::Json(archive_refusal.clone())), (key_logs(OPERATOR, 52_886_286, 52_931_285), Swap::Json(archive_refusal))]).await;
    let store = store_at(clock("operator"));
    store.save_scan_state(OPERATOR, TRACKED_PROTOCOL_V4, TRACKED_NETWORK_BASE, &ScanState { logs_from_block: Some(53_021_286), logs_to_block: Some(58_646_285), ..Default::default() }).await.unwrap();
    let out = detect(&store, &limited, &without_v4, None, clock("operator")).await;
    add("partial", position_detection_block(&out, &raw));

    // The check could not run; what was found before stays listed.
    let store = store_at(clock("operator"));
    detect(&store, &n, &without_v4, None, clock("operator")).await;
    let out = detect_positions(&DetectionInputs { store: &store, http: &Client::new(), rpc_url: None, wallet: Some(OPERATOR), config_positions: &without_v4.positions, prev: None, now: clock("operator") + chrono::Duration::seconds(300), read_budget: None }).await;
    add("unavailable", position_detection_block(&out, &raw));

    let actual = format!("{}\n", serde_json::to_string_pretty(&Value::Object(doc.clone())).unwrap());
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/position_detect/golden_position_detect.json");
    if std::env::var("NOCKNOCK_BLESS_GOLDEN").is_ok() {
        std::fs::write(path, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(path).expect("testdata/position_detect/golden_position_detect.json is checked in");
    assert!(actual == expected, "the position detection block or its text changed; re-bless and review {path}");

    // What the text must say in each case.
    let text = |name: &str| doc[name]["text"].as_str().unwrap().to_string();
    assert_eq!(
        text("nothing_found"),
        "• Positions found on-chain (ok): checked. Every Uniswap v4 position NFT sent to the wallet was read from the transfer logs, from the first block to this cycle's block. None is found that is not already counted. Not listed: 1 in config.yaml, 37 not on the token whitelist, 5 empty.\n"
    );
    let found = text("found_not_counted");
    assert!(found.contains("   ! Found on-chain, NOT counted in any total: 1 position worth $240.61.\n"), "{found}");
    assert!(found.contains("     - v4 USDC/NOCK 3.00% #3163450, ticks 262200..282300 ($0.00550148 to $0.04105518 per NOCK). It holds 120.02 USDC + 8114.3626 NOCK, worth $240.61. Not tracked by itself: no ladder rung is suggested now to match it against. One tap in the dashboard: TRACK AS CAP RUNG, TRACK AS NEAR RUNG, TRACK AS DEEP RUNG, TRACK AS LP, NOT NOW.\n"), "{found}");
    assert!(text("dismissed").contains("   - Set aside by you, not counted: 1 position worth $240.61.\n"));
    assert!(text("dismissed").contains("RESTORE puts it back on offer.\n"));
    assert!(text("tracked_by_itself").contains("   ✓ Tracked from the chain: NFT #3163450 as a ladder (rung not set), tracked by itself: it matched a suggested rung (mint 0x6cdabada…). Counted in the totals.\n"), "{}", text("tracked_by_itself"));
    assert!(text("tracked_by_tap").contains("NFT #3163450 as a ladder (deep rung), tracked by your tap (mint 0x6cdabada…). Counted in the totals.\n"), "{}", text("tracked_by_tap"));
    assert!(text("partial").starts_with("• Positions found on-chain (partial): checked in part: transfers before block 52931286 are not read yet"), "{}", text("partial"));
    assert!(text("unavailable").starts_with("• Positions found on-chain (unavailable): New positions could not be checked this cycle (no Alchemy RPC key is configured)"));
    assert!(text("unavailable").contains("per NOCK). It was not read this cycle, so what it holds and its value are not stated. Not tracked by itself:"), "{}", text("unavailable"));
    // No text tells the operator what to do, and none names the fallback of the past.
    for name in doc.keys() {
        let t = text(name);
        for banned in ["you should", "we recommend", "transaction hash", "deploy.sh", "edit config"] {
            assert!(!t.to_lowercase().contains(banned), "{name}: {banned}: {t}");
        }
    }
    // get_shot_advice carries the block only when it bears on the totals.
    let advice = |name: &str| crate::position_detect_text::with_position_notice("ADVICE", &json!({ "derived_metrics": { "position_detection": doc[name]["position_detection"] } }));
    assert_eq!(advice("nothing_found"), "ADVICE");
    assert_eq!(advice("tracked_by_itself"), "ADVICE");
    assert!(advice("found_not_counted").starts_with("ADVICE\n• Positions found on-chain (ok)") && advice("found_not_counted").contains("NOT counted in any total"));
    assert!(advice("unavailable").starts_with("ADVICE\n• Positions found on-chain (unavailable)"));
    assert!(advice("partial").contains("(partial)"));
    assert_eq!(crate::position_detect_text::with_position_notice("ADVICE", &json!({ "derived_metrics": {} })), "ADVICE");
    assert_eq!(crate::position_detect_text::position_detection_lines(&json!({ "derived_metrics": {} })), "", "an older snapshot prints no line");
}

#[tokio::test]
async fn test_the_scan_block_is_out_of_the_state_hash_and_the_block_is_absent_from_fixture_snapshots() {
    let n = node("operator", vec![wide_logs_swap("operator")]).await;
    let store = store_at(clock("operator"));
    let cfg = book();
    let out = detect(&store, &n, &cfg, None, clock("operator")).await;
    let raw = read_cycle(&store, &n, &cfg).await.unwrap();
    let block = position_detection_block(&out, &raw);
    // A cycle with config.yaml's positions only and no detection block serialises no
    // `position_detection`, no `source` and no `tracked` anything: the golden snapshots
    // of every earlier feature keep their bytes.
    let plain = serde_json::to_value(cycle_data(read_cycle(&store, &n, &cfg).await.unwrap())).unwrap();
    assert!(plain["derived_metrics"].get("position_detection").is_none());
    assert!(plain["positions"].as_array().unwrap().iter().all(|p| p.get("source").is_none()));
    assert!(plain.get("closed_positions").is_none() && plain.get("ladders").is_none());
    for absent in ["tracked_report", "position_sources", "\"tracked\""] {
        assert!(!plain.to_string().contains(absent), "{absent}");
    }
    // With the block attached, the head it was checked at and the check time do not
    // enter the state hash: a quiet chain stores one snapshot row, not one per cycle.
    let mut a = cycle_data(read_cycle(&store, &n, &cfg).await.unwrap());
    let mut b = a.clone();
    a.derived_metrics.position_detection = Some(block.clone());
    let mut later = block.clone();
    later["checked_at_block"] = json!(58_647_785);
    later["last_checked_at"] = json!("2026-11-17T12:52:16+00:00");
    b.derived_metrics.position_detection = Some(later);
    assert_eq!(compute_state_hash(&a), compute_state_hash(&b));
    // What it found does enter it.
    let mut c = a.clone();
    c.derived_metrics.position_detection.as_mut().unwrap()["status"] = json!("partial");
    assert_ne!(compute_state_hash(&a), compute_state_hash(&c));
    // The cycle attaches it where the suggestion can read it, before the suggestion.
    let ingest = include_str!("ingest.rs");
    let (detect_at, read_at) = (ingest.find("detect_positions_for_cycle(state).await").unwrap(), ingest.find("fetch_raw_data(state).await?").unwrap());
    assert!(detect_at < read_at, "detection runs before the position reads, so a deposit is counted in the cycle that finds it");
    let (attach, suggest) = (ingest.find("derived_metrics.position_detection = Some(position_detection)").unwrap(), ingest.find("attach_ladder_suggestion(&mut snapshot_data").unwrap());
    assert!(attach < suggest);
    assert_eq!(SNAPSHOT_SPEC_VERSION, "2.10.0", "additive: detection's 2.9.0 additions ship in 2.10.0 with the ladder shape");
}

#[test]
fn test_the_specs_document_the_block_and_the_five_routes() {
    let api: serde_yaml::Value = serde_yaml::from_str(include_str!("../../specs/core/api.yaml")).expect("api.yaml parses");
    for (path, method) in [
        ("/positions/found", "get"),
        ("/positions/track", "post"),
        ("/positions/untrack", "post"),
        ("/positions/dismiss", "post"),
        ("/positions/restore", "post"),
    ] {
        assert!(api["paths"][path][method].is_mapping(), "specs/core/api.yaml has {method} {path}");
        // The worker routes it under /v1, inside the cross-site guard.
        assert!(include_str!("api.rs").contains(&format!("\"/v1{path}\"")), "{path} is routed");
    }
    // Every write takes a JSON body with exactly the fields the handler reads.
    let required = |path: &str| -> Vec<String> {
        api["paths"][path]["post"]["requestBody"]["content"]["application/json"]["schema"]["required"]
            .as_sequence()
            .unwrap_or_else(|| panic!("{path} declares its required body fields"))
            .iter()
            .map(|v| v.as_str().unwrap().to_string())
            .collect()
    };
    assert_eq!(required("/positions/track"), vec!["nft_id", "as"]);
    assert_eq!(required("/positions/untrack"), vec!["position_id"]);
    assert_eq!(required("/positions/dismiss"), vec!["nft_id"]);
    assert_eq!(required("/positions/restore"), vec!["nft_id"]);
    let api_text = include_str!("../../specs/core/api.yaml");
    for code in ["UNKNOWN_CHOICE", "CONFIG_POSITION", "NOT_ELIGIBLE", "CHOICE_NOT_OFFERED", "NOT_OWNED", "EMPTY", "CHAIN_UNAVAILABLE", "NOT_TRACKED", "NOT_AN_OFFER", "TRACKED_POSITIONS_UNAVAILABLE"] {
        assert!(api_text.contains(code), "specs/core/api.yaml names {code}");
        assert!(include_str!("position_detect.rs").contains(code) || include_str!("claims.rs").contains(code), "{code} is an error the worker returns");
    }

    let snap_text = include_str!("../../specs/core/snapshot.yaml");
    let snap: serde_yaml::Value = serde_yaml::from_str(snap_text).expect("snapshot.yaml parses");
    assert_eq!(snap["info"]["version"].as_str(), Some("2.10.0"), "additive: detection's 2.9.0 notes ship in 2.10.0");
    let prop = &snap["components"]["schemas"]["DerivedMetrics"]["properties"]["position_detection"];
    assert!(prop.is_mapping(), "DerivedMetrics.position_detection is in the schema");
    assert_eq!(prop["nullable"].as_bool(), Some(true));
    // Every key the worker's block carries is named in the spec, and so is every status,
    // tracked state and choice.
    let golden: Value = serde_json::from_str(include_str!("../testdata/position_detect/golden_position_detect.json")).unwrap();
    let block = &golden["found_not_counted"]["position_detection"];
    for key in block.as_object().unwrap().keys() {
        assert!(prop["properties"].get(key.as_str()).is_some(), "the schema lists position_detection.{key}");
    }
    for key in block["found_not_counted"]["positions"][0].as_object().unwrap().keys().chain(golden["tracked_by_itself"]["position_detection"]["tracked"][0].as_object().unwrap().keys()) {
        if !["protocol", "pool_id", "pool_label", "tick_lower", "tick_upper", "bounds_usd", "in_range", "holds", "mint_tx", "minted_by", "received_from", "config_id"].contains(&key.as_str()) {
            assert!(snap_text.contains(key.as_str()), "specs/core/snapshot.yaml names {key}");
        }
    }
    for word in [
        "ok | partial |", "unavailable | not_run", "counted | empty | closed | not_owned | shadowed_by_config | quarantined |",
        "ladder:<rung> | lp | dismiss", "detected | confirmed", "found_untracked", "tracked_automatically",
        "Position.source, LadderReading.source and", "ClosedPosition.source", "never a partial sum", "left out of the state hash",
    ] {
        assert!(snap_text.contains(word), "specs/core/snapshot.yaml says: {word}");
    }
    // The fallback of the past is named only as what an older snapshot carried.
    assert!(snap_text.contains("manual_config in a snapshot stored before"));
}

// ------------------------------------------------------------------
// Second review of the bottle: dust cannot time the check out
// ------------------------------------------------------------------

#[test]
fn test_the_rereads_rotate_within_their_cap() {
    let ids: Vec<u64> = (1..=10).collect();
    assert_eq!(refresh_batch(&ids, 0, 20), (ids.clone(), 0), "under the cap: all, nothing deferred");
    // Under the cap it still starts at the cursor: a cycle cut short resumes where it stopped.
    assert_eq!(refresh_batch(&ids, 3, 20), (vec![4, 5, 6, 7, 8, 9, 10, 1, 2, 3], 0));
    assert_eq!(refresh_batch(&ids, 0, 4), (vec![1, 2, 3, 4], 6));
    assert_eq!(refresh_batch(&ids, 8, 4), (vec![9, 10, 1, 2], 6), "it wraps");
    // Cycle after cycle, every one is read in turn.
    let mut seen: HashSet<u64> = HashSet::new();
    for cycle in 0..3 {
        seen.extend(refresh_batch(&ids, cycle * 4, 4).0);
    }
    assert_eq!(seen.len(), 10);
}

#[tokio::test]
async fn test_dust_in_a_whitelisted_pool_is_reread_at_most_the_cap_per_cycle_and_stated() {
    // The operator's wallet, then 300 dust NFTs in a whitelisted pool sent to it (cheap on
    // Base). Before the cap every one was re-read every cycle, sequentially, past the 60 s
    // timeout: the whole check failed and no rung mint was tracked again.
    let dust: Vec<u64> = (900_000_000..900_000_300).collect();
    // The chain answers for them: each empty, and the wallet holds 43 + 300 NFTs.
    let mut swaps = vec![wide_logs_swap("operator")];
    let empty = format!("0x{}", "0".repeat(64));
    swaps.extend(dust.iter().map(|id| (key_liquidity(*id), Swap::Json(json!({ "result": empty })))));
    let balance_key = call_key(UNISWAP_V4_POSM_BASE, &format!("{ERC721_BALANCE_OF}{:0>64}", OPERATOR.trim_start_matches("0x")));
    swaps.push((balance_key, Swap::Json(json!({ "result": format!("0x{:064x}", 43 + dust.len()) }))));
    let n = node("operator", swaps).await;
    let store = store_at(clock("operator"));
    // The first cycle reads the history (the dust reached the wallet after it, here).
    detect(&store, &n, &book(), None, clock("operator")).await;
    for id in &dust {
        let mut row = candidate(*id);
        row.has_liquidity = Some(false);
        store.upsert_candidate(&row).await.unwrap();
    }
    let mut asked: HashSet<u64> = HashSet::new();
    for _ in 0..8 {
        n.forget();
        let out = detect(&store, &n, &book(), None, clock("operator")).await;
        // The rest waits, and the status says so: never "nothing found", never a failure.
        assert_eq!(out.status, "partial", "{:?}", out.reason);
        let reason = out.reason.clone().unwrap();
        assert!(reason.contains(&format!("untracked position NFT(s) the wallet holds are read again in later cycles (at most {POSITION_REFRESH_MAX_PER_CYCLE} a cycle, in turn)")), "{reason}");
        // The header, the NFT count and at most the cap of re-reads.
        assert!(n.count("eth_call") <= POSITION_REFRESH_MAX_PER_CYCLE + 1, "{}", n.count("eth_call"));
        let keys = n.keys();
        asked.extend(dust.iter().filter(|id| keys.iter().any(|k| k.contains(&format!("{:064x}", id)))));
    }
    assert_eq!(asked.len(), dust.len(), "every dust NFT is read in turn");
}

/// A node answering the operator's recorded chain plus `dust` empty NFTs the wallet holds.
async fn node_with_dust(dust: &[u64]) -> Node {
    let mut swaps = vec![wide_logs_swap("operator")];
    let empty = format!("0x{}", "0".repeat(64));
    swaps.extend(dust.iter().map(|id| (key_liquidity(*id), Swap::Json(json!({ "result": empty })))));
    let balance_key = call_key(UNISWAP_V4_POSM_BASE, &format!("{ERC721_BALANCE_OF}{:0>64}", OPERATOR.trim_start_matches("0x")));
    swaps.push((balance_key, Swap::Json(json!({ "result": format!("0x{:064x}", 43 + dust.len()) }))));
    node("operator", swaps).await
}

#[tokio::test]
async fn test_a_scan_cut_short_by_its_budget_resumes_where_it_stopped() {
    // Third review of the bottle: the cursor moved 40 whatever was read, so a scan cut
    // short re-read the same lowest ids and the rest starved. The budget is injected:
    // 13 reads a cycle, for 30 dust NFTs (under the cap of 40) and the 5 empty ones.
    let dust: Vec<u64> = (900_000_000..900_000_030).collect();
    let n = node_with_dust(&dust).await;
    let store = store_at(clock("operator"));
    let cfg = book_without_v4();
    let http = Client::new();
    let inputs = |budget: Option<usize>| DetectionInputs {
        store: &store,
        http: &http,
        rpc_url: Some(&n.url),
        wallet: Some(OPERATOR),
        config_positions: &cfg.positions,
        prev: None,
        now: clock("operator"),
        read_budget: budget,
    };
    detect_positions(&inputs(None)).await;
    for id in &dust {
        let mut row = candidate(*id);
        row.has_liquidity = Some(false);
        store.upsert_candidate(&row).await.unwrap();
    }
    let mut asked: HashSet<u64> = HashSet::new();
    let mut found_read = 0;
    let mut cursors: Vec<i64> = Vec::new();
    for _ in 0..3 {
        n.forget();
        let out = detect_positions(&inputs(Some(13))).await;
        assert_eq!(out.status, "partial");
        assert!(out.reason.clone().unwrap().contains("the check stopped starting reads at its 40-second budget; the rest is read next cycle"), "{:?}", out.reason);
        let keys = n.keys();
        asked.extend(dust.iter().filter(|id| keys.iter().any(|k| k.contains(&format!("{:064x}", id)))));
        // The found position (the wallet's own v4 range, not in config.yaml) is read in its
        // turn: its liquidity is cached by its re-read, so step 4 never cuts it.
        if out.found.iter().any(|f| f.row.nft_id == 3163450 && f.read.is_some()) {
            found_read += 1;
        }
        cursors.push(store.scan_state(OPERATOR, TRACKED_PROTOCOL_V4, TRACKED_NETWORK_BASE).await.unwrap().refresh_cursor);
    }
    assert_eq!(asked.len(), dust.len(), "every one read within three cycles of 13");
    assert!(found_read >= 1, "the found position is read in its turn");
    // The stored cursor moved by what was read, not by the cap.
    assert!(cursors.windows(2).all(|w| w[0] != w[1]), "{cursors:?}");
}

#[test]
fn test_two_mints_of_one_rung_in_one_cycle_track_one_and_offer_the_other() {
    // Third review of the bottle: the bottom-tick guard ran on the drift path only, so a
    // drifted near (lower id) and an exact near were both tracked in one cycle.
    let near = [rung(273900, 275400, Some("near")), rung(275400, 276300, Some("deep"))];
    let exact = CandidateRow { nft_id: 9000004, tick_lower: Some(273900), tick_upper: Some(275400), ..candidate(9000004) };
    let drifted = CandidateRow { nft_id: 9000003, tick_lower: Some(273600), tick_upper: Some(275400), ..candidate(9000003) };
    let as_range = |c: &CandidateRow| TrackedRange { position_id: v4_position_key("base", c.nft_id), pool_id: POOL_3PCT.to_string(), tick_lower: c.tick_lower.unwrap(), tick_upper: c.tick_upper.unwrap() };
    // The loop decides the exact match first, whatever the ids.
    let mut order = vec![&drifted, &exact];
    order.sort_by_key(|c| (!is_exact_match(c, &near), c.nft_id));
    assert_eq!(order[0].nft_id, 9000004);
    assert_eq!(decide(&exact, OPERATOR, &near, &[]), Decision::Track { rung: Some("near".into()) });
    match decide(&drifted, OPERATOR, &near, &[as_range(&exact)]) {
        Decision::Offer { why_not_automatic, .. } => assert_eq!(why_not_automatic, vec!["its ticks 273600..275400 are not a suggested rung's (273900..275400, 275400..276300)".to_string()]),
        other => panic!("{other:?}"),
    }
    // And when a drifted one is already tracked (an earlier cycle), the exact one is
    // offered, with the reason.
    match decide(&exact, OPERATOR, &near, &[as_range(&drifted)]) {
        Decision::Offer { why_not_automatic, .. } => {
            assert_eq!(why_not_automatic, vec!["a ladder in this pool already holds this rung's bottom tick (uniswap-v4:base:9000003): this would be a second one".to_string()])
        }
        other => panic!("{other:?}"),
    }
}
