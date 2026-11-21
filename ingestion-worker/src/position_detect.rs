//! Finding the wallet's position NFTs on-chain (v0.9.0; #145 for Uniswap v4).
//!
//! The operator deposits in the Uniswap app and comes back. No hash, no NFT id and no
//! config.yaml edit: at the start of every ingestion cycle, before the position reads,
//! the worker looks for Uniswap v4 position NFTs that reached the tracked wallet, reads
//! each one on-chain, and decides:
//!
//! - TRACK, by itself, only a position the wallet minted in a whitelisted pool whose pool
//!   and ticks are exactly a rung of the current ladder suggestion. It is tracked as
//!   `role: ladder` with that rung (unset when the suggestion names none) and its mint
//!   transaction as the #167 baseline, checked by `parse_ladder_deposit` unchanged.
//! - OFFER anything else the wallet holds in a whitelisted pool with liquidity: it is
//!   stated as found and not counted, with its value, and one tap tracks it (as a ladder
//!   rung or as LP) or dismisses it. It is never tracked silently.
//! - IGNORE a pool with a token outside the whitelist (dust), a hooked pool, an NFT the
//!   wallet does not own and an empty one. They are counted, never listed as offers.
//!
//! How it scans (ONCHAIN_INTEGRATION.md §15). The position manager emits the ERC-721
//! `Transfer(from, to, id)` with the wallet as an indexed topic, so two reads find the
//! NFTs, and both are hints only (every fact acted on is read from the chain again):
//!   1. `eth_getLogs`, exact and with no index delay. One request covers the whole
//!      history where the RPC allows it. Where it limits the block range, the range is
//!      read in chunks of `POSITION_LOG_CHUNK_BLOCKS`, at most
//!      `POSITION_LOG_MAX_CALLS_PER_CYCLE` a cycle: forward from where the last cycle
//!      stopped, then backward toward block 0. The cursor is stored, so a backfill
//!      resumes, and after a database loss it starts again from the head.
//!   2. `alchemy_getAssetTransfers` (ERC-721), the RPC's own transfer index, with no
//!      block-range limit. It is read only while the logs do not cover the history.
//! One exact cross-check needs neither: `balanceOf(wallet)` on the position manager
//! against the NFTs known. When the wallet holds more than are known, the status says so.
//!
//! What this never does: fail the cycle (when it cannot check, the snapshot says why and
//! how far it had read), sign or send anything, print the RPC URL or its key, or track a
//! position the wallet did not mint without a tap.
use crate::*;
use serde_json::Value;
use std::collections::{BTreeMap, HashSet};

/// `balanceOf(address)` on the position manager (ERC-721).
pub(crate) const ERC721_BALANCE_OF: &str = "0x70a08231";
/// Blocks per `eth_getLogs` request when the RPC limits the range. The public Base node
/// refuses more than 50,000 (read 2026-11-21).
pub(crate) const POSITION_LOG_CHUNK_BLOCKS: i64 = 45_000;
/// `eth_getLogs` requests per cycle. With an RPC that takes the whole range, one.
pub(crate) const POSITION_LOG_MAX_CALLS_PER_CYCLE: usize = 12;
/// The scan reads up to this many blocks behind the node's latest, so a load-balanced
/// endpoint whose nodes are a block apart still holds the block every read names.
pub(crate) const POSITION_HEAD_LAG_BLOCKS: i64 = 5;
/// Transfers per `alchemy_getAssetTransfers` page, and pages per cycle.
pub(crate) const POSITION_HISTORY_PAGE_SIZE: u32 = 100;
pub(crate) const POSITION_HISTORY_MAX_PAGES: usize = 10;
/// NFTs read per cycle (three cheap calls each), and mint receipts read per cycle. The
/// rest wait for the next cycle and the status says so.
pub(crate) const POSITION_CLASSIFY_MAX_PER_CYCLE: usize = 200;
pub(crate) const POSITION_RECEIPTS_MAX_PER_CYCLE: usize = 25;
/// Owners re-read per cycle when the wallet holds fewer NFTs than are known as owned.
pub(crate) const POSITION_OWNER_RESYNC_MAX_PER_CYCLE: usize = 60;
/// Untracked NFTs the wallet holds re-read per cycle (their liquidity, and their owner
/// when funded). Dust in a whitelisted pool is cheap to send, so the re-reads are capped
/// and rotate: the rest are read in later cycles, and the status says so.
pub(crate) const POSITION_REFRESH_MAX_PER_CYCLE: usize = 40;
/// The check stops starting new reads after this long, well inside the hard timeout: what
/// it read is kept and the rest is read next cycle, so the check never fails the cycle and
/// never runs into the hard timeout because of dust.
pub(crate) const POSITION_SCAN_SOFT_SECONDS: u64 = 40;
/// The whole check is abandoned after this long; what was stored before the cut stays.
pub(crate) const POSITION_SCAN_TIMEOUT_SECONDS: u64 = 60;
/// Where the next cycle's re-reads start, so every untracked NFT is re-read in turn.
static REFRESH_CURSOR: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

/// This cycle's share of the re-reads: at most `max`, starting at `cursor` and wrapping,
/// and how many wait for a later cycle. Pure.
pub(crate) fn refresh_batch(ids: &[u64], cursor: usize, max: usize) -> (Vec<u64>, usize) {
    if ids.len() <= max {
        return (ids.to_vec(), 0);
    }
    let start = cursor % ids.len();
    let batch: Vec<u64> = ids.iter().cycle().skip(start).take(max).copied().collect();
    (batch, ids.len() - max)
}
pub(crate) const POSITION_SCAN_REQUEST_TIMEOUT_SECONDS: u64 = 10;
pub(crate) const POSITION_SCAN_REQUEST_ATTEMPTS: u32 = 2;
/// How long a refusal learned from an RPC (a range limit, an archive limit) is trusted
/// before the wide request is tried again.
pub(crate) const POSITION_LOG_LIMITS_TTL_SECONDS: u64 = 3600;

pub(crate) const POSITION_DETECTION_STATEMENT: &str = "NockNock reads the wallet's Uniswap v4 position NFTs from the chain. It tracks one by itself only when the wallet minted it and its pool and ticks are exactly a rung of the ladder suggestion; anything else it finds waits for one tap and is not counted until then. It never signs or sends.";

pub(crate) const ZERO_ADDRESS: &str = "0x0000000000000000000000000000000000000000";

fn lc(s: &str) -> String {
    s.trim().to_lowercase()
}

fn addr_topic(addr: &str) -> String {
    format!("0x000000000000000000000000{}", lc(addr).trim_start_matches("0x"))
}

/// "0xee11…7a45". Pure.
pub(crate) fn short_addr(a: &str) -> String {
    // An address is ASCII; anything else is printed whole rather than cut mid-character.
    if a.is_ascii() && a.len() > 12 { format!("{}…{}", &a[..6], &a[a.len() - 4..]) } else { a.to_string() }
}

// ------------------------------------------------------------------
// The token whitelist (operator decision 2026-11-17)
// ------------------------------------------------------------------

/// What a pool key says about tracking a position in it. Pure.
/// - `eligible`: both tokens are in the token registry (NOCK, USDC, WETH, native ETH: the
///   tokens of the positions NockNock reads) and the pool has no hooks.
/// - `unsupported`: both are on the whitelist and one is AERO, the fourth pillar, which
///   NockNock holds and prices but cannot yet read as a position leg (#170).
/// - `hooks`: whitelisted tokens in a hooked pool. A hook runs its own code on every
///   swap and deposit, so the position's reads are not verified.
/// - `not_whitelisted`: anything else. People send dust; it is never offered.
pub(crate) fn pool_key_verdict(currency0: &str, currency1: &str, hooks: &str) -> &'static str {
    let on_registry = |a: &str| token_info(a).is_ok();
    let is_aero = |a: &str| lc(a) == lc(AERO_BASE);
    let whitelisted = |a: &str| on_registry(a) || is_aero(a);
    if !(whitelisted(currency0) && whitelisted(currency1)) {
        return "not_whitelisted";
    }
    if hooks.trim_start_matches("0x").chars().any(|c| c != '0') {
        return "hooks";
    }
    if on_registry(currency0) && on_registry(currency1) {
        "eligible"
    } else {
        "unsupported"
    }
}

// ------------------------------------------------------------------
// getPoolAndPositionInfo
// ------------------------------------------------------------------

/// What the position manager stores for one NFT: its pool key and its packed info word
/// `poolId[25 bytes] | tickUpper (int24) | tickLower (int24) | hasSubscriber (uint8)`.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct V4NftInfo {
    /// 0x-prefixed, lowercase.
    pub(crate) currency0: String,
    pub(crate) currency1: String,
    pub(crate) fee_raw: u32,
    pub(crate) tick_spacing: i32,
    pub(crate) hooks: String,
    /// The first 25 bytes of the pool id: 50 hex characters, no 0x.
    pub(crate) pool_id_prefix: String,
    pub(crate) tick_lower: i32,
    pub(crate) tick_upper: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NftInfoRead {
    /// Six zero words: no such NFT (burned, or never minted).
    Gone,
    Info(V4NftInfo),
}

fn int24(hex6: &str) -> Option<i32> {
    let v = i32::from_str_radix(hex6, 16).ok()?;
    Some(if v & 0x80_0000 != 0 { v - 0x100_0000 } else { v })
}

/// Decode a `getPoolAndPositionInfo(uint256)` answer (hex, with or without 0x). Anything
/// that is not six well-formed words is an error, never "gone". Pure.
pub(crate) fn decode_pool_and_position_info(hex: &str) -> std::result::Result<NftInfoRead, String> {
    let clean = hex.trim_start_matches("0x").to_lowercase();
    if clean.len() != V4_POOL_AND_POSITION_INFO_HEX_LEN || !clean.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(format!("getPoolAndPositionInfo answered {} bytes, not six words", clean.len() / 2));
    }
    if clean.bytes().all(|b| b == b'0') {
        return Ok(NftInfoRead::Gone);
    }
    let word = |i: usize| &clean[i * 64..(i + 1) * 64];
    let address = |w: &str| -> std::result::Result<String, String> {
        if !w[..24].bytes().all(|b| b == b'0') {
            return Err("a pool key word is not an address".to_string());
        }
        Ok(format!("0x{}", &w[24..]))
    };
    let fee_raw = parse_hex_word_u128(word(2)).ok().and_then(|f| u32::try_from(f).ok()).ok_or("the pool key's fee is not a uint24")?;
    let tick_spacing = parse_hex_word_i32(word(3)).map_err(|_| "the pool key's tick spacing is not an int24".to_string())?;
    let packed = word(5);
    let (tick_upper, tick_lower) =
        (int24(&packed[50..56]).ok_or("the packed tickUpper did not parse")?, int24(&packed[56..62]).ok_or("the packed tickLower did not parse")?);
    Ok(NftInfoRead::Info(V4NftInfo {
        currency0: address(word(0))?,
        currency1: address(word(1))?,
        fee_raw,
        tick_spacing,
        hooks: address(word(4))?,
        pool_id_prefix: packed[..50].to_string(),
        tick_lower,
        tick_upper,
    }))
}

// ------------------------------------------------------------------
// Transfers into the wallet (hints, never trusted)
// ------------------------------------------------------------------

/// One position NFT arriving in the wallet, as a log or the transfer index reports it.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TransferSeen {
    pub(crate) nft_id: u64,
    pub(crate) block: i64,
    /// Lowercase, 0x-prefixed.
    pub(crate) tx_hash: String,
    /// Lowercase; the zero address for a mint.
    pub(crate) from: String,
}

fn token_id_from_word(w: &str) -> Option<u64> {
    let clean = w.trim_start_matches("0x");
    if clean.len() != 64 || !clean[..48].bytes().all(|b| b == b'0') {
        return None;
    }
    u64::from_str_radix(&clean[48..], 16).ok()
}

fn hex_block(v: Option<&Value>) -> Option<i64> {
    v.and_then(|b| b.as_str()).and_then(|b| i64::from_str_radix(b.trim_start_matches("0x"), 16).ok())
}

/// `eth_getLogs` for ERC-721 transfers of the position manager's NFTs to `wallet`. Pure.
pub(crate) fn transfer_logs_request(wallet: &str, from_block: i64, to_block: i64) -> Value {
    json!({
        "jsonrpc": "2.0", "id": 146, "method": "eth_getLogs",
        "params": [{
            "address": UNISWAP_V4_POSM_BASE,
            "topics": [TRANSFER_EVENT_TOPIC, Value::Null, addr_topic(wallet)],
            "fromBlock": format!("0x{from_block:x}"),
            "toBlock": format!("0x{to_block:x}"),
        }]
    })
}

