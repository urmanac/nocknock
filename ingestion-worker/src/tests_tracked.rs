//! Positions tracked from the database (tracked.rs; v0.9.0).
//!
//! The chain answers are the recorded ones of tests_position_detect.rs (the operator's
//! wallet at block 58646285) and of the #144 cycle fixture at the same block.
//!
//! The store's rules (which row wins, what a tap replaces, what a re-read keeps) are one
//! contract, `store_contract`. It runs here against the in-memory store on every test
//! run, and against PostgreSQL through the worker's own SQL when a scratch database is
//! given (ignored otherwise, like the route-quote SQL test):
//!   NOCKNOCK_SCRATCH_DATABASE_URL=postgres://…/nocknock_scratch \
//!     cargo test scratch_database -- --ignored
use super::*;
use crate::tests_position_detect::{book, book_without_v4, clock, cycle_data, detect, key_liquidity, key_owner, node, prev_suggesting, read_cycle, store_at, wide_logs_swap, Swap, MINT_6CDA, OPERATOR, POOL_3PCT};
use serde_json::Value;

const USDC: &str = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913";
const NOCK: &str = "0x9b5e262cf9bb04869ab40b19af91d2dc85761722";
const WETH: &str = "0x4200000000000000000000000000000000000006";
const POOL_075PCT: &str = "0x91a9ef0a8dd231b99fa53a2a145a9e8bca7897e0d85852107abb997916f9f99c";
const STRANGER: &str = "0x4598b3ba7d6714091b18da5d110b64f56e94e826";
/// A v4 token id that was burned (tests_closed.rs; its reads are in the #144 fixture).
const V4_BURNED_NFT: u64 = 3229273;

/// A ladder row in the 3% pool, as detection writes one.
fn ladder_row(wallet: &str, nft: u64, mint_block: Option<i64>) -> TrackedRow {
    TrackedRow {
        wallet: wallet.to_string(),
        position_key: v4_position_key("base", nft),
        protocol: "uniswap-v4".to_string(),
        network: "base".to_string(),
        nft_id: nft,
        pool_id: POOL_3PCT.to_string(),
        currency0: USDC.to_string(),
        currency1: NOCK.to_string(),
        tick_lower: 273900,
        tick_upper: 275400,
        tick_spacing: Some(300),
        role: Some(LADDER_ROLE.to_string()),
        rung: None,
        deposit_tx: Some(format!("0x{:064x}", nft)),
        mint_block,
        source: TRACKED_SOURCE_DETECTED.to_string(),
        tracked_at: None,
        deposit_liquidity: None,
        max_liquidity_seen: None,
    }
}

/// The operator's own NFT #3163450 as a row.
fn row_3163450(role: Option<&str>) -> TrackedRow {
    TrackedRow {
        tick_lower: 262200,
        tick_upper: 282300,
        role: role.map(str::to_string),
        deposit_tx: role.map(|_| MINT_6CDA.to_string()),
        mint_block: Some(53008241),
        ..ladder_row(OPERATOR, 3163450, None)
    }
}

/// The wallet's NFT #3163431: it exists and holds nothing (the 0.75% pool).
fn row_emptied() -> TrackedRow {
    TrackedRow { pool_id: POOL_075PCT.to_string(), tick_lower: 267825, tick_upper: 271050, tick_spacing: Some(75), rung: Some("near".to_string()), ..ladder_row(OPERATOR, 3163431, Some(53007505)) }
}

fn candidate_row(wallet: &str, nft: u64) -> CandidateRow {
    CandidateRow {
        wallet: wallet.to_string(),
        protocol: "uniswap-v4".to_string(),
        network: "base".to_string(),
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
        tick_lower: Some(-600),
        tick_upper: Some(275400),
        received_tx: Some(format!("0x{:064x}", nft)),
        received_block: Some(60_000_000),
        received_from: Some("0x0000000000000000000000000000000000000000".to_string()),
        mint_tx: Some(format!("0x{:064x}", nft)),
        mint_block: Some(60_000_000),
        minted_by: Some(wallet.to_lowercase()),
        deposit_check: Some("ok".to_string()),
    }
}

// ------------------------------------------------------------------
// The store contract: the SQL's rules, stated once
// ------------------------------------------------------------------

