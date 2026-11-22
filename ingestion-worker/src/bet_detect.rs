//! WETH/NOCK bets found through the gauge (issue #181).
//!
//! A bet is deposited in the Aero app with its default `rewards=aero`: the position NFT is
//! minted to the Aero router and moved to the pool's gauge in the same transaction. It
//! never reaches the wallet, so a Transfer-to-wallet scan (position_detect.rs) finds
//! nothing. Bets are found through the gauge instead, every cycle, before the position
//! reads:
//!
//! 1. The bet pool (`operator_profile.nock_bets.pool`) is read at one block: its tokens,
//!    tick spacing, fee, gauge, position manager, factory, price and in-range liquidity,
//!    and the factory's own `getPool` for those tokens and that spacing. It is read even
//!    with no position in it: the bet card sizes its range on it.
//! 2. `gauge.stakedValues(wallet)`: the NFT ids the wallet itself has staked there now.
//!    One exact read, with no block range to scan.
//! 3. For each id not yet decided: `positions(id)` on the position manager, then the
//!    block of its stake (`gauge.depositBlock(id)`), the gauge's `Deposit` log for this
//!    wallet and id in that one block, and that transaction's receipt.
//!
//! The proof (`bet_mint_proof`) is strict and is never loosened: the transaction was sent
//! by the wallet itself; in it the position manager minted the NFT to the Aero router and
//! the router moved it to the bet pool's gauge; the gauge recorded the deposit for this
//! wallet; and the pool's own Mint names the position manager and the NFT's ticks. Only
//! then is the NFT tracked by itself, as `role: bet`. Anything else the wallet has staked
//! in that gauge (an NFT that reached it by transfer, a stake made after an unstaked mint,
//! a mint sent by another address) is offered with one tap and is not counted until then.
//!
//! The reads are capped per cycle (`POSITION_REFRESH_MAX_PER_CYCLE` NFTs) inside the same
//! soft time budget as position detection; the rest are read next cycle and the status
//! says so. It never fails the cycle, and it signs and sends nothing.
use crate::*;
use serde_json::Value;
use std::collections::HashSet;

/// The Aero Slipstream pool factory on Base (ONCHAIN_INTEGRATION.md §8): `pool.factory()`
/// and the position manager's `factory()` both return it.
pub(crate) const AERO_CL_FACTORY_BASE: &str = "0x9cb220581b91b0c83223012fe69688b92ad07a76";
/// The Aero router the app deposits through (`execute`, ONCHAIN_INTEGRATION.md §6, §16):
/// a staked deposit mints the NFT to it and it stakes the NFT in the gauge.
pub(crate) const AERO_ROUTER_BASE: &str = "0x0fef9e88a7b76bcdbf451780c6774fe32965ee98";
pub(crate) const CL_POOL_FACTORY: &str = "0xc45a0155"; // factory()
pub(crate) const CL_FACTORY_GET_POOL: &str = "0x28af8d0b"; // getPool(address,address,int24)
pub(crate) const CL_GAUGE_POOL: &str = "0x16f0115b"; // pool()
pub(crate) const CL_GAUGE_STAKED_VALUES: &str = "0x4b937763"; // stakedValues(address)
pub(crate) const CL_GAUGE_DEPOSIT_BLOCK: &str = "0x32590654"; // depositBlock(uint256)
/// The gauge's `Deposit(address indexed from, address indexed to, uint256 indexed tokenId,
/// uint128 liquidity)`, read from the receipt of NFT #2914490's deposit (2026-11-14):
/// from = the router, to = the wallet.
pub(crate) const CL_GAUGE_DEPOSIT_TOPIC: &str = "0xc9817c7b5bf7b2417ea338292b8681857834d37427fbb1899f98e6897f3fe426";
/// The pool's `Mint(address sender, address indexed owner, int24 indexed tickLower, int24
/// indexed tickUpper, uint128 amount, uint256 amount0, uint256 amount1)`, the Uniswap v3
/// event, emitted by the Slipstream pool with the position manager as owner.
pub(crate) const CL_POOL_MINT_TOPIC: &str = "0x7a53080ba414158be7ec69b987b5fb7d07dee101fe85488f0853ae16239d0bde";

/// The one-tap choices for a bet found and not counted.
pub(crate) const BET_TRACK_CHOICE: &str = "bet";

pub(crate) const BET_DETECTION_STATEMENT: &str = "NockNock reads the bet pool's gauge for the NFTs your wallet staked there. It tracks one by itself only when your wallet sent the deposit, the NFT was minted to the Aero router and moved to the gauge in that same transaction, and the pool is the bet pool; anything else waits for one tap and is not counted until then. It never signs or sends.";

fn lc(s: &str) -> String {
    s.trim().to_lowercase()
}

fn addr_word(a: &str) -> String {
    format!("{:0>64}", lc(a).trim_start_matches("0x"))
}

fn word_addr(w: &str) -> Option<String> {
    let w = w.trim_start_matches("0x");
    if w.len() != 64 || !w.bytes().all(|b| b.is_ascii_hexdigit()) || !w[..24].bytes().all(|b| b == b'0') {
        return None;
    }
    Some(format!("0x{}", w[24..].to_lowercase()))
}

