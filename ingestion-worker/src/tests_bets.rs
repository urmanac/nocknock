//! WETH/NOCK bets (issue #181): `role: bet`, the gauge detection and the bet card.
//!
//! The chain answers are real. `testdata/bets/chain_fixture.json` was recorded from
//! https://base-rpc.publicnode.com on 2026-11-21 by the requests bet_detect.rs makes:
//! - `staked`: block 57671680, when the operator's WETH/NOCK deposit of 2026-11-14 (NFT
//!   #2914490, tx 0x80277e2b…93ddb, minted to the Aero router and staked in the CL200 gauge
//!   in one transaction) was staked;
//! - `live`: the chain on 2026-11-21 with no bet staked (tick 70627).
//! The card's snapshot is the operator's live snapshot of 2026-11-21 (the ladder fixture),
//! with today's wallet ETH, WETH and ETH/USD read at the `live` block.
use super::*;
use serde_json::Value;

const CHAIN: &str = include_str!("../testdata/bets/chain_fixture.json");
const WALLET: &str = "0xee114cc97ec5adeb34ba5645e315b497d8487a45";
/// The pool as the operator would paste it (checksummed), for the link.
const POOL_CONFIGURED: &str = "0x957256c98d61A714081DE2EE70ae3A96c9b40874";
const POOL: &str = "0x957256c98d61a714081de2ee70ae3a96c9b40874";
const GAUGE: &str = "0x3bf70c880adc8370917994ab0df68e763b7a60fd";
const MINT_TX: &str = "0x80277e2bc5f351b66051967b44a47d22edd2704ad690cc5dc1f47e878cc93ddb";

fn chain() -> Value {
    serde_json::from_str(CHAIN).expect("testdata/bets/chain_fixture.json parses")
}

fn scenario(name: &str) -> Value {
    chain()[name].clone()
}

fn s<'a>(v: &'a Value, ptr: &str) -> &'a str {
    v.pointer(ptr).and_then(|x| x.as_str()).unwrap_or_else(|| panic!("{ptr} missing in {v:#}"))
}

/// The operator's decisions of 2026-11-21 as settings: $40, two 200-tick steps each side,
/// a 0.005 ETH gas floor, the CL200 pool. Written by the operator, never defaults.
fn operator_bets() -> NockBetsConfig {
    NockBetsConfig { pool: Some(POOL_CONFIGURED.to_string()), bet_usd: Some(40.0), width_ticks: Some(400), width_pct: None, gas_floor_eth: Some(0.005) }
}

fn profile_with(b: Option<NockBetsConfig>) -> OperatorProfile {
    OperatorProfile { treasury_floor_nock: 100_000.0, lp_floor_tolerance_bps: 1000, nock_bets: b, ..Default::default() }
}

/// The recorded answer of one eth_call at the scenario's block, hex without 0x.
fn answer(scn: &Value, to: &str, data: &str) -> Option<String> {
    let key = format!("eth_call|{}|{}|{}", to.to_lowercase(), data.to_lowercase(), s(scn, "/block_hex"));
    scn["responses"][&key]["result"].as_str().map(|h| h.trim_start_matches("0x").to_string())
}

fn pool_answers(scn: &Value) -> BetPoolAnswers {
    let a = |d: &str| answer(scn, POOL, d);
    let mut out = BetPoolAnswers {
        token0: a(CL_POOL_TOKEN0),
        token1: a(CL_POOL_TOKEN1),
        tick_spacing: a(CL_POOL_TICK_SPACING),
        fee: a(CL_POOL_FEE),
        gauge: a(CL_POOL_GAUGE),
        nft: a(CL_POOL_NFT),
        factory: a(CL_POOL_FACTORY),
        slot0: a(V3_SLOT0),
        liquidity: a(V3_LIQUIDITY),
        ..Default::default()
    };
    let w = |h: &Option<String>| h.clone().unwrap();
    let data = format!("{CL_FACTORY_GET_POOL}{}{}{}", w(&out.token0), w(&out.token1), w(&out.tick_spacing));
    out.factory_pool = answer(scn, AERO_CL_FACTORY_BASE, &data);
    out.gauge_pool = answer(scn, GAUGE, CL_GAUGE_POOL);
    out
}

fn live_pool() -> BetPoolRead {
    let scn = scenario("live");
    bet_pool_from_answers(POOL_CONFIGURED, scn["number"].as_i64().unwrap(), &pool_answers(&scn)).expect("the live pool is the bet pool")
}

fn receipt() -> Value {
    let scn = scenario("staked");
    scn["responses"][format!("eth_getTransactionReceipt|{MINT_TX}")]["result"].clone()
}

/// The live snapshot with today's wallet ETH and WETH and ETH/USD (read at the `live` block).
fn live_data() -> SnapshotData {
    let mut data = crate::tests_ladder_suggest::live().data;
    data.derived_metrics.global_status.eth_price_usd = Some("1683.07".to_string());
    for w in data.wallet_balances.iter_mut() {
        match w.token_symbol.as_str() {
            "ETH" => w.amount = "0.016378".to_string(),
            "WETH" => w.amount = "0.002177".to_string(),
            _ => {}
        }
    }
    data
}