/// Parse an `eth_getLogs` answer. Every log must be a position-manager Transfer to the
/// wallet with a token id, a block and a transaction hash: anything else is an error,
/// never a shorter list ("nothing found" must mean the list was read). Pure.
pub(crate) fn parse_transfer_logs(resp: &Value, wallet: &str) -> std::result::Result<Vec<TransferSeen>, String> {
    let logs = resp.get("result").and_then(|r| r.as_array()).ok_or("the answer has no result array")?;
    let to = addr_topic(wallet);
    let mut out = Vec::with_capacity(logs.len());
    for (i, l) in logs.iter().enumerate() {
        if l.get("address").and_then(|a| a.as_str()).map(lc).as_deref() != Some(UNISWAP_V4_POSM_BASE) {
            return Err(format!("log {i} is not from the position manager"));
        }
        if l.get("removed").and_then(|r| r.as_bool()) == Some(true) {
            return Err(format!("log {i} was removed by a chain reorganisation"));
        }
        let topics: Vec<String> = l.get("topics").and_then(|t| t.as_array()).map(|a| a.iter().filter_map(|x| x.as_str()).map(lc).collect()).unwrap_or_default();
        if topics.len() != 4 || topics[0] != TRANSFER_EVENT_TOPIC || topics[2] != to {
            return Err(format!("log {i} is not an ERC-721 Transfer to the wallet"));
        }
        let from = topics[1].get(26..).filter(|_| topics[1].len() == 66).map(|a| format!("0x{a}")).ok_or_else(|| format!("log {i} has no sender"))?;
        let nft_id = token_id_from_word(&topics[3]).ok_or_else(|| format!("log {i} has no usable token id"))?;
        let block = hex_block(l.get("blockNumber")).ok_or_else(|| format!("log {i} has no block number"))?;
        let tx_hash = l.get("transactionHash").and_then(|h| h.as_str()).filter(|h| is_valid_tx_hash(h)).map(lc).ok_or_else(|| format!("log {i} has no transaction hash"))?;
        out.push(TransferSeen { nft_id, block, tx_hash, from });
    }
    Ok(out)
}

/// `alchemy_getAssetTransfers` for the same transfers from the RPC's index, newest
/// first. Pure.
pub(crate) fn nft_history_request(wallet: &str, page_key: Option<&str>) -> Value {
    let mut params = json!({
        "fromBlock": "0x0",
        "toBlock": "latest",
        "toAddress": wallet,
        "contractAddresses": [UNISWAP_V4_POSM_BASE],
        "category": ["erc721"],
        "withMetadata": false,
        "order": "desc",
        "maxCount": format!("0x{:x}", POSITION_HISTORY_PAGE_SIZE),
    });
    if let Some(key) = page_key {
        params["pageKey"] = json!(key);
    }
    json!({ "jsonrpc": "2.0", "id": 147, "method": "alchemy_getAssetTransfers", "params": [params] })
}

/// Parse one page of it: the transfers and the key of the next page. A page that is not
/// the expected shape is an error, never an empty list. Pure.
pub(crate) fn parse_nft_history(resp: &Value) -> std::result::Result<(Vec<TransferSeen>, Option<String>), String> {
    let result = resp.get("result").ok_or("the answer has no result")?;
    let transfers = result.get("transfers").and_then(|t| t.as_array()).ok_or("the result has no transfers array")?;
    let mut out = Vec::with_capacity(transfers.len());
    for (i, t) in transfers.iter().enumerate() {
        let contract = t.pointer("/rawContract/address").and_then(|a| a.as_str()).map(lc);
        if contract.as_deref() != Some(UNISWAP_V4_POSM_BASE) {
            return Err(format!("transfer {i} is not of the position manager's NFT"));
        }
        let id_word = t.get("tokenId").and_then(|v| v.as_str()).or_else(|| t.get("erc721TokenId").and_then(|v| v.as_str()));
        let nft_id = id_word
            .map(|w| format!("{:0>64}", w.trim_start_matches("0x")))
            .and_then(|w| token_id_from_word(&w))
            .ok_or_else(|| format!("transfer {i} has no usable token id"))?;
        let tx_hash = t.get("hash").and_then(|h| h.as_str()).filter(|h| is_valid_tx_hash(h)).map(lc).ok_or_else(|| format!("transfer {i} has no transaction hash"))?;
        let block = hex_block(t.get("blockNum")).ok_or_else(|| format!("transfer {i} has no block number"))?;
        let from = t.get("from").and_then(|f| f.as_str()).map(lc).filter(|f| f.len() == 42).ok_or_else(|| format!("transfer {i} has no sender"))?;
        out.push(TransferSeen { nft_id, block, tx_hash, from });
    }
    let page_key = result.get("pageKey").and_then(|k| k.as_str()).filter(|k| !k.is_empty()).map(|k| k.to_string());
    Ok((out, page_key))
}

/// Keep, per NFT, the newest transfer seen. Pure.
pub(crate) fn absorb_transfers(seen: &mut HashMap<u64, TransferSeen>, transfers: Vec<TransferSeen>) {
    for t in transfers {
        match seen.get(&t.nft_id) {
            Some(old) if old.block >= t.block => {}
            _ => {
                seen.insert(t.nft_id, t);
            }
        }
    }
}

// ------------------------------------------------------------------
// The log scan's windows (pure)
// ------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct LogWindow {
    pub(crate) from: i64,
    pub(crate) to: i64,
}

impl LogWindow {
    pub(crate) fn blocks(&self) -> i64 {
        self.to - self.from + 1
    }
}

/// The next block range to read, or None when `floor..=head` is covered. `covered` is the
/// one range already read without a gap. Forward first (new transfers matter most), then
/// backward toward `floor`. With `wide` the whole remainder is one request; otherwise at
/// most `POSITION_LOG_CHUNK_BLOCKS`, and a first read starts with the newest chunk. Pure.
pub(crate) fn next_log_window(covered: Option<(i64, i64)>, head: i64, wide: bool, floor: i64) -> Option<LogWindow> {
    let chunk = POSITION_LOG_CHUNK_BLOCKS;
    match covered {
        None => {
            if head < floor {
                return None;
            }
            Some(LogWindow { from: if wide { floor } else { floor.max(head - chunk + 1) }, to: head })
        }
        Some((lo, hi)) => {
            if hi < head {
                Some(LogWindow { from: hi + 1, to: if wide { head } else { head.min(hi + chunk) } })
            } else if lo > floor {
                Some(LogWindow { from: if wide { floor } else { floor.max(lo - chunk) }, to: lo - 1 })
            } else {
                None
            }
        }
    }
}

/// `covered` after `w` was read. `w` always touches it (next_log_window). Pure.
pub(crate) fn cover(covered: Option<(i64, i64)>, w: LogWindow) -> (i64, i64) {
    match covered {
        None => (w.from, w.to),
        Some((lo, hi)) => (lo.min(w.from), hi.max(w.to)),
    }
}

/// What one RPC taught the scan about its `eth_getLogs` limits, kept per process and per
/// URL for `POSITION_LOG_LIMITS_TTL_SECONDS`.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct LogLimits {
    /// A request wider than one chunk was refused: read in chunks.
    pub(crate) chunked: bool,
    /// A one-chunk request reaching the head was refused: logs cannot be read here.
    pub(crate) unusable: Option<String>,
    /// A backfill chunk was refused: blocks below this are not served, with the reason.
    pub(crate) refused_below: Option<(i64, String)>,
}

static LOG_LIMITS: std::sync::Mutex<Option<HashMap<String, (std::time::Instant, LogLimits)>>> = std::sync::Mutex::new(None);

fn known_log_limits(url: &str) -> LogLimits {
    let guard = LOG_LIMITS.lock().unwrap_or_else(|p| p.into_inner());
    match guard.as_ref().and_then(|m| m.get(url)) {
        Some((at, limits)) if at.elapsed().as_secs() < POSITION_LOG_LIMITS_TTL_SECONDS => limits.clone(),
        _ => LogLimits::default(),
    }
}

fn remember_log_limits(url: &str, limits: &LogLimits) {
    let mut guard = LOG_LIMITS.lock().unwrap_or_else(|p| p.into_inner());
    let map = guard.get_or_insert_with(HashMap::new);
    // The clock of the first refusal: every limit is tried again an hour after that.
    let at = match map.get(url) {
        Some((at, _)) if at.elapsed().as_secs() < POSITION_LOG_LIMITS_TTL_SECONDS => *at,
        _ => std::time::Instant::now(),
    };
    map.insert(url.to_string(), (at, limits.clone()));
}

/// Forget what was learned about one RPC's limits (tests).
#[cfg(test)]
pub(crate) fn forget_log_limits(url: &str) {
    if let Some(map) = LOG_LIMITS.lock().unwrap_or_else(|p| p.into_inner()).as_mut() {
        map.remove(url);
    }
}

// ------------------------------------------------------------------
// The RPC, with a reason for every failure
// ------------------------------------------------------------------

/// What the RPC said to one request. A refusal (a JSON-RPC error, whatever the HTTP
/// status) is told apart from no answer at all: a refused block range changes how the
/// scan reads, a dropped connection does not.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RpcAnswer {
    Result(Value),
    Refused(String),
    NoAnswer(String),
}

/// One JSON-RPC request. The reasons are written by hand from the kind of failure: a
/// transport error's own text is never used, because it can contain the request URL and
/// the URL contains the API key.
pub(crate) async fn rpc_answer(client: &Client, url: &str, payload: &Value) -> RpcAnswer {
    let mut last = "no response".to_string();
    for attempt in 0..POSITION_SCAN_REQUEST_ATTEMPTS {
        let sent = client.post(url).timeout(std::time::Duration::from_secs(POSITION_SCAN_REQUEST_TIMEOUT_SECONDS)).json(payload).send().await;
        match sent {
            Ok(resp) => {
                let status = resp.status();
                match resp.json::<Value>().await {
                    Ok(v) if status.is_success() && v.get("result").is_some() => return RpcAnswer::Result(v),
                    Ok(_) if status.as_u16() == 429 => last = "the RPC is rate limiting".to_string(),
                    Ok(v) => match rpc_error_reason(&v) {
                        Some(why) => return RpcAnswer::Refused(why),
                        None => last = format!("the RPC answered HTTP {} with neither a result nor an error", status.as_u16()),
                    },
                    Err(_) => last = format!("the RPC answered HTTP {} and no JSON", status.as_u16()),
                }
            }
            Err(e) if e.is_timeout() => last = "the request timed out".to_string(),
            Err(e) if e.is_connect() => last = "the connection to the RPC failed".to_string(),
            Err(_) => last = "the request failed in transit".to_string(),
        }
        if attempt + 1 < POSITION_SCAN_REQUEST_ATTEMPTS {
            tokio::time::sleep(tokio::time::Duration::from_millis(150 << attempt)).await;
        }
    }
    RpcAnswer::NoAnswer(last)
}

/// One `eth_call` at a block, keeping a revert apart from no answer. The error is a
/// hand-written reason.
async fn call_at(client: &Client, url: &str, to: &str, data: &str, block_tag: &str) -> std::result::Result<CallOutcome, String> {
    let payload = json!({ "jsonrpc": "2.0", "method": "eth_call", "id": 148, "params": [{ "to": to, "data": data }, block_tag] });
    eth_call_outcome(client, url, &payload).await.map_err(|_| "the RPC did not answer".to_string())
}

// ------------------------------------------------------------------
// The mint receipt
// ------------------------------------------------------------------

/// What a transaction's receipt says about one NFT reaching the wallet.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct MintFacts {
    pub(crate) block: i64,
    /// The transaction's sender, lowercase.
    pub(crate) sender: String,
    /// The receipt holds the position manager's Transfer of this id from the zero address
    /// to the wallet: the NFT was minted to the wallet in this transaction.
    pub(crate) minted_to_wallet: bool,
    /// The pool of the PoolManager ModifyLiquidity whose salt is this id, through the
    /// position manager. None when the receipt holds none, or more than one pool.
    pub(crate) pool_id: Option<String>,
}

/// Read them from a receipt (the `result` of eth_getTransactionReceipt). Pure.
pub(crate) fn mint_facts(receipt: &Value, wallet: &str, nft_id: u64) -> std::result::Result<MintFacts, String> {
    let status = receipt.get("status").and_then(|v| v.as_str()).map(lc).unwrap_or_default();
    if status != "0x1" {
        return Err(format!("the transaction failed (status {status})"));
    }
    let block = hex_block(receipt.get("blockNumber")).ok_or("the receipt has no block number")?;
    let sender = receipt.get("from").and_then(|v| v.as_str()).map(lc).ok_or("the receipt has no sender")?;
    let logs: Vec<Value> = receipt.get("logs").and_then(|l| l.as_array()).cloned().unwrap_or_default();
    let topics = |l: &Value| -> Vec<String> { l.get("topics").and_then(|t| t.as_array()).map(|a| a.iter().filter_map(|x| x.as_str()).map(lc).collect()).unwrap_or_default() };
    let address = |l: &Value| l.get("address").and_then(|a| a.as_str()).map(lc).unwrap_or_default();
    let id_word = format!("{nft_id:064x}");
    let minted_to_wallet = logs.iter().any(|l| {
        let t = topics(l);
        address(l) == UNISWAP_V4_POSM_BASE
            && t.len() == 4
            && t[0] == TRANSFER_EVENT_TOPIC
            && t[1] == addr_topic(ZERO_ADDRESS)
            && t[2] == addr_topic(wallet)
            && t[3].trim_start_matches("0x") == id_word
    });
    let mut pools: Vec<String> = Vec::new();
    for l in &logs {
        let t = topics(l);
        if address(l) != V4_POOL_MANAGER_BASE || t.len() != 3 || t[0] != V4_MODIFY_LIQUIDITY_TOPIC || t[2] != addr_topic(UNISWAP_V4_POSM_BASE) {
            continue;
        }
        let data = l.get("data").and_then(|d| d.as_str()).unwrap_or("").trim_start_matches("0x").to_lowercase();
        if data.len() == 4 * 64 && data[3 * 64..] == id_word && !pools.contains(&t[1]) {
            pools.push(t[1].clone());
        }
    }
    Ok(MintFacts { block, sender, minted_to_wallet, pool_id: if pools.len() == 1 { pools.pop() } else { None } })
}

