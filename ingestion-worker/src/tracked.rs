//! Positions tracked from the database (v0.9.0; #145 for Uniswap v4).
//!
//! A position the operator deposited in the Uniswap app is found on-chain
//! (position_detect.rs) and tracked here, with no config.yaml edit. Each ingestion cycle
//! appends these rows after config.yaml's positions:
//!
//! - config.yaml wins. A row for a position config.yaml also lists is not merged and is
//!   reported as shadowed.
//! - A row is never a price source, never a rebuild template and never carries a deposit
//!   link. A ladder row is left out of NOCK LP by its role (#167); a row with no role is
//!   classified by its pair as a config entry is, with `rebuild_weight_bps: 0`.
//! - A row whose NFT another address now owns is not merged and not counted, and the
//!   snapshot names it. A row whose NFT does not exist goes through the #144 closed proof
//!   like a config entry. A row that holds nothing reads as an empty position.
//! - A row that would stall the cycle (an unknown protocol, a ladder that is not a
//!   USDC/NOCK range) is quarantined: not merged, named with its reason.
//! - A table that cannot be read fails the cycle. A total published without the
//!   operator's tracked positions would be a false total.
//!
//! The SQL is in the `pg_*` functions and nowhere else. `PositionStore::Mem` holds the
//! same rules in memory for the tests of everything built on the store; the store
//! contract (tests_tracked.rs) runs against both, the PostgreSQL half when
//! NOCKNOCK_SCRATCH_DATABASE_URL names a scratch database.
use crate::*;

pub(crate) const TRACKED_PROTOCOL_V4: &str = "uniswap-v4";
pub(crate) const TRACKED_NETWORK_BASE: &str = "base";
/// `tracked_positions.source`: how the row came to be tracked.
pub(crate) const TRACKED_SOURCE_DETECTED: &str = "detected";
pub(crate) const TRACKED_SOURCE_CONFIRMED: &str = "confirmed";
/// `positions[].source` for a position that comes from a database row. A config entry
/// carries no source.
pub(crate) const POSITION_SOURCE_TRACKED: &str = "tracked";
/// `position_candidates.choice`: the operator's "not now", or an untrack.
pub(crate) const CHOICE_DISMISSED: &str = "dismissed";

/// What an unset rung reads as on a database-tracked ladder. Never a default rung.
pub(crate) const RUNG_UNSET_NOTE_TRACKED: &str =
    "no rung is set: the suggestion it matched named none, and NockNock assumes neither near nor deep. One tap sets it";

/// One row of `tracked_positions`.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct TrackedRow {
    /// Lowercase.
    pub(crate) wallet: String,
    pub(crate) position_key: String,
    pub(crate) protocol: String,
    pub(crate) network: String,
    pub(crate) nft_id: u64,
    pub(crate) pool_id: String,
    pub(crate) currency0: String,
    pub(crate) currency1: String,
    pub(crate) tick_lower: i32,
    pub(crate) tick_upper: i32,
    pub(crate) tick_spacing: Option<i32>,
    pub(crate) role: Option<String>,
    pub(crate) rung: Option<String>,
    pub(crate) deposit_tx: Option<String>,
    pub(crate) mint_block: Option<i64>,
    pub(crate) source: String,
    /// Set by the database; None on a row not yet stored.
    pub(crate) tracked_at: Option<DateTime<Utc>>,
    /// The liquidity its verified mint receipt added; None until read.
    pub(crate) deposit_liquidity: Option<u128>,
    /// The most liquidity a cycle has read it holding; None until read.
    pub(crate) max_liquidity_seen: Option<u128>,
}

/// One row of `position_candidates`: what the chain said about one NFT when last read.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct CandidateRow {
    pub(crate) wallet: String,
    pub(crate) protocol: String,
    pub(crate) network: String,
    pub(crate) nft_id: u64,
    /// eligible | not_whitelisted | hooks | unsupported | gone
    pub(crate) verdict: String,
    /// None, or `dismissed`.
    pub(crate) choice: Option<String>,
    pub(crate) owned: bool,
    pub(crate) has_liquidity: Option<bool>,
    pub(crate) pool_id: Option<String>,
    pub(crate) pool_id_prefix: Option<String>,
    pub(crate) currency0: Option<String>,
    pub(crate) currency1: Option<String>,
    pub(crate) fee_raw: Option<i64>,
    pub(crate) tick_spacing: Option<i32>,
    pub(crate) hooks: Option<String>,
    pub(crate) tick_lower: Option<i32>,
    pub(crate) tick_upper: Option<i32>,
    pub(crate) received_tx: Option<String>,
    pub(crate) received_block: Option<i64>,
    pub(crate) received_from: Option<String>,
    pub(crate) mint_tx: Option<String>,
    pub(crate) mint_block: Option<i64>,
    pub(crate) minted_by: Option<String>,
    /// `ok`, or parse_ladder_deposit's refusal. None when not read.
    pub(crate) deposit_check: Option<String>,
}

/// One row of `position_detection_state`: how far the transfers were read.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct ScanState {
    pub(crate) logs_from_block: Option<i64>,
    pub(crate) logs_to_block: Option<i64>,
    pub(crate) history_complete_at: Option<DateTime<Utc>>,
    pub(crate) last_checked_at: Option<DateTime<Utc>>,
    /// Where the next cycle's re-reads of untracked NFTs start.
    pub(crate) refresh_cursor: i64,
}

/// The tables of `PositionStore::Mem`.
#[cfg(test)]
#[derive(Debug, Default)]
pub(crate) struct MemTables {
    pub(crate) tracked: Vec<TrackedRow>,
    pub(crate) candidates: Vec<CandidateRow>,
    pub(crate) scan: HashMap<(String, String, String), ScanState>,
    /// When set, every call fails: an unreadable database.
    pub(crate) unreadable: bool,
    /// The clock `tracked_at` is stamped from.
    pub(crate) now: Option<DateTime<Utc>>,
}

/// Where the three tables live. `Pg` is the worker's database; `Mem` exists only in tests.
pub(crate) enum PositionStore {
    Pg(PgPool),
    #[cfg(test)]
    Mem(std::sync::Mutex<MemTables>),
}