fn detection_live() -> BetDetection {
    BetDetection { status: "ok".to_string(), block: Some(live_pool().block), pool: Some(live_pool()), staked: Some(vec![]), ..Default::default() }
}

fn card(data: &SnapshotData, profile: &OperatorProfile, det: &BetDetection) -> Value {
    bet_suggestion(data, profile, profile.nock_bets.as_ref().unwrap(), det)
}

fn text(card: &Value, health: Option<&str>) -> String {
    crate::bet_suggest_text::bet_suggestion_lines(&json!({ "derived_metrics": { "bet_suggestion": card } }), health)
}

/// The bet pool's row, as a cycle with a tracked bet lists it in `pool_mechanics`.
fn with_bet_pool(data: &mut SnapshotData) {
    let p = live_pool();
    data.pool_mechanics.push(PoolMechanic {
        pool_id: POOL.to_string(),
        platform: "aerodrome-cl".to_string(),
        pool_type: "concentrated".to_string(),
        volatile_assets: vec![],
        liquidity_usd: "0.00".to_string(),
        fee_accrued_usd: "0.00".to_string(),
        fee_tier_bps: 97,
        tick_current: Some(p.tick),
        sqrt_price_x96: Some(p.sqrt_price_x96.clone()),
        active_liquidity_raw: Some(p.liquidity.to_string()),
    });
}

/// A tracked bet on the live pool, as the cycle would list it.
fn bet_position(id: u64, lo: i32, hi: i32, in_range: bool, weth: &str, nock: &str, usd: &str) -> Position {
    let mut p = live_data().positions.into_iter().find(|p| p.protocol == "aerodrome-cl").unwrap();
    p.position_id = format!("aerodrome-cl:base:{id}");
    p.nft_id = id;
    p.pool_id = POOL.to_string();
    p.token0 = token_info(WETH_BASE).unwrap();
    p.token1 = token_info(NOCK_BASE).unwrap();
    p.lower_tick = lo;
    p.upper_tick = hi;
    p.in_range = in_range;
    p.amount0 = weth.to_string();
    p.amount1 = nock.to_string();
    p.amount_usd = usd.to_string();
    p.role = Some(BET_ROLE.to_string());
    p.source = Some(POSITION_SOURCE_TRACKED.to_string());
    p.unclaimed_aero = Some("0.412345".to_string());
    p.unclaimed_aero_usd = Some("0.15".to_string());
    p.fees_note = Some(AERO_CL_STAKED_FEES_NOTE.to_string());
    p.staked_in_gauge = Some(GAUGE.to_string());
    p
}

// ------------------------------------------------------------------
// Settings
// ------------------------------------------------------------------

#[test]
fn test_every_bet_setting_has_no_default_and_each_missing_one_is_named() {
    let empty = bet_settings(&NockBetsConfig::default());
    assert_eq!(empty.missing, vec![BET_KEY_POOL, BET_KEY_USD, BET_KEY_WIDTH, BET_KEY_GAS]);
    assert!(empty.invalid.is_empty() && empty.params.is_none());
    let mut c = operator_bets();
    c.gas_floor_eth = None;
    assert_eq!(bet_settings(&c).missing, vec![BET_KEY_GAS]);
    let ok = bet_settings(&operator_bets()).params.unwrap();
    assert_eq!((ok.bet_usd, ok.width_ticks, ok.gas_floor_eth), (40.0, 400, 0.005));
    // The profile's own default has no bets at all.
    assert!(OperatorProfile::default().nock_bets.is_none());
}

#[test]
fn test_invalid_bet_settings_are_stated_and_size_nothing() {
    let cases: Vec<(NockBetsConfig, &str)> = vec![
        (NockBetsConfig { pool: Some("0x9572".to_string()), ..operator_bets() }, "must be a pool address"),
        (NockBetsConfig { bet_usd: Some(0.0), ..operator_bets() }, "bet_usd must be a USD amount above 0"),
        (NockBetsConfig { bet_usd: Some(f64::NAN), ..operator_bets() }, "bet_usd must be a USD amount above 0"),
        (NockBetsConfig { width_pct: Some(4.0), ..operator_bets() }, "sets both width_ticks and width_pct"),
        (NockBetsConfig { width_ticks: Some(0), ..operator_bets() }, "width_ticks must be above 0"),
        (NockBetsConfig { width_ticks: None, width_pct: Some(100.0), ..operator_bets() }, "width_pct must be above 0 and below 100"),
        (NockBetsConfig { gas_floor_eth: Some(-0.1), ..operator_bets() }, "gas_floor_eth must be an ETH amount of 0 or more"),
    ];
    for (c, want) in cases {
        let st = bet_settings(&c);
        assert!(st.params.is_none(), "{c:?}");
        assert!(st.invalid.iter().any(|i| i.contains(want)), "{:?} lacks {want}", st.invalid);
        let v = card(&live_data(), &profile_with(Some(c.clone())), &detection_live());
        assert_eq!(s(&v, "/status"), "invalid");
        assert_eq!(v["suggestion"], Value::Null);
        assert_eq!(v["shown"], json!(false));
    }
}