/// The position a candidate row describes, as far as `parse_ladder_deposit` reads one
/// (protocol, NFT id, pool, ticks, the two tokens). Nothing else of it is a read. None
/// when the row lacks one of those or a token is not in the registry. Pure.
pub(crate) fn candidate_raw_position(c: &CandidateRow, pool_id: &str) -> Option<RawPosition> {
    let (c0, c1) = (c.currency0.as_deref()?, c.currency1.as_deref()?);
    Some(RawPosition {
        position_id: v4_position_key(&c.network, c.nft_id),
        network: c.network.clone(),
        protocol: c.protocol.clone(),
        wallet: c.wallet.clone(),
        nft_id: c.nft_id,
        pool_id: pool_id.to_string(),
        npm_address: None,
        fee_tier_bps: (c.fee_raw.unwrap_or(0) / 100) as u32,
        token0: token_info(c0).ok()?,
        token1: token_info(c1).ok()?,
        liquidity: 0,
        sqrt_price_x96: U256::default(),
        tick: 0,
        tick_lower: c.tick_lower?,
        tick_upper: c.tick_upper?,
        amount0: 0.0,
        amount1: 0.0,
        fee0: 0.0,
        fee1: 0.0,
        in_range: false,
        is_price_source: false,
    })
}

// ------------------------------------------------------------------
// The decision (pure)
// ------------------------------------------------------------------

/// One rung the snapshot currently suggests.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SuggestedRung {
    pub(crate) protocol: String,
    /// Lowercase, 0x-prefixed.
    pub(crate) pool_id: String,
    pub(crate) tick_lower: i32,
    pub(crate) tick_upper: i32,
    /// `near` | `deep` | `cap`, or None when the suggestion names no rung.
    pub(crate) rung: Option<String>,
}

/// Every rung a snapshot's `data` suggests, from either place a release states them:
/// `derived_metrics.ladder_suggestion.rungs[]` (#168), each with its own pool, and
/// `derived_metrics.ladder_shape.rungs[]` (the configured shape), whose pool is the
/// position `ladder_shape.pool.position_id` names. A rung without both ticks and a pool is
/// left out. A rung name outside `LADDER_RUNGS` is read as unset, never passed on. Pure.
pub(crate) fn suggested_rungs(data: &Value) -> Vec<SuggestedRung> {
    let mut out: Vec<SuggestedRung> = Vec::new();
    let ticks = |r: &Value| -> Option<(i32, i32)> {
        let lo = r.get("tick_lower").and_then(|v| v.as_i64()).and_then(|v| i32::try_from(v).ok())?;
        let hi = r.get("tick_upper").and_then(|v| v.as_i64()).and_then(|v| i32::try_from(v).ok())?;
        (lo < hi).then_some((lo, hi))
    };
    let rung_name = |r: &Value| r.get("rung").and_then(|v| v.as_str()).filter(|n| LADDER_RUNGS.contains(n)).map(str::to_string);
    let mut push = |protocol: Option<&str>, pool: Option<&str>, r: &Value| {
        let (Some(protocol), Some(pool), Some((lo, hi))) = (protocol, pool, ticks(r)) else { return };
        let rung = SuggestedRung { protocol: protocol.to_string(), pool_id: lc(pool), tick_lower: lo, tick_upper: hi, rung: rung_name(r) };
        if !out.iter().any(|x| x.protocol == rung.protocol && x.pool_id == rung.pool_id && x.tick_lower == lo && x.tick_upper == hi) {
            out.push(rung);
        }
    };
    for r in data.pointer("/derived_metrics/ladder_suggestion/rungs").and_then(|v| v.as_array()).into_iter().flatten() {
        push(r.pointer("/pool/protocol").and_then(|v| v.as_str()), r.pointer("/pool/pool_id").and_then(|v| v.as_str()), r);
    }
    if let Some(shape) = data.pointer("/derived_metrics/ladder_shape") {
        let source = shape.pointer("/pool/position_id").and_then(|v| v.as_str());
        let position = data.get("positions").and_then(|p| p.as_array()).and_then(|ps| ps.iter().find(|p| p.get("position_id").and_then(|v| v.as_str()) == source && source.is_some()));
        let (protocol, pool) = (
            position.and_then(|p| p.get("protocol")).and_then(|v| v.as_str()),
            position.and_then(|p| p.get("pool_id")).and_then(|v| v.as_str()),
        );
        for r in shape.get("rungs").and_then(|v| v.as_array()).into_iter().flatten() {
            push(protocol, pool, r);
        }
    }
    out
}

/// A range some live ladder already holds (config.yaml's or a database row's).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct TrackedRange {
    pub(crate) position_id: String,
    pub(crate) pool_id: String,
    pub(crate) tick_lower: i32,
    pub(crate) tick_upper: i32,
}

/// The ladder ranges a snapshot's `data` holds: its positions with `role: ladder`, less
/// the ones whose reading says they are empty. Pure.
pub(crate) fn tracked_ranges_in_snapshot(data: &Value) -> Vec<TrackedRange> {
    let empty: Vec<&str> = data
        .get("ladders")
        .and_then(|l| l.as_array())
        .into_iter()
        .flatten()
        .filter(|l| l.get("state").and_then(|s| s.as_str()) == Some("empty"))
        .filter_map(|l| l.get("position_id").and_then(|v| v.as_str()))
        .collect();
    data.get("positions")
        .and_then(|p| p.as_array())
        .into_iter()
        .flatten()
        .filter(|p| p.get("role").and_then(|r| r.as_str()) == Some(LADDER_ROLE))
        .filter_map(|p| {
            let id = p.get("position_id")?.as_str()?;
            if empty.contains(&id) {
                return None;
            }
            Some(TrackedRange {
                position_id: id.to_string(),
                pool_id: lc(p.get("pool_id")?.as_str()?),
                tick_lower: i32::try_from(p.get("lower_tick")?.as_i64()?).ok()?,
                tick_upper: i32::try_from(p.get("upper_tick")?.as_i64()?).ok()?,
            })
        })
        .collect()
}

/// Is the row's pair USDC/NOCK in a range that is not the whole curve? Only such a
/// position can be tracked as a ladder (#167; AERO and ETH ladders are #170). Pure.
pub(crate) fn can_be_ladder(c: &CandidateRow) -> bool {
    let (Some(Ok(t0)), Some(Ok(t1)), Some(lo), Some(hi)) =
        (c.currency0.as_deref().map(token_info), c.currency1.as_deref().map(token_info), c.tick_lower, c.tick_upper)
    else {
        return false;
    };
    let usdc_nock = (is_usdc_token(&t0) && is_nock_token(&t1)) || (is_nock_token(&t0) && is_usdc_token(&t1));
    let (at_min, at_max) = ladder_bounds_at_extremes(lo, hi, c.tick_spacing.filter(|s| *s > 0));
    usdc_nock && lo < hi && !is_full_range(lo, hi) && !(at_min && at_max)
}

/// The one-tap choices for a found position: `ladder:<rung>` for each rung when it can be
/// a ladder, `lp`, and `dismiss`. Without a known pool id it cannot be read as a position,
/// so only `dismiss` is offered. Pure.
pub(crate) fn track_choices(c: &CandidateRow) -> Vec<String> {
    let mut out = Vec::new();
    if c.pool_id.is_some() {
        if can_be_ladder(c) {
            out.extend(LADDER_RUNGS.iter().map(|r| format!("{LADDER_ROLE}:{r}")));
        }
        out.push("lp".to_string());
    }
    out.push("dismiss".to_string());
    out
}

/// What to do with one position found on-chain.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Decision {
    /// Track it as a ladder with this rung (None: the suggestion names none).
    Track { rung: Option<String> },
    /// Offer it: why it is not tracked by itself, and the one-tap choices.
    Offer { why_not_automatic: Vec<String>, choices: Vec<String> },
}

/// Decide for a position that is eligible, owned by the wallet and holds liquidity (the
/// caller checks those from fresh reads). It is tracked by itself only when ALL hold:
/// its pool id is known; it was minted to the wallet, in a transaction the wallet itself
/// sent (anyone can mint a position on the exact ticks to this wallet, and that must not
/// mark a rung deployed); that receipt passes `parse_ladder_deposit`; its pool and ticks
/// are exactly a suggested rung's; and no live ladder already holds that range. Every
/// failed condition is stated. Pure.
pub(crate) fn decide(c: &CandidateRow, wallet: &str, suggested: &[SuggestedRung], tracked: &[TrackedRange]) -> Decision {
    let choices = track_choices(c);
    let mut why: Vec<String> = Vec::new();
    let Some(pool_id) = c.pool_id.as_deref().map(lc) else {
        why.push("its pool could not be identified yet: the transaction that brought it to the wallet has not been read, and its pool is not one NockNock already reads".to_string());
        return Decision::Offer { why_not_automatic: why, choices };
    };
    match (c.mint_tx.as_deref(), c.minted_by.as_deref()) {
        (Some(_), Some(by)) if lc(by) == lc(wallet) => {}
        (Some(_), Some(by)) => why.push(format!("its mint was sent by {}, not by your wallet", short_addr(by))),
        _ => match c.received_from.as_deref() {
            Some(from) if lc(from) != ZERO_ADDRESS => why.push(format!("it reached the wallet by transfer from {}, it was not minted to it", short_addr(from))),
            // Its receipt was read and is no mint: the reason is stated below.
            _ if c.deposit_check.is_some() => {}
            _ => why.push("its mint transaction has not been read yet".to_string()),
        },
    }
    match c.deposit_check.as_deref() {
        Some("ok") => {}
        Some(reason) => why.push(format!("its mint is not a plain deposit into it: {reason}")),
        None if c.mint_tx.is_some() => why.push("its mint receipt has not been checked as a deposit".to_string()),
        None => {}
    }
    let (lo, hi) = (c.tick_lower.unwrap_or(0), c.tick_upper.unwrap_or(0));
    let exact = suggested.iter().find(|s| s.protocol == c.protocol && s.pool_id == pool_id && s.tick_lower == lo && s.tick_upper == hi);
    // A near or deep rung's top follows the price until it is deposited: a mint from a
    // card read before the price crossed a band edge has the rung's own bottom tick and a
    // top one band away. It matches that rung when no live ladder already holds a range
    // with that bottom in this pool (ladder.rs applies the same rule to the card).
    let drifted = || {
        let spacing = c.tick_spacing.filter(|s| *s > 0)?;
        let nock1 = c.currency1.as_deref().and_then(|a| token_info(a).ok()).is_some_and(|t| is_nock_token(&t));
        let ends = |l: i32, h: i32| if nock1 { (h, l) } else { (l, h) };
        let (bottom, top) = ends(lo, hi);
        if tracked.iter().any(|t| t.pool_id == pool_id && ends(t.tick_lower, t.tick_upper).0 == bottom) {
            return None;
        }
        suggested.iter().find(|s| {
            let (s_bottom, s_top) = ends(s.tick_lower, s.tick_upper);
            s.protocol == c.protocol && s.pool_id == pool_id && matches!(s.rung.as_deref(), Some("near") | Some("deep")) && s_bottom == bottom && (s_top - top).abs() <= spacing
        })
    };
    let matched = exact.or_else(drifted);
    if matched.is_none() {
        let in_pool: Vec<&SuggestedRung> = suggested.iter().filter(|s| s.protocol == c.protocol && s.pool_id == pool_id).collect();
        if suggested.is_empty() {
            why.push("no ladder rung is suggested now to match it against".to_string());
        } else if in_pool.is_empty() {
            why.push("it is not in the ladder suggestion's pool".to_string());
        } else {
            let rungs: Vec<String> = in_pool.iter().map(|s| format!("{}..{}", s.tick_lower, s.tick_upper)).collect();
            why.push(format!("its ticks {lo}..{hi} are not a suggested rung's ({})", rungs.join(", ")));
        }
    }
    if let Some(dup) = tracked.iter().find(|t| t.pool_id == pool_id && t.tick_lower == lo && t.tick_upper == hi) {
        why.push(format!("that range is already tracked as {}", dup.position_id));
    }
    if !can_be_ladder(c) && matched.is_some() {
        why.push("it is not a USDC/NOCK concentrated range".to_string());
    }
    match (why.is_empty(), matched) {
        (true, Some(m)) => Decision::Track { rung: m.rung.clone() },
        _ => Decision::Offer { why_not_automatic: why, choices },
    }
}