fn word_u128(w: &str) -> Option<u128> {
    let w = w.trim_start_matches("0x");
    if w.len() != 64 || !w[..32].bytes().all(|b| b == b'0') {
        return None;
    }
    u128::from_str_radix(&w[32..], 16).ok()
}

fn words(hex: &str) -> Vec<String> {
    let h = hex.trim_start_matches("0x");
    (0..h.len() / 64).map(|i| h[i * 64..(i + 1) * 64].to_string()).collect()
}

// ------------------------------------------------------------------
// The bet pool (pure decoding)
// ------------------------------------------------------------------

/// The bet pool as one cycle read it, at one block.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct BetPoolRead {
    /// Lowercase.
    pub(crate) pool: String,
    /// The address as the operator wrote it, for the deposit link.
    pub(crate) configured: String,
    pub(crate) block: i64,
    pub(crate) token0: String,
    pub(crate) token1: String,
    pub(crate) tick_spacing: i32,
    /// The pool's current swap fee in pips (1e-6): 9700 is 0.97%. Aero's fee is dynamic.
    pub(crate) fee_pips: u32,
    pub(crate) gauge: String,
    /// slot0's sqrtPriceX96, 64 hex characters, and tick.
    pub(crate) sqrt_price_x96: String,
    pub(crate) tick: i32,
    /// In-range liquidity.
    pub(crate) liquidity: u128,
}

/// The pool's raw answers, hex without 0x. None = not read.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct BetPoolAnswers {
    pub(crate) token0: Option<String>,
    pub(crate) token1: Option<String>,
    pub(crate) tick_spacing: Option<String>,
    pub(crate) fee: Option<String>,
    pub(crate) gauge: Option<String>,
    pub(crate) nft: Option<String>,
    pub(crate) factory: Option<String>,
    pub(crate) slot0: Option<String>,
    pub(crate) liquidity: Option<String>,
    /// factory.getPool(token0, token1, tickSpacing).
    pub(crate) factory_pool: Option<String>,
    /// gauge.pool().
    pub(crate) gauge_pool: Option<String>,
}

/// The bet pool from its answers, or why it is not one NockNock bets in. Every check is
/// stated; none is skipped. Pure.
pub(crate) fn bet_pool_from_answers(configured: &str, block: i64, a: &BetPoolAnswers) -> std::result::Result<BetPoolRead, String> {
    let pool = lc(configured);
    let need = |v: &Option<String>, what: &str| v.clone().ok_or_else(|| format!("the pool's {what} could not be read this cycle"));
    let addr = |v: &Option<String>, what: &str| -> std::result::Result<String, String> {
        word_addr(&need(v, what)?).ok_or_else(|| format!("the pool's {what} answer is not an address"))
    };
    let token0 = addr(&a.token0, "token0()")?;
    let token1 = addr(&a.token1, "token1()")?;
    let syms = (token_info(&token0).map(|t| t.symbol), token_info(&token1).map(|t| t.symbol));
    let weth_nock = matches!((syms.0.as_deref(), syms.1.as_deref()), (Ok("WETH"), Ok("NOCK")) | (Ok("NOCK"), Ok("WETH")));
    if !weth_nock {
        return Err(format!("it is not a WETH/NOCK pool (token0 {token0}, token1 {token1})"));
    }
    let nft = addr(&a.nft, "nft()")?;
    if nft != AERO_CL_POSITION_MANAGER_BASE {
        return Err(format!("its position manager is {nft}, not the Aero CL position manager {AERO_CL_POSITION_MANAGER_BASE}"));
    }
    let factory = addr(&a.factory, "factory()")?;
    if factory != AERO_CL_FACTORY_BASE {
        return Err(format!("its factory is {factory}, not the Aero CL factory {AERO_CL_FACTORY_BASE}"));
    }
    let spacing = parse_hex_word_i32(&need(&a.tick_spacing, "tickSpacing()")?).map_err(|_| "the pool's tickSpacing() answer is not a number".to_string())?;
    if spacing <= 0 {
        return Err(format!("its tick spacing {spacing} is not positive"));
    }
    let factory_pool = addr(&a.factory_pool, "factory getPool()")?;
    if factory_pool != pool {
        return Err(format!("the factory's getPool for these tokens and spacing {spacing} is {factory_pool}, not this address"));
    }
    let gauge = addr(&a.gauge, "gauge()")?;
    if gauge == NATIVE_ETH {
        return Err("it has no gauge, so a bet could not be staked for AERO".to_string());
    }
    let gauge_pool = word_addr(&need(&a.gauge_pool, "gauge's pool()")?).ok_or("the gauge's pool() answer is not an address")?;
    if gauge_pool != pool {
        return Err(format!("its gauge {gauge} names another pool ({gauge_pool})"));
    }
    let fee_pips = word_u128(&need(&a.fee, "fee()")?).and_then(|f| u32::try_from(f).ok()).ok_or("the pool's fee() answer is not a fee")?;
    let (sqrt, tick) = decode_slipstream_slot0(&need(&a.slot0, "slot0()")?).map_err(|e| format!("the pool's slot0() answer could not be read ({e})"))?;
    let liquidity = word_u128(&need(&a.liquidity, "liquidity()")?).ok_or("the pool's liquidity() answer is not a number")?;
    if liquidity == 0 {
        return Err("it has no in-range liquidity, so its price is frozen".to_string());
    }
    Ok(BetPoolRead {
        pool,
        configured: configured.trim().to_string(),
        block,
        token0,
        token1,
        tick_spacing: spacing,
        fee_pips,
        gauge,
        sqrt_price_x96: sqrt.to_hex_be(),
        tick,
        liquidity,
    })
}