#[test]
fn test_a_percent_width_is_its_ticks_and_the_range_rounds_to_the_spacing() {
    // Aero's own +-4% preset: ln(1.04)/ln(1.0001) = 392 ticks each side.
    assert_eq!(pct_to_ticks(4.0), 392);
    let c = NockBetsConfig { width_ticks: None, width_pct: Some(4.0), ..operator_bets() };
    assert_eq!(bet_settings(&c).params.unwrap().width_ticks, 392);
    // Spot tick 70627, CL200: both widths give two 200-tick steps each side.
    assert_eq!(bet_range(70627, 200, 400), (70200, 71000));
    assert_eq!(bet_range(70627, 200, 392), (70200, 71000));
    // A reach smaller than half a step still holds the price: the band containing it.
    assert_eq!(bet_range(70627, 200, 10), (70600, 70800));
    // Negative ticks round the same way.
    assert_eq!(bet_range(-70627, 200, 400), (-71000, -70200));
    // The price is always inside [lower, upper).
    for tick in [70600, 70601, 70699, 70799, -1, 0, 1] {
        for w in [1, 99, 100, 200, 400, 1000] {
            let (lo, hi) = bet_range(tick, 200, w);
            assert!(lo <= tick && tick < hi && lo % 200 == 0 && hi % 200 == 0, "{tick} {w}: {lo}..{hi}");
        }
    }
}

// ------------------------------------------------------------------
// The pool read
// ------------------------------------------------------------------

#[test]
fn test_the_recorded_pool_is_the_bet_pool() {
    let p = live_pool();
    assert_eq!(p.pool, POOL);
    assert_eq!(p.configured, POOL_CONFIGURED);
    assert_eq!((p.token0.as_str(), p.token1.as_str()), ("0x4200000000000000000000000000000000000006", "0x9b5e262cf9bb04869ab40b19af91d2dc85761722"));
    assert_eq!((p.tick_spacing, p.fee_pips, p.tick), (200, 9700, 70627));
    assert_eq!(p.gauge, GAUGE);
    assert_eq!(p.liquidity, 273693898720236409447);
}

#[test]
fn test_a_pool_that_is_not_the_aero_weth_nock_pool_is_refused_with_its_reason() {
    let scn = scenario("live");
    let base = pool_answers(&scn);
    let word = |a: &str| format!("{:0>64}", a.trim_start_matches("0x"));
    let cases: Vec<(BetPoolAnswers, &str)> = vec![
        (BetPoolAnswers { nft: Some(word("0x827922686190790b37229fd06084350e74485b72")), ..base.clone() }, "not the Aero CL position manager"),
        (BetPoolAnswers { nft: None, ..base.clone() }, "nft() could not be read"),
        (BetPoolAnswers { factory: Some(word("0x5e7bb104d84c7cb9b682aac2f3d509f5f406809a")), ..base.clone() }, "not the Aero CL factory"),
        (BetPoolAnswers { token1: Some(word(USDC_BASE)), ..base.clone() }, "not a WETH/NOCK pool"),
        (BetPoolAnswers { factory_pool: Some(word("0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a")), ..base.clone() }, "getPool"),
        (BetPoolAnswers { gauge: Some(word(NATIVE_ETH)), ..base.clone() }, "no gauge"),
        (BetPoolAnswers { gauge_pool: Some(word(USDC_BASE)), ..base.clone() }, "names another pool"),
        (BetPoolAnswers { liquidity: Some(word("0")), ..base.clone() }, "price is frozen"),
    ];
    for (a, want) in cases {
        let e = bet_pool_from_answers(POOL, 1, &a).unwrap_err();
        assert!(e.contains(want), "{e} lacks {want}");
    }
}

#[test]
fn test_staked_values_decode_strictly() {
    assert_eq!(decode_staked_values(&answer(&scenario("live"), GAUGE, &format!("{CL_GAUGE_STAKED_VALUES}{:0>64}", &WALLET[2..])).unwrap()).unwrap(), Vec::<u64>::new());
    assert_eq!(decode_staked_values(&answer(&scenario("staked"), GAUGE, &format!("{CL_GAUGE_STAKED_VALUES}{:0>64}", &WALLET[2..])).unwrap()).unwrap(), vec![2914490]);
    assert!(decode_staked_values("").is_err());
    assert!(decode_staked_values(&format!("{:064x}{:064x}", 32, 2)).is_err());
}

// ------------------------------------------------------------------
// The proof
// ------------------------------------------------------------------

#[test]
fn test_the_real_staked_deposit_proves_the_wallet_made_it() {
    let m = bet_mint_proof(&receipt(), WALLET, 2914490, POOL, GAUGE).unwrap();
    assert_eq!(m.tx, MINT_TX);
    assert_eq!(m.block, 57425824);
    assert_eq!((m.tick_lower, m.tick_upper), (73400, 74800));
    assert_eq!(m.liquidity, 0x322f4dd43fbc87e73);
    // 0.081995 ETH + 2,755 NOCK went in (ONCHAIN_INTEGRATION.md §7).
    assert_eq!(m.amount0, 0x1234dac5768f0c0);
    assert_eq!(format!("{:.6}", m.amount0 as f64 / 1e18), "0.081995");
    assert_eq!(format!("{:.0}", m.amount1 as f64 / 1e16), "2755");
}