/// Every rule the tables hold. `w` is a wallet of this test's own, in mixed case.
async fn store_contract(store: &PositionStore, w: &str, other: &str) {
    let lw = w.to_lowercase();
    let unstamped = |mut rows: Vec<TrackedRow>| {
        rows.iter_mut().for_each(|r| r.tracked_at = None);
        rows
    };
    assert!(store.tracked(w).await.unwrap().is_empty());

    // ---- tracked_positions: PRIMARY KEY (wallet, position_key) ----
    let a = ladder_row(w, 7, Some(100));
    assert!(store.insert_tracked_if_absent(&a).await.unwrap(), "a new row is added");
    // The automatic path never overwrites a row: the second insert changes nothing.
    let clash = TrackedRow { role: None, rung: None, source: TRACKED_SOURCE_CONFIRMED.to_string(), deposit_tx: None, tick_lower: 1, ..a.clone() };
    assert!(!store.insert_tracked_if_absent(&clash).await.unwrap(), "the same wallet and position is one row");
    let got = store.tracked(&lw).await.unwrap();
    assert_eq!(got.len(), 1);
    // The wallet is stored lowercase whatever case it came in, and read back either way.
    assert_eq!(unstamped(got.clone()), vec![TrackedRow { wallet: lw.clone(), ..a.clone() }]);
    assert_eq!(store.tracked(w).await.unwrap(), got);
    let first_at = got[0].tracked_at.expect("the store stamps tracked_at");

    // A tap replaces role, rung and source; it keeps a known deposit_tx when it brings
    // none, keeps the position's facts, and keeps when it was first tracked.
    store.upsert_tracked(&TrackedRow { rung: Some("deep".to_string()), source: TRACKED_SOURCE_CONFIRMED.to_string(), deposit_tx: None, tick_lower: 1, ..a.clone() }).await.unwrap();
    let got = store.tracked(w).await.unwrap();
    assert_eq!(got.len(), 1);
    assert_eq!((got[0].rung.as_deref(), got[0].source.as_str(), got[0].role.as_deref()), (Some("deep"), "confirmed", Some("ladder")));
    assert_eq!(got[0].deposit_tx, a.deposit_tx, "a tap without a mint does not erase the mint that was found");
    assert_eq!((got[0].tick_lower, got[0].tracked_at), (a.tick_lower, Some(first_at)));
    store.upsert_tracked(&TrackedRow { role: None, rung: None, deposit_tx: Some("0xnew".to_string()), source: TRACKED_SOURCE_CONFIRMED.to_string(), ..a.clone() }).await.unwrap();
    let got = store.tracked(w).await.unwrap();
    assert_eq!((got[0].role.clone(), got[0].rung.clone(), got[0].deposit_tx.as_deref()), (None, None, Some("0xnew")));

    // A tap on a position that is not tracked yet adds it.
    store.upsert_tracked(&ladder_row(w, 9, None)).await.unwrap();
    assert!(store.insert_tracked_if_absent(&ladder_row(w, 3, Some(50))).await.unwrap());
    // Oldest mint first; a row with no mint block last; then by NFT id.
    assert!(store.insert_tracked_if_absent(&ladder_row(w, 2, None)).await.unwrap());
    let ids: Vec<u64> = store.tracked(w).await.unwrap().iter().map(|r| r.nft_id).collect();
    assert_eq!(ids, vec![3, 7, 2, 9]);
    // Another wallet's row for the same position key is its own row.
    assert!(store.insert_tracked_if_absent(&ladder_row(other, 7, Some(100))).await.unwrap());
    assert_eq!(store.tracked(w).await.unwrap().len(), 4);
    assert_eq!(store.tracked(other).await.unwrap().len(), 1);
    // Untrack removes exactly that row, once.
    assert!(store.delete_tracked(w, &a.position_key).await.unwrap());
    assert!(!store.delete_tracked(w, &a.position_key).await.unwrap());
    assert!(!store.delete_tracked(w, "uniswap-v4:base:999").await.unwrap());
    assert_eq!(store.tracked(w).await.unwrap().iter().map(|r| r.nft_id).collect::<Vec<_>>(), vec![3, 2, 9]);
    assert_eq!(store.tracked(other).await.unwrap().len(), 1, "the other wallet's row is untouched");

    // ---- position_candidates: PRIMARY KEY (wallet, protocol, network, nft_id) ----
    assert!(store.candidates(w).await.unwrap().is_empty());
    let c = candidate_row(w, 7);
    // A choice is never written by a chain read, even when the row passed in carries one.
    store.upsert_candidate(&CandidateRow { choice: Some(CHOICE_DISMISSED.to_string()), ..c.clone() }).await.unwrap();
    assert_eq!(store.candidates(w).await.unwrap(), vec![CandidateRow { wallet: lw.clone(), ..c.clone() }]);
    assert!(store.set_choice(w, "uniswap-v4", "base", 7, Some(CHOICE_DISMISSED)).await.unwrap());
    // Reading the chain again replaces every fact (also with nothing) and keeps the choice.
    let reread = CandidateRow {
        verdict: "gone".to_string(),
        owned: false,
        has_liquidity: None,
        pool_id: None,
        mint_tx: None,
        mint_block: None,
        minted_by: None,
        deposit_check: Some("a refusal".to_string()),
        tick_lower: Some(-887100),
        ..c.clone()
    };
    store.upsert_candidate(&reread).await.unwrap();
    assert_eq!(store.candidates(w).await.unwrap(), vec![CandidateRow { wallet: lw.clone(), choice: Some(CHOICE_DISMISSED.to_string()), ..reread.clone() }]);
    assert!(store.set_choice(w, "uniswap-v4", "base", 7, None).await.unwrap());
    assert_eq!(store.candidates(w).await.unwrap()[0].choice, None);
    assert!(!store.set_choice(w, "uniswap-v4", "base", 8, Some(CHOICE_DISMISSED)).await.unwrap(), "no such row: nothing is created");
    // The same NFT id under another protocol or network is another row; so is another wallet's.
    store.upsert_candidate(&CandidateRow { protocol: "uniswap-v3".to_string(), ..c.clone() }).await.unwrap();
    store.upsert_candidate(&CandidateRow { network: "other".to_string(), ..c.clone() }).await.unwrap();
    store.upsert_candidate(&candidate_row(w, 5)).await.unwrap();
    store.upsert_candidate(&candidate_row(other, 7)).await.unwrap();
    let rows = store.candidates(w).await.unwrap();
    assert_eq!(rows.iter().map(|r| r.nft_id).collect::<Vec<_>>(), vec![5, 7, 7, 7], "by NFT id");
    assert!(store.set_choice(w, "uniswap-v3", "base", 7, Some(CHOICE_DISMISSED)).await.unwrap());
    let dismissed: Vec<(String, String)> = store.candidates(w).await.unwrap().into_iter().filter(|r| r.choice.is_some()).map(|r| (r.protocol, r.network)).collect();
    assert_eq!(dismissed, vec![("uniswap-v3".to_string(), "base".to_string())], "a choice touches one row");
    assert_eq!(store.candidates(other).await.unwrap().len(), 1);

    // ---- position_detection_state: PRIMARY KEY (wallet, protocol, network) ----
    assert_eq!(store.scan_state(w, "uniswap-v4", "base").await.unwrap(), ScanState::default(), "nothing read yet");
    let t = |s: i64| Utc.timestamp_opt(1_795_000_000 + s, 0).unwrap();
    let s1 = ScanState { logs_from_block: Some(58_601_286), logs_to_block: Some(58_646_285), history_complete_at: None, last_checked_at: Some(t(0)), refresh_cursor: 0 };
    store.save_scan_state(w, "uniswap-v4", "base", &s1).await.unwrap();
    assert_eq!(store.scan_state(&lw, "uniswap-v4", "base").await.unwrap(), s1);
    let s2 = ScanState { logs_from_block: Some(0), logs_to_block: Some(58_647_000), history_complete_at: Some(t(300)), last_checked_at: Some(t(300)), refresh_cursor: 120 };
    store.save_scan_state(w, "uniswap-v4", "base", &s2).await.unwrap();
    assert_eq!(store.scan_state(w, "uniswap-v4", "base").await.unwrap(), s2, "one row per wallet, protocol and network");
    // A cursor can be cleared, and another protocol's or wallet's is its own.
    store.save_scan_state(w, "uniswap-v4", "base", &ScanState::default()).await.unwrap();
    assert_eq!(store.scan_state(w, "uniswap-v4", "base").await.unwrap(), ScanState::default());
    store.save_scan_state(other, "uniswap-v4", "base", &s1).await.unwrap();
    assert_eq!(store.scan_state(w, "uniswap-v3", "base").await.unwrap(), ScanState::default());
    assert_eq!(store.scan_state(other, "uniswap-v4", "base").await.unwrap(), s1);
}

const CONTRACT_WALLET: &str = "0x00000000000000000000000000000000000145Aa";
const CONTRACT_OTHER: &str = "0x00000000000000000000000000000000000145bb";