/// `stakedValues(address)`: an ABI uint256[] (offset, length, items). Pure.
pub(crate) fn decode_staked_values(hex: &str) -> std::result::Result<Vec<u64>, String> {
    let w = words(hex);
    let bad = || "the gauge's stakedValues() answer is not a list of token ids".to_string();
    if w.len() < 2 || word_u128(&w[0]) != Some(32) {
        return Err(bad());
    }
    let n = word_u128(&w[1]).ok_or_else(bad)? as usize;
    if w.len() != 2 + n {
        return Err(bad());
    }
    w[2..].iter().map(|x| word_u128(x).and_then(|v| u64::try_from(v).ok()).ok_or_else(bad)).collect()
}

// ------------------------------------------------------------------
// The proof (pure)
// ------------------------------------------------------------------

/// What a bet's deposit transaction proves.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct BetMint {
    pub(crate) tx: String,
    pub(crate) block: i64,
    /// The liquidity the gauge recorded and the pool minted, and the raw amounts of each
    /// token the pool took.
    pub(crate) liquidity: u128,
    pub(crate) amount0: u128,
    pub(crate) amount1: u128,
    pub(crate) tick_lower: i32,
    pub(crate) tick_upper: i32,
}

/// Read the proof from a receipt (the `result` of eth_getTransactionReceipt). `Err` is the
/// reason the NFT is not tracked by itself, in words. Every condition must hold; none is
/// ever loosened (#181):
/// - the transaction succeeded and was sent by the wallet itself;
/// - the position manager minted this NFT to the Aero router in it;
/// - the router moved it to the bet pool's gauge in it;
/// - the gauge recorded the deposit for the wallet;
/// - the bet pool minted exactly one range to the position manager with the liquidity the
///   gauge recorded. Pure.
pub(crate) fn bet_mint_proof(receipt: &Value, wallet: &str, nft_id: u64, pool: &str, gauge: &str) -> std::result::Result<BetMint, String> {
    let status = receipt.get("status").and_then(|v| v.as_str()).map(lc).unwrap_or_default();
    if status != "0x1" {
        return Err(format!("its deposit transaction failed (status {status})"));
    }
    let tx = receipt.get("transactionHash").and_then(|v| v.as_str()).map(lc).ok_or("the receipt has no transaction hash")?;
    let block = receipt
        .get("blockNumber")
        .and_then(|v| v.as_str())
        .and_then(|h| i64::from_str_radix(h.trim_start_matches("0x"), 16).ok())
        .ok_or("the receipt has no block number")?;
    let sender = receipt.get("from").and_then(|v| v.as_str()).map(lc).ok_or("the receipt has no sender")?;
    if sender != lc(wallet) {
        return Err(format!("its deposit was sent by {}, not by your wallet", short_addr(&sender)));
    }
    let logs: Vec<Value> = receipt.get("logs").and_then(|l| l.as_array()).cloned().unwrap_or_default();
    let topics = |l: &Value| -> Vec<String> { l.get("topics").and_then(|t| t.as_array()).map(|a| a.iter().filter_map(|x| x.as_str()).map(lc).collect()).unwrap_or_default() };
    let address = |l: &Value| l.get("address").and_then(|a| a.as_str()).map(lc).unwrap_or_default();
    let data = |l: &Value| l.get("data").and_then(|d| d.as_str()).unwrap_or("").to_string();
    let id_word = format!("0x{nft_id:064x}");
    let at = |a: &str| format!("0x{}", addr_word(a));
    let transfer = |from: &str, to: &str| {
        logs.iter().any(|l| {
            let t = topics(l);
            address(l) == AERO_CL_POSITION_MANAGER_BASE && t.len() == 4 && t[0] == TRANSFER_EVENT_TOPIC && t[1] == at(from) && t[2] == at(to) && t[3] == id_word
        })
    };
    if !transfer(NATIVE_ETH, AERO_ROUTER_BASE) {
        return Err("it was not minted to the Aero router in this transaction (it was minted elsewhere, or reached the gauge another way)".to_string());
    }
    if !transfer(AERO_ROUTER_BASE, gauge) {
        return Err("the Aero router did not move it to the bet pool's gauge in this transaction".to_string());
    }
    let deposits: Vec<&Value> = logs
        .iter()
        .filter(|l| {
            let t = topics(l);
            address(l) == lc(gauge) && t.len() == 4 && t[0] == CL_GAUGE_DEPOSIT_TOPIC && t[2] == at(wallet) && t[3] == id_word
        })
        .collect();
    let [deposit] = deposits.as_slice() else {
        return Err("the gauge did not record exactly one deposit of it for your wallet in this transaction".to_string());
    };
    let liquidity = words(&data(deposit)).first().and_then(|w| word_u128(w)).ok_or("the gauge's deposit record has no liquidity")?;
    let mints: Vec<(i32, i32, u128, u128, u128)> = logs
        .iter()
        .filter_map(|l| {
            let t = topics(l);
            if address(l) != lc(pool) || t.len() != 4 || t[0] != CL_POOL_MINT_TOPIC || t[1] != at(AERO_CL_POSITION_MANAGER_BASE) {
                return None;
            }
            let w = words(&data(l));
            if w.len() != 4 {
                return None;
            }
            let lo = parse_hex_word_i32(&t[2]).ok()?;
            let hi = parse_hex_word_i32(&t[3]).ok()?;
            Some((lo, hi, word_u128(&w[1])?, word_u128(&w[2])?, word_u128(&w[3])?))
        })
        .filter(|m| m.2 == liquidity)
        .collect();
    let [(tick_lower, tick_upper, _, amount0, amount1)] = mints.as_slice() else {
        return Err("the bet pool did not mint exactly one range of that liquidity to the position manager in this transaction".to_string());
    };
    Ok(BetMint { tx, block, liquidity, amount0: *amount0, amount1: *amount1, tick_lower: *tick_lower, tick_upper: *tick_upper })
}