fn lc(s: &str) -> String {
    s.trim().to_lowercase()
}

fn nft_i64(nft_id: u64) -> Result<i64> {
    i64::try_from(nft_id).map_err(|_| anyhow::anyhow!("NFT id {nft_id} does not fit the database column"))
}

#[cfg(test)]
fn mem_unreadable() -> anyhow::Error {
    anyhow::anyhow!("the database is unreachable (test)")
}

impl PositionStore {
    pub(crate) fn of(state: &AppState) -> PositionStore {
        PositionStore::Pg(state.db.clone())
    }

    #[cfg(test)]
    pub(crate) fn mem() -> PositionStore {
        PositionStore::Mem(std::sync::Mutex::new(MemTables::default()))
    }

    /// The wallet's tracked rows, oldest mint first (rows with no mint block last), then
    /// by NFT id. A row that cannot be decoded is an error, never a row left out.
    pub(crate) async fn tracked(&self, wallet: &str) -> Result<Vec<TrackedRow>> {
        let wallet = lc(wallet);
        match self {
            PositionStore::Pg(db) => pg_tracked(db, &wallet).await,
            #[cfg(test)]
            PositionStore::Mem(m) => {
                let t = m.lock().unwrap();
                if t.unreadable {
                    return Err(mem_unreadable());
                }
                let mut rows: Vec<TrackedRow> = t.tracked.iter().filter(|r| r.wallet == wallet).cloned().collect();
                rows.sort_by_key(|r| (r.mint_block.is_none(), r.mint_block, r.nft_id));
                Ok(rows)
            }
        }
    }

    /// Track a position detection matched to a rung. A row that already exists is left
    /// exactly as it is (an operator's tap is never overwritten). True when it was added.
    pub(crate) async fn insert_tracked_if_absent(&self, row: &TrackedRow) -> Result<bool> {
        let row = TrackedRow { wallet: lc(&row.wallet), ..row.clone() };
        match self {
            PositionStore::Pg(db) => pg_write_tracked(db, &row, false).await,
            #[cfg(test)]
            PositionStore::Mem(m) => {
                let mut t = m.lock().unwrap();
                if t.unreadable {
                    return Err(mem_unreadable());
                }
                if t.tracked.iter().any(|r| r.wallet == row.wallet && r.position_key == row.position_key) {
                    return Ok(false);
                }
                let at = t.now.unwrap_or_else(Utc::now);
                t.tracked.push(TrackedRow { tracked_at: Some(at), ..row });
                Ok(true)
            }
        }
    }

    /// Track a position by the operator's tap, or change how a tracked one is tracked:
    /// role, rung and source are replaced; a known deposit_tx is kept when the new row
    /// has none; the first tracked_at stays.
    pub(crate) async fn upsert_tracked(&self, row: &TrackedRow) -> Result<()> {
        let row = TrackedRow { wallet: lc(&row.wallet), ..row.clone() };
        match self {
            PositionStore::Pg(db) => pg_write_tracked(db, &row, true).await.map(|_| ()),
            #[cfg(test)]
            PositionStore::Mem(m) => {
                let mut t = m.lock().unwrap();
                if t.unreadable {
                    return Err(mem_unreadable());
                }
                let at = t.now.unwrap_or_else(Utc::now);
                match t.tracked.iter_mut().find(|r| r.wallet == row.wallet && r.position_key == row.position_key) {
                    Some(old) => {
                        old.role = row.role.clone();
                        old.rung = row.rung.clone();
                        old.source = row.source.clone();
                        if row.deposit_tx.is_some() {
                            old.deposit_tx = row.deposit_tx.clone();
                        }
                        old.deposit_liquidity = old.deposit_liquidity.or(row.deposit_liquidity);
                    }
                    None => t.tracked.push(TrackedRow { tracked_at: Some(at), ..row }),
                }
                Ok(())
            }
        }
    }

    /// Remember what a tracked rung stands for: its deposit's liquidity (kept once known)
    /// and the most liquidity a cycle has read it holding (`max_seen`, already the larger
    /// of the stored value and this cycle's read). True when a row was updated.
    pub(crate) async fn record_liquidity(&self, wallet: &str, position_key: &str, deposit: Option<u128>, max_seen: u128) -> Result<bool> {
        let wallet = lc(wallet);
        match self {
            PositionStore::Pg(db) => {
                let done = sqlx::query(
                    "UPDATE tracked_positions SET deposit_liquidity = COALESCE(deposit_liquidity, $3), max_liquidity_seen = $4, updated_at = NOW() \
                     WHERE wallet = $1 AND position_key = $2",
                )
                .bind(&wallet)
                .bind(position_key)
                .bind(deposit.map(|d| d.to_string()))
                .bind(max_seen.to_string())
                .execute(db)
                .await?;
                Ok(done.rows_affected() > 0)
            }
            #[cfg(test)]
            PositionStore::Mem(m) => {
                let mut t = m.lock().unwrap();
                if t.unreadable {
                    return Err(mem_unreadable());
                }
                match t.tracked.iter_mut().find(|r| r.wallet == wallet && r.position_key == position_key) {
                    Some(r) => {
                        r.deposit_liquidity = r.deposit_liquidity.or(deposit);
                        r.max_liquidity_seen = Some(max_seen);
                        Ok(true)
                    }
                    None => Ok(false),
                }
            }
        }
    }

    /// Stop tracking. True when a row was removed.
    pub(crate) async fn delete_tracked(&self, wallet: &str, position_key: &str) -> Result<bool> {
        let wallet = lc(wallet);
        match self {
            PositionStore::Pg(db) => {
                let done = sqlx::query("DELETE FROM tracked_positions WHERE wallet = $1 AND position_key = $2")
                    .bind(&wallet)
                    .bind(position_key)
                    .execute(db)
                    .await?;
                Ok(done.rows_affected() > 0)
            }
            #[cfg(test)]
            PositionStore::Mem(m) => {
                let mut t = m.lock().unwrap();
                if t.unreadable {
                    return Err(mem_unreadable());
                }
                let before = t.tracked.len();
                t.tracked.retain(|r| !(r.wallet == wallet && r.position_key == position_key));
                Ok(t.tracked.len() < before)
            }
        }
    }