/// The `tracked_positions` row for a candidate. Its deposit_tx is the mint only when the
/// NFT was minted to the wallet in it; None for one that arrived by transfer. None when
/// the candidate lacks what a position read needs. Pure.
pub(crate) fn tracked_row_for(c: &CandidateRow, role: Option<&str>, rung: Option<&str>, source: &str) -> Option<TrackedRow> {
    Some(TrackedRow {
        wallet: lc(&c.wallet),
        position_key: v4_position_key(&c.network, c.nft_id),
        protocol: c.protocol.clone(),
        network: c.network.clone(),
        nft_id: c.nft_id,
        pool_id: c.pool_id.clone()?,
        currency0: c.currency0.clone()?,
        currency1: c.currency1.clone()?,
        tick_lower: c.tick_lower?,
        tick_upper: c.tick_upper?,
        tick_spacing: c.tick_spacing,
        role: role.map(str::to_string),
        rung: rung.map(str::to_string),
        deposit_tx: c.mint_tx.clone(),
        mint_block: c.mint_block,
        source: source.to_string(),
        tracked_at: None,
    })
}

// ------------------------------------------------------------------
// One cycle's scan
// ------------------------------------------------------------------

/// What the scan read about one found position this cycle, for its value.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LiveRead {
    pub(crate) liquidity: u128,
    /// The pool's price; None when it could not be read.
    pub(crate) slot0: Option<(U256, i32)>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct LogsReport {
    /// used | refused | failed | not_run
    pub(crate) status: String,
    pub(crate) covered: Option<(i64, i64)>,
    pub(crate) calls: usize,
    pub(crate) reason: Option<String>,
    /// Blocks below this are not served by this RPC, with its reason.
    pub(crate) refused_below: Option<(i64, String)>,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct HistoryReport {
    /// not_used | complete | partial | failed
    pub(crate) status: String,
    pub(crate) pages: usize,
    pub(crate) reason: Option<String>,
}

/// What one cycle's scan managed to read.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct ScanRun {
    /// The block every read of this cycle names. None when the head was not read.
    pub(crate) block: Option<i64>,
    pub(crate) logs: LogsReport,
    pub(crate) history: HistoryReport,
    /// NFTs whose reads failed this cycle, and NFTs left for a later cycle by a limit.
    pub(crate) unread: usize,
    pub(crate) deferred: usize,
    /// Untracked NFTs whose re-read waits for a later cycle (the per-cycle cap).
    pub(crate) refresh_deferred: usize,
    /// The check stopped starting reads at its time budget.
    pub(crate) time_cut: bool,
    /// `balanceOf(wallet)` at the block. None when it could not be read.
    pub(crate) held: Option<u64>,
    /// NFTs known as owned by the wallet after this cycle's reads.
    pub(crate) known_owned: u64,
    pub(crate) reads: HashMap<u64, LiveRead>,
    /// Set when the scan stopped before it read anything: the reason.
    pub(crate) failed: Option<String>,
    /// When the transfers were last read through, from the stored state.
    pub(crate) previously_checked_at: Option<DateTime<Utc>>,
}

impl ScanRun {
    fn failed(reason: &str) -> ScanRun {
        ScanRun { failed: Some(reason.to_string()), ..Default::default() }
    }
}

/// What the cycle hands to the snapshot.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct DetectionOutcome {
    /// ok | partial | unavailable | not_run
    pub(crate) status: String,
    pub(crate) reason: Option<String>,
    pub(crate) last_checked_at: Option<DateTime<Utc>>,
    pub(crate) run: ScanRun,
    /// The scan's coverage, as the snapshot states it.
    pub(crate) scan: Value,
    /// Found and not counted: offered, then dismissed. Each with why and the choices.
    pub(crate) found: Vec<FoundPosition>,
    /// NFTs never listed, by reason.
    pub(crate) ignored: BTreeMap<String, u64>,
    /// Position keys tracked by this cycle's detection.
    pub(crate) newly_tracked: Vec<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FoundPosition {
    pub(crate) row: CandidateRow,
    pub(crate) dismissed: bool,
    pub(crate) why_not_automatic: Vec<String>,
    pub(crate) choices: Vec<String>,
    /// None when it was not read this cycle: its value is then not stated.
    pub(crate) read: Option<LiveRead>,
}

impl DetectionOutcome {
    pub(crate) fn not_run(reason: &str) -> DetectionOutcome {
        DetectionOutcome { status: "not_run".to_string(), reason: Some(reason.to_string()), scan: json!({ "coverage": "none", "text": reason }), ..Default::default() }
    }
}

pub(crate) struct DetectionInputs<'a> {
    pub(crate) store: &'a PositionStore,
    pub(crate) http: &'a Client,
    /// None when no RPC key is configured.
    pub(crate) rpc_url: Option<&'a str>,
    pub(crate) wallet: Option<&'a str>,
    pub(crate) config_positions: &'a [PositionConfig],
    /// The previous snapshot's `data`, for the suggestion's rungs and the live ladders.
    pub(crate) prev: Option<&'a Value>,
    pub(crate) now: DateTime<Utc>,
}

/// The v4 NFT ids config.yaml lists on Base. They are read and counted through config.
pub(crate) fn config_v4_ids(config_positions: &[PositionConfig]) -> HashSet<u64> {
    config_positions.iter().filter(|c| c.protocol == TRACKED_PROTOCOL_V4 && c.network == TRACKED_NETWORK_BASE).map(|c| c.nft_id).collect()
}

/// Pool ids NockNock already reads, to identify the pool of an NFT whose mint receipt is
/// not available (it arrived by transfer): config.yaml's v4 pools, the tracked rows' and
/// the suggestion's. Lowercase, 0x-prefixed.
fn known_pool_ids(config_positions: &[PositionConfig], tracked: &[TrackedRow], suggested: &[SuggestedRung]) -> Vec<String> {
    let mut out: Vec<String> = Vec::new();
    let mut add = |p: &str| {
        let p = lc(p);
        let p = if p.starts_with("0x") { p } else { format!("0x{p}") };
        if p.len() == 66 && !out.contains(&p) {
            out.push(p);
        }
    };
    config_positions.iter().filter(|c| c.protocol == TRACKED_PROTOCOL_V4).filter_map(|c| c.pool_id.as_deref()).for_each(&mut add);
    tracked.iter().for_each(|t| add(&t.pool_id));
    suggested.iter().for_each(|s| add(&s.pool_id));
    out
}

fn pool_matches_prefix(pool_id: &str, prefix: &str) -> bool {
    lc(pool_id).trim_start_matches("0x").starts_with(&lc(prefix))
}

enum Classified {
    Row(CandidateRow),
    Unread,
    Deferred,
}

struct Reader<'a> {
    http: &'a Client,
    url: &'a str,
    wallet: String,
    tag: String,
    known_pools: Vec<String>,
    receipts_left: usize,
    /// Liquidity read this cycle, so no NFT is asked twice.
    liquidity_read: HashMap<u64, u128>,
}

impl Reader<'_> {
    async fn liquidity(&mut self, nft_id: u64) -> Option<u128> {
        if let Some(l) = self.liquidity_read.get(&nft_id) {
            return Some(*l);
        }
        let read = match call_at(self.http, self.url, UNISWAP_V4_POSM_BASE, &format!("{V4_GET_POSITION_LIQUIDITY}{nft_id:064x}"), &self.tag).await {
            Ok(CallOutcome::Returned(hex)) if hex.len() == 64 => u128::from_str_radix(&hex[32..], 16).ok().filter(|_| hex[..32].bytes().all(|b| b == b'0')),
            _ => None,
        };
        if let Some(l) = read {
            self.liquidity_read.insert(nft_id, l);
        }
        read
    }

    /// Some(Some(owner)) = an owner; Some(None) = the NFT does not exist; None = unread.
    async fn owner(&self, nft_id: u64) -> Option<Option<String>> {
        match call_at(self.http, self.url, UNISWAP_V4_POSM_BASE, &format!("{ERC721_OWNER_OF}{nft_id:064x}"), &self.tag).await {
            Ok(CallOutcome::Returned(hex)) => owner_from_word(&hex).map(Some),
            Ok(reverted) => v4_owner_of_says_nonexistent(&reverted).ok().map(|_| None),
            Err(_) => None,
        }
    }

    async fn slot0(&self, pool_id: &str) -> Option<(U256, i32)> {
        let data = format!("{V4_STATE_VIEW_GET_SLOT0}{}", lc(pool_id).trim_start_matches("0x"));
        match call_at(self.http, self.url, UNISWAP_V4_STATE_VIEW_BASE, &data, &self.tag).await {
            Ok(CallOutcome::Returned(hex)) if hex.len() >= 128 => Some((U256::from_hex_be(&hex[..64]).ok()?, parse_hex_word_i32(&hex[64..128]).ok()?)),
            _ => None,
        }
    }

    /// Read one NFT. `received` is the transfer that brought it to the wallet (from this
    /// cycle's scan, or the stored one); `prior` is its stored row.
    async fn classify(&mut self, nft_id: u64, received: Option<&TransferSeen>, prior: Option<&CandidateRow>) -> Classified {
        let mut row = CandidateRow {
            wallet: self.wallet.clone(),
            protocol: TRACKED_PROTOCOL_V4.to_string(),
            network: TRACKED_NETWORK_BASE.to_string(),
            nft_id,
            choice: prior.and_then(|p| p.choice.clone()),
            received_tx: received.map(|r| r.tx_hash.clone()).or_else(|| prior.and_then(|p| p.received_tx.clone())),
            received_block: received.map(|r| r.block).or_else(|| prior.and_then(|p| p.received_block)),
            received_from: received.map(|r| r.from.clone()).or_else(|| prior.and_then(|p| p.received_from.clone())),
            ..Default::default()
        };
        let info = match call_at(self.http, self.url, UNISWAP_V4_POSM_BASE, &format!("{V4_GET_POOL_AND_POSITION_INFO}{nft_id:064x}"), &self.tag).await {
            Ok(CallOutcome::Returned(hex)) => match decode_pool_and_position_info(&hex) {
                Ok(i) => i,
                Err(_) => return Classified::Unread,
            },
            _ => return Classified::Unread,
        };
        let info = match info {
            NftInfoRead::Gone => {
                row.verdict = "gone".to_string();
                return Classified::Row(row);
            }
            NftInfoRead::Info(i) => i,
        };
        row.verdict = pool_key_verdict(&info.currency0, &info.currency1, &info.hooks).to_string();
        row.currency0 = Some(info.currency0.clone());
        row.currency1 = Some(info.currency1.clone());
        row.fee_raw = Some(info.fee_raw as i64);
        row.tick_spacing = Some(info.tick_spacing);
        row.hooks = Some(info.hooks.clone());
        row.pool_id_prefix = Some(info.pool_id_prefix.clone());
        row.tick_lower = Some(info.tick_lower);
        row.tick_upper = Some(info.tick_upper);
        match self.owner(nft_id).await {
            None => return Classified::Unread,
            Some(None) => {
                row.verdict = "gone".to_string();
                return Classified::Row(row);
            }
            Some(Some(owner)) => row.owned = owner == self.wallet,
        }
        if row.verdict != "eligible" {
            return Classified::Row(row);
        }
        let Some(liquidity) = self.liquidity(nft_id).await else {
            return Classified::Unread;
        };
        row.has_liquidity = Some(liquidity > 0);
        // The pool id, from a pool NockNock already reads when the prefix names one.
        row.pool_id = self.known_pools.iter().find(|p| pool_matches_prefix(p, &info.pool_id_prefix)).cloned();
        if !(row.owned && liquidity > 0) {
            return Classified::Row(row);
        }
        // The mint: kept from the stored row when it is the same transfer, else read.
        let same_transfer = prior.is_some_and(|p| p.received_tx == row.received_tx && p.received_tx.is_some());
        if let Some(p) = prior.filter(|p| same_transfer && (p.mint_tx.is_some() || p.deposit_check.is_some())) {
            row.mint_tx = p.mint_tx.clone();
            row.mint_block = p.mint_block;
            row.minted_by = p.minted_by.clone();
            row.deposit_check = p.deposit_check.clone();
            row.pool_id = p.pool_id.clone().or(row.pool_id);
            return Classified::Row(row);
        }
        let minted = row.received_from.as_deref() == Some(ZERO_ADDRESS);
        let Some(tx) = row.received_tx.clone().filter(|_| minted) else {
            return Classified::Row(row);
        };
        if self.receipts_left == 0 {
            return Classified::Deferred;
        }
        self.receipts_left -= 1;
        let payload = json!({ "jsonrpc": "2.0", "id": 149, "method": "eth_getTransactionReceipt", "params": [tx] });
        let receipt = match rpc_answer(self.http, self.url, &payload).await {
            RpcAnswer::Result(v) if v.get("result").is_some_and(|r| r.is_object()) => v["result"].clone(),
            _ => return Classified::Unread,
        };
        // A receipt that was read and does not show the mint is a verdict too: it is
        // stored, so the same receipt is not asked for again every cycle.
        let facts = match mint_facts(&receipt, &self.wallet, nft_id) {
            Ok(f) if f.minted_to_wallet => f,
            Ok(_) => {
                row.deposit_check = Some("the transaction that brought it to the wallet does not mint it to the wallet".to_string());
                return Classified::Row(row);
            }
            Err(why) => {
                row.deposit_check = Some(format!("the transaction that brought it to the wallet could not be read as a mint ({why})"));
                return Classified::Row(row);
            }
        };
        row.mint_tx = Some(tx);
        row.mint_block = Some(facts.block);
        row.minted_by = Some(facts.sender);
        // The receipt's pool id must be the pool the position manager stores for the NFT.
        if let Some(pool) = facts.pool_id.filter(|p| pool_matches_prefix(p, &info.pool_id_prefix)) {
            row.pool_id = Some(pool);
        }
        row.deposit_check = Some(match row.pool_id.as_deref().and_then(|pool| candidate_raw_position(&row, pool)) {
            Some(p) => match parse_ladder_deposit(&receipt, &self.wallet, &p) {
                Ok(_) => "ok".to_string(),
                Err(reason) => reason,
            },
            None => "its pool could not be identified from the receipt".to_string(),
        });
        Classified::Row(row)
    }
}