// ------------------------------------------------------------------
// One cycle
// ------------------------------------------------------------------

/// A bet staked by the wallet in the bet gauge that is not counted: offered, or set aside.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FoundBet {
    pub(crate) row: CandidateRow,
    pub(crate) dismissed: bool,
    pub(crate) why_not_automatic: Vec<String>,
    pub(crate) choices: Vec<String>,
    /// Its liquidity read this cycle; None when it was not read this cycle.
    pub(crate) liquidity: Option<u128>,
}

/// What the cycle hands to the bet card.
#[derive(Debug, Clone, Default, PartialEq)]
pub(crate) struct BetDetection {
    /// ok | partial | unavailable | not_run
    pub(crate) status: String,
    pub(crate) reason: Option<String>,
    /// The block every read named.
    pub(crate) block: Option<i64>,
    pub(crate) pool: Option<BetPoolRead>,
    /// Why the pool is not one NockNock bets in, or could not be read.
    pub(crate) pool_reason: Option<String>,
    /// The NFT ids the wallet has staked in the bet gauge, as read; None when not read.
    pub(crate) staked: Option<Vec<u64>>,
    pub(crate) newly_tracked: Vec<String>,
    pub(crate) found: Vec<FoundBet>,
    /// NFTs left for a later cycle by the cap or the time budget.
    pub(crate) deferred: usize,
}

impl BetDetection {
    pub(crate) fn not_run(reason: &str) -> BetDetection {
        BetDetection { status: "not_run".to_string(), reason: Some(reason.to_string()), ..Default::default() }
    }
}

pub(crate) struct BetDetectionInputs<'a> {
    pub(crate) store: &'a PositionStore,
    pub(crate) http: &'a Client,
    pub(crate) rpc_url: Option<&'a str>,
    pub(crate) wallet: Option<&'a str>,
    /// `operator_profile.nock_bets.pool` when it is a well-formed address.
    pub(crate) pool: Option<&'a str>,
    pub(crate) config_positions: &'a [PositionConfig],
    /// Tests only: reads allowed before the budget is spent. None in every cycle.
    pub(crate) read_budget: Option<usize>,
}

struct BetReader<'a> {
    http: &'a Client,
    url: &'a str,
    tag: String,
    reads: usize,
}

impl BetReader<'_> {
    async fn call(&mut self, to: &str, data: &str) -> std::result::Result<Option<String>, String> {
        self.reads += 1;
        let payload = json!({ "jsonrpc": "2.0", "method": "eth_call", "id": 181, "params": [{ "to": to, "data": data }, self.tag] });
        match eth_call_outcome(self.http, self.url, &payload).await {
            Ok(CallOutcome::Returned(hex)) => Ok(Some(hex)),
            Ok(CallOutcome::Reverted(_)) => Ok(None),
            Err(_) => Err("the RPC did not answer".to_string()),
        }
    }

    async fn rpc(&mut self, payload: Value) -> std::result::Result<Value, String> {
        self.reads += 1;
        match rpc_answer(self.http, self.url, &payload).await {
            RpcAnswer::Result(v) => Ok(v.get("result").cloned().unwrap_or(Value::Null)),
            RpcAnswer::Refused(why) | RpcAnswer::NoAnswer(why) => Err(why),
        }
    }
}