#[tokio::test]
async fn test_the_store_contract_holds_in_memory() {
    store_contract(&PositionStore::mem(), CONTRACT_WALLET, CONTRACT_OTHER).await;
    // An unreadable store fails every call: no call answers with an empty table.
    let store = PositionStore::mem();
    if let PositionStore::Mem(m) = &store {
        m.lock().unwrap().unreadable = true;
    }
    assert!(store.tracked(OPERATOR).await.is_err() && store.candidates(OPERATOR).await.is_err());
    assert!(store.scan_state(OPERATOR, "uniswap-v4", "base").await.is_err());
    assert!(store.insert_tracked_if_absent(&ladder_row(OPERATOR, 1, None)).await.is_err());
    assert!(store.upsert_tracked(&ladder_row(OPERATOR, 1, None)).await.is_err());
    assert!(store.delete_tracked(OPERATOR, "x").await.is_err());
    assert!(store.upsert_candidate(&candidate_row(OPERATOR, 1)).await.is_err());
    assert!(store.set_choice(OPERATOR, "uniswap-v4", "base", 1, None).await.is_err());
    assert!(store.save_scan_state(OPERATOR, "uniswap-v4", "base", &ScanState::default()).await.is_err());
}

// ---------------- SQL, against a scratch database ----------------
// Every query in tracked.rs is a runtime string, so this runs them for real, after
// applying EVERY migration to the scratch database with the worker's own migrator. It
// refuses a database whose name does not end in "_scratch".

#[tokio::test]
#[ignore = "needs NOCKNOCK_SCRATCH_DATABASE_URL (a scratch PostgreSQL; the migrations are applied to it)"]
async fn test_tracked_position_sql_against_a_scratch_database() {
    let url = std::env::var("NOCKNOCK_SCRATCH_DATABASE_URL").expect("NOCKNOCK_SCRATCH_DATABASE_URL is set");
    let db = PgPool::connect(&url).await.expect("scratch database reachable");
    let name: String = sqlx::query("SELECT current_database() AS n").fetch_one(&db).await.unwrap().get("n");
    assert!(name.ends_with("_scratch"), "refusing to run against database {name:?}: its name must end in _scratch");
    sqlx::migrate!("./migrations").run(&db).await.expect("every migration applies to an empty or migrated database");
    // Applying them again changes nothing: the worker does this at every start.
    sqlx::migrate!("./migrations").run(&db).await.expect("the migrations are idempotent");
    let applied: Vec<i64> = sqlx::query("SELECT version FROM _sqlx_migrations ORDER BY version").fetch_all(&db).await.unwrap().iter().map(|r| r.get("version")).collect();
    assert_eq!(applied.last(), Some(&20261121000013), "this branch's migration is the newest");

    let cleanup = |db: PgPool| async move {
        for wallet in [CONTRACT_WALLET.to_lowercase(), CONTRACT_OTHER.to_lowercase()] {
            for table in ["tracked_positions", "position_candidates", "position_detection_state"] {
                // `table` is one of three literals above, never input.
                sqlx::query(&format!("DELETE FROM {table} WHERE wallet = $1")).bind(&wallet).execute(&db).await.unwrap();
            }
        }
    };
    cleanup(db.clone()).await;
    let store = PositionStore::Pg(db.clone());
    store_contract(&store, CONTRACT_WALLET, CONTRACT_OTHER).await;

    // What only the real tables can say.
    // The migration creates exactly these three tables, and no deploy-attempt table.
    let tables: Vec<String> = sqlx::query("SELECT table_name::text AS t FROM information_schema.tables WHERE table_schema = 'public' ORDER BY 1")
        .fetch_all(&db)
        .await
        .unwrap()
        .iter()
        .map(|r| r.get("t"))
        .collect();
    for t in ["tracked_positions", "position_candidates", "position_detection_state"] {
        assert!(tables.iter().any(|x| x == t), "{t} exists: {tables:?}");
    }
    assert!(!tables.iter().any(|t| t.contains("deploy") || t.contains("attempt")), "{tables:?}");
    // The primary keys are the ones the contract relies on.
    let pk = |table: &'static str| {
        let db = db.clone();
        async move {
            sqlx::query(
                "SELECT a.attname::text AS c FROM pg_index i JOIN pg_attribute a ON a.attrelid = i.indrelid AND a.attnum = ANY(i.indkey) \
                 WHERE i.indrelid = $1::regclass AND i.indisprimary ORDER BY array_position(i.indkey, a.attnum)",
            )
            .bind(table)
            .fetch_all(&db)
            .await
            .unwrap()
            .iter()
            .map(|r| r.get::<String, _>("c"))
            .collect::<Vec<_>>()
        }
    };
    assert_eq!(pk("tracked_positions").await, vec!["wallet", "position_key"]);
    assert_eq!(pk("position_candidates").await, vec!["wallet", "protocol", "network", "nft_id"]);
    assert_eq!(pk("position_detection_state").await, vec!["wallet", "protocol", "network"]);
    // A direct duplicate insert is refused by the database itself, not only by ON CONFLICT.
    let w = CONTRACT_WALLET.to_lowercase();
    let dup = sqlx::query("INSERT INTO tracked_positions (wallet, position_key, protocol, network, nft_id, pool_id, currency0, currency1, tick_lower, tick_upper, source) SELECT wallet, position_key, protocol, network, nft_id, pool_id, currency0, currency1, tick_lower, tick_upper, source FROM tracked_positions WHERE wallet = $1 LIMIT 1")
        .bind(&w)
        .execute(&db)
        .await;
    assert!(dup.unwrap_err().to_string().contains("duplicate key"), "the primary key refuses a second row");
    // A negative NFT id is refused by the CHECK; an id beyond BIGINT never reaches SQL.
    let negative = sqlx::query("INSERT INTO position_candidates (wallet, protocol, network, nft_id, verdict, owned) VALUES ($1, 'uniswap-v4', 'base', -1, 'eligible', true)").bind(&w).execute(&db).await;
    assert!(negative.is_err());
    assert!(store.upsert_candidate(&candidate_row(CONTRACT_WALLET, u64::MAX)).await.unwrap_err().to_string().contains("does not fit"));
    // Timestamps are the database's: tracked_at is set, and updated_at moves on a tap.
    let stamps = |db: PgPool, w: String| async move {
        let r = sqlx::query("SELECT tracked_at, updated_at FROM tracked_positions WHERE wallet = $1 AND nft_id = 3").bind(&w).fetch_one(&db).await.unwrap();
        (r.get::<DateTime<Utc>, _>("tracked_at"), r.get::<DateTime<Utc>, _>("updated_at"))
    };
    let (tracked_at, updated_before) = stamps(db.clone(), w.clone()).await;
    store.upsert_tracked(&TrackedRow { rung: Some("near".to_string()), ..ladder_row(CONTRACT_WALLET, 3, Some(50)) }).await.unwrap();
    let (tracked_after, updated_after) = stamps(db.clone(), w.clone()).await;
    assert_eq!(tracked_at, tracked_after);
    assert!(updated_after >= updated_before);
    cleanup(db).await;
}