    /// Every NFT seen arriving in the wallet, by NFT id.
    pub(crate) async fn candidates(&self, wallet: &str) -> Result<Vec<CandidateRow>> {
        let wallet = lc(wallet);
        match self {
            PositionStore::Pg(db) => pg_candidates(db, &wallet).await,
            #[cfg(test)]
            PositionStore::Mem(m) => {
                let t = m.lock().unwrap();
                if t.unreadable {
                    return Err(mem_unreadable());
                }
                let mut rows: Vec<CandidateRow> = t.candidates.iter().filter(|r| r.wallet == wallet).cloned().collect();
                rows.sort_by_key(|r| r.nft_id);
                Ok(rows)
            }
        }
    }

    /// Store what the chain said about one NFT. The operator's `choice` on an existing
    /// row is kept: only POST dismiss / restore / track / untrack change it.
    pub(crate) async fn upsert_candidate(&self, row: &CandidateRow) -> Result<()> {
        let row = CandidateRow { wallet: lc(&row.wallet), ..row.clone() };
        match self {
            PositionStore::Pg(db) => pg_upsert_candidate(db, &row).await,
            #[cfg(test)]
            PositionStore::Mem(m) => {
                let mut t = m.lock().unwrap();
                if t.unreadable {
                    return Err(mem_unreadable());
                }
                let same = |r: &CandidateRow| r.wallet == row.wallet && r.protocol == row.protocol && r.network == row.network && r.nft_id == row.nft_id;
                match t.candidates.iter_mut().find(|r| same(r)) {
                    Some(old) => {
                        let choice = old.choice.clone();
                        *old = CandidateRow { choice, ..row };
                    }
                    None => t.candidates.push(CandidateRow { choice: None, ..row }),
                }
                Ok(())
            }
        }
    }

    /// Set or clear the operator's choice on one NFT. False when no such row exists.
    pub(crate) async fn set_choice(&self, wallet: &str, protocol: &str, network: &str, nft_id: u64, choice: Option<&str>) -> Result<bool> {
        let wallet = lc(wallet);
        match self {
            PositionStore::Pg(db) => {
                let done = sqlx::query(
                    "UPDATE position_candidates SET choice = $5, updated_at = NOW() \
                     WHERE wallet = $1 AND protocol = $2 AND network = $3 AND nft_id = $4",
                )
                .bind(&wallet)
                .bind(protocol)
                .bind(network)
                .bind(nft_i64(nft_id)?)
                .bind(choice)
                .execute(db)
                .await?;
                Ok(done.rows_affected() > 0)
            }
            #[cfg(test)]
            PositionStore::Mem(m) => {
                let mut t = m.lock().unwrap();
                if t.unreadable {
                    return Err(mem_unreadable());
                }
                match t.candidates.iter_mut().find(|r| r.wallet == wallet && r.protocol == protocol && r.network == network && r.nft_id == nft_id) {
                    Some(row) => {
                        row.choice = choice.map(str::to_string);
                        Ok(true)
                    }
                    None => Ok(false),
                }
            }
        }
    }

    /// How far the transfers were read. The default (nothing read) when there is no row.
    pub(crate) async fn scan_state(&self, wallet: &str, protocol: &str, network: &str) -> Result<ScanState> {
        let wallet = lc(wallet);
        match self {
            PositionStore::Pg(db) => {
                let row = sqlx::query(
                    "SELECT logs_from_block, logs_to_block, history_complete_at, last_checked_at, refresh_cursor \
                     FROM position_detection_state WHERE wallet = $1 AND protocol = $2 AND network = $3",
                )
                .bind(&wallet)
                .bind(protocol)
                .bind(network)
                .fetch_optional(db)
                .await?;
                match row {
                    None => Ok(ScanState::default()),
                    Some(r) => Ok(ScanState {
                        logs_from_block: r.try_get("logs_from_block")?,
                        logs_to_block: r.try_get("logs_to_block")?,
                        history_complete_at: r.try_get("history_complete_at")?,
                        last_checked_at: r.try_get("last_checked_at")?,
                        refresh_cursor: r.try_get("refresh_cursor")?,
                    }),
                }
            }
            #[cfg(test)]
            PositionStore::Mem(m) => {
                let t = m.lock().unwrap();
                if t.unreadable {
                    return Err(mem_unreadable());
                }
                Ok(t.scan.get(&(wallet, protocol.to_string(), network.to_string())).cloned().unwrap_or_default())
            }
        }
    }

    pub(crate) async fn save_scan_state(&self, wallet: &str, protocol: &str, network: &str, s: &ScanState) -> Result<()> {
        let wallet = lc(wallet);
        match self {
            PositionStore::Pg(db) => {
                sqlx::query(
                    "INSERT INTO position_detection_state \
                        (wallet, protocol, network, logs_from_block, logs_to_block, history_complete_at, last_checked_at, refresh_cursor) \
                     VALUES ($1, $2, $3, $4, $5, $6, $7, $8) \
                     ON CONFLICT (wallet, protocol, network) DO UPDATE SET \
                        logs_from_block = EXCLUDED.logs_from_block, logs_to_block = EXCLUDED.logs_to_block, \
                        history_complete_at = EXCLUDED.history_complete_at, last_checked_at = EXCLUDED.last_checked_at, \
                        refresh_cursor = EXCLUDED.refresh_cursor, updated_at = NOW()",
                )
                .bind(&wallet)
                .bind(protocol)
                .bind(network)
                .bind(s.logs_from_block)
                .bind(s.logs_to_block)
                .bind(s.history_complete_at)
                .bind(s.last_checked_at)
                .bind(s.refresh_cursor)
                .execute(db)
                .await?;
                Ok(())
            }
            #[cfg(test)]
            PositionStore::Mem(m) => {
                let mut t = m.lock().unwrap();
                if t.unreadable {
                    return Err(mem_unreadable());
                }
                t.scan.insert((wallet, protocol.to_string(), network.to_string()), s.clone());
                Ok(())
            }
        }
    }
}

// ------------------------------------------------------------------
// SQL
// ------------------------------------------------------------------