async fn read_bet_pool(r: &mut BetReader<'_>, pool: &str, block: i64) -> std::result::Result<BetPoolRead, String> {
    let p = lc(pool);
    let mut a = BetPoolAnswers::default();
    let call = |v: std::result::Result<Option<String>, String>| -> std::result::Result<Option<String>, String> { v };
    a.token0 = call(r.call(&p, CL_POOL_TOKEN0).await)?;
    a.token1 = call(r.call(&p, CL_POOL_TOKEN1).await)?;
    a.tick_spacing = call(r.call(&p, CL_POOL_TICK_SPACING).await)?;
    a.fee = call(r.call(&p, CL_POOL_FEE).await)?;
    a.gauge = call(r.call(&p, CL_POOL_GAUGE).await)?;
    a.nft = call(r.call(&p, CL_POOL_NFT).await)?;
    a.factory = call(r.call(&p, CL_POOL_FACTORY).await)?;
    a.slot0 = call(r.call(&p, V3_SLOT0).await)?;
    a.liquidity = call(r.call(&p, V3_LIQUIDITY).await)?;
    if let (Some(t0), Some(t1), Some(ts)) = (a.token0.as_deref().and_then(word_addr), a.token1.as_deref().and_then(word_addr), a.tick_spacing.clone()) {
        let data = format!("{CL_FACTORY_GET_POOL}{}{}{}", addr_word(&t0), addr_word(&t1), ts.trim_start_matches("0x"));
        a.factory_pool = call(r.call(AERO_CL_FACTORY_BASE, &data).await)?;
    }
    if let Some(g) = a.gauge.as_deref().and_then(word_addr).filter(|g| g != NATIVE_ETH) {
        a.gauge_pool = call(r.call(&g, CL_GAUGE_POOL).await)?;
    }
    bet_pool_from_answers(pool, block, &a)
}

/// The deposit transaction of one staked NFT: the gauge's Deposit log for this wallet and
/// id, in the block the gauge says it was staked.
async fn deposit_receipt(r: &mut BetReader<'_>, gauge: &str, wallet: &str, nft_id: u64) -> std::result::Result<Value, String> {
    let id_word = format!("{nft_id:064x}");
    let at = r.call(gauge, &format!("{CL_GAUGE_DEPOSIT_BLOCK}{id_word}")).await?.ok_or("the gauge's depositBlock() reverted")?;
    let block = word_u128(&at).and_then(|b| i64::try_from(b).ok()).filter(|b| *b > 0).ok_or("the gauge records no deposit block for it")?;
    let logs = r
        .rpc(json!({
            "jsonrpc": "2.0", "method": "eth_getLogs", "id": 182,
            "params": [{
                "address": lc(gauge),
                "fromBlock": format!("0x{block:x}"),
                "toBlock": format!("0x{block:x}"),
                "topics": [CL_GAUGE_DEPOSIT_TOPIC, Value::Null, format!("0x{}", addr_word(wallet)), format!("0x{id_word}")],
            }],
        }))
        .await?;
    let txs: Vec<String> = logs.as_array().map(|a| a.iter().filter_map(|l| l.get("transactionHash").and_then(|h| h.as_str()).map(lc)).collect()).unwrap_or_default();
    let [tx] = txs.as_slice() else {
        return Err(format!("the gauge's deposit of it for your wallet was not found in block {block}"));
    };
    let receipt = r.rpc(json!({ "jsonrpc": "2.0", "method": "eth_getTransactionReceipt", "id": 183, "params": [tx] })).await?;
    if receipt.is_null() {
        return Err("its deposit transaction's receipt could not be read".to_string());
    }
    Ok(receipt)
}

/// The tracked row of a proven bet. Pure.
pub(crate) fn bet_tracked_row(wallet: &str, pool: &BetPoolRead, nft_id: u64, mint: Option<&BetMint>, c: &CandidateRow, source: &str) -> TrackedRow {
    TrackedRow {
        wallet: lc(wallet),
        position_key: aero_cl_position_key(TRACKED_NETWORK_BASE, nft_id),
        protocol: AERO_CL_PROTOCOL.to_string(),
        network: TRACKED_NETWORK_BASE.to_string(),
        nft_id,
        pool_id: pool.pool.clone(),
        currency0: pool.token0.clone(),
        currency1: pool.token1.clone(),
        tick_lower: c.tick_lower.unwrap_or_default(),
        tick_upper: c.tick_upper.unwrap_or_default(),
        tick_spacing: Some(pool.tick_spacing),
        role: Some(BET_ROLE.to_string()),
        rung: None,
        deposit_tx: mint.map(|m| m.tx.clone()).or_else(|| c.mint_tx.clone()),
        mint_block: mint.map(|m| m.block).or(c.mint_block),
        source: source.to_string(),
        tracked_at: None,
        deposit_liquidity: mint.map(|m| m.liquidity),
        max_liquidity_seen: None,
    }
}

/// One cycle's bet detection. Never fails the cycle.
pub(crate) async fn detect_bets(inp: &BetDetectionInputs<'_>) -> BetDetection {
    let Some(pool) = inp.pool else {
        return BetDetection::not_run("operator_profile.nock_bets.pool is not set to a pool address");
    };
    let Some(wallet) = inp.wallet.map(lc) else {
        return BetDetection::not_run("no tracked wallet is configured");
    };
    let Some(url) = inp.rpc_url else {
        return BetDetection::not_run("no Alchemy RPC key is configured");
    };
    match tokio::time::timeout(tokio::time::Duration::from_secs(POSITION_SCAN_TIMEOUT_SECONDS), detect_bets_inner(inp, pool, &wallet, url)).await {
        Ok(d) => d,
        Err(_) => BetDetection {
            status: "unavailable".to_string(),
            reason: Some(format!("the check took longer than {POSITION_SCAN_TIMEOUT_SECONDS} seconds and was abandoned")),
            ..Default::default()
        },
    }
}