/// The whole flow through the real tables: a scan of the recorded chain stores its 43
/// candidates, a tap tracks and untracks, an exact match is tracked by itself, and the
/// cycle reads the row it wrote. Same guard, same scratch database.
#[tokio::test]
#[ignore = "needs NOCKNOCK_SCRATCH_DATABASE_URL (a scratch PostgreSQL; the migrations are applied to it)"]
async fn test_detection_and_the_one_tap_writes_against_a_scratch_database() {
    let url = std::env::var("NOCKNOCK_SCRATCH_DATABASE_URL").expect("NOCKNOCK_SCRATCH_DATABASE_URL is set");
    let db = PgPool::connect(&url).await.expect("scratch database reachable");
    let name: String = sqlx::query("SELECT current_database() AS n").fetch_one(&db).await.unwrap().get("n");
    assert!(name.ends_with("_scratch"), "refusing to run against database {name:?}: its name must end in _scratch");
    sqlx::migrate!("./migrations").run(&db).await.expect("every migration applies");
    let cleanup = |db: PgPool| async move {
        for table in ["tracked_positions", "position_candidates", "position_detection_state"] {
            // `table` is one of three literals above, never input.
            sqlx::query(&format!("DELETE FROM {table} WHERE wallet = $1")).bind(OPERATOR).execute(&db).await.unwrap();
        }
    };
    cleanup(db.clone()).await;
    let store = PositionStore::Pg(db.clone());
    let n = node("operator", vec![wide_logs_swap("operator")]).await;
    let cfg = book_without_v4();
    let http = Client::new();

    // The scan stores every NFT it read, and lists the one that is found and not counted.
    let out = detect(&store, &n, &cfg, None, clock("operator")).await;
    assert_eq!((out.status.as_str(), out.found.len()), ("ok", 1));
    assert_eq!(out.ignored, std::collections::BTreeMap::from([("empty".to_string(), 5), ("not_whitelisted".to_string(), 37)]));
    let rows = store.candidates(OPERATOR).await.unwrap();
    assert_eq!(rows.len(), 43);
    // The rows PostgreSQL holds are the rows the in-memory store holds for the same scan.
    let mem = store_at(clock("operator"));
    detect(&mem, &n, &cfg, None, clock("operator")).await;
    assert_eq!(rows, mem.candidates(OPERATOR).await.unwrap());
    assert_eq!(store.scan_state(OPERATOR, "uniswap-v4", "base").await.unwrap(), mem.scan_state(OPERATOR, "uniswap-v4", "base").await.unwrap());

    // One tap each way.
    track_position(&store, &http, Some(&n.url), OPERATOR, &cfg.positions, 3163450, "lp").await.unwrap();
    track_position(&store, &http, Some(&n.url), OPERATOR, &cfg.positions, 3163450, "ladder:near").await.unwrap();
    let row = store.tracked(OPERATOR).await.unwrap().remove(0);
    assert_eq!((row.role.as_deref(), row.rung.as_deref(), row.source.as_str(), row.deposit_tx.as_deref()), (Some("ladder"), Some("near"), "confirmed", Some(MINT_6CDA)));
    let raw = read_cycle(&store, &n, &cfg).await.unwrap();
    assert_eq!(raw.position_sources.len(), 1, "the cycle read the row PostgreSQL holds");
    untrack_position(&store, OPERATOR, &cfg.positions, "uniswap-v4:base:3163450").await.unwrap();
    assert!(store.tracked(OPERATOR).await.unwrap().is_empty());
    assert_eq!(store.candidates(OPERATOR).await.unwrap().iter().find(|c| c.nft_id == 3163450).unwrap().choice.as_deref(), Some(CHOICE_DISMISSED));
    set_found_choice(&store, OPERATOR, 3163450, false).await.unwrap();

    // An exact rung match is tracked by itself, and a second cycle leaves the row alone.
    let prev = prev_suggesting(&[(262200, 282300, None)]);
    let out = detect(&store, &n, &cfg, Some(&prev), clock("operator")).await;
    assert_eq!(out.newly_tracked, vec!["uniswap-v4:base:3163450".to_string()]);
    let first = store.tracked(OPERATOR).await.unwrap();
    detect(&store, &n, &cfg, Some(&prev), clock("operator")).await;
    assert_eq!(store.tracked(OPERATOR).await.unwrap(), first);
    assert_eq!((first[0].source.as_str(), first[0].rung.clone(), first[0].mint_block), ("detected", None, Some(53008241)));
    let block = position_detection_block(&out, &read_cycle(&store, &n, &cfg).await.unwrap());
    assert_eq!(block["tracked"][0]["state"], "counted");
    cleanup(db).await;
}

// ------------------------------------------------------------------
// From a row to a position (pure)
// ------------------------------------------------------------------

#[test]
fn test_a_row_is_never_a_price_source_a_rebuild_template_or_a_deposit_link() {
    for rung in LADDER_RUNGS.iter().map(|r| Some(r.to_string())).chain([None]) {
        let row = TrackedRow { rung: rung.clone(), ..ladder_row(OPERATOR, 3243807, Some(60_240_576)) };
        assert_eq!(validate_tracked_row(&row), Ok(()), "{rung:?}");
        let cfg = tracked_position_config(&row);
        assert_eq!(cfg.id, "tracked-v4-3243807");
        assert_eq!(position_key(&cfg), row.position_key);
        assert!(!cfg.is_price_source && cfg.deposit_url.is_none() && !cfg.rebuild_when_out_of_range);
        // A ladder is left out of NOCK LP by its role (#167); the key that would say so
        // again is one validate_role_config refuses on a ladder.
        assert_eq!(cfg.rebuild_weight_bps, None);
        assert_eq!((cfg.role.as_deref(), cfg.rung.clone(), cfg.deposit_tx.clone()), (Some("ladder"), rung, row.deposit_tx.clone()));
        validate_positions_config(std::slice::from_ref(&cfg)).expect("a detected ladder row passes the config checks");
    }
    // A row with no role: classified by its pair as a config entry is, never sized.
    let lp = TrackedRow { role: None, rung: Some("near".to_string()), ..ladder_row(OPERATOR, 3243807, None) };
    assert_eq!(validate_tracked_row(&lp), Ok(()));
    let cfg = tracked_position_config(&lp);
    assert!(!cfg.is_price_source && cfg.deposit_url.is_none());
    validate_positions_config(std::slice::from_ref(&cfg)).unwrap();
    assert_eq!((cfg.role, cfg.rung, cfg.deposit_tx, cfg.rebuild_weight_bps), (None, None, None, Some(0)), "rung and deposit_tx belong to a ladder only");
}