async fn pg_tracked(db: &PgPool, wallet: &str) -> Result<Vec<TrackedRow>> {
    let rows = sqlx::query(
        "SELECT wallet, position_key, protocol, network, nft_id, pool_id, currency0, currency1, tick_lower, tick_upper, \
                tick_spacing, role, rung, deposit_tx, mint_block, source, tracked_at, deposit_liquidity, max_liquidity_seen \
         FROM tracked_positions WHERE wallet = $1 ORDER BY mint_block ASC NULLS LAST, nft_id ASC",
    )
    .bind(wallet)
    .fetch_all(db)
    .await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in &rows {
        let nft: i64 = r.try_get("nft_id")?;
        let liquidity = |col: &str| -> Result<Option<u128>> {
            let v: Option<String> = r.try_get(col)?;
            v.map(|s| s.parse::<u128>().map_err(|_| anyhow::anyhow!("tracked_positions.{col} is not a liquidity: {s}"))).transpose()
        };
        out.push(TrackedRow {
            wallet: r.try_get("wallet")?,
            position_key: r.try_get("position_key")?,
            protocol: r.try_get("protocol")?,
            network: r.try_get("network")?,
            nft_id: u64::try_from(nft).map_err(|_| anyhow::anyhow!("tracked_positions holds a negative nft_id"))?,
            pool_id: r.try_get("pool_id")?,
            currency0: r.try_get("currency0")?,
            currency1: r.try_get("currency1")?,
            tick_lower: r.try_get("tick_lower")?,
            tick_upper: r.try_get("tick_upper")?,
            tick_spacing: r.try_get("tick_spacing")?,
            role: r.try_get("role")?,
            rung: r.try_get("rung")?,
            deposit_tx: r.try_get("deposit_tx")?,
            mint_block: r.try_get("mint_block")?,
            source: r.try_get("source")?,
            tracked_at: Some(r.try_get("tracked_at")?),
            deposit_liquidity: liquidity("deposit_liquidity")?,
            max_liquidity_seen: liquidity("max_liquidity_seen")?,
        });
    }
    Ok(out)
}

/// One INSERT for both writes. `replace` false: a conflicting row is left alone (the
/// automatic path). `replace` true: its role, rung and source are replaced (a tap).
async fn pg_write_tracked(db: &PgPool, row: &TrackedRow, replace: bool) -> Result<bool> {
    let on_conflict = if replace {
        "DO UPDATE SET role = EXCLUDED.role, rung = EXCLUDED.rung, source = EXCLUDED.source, \
            deposit_tx = COALESCE(EXCLUDED.deposit_tx, tracked_positions.deposit_tx), \
            deposit_liquidity = COALESCE(tracked_positions.deposit_liquidity, EXCLUDED.deposit_liquidity), updated_at = NOW()"
    } else {
        "DO NOTHING"
    };
    // `on_conflict` is one of the two literals above, never input.
    let sql = format!(
        "INSERT INTO tracked_positions \
            (wallet, position_key, protocol, network, nft_id, pool_id, currency0, currency1, tick_lower, tick_upper, \
             tick_spacing, role, rung, deposit_tx, mint_block, source, deposit_liquidity, max_liquidity_seen) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18) \
         ON CONFLICT (wallet, position_key) {on_conflict}"
    );
    let done = sqlx::query(&sql)
        .bind(&row.wallet)
        .bind(&row.position_key)
        .bind(&row.protocol)
        .bind(&row.network)
        .bind(nft_i64(row.nft_id)?)
        .bind(&row.pool_id)
        .bind(&row.currency0)
        .bind(&row.currency1)
        .bind(row.tick_lower)
        .bind(row.tick_upper)
        .bind(row.tick_spacing)
        .bind(&row.role)
        .bind(&row.rung)
        .bind(&row.deposit_tx)
        .bind(row.mint_block)
        .bind(&row.source)
        .bind(row.deposit_liquidity.map(|d| d.to_string()))
        .bind(row.max_liquidity_seen.map(|d| d.to_string()))
        .execute(db)
        .await?;
    Ok(done.rows_affected() > 0)
}

async fn pg_candidates(db: &PgPool, wallet: &str) -> Result<Vec<CandidateRow>> {
    let rows = sqlx::query(
        "SELECT wallet, protocol, network, nft_id, verdict, choice, owned, has_liquidity, pool_id, pool_id_prefix, \
                currency0, currency1, fee_raw, tick_spacing, hooks, tick_lower, tick_upper, received_tx, received_block, \
                received_from, mint_tx, mint_block, minted_by, deposit_check \
         FROM position_candidates WHERE wallet = $1 ORDER BY nft_id ASC",
    )
    .bind(wallet)
    .fetch_all(db)
    .await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in &rows {
        let nft: i64 = r.try_get("nft_id")?;
        out.push(CandidateRow {
            wallet: r.try_get("wallet")?,
            protocol: r.try_get("protocol")?,
            network: r.try_get("network")?,
            nft_id: u64::try_from(nft).map_err(|_| anyhow::anyhow!("position_candidates holds a negative nft_id"))?,
            verdict: r.try_get("verdict")?,
            choice: r.try_get("choice")?,
            owned: r.try_get("owned")?,
            has_liquidity: r.try_get("has_liquidity")?,
            pool_id: r.try_get("pool_id")?,
            pool_id_prefix: r.try_get("pool_id_prefix")?,
            currency0: r.try_get("currency0")?,
            currency1: r.try_get("currency1")?,
            fee_raw: r.try_get("fee_raw")?,
            tick_spacing: r.try_get("tick_spacing")?,
            hooks: r.try_get("hooks")?,
            tick_lower: r.try_get("tick_lower")?,
            tick_upper: r.try_get("tick_upper")?,
            received_tx: r.try_get("received_tx")?,
            received_block: r.try_get("received_block")?,
            received_from: r.try_get("received_from")?,
            mint_tx: r.try_get("mint_tx")?,
            mint_block: r.try_get("mint_block")?,
            minted_by: r.try_get("minted_by")?,
            deposit_check: r.try_get("deposit_check")?,
        });
    }
    Ok(out)
}