async fn detect_bets_inner(inp: &BetDetectionInputs<'_>, pool: &str, wallet: &str, url: &str) -> BetDetection {
    let unavailable = |reason: String| BetDetection { status: "unavailable".to_string(), reason: Some(reason), ..Default::default() };
    let (latest, _) = match latest_block_header(inp.http, url).await {
        Ok(h) => h,
        Err(_) => return unavailable("the node's latest block could not be read".to_string()),
    };
    let block = (latest - POSITION_HEAD_LAG_BLOCKS).max(0);
    let mut r = BetReader { http: inp.http, url, tag: format!("0x{block:x}"), reads: 0 };
    let mut out = BetDetection { status: "ok".to_string(), block: Some(block), ..Default::default() };
    let pool_read = match read_bet_pool(&mut r, pool, block).await {
        Ok(p) => p,
        Err(why) => {
            out.status = "unavailable".to_string();
            out.pool_reason = Some(why.clone());
            out.reason = Some(format!("the bet pool {} is not read as a bet pool: {why}", short_addr(pool)));
            return out;
        }
    };
    out.pool = Some(pool_read.clone());
    let staked = match r.call(&pool_read.gauge, &format!("{CL_GAUGE_STAKED_VALUES}{}", addr_word(wallet))).await {
        Ok(Some(hex)) => match decode_staked_values(&hex) {
            Ok(ids) => ids,
            Err(why) => {
                out.status = "unavailable".to_string();
                out.reason = Some(why);
                return out;
            }
        },
        Ok(None) => {
            out.status = "unavailable".to_string();
            out.reason = Some("the gauge's stakedValues() reverted".to_string());
            return out;
        }
        Err(why) => {
            out.status = "unavailable".to_string();
            out.reason = Some(format!("the gauge's stakes could not be read ({why})"));
            return out;
        }
    };
    out.staked = Some(staked.clone());
    let (candidates, tracked) = match (inp.store.candidates(wallet).await, inp.store.tracked(wallet).await) {
        (Ok(c), Ok(t)) => (c, t),
        (a, b) => {
            for e in [a.err(), b.err()].into_iter().flatten() {
                tracing::warn!("Bet detection: tables unreadable: {e}");
            }
            out.status = "unavailable".to_string();
            out.reason = Some("the detection tables could not be read".to_string());
            return out;
        }
    };
    let in_config: HashSet<u64> = inp.config_positions.iter().filter(|c| c.protocol == AERO_CL_PROTOCOL && c.network == TRACKED_NETWORK_BASE).map(|c| c.nft_id).collect();
    let tracked_ids: HashSet<u64> = tracked.iter().filter(|t| t.protocol == AERO_CL_PROTOCOL).map(|t| t.nft_id).collect();
    let mut rows: HashMap<u64, CandidateRow> = candidates.into_iter().filter(|c| c.protocol == AERO_CL_PROTOCOL).map(|c| (c.nft_id, c)).collect();
    let soft_deadline = std::time::Instant::now() + std::time::Duration::from_secs(POSITION_SCAN_SOFT_SECONDS);
    let mut liquidity_read: HashMap<u64, u128> = HashMap::new();
    let mut read_count = 0usize;
    for nft_id in staked.iter().copied() {
        if in_config.contains(&nft_id) || tracked_ids.contains(&nft_id) {
            continue;
        }
        let spent = read_count >= POSITION_REFRESH_MAX_PER_CYCLE
            || std::time::Instant::now() >= soft_deadline
            || inp.read_budget.is_some_and(|k| r.reads >= k);
        if spent {
            out.deferred += 1;
            continue;
        }
        read_count += 1;
        let mut row = rows.remove(&nft_id).unwrap_or_else(|| CandidateRow {
            wallet: wallet.to_string(),
            protocol: AERO_CL_PROTOCOL.to_string(),
            network: TRACKED_NETWORK_BASE.to_string(),
            nft_id,
            verdict: "eligible".to_string(),
            ..Default::default()
        });
        row.owned = true;
        row.pool_id = Some(pool_read.pool.clone());
        // 1. The NFT itself.
        match r.call(AERO_CL_POSITION_MANAGER_BASE, &format!("{V3_POSITIONS}{nft_id:064x}")).await {
            Ok(Some(hex)) => match decode_slipstream_positions_return(&hex) {
                Ok(d) => {
                    row.currency0 = Some(lc(&d.token0));
                    row.currency1 = Some(lc(&d.token1));
                    row.tick_spacing = Some(d.tick_spacing);
                    row.tick_lower = Some(d.tick_lower);
                    row.tick_upper = Some(d.tick_upper);
                    row.has_liquidity = Some(d.liquidity > 0);
                    liquidity_read.insert(nft_id, d.liquidity);
                }
                Err(_) => {
                    out.deferred += 1;
                    rows.insert(nft_id, row);
                    continue;
                }
            },
            _ => {
                out.deferred += 1;
                rows.insert(nft_id, row);
                continue;
            }
        }
        // 2. The proof, read once: the chain's history does not change.
        if row.deposit_check.is_none() {
            match deposit_receipt(&mut r, &pool_read.gauge, wallet, nft_id).await {
                Ok(receipt) => match bet_mint_proof(&receipt, wallet, nft_id, &pool_read.pool, &pool_read.gauge) {
                    Ok(m) => {
                        row.mint_tx = Some(m.tx.clone());
                        row.mint_block = Some(m.block);
                        row.minted_by = Some(wallet.to_string());
                        row.deposit_check = Some(if (Some(m.tick_lower), Some(m.tick_upper)) == (row.tick_lower, row.tick_upper) {
                            "ok".to_string()
                        } else {
                            "the pool's mint in its deposit transaction has other ticks than the NFT".to_string()
                        });
                    }
                    Err(why) => {
                        row.mint_tx = receipt.get("transactionHash").and_then(|v| v.as_str()).map(lc);
                        row.minted_by = receipt.get("from").and_then(|v| v.as_str()).map(lc);
                        row.deposit_check = Some(why);
                    }
                },
                // Not read this cycle: tried again next cycle.
                Err(why) => {
                    tracing::info!("Bet detection: NFT #{nft_id}'s deposit could not be read this cycle: {why}");
                    out.status = "partial".to_string();
                    out.reason = Some(format!("a staked NFT's deposit could not be read this cycle ({why}); it is read again next cycle"));
                }
            }
        }
        store_candidate(inp.store, &row).await;
        rows.insert(nft_id, row);
    }
    if out.deferred > 0 && out.status == "ok" {
        out.status = "partial".to_string();
        out.reason = Some(format!("{} staked NFT{} wait{} for a later cycle", out.deferred, if out.deferred == 1 { "" } else { "s" }, if out.deferred == 1 { "s" } else { "" }));
    }
    // The decision for every staked NFT that is not counted.
    let mut ids: Vec<u64> = staked.iter().copied().filter(|id| !in_config.contains(id) && !tracked_ids.contains(id)).collect();
    ids.sort_unstable();
    ids.dedup();
    for nft_id in ids {
        let Some(row) = rows.get(&nft_id).cloned() else { continue };
        let dismissed = row.choice.as_deref() == Some(CHOICE_DISMISSED);
        let liquidity = liquidity_read.get(&nft_id).copied();
        let mut why: Vec<String> = Vec::new();
        match row.deposit_check.as_deref() {
            Some("ok") => {}
            Some(reason) => why.push(reason.to_string()),
            None => why.push("its deposit transaction has not been read yet".to_string()),
        }
        let pair_ok = (row.currency0.as_deref(), row.currency1.as_deref()) == (Some(pool_read.token0.as_str()), Some(pool_read.token1.as_str()))
            && row.tick_spacing == Some(pool_read.tick_spacing);
        if !pair_ok {
            why.push("its tokens and tick spacing are not the bet pool's".to_string());
        }
        match liquidity {
            Some(0) => why.push("it holds no liquidity".to_string()),
            Some(_) => {}
            None => why.push("it was not read this cycle, so it is not tracked on an old reading".to_string()),
        }
        if why.is_empty() && !dismissed {
            let t = bet_tracked_row(wallet, &pool_read, nft_id, None, &row, TRACKED_SOURCE_DETECTED);
            match validate_tracked_row(&t) {
                Ok(()) => match inp.store.insert_tracked_if_absent(&t).await {
                    Ok(_) => {
                        tracing::info!("Bet detection: NFT #{nft_id} was deposited by the wallet into the bet pool and is tracked as a bet (deposit {})", t.deposit_tx.clone().unwrap_or_default());
                        out.newly_tracked.push(t.position_key);
                        continue;
                    }
                    Err(e) => {
                        tracing::warn!("Bet detection: could not track NFT #{nft_id}: {e}");
                        why.push("tracking it did not go through this cycle; it is tried again next cycle".to_string());
                    }
                },
                Err(e) => why.push(format!("its row did not pass the checks ({e})")),
            }
        }
        if dismissed && why.is_empty() {
            why.push("you set it aside (NOT NOW or UNTRACK)".to_string());
        }
        out.found.push(FoundBet { row, dismissed, why_not_automatic: why, choices: vec![BET_TRACK_CHOICE.to_string(), "dismiss".to_string()], liquidity });
    }
    out.found.sort_by_key(|f| (f.dismissed, f.row.nft_id));
    out
}