#[test]
fn test_the_proof_is_never_loosened() {
    let r = receipt();
    let refuse = |r: &Value, wallet: &str, id: u64, pool: &str, gauge: &str| bet_mint_proof(r, wallet, id, pool, gauge).unwrap_err();
    // Another account's view of the same receipt: not its deposit.
    assert!(refuse(&r, "0x4598b3ba7d6714091b18da5d110b64f56e94e826", 2914490, POOL, GAUGE).contains("not by your wallet"));
    // Another id, another pool, another gauge.
    assert!(refuse(&r, WALLET, 2914491, POOL, GAUGE).contains("not minted to the Aero router"));
    assert!(refuse(&r, WALLET, 2914490, "0x71a4a6f6ea5c26ce486b7ed0d8723a897ef5ad9a", GAUGE).contains("did not mint exactly one range"));
    assert!(refuse(&r, WALLET, 2914490, POOL, "0xc82782ee66d82c4d19db0429652091a085083526").contains("did not move it to the bet pool's gauge"));
    // Minted to the wallet itself (an unstaked mint staked later): not the staked path.
    let mut minted_to_wallet = r.clone();
    for l in minted_to_wallet["logs"].as_array_mut().unwrap() {
        let t = l["topics"].as_array().map(|t| t.iter().map(|x| x.as_str().unwrap().to_string()).collect::<Vec<_>>()).unwrap_or_default();
        if t.len() == 4 && t[1] == format!("0x{:064x}", 0) && l["address"] == json!(AERO_CL_POSITION_MANAGER_BASE) {
            l["topics"][2] = json!(format!("0x{:0>64}", &WALLET[2..]));
        }
    }
    assert!(refuse(&minted_to_wallet, WALLET, 2914490, POOL, GAUGE).contains("not minted to the Aero router"));
    // A failed transaction.
    let mut failed = r.clone();
    failed["status"] = json!("0x0");
    assert!(refuse(&failed, WALLET, 2914490, POOL, GAUGE).contains("failed"));
    // The gauge's record of the deposit for the wallet is required.
    let mut no_deposit = r.clone();
    no_deposit["logs"].as_array_mut().unwrap().retain(|l| l["topics"][0] != json!(CL_GAUGE_DEPOSIT_TOPIC));
    assert!(refuse(&no_deposit, WALLET, 2914490, POOL, GAUGE).contains("did not record exactly one deposit"));
}

// ------------------------------------------------------------------
// Detection against a local node replaying the recorded answers
// ------------------------------------------------------------------

fn request_key(req: &Value) -> String {
    let lower = |v: &Value| v.as_str().unwrap_or_default().to_lowercase();
    match req["method"].as_str().unwrap_or_default() {
        "eth_call" => format!("eth_call|{}|{}|{}", lower(&req["params"][0]["to"]), lower(&req["params"][0]["data"]), lower(&req["params"][1])),
        "eth_getLogs" => {
            let f = &req["params"][0];
            let topics: Vec<String> = f["topics"].as_array().unwrap().iter().map(|t| if t.is_null() { "null".to_string() } else { lower(t) }).collect();
            format!("eth_getLogs|{}|{}|{}|{}", lower(&f["address"]), lower(&f["fromBlock"]), lower(&f["toBlock"]), topics.join(","))
        }
        "eth_getTransactionReceipt" => format!("eth_getTransactionReceipt|{}", lower(&req["params"][0])),
        other => other.to_string(),
    }
}

/// A local JSON-RPC node replaying one scenario; `swaps` replace recorded answers by key.
async fn node(name: &str, swaps: Vec<(String, Value)>) -> String {
    let scn = scenario(name);
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.expect("a local port");
    let url = format!("http://{}", listener.local_addr().unwrap());
    let handler = move |Json(req): Json<Value>| {
        let (scn, swaps) = (scn.clone(), swaps.clone());
        async move {
            let key = request_key(&req);
            let answer = if let Some((_, v)) = swaps.iter().find(|(k, _)| k == &key) {
                v.clone()
            } else if req["method"] == json!("eth_getBlockByNumber") {
                let n = scn["number"].as_i64().unwrap() + POSITION_HEAD_LAG_BLOCKS;
                json!({ "result": { "number": format!("0x{n:x}"), "timestamp": format!("0x{:x}", scn["block_timestamp"].as_i64().unwrap() + 10) } })
            } else {
                scn["responses"].get(&key).cloned().unwrap_or_else(|| json!({ "error": { "code": -32000, "message": format!("no recorded answer for {key}") } }))
            };
            let mut body = json!({ "jsonrpc": "2.0", "id": req["id"] });
            for (k, v) in answer.as_object().unwrap() {
                body[k] = v.clone();
            }
            Json(body).into_response()
        }
    };
    let app = axum::Router::new().route("/", axum::routing::post(handler));
    tokio::spawn(async move {
        let _ = axum::serve(listener, app).await;
    });
    url
}