#[test]
fn test_a_row_that_would_stall_the_cycle_is_quarantined_with_its_reason() {
    let base = ladder_row(OPERATOR, 7, None);
    let refused = |change: &dyn Fn(&mut TrackedRow)| {
        let mut r = base.clone();
        change(&mut r);
        validate_tracked_row(&r).unwrap_err()
    };
    assert_eq!(refused(&|r| r.protocol = "uniswap-v3".to_string()), "protocol uniswap-v3 on base is not one NockNock tracks from the chain (Uniswap v4 on Base only)");
    assert_eq!(refused(&|r| r.network = "mainnet".to_string()), "protocol uniswap-v4 on mainnet is not one NockNock tracks from the chain (Uniswap v4 on Base only)");
    assert_eq!(refused(&|r| r.position_key = "uniswap-v4:base:8".to_string()), "its key uniswap-v4:base:8 does not name NFT #7");
    assert_eq!(refused(&|r| r.pool_id = "0x1234".to_string()), "its pool id is not a 32-byte id");
    assert!(refused(&|r| r.currency1 = "0xba0dda8762c24da9487f5fa026a9b64b695a07ea".to_string()).contains("are not both in the token registry"));
    assert_eq!(refused(&|r| r.tick_upper = r.tick_lower), "its ticks 273900..273900 are not a range");
    assert_eq!(refused(&|r| r.currency0 = WETH.to_string()), "it is tracked as a ladder but is a WETH/NOCK pair: a ladder is a USDC/NOCK range");
    assert_eq!(refused(&|r| { r.tick_lower = -887100; r.tick_upper = 887100; }), "it is tracked as a ladder but spans every price: a ladder is a concentrated range");
    assert!(refused(&|r| r.rung = Some("middle".to_string())).contains("unknown rung 'middle'"));
    assert_eq!(refused(&|r| r.role = Some("hedge".to_string())), "Configuration error: position 'tracked-v4-7' has unknown role 'hedge'. Supported: ladder (or leave role unset). Nothing was read; correct `positions:` in config.yaml");
    assert_eq!(refused(&|r| r.source = "deploy".to_string()), "its source 'deploy' is not one NockNock knows");
    assert!(refused(&|r| r.deposit_tx = Some("0x12".to_string())).contains("is not a transaction hash"));
    // A WETH/USDC range tracked as LP is fine: it is a measured gas-side range (#151).
    let gas = TrackedRow { role: None, currency0: WETH.to_string(), currency1: USDC.to_string(), ..base.clone() };
    assert_eq!(validate_tracked_row(&gas), Ok(()));
}

#[test]
fn test_config_wins_rows_are_appended_and_an_empty_table_changes_nothing() {
    let cfg = book();
    let as_json = |p: &[PositionConfig]| serde_json::to_value(p).unwrap();
    // No row: the list is config.yaml's own, byte for byte.
    let none = merge_position_configs(&cfg.positions, &[], &HashMap::new());
    assert_eq!(as_json(&none.positions), as_json(&cfg.positions));
    assert!(none.sources.is_empty() && none.report.is_empty());
    assert_eq!(as_json(&MergedPositions::config_only(&cfg.positions).positions), as_json(&cfg.positions));

    let rows = vec![
        row_3163450(Some("ladder")),                                             // config.yaml lists it too
        ladder_row(OPERATOR, 3243807, Some(60_240_576)),                         // merged
        TrackedRow { protocol: "uniswap-v3".to_string(), ..ladder_row(OPERATOR, 11, None) }, // quarantined
        ladder_row(OPERATOR, 3243808, Some(60_240_580)),                         // another address owns it now
        TrackedRow { role: None, rung: None, ..ladder_row(OPERATOR, 3243809, None) }, // merged, as LP
    ];
    let not_owned = HashMap::from([("uniswap-v4:base:3243808".to_string(), STRANGER.to_string())]);
    let merged = merge_position_configs(&cfg.positions, &rows, &not_owned);
    // Config entries first, in config order and untouched; then the rows, in store order.
    assert_eq!(as_json(&merged.positions[..3]), as_json(&cfg.positions));
    let ids: Vec<&str> = merged.positions.iter().map(|p| p.id.as_str()).collect();
    assert_eq!(ids, vec!["uniswap-v4-nock-usdc-wide", "uniswap-v3-weth-nock", "aero-basic-nock-usdc", "tracked-v4-3243807", "tracked-v4-3243809"]);
    assert_eq!(
        merged.sources,
        HashMap::from([("uniswap-v4:base:3243807".to_string(), "tracked".to_string()), ("uniswap-v4:base:3243809".to_string(), "tracked".to_string())]),
        "only a merged row marks a position as tracked: the config entry of #3163450 is config's"
    );
    let fates: Vec<&TrackedFate> = merged.report.iter().map(|r| &r.fate).collect();
    assert_eq!(fates[0], &TrackedFate::ShadowedByConfig { config_id: "uniswap-v4-nock-usdc-wide".to_string() });
    assert_eq!(fates[1], &TrackedFate::Merged);
    assert!(matches!(fates[2], TrackedFate::Quarantined { reason } if reason.contains("Uniswap v4 on Base only")));
    assert_eq!(fates[3], &TrackedFate::NotOwned { owner: STRANGER.to_string() });
    assert_eq!(fates[4], &TrackedFate::Merged);
    // The whole merged list passes the checks every cycle applies to config.yaml.
    validate_positions_config(&merged.positions).unwrap();
    // Exactly one price source, and it is config.yaml's.
    let sources: Vec<&str> = merged.positions.iter().filter(|p| p.is_price_source).map(|p| p.id.as_str()).collect();
    assert_eq!(sources, vec!["uniswap-v4-nock-usdc-wide"]);
    // Two rows for one position cannot both be merged (the store's key forbids them; the
    // merge does not rely on it).
    let twice = merge_position_configs(&[], &[ladder_row(OPERATOR, 5, None), ladder_row(OPERATOR, 5, None)], &HashMap::new());
    assert_eq!(twice.positions.len(), 1);
    assert_eq!(twice.report[1].fate, TrackedFate::ShadowedByConfig { config_id: "tracked-v4-5".to_string() });
}