async fn pg_upsert_candidate(db: &PgPool, row: &CandidateRow) -> Result<()> {
    sqlx::query(
        "INSERT INTO position_candidates \
            (wallet, protocol, network, nft_id, verdict, owned, has_liquidity, pool_id, pool_id_prefix, currency0, currency1, \
             fee_raw, tick_spacing, hooks, tick_lower, tick_upper, received_tx, received_block, received_from, mint_tx, \
             mint_block, minted_by, deposit_check) \
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14, $15, $16, $17, $18, $19, $20, $21, $22, $23) \
         ON CONFLICT (wallet, protocol, network, nft_id) DO UPDATE SET \
            verdict = EXCLUDED.verdict, owned = EXCLUDED.owned, has_liquidity = EXCLUDED.has_liquidity, \
            pool_id = EXCLUDED.pool_id, pool_id_prefix = EXCLUDED.pool_id_prefix, currency0 = EXCLUDED.currency0, \
            currency1 = EXCLUDED.currency1, fee_raw = EXCLUDED.fee_raw, tick_spacing = EXCLUDED.tick_spacing, \
            hooks = EXCLUDED.hooks, tick_lower = EXCLUDED.tick_lower, tick_upper = EXCLUDED.tick_upper, \
            received_tx = EXCLUDED.received_tx, received_block = EXCLUDED.received_block, \
            received_from = EXCLUDED.received_from, mint_tx = EXCLUDED.mint_tx, mint_block = EXCLUDED.mint_block, \
            minted_by = EXCLUDED.minted_by, deposit_check = EXCLUDED.deposit_check, updated_at = NOW()",
    )
    .bind(&row.wallet)
    .bind(&row.protocol)
    .bind(&row.network)
    .bind(nft_i64(row.nft_id)?)
    .bind(&row.verdict)
    .bind(row.owned)
    .bind(row.has_liquidity)
    .bind(&row.pool_id)
    .bind(&row.pool_id_prefix)
    .bind(&row.currency0)
    .bind(&row.currency1)
    .bind(row.fee_raw)
    .bind(row.tick_spacing)
    .bind(&row.hooks)
    .bind(row.tick_lower)
    .bind(row.tick_upper)
    .bind(&row.received_tx)
    .bind(row.received_block)
    .bind(&row.received_from)
    .bind(&row.mint_tx)
    .bind(row.mint_block)
    .bind(&row.minted_by)
    .bind(&row.deposit_check)
    .execute(db)
    .await?;
    Ok(())
}

// ------------------------------------------------------------------
// From a row to a position the cycle reads (pure)
// ------------------------------------------------------------------

/// The key a v4 NFT has in `positions[]` and in `tracked_positions`.
pub(crate) fn v4_position_key(network: &str, nft_id: u64) -> String {
    format!("{TRACKED_PROTOCOL_V4}:{network}:{nft_id}")
}

/// The key an Aero CL NFT has in `positions[]` and in `tracked_positions` (#181): the
/// same key adapters::position_key gives an `aerodrome-cl` config entry, so a config entry
/// for the same NFT shadows the row.
pub(crate) fn aero_cl_position_key(network: &str, nft_id: u64) -> String {
    format!("{AERO_CL_PROTOCOL}:{network}:{nft_id}")
}

/// The `id` a database row carries where a config entry carries its own.
pub(crate) fn tracked_config_id(row: &TrackedRow) -> String {
    if row.protocol == AERO_CL_PROTOCOL {
        return format!("tracked-bet-{}", row.nft_id);
    }
    format!("tracked-v4-{}", row.nft_id)
}

/// The position a row stands for, as the cycle reads a config entry. Never a price
/// source and never with a deposit link. A ladder is left out of NOCK LP by its role; any
/// other row is never a rebuild target (`rebuild_weight_bps: 0`). Pure.
pub(crate) fn tracked_position_config(row: &TrackedRow) -> PositionConfig {
    // A bet (#181): an Aero CL NFT, read like an `aerodrome-cl` config entry (its pool,
    // tokens, ticks and staking are read and checked every cycle by the adapter).
    if row.protocol == AERO_CL_PROTOCOL {
        return PositionConfig {
            id: tracked_config_id(row),
            protocol: row.protocol.clone(),
            network: row.network.clone(),
            nft_id: row.nft_id,
            fee_bps: None,
            gauge_address: None,
            deposit_url: None,
            pool_id: None,
            npm_address: Some(AERO_CL_POSITION_MANAGER_BASE.to_string()),
            pool_address: Some(row.pool_id.clone()),
            currency0: None,
            currency1: None,
            tick_lower: None,
            tick_upper: None,
            is_price_source: false,
            rebuild_weight_bps: None,
            rebuild_when_out_of_range: false,
            role: row.role.clone(),
            rung: None,
            deposit_tx: None,
        };
    }
    let ladder = row.role.as_deref() == Some(LADDER_ROLE);
    PositionConfig {
        id: tracked_config_id(row),
        protocol: row.protocol.clone(),
        network: row.network.clone(),
        nft_id: row.nft_id,
        fee_bps: None,
        gauge_address: None,
        deposit_url: None,
        pool_id: Some(row.pool_id.clone()),
        npm_address: None,
        pool_address: None,
        currency0: Some(row.currency0.clone()),
        currency1: Some(row.currency1.clone()),
        tick_lower: Some(row.tick_lower),
        tick_upper: Some(row.tick_upper),
        is_price_source: false,
        // validate_role_config refuses the key on a ladder: the role already leaves it out.
        rebuild_weight_bps: if ladder { None } else { Some(0) },
        rebuild_when_out_of_range: false,
        role: row.role.clone(),
        rung: if ladder { row.rung.clone() } else { None },
        deposit_tx: if ladder { row.deposit_tx.clone() } else { None },
    }
}