async fn detect(store: &PositionStore, url: &str, config: &[PositionConfig]) -> BetDetection {
    let http = Client::new();
    detect_bets(&BetDetectionInputs { store, http: &http, rpc_url: Some(url), wallet: Some(WALLET), pool: Some(POOL_CONFIGURED), config_positions: config, read_budget: None }).await
}

#[tokio::test]
async fn test_a_bet_the_wallet_deposited_and_staked_is_tracked_by_itself() {
    let url = node("staked", vec![]).await;
    let store = PositionStore::mem();
    let d = detect(&store, &url, &[]).await;
    assert_eq!(d.status, "ok", "{d:?}");
    assert_eq!(d.staked, Some(vec![2914490]));
    assert_eq!(d.newly_tracked, vec!["aerodrome-cl:base:2914490".to_string()]);
    assert!(d.found.is_empty(), "{:?}", d.found);
    let rows = store.tracked(WALLET).await.unwrap();
    assert_eq!(rows.len(), 1);
    let r = &rows[0];
    assert_eq!((r.protocol.as_str(), r.role.as_deref(), r.source.as_str()), (AERO_CL_PROTOCOL, Some(BET_ROLE), TRACKED_SOURCE_DETECTED));
    assert_eq!((r.pool_id.as_str(), r.tick_lower, r.tick_upper, r.tick_spacing), (POOL, 73400, 74800, Some(200)));
    assert_eq!((r.deposit_tx.as_deref(), r.mint_block), (Some(MINT_TX), Some(57425824)));
    assert!(validate_tracked_row(r).is_ok());
    // The row reads as an aerodrome-cl config entry of role bet, and round-trips to one.
    let cfg = tracked_position_config(r);
    assert_eq!((cfg.protocol.as_str(), cfg.pool_address.as_deref(), cfg.npm_address.as_deref(), cfg.role.as_deref()), (AERO_CL_PROTOCOL, Some(POOL), Some(AERO_CL_POSITION_MANAGER_BASE), Some(BET_ROLE)));
    assert_eq!(position_key(&cfg), r.position_key);
    assert!(tracked_row_config_entry(r).contains("role: \"bet\""));
    // A second cycle finds it tracked and reads no receipt again.
    let d2 = detect(&store, &url, &[]).await;
    assert!(d2.newly_tracked.is_empty() && d2.found.is_empty());
}

#[tokio::test]
async fn test_a_stake_the_wallet_did_not_deposit_itself_is_offered_never_tracked() {
    // The same answers, except that the deposit was sent by another account.
    let mut r = receipt();
    r["from"] = json!("0x4598b3ba7d6714091b18da5d110b64f56e94e826");
    let url = node("staked", vec![(format!("eth_getTransactionReceipt|{MINT_TX}"), json!({ "result": r }))]).await;
    let store = PositionStore::mem();
    let d = detect(&store, &url, &[]).await;
    assert!(d.newly_tracked.is_empty());
    assert!(store.tracked(WALLET).await.unwrap().is_empty());
    assert_eq!(d.found.len(), 1);
    let f = &d.found[0];
    assert_eq!(f.row.nft_id, 2914490);
    assert!(f.why_not_automatic.iter().any(|w| w.contains("not by your wallet")), "{:?}", f.why_not_automatic);
    assert_eq!(f.choices, vec!["bet".to_string(), "dismiss".to_string()]);
    // The card waits for the tap.
    let mut det = d.clone();
    det.pool = Some(live_pool());
    let v = card(&live_data(), &profile_with(Some(operator_bets())), &det);
    assert_eq!(s(&v, "/status"), "found_untracked");
    assert_eq!(s(&v, "/tier"), "top");
    // Set aside, it is listed as such and is not offered again by itself.
    set_found_choice_for(&store, WALLET, AERO_CL_PROTOCOL, 2914490, true).await.unwrap();
    let d3 = detect(&store, &url, &[]).await;
    assert!(d3.found[0].dismissed);
}

#[tokio::test]
async fn test_a_stake_in_config_is_read_through_config_and_the_live_chain_has_none() {
    let url = node("staked", vec![]).await;
    let store = PositionStore::mem();
    let cfg: PositionConfig = serde_yaml::from_str(&format!(
        "id: bet-1\nprotocol: aerodrome-cl\nnft_id: 2914490\nnpm_address: \"{AERO_CL_POSITION_MANAGER_BASE}\"\npool_address: \"{POOL}\"\nrole: bet\n"
    ))
    .unwrap();
    validate_positions_config(std::slice::from_ref(&cfg)).unwrap();
    let d = detect(&store, &url, &[cfg]).await;
    assert!(d.newly_tracked.is_empty() && d.found.is_empty());
    // Today: the pool is read with no bet staked, and nothing is tracked or found.
    let live = node("live", vec![]).await;
    let d = detect(&store, &live, &[]).await;
    assert_eq!(d.status, "ok");
    assert_eq!(d.staked, Some(vec![]));
    assert_eq!(d.pool.as_ref().map(|p| p.tick), Some(70627));
}