/// Read the wallet's transfers from the logs, as far as this cycle's request budget goes.
async fn read_logs(http: &Client, url: &str, wallet: &str, head: i64, state: &mut ScanState, seen: &mut HashMap<u64, TransferSeen>) -> LogsReport {
    let mut limits = known_log_limits(url);
    let mut covered = match (state.logs_from_block, state.logs_to_block) {
        (Some(a), Some(b)) if a <= b => Some((a, b)),
        _ => None,
    };
    let mut report = LogsReport { status: "used".to_string(), covered, ..Default::default() };
    if let Some(why) = &limits.unusable {
        report.status = "refused".to_string();
        report.reason = Some(why.clone());
        return report;
    }
    while report.calls < POSITION_LOG_MAX_CALLS_PER_CYCLE {
        let floor = limits.refused_below.as_ref().map(|(b, _)| *b).unwrap_or(0);
        let Some(w) = next_log_window(covered, head, !limits.chunked, floor) else { break };
        report.calls += 1;
        match rpc_answer(http, url, &transfer_logs_request(wallet, w.from, w.to)).await {
            RpcAnswer::Result(v) => match parse_transfer_logs(&v, wallet) {
                Ok(list) => {
                    absorb_transfers(seen, list);
                    covered = Some(cover(covered, w));
                }
                Err(e) => {
                    report.status = "failed".to_string();
                    report.reason = Some(format!("the node's log answer was not understood ({e})"));
                    break;
                }
            },
            RpcAnswer::Refused(why) => {
                if w.blocks() > POSITION_LOG_CHUNK_BLOCKS {
                    // A wide range was refused: read in chunks from here on.
                    limits.chunked = true;
                    remember_log_limits(url, &limits);
                    continue;
                }
                match covered {
                    // Nothing was ever read and one chunk at the head is refused: this
                    // RPC does not serve logs in ranges of any use (a ten-block limit).
                    None => {
                        limits.unusable = Some(why.clone());
                        remember_log_limits(url, &limits);
                        report.status = "refused".to_string();
                        report.reason = Some(why);
                    }
                    // A chunk of new blocks is refused by an RPC that served them before:
                    // this cycle's failure, not a limit to remember.
                    Some((_, hi)) if w.from > hi => {
                        report.status = "failed".to_string();
                        report.reason = Some(why);
                    }
                    // A backfill chunk is refused: blocks below it are not served.
                    Some(_) => {
                        limits.refused_below = Some((w.to + 1, why));
                        remember_log_limits(url, &limits);
                        continue;
                    }
                }
                break;
            }
            RpcAnswer::NoAnswer(why) => {
                report.status = "failed".to_string();
                report.reason = Some(why);
                break;
            }
        }
    }
    report.covered = covered;
    report.refused_below = limits.refused_below.clone();
    state.logs_from_block = covered.map(|c| c.0);
    state.logs_to_block = covered.map(|c| c.1);
    report
}

/// Read the wallet's transfers from the RPC's index, newest first. It stops at the end of
/// the history, or (once the whole history was read before) at a page with nothing new.
async fn read_history(http: &Client, url: &str, wallet: &str, known: &HashSet<u64>, read_before: bool, seen: &mut HashMap<u64, TransferSeen>) -> HistoryReport {
    let mut report = HistoryReport { status: "partial".to_string(), ..Default::default() };
    let mut page_key: Option<String> = None;
    loop {
        let page = match rpc_answer(http, url, &nft_history_request(wallet, page_key.as_deref())).await {
            RpcAnswer::Result(v) => parse_nft_history(&v).map_err(|e| format!("the transfer index's answer was not understood ({e})")),
            RpcAnswer::Refused(why) | RpcAnswer::NoAnswer(why) => Err(why),
        };
        let (list, next) = match page {
            Ok(p) => p,
            Err(why) => {
                report.status = "failed".to_string();
                report.reason = Some(why);
                return report;
            }
        };
        report.pages += 1;
        let any_new = list.iter().any(|t| !known.contains(&t.nft_id) && !seen.contains_key(&t.nft_id));
        absorb_transfers(seen, list);
        if next.is_none() || (read_before && !any_new) {
            report.status = "complete".to_string();
            return report;
        }
        if report.pages >= POSITION_HISTORY_MAX_PAGES {
            report.reason = Some(format!("the transfer index was read for {} pages and has more", report.pages));
            return report;
        }
        page_key = next;
    }
}

/// One cycle's reads. Everything it learns is stored; the lists are built afterwards
/// from the store, so what an earlier cycle found stays listed when this one fails.
async fn scan(inp: &DetectionInputs<'_>, wallet: &str, url: &str) -> ScanRun {
    // The tables first: when the chain cannot be read, the failure still says when the
    // transfers were last read.
    let (mut state, candidates, tracked) = match (
        inp.store.scan_state(wallet, TRACKED_PROTOCOL_V4, TRACKED_NETWORK_BASE).await,
        inp.store.candidates(wallet).await,
        inp.store.tracked(wallet).await,
    ) {
        (Ok(s), Ok(c), Ok(t)) => (s, c, t),
        (a, b, c) => {
            for e in [a.err(), b.err(), c.err()].into_iter().flatten() {
                tracing::warn!("Position detection: tables unreadable: {e}");
            }
            return ScanRun::failed("the detection tables could not be read");
        }
    };
    let (latest, _) = match latest_block_header(inp.http, url).await {
        Ok(h) => h,
        Err(_) => return ScanRun { previously_checked_at: state.last_checked_at, ..ScanRun::failed("the node's latest block could not be read") },
    };
    let head = (latest - POSITION_HEAD_LAG_BLOCKS).max(0);
    let mut run = ScanRun { block: Some(head), previously_checked_at: state.last_checked_at, ..Default::default() };
    let soft_deadline = std::time::Instant::now() + std::time::Duration::from_secs(POSITION_SCAN_SOFT_SECONDS);
    let out_of_time = || std::time::Instant::now() >= soft_deadline;
    let mut seen: HashMap<u64, TransferSeen> = HashMap::new();

    // 1. Transfers: the logs, then the index while the logs do not cover the history.
    run.logs = read_logs(inp.http, url, wallet, head, &mut state, &mut seen).await;
    let logs_complete = run.logs.covered.is_some_and(|(lo, hi)| lo <= 0 && hi >= head);
    run.history = if logs_complete {
        HistoryReport { status: "not_used".to_string(), ..Default::default() }
    } else {
        let known: HashSet<u64> = candidates.iter().map(|c| c.nft_id).collect();
        let h = read_history(inp.http, url, wallet, &known, state.history_complete_at.is_some(), &mut seen).await;
        if h.status == "complete" {
            state.history_complete_at = Some(inp.now);
        }
        h
    };

    // 2. Read every NFT that is new, or that arrived again since it was last read.
    let suggested = inp.prev.map(suggested_rungs).unwrap_or_default();
    let in_config = config_v4_ids(inp.config_positions);
    let tracked_ids: HashSet<u64> = tracked.iter().map(|t| t.nft_id).collect();
    let mut rows: HashMap<u64, CandidateRow> = candidates.into_iter().map(|c| (c.nft_id, c)).collect();
    let mut reader = Reader {
        http: inp.http,
        url,
        wallet: lc(wallet),
        tag: format!("0x{head:x}"),
        known_pools: known_pool_ids(inp.config_positions, &tracked, &suggested),
        receipts_left: POSITION_RECEIPTS_MAX_PER_CYCLE,
        liquidity_read: HashMap::new(),
    };
    let mut todo: Vec<u64> = seen
        .values()
        .filter(|t| rows.get(&t.nft_id).map_or(true, |r| r.received_block.map_or(true, |b| b < t.block)))
        .map(|t| t.nft_id)
        .collect();
    todo.sort_unstable();
    let mut fresh: HashSet<u64> = HashSet::new();
    for (i, nft_id) in todo.iter().enumerate() {
        if i >= POSITION_CLASSIFY_MAX_PER_CYCLE {
            run.deferred += 1;
            continue;
        }
        if out_of_time() {
            run.time_cut = true;
            run.deferred += 1;
            continue;
        }
        let prior = rows.get(nft_id).cloned();
        match reader.classify(*nft_id, seen.get(nft_id), prior.as_ref()).await {
            Classified::Row(row) => store_row(inp.store, &mut rows, &mut fresh, &mut run, row).await,
            Classified::Unread => run.unread += 1,
            Classified::Deferred => run.deferred += 1,
        }
    }
    // 2b. And again every eligible NFT the wallet owned that is not read as a position.
    //     Its pool and ticks never change, so only what can change is read:
    //     - an empty one: its liquidity (it can be refilled with no transfer);
    //     - one that holds liquidity (offered or set aside): its owner and its liquidity.
    //     One that became funded, or whose mint was never read, is read in full.
    let mut refresh: Vec<u64> = rows
        .values()
        .filter(|r| r.verdict == "eligible" && r.owned && !in_config.contains(&r.nft_id) && !tracked_ids.contains(&r.nft_id))
        .map(|r| r.nft_id)
        .filter(|id| !todo.contains(id))
        .collect();
    refresh.sort_unstable();
    let max = POSITION_REFRESH_MAX_PER_CYCLE;
    // The cursor moves only when there are more than the cap, so every one is read in turn.
    let cursor = if refresh.len() > max { REFRESH_CURSOR.fetch_add(max, std::sync::atomic::Ordering::Relaxed) } else { 0 };
    let (refresh, waiting) = refresh_batch(&refresh, cursor, max);
    run.refresh_deferred = waiting;
    for nft_id in refresh {
        if out_of_time() {
            run.time_cut = true;
            run.refresh_deferred += 1;
            continue;
        }
        let prior = rows[&nft_id].clone();
        let Some(liquidity) = reader.liquidity(nft_id).await else {
            run.unread += 1;
            continue;
        };
        let was_funded = prior.has_liquidity == Some(true);
        let mint_unread = prior.received_from.as_deref() == Some(ZERO_ADDRESS) && prior.mint_tx.is_none() && prior.deposit_check.is_none();
        if liquidity > 0 && (!was_funded || mint_unread) {
            match reader.classify(nft_id, None, Some(&prior)).await {
                Classified::Row(row) => store_row(inp.store, &mut rows, &mut fresh, &mut run, row).await,
                Classified::Unread => run.unread += 1,
                Classified::Deferred => run.deferred += 1,
            }
            continue;
        }
        if !was_funded {
            // Still empty: nothing to store.
            fresh.insert(nft_id);
            continue;
        }
        let owner = match reader.owner(nft_id).await {
            Some(o) => o,
            None => {
                run.unread += 1;
                continue;
            }
        };
        let owned = owner.as_deref() == Some(reader.wallet.as_str());
        if owned && liquidity > 0 {
            fresh.insert(nft_id);
            continue;
        }
        let mut row = prior;
        row.owned = owned;
        row.has_liquidity = Some(liquidity > 0);
        if owner.is_none() {
            row.verdict = "gone".to_string();
        }
        store_row(inp.store, &mut rows, &mut fresh, &mut run, row).await;
    }

    // 3. The exact cross-check: how many NFTs the wallet holds on the position manager.
    let data = format!("{ERC721_BALANCE_OF}{}", addr_topic(wallet).trim_start_matches("0x"));
    run.held = match call_at(inp.http, url, UNISWAP_V4_POSM_BASE, &data, &reader.tag).await {
        Ok(CallOutcome::Returned(hex)) if hex.len() == 64 => u64::from_str_radix(&hex[48..], 16).ok(),
        _ => None,
    };
    let owned_count = |rows: &HashMap<u64, CandidateRow>| rows.values().filter(|r| r.owned).count() as u64;
    // 3b. Fewer held than known as owned: some left the wallet. Read those owners again.
    if run.held.is_some_and(|h| h < owned_count(&rows)) {
        let stale: Vec<u64> = rows.values().filter(|r| r.owned && !fresh.contains(&r.nft_id)).map(|r| r.nft_id).take(POSITION_OWNER_RESYNC_MAX_PER_CYCLE).collect();
        for nft_id in stale {
            if out_of_time() {
                run.time_cut = true;
                break;
            }
            if let Some(owner) = reader.owner(nft_id).await {
                let mut row = rows[&nft_id].clone();
                let owned = owner.as_deref() == Some(reader.wallet.as_str());
                if !owned {
                    row.owned = false;
                    if owner.is_none() {
                        row.verdict = "gone".to_string();
                    }
                    store_row(inp.store, &mut rows, &mut fresh, &mut run, row).await;
                }
            }
        }
    }
    run.known_owned = owned_count(&rows);

    // 4. What each found position holds now: its liquidity and its pool's price.
    let mut prices: HashMap<String, Option<(U256, i32)>> = HashMap::new();
    for r in rows.values().filter(|r| is_found(r, &in_config, &tracked_ids) && fresh.contains(&r.nft_id)) {
        if out_of_time() {
            run.time_cut = true;
            break;
        }
        let Some(liquidity) = reader.liquidity(r.nft_id).await else { continue };
        let slot0 = match r.pool_id.as_deref() {
            Some(pool) => match prices.get(pool) {
                Some(p) => *p,
                None => {
                    let p = reader.slot0(pool).await;
                    prices.insert(pool.to_string(), p);
                    p
                }
            },
            None => None,
        };
        run.reads.insert(r.nft_id, LiveRead { liquidity, slot0 });
    }

    // 5. Remember how far this cycle read.
    let read_something = run.logs.covered.is_some_and(|(_, hi)| hi >= head) || run.history.status == "complete";
    if read_something {
        state.last_checked_at = Some(inp.now);
    }
    if let Err(e) = inp.store.save_scan_state(wallet, TRACKED_PROTOCOL_V4, TRACKED_NETWORK_BASE, &state).await {
        tracing::warn!("Position detection: could not store the scan state: {e}");
    }
    run
}