/// Would this row stall the cycle or be read as something it is not? `Err` is the reason
/// it is quarantined. Every check here is one a config entry would fail the cycle on;
/// a row is the worker's own writing, so it is set aside and named instead. Pure.
pub(crate) fn validate_tracked_row(row: &TrackedRow) -> std::result::Result<(), String> {
    if row.protocol == AERO_CL_PROTOCOL && row.network == TRACKED_NETWORK_BASE {
        return validate_bet_row(row);
    }
    if row.protocol != TRACKED_PROTOCOL_V4 || row.network != TRACKED_NETWORK_BASE {
        return Err(format!("protocol {} on {} is not one NockNock tracks from the chain (Uniswap v4 on Base only)", row.protocol, row.network));
    }
    if row.position_key != v4_position_key(&row.network, row.nft_id) {
        return Err(format!("its key {} does not name NFT #{}", row.position_key, row.nft_id));
    }
    let pool = row.pool_id.trim_start_matches("0x");
    if pool.len() != 64 || !pool.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err("its pool id is not a 32-byte id".to_string());
    }
    let (t0, t1) = match (token_info(&row.currency0), token_info(&row.currency1)) {
        (Ok(a), Ok(b)) => (a, b),
        _ => return Err(format!("its pool's tokens ({}, {}) are not both in the token registry", row.currency0, row.currency1)),
    };
    if row.tick_lower >= row.tick_upper {
        return Err(format!("its ticks {}..{} are not a range", row.tick_lower, row.tick_upper));
    }
    let cfg = tracked_position_config(row);
    validate_positions_config(std::slice::from_ref(&cfg)).map_err(|e| e.to_string())?;
    if is_ladder_config(&cfg) {
        let usdc_nock = (is_usdc_token(&t0) && is_nock_token(&t1)) || (is_nock_token(&t0) && is_usdc_token(&t1));
        if !usdc_nock {
            return Err(format!("it is tracked as a ladder but is a {}/{} pair: a ladder is a USDC/NOCK range", t0.symbol, t1.symbol));
        }
        let (at_min, at_max) = ladder_bounds_at_extremes(row.tick_lower, row.tick_upper, row.tick_spacing.filter(|s| *s > 0));
        if is_full_range(row.tick_lower, row.tick_upper) || (at_min && at_max) {
            return Err("it is tracked as a ladder but spans every price: a ladder is a concentrated range".to_string());
        }
    } else if row.role.is_some() {
        return Err(format!("its role '{}' is not one NockNock knows", row.role.clone().unwrap_or_default()));
    }
    if !matches!(row.source.as_str(), TRACKED_SOURCE_DETECTED | TRACKED_SOURCE_CONFIRMED) {
        return Err(format!("its source '{}' is not one NockNock knows", row.source));
    }
    Ok(())
}

/// The checks of a bet row (#181): an Aero CL WETH/NOCK range tracked as `role: bet`. Pure.
fn validate_bet_row(row: &TrackedRow) -> std::result::Result<(), String> {
    if row.role.as_deref() != Some(BET_ROLE) {
        return Err(format!("an Aero CL row must be a bet; its role is {}", row.role.clone().unwrap_or_else(|| "unset".to_string())));
    }
    if row.position_key != aero_cl_position_key(&row.network, row.nft_id) {
        return Err(format!("its key {} does not name NFT #{}", row.position_key, row.nft_id));
    }
    if !is_address(&row.pool_id) {
        return Err("its pool is not a contract address".to_string());
    }
    let (t0, t1) = match (token_info(&row.currency0), token_info(&row.currency1)) {
        (Ok(a), Ok(b)) => (a, b),
        _ => return Err(format!("its pool's tokens ({}, {}) are not both in the token registry", row.currency0, row.currency1)),
    };
    let weth_nock = (t0.symbol == "WETH" && is_nock_token(&t1)) || (is_nock_token(&t0) && t1.symbol == "WETH");
    if !weth_nock {
        return Err(format!("it is tracked as a bet but is a {}/{} pair: a bet is a WETH/NOCK range", t0.symbol, t1.symbol));
    }
    if row.tick_lower >= row.tick_upper {
        return Err(format!("its ticks {}..{} are not a range", row.tick_lower, row.tick_upper));
    }
    if row.rung.is_some() {
        return Err("a bet has no rung".to_string());
    }
    validate_positions_config(std::slice::from_ref(&tracked_position_config(row))).map_err(|e| e.to_string())?;
    if !matches!(row.source.as_str(), TRACKED_SOURCE_DETECTED | TRACKED_SOURCE_CONFIRMED) {
        return Err(format!("its source '{}' is not one NockNock knows", row.source));
    }
    Ok(())
}

/// What became of one database row in this cycle's position list.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum TrackedFate {
    /// Appended to the cycle's positions.
    Merged,
    /// config.yaml lists the same position: the config entry is the one read.
    ShadowedByConfig { config_id: String },
    /// Another address owns the NFT now: not read, not counted.
    NotOwned { owner: String },
    /// It would stall the cycle or be misread: not read, not counted.
    Quarantined { reason: String },
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TrackedReport {
    pub(crate) row: TrackedRow,
    pub(crate) fate: TrackedFate,
}

/// The cycle's position list: config.yaml's entries, then the database rows.
#[derive(Debug, Clone, Default)]
pub(crate) struct MergedPositions {
    pub(crate) positions: Vec<PositionConfig>,
    /// position_key -> `tracked`, for every row that was merged.
    pub(crate) sources: HashMap<String, String>,
    /// One entry per database row, in the store's order.
    pub(crate) report: Vec<TrackedReport>,
}

impl MergedPositions {
    /// config.yaml alone: what every cycle read before database rows existed.
    pub(crate) fn config_only(config_positions: &[PositionConfig]) -> MergedPositions {
        MergedPositions { positions: config_positions.to_vec(), ..Default::default() }
    }
}