#[test]
fn test_an_owner_word_is_one_address_or_nothing() {
    assert_eq!(owner_from_word(&format!("0x{:0>64}", &OPERATOR[2..])), Some(OPERATOR.to_string()));
    assert_eq!(owner_from_word(&format!("{:0>64}", &OPERATOR[2..].to_uppercase())), Some(OPERATOR.to_string()));
    assert_eq!(owner_from_word("0x"), None);
    assert_eq!(owner_from_word(&"f".repeat(64)), None, "not an address word");
    assert_eq!(owner_from_word(&"0".repeat(128)), None);
}

#[test]
fn test_a_row_exports_as_the_config_entry_that_tracks_the_same_position() {
    // For a rollback to a release without database rows: the entry must parse as a
    // config position, name the same position, and keep the role, rung and baseline.
    for row in [
        TrackedRow { rung: Some("deep".to_string()), ..ladder_row(OPERATOR, 3243807, Some(60_240_576)) },
        ladder_row(OPERATOR, 3243808, None),
        TrackedRow { role: None, rung: None, deposit_tx: None, ..ladder_row(OPERATOR, 3243809, None) },
    ] {
        let entry = tracked_row_config_entry(&row);
        let parsed: Vec<PositionConfig> = serde_yaml::from_str(&entry).unwrap_or_else(|e| panic!("{e}: {entry}"));
        assert_eq!(parsed.len(), 1);
        validate_positions_config(&parsed).unwrap();
        assert_eq!(serde_json::to_value(&parsed[0]).unwrap(), serde_json::to_value(tracked_position_config(&row)).unwrap(), "{entry}");
        assert_eq!(position_key(&parsed[0]), row.position_key);
    }
    let body = found_positions_body(None, &[ladder_row(OPERATOR, 3243807, None)]);
    assert!(body["detection"].is_null());
    assert!(body["tracked_rows"][0]["config_entry"].as_str().unwrap().contains("nft_id: 3243807"));
    assert!(body["rollback_note"].as_str().unwrap().contains("its total leaves these positions out without saying so"));
}

// ------------------------------------------------------------------
// One cycle with rows, against the recorded chain
// ------------------------------------------------------------------

async fn seeded(rows: &[TrackedRow]) -> PositionStore {
    let store = store_at(clock("operator"));
    for r in rows {
        assert!(store.insert_tracked_if_absent(r).await.unwrap());
    }
    store
}

fn positions_json(raw: &RawOnChainData) -> Value {
    serde_json::to_value(snapshot_positions(raw)).unwrap()
}

#[tokio::test]
async fn test_with_no_row_the_cycle_reads_exactly_what_it_read_before() {
    let n = node("operator", vec![]).await;
    let cfg = book();
    let http = Client::new();
    let store = seeded(&[]).await;
    let merged = load_cycle_positions(&store, &http, &n.url, &cfg).await.unwrap();
    assert!(merged.report.is_empty() && merged.sources.is_empty());
    let with_store = fetch_raw_data_merged(&http, &n.url, &cfg, &merged, clock("operator")).await.unwrap();
    let before = fetch_raw_data_from(&http, &n.url, &cfg, clock("operator")).await.unwrap();
    // Byte for byte, apart from the wall-clock stamps of the two reads.
    assert_eq!(serde_json::to_string(&canonical_state_json(&cycle_data(with_store))).unwrap(), serde_json::to_string(&canonical_state_json(&cycle_data(before))).unwrap());
    // The onchain reads go through the merged list only: no second source of positions.
    let onchain = include_str!("onchain.rs");
    let body = &onchain[onchain.find("pub(crate) async fn fetch_raw_data_merged(").unwrap()..onchain.find("pub(crate) const USDC_BASE_LC").unwrap()];
    assert!(!body.contains("config.positions"), "fetch_raw_data_merged reads merged.positions, never config.positions");
    assert!(onchain.contains("load_cycle_positions(&PositionStore::of(state), &state.http_client, &alchemy_url, &state.config).await?"), "a table that cannot be read fails the cycle");
}

#[tokio::test]
async fn test_an_unreadable_table_or_an_unread_owner_fails_the_cycle_and_never_drops_a_position() {
    let cfg = book_without_v4();
    let http = Client::new();
    // The table cannot be read.
    let store = seeded(&[row_3163450(Some("ladder"))]).await;
    if let PositionStore::Mem(m) = &store {
        m.lock().unwrap().unreadable = true;
    }
    let n = node("operator", vec![]).await;
    let e = load_cycle_positions(&store, &http, &n.url, &cfg).await.unwrap_err().to_string();
    assert_eq!(e, TRACKED_TABLE_UNREADABLE);
    assert!(!e.contains("test") && !e.contains("postgres"), "the database's own error is logged, not published");

    // The row's owner cannot be read (the connection drops, every attempt).
    let store = seeded(&[row_3163450(Some("ladder"))]).await;
    let n = node("operator", vec![(key_owner(3163450), Swap::Drop)]).await;
    let e = load_cycle_positions(&store, &http, &n.url, &cfg).await.unwrap_err().to_string();
    assert!(e.starts_with("tracked position uniswap-v4:base:3163450 (NFT #3163450): its owner could not be read ("), "{e}");
    assert!(e.ends_with("the cycle stops rather than count or drop it unread"), "{e}");
    assert!(!e.contains(&n.url) && !e.contains("127.0.0.1") && !e.contains("http"), "{e}");
    // An answer that is not an address is not read as "someone else".
    let n = node("operator", vec![(key_owner(3163450), Swap::Json(json!({ "result": "0x" })))]).await;
    assert!(load_cycle_positions(&store, &http, &n.url, &cfg).await.unwrap_err().to_string().contains("answered something that is not an address"));
}

#[tokio::test]
async fn test_a_row_whose_nft_left_the_wallet_stops_being_counted_and_is_named() {
    let cfg = book_without_v4();
    let n = node("operator", vec![(key_owner(3163450), Swap::Json(json!({ "result": format!("0x{:0>64}", &STRANGER[2..]) })))]).await;
    let store = seeded(&[row_3163450(Some("ladder"))]).await;
    let raw = read_cycle(&store, &n, &cfg).await.unwrap();
    // Liquidity above zero used to be all a v4 read asked for. The row is not read at all.
    assert!(raw.positions.iter().all(|p| p.nft_id != 3163450));
    assert!(raw.position_sources.is_empty() && raw.ladders.is_empty());
    assert_eq!(raw.tracked_report[0].fate, TrackedFate::NotOwned { owner: STRANGER.to_string() });
    assert_eq!(n.count(&key_liquidity(3163450)), 0);
    let block = position_detection_block(&DetectionOutcome { status: "ok".to_string(), ..Default::default() }, &raw);
    assert_eq!(block["tracked"][0]["state"], "not_owned");
    assert_eq!(block["tracked"][0]["note"], "NFT #3163450 is now owned by 0x4598…e826: it is no longer counted. UNTRACK removes it from this list.");
    assert_eq!(block["tracked"][0]["choices"], json!(["untrack"]));
    // The same book without the row reads the same positions.
    let without = read_cycle(&seeded(&[]).await, &n, &cfg).await.unwrap();
    assert_eq!(positions_json(&raw), positions_json(&without));
}