#[tokio::test]
async fn test_detection_never_fails_and_says_why() {
    // No pool configured, no RPC: not run.
    let store = PositionStore::mem();
    let http = Client::new();
    let none = detect_bets(&BetDetectionInputs { store: &store, http: &http, rpc_url: None, wallet: Some(WALLET), pool: Some(POOL), config_positions: &[], read_budget: None }).await;
    assert_eq!(none.status, "not_run");
    // A pool answer that is not the bet pool: unavailable, with the reason.
    let word = format!("0x{:0>64}", "827922686190790b37229fd06084350e74485b72");
    let scn = scenario("live");
    let key = format!("eth_call|{POOL}|{CL_POOL_NFT}|{}", s(&scn, "/block_hex"));
    let url = node("live", vec![(key, json!({ "result": word }))]).await;
    let d = detect(&store, &url, &[]).await;
    assert_eq!(d.status, "unavailable");
    assert!(d.reason.unwrap().contains("not the Aero CL position manager"));
}

// ------------------------------------------------------------------
// role: bet
// ------------------------------------------------------------------

#[test]
fn test_role_bet_is_an_aero_cl_position_that_steers_nothing() {
    let base = format!("id: bet-1\nprotocol: aerodrome-cl\nnft_id: 2914490\nnpm_address: \"{AERO_CL_POSITION_MANAGER_BASE}\"\npool_address: \"{POOL}\"\nrole: bet\n");
    let parse = |extra: &str| -> PositionConfig { serde_yaml::from_str(&format!("{base}{extra}")).unwrap() };
    assert!(validate_positions_config(&[parse("")]).is_ok());
    let refused = |c: PositionConfig| validate_positions_config(&[c]).unwrap_err().to_string();
    let mut v4 = parse("");
    v4.protocol = "uniswap-v4".to_string();
    assert!(refused(v4).contains("a bet is an Aero CL position"));
    assert!(refused(parse("rebuild_weight_bps: 100\n")).contains("never a rebuild template"));
    assert!(refused(parse("is_price_source: true\n")).contains("is_price_source"));
    assert!(refused(parse("rung: near\n")).contains("rung"));
    assert!(refused(parse("deposit_url: \"https://app.aero.xyz\"\n")).contains("deposit_url"));
}

#[test]
fn test_a_bet_is_counted_but_is_not_nock_lp() {
    let mut raw = crate::tests::rebuild_fixture();
    let mut bet = raw.positions.iter().find(|p| is_nock_pair(p)).unwrap().clone();
    bet.position_id = "aerodrome-cl:base:2914490".to_string();
    bet.protocol = AERO_CL_PROTOCOL.to_string();
    bet.token0 = token_info(WETH_BASE).unwrap();
    bet.token1 = token_info(NOCK_BASE).unwrap();
    bet.amount0 = 0.012;
    bet.amount1 = 1400.0;
    raw.positions.push(bet.clone());
    raw.bets.insert(bet.position_id.clone());
    assert!(is_nock_pair(&bet) && is_bet(&raw, &bet) && !is_nock_lp(&raw, &bet));
    // Never a rebuild suggestion nor an exclusion.
    let (sugg, excl) = lp_rebuild_suggestions(&raw, 100_000.0, RebuildFunds { nock: 1e9, floor_tolerance_nock: 0.0, floor_set: true, usdc: 1e9 });
    assert!(sugg.iter().all(|t| t.position_id != bet.position_id));
    assert!(excl.iter().all(|t| t.position_id != bet.position_id));
    // Counted, in its own measured deviation row.
    let profile = crate::tests::operator_profile_2026_11();
    let dm = compute_derived_metrics_with_plan(
        &raw, 0.0, &profile, compute_structural_channels_sync(&[]), &[], &LedgerAggregate::default(), &MarketRegime::default(),
        &PlanContext { cfg: None, fills: PlanFills::default(), ledger_unavailable: false, now: t("2026-11-21T12:00:00Z") },
    );
    let rows = &dm.portfolio_deviation_matrix.allocations;
    assert!(rows.iter().any(|r| r.bucket_name == BET_BUCKET_NAME && r.status == "no_target"), "{rows:?}");
    let mut positions = build_position_snapshots(&raw.positions, raw.spot_price_nock_usd, raw.live_eth_price_usd);
    apply_bet_roles(&mut positions, &raw.bets);
    assert_eq!(positions.last().unwrap().role.as_deref(), Some(BET_ROLE));
    assert!(check_bet_pairs(&raw.positions, &raw.bets, &[]).is_ok());
    let mut not_weth = std::collections::HashSet::new();
    not_weth.insert(raw.positions[0].position_id.clone());
    assert!(check_bet_pairs(&raw.positions, &not_weth, &[]).unwrap_err().to_string().contains("A bet is a WETH/NOCK range"));
}

// ------------------------------------------------------------------
// The card
// ------------------------------------------------------------------