async fn store_row(store: &PositionStore, rows: &mut HashMap<u64, CandidateRow>, fresh: &mut HashSet<u64>, run: &mut ScanRun, row: CandidateRow) {
    match store.upsert_candidate(&row).await {
        Ok(()) => {
            fresh.insert(row.nft_id);
            rows.insert(row.nft_id, row);
        }
        Err(e) => {
            // Not stored means not read: it is tried again next cycle.
            tracing::warn!("Position detection: could not store NFT #{}: {e}", row.nft_id);
            run.unread += 1;
        }
    }
}

/// A position that is found and not counted: eligible, owned, holding liquidity, and
/// neither in config.yaml nor tracked. Pure.
fn is_found(r: &CandidateRow, in_config: &HashSet<u64>, tracked: &HashSet<u64>) -> bool {
    r.verdict == "eligible" && r.owned && r.has_liquidity == Some(true) && !in_config.contains(&r.nft_id) && !tracked.contains(&r.nft_id)
}

/// The status, its reason and the coverage statement, from what the scan read. Pure.
/// - `ok`: the transfers were read up to this cycle's block and back to the first block
///   (or the RPC's index was read to its end), every NFT was read, and the wallet holds
///   no more NFTs than are known.
/// - `partial`: this cycle read the newest transfers, and something is missing; the
///   reason says exactly what.
/// - `unavailable`: this cycle could not read the newest transfers at all.
pub(crate) fn detection_status(run: &ScanRun) -> (&'static str, Option<String>, Value) {
    if let Some(why) = &run.failed {
        let since = run.previously_checked_at.map(|t| format!(" The transfers were last read at {}.", t.to_rfc3339())).unwrap_or_default();
        let reason = format!("New positions could not be checked this cycle ({why}): a deposit made since the last check would not be counted yet.{since}");
        return ("unavailable", Some(reason.clone()), json!({ "coverage": "none", "text": reason }));
    }
    let head = run.block.unwrap_or(0);
    let logs_fresh = run.logs.covered.is_some_and(|(_, hi)| hi >= head);
    let logs_complete = run.logs.covered.is_some_and(|(lo, hi)| lo <= 0 && hi >= head);
    let history_complete = run.history.status == "complete";
    let sources = json!({ "transfer_logs": run.logs.status, "transfer_index": run.history.status });
    let behind = run.logs.covered.map(|(_, hi)| (head - hi).max(0));
    let source_reasons = || -> String {
        let mut r: Vec<String> = Vec::new();
        if let Some(why) = &run.logs.reason {
            r.push(format!("transfer logs: {why}"));
        }
        if let Some(why) = &run.history.reason {
            r.push(format!("transfer index: {why}"));
        }
        if r.is_empty() { "no source could be read".to_string() } else { r.join("; ") }
    };
    if !logs_fresh && !history_complete {
        let through = match run.logs.covered {
            Some((_, hi)) => format!(" The transfer logs were last read through block {hi}."),
            None => " No transfers have been read yet.".to_string(),
        };
        let reason = format!("New positions could not be checked this cycle ({}): a deposit made since the last check would not be counted yet.{through}", source_reasons());
        let scan = json!({ "coverage": "none", "sources": sources, "logs_from_block": run.logs.covered.map(|c| c.0), "logs_behind_blocks": behind, "text": reason });
        return ("unavailable", Some(reason), scan);
    }
    let mut gaps: Vec<String> = Vec::new();
    let coverage_complete = logs_complete || history_complete;
    let coverage_text = if logs_complete {
        "Every Uniswap v4 position NFT sent to the wallet was read from the transfer logs, from the first block to this cycle's block.".to_string()
    } else if history_complete {
        let logs = match run.logs.covered {
            Some((lo, _)) if logs_fresh => format!(" The transfer logs, which have no delay, cover block {lo} onward."),
            _ => String::new(),
        };
        format!("Every Uniswap v4 position NFT sent to the wallet was read from the RPC's transfer index, which can trail the chain by a short time.{logs}")
    } else {
        let (lo, _) = run.logs.covered.unwrap_or((head, head));
        let why = match &run.logs.refused_below {
            Some((_, why)) => format!("this RPC does not serve older blocks ({why})"),
            None => format!("the backfill reads up to {} blocks more each cycle", POSITION_LOG_CHUNK_BLOCKS * (POSITION_LOG_MAX_CALLS_PER_CYCLE as i64)),
        };
        let text = format!("The wallet's position-NFT transfers were read from block {lo} to this cycle's block. A position that reached the wallet before block {lo} is not found yet: {why}.");
        gaps.push(format!("transfers before block {lo} are not read yet ({why}; {})", source_reasons()));
        text
    };
    if run.unread > 0 {
        gaps.push(format!("{} position NFT(s) could not be read and will be tried again next cycle", run.unread));
    }
    if run.deferred > 0 {
        gaps.push(format!("{} more position NFT(s) are still to be read (a per-cycle limit)", run.deferred));
    }
    if run.refresh_deferred > 0 {
        gaps.push(format!(
            "{} untracked position NFT(s) the wallet holds are read again in later cycles (at most {POSITION_REFRESH_MAX_PER_CYCLE} a cycle, in turn); what they hold is not stated this cycle",
            run.refresh_deferred
        ));
    }
    if run.time_cut {
        gaps.push(format!("the check stopped starting reads at its {POSITION_SCAN_SOFT_SECONDS}-second budget; the rest is read next cycle"));
    }
    match run.held {
        Some(h) if h > run.known_owned => gaps.push(format!(
            "the wallet holds {h} Uniswap v4 position NFTs on-chain and {} are known: {} not found yet",
            run.known_owned,
            h - run.known_owned
        )),
        Some(_) => {}
        None => gaps.push("the wallet's NFT count on the position manager could not be read, so the scan was not cross-checked".to_string()),
    }
    let scan = json!({
        "coverage": if coverage_complete { "complete" } else { "partial" },
        "sources": sources,
        "logs_from_block": run.logs.covered.map(|c| c.0),
        "logs_behind_blocks": behind,
        "text": coverage_text,
    });
    if gaps.is_empty() {
        ("ok", None, scan)
    } else {
        ("partial", Some(gaps.join("; ")), scan)
    }
}

/// One cycle's detection. It never returns an error and never fails the cycle: a check
/// that could not run is reported with its reason, and what earlier cycles found stays
/// listed.
pub(crate) async fn detect_positions(inp: &DetectionInputs<'_>) -> DetectionOutcome {
    let Some(wallet) = inp.wallet.map(lc) else {
        return DetectionOutcome::not_run("no tracked wallet is configured");
    };
    let run = match inp.rpc_url {
        None => ScanRun::failed("no Alchemy RPC key is configured"),
        Some(url) => match tokio::time::timeout(tokio::time::Duration::from_secs(POSITION_SCAN_TIMEOUT_SECONDS), scan(inp, &wallet, url)).await {
            Ok(run) => run,
            Err(_) => ScanRun::failed(&format!("the check took longer than {POSITION_SCAN_TIMEOUT_SECONDS} seconds and was abandoned")),
        },
    };
    let (status, reason, scan_json) = detection_status(&run);
    if let Some(why) = &reason {
        if status == "unavailable" {
            tracing::warn!("Position detection could not check this cycle: {why}");
        }
    }
    let mut out = DetectionOutcome {
        status: status.to_string(),
        reason,
        last_checked_at: if status == "unavailable" { run.previously_checked_at } else { Some(inp.now) },
        scan: scan_json,
        ..Default::default()
    };

    // The lists come from the store, so what an earlier check found stays listed.
    let (candidates, tracked) = match (inp.store.candidates(&wallet).await, inp.store.tracked(&wallet).await) {
        (Ok(c), Ok(t)) => (c, t),
        (a, b) => {
            for e in [a.err(), b.err()].into_iter().flatten() {
                tracing::warn!("Position detection: lists unreadable: {e}");
            }
            out.status = "unavailable".to_string();
            out.reason = Some("Positions found on-chain could not be listed this cycle (the detection tables could not be read): one that is found and not counted is not shown.".to_string());
            out.run = run;
            return out;
        }
    };
    let in_config = config_v4_ids(inp.config_positions);
    let mut tracked_ids: HashSet<u64> = tracked.iter().map(|t| t.nft_id).collect();
    let suggested = inp.prev.map(suggested_rungs).unwrap_or_default();
    let mut ranges = inp.prev.map(tracked_ranges_in_snapshot).unwrap_or_default();
    for t in tracked.iter().filter(|t| t.role.as_deref() == Some(LADDER_ROLE)) {
        if !ranges.iter().any(|r| r.position_id == t.position_key) {
            ranges.push(TrackedRange { position_id: t.position_key.clone(), pool_id: lc(&t.pool_id), tick_lower: t.tick_lower, tick_upper: t.tick_upper });
        }
    }
    for c in &candidates {
        let bucket = if in_config.contains(&c.nft_id) {
            Some("in_config")
        } else if c.verdict != "eligible" {
            Some(c.verdict.as_str())
        } else if !c.owned {
            Some("not_owned")
        } else if c.has_liquidity != Some(true) {
            Some("empty")
        } else {
            None
        };
        if let Some(b) = bucket {
            *out.ignored.entry(b.to_string()).or_insert(0) += 1;
        }
    }
    let found: Vec<&CandidateRow> = candidates.iter().filter(|c| is_found(c, &in_config, &tracked_ids)).collect();
    for c in found {
        let dismissed = c.choice.as_deref() == Some(CHOICE_DISMISSED);
        let read = run.reads.get(&c.nft_id).cloned();
        let decision = decide(c, &wallet, &suggested, &ranges);
        // Tracked by itself only on this cycle's own reads, and never against a dismissal.
        if let (Decision::Track { rung }, false, Some(_)) = (&decision, dismissed, &read) {
            if let Some(row) = tracked_row_for(c, Some(LADDER_ROLE), rung.as_deref(), TRACKED_SOURCE_DETECTED).filter(|r| validate_tracked_row(r).is_ok()) {
                match inp.store.insert_tracked_if_absent(&row).await {
                    Ok(_) => {
                        tracing::info!("Position detection: NFT #{} matches a suggested rung and is tracked as a ladder (mint {})", c.nft_id, row.deposit_tx.clone().unwrap_or_default());
                        ranges.push(TrackedRange { position_id: row.position_key.clone(), pool_id: lc(&row.pool_id), tick_lower: row.tick_lower, tick_upper: row.tick_upper });
                        tracked_ids.insert(c.nft_id);
                        out.newly_tracked.push(row.position_key);
                        continue;
                    }
                    Err(e) => tracing::warn!("Position detection: could not track NFT #{}: {e}", c.nft_id),
                }
            }
        }
        let (why_not_automatic, choices) = match decision {
            Decision::Offer { why_not_automatic, choices } => (why_not_automatic, choices),
            Decision::Track { .. } if dismissed => (vec!["you set it aside (NOT NOW or UNTRACK)".to_string()], track_choices(c)),
            Decision::Track { .. } if read.is_none() => (vec!["it was not read this cycle, so it is not tracked on an old reading".to_string()], track_choices(c)),
            // It matched and could not be written (the row did not pass its checks, or
            // the table could not be written): said as it is, and tried again next cycle.
            Decision::Track { .. } => (vec!["it matches a suggested rung, and tracking it did not go through this cycle; it is tried again next cycle".to_string()], track_choices(c)),
        };
        out.found.push(FoundPosition { row: c.clone(), dismissed, why_not_automatic, choices, read });
    }
    out.found.sort_by_key(|f| (f.dismissed, f.row.nft_id));
    out.run = run;
    out
}