/// Merge the database rows into config.yaml's list. Config entries come first, in config
/// order and untouched, so with no row the list is exactly config.yaml's. `not_owned` maps
/// a position key to the address that owns the NFT now (tracked_owners). Pure.
pub(crate) fn merge_position_configs(
    config_positions: &[PositionConfig],
    rows: &[TrackedRow],
    not_owned: &HashMap<String, String>,
) -> MergedPositions {
    let mut out = MergedPositions::config_only(config_positions);
    let mut seen: HashMap<String, String> = config_positions.iter().map(|c| (position_key(c), c.id.clone())).collect();
    for row in rows {
        let fate = if let Some(config_id) = seen.get(&row.position_key) {
            TrackedFate::ShadowedByConfig { config_id: config_id.clone() }
        } else if let Err(reason) = validate_tracked_row(row) {
            TrackedFate::Quarantined { reason }
        } else if let Some(owner) = not_owned.get(&row.position_key) {
            TrackedFate::NotOwned { owner: owner.clone() }
        } else {
            let cfg = tracked_position_config(row);
            seen.insert(row.position_key.clone(), cfg.id.clone());
            out.sources.insert(row.position_key.clone(), POSITION_SOURCE_TRACKED.to_string());
            out.positions.push(cfg);
            TrackedFate::Merged
        };
        out.report.push(TrackedReport { row: row.clone(), fate });
    }
    out
}

/// The owner word of an `ownerOf` answer, as a lowercase 0x address. None when the
/// answer is not one address word. Pure.
pub(crate) fn owner_from_word(hex: &str) -> Option<String> {
    let clean = hex.trim_start_matches("0x");
    if clean.len() != 64 || !clean.bytes().all(|b| b.is_ascii_hexdigit()) || !clean[..24].bytes().all(|b| b == b'0') {
        return None;
    }
    Some(format!("0x{}", clean[24..].to_lowercase()))
}

/// Who owns each database row's NFT now, read at `latest`. The v4 adapter checks
/// ownership only at zero liquidity, which is right for a config entry (the operator's own
/// statement) and not for a row the worker wrote: an NFT sent to another address must
/// stop being counted. Returns the rows another address owns, keyed by position key.
/// - the wallet owns it: nothing to say;
/// - another address: reported, so the merge leaves the row out;
/// - the call reverts: the row is merged and the adapter decides (a burned NFT goes
///   through the #144 proof; anything else fails the cycle there);
/// - no answer: an error, which fails the cycle. Whether to count it is not known.
pub(crate) async fn tracked_owners(http: &Client, rpc_url: &str, wallet: &str, rows: &[TrackedRow]) -> Result<HashMap<String, String>> {
    let mut not_owned = tracked_bet_owners(http, rpc_url, wallet, rows).await?;
    for row in rows.iter().filter(|r| r.protocol == TRACKED_PROTOCOL_V4) {
        let payload = json!({
            "jsonrpc": "2.0", "method": "eth_call", "id": 145,
            "params": [{ "to": UNISWAP_V4_POSM_BASE, "data": format!("{}{:064x}", ERC721_OWNER_OF, row.nft_id) }, "latest"]
        });
        let outcome = eth_call_outcome(http, rpc_url, &payload).await.map_err(|e| {
            anyhow::anyhow!(
                "tracked position {} (NFT #{}): its owner could not be read ({e}); the cycle stops rather than count or drop it unread",
                row.position_key,
                row.nft_id
            )
        })?;
        if let CallOutcome::Returned(hex) = outcome {
            match owner_from_word(&hex) {
                Some(owner) if owner == lc(wallet) => {}
                Some(owner) => {
                    not_owned.insert(row.position_key.clone(), owner);
                }
                None => {
                    return Err(anyhow::anyhow!(
                        "tracked position {} (NFT #{}): ownerOf() answered something that is not an address; the cycle stops rather than count or drop it unread",
                        row.position_key,
                        row.nft_id
                    ))
                }
            }
        }
    }
    Ok(not_owned)
}

/// Who holds each bet row's NFT now (#181), read at `latest`. A bet is counted while the
/// wallet holds the NFT, or the pool's gauge holds it and `stakedContains(wallet, id)` is
/// true. Anything else is reported, so the merge leaves the row out (the Aero CL adapter
/// would fail the cycle on it). A revert is left to the adapter's closed proof (#144). No
/// answer fails the cycle, as for a v4 row.
async fn tracked_bet_owners(http: &Client, rpc_url: &str, wallet: &str, rows: &[TrackedRow]) -> Result<HashMap<String, String>> {
    let mut not_owned = HashMap::new();
    for row in rows.iter().filter(|r| r.protocol == AERO_CL_PROTOCOL) {
        let call = |to: String, data: String, what: &'static str| {
            let payload = json!({ "jsonrpc": "2.0", "method": "eth_call", "id": 181, "params": [{ "to": to, "data": data }, "latest"] });
            let (key, nft) = (row.position_key.clone(), row.nft_id);
            async move {
                eth_call_outcome(http, rpc_url, &payload).await.map_err(|e| {
                    anyhow::anyhow!("tracked bet {key} (NFT #{nft}): its {what} could not be read ({e}); the cycle stops rather than count or drop it unread")
                })
            }
        };
        let unreadable = |what: &str| {
            anyhow::anyhow!("tracked bet {} (NFT #{}): {what} answered something that is not an address; the cycle stops rather than count or drop it unread", row.position_key, row.nft_id)
        };
        let owner = match call(AERO_CL_POSITION_MANAGER_BASE.to_string(), format!("{}{:064x}", ERC721_OWNER_OF, row.nft_id), "owner").await? {
            CallOutcome::Returned(hex) => owner_from_word(&hex).ok_or_else(|| unreadable("ownerOf()"))?,
            CallOutcome::Reverted(_) => continue,
        };
        if owner == lc(wallet) {
            continue;
        }
        let gauge = match call(row.pool_id.clone(), CL_POOL_GAUGE.to_string(), "pool's gauge").await? {
            CallOutcome::Returned(hex) => owner_from_word(&hex).ok_or_else(|| unreadable("the pool's gauge()"))?,
            CallOutcome::Reverted(_) => String::new(),
        };
        if !gauge.is_empty() && owner == gauge {
            let data = format!("{CL_GAUGE_STAKED_CONTAINS}{:0>64}{:064x}", lc(wallet).trim_start_matches("0x"), row.nft_id);
            if let CallOutcome::Returned(hex) = call(gauge.clone(), data, "stake").await? {
                if hex.len() == 64 && hex.trim_start_matches('0') == "1" {
                    continue;
                }
            }
        }
        not_owned.insert(row.position_key.clone(), owner);
    }
    Ok(not_owned)
}