async fn store_candidate(store: &PositionStore, row: &CandidateRow) {
    if let Err(e) = store.upsert_candidate(row).await {
        tracing::warn!("Bet detection: could not store NFT #{}: {e}", row.nft_id);
    }
}

/// The well-formed bet pool address of the profile, if any. Pure.
pub(crate) fn bet_pool_address(profile: &OperatorProfile) -> Option<String> {
    profile.nock_bets.as_ref().and_then(|b| b.pool.as_deref()).map(str::trim).filter(|p| is_address(p)).map(str::to_string)
}

pub(crate) fn is_address(p: &str) -> bool {
    p.len() == 42 && p.starts_with("0x") && p[2..].bytes().all(|b| b.is_ascii_hexdigit())
}

/// The cycle's bet detection, from the worker's state.
pub(crate) async fn detect_bets_for_cycle(state: &Arc<AppState>) -> BetDetection {
    let profile = state.config.profile();
    if profile.nock_bets.is_none() {
        return BetDetection::not_run("operator_profile.nock_bets is not set");
    }
    let store = PositionStore::of(state);
    let key = state.config.alchemy_api_keys.first().cloned().unwrap_or_default();
    let url = (!key.is_empty() && key != "YOUR_ALCHEMY_API_KEY").then(|| format!("https://base-mainnet.g.alchemy.com/v2/{key}"));
    let pool = bet_pool_address(&profile);
    detect_bets(&BetDetectionInputs {
        store: &store,
        http: &state.http_client,
        rpc_url: url.as_deref(),
        wallet: state.config.tracked_wallets.first().map(String::as_str),
        pool: pool.as_deref(),
        config_positions: &state.config.positions,
        read_budget: None,
    })
    .await
}