#[tokio::test]
async fn test_an_emptied_rung_reads_as_empty_and_a_burned_one_as_closed() {
    let cfg = book();
    let n = node("operator", vec![]).await;
    let config_only = read_cycle(&seeded(&[]).await, &n, &cfg).await.unwrap();

    // EMPTIED: the wallet's own NFT #3163431 exists and holds nothing.
    let store = seeded(&[row_emptied()]).await;
    let raw = read_cycle(&store, &n, &cfg).await.unwrap();
    let p = raw.positions.iter().find(|p| p.nft_id == 3163431).expect("an NFT that exists is a live position, holding nothing");
    assert_eq!((p.liquidity, p.amount0, p.amount1), (0, 0.0, 0.0));
    let ladders = ladder_readings(&raw);
    assert_eq!((ladders[0].state.as_str(), ladders[0].rung.as_deref(), ladders[0].source.as_deref()), ("empty", Some("near"), Some("tracked")));
    assert_eq!(ladders[0].state_text, "empty: the NFT holds no liquidity");
    assert!(raw.closed_positions.is_empty());
    let block = position_detection_block(&DetectionOutcome { status: "ok".to_string(), ..Default::default() }, &raw);
    assert_eq!(block["tracked"][0]["state"], "empty");
    assert_eq!(block["tracked"][0]["note"], "it holds nothing (emptied; the NFT is still in the wallet). It is not a deployed rung.");
    // It adds nothing to any total, and it does not stand in for a deployed ladder: the
    // suggestion measures a new one.
    let (with, without) = (cycle_data(raw), cycle_data(read_cycle(&seeded(&[]).await, &n, &cfg).await.unwrap()));
    let gs = |d: &SnapshotData| (d.derived_metrics.global_status.total_portfolio_usd.clone(), d.derived_metrics.global_status.total_nock_amount.clone(), d.derived_metrics.global_status.total_usdc_amount.clone());
    assert_eq!(gs(&with), gs(&without));
    let env = SnapshotEnvelope {
        meta: SnapshotMeta { generated_at: clock("operator"), state_hash: String::new(), version: SNAPSHOT_SPEC_VERSION.to_string(), snapshot_id: Uuid::nil() },
        data: with,
    };
    let l = ladder_suggestion(&env, Ok(Some(0.01)));
    assert_ne!(l["status"], "ladder_tracked");
    assert_eq!(l["tracked"][0]["state"], "empty");
    assert_eq!(l["tracked"][0]["source"], "tracked");

    // BURNED: a token id that no longer exists goes through the #144 proof unchanged
    // (two reads at one current block), and its texts name UNTRACK, not config.yaml.
    let burned = TrackedRow { rung: Some("deep".to_string()), ..ladder_row(OPERATOR, V4_BURNED_NFT, Some(60_000_000)) };
    let store = seeded(&[burned]).await;
    let raw = read_cycle(&store, &n, &cfg).await.unwrap();
    assert!(raw.positions.iter().all(|p| p.nft_id != V4_BURNED_NFT), "a closed position is in no position list");
    assert_eq!(positions_json(&raw), positions_json(&config_only), "and changes no figure of the live ones");
    assert_eq!(raw.closed_positions.len(), 1);
    let c = &raw.closed_positions[0];
    assert_eq!((c.position_id.as_str(), c.config_id.as_str(), c.source.as_deref(), c.status.as_str()), ("uniswap-v4:base:3229273", "tracked-v4-3229273", Some("tracked"), "closed"));
    assert_eq!(c.label, "v4 USDC/NOCK #3229273 (tracked-v4-3229273, ladder)");
    assert_eq!(c.evidence.len(), 2);
    let note = closed_positions_note(&raw.closed_positions).unwrap();
    assert!(note.ends_with("v4 USDC/NOCK #3229273 (tracked-v4-3229273, ladder) was tracked from the chain, not from config.yaml: UNTRACK removes it from this list."), "{note}");
    assert!(!note.contains("can be removed from config.yaml"), "{note}");
    let pushed = closed_position_notes(Some(&["uniswap-v4:base:3229273".to_string()]), &raw.closed_positions);
    assert!(pushed[0].ends_with("It is no longer counted in any total. It was tracked from the chain, not from config.yaml: UNTRACK removes it from the list."), "{}", pushed[0]);
    let text = closed_position_statement(&serde_json::to_value(c).unwrap());
    assert!(text.ends_with("Not counted in any total. It was tracked from the chain, not from config.yaml: UNTRACK in the dashboard removes it from this list."), "{text}");
    let block = position_detection_block(&DetectionOutcome { status: "ok".to_string(), ..Default::default() }, &raw);
    assert_eq!(block["tracked"][0]["state"], "closed");
    assert_eq!(block["tracked"][0]["note"], "NFT #3229273 no longer exists (withdrawn and burned): it is in no total. UNTRACK removes it from this list.");
    // A closed config entry keeps the wording it had.
    let mut config_closed = raw.closed_positions.clone();
    config_closed[0].source = None;
    assert!(closed_positions_note(&config_closed).unwrap().ends_with("Its entry can be removed from config.yaml."));
    assert!(closed_position_statement(&serde_json::to_value(&config_closed[0]).unwrap()).ends_with("from config.yaml."));
}