/// What to remember of each database-tracked ladder after this cycle's reads: its
/// verified mint's liquidity (once known) and the most liquidity a cycle read it holding.
/// Only rows that changed. A rung not read this cycle (burned, sent away) keeps what was
/// remembered. Pure.
pub(crate) fn liquidity_updates(raw: &RawOnChainData) -> Vec<(String, Option<u128>, u128)> {
    let mut out = Vec::new();
    for r in raw.tracked_report.iter().filter(|r| r.fate == TrackedFate::Merged && r.row.role.as_deref() == Some(LADDER_ROLE)) {
        let key = &r.row.position_key;
        let Some(now) = raw.positions.iter().find(|p| &p.position_id == key).map(|p| p.liquidity) else { continue };
        let deposit = match raw.ladder_baselines.get(key) {
            Some(LadderBaselineRead::Verified(d)) => Some(d.liquidity),
            _ => None,
        };
        let max_seen = [r.row.max_liquidity_seen, Some(now), deposit].into_iter().flatten().max().unwrap_or(now);
        let new_deposit = deposit.filter(|_| r.row.deposit_liquidity.is_none());
        if new_deposit.is_some() || Some(max_seen) != r.row.max_liquidity_seen {
            out.push((key.clone(), new_deposit, max_seen));
        }
    }
    out
}

/// Write `liquidity_updates` after the cycle's reads. It never fails the cycle: a write
/// that fails is logged and tried again next cycle.
pub(crate) async fn record_tracked_liquidity(store: &PositionStore, wallet: Option<&str>, raw: &RawOnChainData) {
    let Some(wallet) = wallet else { return };
    for (key, deposit, max_seen) in liquidity_updates(raw) {
        if let Err(e) = store.record_liquidity(wallet, &key, deposit, max_seen).await {
            tracing::warn!("Tracked positions: could not record the liquidity of {key}: {e}");
        }
    }
}

/// The words a failed read of the tracked-positions table fails the cycle with. The
/// database's own error is logged, not quoted: the cycle error is a public surface.
pub(crate) const TRACKED_TABLE_UNREADABLE: &str =
    "The tracked positions table could not be read: the cycle stops rather than publish a total without the positions tracked from the chain";

/// This cycle's position list: the database rows read, their owners checked, merged after
/// config.yaml's entries. An unreadable table or an unread owner fails the cycle.
pub(crate) async fn load_cycle_positions(
    store: &PositionStore,
    http: &Client,
    rpc_url: &str,
    config: &AppConfig,
) -> Result<MergedPositions> {
    let Some(wallet) = config.tracked_wallets.first() else {
        // fetch_raw_data_merged states the missing wallet itself.
        return Ok(MergedPositions::config_only(&config.positions));
    };
    let rows = store.tracked(wallet).await.map_err(|e| {
        tracing::error!("Tracked positions unreadable: {e}");
        anyhow::anyhow!(TRACKED_TABLE_UNREADABLE)
    })?;
    if rows.is_empty() {
        return Ok(MergedPositions::config_only(&config.positions));
    }
    // Owners are read only for the rows that would be merged: a row config.yaml shadows
    // or one that is quarantined is not read at all.
    let first = merge_position_configs(&config.positions, &rows, &HashMap::new());
    let to_read: Vec<TrackedRow> = first.report.into_iter().filter(|r| r.fate == TrackedFate::Merged).map(|r| r.row).collect();
    let not_owned = tracked_owners(http, rpc_url, wallet, &to_read).await?;
    Ok(merge_position_configs(&config.positions, &rows, &not_owned))
}

/// The positions the claim payload may build an action for: config.yaml's entries and the
/// database rows the latest snapshot holds as live positions (a row tracked a moment ago
/// gets its action once a cycle has read it). Pure.
pub(crate) fn claim_position_configs(config_positions: &[PositionConfig], rows: &[TrackedRow], snapshot: &SnapshotData) -> Vec<PositionConfig> {
    let mut out = config_positions.to_vec();
    let known: Vec<String> = config_positions.iter().map(position_key).collect();
    for row in rows {
        let in_snapshot = snapshot.positions.iter().any(|p| p.position_id == row.position_key);
        if in_snapshot && !known.contains(&row.position_key) && validate_tracked_row(row).is_ok() {
            out.push(tracked_position_config(row));
        }
    }
    out
}

/// A row as the config.yaml entry that would track the same position (#167), for a
/// rollback to a release that knows no database rows: without the entry that release
/// would publish a total without this position. Pure.
pub(crate) fn tracked_row_config_entry(row: &TrackedRow) -> String {
    if row.protocol == AERO_CL_PROTOCOL {
        return [
            format!("- id: \"{}\"", tracked_config_id(row)),
            format!("  protocol: \"{AERO_CL_PROTOCOL}\""),
            format!("  network: \"{}\"", row.network),
            format!("  nft_id: {}", row.nft_id),
            format!("  npm_address: \"{AERO_CL_POSITION_MANAGER_BASE}\""),
            format!("  pool_address: \"{}\"", row.pool_id),
            format!("  role: \"{BET_ROLE}\""),
        ]
        .join("\n");
    }
    let mut lines = vec![
        format!("- id: \"{}\"", tracked_config_id(row)),
        format!("  protocol: \"{}\"", row.protocol),
        format!("  network: \"{}\"", row.network),
        format!("  nft_id: {}", row.nft_id),
        format!("  pool_id: \"{}\"", row.pool_id),
        format!("  currency0: \"{}\"", row.currency0),
        format!("  currency1: \"{}\"", row.currency1),
        format!("  tick_lower: {}", row.tick_lower),
        format!("  tick_upper: {}", row.tick_upper),
    ];
    if row.role.as_deref() == Some(LADDER_ROLE) {
        lines.push(format!("  role: \"{LADDER_ROLE}\""));
        if let Some(r) = &row.rung {
            lines.push(format!("  rung: \"{r}\""));
        }
        if let Some(tx) = &row.deposit_tx {
            lines.push(format!("  deposit_tx: \"{tx}\""));
        }
    } else {
        lines.push("  rebuild_weight_bps: 0".to_string());
    }
    lines.join("\n")
}