// ------------------------------------------------------------------
// The one-tap TRACK of a found bet
// ------------------------------------------------------------------

/// TRACK a bet found staked in the bet gauge and not proven by itself: the operator's tap.
/// Its stake by the wallet and its liquidity are read again first, at `latest`.
pub(crate) async fn track_bet(
    store: &PositionStore,
    http: &Client,
    rpc_url: Option<&str>,
    wallet: &str,
    config_positions: &[PositionConfig],
    nft_id: u64,
) -> std::result::Result<Value, TapRefusal> {
    if config_positions.iter().any(|c| c.protocol == AERO_CL_PROTOCOL && c.nft_id == nft_id) {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "CONFIG_POSITION", format!("config.yaml already lists NFT #{nft_id}: its entry is the one read. Nothing changed.")));
    }
    let candidates = store.candidates(wallet).await.map_err(bet_tap_db_error)?;
    let Some(c) = candidates.iter().find(|c| c.protocol == AERO_CL_PROTOCOL && c.nft_id == nft_id) else {
        return Err((StatusCode::NOT_FOUND, "NOT_FOUND", format!("NockNock has not found NFT #{nft_id} staked by the tracked wallet in the bet gauge. Nothing changed.")));
    };
    let Some(pool) = c.pool_id.clone() else {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "CHOICE_NOT_OFFERED", format!("NFT #{nft_id}'s pool is not known. Nothing changed.")));
    };
    let Some(url) = rpc_url else {
        return Err((StatusCode::SERVICE_UNAVAILABLE, "NO_RPC", "No Alchemy RPC key is configured, so the position could not be read again. Nothing changed.".to_string()));
    };
    let unread = || (StatusCode::SERVICE_UNAVAILABLE, "CHAIN_UNAVAILABLE", format!("NFT #{nft_id} could not be read on-chain just now. Nothing changed."));
    let mut r = BetReader { http, url, tag: "latest".to_string(), reads: 0 };
    let gauge = r.call(&pool, CL_POOL_GAUGE).await.ok().flatten().and_then(|h| word_addr(&h)).ok_or_else(unread)?;
    let staked = r
        .call(&gauge, &format!("{CL_GAUGE_STAKED_CONTAINS}{}{nft_id:064x}", addr_word(wallet)))
        .await
        .ok()
        .flatten()
        .and_then(|h| word_u128(&h))
        .ok_or_else(unread)?;
    if staked != 1 {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "NOT_OWNED", format!("NFT #{nft_id} is not staked by the tracked wallet in the bet gauge any more. Nothing changed.")));
    }
    let d = r
        .call(AERO_CL_POSITION_MANAGER_BASE, &format!("{V3_POSITIONS}{nft_id:064x}"))
        .await
        .ok()
        .flatten()
        .and_then(|h| decode_slipstream_positions_return(&h).ok())
        .ok_or_else(unread)?;
    if d.liquidity == 0 {
        return Err((StatusCode::UNPROCESSABLE_ENTITY, "EMPTY", format!("NFT #{nft_id} holds no liquidity: there is nothing to track. Nothing changed.")));
    }
    let pool_read = BetPoolRead { pool: lc(&pool), token0: lc(&d.token0), token1: lc(&d.token1), tick_spacing: d.tick_spacing, gauge, ..Default::default() };
    let mut cand = c.clone();
    cand.tick_lower = Some(d.tick_lower);
    cand.tick_upper = Some(d.tick_upper);
    let row = bet_tracked_row(wallet, &pool_read, nft_id, None, &cand, TRACKED_SOURCE_CONFIRMED);
    validate_tracked_row(&row).map_err(|why| (StatusCode::UNPROCESSABLE_ENTITY, "CHOICE_NOT_OFFERED", format!("NFT #{nft_id} cannot be tracked as a bet: {why}. Nothing changed.")))?;
    store.upsert_tracked(&row).await.map_err(bet_tap_db_error)?;
    store.set_choice(wallet, AERO_CL_PROTOCOL, TRACKED_NETWORK_BASE, nft_id, None).await.map_err(bet_tap_db_error)?;
    Ok(json!({
        "status": "tracked",
        "position_id": row.position_key,
        "nft_id": nft_id,
        "role": BET_ROLE,
        "deposit_tx": row.deposit_tx,
        "note": "Tracked as a bet: counted from the next cycle, measured and never steered.",
    }))
}

fn bet_tap_db_error(e: anyhow::Error) -> TapRefusal {
    tracing::error!("Bet write failed: {e}");
    (StatusCode::INTERNAL_SERVER_ERROR, "DB_ERROR", "The detection tables could not be read or written; nothing changed.".to_string())
}