#[tokio::test]
async fn test_config_wins_a_collision_and_a_quarantined_row_does_not_stall_the_cycle() {
    let cfg = book();
    let n = node("operator", vec![]).await;
    let config_only = read_cycle(&seeded(&[]).await, &n, &cfg).await.unwrap();
    // config.yaml lists #3163450 (its price source); a row for the same NFT, as a ladder.
    let quarantined = TrackedRow { currency0: WETH.to_string(), ..ladder_row(OPERATOR, 12, None) };
    let store = seeded(&[row_3163450(Some("ladder")), quarantined]).await;
    let raw = read_cycle(&store, &n, &cfg).await.expect("neither row stalls the cycle");
    assert_eq!(positions_json(&raw), positions_json(&config_only), "the config entry is the one read, exactly as before");
    assert!(raw.ladders.is_empty() && raw.position_sources.is_empty(), "the row's role does not reach the config entry");
    assert!(raw.positions.iter().find(|p| p.nft_id == 3163450).unwrap().is_price_source);
    let block = position_detection_block(&DetectionOutcome { status: "ok".to_string(), ..Default::default() }, &raw);
    assert_eq!(block["tracked"][0]["state"], "shadowed_by_config");
    assert_eq!(block["tracked"][0]["note"], "config.yaml also lists this position (uniswap-v4-nock-usdc-wide): the config entry is the one read. UNTRACK removes this row.");
    assert_eq!(block["tracked"][1]["state"], "quarantined");
    assert_eq!(block["tracked"][1]["note"], "not read and not counted: it is tracked as a ladder but is a WETH/NOCK pair: a ladder is a USDC/NOCK range. UNTRACK removes it from this list.");
    // The shadowed row can be removed with one tap, and nothing else changes.
    let done = untrack_position(&store, OPERATOR, &cfg.positions, "uniswap-v4:base:3163450").await.unwrap();
    assert_eq!(done["status"], "untracked");
    assert_eq!(store.tracked(OPERATOR).await.unwrap().len(), 1);
}

#[tokio::test]
async fn test_a_position_tracked_as_lp_is_counted_as_lp_held_and_is_never_a_rebuild_template() {
    let cfg = book_without_v4();
    let n = node("operator", vec![]).await;
    let without = read_cycle(&seeded(&[]).await, &n, &cfg).await.unwrap();
    let with = read_cycle(&seeded(&[row_3163450(None)]).await, &n, &cfg).await.unwrap();
    // TRACK AS LP is the operator's tap: by its pair it is NOCK LP, as a config entry
    // with no role is. It is held, so it is counted in what LP holds.
    let p = with.positions.iter().find(|p| p.nft_id == 3163450).unwrap();
    assert!(is_nock_lp(&with, p) && !is_ladder(&with, p));
    let held = snapshot_positions(&with).into_iter().find(|p| p.nft_id == 3163450).unwrap();
    assert_eq!((held.source.as_deref(), held.amount_usd.as_str()), (Some("tracked"), "240.61"));
    // It is never a rebuild template: no deposit into it is ever suggested, whatever the
    // target and the funds.
    let funds = RebuildFunds { nock: 1e9, floor_tolerance_nock: 0.0, floor_set: false, usdc: 1e9 };
    let (s_with, x_with) = lp_rebuild_suggestions(&with, 5_000.0, funds);
    let (s_without, _) = lp_rebuild_suggestions(&without, 5_000.0, funds);
    assert!(s_with.iter().all(|s| s.nft_id != 3163450));
    assert_eq!(x_with.iter().find(|x| x.nft_id == 3163450).unwrap().reason, LP_TRACKED_REASON);
    // The other position's share of the target is the same position, the same link and
    // the same kind of deposit; what is left to add is smaller by exactly what the
    // tracked position holds, because that is LP already held.
    assert_eq!((s_with.len(), s_without.len()), (1, 1));
    assert_eq!((s_with[0].position_id.as_str(), s_with[0].url.as_str()), (s_without[0].position_id.as_str(), s_without[0].url.as_str()));
    let f = |v: &str| v.parse::<f64>().unwrap();
    assert!((f(&s_without[0].target_usd) - f(&s_with[0].target_usd) - 240.61).abs() < 0.011, "{} vs {}", s_without[0].target_usd, s_with[0].target_usd);
    assert_eq!(s_with[0].current_usd, s_without[0].current_usd);
    // A LADDER row is not NOCK LP at all: it leaves the rebuild exactly as it was.
    let ladder = read_cycle(&seeded(&[row_3163450(Some("ladder"))]).await, &n, &cfg).await.unwrap();
    let (s_ladder, x_ladder) = lp_rebuild_suggestions(&ladder, 5_000.0, funds);
    assert_eq!(serde_json::to_value(&s_ladder).unwrap(), serde_json::to_value(&s_without).unwrap(), "a detected position (always a ladder) changes no rebuild figure");
    assert!(x_ladder.iter().all(|x| x.nft_id != 3163450), "neither a template nor an exclusion");
    // The same NFT listed in config.yaml with no rebuild key IS a template: the
    // difference is the row's rule, not the position.
    let (s_config, _) = lp_rebuild_suggestions(&read_cycle(&seeded(&[]).await, &n, &book()).await.unwrap(), 5_000.0, funds);
    assert!(s_config.iter().any(|s| s.nft_id == 3163450));
}

#[tokio::test]
async fn test_claims_cover_a_row_only_once_a_cycle_has_read_it() {
    let cfg = book_without_v4();
    let n = node("operator", vec![]).await;
    let row = row_3163450(Some("ladder"));
    // Tracked a moment ago: the latest snapshot does not hold it yet.
    let before = cycle_data(read_cycle(&seeded(&[]).await, &n, &cfg).await.unwrap());
    let ids = |c: Vec<PositionConfig>| c.iter().map(|p| p.id.clone()).collect::<Vec<_>>();
    assert_eq!(ids(claim_position_configs(&cfg.positions, std::slice::from_ref(&row), &before)), vec!["uniswap-v3-weth-nock", "aero-basic-nock-usdc"]);
    // After a cycle read it: a collect action like any v4 entry's.
    let after = cycle_data(read_cycle(&seeded(std::slice::from_ref(&row)).await, &n, &cfg).await.unwrap());
    let claims = claim_position_configs(&cfg.positions, std::slice::from_ref(&row), &after);
    assert_eq!(ids(claims.clone()), vec!["uniswap-v3-weth-nock", "aero-basic-nock-usdc", "tracked-v4-3163450"]);
    let live = live_position_configs(&claims, &after.closed_positions);
    assert_eq!(live.len(), 3);
    // A row config.yaml shadows is not listed twice, and a quarantined one not at all.
    let with_v4 = book();
    let snapshot = cycle_data(read_cycle(&seeded(&[]).await, &n, &with_v4).await.unwrap());
    let quarantined = TrackedRow { protocol: "uniswap-v3".to_string(), ..row_3163450(None) };
    assert_eq!(claim_position_configs(&with_v4.positions, &[row.clone(), quarantined], &snapshot).len(), 3);
    // The handler reads the table and says so when it cannot.
    let claims_src = include_str!("claims.rs");
    assert!(claims_src.contains("TRACKED_POSITIONS_UNAVAILABLE"));
}