/// The cycle's detection, from the worker's state.
pub(crate) async fn detect_positions_for_cycle(state: &Arc<AppState>) -> DetectionOutcome {
    let store = PositionStore::of(state);
    let key = state.config.alchemy_api_keys.first().cloned().unwrap_or_default();
    let url = (!key.is_empty() && key != "YOUR_ALCHEMY_API_KEY").then(|| format!("https://base-mainnet.g.alchemy.com/v2/{key}"));
    let prev: Option<Value> = state.latest_snapshot.lock().await.as_ref().and_then(|s| serde_json::to_value(&s.data).ok());
    detect_positions(&DetectionInputs {
        store: &store,
        http: &state.http_client,
        rpc_url: url.as_deref(),
        wallet: state.config.tracked_wallets.first().map(String::as_str),
        config_positions: &state.config.positions,
        prev: prev.as_ref(),
        now: Utc::now(),
    })
    .await
}

// ------------------------------------------------------------------
// The snapshot block (pure)
// ------------------------------------------------------------------

fn fx(v: f64, dp: usize) -> String {
    let m = 10f64.powi(dp as i32);
    format!("{:.*}", dp, (v * m).round() / m + 0.0)
}

/// "v4 USDC/NOCK 3.00%" from a candidate's pool key. Pure.
pub(crate) fn candidate_pool_label(c: &CandidateRow) -> String {
    let sym = |a: Option<&str>| a.and_then(|a| token_info(a).ok()).map(|t| t.symbol).unwrap_or_else(|| "?".to_string());
    let fee = c.fee_raw.map(|f| format!(" {:.2}%", f as f64 / 10_000.0)).unwrap_or_default();
    format!("v4 {}/{}{fee}", sym(c.currency0.as_deref()), sym(c.currency1.as_deref()))
}

/// One found position as the snapshot states it. Amounts and value come from this
/// cycle's reads; without them they are null and `value_note` says why. Pure.
fn found_json(f: &FoundPosition, spot: f64, eth_usd: f64) -> (Value, Option<f64>) {
    let c = &f.row;
    let tokens = (c.currency0.as_deref().and_then(|a| token_info(a).ok()), c.currency1.as_deref().and_then(|a| token_info(a).ok()));
    let (lo, hi) = (c.tick_lower.unwrap_or(0), c.tick_upper.unwrap_or(0));
    let mut holds = Value::Null;
    let mut value: Option<f64> = None;
    let mut value_note: Option<String> = None;
    let mut in_range = Value::Null;
    match (&f.read, &tokens) {
        (Some(LiveRead { liquidity, slot0: Some((sqrt, tick)) }), (Some(t0), Some(t1))) => {
            let (a0, a1) = concentrated_amounts_raw(*liquidity, *sqrt, lo, hi);
            let (h0, h1) = (a0 / 10f64.powi(t0.decimals), a1 / 10f64.powi(t1.decimals));
            holds = json!({
                "token0": { "symbol": t0.symbol, "amount": fx(h0, if is_usdc_token(t0) { 2 } else { 4 }) },
                "token1": { "symbol": t1.symbol, "amount": fx(h1, if is_usdc_token(t1) { 2 } else { 4 }) },
                "liquidity": liquidity.to_string(),
            });
            in_range = json!(lo <= *tick && *tick < hi);
            let (p0, p1) = (leg_usd_price(t0, spot, eth_usd), leg_usd_price(t1, spot, eth_usd));
            let priced = |t: &PositionTokenInfo, amount: f64, p: f64| amount == 0.0 || is_usdc_token(t) || (p.is_finite() && p > 0.0);
            if priced(t0, h0, p0) && priced(t1, h1, p1) {
                value = Some(h0 * p0 + h1 * p1);
            } else {
                value_note = Some("one of its tokens has no price this cycle, so no value is stated".to_string());
            }
        }
        (Some(LiveRead { liquidity, slot0: None }), _) => {
            holds = json!({ "liquidity": liquidity.to_string() });
            value_note = Some("its pool's price could not be read this cycle, so what it holds and its value are not stated".to_string());
        }
        _ => value_note = Some("it was not read this cycle, so what it holds and its value are not stated".to_string()),
    }
    let bounds_usd = match &tokens {
        (Some(t0), Some(t1)) if is_usdc_token(t0) && is_nock_token(t1) => {
            json!({ "low": fx(tick_to_display_price_dec(hi, t0.decimals, t1.decimals), 8), "high": fx(tick_to_display_price_dec(lo, t0.decimals, t1.decimals), 8) })
        }
        (Some(t0), Some(t1)) if is_nock_token(t0) && is_usdc_token(t1) => {
            let at = |t: i32| 1.0001f64.powi(t) * 10f64.powi(t0.decimals - t1.decimals);
            json!({ "low": fx(at(lo), 8), "high": fx(at(hi), 8) })
        }
        _ => Value::Null,
    };
    let v = json!({
        "nft_id": c.nft_id,
        "position_id": v4_position_key(&c.network, c.nft_id),
        "protocol": c.protocol,
        "pool_id": c.pool_id,
        "pool_label": candidate_pool_label(c),
        "tick_lower": c.tick_lower,
        "tick_upper": c.tick_upper,
        "bounds_usd": bounds_usd,
        "in_range": in_range,
        "holds": holds,
        "value_usd": value.map(|v| fx(v, 2)),
        "value_note": value_note,
        "mint_tx": c.mint_tx,
        "mint_block": c.mint_block,
        "minted_by": c.minted_by,
        "received_from": c.received_from.as_deref().filter(|f| *f != ZERO_ADDRESS),
        "why_not_automatic": f.why_not_automatic,
        "choices": f.choices,
    });
    (v, value)
}

/// A list of found positions with its count and total. The total is null when any
/// member's value is: never a partial sum. Pure.
fn found_list(found: &[&FoundPosition], spot: f64, eth_usd: f64) -> Value {
    let rows: Vec<(Value, Option<f64>)> = found.iter().map(|f| found_json(f, spot, eth_usd)).collect();
    let total: Option<f64> = rows.iter().map(|(_, v)| *v).sum();
    let unvalued = rows.iter().filter(|(_, v)| v.is_none()).count();
    json!({
        "count": rows.len(),
        "value_usd": if rows.is_empty() { Value::Null } else { json!(total.map(|t| fx(t, 2))) },
        "value_note": (unvalued > 0).then(|| format!("{unvalued} of {} could not be valued this cycle, so no total is stated", rows.len())),
        "positions": rows.into_iter().map(|(v, _)| v).collect::<Vec<_>>(),
    })
}

/// One database row as the snapshot states it: what it is tracked as, where it came
/// from, and whether this cycle counted it. Pure.
fn tracked_json(r: &TrackedReport, raw: &RawOnChainData) -> Value {
    let row = &r.row;
    let ladder = row.role.as_deref() == Some(LADDER_ROLE);
    let (state, note) = match &r.fate {
        TrackedFate::ShadowedByConfig { config_id } => (
            "shadowed_by_config",
            Some(format!("config.yaml also lists this position ({config_id}): the config entry is the one read. UNTRACK removes this row.")),
        ),
        TrackedFate::NotOwned { owner } => (
            "not_owned",
            Some(format!("NFT #{} is now owned by {}: it is no longer counted. UNTRACK removes it from this list.", row.nft_id, short_addr(owner))),
        ),
        TrackedFate::Quarantined { reason } => ("quarantined", Some(format!("not read and not counted: {reason}. UNTRACK removes it from this list."))),
        TrackedFate::Merged => {
            if raw.closed_positions.iter().any(|c| c.position_id == row.position_key) {
                ("closed", Some(format!("NFT #{} no longer exists (withdrawn and burned): it is in no total. UNTRACK removes it from this list.", row.nft_id)))
            } else {
                match raw.positions.iter().find(|p| p.position_id == row.position_key) {
                    Some(p) if p.liquidity == 0 => (
                        "empty",
                        Some(if ladder {
                            "it holds nothing (emptied; the NFT is still in the wallet). It is not a deployed rung.".to_string()
                        } else {
                            "it holds nothing (emptied; the NFT is still in the wallet).".to_string()
                        }),
                    ),
                    Some(_) => ("counted", None),
                    None => ("not_read", Some("it was not read this cycle".to_string())),
                }
            }
        }
    };
    json!({
        "position_id": row.position_key,
        "nft_id": row.nft_id,
        "config_id": tracked_config_id(row),
        "role": row.role,
        "rung": if ladder { json!(row.rung) } else { Value::Null },
        "rung_note": (ladder && row.rung.is_none()).then_some(RUNG_UNSET_NOTE_TRACKED),
        "source": row.source,
        "deposit_tx": row.deposit_tx,
        "mint_block": row.mint_block,
        "tracked_at": row.tracked_at.map(|t| t.to_rfc3339()),
        "tick_lower": row.tick_lower,
        "tick_upper": row.tick_upper,
        "state": state,
        "note": note,
        "choices": if ladder && state == "counted" && row.rung.is_none() {
            LADDER_RUNGS.iter().map(|r| format!("{LADDER_ROLE}:{r}")).chain(["untrack".to_string()]).collect::<Vec<_>>()
        } else {
            vec!["untrack".to_string()]
        },
    })
}

/// `derived_metrics.position_detection` (2.9.0): whether new positions were looked for
/// and how far back, the positions tracked from the chain, and every position found and
/// not counted with its value. `last_checked_at` and `checked_at_block` change every
/// cycle and are left out of the state hash. Pure.
pub(crate) fn position_detection_block(outcome: &DetectionOutcome, raw: &RawOnChainData) -> Value {
    let (spot, eth_usd) = (raw.spot_price_nock_usd, raw.live_eth_price_usd);
    let offers: Vec<&FoundPosition> = outcome.found.iter().filter(|f| !f.dismissed).collect();
    let dismissed: Vec<&FoundPosition> = outcome.found.iter().filter(|f| f.dismissed).collect();
    let tracked: Vec<Value> = raw.tracked_report.iter().map(|r| tracked_json(r, raw)).collect();
    json!({
        "status": outcome.status,
        "reason": outcome.reason,
        "last_checked_at": outcome.last_checked_at.map(|t| t.to_rfc3339()),
        "checked_at_block": outcome.run.block,
        "scan": outcome.scan,
        "onchain_count": {
            "held": outcome.run.held,
            "known": outcome.run.block.map(|_| outcome.run.known_owned),
        },
        "tracked": tracked,
        "found_not_counted": found_list(&offers, spot, eth_usd),
        "dismissed": found_list(&dismissed, spot, eth_usd),
        "ignored": outcome.ignored,
        "statement": POSITION_DETECTION_STATEMENT,
    })
}

// ------------------------------------------------------------------
// One-tap writes: track, untrack, dismiss, restore
// ------------------------------------------------------------------