#[test]
fn test_the_operators_bet_today_is_ready_with_exact_amounts_and_the_link() {
    let v = card(&live_data(), &profile_with(Some(operator_bets())), &detection_live());
    assert_eq!(s(&v, "/status"), "ready", "{v:#}");
    assert_eq!(v["shown"], json!(true));
    assert_eq!(s(&v, "/tier"), "top");
    assert_eq!(v["suggestion"]["tick_lower"], json!(70200));
    assert_eq!(v["suggestion"]["tick_upper"], json!(71000));
    let value: f64 = s(&v, "/suggestion/value_usd").parse().unwrap();
    assert!(value > 39.98 && value <= 40.0, "{value}");
    // The exact amounts are the mint's own: they buy the stated liquidity and no more.
    let sqrt = mint_math::Big::from_hex(&live_pool().sqrt_price_x96).unwrap();
    let l = mint_math::Big::from_hex(&format!("{:x}", s(&v, "/suggestion/liquidity").parse::<u128>().unwrap())).unwrap();
    let (a0, a1) = mint_math::amounts_for_liquidity(70627, &sqrt, 70200, 71000, &l).unwrap();
    assert_eq!(a0.to_dec(), s(&v, "/suggestion/amount0_raw"));
    assert_eq!(a1.to_dec(), s(&v, "/suggestion/amount1_raw"));
    // The wallet's WETH (0.002177) does not cover the ETH side; its ETH does.
    assert_eq!(s(&v, "/suggestion/deposit_token"), "ETH");
    assert_eq!(
        s(&v, "/suggestion/link/url"),
        "https://app.aero.xyz/deposit/8453-0x957256c98d61A714081DE2EE70ae3A96c9b40874?range=custom&tickLow=70200&tickHigh=71000&native=true"
    );
    assert!(!s(&v, "/suggestion/link/url").contains("aerodrome.finance"));
    assert!(s(&v, "/suggestion/link/amounts_to_enter").contains(" ETH and "));
    // Gates, in the issue's order, all passing.
    let gates: Vec<(&str, &str)> = v["gates"].as_array().unwrap().iter().map(|g| (g["gate"].as_str().unwrap(), g["status"].as_str().unwrap())).collect();
    assert_eq!(gates, vec![("settings", "pass"), ("gas_floor", "pass"), ("nock_floor", "pass"), ("no_bet_in_range", "pass")]);
    // The floor-crossing price includes the hypothetical bet.
    assert!(s(&v, "/floor_crossing/text").starts_with("With this bet, NOCK could rise to $"), "{}", v["floor_crossing"]);
}

#[test]
fn test_short_of_gas_the_card_measures_what_would_come_out_of_the_eth_usdc_ranges() {
    let mut data = live_data();
    for w in data.wallet_balances.iter_mut().filter(|w| w.token_symbol == "ETH") {
        w.amount = "0.010000".to_string();
    }
    let v = card(&data, &profile_with(Some(operator_bets())), &detection_live());
    assert_eq!(s(&v, "/status"), "gas_short");
    assert_eq!(v["shown"], json!(false));
    assert_eq!(s(&v, "/tier"), "quiet");
    let t = s(&v, "/gas/text");
    assert!(t.contains("would have to come out of them first. A measurement, not an instruction."), "{t}");
    assert!(t.contains("The 2 ETH/USDC ranges hold 0.2766 WETH"), "{t}");
    let lower = t.to_lowercase();
    assert!(!lower.contains("pull now") && !lower.contains("withdraw now"));
    // The link is still stated, so the measurement can be checked.
    assert!(v["suggestion"]["link"]["url"].is_string());
}

#[test]
fn test_the_nock_gate_is_total_nock_held_and_a_bet_in_range_is_shown_instead() {
    // Below the floor: no bet.
    let mut low = profile_with(Some(operator_bets()));
    low.treasury_floor_nock = 200_000.0;
    let v = card(&live_data(), &low, &detection_live());
    assert_eq!(s(&v, "/status"), "nock_below_floor");
    // Wallet NOCK (90,087) is below the 100,000 floor; total NOCK held (109,640) is not: the
    // gate is the total, not the LP NOCK budget.
    assert_eq!(s(&card(&live_data(), &profile_with(Some(operator_bets())), &detection_live()), "/status"), "ready");
    // A bet in range: shown instead of a new one. One out of range: cashed out, blocks nothing.
    let mut data = live_data();
    with_bet_pool(&mut data);
    data.positions.push(bet_position(3500001, 70200, 71000, true, "0.011500", "1385.20", "39.82"));
    let v = card(&data, &profile_with(Some(operator_bets())), &detection_live());
    assert_eq!(s(&v, "/status"), "bet_in_range");
    assert_eq!(v["shown"], json!(false));
    assert_eq!(v["bets"].as_array().unwrap().len(), 1);
    let mut data = live_data();
    data.positions.push(bet_position(3400001, 73400, 74800, false, "0.000000", "2940.10", "42.37"));
    let v = card(&data, &profile_with(Some(operator_bets())), &detection_live());
    assert_eq!(s(&v, "/status"), "ready");
    assert!(s(&v, "/bets/0/line").contains("out of range, cashed out: it holds only NOCK"), "{}", v["bets"]);
    assert!(s(&v, "/bets/0/line").contains("0.41 AERO earned ($0.15)"));
}