/// A refusal of one of the writes: the HTTP status, the code and the words.
pub(crate) type TapRefusal = (StatusCode, &'static str, String);

/// `as` of POST /v1/positions/track: (role, rung). Pure.
pub(crate) fn parse_track_as(choice: &str) -> std::result::Result<(Option<&'static str>, Option<String>), TapRefusal> {
    if choice == "lp" {
        return Ok((None, None));
    }
    if let Some(rung) = choice.strip_prefix("ladder:") {
        if LADDER_RUNGS.contains(&rung) {
            return Ok((Some(LADDER_ROLE), Some(rung.to_string())));
        }
    }
    Err((
        StatusCode::BAD_REQUEST,
        "UNKNOWN_CHOICE",
        format!("`as` must be lp or ladder:<rung> with a rung of {}; got '{choice}'. Nothing changed.", LADDER_RUNGS.join(", ")),
    ))
}

fn tap_db_error(e: anyhow::Error) -> TapRefusal {
    tracing::error!("Position write failed: {e}");
    (StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", "The detection tables could not be read or written; nothing changed.".to_string())
}

/// TRACK: count a found position from the next cycle on, as a ladder rung or as LP. The
/// NFT's owner and liquidity are read again first (at `latest`); a position the wallet
/// no longer holds, or an empty one, is refused. It also changes how an already tracked
/// row is tracked (its rung, say). Database rows only: a config.yaml entry is the
/// operator's file.
pub(crate) async fn track_position(
    store: &PositionStore,
    http: &Client,
    rpc_url: Option<&str>,
    wallet: &str,
    config_positions: &[PositionConfig],
    nft_id: u64,
    choice: &str,
) -> std::result::Result<Value, TapRefusal> {
    let (role, rung) = parse_track_as(choice)?;
    if config_v4_ids(config_positions).contains(&nft_id) {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "CONFIG_POSITION", format!("config.yaml already lists NFT #{nft_id}: its entry is the one read. Nothing changed.")));
    }
    let candidates = store.candidates(wallet).await.map_err(tap_db_error)?;
    let Some(c) = candidates.iter().find(|c| c.protocol == TRACKED_PROTOCOL_V4 && c.nft_id == nft_id) else {
        return Err((StatusCode::NOT_FOUND, "NOT_FOUND", format!("NockNock has not found NFT #{nft_id} arriving in the tracked wallet. Nothing changed.")));
    };
    if c.verdict != "eligible" {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "NOT_ELIGIBLE", format!("NFT #{nft_id} is not in a pool NockNock tracks ({}). Nothing changed.", c.verdict)));
    }
    if !track_choices(c).iter().any(|x| x == choice) {
        let why = if c.pool_id.is_none() { "its pool has not been identified yet" } else { "it is not a USDC/NOCK concentrated range, so it cannot be a ladder rung" };
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "CHOICE_NOT_OFFERED", format!("NFT #{nft_id} cannot be tracked as {choice}: {why}. Nothing changed.")));
    }
    let Some(url) = rpc_url else {
        return Err((StatusCode::SERVICE_UNAVAILABLE, "NO_RPC", "No Alchemy RPC key is configured, so the position could not be read again. Nothing changed.".to_string()));
    };
    let mut reader = Reader { http, url, wallet: lc(wallet), tag: "latest".to_string(), known_pools: Vec::new(), receipts_left: 0, liquidity_read: HashMap::new() };
    let unread = || (StatusCode::SERVICE_UNAVAILABLE, "CHAIN_UNAVAILABLE", format!("NFT #{nft_id} could not be read on-chain just now. Nothing changed."));
    match reader.owner(nft_id).await.ok_or_else(unread)? {
        Some(owner) if owner == reader.wallet => {}
        Some(owner) => return Err((StatusCode::UNPROCESSABLE_ENTITY, "NOT_OWNED", format!("NFT #{nft_id} is owned by {}, not by the tracked wallet. Nothing changed.", short_addr(&owner)))),
        None => return Err((StatusCode::UNPROCESSABLE_ENTITY, "NOT_OWNED", format!("NFT #{nft_id} no longer exists. Nothing changed."))),
    }
    if reader.liquidity(nft_id).await.ok_or_else(unread)? == 0 {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "EMPTY", format!("NFT #{nft_id} holds no liquidity: there is nothing to track. Nothing changed.")));
    }
    let row = tracked_row_for(c, role, rung.as_deref(), TRACKED_SOURCE_CONFIRMED)
        .ok_or_else(|| (StatusCode::UNPROCESSABLE_ENTITY, "CHOICE_NOT_OFFERED", format!("NFT #{nft_id} lacks a pool id or ticks, so it cannot be read as a position. Nothing changed.")))?;
    validate_tracked_row(&row).map_err(|why| (StatusCode::UNPROCESSABLE_ENTITY, "CHOICE_NOT_OFFERED", format!("NFT #{nft_id} cannot be tracked as {choice}: {why}. Nothing changed.")))?;
    store.upsert_tracked(&row).await.map_err(tap_db_error)?;
    store.set_choice(wallet, TRACKED_PROTOCOL_V4, TRACKED_NETWORK_BASE, nft_id, None).await.map_err(tap_db_error)?;
    Ok(json!({
        "status": "tracked",
        "position_id": row.position_key,
        "nft_id": nft_id,
        "role": row.role,
        "rung": row.rung,
        "deposit_tx": row.deposit_tx,
        "note": match role {
            Some(_) => "Tracked as a ladder: measured, never steered, and counted from the next cycle.",
            None => "Tracked as LP: counted from the next cycle and classified by its pair. It is never a rebuild target.",
        },
    }))
}

/// UNTRACK: stop counting a database-tracked position. The NFT stays in the wallet; it is
/// then listed as set aside, not counted, and can be tracked again. Pure of chain reads.
pub(crate) async fn untrack_position(store: &PositionStore, wallet: &str, config_positions: &[PositionConfig], position_id: &str) -> std::result::Result<Value, TapRefusal> {
    let rows = store.tracked(wallet).await.map_err(tap_db_error)?;
    let Some(row) = rows.iter().find(|t| t.position_key == position_id) else {
        // A config.yaml entry is the operator's file: it is never removed from here.
        if config_positions.iter().any(|c| position_key(c) == position_id) {
            return Err((StatusCode::UNPROCESSABLE_ENTITY, "CONFIG_POSITION", format!("{position_id} is a config.yaml entry: it is removed by editing that file, not here. Nothing changed.")));
        }
        return Err((StatusCode::NOT_FOUND, "NOT_TRACKED", format!("{position_id} is not a position tracked from the chain. Nothing changed.")));
    };
    store.delete_tracked(wallet, position_id).await.map_err(tap_db_error)?;
    // Set aside, so detection does not track it again by itself.
    store.set_choice(wallet, &row.protocol, &row.network, row.nft_id, Some(CHOICE_DISMISSED)).await.map_err(tap_db_error)?;
    Ok(json!({
        "status": "untracked",
        "position_id": position_id,
        "nft_id": row.nft_id,
        "note": format!("NFT #{} is still in your wallet and is no longer counted. It is listed as set aside and can be tracked again.", row.nft_id),
    }))
}

/// NOT NOW / RESTORE: set a found position aside, or put it back on offer. Neither counts
/// it or stops counting it: a tracked position is refused (untrack it instead).
pub(crate) async fn set_found_choice(store: &PositionStore, wallet: &str, nft_id: u64, dismiss: bool) -> std::result::Result<Value, TapRefusal> {
    let candidates = store.candidates(wallet).await.map_err(tap_db_error)?;
    let Some(c) = candidates.iter().find(|c| c.protocol == TRACKED_PROTOCOL_V4 && c.nft_id == nft_id) else {
        return Err((StatusCode::NOT_FOUND, "NOT_FOUND", format!("NockNock has not found NFT #{nft_id} arriving in the tracked wallet. Nothing changed.")));
    };
    if store.tracked(wallet).await.map_err(tap_db_error)?.iter().any(|t| t.nft_id == nft_id && t.protocol == c.protocol) {
        return Err((StatusCode::CONFLICT, "TRACKED", format!("NFT #{nft_id} is tracked and counted. UNTRACK stops counting it. Nothing changed.")));
    }
    if c.verdict != "eligible" {
        return Err((StatusCode::CONFLICT, "NOT_AN_OFFER", format!("NFT #{nft_id} was never offered ({}). Nothing changed.", c.verdict)));
    }
    let changed = (c.choice.as_deref() == Some(CHOICE_DISMISSED)) != dismiss;
    if changed {
        store.set_choice(wallet, &c.protocol, &c.network, nft_id, dismiss.then_some(CHOICE_DISMISSED)).await.map_err(tap_db_error)?;
    }
    Ok(json!({
        "status": if dismiss { "dismissed" } else { "offered" },
        "nft_id": nft_id,
        "changed": changed,
        "note": if dismiss {
            "Set aside: it is not counted and stays listed, and can be restored."
        } else {
            "Back on offer: it is still not counted until you track it."
        },
    }))
}

// ------------------------------------------------------------------
// REST
// ------------------------------------------------------------------

#[derive(Deserialize)]
pub(crate) struct TrackRequest {
    nft_id: u64,
    #[serde(rename = "as")]
    track_as: String,
}

#[derive(Deserialize)]
pub(crate) struct NftRequest {
    nft_id: u64,
}

#[derive(Deserialize)]
pub(crate) struct UntrackRequest {
    position_id: String,
}

fn tap_response(result: std::result::Result<Value, TapRefusal>) -> Response {
    match result {
        Ok(body) => Json(body).into_response(),
        Err((code, error, detail)) => (code, Json(json!({ "error": error, "detail": detail }))).into_response(),
    }
}

fn state_rpc_url(state: &AppState) -> Option<String> {
    let key = state.config.alchemy_api_keys.first().cloned().unwrap_or_default();
    (!key.is_empty() && key != "YOUR_ALCHEMY_API_KEY").then(|| format!("https://base-mainnet.g.alchemy.com/v2/{key}"))
}

/// A write that changes what is counted is followed by a cycle, so the totals and the
/// card agree with it at once. The cycle runs in its own task (#134); its failure does
/// not undo the write, and is reported beside it.
async fn with_cycle(state: Arc<AppState>, result: std::result::Result<Value, TapRefusal>) -> Response {
    let mut body = match result {
        Ok(b) => b,
        refused => return tap_response(refused),
    };
    let cycle = tokio::spawn(async move { run_requested_cycle(&state).await });
    body["cycle"] = match cycle.await {
        Ok(Ok(())) => json!("ok"),
        Ok(Err(e)) => json!(format!("the write is stored; the cycle after it failed: {e}")),
        Err(_) => json!("the write is stored; the cycle after it did not finish"),
    };
    Json(body).into_response()
}

fn no_wallet() -> Response {
    tap_response(Err((StatusCode::SERVICE_UNAVAILABLE, "NO_TRACKED_WALLET", "No tracked wallet configured. Nothing changed.".to_string())))
}

/// POST /v1/positions/track {nft_id, as}: `as` is `ladder:<rung>` or `lp`.
pub(crate) async fn track_position_handler(Extension(state): Extension<Arc<AppState>>, Json(req): Json<TrackRequest>) -> Response {
    let Some(wallet) = state.config.tracked_wallets.first().cloned() else { return no_wallet() };
    let url = state_rpc_url(&state);
    let result = track_position(&PositionStore::of(&state), &state.http_client, url.as_deref(), &wallet, &state.config.positions, req.nft_id, &req.track_as).await;
    with_cycle(state, result).await
}

/// POST /v1/positions/untrack {position_id}: database rows only.
pub(crate) async fn untrack_position_handler(Extension(state): Extension<Arc<AppState>>, Json(req): Json<UntrackRequest>) -> Response {
    let Some(wallet) = state.config.tracked_wallets.first().cloned() else { return no_wallet() };
    let result = untrack_position(&PositionStore::of(&state), &wallet, &state.config.positions, req.position_id.trim()).await;
    with_cycle(state, result).await
}

/// POST /v1/positions/dismiss {nft_id}: set a found position aside (NOT NOW).
pub(crate) async fn dismiss_position_handler(Extension(state): Extension<Arc<AppState>>, Json(req): Json<NftRequest>) -> Response {
    let Some(wallet) = state.config.tracked_wallets.first().cloned() else { return no_wallet() };
    tap_response(set_found_choice(&PositionStore::of(&state), &wallet, req.nft_id, true).await)
}

/// POST /v1/positions/restore {nft_id}: put a set-aside position back on offer.
pub(crate) async fn restore_position_handler(Extension(state): Extension<Arc<AppState>>, Json(req): Json<NftRequest>) -> Response {
    let Some(wallet) = state.config.tracked_wallets.first().cloned() else { return no_wallet() };
    tap_response(set_found_choice(&PositionStore::of(&state), &wallet, req.nft_id, false).await)
}

/// The body of GET /v1/positions/found: the latest snapshot's detection block as it is,
/// and every database row with the config.yaml entry that would track the same position
/// (for a rollback to a release without database-tracked positions). Pure.
pub(crate) fn found_positions_body(detection: Option<&Value>, rows: &[TrackedRow]) -> Value {
    json!({
        "detection": detection,
        "tracked_rows": rows.iter().map(|r| json!({
            "position_id": r.position_key,
            "nft_id": r.nft_id,
            "role": r.role,
            "rung": r.rung,
            "source": r.source,
            "deposit_tx": r.deposit_tx,
            "tracked_at": r.tracked_at.map(|t| t.to_rfc3339()),
            "config_entry": tracked_row_config_entry(r),
        })).collect::<Vec<_>>(),
        "rollback_note": "A release older than v0.9.0 does not read these rows. Before running one, add each config_entry under positions: in config.yaml, or its total leaves these positions out without saying so.",
    })
}

/// GET /v1/positions/found.
pub(crate) async fn found_positions_handler(Extension(state): Extension<Arc<AppState>>) -> Response {
    let detection = state.latest_snapshot.lock().await.as_ref().and_then(|s| s.data.derived_metrics.position_detection.clone());
    let wallet = state.config.tracked_wallets.first().cloned().unwrap_or_default();
    match PositionStore::of(&state).tracked(&wallet).await {
        Ok(rows) => Json(found_positions_body(detection.as_ref(), &rows)).into_response(),
        Err(e) => {
            tracing::error!("Tracked positions unreadable: {e}");
            (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(json!({ "error": "TRACKED_POSITIONS_UNAVAILABLE", "detail": "The tracked positions table could not be read." })),
            )
                .into_response()
        }
    }
}