#[test]
fn test_without_nock_bets_there_is_no_card() {
    let mut data = live_data();
    attach_bet_suggestion(&mut data, &profile_with(None), &detection_live());
    assert!(data.derived_metrics.bet_suggestion.is_none());
    attach_bet_suggestion(&mut data, &profile_with(Some(NockBetsConfig::default())), &BetDetection::not_run("operator_profile.nock_bets.pool is not set to a pool address"));
    let v = data.derived_metrics.bet_suggestion.clone().unwrap();
    assert_eq!(s(&v, "/status"), "needs_setting");
    assert_eq!(v["missing"].as_array().unwrap().len(), 4);
}

#[test]
fn test_the_bet_card_is_out_of_the_state_hash() {
    let mut data = live_data();
    let before = compute_state_hash(&data);
    attach_bet_suggestion(&mut data, &profile_with(Some(operator_bets())), &detection_live());
    assert!(data.derived_metrics.bet_suggestion.is_some());
    assert_eq!(compute_state_hash(&data), before);
}

#[test]
fn test_bet_events_are_top_only_while_a_bet_is_suggested() {
    let mut data = live_data();
    data.positions.push(bet_position(3400001, 73400, 74800, false, "0.000000", "2940.10", "42.37"));
    attach_bet_suggestion(&mut data, &profile_with(Some(operator_bets())), &detection_live());
    let ev = events_block(&data, None, false, t("2026-11-21T12:00:00Z"));
    let bets: Vec<&Value> = ev.as_array().unwrap().iter().filter(|e| e["card"] == json!("bet_suggestion")).collect();
    assert_eq!(bets.len(), 2, "{bets:#?}");
    assert_eq!(bets[0]["tier"], json!("top"));
    assert_eq!(bets[0]["links"][0]["url"].as_str().unwrap(), s(data.derived_metrics.bet_suggestion.as_ref().unwrap(), "/suggestion/link/url"));
    assert_eq!(bets[1]["tier"], json!("quiet"));
    assert_eq!(bets[1]["title"], json!("BET CASHED OUT"));
    // Gas short: the card's event is a quiet line.
    for w in data.wallet_balances.iter_mut().filter(|w| w.token_symbol == "ETH") {
        w.amount = "0.001".to_string();
    }
    attach_bet_suggestion(&mut data, &profile_with(Some(operator_bets())), &detection_live());
    let ev = events_block(&data, None, false, t("2026-11-21T12:00:00Z"));
    assert!(ev.as_array().unwrap().iter().filter(|e| e["card"] == json!("bet_suggestion")).all(|e| e["tier"] == json!("quiet")));
}

fn t(s: &str) -> DateTime<Utc> {
    DateTime::parse_from_rfc3339(s).unwrap().with_timezone(&Utc)
}

/// Pins the card and its MCP text byte for byte, for both transports and the frontend
/// fixture. Regenerate only for an intended change:
///   NOCKNOCK_BLESS_GOLDEN=1 cargo test test_golden_bet_suggestion
#[test]
fn test_golden_bet_suggestion() {
    let ops = profile_with(Some(operator_bets()));
    let mut cases: Vec<(&str, Value, Option<&str>)> = Vec::new();
    let ready = card(&live_data(), &ops, &detection_live());
    cases.push(("ready", ready.clone(), Some("ok")));
    cases.push(("ready_stale", ready, Some("stale")));
    let mut short = live_data();
    for w in short.wallet_balances.iter_mut().filter(|w| w.token_symbol == "ETH") {
        w.amount = "0.010000".to_string();
    }
    cases.push(("gas_short", card(&short, &ops, &detection_live()), Some("ok")));
    let mut open = live_data();
    with_bet_pool(&mut open);
    open.positions.push(bet_position(3500001, 70200, 71000, true, "0.011500", "1385.20", "39.82"));
    open.positions.push(bet_position(3400001, 73400, 74800, false, "0.000000", "2940.10", "42.37"));
    cases.push(("bet_in_range", card(&open, &ops, &detection_live()), Some("ok")));
    let needs = profile_with(Some(NockBetsConfig { gas_floor_eth: None, ..operator_bets() }));
    cases.push(("needs_setting", card(&live_data(), &needs, &detection_live()), Some("ok")));
    let mut out = serde_json::Map::new();
    for (name, v, health) in cases {
        let txt = text(&v, health);
        assert!(!txt.contains("aerodrome.finance"));
        out.insert(name.to_string(), json!({ "bet_suggestion": v, "health": health, "text": txt }));
    }
    let actual = format!("{}\n", serde_json::to_string_pretty(&Value::Object(out)).unwrap());
    let path = concat!(env!("CARGO_MANIFEST_DIR"), "/testdata/bets/golden_bet_suggestion.json");
    if std::env::var("NOCKNOCK_BLESS_GOLDEN").is_ok() {
        std::fs::write(path, &actual).unwrap();
    }
    let expected = std::fs::read_to_string(path).expect("testdata/bets/golden_bet_suggestion.json is checked in");
    assert!(actual == expected, "bet card bytes changed; compare {path} with a fresh NOCKNOCK_BLESS_GOLDEN=1 run and `git diff` it");
}
