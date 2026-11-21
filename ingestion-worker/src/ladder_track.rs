//! A deployed ladder, tracked as `role: ladder` (#167).
//!
//! A ladder rung is a USDC/NOCK concentrated range the operator deployed to buy NOCK on the
//! way down (a deep rung) or to trade both ways near spot for fees (a near rung). Each rung
//! is its own NFT position. NockNock measures it and never steers it:
//!
//! - It is counted in every total, in the floor-crossing price (#165) and in the standing
//!   reserve (#162), whose stable balance holds its USDC.
//! - It is not NOCK LP: no LP target, no deviation row of NOCK LP, no rebuild template or
//!   exclusion, no engine label (`contracts::is_nock_lp`).
//! - Its reading (`ladders[]`) states what it holds against its own curve, and what it has
//!   bought since its deposit only from the deposit transaction's receipt. Without a
//!   `deposit_tx` there is no baseline, and none is inferred.
//!
//! Baselines considered and rejected as invented: the first snapshot that saw the position
//! (the database is disposable and the position may already have moved); "it looks
//! single-sided, so the deposit was 0 NOCK"; the pool price at the block before (a swap
//! earlier in the same block moves it); and discovering the mint from logs (an ERC721
//! Transfer from 0x0 for the token id), because a bounded `eth_getLogs` cannot find a mint
//! older than the node's range limit (200,000 blocks on the public node) without a scan the
//! worker should not make every cycle. The operator names the transaction; the receipt is
//! checked against the position before it is used.
//!
//! Nothing here pushes a notification or suggests a re-centre, a withdrawal or a deposit:
//! maintenance is #169, which reads `ladders[].state` and `since_deposit`.
use crate::*;

/// The one `role` value (#167). Matching is exact and case-sensitive, like `protocol`.
pub(crate) const LADDER_ROLE: &str = "ladder";
pub(crate) const SUPPORTED_ROLES: &str = "ladder (or leave role unset)";
pub(crate) const LADDER_RUNGS: [&str; 2] = ["near", "deep"];
pub(crate) const SUPPORTED_RUNGS: &str = "near, deep (or leave rung unset)";
/// What an unset `rung` reads as. Never a default rung.
pub(crate) const RUNG_UNSET_NOTE: &str = "rung is not set in config.yaml (near or deep): NockNock assumes neither";
/// The deviation-matrix row of tracked ladders: measured, no target (#167).
pub(crate) const LADDER_BUCKET_NAME: &str = "NOCK ladders (measured)";

/// The Uniswap v4 PoolManager on Base, which emits `ModifyLiquidity` and holds v4 deposits.
pub(crate) const V4_POOL_MANAGER_BASE: &str = "0x498581ff718922c3f8e6a244956af099b2652b2b";
/// `IncreaseLiquidity(uint256 indexed tokenId, uint128 liquidity, uint256 amount0, uint256 amount1)`,
/// the same on the Uniswap v3 NPM and the Aero CL (Slipstream) position manager.
pub(crate) const INCREASE_LIQUIDITY_TOPIC: &str =
    "0x3067048beee31b25b2f1681f88dac838c8bba36af25bfb2b7cf7473a5847e35f";
/// Swap events whose presence means a receipt's token flows are not a deposit alone:
/// Uniswap v4 PoolManager, Uniswap v3 / Slipstream pools, Aero basic pools, Uniswap v2 pools.
pub(crate) const SWAP_TOPICS: [&str; 4] = [
    "0x40e9cecb9f5f1f1c5b9c97dec2917b7ee92e57ba5563708daca94dd84ad7112f",
    "0xc42079f94a6350d7e6235f29174924f928cc2ac818eb64fed8004e115fbcca67",
    "0xb3e2773606abfd36b5bd91394b3a54d1398336c65005baf7bf7a05efeffaf75b",
    "0xd78ad95fa46c994b6551d0da85fc275fe613ce37657fb8d5e3d130840159d822",
];

/// What the cycle knows about one live ladder from config (#167).
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct LadderTag {
    /// The entry's `id` in config.yaml.
    pub(crate) config_id: String,
    /// `near` | `deep`, or None when unset (stated as unset, never assumed).
    pub(crate) rung: Option<String>,
    pub(crate) deposit_tx: Option<String>,
}

pub(crate) fn is_ladder_config(c: &PositionConfig) -> bool {
    c.role.as_deref() == Some(LADDER_ROLE)
}

/// The tag of a ladder config entry. Pure.
pub(crate) fn ladder_tag(c: &PositionConfig) -> LadderTag {
    LadderTag {
        config_id: c.id.clone(),
        rung: c.rung.clone(),
        deposit_tx: c.deposit_tx.as_ref().map(|t| t.trim().to_lowercase()),
    }
}

/// The role, rung and deposit_tx checks of one config entry, before any read (#167). Each
/// failure names the position and fails the cycle, like an unknown protocol. Pure.
pub(crate) fn validate_role_config(c: &PositionConfig) -> Result<()> {
    let fail = |what: String| -> Result<()> {
        Err(anyhow::anyhow!(
            "Configuration error: position '{}' {}. Nothing was read; correct `positions:` in config.yaml",
            c.id,
            what
        ))
    };
    if let Some(r) = c.role.as_deref() {
        if r != LADDER_ROLE {
            return fail(format!("has unknown role '{r}'. Supported: {SUPPORTED_ROLES}"));
        }
    }
    let ladder = is_ladder_config(c);
    if !ladder {
        if c.rung.is_some() {
            return fail("sets rung, which applies only to role: ladder".to_string());
        }
        if c.deposit_tx.is_some() {
            return fail("sets deposit_tx, which applies only to role: ladder".to_string());
        }
        return Ok(());
    }
    if let Some(r) = c.rung.as_deref() {
        if !LADDER_RUNGS.contains(&r) {
            return fail(format!("has unknown rung '{r}'. Supported: {SUPPORTED_RUNGS}"));
        }
    }
    match Protocol::parse(&c.protocol) {
        Some(Protocol::AeroBasic) => {
            return fail("is role: ladder on protocol aerodrome, a full-range pool: a ladder is a concentrated range".to_string())
        }
        Some(Protocol::UniswapV4) | Some(Protocol::UniswapV3) | Some(Protocol::AeroCl) | None => {}
    }
    if c.is_price_source {
        return fail(
            "is role: ladder and is_price_source: a ladder sits out of range by design, and the price source's range is what RANGE EXIT and the range-exit push watch; flag another position as the price source"
                .to_string(),
        );
    }
    if let Some(w) = c.rebuild_weight_bps {
        return fail(format!(
            "is role: ladder and sets rebuild_weight_bps ({w}): a ladder is never a rebuild template, the role already leaves it out; remove the key"
        ));
    }
    // A plain bool with a default: only an explicit `true` can be told apart from unset.
    if c.rebuild_when_out_of_range {
        return fail(
            "is role: ladder and sets rebuild_when_out_of_range: true: a ladder is never a rebuild template; remove the key".to_string(),
        );
    }
    if c.deposit_url.is_some() {
        return fail(
            "is role: ladder and sets deposit_url, which only rebuild suggestions use: NockNock suggests no deposit into a ladder; remove the key"
                .to_string(),
        );
    }
    if let Some(tx) = c.deposit_tx.as_deref() {
        let t = tx.trim();
        let hex = t.strip_prefix("0x").unwrap_or("");
        if hex.len() != 64 || !hex.bytes().all(|b| b.is_ascii_hexdigit()) {
            return fail(format!("has deposit_tx '{t}', which is not a transaction hash (0x and 64 hex characters)"));
        }
    }
    Ok(())
}

/// How close to the pool's usable extremes a bound must be to count as reaching it, when
/// the pool's tick spacing is unknown: 1,000 ticks (a 10.5% price step) is wider than the
/// spacing of any Uniswap or Aero pool NockNock reads on Base.
pub(crate) const LADDER_EDGE_TICKS_UNKNOWN_SPACING: i32 = 1000;

/// The tick spacing of a ladder's pool: read for Uniswap v4 and Aero CL (position facts);
/// for Uniswap v3 the factory's fixed fee-to-spacing table (0.01% 1, 0.05% 10, 0.30% 60,
/// 1.00% 200). None when neither applies. Pure.
pub(crate) fn ladder_tick_spacing(facts: &HashMap<String, PositionFacts>, p: &RawPosition) -> Option<i32> {
    if let Some(s) = facts.get(&p.position_id).and_then(|f| f.tick_spacing).filter(|s| *s > 0) {
        return Some(s);
    }
    match Protocol::parse(&p.protocol) {
        Some(Protocol::UniswapV3) => match p.fee_tier_bps {
            1 => Some(1),
            5 => Some(10),
            30 => Some(60),
            100 => Some(200),
            _ => None,
        },
        Some(Protocol::UniswapV4) | Some(Protocol::AeroCl) | Some(Protocol::AeroBasic) | None => None,
    }
}

/// Whether each bound reaches the pool's usable extremes (within one tick spacing of the
/// lowest or highest usable tick): (lower bound at the minimum, upper bound at the
/// maximum). A full-range NFT made in a Uniswap or Aero app sits on those usable ticks
/// (±887220 at spacing 60, ±887200 at 200, ±887100 at 300), not on MIN_TICK/MAX_TICK. Pure.
pub(crate) fn ladder_bounds_at_extremes(tick_lower: i32, tick_upper: i32, spacing: Option<i32>) -> (bool, bool) {
    let (lo, hi, margin) = match spacing {
        Some(s) => {
            let lo = (MIN_TICK as f64 / s as f64).ceil() as i32 * s;
            let hi = (MAX_TICK as f64 / s as f64).floor() as i32 * s;
            (lo, hi, s)
        }
        None => (MIN_TICK, MAX_TICK, LADDER_EDGE_TICKS_UNKNOWN_SPACING),
    };
    (tick_lower <= lo + margin, tick_upper >= hi - margin)
}

/// After the reads (#167): a ladder must be a USDC/NOCK concentrated range. AERO and ETH
/// ladders are #170. A range whose two bounds both reach the pool's usable extremes is
/// full range, and is refused too. A failure is a configuration error that fails the
/// cycle. Pure.
pub(crate) fn check_ladder_pairs(positions: &[RawPosition], ladders: &HashMap<String, LadderTag>, facts: &HashMap<String, PositionFacts>) -> Result<()> {
    for p in positions {
        let Some(tag) = ladders.get(&p.position_id) else { continue };
        let usdc_nock = (is_usdc_token(&p.token0) && is_nock_token(&p.token1)) || (is_nock_token(&p.token0) && is_usdc_token(&p.token1));
        if !usdc_nock {
            return Err(anyhow::anyhow!(
                "Configuration error: position '{}' is role: ladder but is a {}/{} pair. A ladder is a USDC/NOCK range; AERO and ETH ladders are not supported yet (#170). Correct `positions:` in config.yaml",
                tag.config_id,
                p.token0.symbol,
                p.token1.symbol
            ));
        }
        let spacing = ladder_tick_spacing(facts, p);
        let (at_min, at_max) = ladder_bounds_at_extremes(p.tick_lower, p.tick_upper, spacing);
        if is_full_range(p.tick_lower, p.tick_upper) || (at_min && at_max) {
            return Err(anyhow::anyhow!(
                "Configuration error: position '{}' is role: ladder but spans every price (full range: ticks {}..{} reach the pool's usable extremes{}): a ladder is a concentrated range. Correct `positions:` in config.yaml",
                tag.config_id,
                p.tick_lower,
                p.tick_upper,
                spacing.map(|s| format!(" at tick spacing {s}")).unwrap_or_else(|| format!(", within {LADDER_EDGE_TICKS_UNKNOWN_SPACING} ticks (tick spacing not read)"))
            ));
        }
    }
    Ok(())
}

// ------------------------------------------------------------------
// The deposit receipt
// ------------------------------------------------------------------

/// A verified deposit: the receipt of the ladder's `deposit_tx`, checked against the position.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LadderDeposit {
    pub(crate) tx: String,
    pub(crate) block: i64,
    /// The block's time; None when the header could not be read (the baseline still stands).
    pub(crate) at: Option<DateTime<Utc>>,
    /// Liquidity the deposit added.
    pub(crate) liquidity: u128,
    /// What the wallet paid, in raw units of the position's token0 and token1.
    pub(crate) amount0_raw: u128,
    pub(crate) amount1_raw: u128,
}

/// The outcome of reading one ladder's `deposit_tx` (#167).
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum LadderBaselineRead {
    /// No deposit_tx is set.
    NotConfigured,
    Verified(LadderDeposit),
    /// The receipt was read and does not prove a clean deposit into this position. Cached:
    /// a receipt does not change.
    Refused { tx: String, reason: String },
    /// The receipt could not be read this cycle. Not cached: retried next cycle.
    Unverified { tx: String, reason: String },
}

fn lc(s: &str) -> String {
    s.trim().to_lowercase()
}

fn addr_word(addr: &str) -> String {
    format!("000000000000000000000000{}", addr.trim().trim_start_matches("0x").to_lowercase())
}

fn topics_of(log: &serde_json::Value) -> Vec<String> {
    log.get("topics")
        .and_then(|t| t.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str()).map(lc).collect())
        .unwrap_or_default()
}

fn words_of(log: &serde_json::Value) -> Vec<String> {
    let data = log.get("data").and_then(|d| d.as_str()).unwrap_or("").trim_start_matches("0x").to_lowercase();
    if data.len() % 64 != 0 {
        return Vec::new();
    }
    (0..data.len() / 64).map(|i| data[i * 64..(i + 1) * 64].to_string()).collect()
}

/// An unsigned word that fits a u128 (the high half is zero).
fn word_u128(w: &str) -> Option<u128> {
    let w = w.trim_start_matches("0x");
    if w.len() != 64 || !w[..32].bytes().all(|b| b == b'0') {
        return None;
    }
    u128::from_str_radix(&w[32..], 16).ok()
}

fn log_address(log: &serde_json::Value) -> String {
    log.get("address").and_then(|a| a.as_str()).map(lc).unwrap_or_default()
}

/// The raw amounts a deposit of `l` into ticks `lower..upper` can take, checked: they must
/// fit the range (no more than its all-token0 and all-token1 amounts) and, when both are
/// paid, imply one pool price (within 1 bp of sqrt price). Uniswap rounds an amount owed up
/// by one raw unit, so each bound allows 1 raw unit plus 1 bp. Pure.
pub(crate) fn check_deposit_on_curve(l: u128, a0: u128, a1: u128, tick_lower: i32, tick_upper: i32) -> std::result::Result<(), String> {
    let lf = l as f64;
    let (sa, sb) = (tick_sqrt_price(tick_lower), tick_sqrt_price(tick_upper));
    let (max0, _) = concentrated_amounts_at(lf, sa, sa, sb);
    let (_, max1) = concentrated_amounts_at(lf, sb, sa, sb);
    let tol = |x: f64| 1.0 + x.abs() * 1e-4;
    let (f0, f1) = (a0 as f64, a1 as f64);
    if f0 > max0 + tol(max0) || f1 > max1 + tol(max1) {
        return Err(format!(
            "the amounts paid ({a0} and {a1} raw) are more than liquidity {l} in this range can hold; the transfers are not the deposit alone"
        ));
    }
    let one_sided0 = f1 <= 1.0;
    let one_sided1 = f0 <= 1.0;
    if one_sided0 && one_sided1 {
        return Err("the wallet paid nothing into the position".to_string());
    }
    if one_sided0 {
        if (f0 - max0).abs() > tol(max0) {
            return Err(format!("a deposit of only token0 into this range takes {max0:.0} raw for liquidity {l}, not {a0}"));
        }
        return Ok(());
    }
    if one_sided1 {
        if (f1 - max1).abs() > tol(max1) {
            return Err(format!("a deposit of only token1 into this range takes {max1:.0} raw for liquidity {l}, not {a1}"));
        }
        return Ok(());
    }
    // In range at the deposit: each amount implies the pool's sqrt price.
    let s_from1 = f1 / lf + sa;
    let s_from0 = 1.0 / (f0 / lf + 1.0 / sb);
    if !(s_from0.is_finite() && s_from1.is_finite()) || (s_from0 / s_from1 - 1.0).abs() > 1e-4 {
        return Err(format!(
            "the two amounts paid ({a0} and {a1} raw) do not imply one pool price for liquidity {l}; the transfers are not the deposit alone"
        ));
    }
    Ok(())
}

/// Check one receipt (the `result` of eth_getTransactionReceipt) against a ladder
/// position, and read its deposit. Refuses unless: the transaction succeeded and was sent
/// by the wallet; it contains no swap; it made exactly one deposit, into this position
/// (v4: one PoolManager ModifyLiquidity through the position manager with this pool, these
/// ticks, salt = the NFT id and liquidity added; v3 and Aero CL: one IncreaseLiquidity of
/// a known position manager for this NFT); and the amounts fit the range's curve. v4
/// amounts are the wallet's ERC20 transfers to the PoolManager less any refunds back;
/// v3 / Aero CL amounts are the event's own. `at` is left None. Pure.
pub(crate) fn parse_ladder_deposit(receipt: &serde_json::Value, wallet: &str, p: &RawPosition) -> std::result::Result<LadderDeposit, String> {
    let tx = receipt.get("transactionHash").and_then(|v| v.as_str()).map(lc).unwrap_or_default();
    let status = receipt.get("status").and_then(|v| v.as_str()).map(lc).unwrap_or_default();
    if status != "0x1" {
        return Err(format!("the transaction failed (status {status}): it deposited nothing"));
    }
    let from = receipt.get("from").and_then(|v| v.as_str()).map(lc).unwrap_or_default();
    if from != lc(wallet) {
        return Err(format!("the transaction was sent by {from}, not by the tracked wallet"));
    }
    let block = receipt
        .get("blockNumber")
        .and_then(|v| v.as_str())
        .and_then(|h| i64::from_str_radix(h.trim_start_matches("0x"), 16).ok())
        .ok_or_else(|| "the receipt has no block number".to_string())?;
    let logs: Vec<serde_json::Value> = receipt.get("logs").and_then(|l| l.as_array()).cloned().unwrap_or_default();
    if logs.iter().any(|l| topics_of(l).first().is_some_and(|t| SWAP_TOPICS.contains(&t.as_str()))) {
        return Err("the transaction also swaps, so its token flows are not the deposit alone".to_string());
    }
    let nft_word = format!("{:064x}", p.nft_id);
    let (liquidity, a0, a1) = match Protocol::parse(&p.protocol) {
        Some(Protocol::UniswapV4) => {
            let mods: Vec<&serde_json::Value> = logs
                .iter()
                .filter(|l| log_address(l) == V4_POOL_MANAGER_BASE && topics_of(l).first().map(String::as_str) == Some(V4_MODIFY_LIQUIDITY_TOPIC))
                .collect();
            if mods.len() != 1 {
                return Err(format!("the receipt has {} PoolManager ModifyLiquidity events; a ladder deposit has exactly one", mods.len()));
            }
            let log = mods[0];
            let topics = topics_of(log);
            let pool = lc(&p.pool_id);
            if topics.get(1).map(String::as_str) != Some(pool.as_str()) {
                return Err(format!("it modifies pool {}, not this position's pool {pool}", topics.get(1).cloned().unwrap_or_default()));
            }
            if topics.get(2).map(|t| t.trim_start_matches("0x").to_string()) != Some(addr_word(UNISWAP_V4_POSM_BASE)) {
                return Err("its liquidity was not added through the Uniswap v4 position manager".to_string());
            }
            let w = words_of(log);
            if w.len() != 4 {
                return Err("its ModifyLiquidity event could not be read".to_string());
            }
            let (lo, hi) = match (parse_hex_word_i32(&w[0]), parse_hex_word_i32(&w[1])) {
                (Ok(a), Ok(b)) => (a, b),
                _ => return Err("its ModifyLiquidity ticks could not be read".to_string()),
            };
            if (lo, hi) != (p.tick_lower, p.tick_upper) {
                return Err(format!("it deposits into ticks {lo}..{hi}, not this position's {}..{}", p.tick_lower, p.tick_upper));
            }
            if w[3] != nft_word {
                return Err(format!("its salt is not NFT #{}: it is another position's deposit", p.nft_id));
            }
            if w[2].starts_with(|c: char| ('8'..='f').contains(&c)) {
                return Err("it removes liquidity: it is a withdrawal, not a deposit".to_string());
            }
            let l = word_u128(&w[2]).filter(|l| *l > 0).ok_or_else(|| "it adds no liquidity".to_string())?;
            if p.token0.address == NATIVE_ETH || p.token1.address == NATIVE_ETH {
                return Err("a native-ETH leg cannot be read from token transfers".to_string());
            }
            let (pm, wl) = (addr_word(V4_POOL_MANAGER_BASE), addr_word(wallet));
            let paid = |token: &str| -> std::result::Result<u128, String> {
                let (mut out, mut back) = (0u128, 0u128);
                for l in logs.iter().filter(|l| log_address(l) == lc(token)) {
                    let t = topics_of(l);
                    if t.len() != 3 || t[0] != TRANSFER_EVENT_TOPIC {
                        continue;
                    }
                    let (f, to) = (t[1].trim_start_matches("0x"), t[2].trim_start_matches("0x"));
                    let amount = words_of(l).first().and_then(|w| word_u128(w)).unwrap_or(0);
                    if f == wl && to == pm {
                        out = out.saturating_add(amount);
                    } else if f == pm && to == wl {
                        back = back.saturating_add(amount);
                    }
                }
                out.checked_sub(back).ok_or_else(|| format!("the wallet received more {token} from the PoolManager than it paid"))
            };
            (l, paid(&p.token0.address)?, paid(&p.token1.address)?)
        }
        Some(proto @ (Protocol::UniswapV3 | Protocol::AeroCl)) => {
            // Only the position manager of this position's protocol: the two managers'
            // NFT ids are separate namespaces, and the event names neither pool nor ticks.
            let (manager, name) = match proto {
                Protocol::UniswapV3 => (UNISWAP_V3_NPM_BASE, "the Uniswap v3 position manager"),
                _ => (AERO_CL_POSITION_MANAGER_BASE, "the Aero CL position manager"),
            };
            let is_inc = |l: &&serde_json::Value| topics_of(l).first().map(String::as_str) == Some(INCREASE_LIQUIDITY_TOPIC);
            let incs: Vec<&serde_json::Value> = logs.iter().filter(is_inc).filter(|l| log_address(l) == lc(manager)).collect();
            if incs.is_empty() && logs.iter().filter(is_inc).any(|l| PRINCIPAL_POSITION_MANAGERS.iter().any(|m| log_address(l) == lc(m))) {
                return Err(format!("its IncreaseLiquidity was emitted by another position manager, not {name}: it is another protocol's NFT"));
            }
            if incs.len() != 1 {
                return Err(format!("the receipt has {} IncreaseLiquidity events from {name}; a ladder deposit has exactly one", incs.len()));
            }
            let log = incs[0];
            if topics_of(log).get(1).map(|t| t.trim_start_matches("0x").to_string()) != Some(nft_word) {
                return Err(format!("its IncreaseLiquidity is for another NFT, not #{}", p.nft_id));
            }
            let w = words_of(log);
            match (w.first().and_then(|x| word_u128(x)), w.get(1).and_then(|x| word_u128(x)), w.get(2).and_then(|x| word_u128(x))) {
                (Some(l), Some(a), Some(b)) if l > 0 => (l, a, b),
                _ => return Err("its IncreaseLiquidity event could not be read".to_string()),
            }
        }
        Some(Protocol::AeroBasic) | None => {
            return Err(format!("protocol {} has no ladder deposit to read", p.protocol));
        }
    };
    check_deposit_on_curve(liquidity, a0, a1, p.tick_lower, p.tick_upper)?;
    Ok(LadderDeposit { tx, block, at: None, liquidity, amount0_raw: a0, amount1_raw: a1 })
}

/// Verified and refused reads, keyed by (position_id, tx). A receipt never changes, so
/// they are read once per process; the database is disposable (AGENTS.md section 13) and
/// holds none. An unverified read is never cached, and neither is a verified one whose
/// block time could not be read (it is read again until the time is known).
static LADDER_RECEIPTS: std::sync::Mutex<Option<HashMap<(String, String), LadderBaselineRead>>> = std::sync::Mutex::new(None);

fn cached_baseline(key: &(String, String)) -> Option<LadderBaselineRead> {
    LADDER_RECEIPTS.lock().unwrap_or_else(|p| p.into_inner()).as_ref().and_then(|m| m.get(key).cloned())
}

fn cache_baseline(key: (String, String), read: &LadderBaselineRead) {
    LADDER_RECEIPTS.lock().unwrap_or_else(|p| p.into_inner()).get_or_insert_with(HashMap::new).insert(key, read.clone());
}

/// Read one ladder's baseline: its deposit_tx receipt, checked by parse_ladder_deposit,
/// and the block's time. Never fails the cycle.
pub(crate) async fn read_ladder_baseline(client: &Client, rpc_url: &str, wallet: &str, p: &RawPosition, tag: &LadderTag) -> LadderBaselineRead {
    let Some(tx) = tag.deposit_tx.clone() else {
        return LadderBaselineRead::NotConfigured;
    };
    let key = (p.position_id.clone(), tx.clone());
    if let Some(hit) = cached_baseline(&key) {
        return hit;
    }
    let payload = json!({ "jsonrpc": "2.0", "id": 167, "method": "eth_getTransactionReceipt", "params": [tx] });
    let Some(resp) = post_rpc_with_retry(client, rpc_url, &payload, 2).await else {
        return LadderBaselineRead::Unverified { tx, reason: "the receipt could not be read this cycle (the RPC did not answer); it is read again next cycle".to_string() };
    };
    let receipt = match resp.get("result") {
        Some(r) if r.is_object() => r.clone(),
        // post_rpc_with_retry returns no JSON-RPC error (it retries them, then gives None):
        // a null result is a hash the node does not know, or one not mined yet.
        _ => {
            return LadderBaselineRead::Unverified {
                tx,
                reason: "the receipt could not be read this cycle: the node has no receipt for it (an unknown hash, or not mined yet); it is read again next cycle".to_string(),
            };
        }
    };
    let read = match parse_ladder_deposit(&receipt, wallet, p) {
        Err(reason) => LadderBaselineRead::Refused { tx, reason },
        Ok(mut d) => {
            d.at = fetch_block_time(client, rpc_url, d.block).await;
            LadderBaselineRead::Verified(d)
        }
    };
    let keep = match &read {
        LadderBaselineRead::Verified(d) => d.at.is_some(),
        LadderBaselineRead::Refused { .. } => true,
        _ => false,
    };
    if keep {
        cache_baseline(key, &read);
    }
    read
}

// ------------------------------------------------------------------
// The reading
// ------------------------------------------------------------------

/// What a ladder holds against its own curve (#167). A range whose liquidity has not
/// changed traces one fixed curve, so these need no baseline: they say where the range is
/// between its all-USDC top and its all-NOCK bottom. They are holdings, not trades: a range
/// in range converts both ways.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub(crate) struct LadderVsTop {
    /// USDC the range holds when the price is at or above its top, `{:.2}`. Null when the
    /// top reaches the pool's highest usable price (`open_end` says so).
    pub(crate) usdc_at_top: Option<String>,
    /// NOCK it holds at or below its bottom, `{:.4}`. Null when the bottom reaches the
    /// pool's lowest usable price: that amount is not a figure anyone could hold.
    pub(crate) nock_at_bottom: Option<String>,
    /// NOCK it holds now, against 0 at the top, `{:.4}`.
    pub(crate) nock_held_vs_top: String,
    /// `usdc_at_top` less the USDC it holds now, `{:.2}`. Null with `usdc_at_top`.
    pub(crate) usdc_converted_vs_top: Option<String>,
    /// `usdc_converted_vs_top` / `nock_held_vs_top`: the curve's average price between the
    /// top and the pool price, `{:.8}`. Null when it holds no NOCK.
    pub(crate) curve_avg_price_usd: Option<String>,
    /// `usdc_converted_vs_top` as a percentage of `usdc_at_top`, `{:.2}`.
    pub(crate) converted_pct: Option<String>,
}

/// What a ladder bought since its deposit, from the deposit_tx receipt only (#167).
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub(crate) struct LadderSinceDeposit {
    /// verified | no_baseline | refused | unverified | liquidity_changed
    pub(crate) status: String,
    pub(crate) deposit_tx: Option<String>,
    pub(crate) deposit_block: Option<i64>,
    /// The deposit block's time; null when the header could not be read.
    pub(crate) deposited_at: Option<String>,
    /// What the wallet paid in, `{:.2}` USDC and `{:.4}` NOCK (from the receipt).
    pub(crate) deposited_usdc: Option<String>,
    pub(crate) deposited_nock: Option<String>,
    pub(crate) deposit_liquidity: Option<String>,
    pub(crate) current_liquidity: String,
    /// NOCK now less NOCK deposited, signed (negative: it sold NOCK it was deposited with).
    /// Only when `verified`.
    pub(crate) net_nock_bought: Option<String>,
    /// USDC deposited less USDC now, signed (negative: it gained USDC). Only when `verified`.
    pub(crate) net_usdc_spent: Option<String>,
    /// net_usdc_spent / net_nock_bought, only when both are above zero, `{:.8}`.
    pub(crate) avg_buy_price_usd: Option<String>,
    /// Why the figures are null; null when verified.
    pub(crate) reason: Option<String>,
}

/// One tracked ladder, measured (2.9.0, #167). `data.ladders[]`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
pub(crate) struct LadderReading {
    pub(crate) position_id: String,
    pub(crate) config_id: String,
    pub(crate) label: String,
    /// `near` | `deep` from config; null when unset (`rung_note` says so).
    pub(crate) rung: Option<String>,
    pub(crate) rung_note: Option<String>,
    /// holds_only_usdc | in_range | holds_only_nock | empty | pool_stale. Named by what
    /// the range holds, not by a direction.
    pub(crate) state: String,
    pub(crate) state_text: String,
    /// The range's bottom and top in USD per NOCK, `{:.8}`. Null for a bound that reaches
    /// the pool's usable extremes (`open_end` says which): no price is printed for it.
    pub(crate) low_price_usd: Option<String>,
    pub(crate) high_price_usd: Option<String>,
    /// Set when a bound reaches the pool's usable extremes: which one, in words. Null
    /// otherwise.
    pub(crate) open_end: Option<String>,
    /// The bounds against the NOCK/USD spot, signed percent, `{:+.2}`. Null without a spot.
    pub(crate) low_pct_of_spot: Option<String>,
    pub(crate) high_pct_of_spot: Option<String>,
    pub(crate) spot_price_usd: Option<String>,
    /// USD per NOCK in the ladder's own pool, `{:.8}`.
    pub(crate) pool_price_usd: Option<String>,
    pub(crate) usdc_now: String,
    pub(crate) nock_now: String,
    /// Null without a spot.
    pub(crate) value_usd: Option<String>,
    /// Uncollected fees, apart from every bought figure.
    pub(crate) fees_usd: String,
    pub(crate) vs_top: LadderVsTop,
    pub(crate) since_deposit: LadderSinceDeposit,
}

/// One USD-per-NOCK price at a tick of a USDC/NOCK pool, either token order. Pure.
fn usd_per_nock_at_tick(p: &RawPosition, tick: i32) -> f64 {
    if is_nock_token(&p.token1) {
        tick_to_display_price_dec(tick, p.token0.decimals, p.token1.decimals)
    } else {
        // NOCK token0: token1 (USDC) per token0 = 1.0001^tick * 10^(dec0 - dec1).
        1.0001f64.powi(tick) * 10f64.powi(p.token0.decimals - p.token1.decimals)
    }
}

/// (usdc, nock) human amounts of a (token0, token1) pair. Pure.
fn usdc_nock(p: &RawPosition, a0: f64, a1: f64) -> (f64, f64) {
    if is_nock_token(&p.token1) { (a0, a1) } else { (a1, a0) }
}

/// "-0.00" never: a value that rounds to zero is zero.
fn signed(v: f64, dp: usize) -> String {
    let s = format!("{:.*}", dp, v);
    if s.trim_start_matches('-').chars().all(|c| c == '0' || c == '.') { format!("{:.*}", dp, 0.0) } else { s }
}

/// The state of a ladder: what it holds now. Pure.
pub(crate) fn ladder_state(raw: &RawOnChainData, p: &RawPosition) -> &'static str {
    let stale = raw.pool_liquidity.get(&p.position_id).and_then(|v| v.parse::<u128>().ok()) == Some(0);
    let nock_is_1 = is_nock_token(&p.token1);
    if p.liquidity == 0 {
        "empty"
    } else if stale {
        "pool_stale"
    } else if p.in_range {
        "in_range"
    } else if (nock_is_1 && p.tick < p.tick_lower) || (!nock_is_1 && p.tick >= p.tick_upper) {
        "holds_only_usdc"
    } else {
        "holds_only_nock"
    }
}

/// The measured reading of one ladder. Pure.
pub(crate) fn ladder_reading(raw: &RawOnChainData, p: &RawPosition, tag: &LadderTag) -> LadderReading {
    let spot = raw.spot_price_nock_usd;
    let spot_ok = spot.is_finite() && spot > 0.0;
    let nock_is_1 = is_nock_token(&p.token1);
    let (usdc_dec, nock_dec) = if nock_is_1 { (p.token0.decimals, p.token1.decimals) } else { (p.token1.decimals, p.token0.decimals) };
    let (low, high) = if nock_is_1 {
        (usd_per_nock_at_tick(p, p.tick_upper), usd_per_nock_at_tick(p, p.tick_lower))
    } else {
        (usd_per_nock_at_tick(p, p.tick_lower), usd_per_nock_at_tick(p, p.tick_upper))
    };
    let pct = |v: f64| spot_ok.then(|| signed((v / spot - 1.0) * 100.0, 2)).map(|s| if s.starts_with('-') { s } else { format!("+{s}") });
    // A bound at the pool's usable extremes is open-ended: its price and the amount held
    // there are stated as such, never as a number (a NOCK amount beyond any supply).
    let (at_min, at_max) = ladder_bounds_at_extremes(p.tick_lower, p.tick_upper, ladder_tick_spacing(&raw.position_facts, p));
    // NOCK token1: the bottom (lowest USD) is the upper tick, the top the lower tick.
    let (bottom_open, top_open) = if nock_is_1 { (at_max, at_min) } else { (at_min, at_max) };
    let open_end = match (bottom_open, top_open) {
        (true, true) => Some("the range spans every usable price: it has no bottom and no top to state".to_string()),
        (true, false) => Some(format!(
            "the bottom reaches the pool's lowest usable price (tick {}): the range buys all the way down, so no bottom price and no NOCK at the bottom are stated",
            if nock_is_1 { p.tick_upper } else { p.tick_lower }
        )),
        (false, true) => Some(format!(
            "the top reaches the pool's highest usable price (tick {}): no top price and no USDC at the top are stated",
            if nock_is_1 { p.tick_lower } else { p.tick_upper }
        )),
        (false, false) => None,
    };
    // The range at its edges: all token0 at the lower tick, all token1 at the upper.
    let l = p.liquidity as f64;
    let (sa, sb) = (tick_sqrt_price(p.tick_lower), tick_sqrt_price(p.tick_upper));
    let (full0, _) = concentrated_amounts_at(l, sa, sa, sb);
    let (_, full1) = concentrated_amounts_at(l, sb, sa, sb);
    let (usdc_top_raw, nock_bottom_raw) = if nock_is_1 { (full0, full1) } else { (full1, full0) };
    let usdc_at_top = usdc_top_raw / 10f64.powi(usdc_dec);
    let nock_at_bottom = nock_bottom_raw / 10f64.powi(nock_dec);
    let (usdc_now, nock_now) = usdc_nock(p, p.amount0, p.amount1);
    let converted = (usdc_at_top - usdc_now).max(0.0);
    let p0 = leg_usd_price(&p.token0, spot, raw.live_eth_price_usd);
    let p1 = leg_usd_price(&p.token1, spot, raw.live_eth_price_usd);
    let pool_price = {
        let mut q = p.clone();
        q.is_price_source = true;
        nock_spot_from_positions(std::slice::from_ref(&q))
    };
    let state = ladder_state(raw, p);
    let state_text = match state {
        "holds_only_usdc" if !top_open => format!(
            "holds only USDC: the price is above the range, so it converts nothing until NOCK falls into it at ${:.8}",
            high
        ),
        "holds_only_usdc" => "holds only USDC: the price is above the range, so it converts nothing until NOCK falls into it".to_string(),
        "in_range" => "in range: it converts USDC to NOCK as the price falls and NOCK back to USDC as it rises".to_string(),
        "holds_only_nock" if !bottom_open => format!(
            "holds only NOCK: the price is below the range's bottom (${:.8}); a rise back into it sells NOCK for USDC",
            low
        ),
        "holds_only_nock" => "holds only NOCK: the price is below the range's bottom; a rise back into it sells NOCK for USDC".to_string(),
        "empty" => "empty: the NFT holds no liquidity".to_string(),
        _ => "its pool has no in-range liquidity, so the pool price is frozen and the state is not a market reading".to_string(),
    };
    LadderReading {
        position_id: p.position_id.clone(),
        config_id: tag.config_id.clone(),
        label: position_label(p),
        rung: tag.rung.clone(),
        rung_note: tag.rung.is_none().then(|| RUNG_UNSET_NOTE.to_string()),
        state: state.to_string(),
        state_text,
        low_price_usd: (!bottom_open).then(|| format!("{:.8}", low)),
        high_price_usd: (!top_open).then(|| format!("{:.8}", high)),
        open_end,
        low_pct_of_spot: if bottom_open { None } else { pct(low) },
        high_pct_of_spot: if top_open { None } else { pct(high) },
        spot_price_usd: spot_ok.then(|| format!("{:.8}", spot)),
        pool_price_usd: pool_price.map(|v| format!("{:.8}", v)),
        usdc_now: format!("{:.2}", usdc_now),
        nock_now: format!("{:.4}", nock_now),
        value_usd: spot_ok.then(|| format!("{:.2}", p.amount0 * p0 + p.amount1 * p1)),
        fees_usd: format!("{:.2}", p.fee0 * p0 + p.fee1 * p1),
        vs_top: LadderVsTop {
            usdc_at_top: (!top_open).then(|| format!("{:.2}", usdc_at_top)),
            nock_at_bottom: (!bottom_open).then(|| format!("{:.4}", nock_at_bottom)),
            nock_held_vs_top: format!("{:.4}", nock_now),
            usdc_converted_vs_top: (!top_open).then(|| format!("{:.2}", converted)),
            curve_avg_price_usd: (!top_open && nock_now > 0.0 && converted > 0.0).then(|| format!("{:.8}", converted / nock_now)),
            converted_pct: (!top_open && usdc_at_top > 0.0).then(|| format!("{:.2}", converted / usdc_at_top * 100.0)),
        },
        since_deposit: since_deposit(raw, p, tag, usdc_now, nock_now),
    }
}

fn since_deposit(raw: &RawOnChainData, p: &RawPosition, tag: &LadderTag, usdc_now: f64, nock_now: f64) -> LadderSinceDeposit {
    let current_liquidity = p.liquidity.to_string();
    let read = match (&tag.deposit_tx, raw.ladder_baselines.get(&p.position_id)) {
        (None, _) => LadderBaselineRead::NotConfigured,
        (Some(tx), None) => LadderBaselineRead::Unverified { tx: tx.clone(), reason: "the receipt was not read this cycle".to_string() },
        (Some(_), Some(r)) => r.clone(),
    };
    match read {
        LadderBaselineRead::NotConfigured => LadderSinceDeposit {
            status: "no_baseline".to_string(),
            current_liquidity,
            reason: Some(
                "no deposit_tx is set for this ladder in config.yaml: NockNock has no baseline for what it bought and does not invent one".to_string(),
            ),
            ..Default::default()
        },
        LadderBaselineRead::Refused { tx, reason } => LadderSinceDeposit {
            status: "refused".to_string(),
            deposit_tx: Some(tx),
            current_liquidity,
            reason: Some(format!("the deposit_tx receipt does not prove a deposit into this position: {reason}")),
            ..Default::default()
        },
        LadderBaselineRead::Unverified { tx, reason } => LadderSinceDeposit {
            status: "unverified".to_string(),
            deposit_tx: Some(tx),
            current_liquidity,
            reason: Some(reason),
            ..Default::default()
        },
        LadderBaselineRead::Verified(d) => {
            let (u_raw, n_raw) = if is_nock_token(&p.token1) { (d.amount0_raw, d.amount1_raw) } else { (d.amount1_raw, d.amount0_raw) };
            let (u_dec, n_dec) = if is_nock_token(&p.token1) { (p.token0.decimals, p.token1.decimals) } else { (p.token1.decimals, p.token0.decimals) };
            let dep_usdc = u_raw as f64 / 10f64.powi(u_dec);
            let dep_nock = n_raw as f64 / 10f64.powi(n_dec);
            let base = LadderSinceDeposit {
                deposit_tx: Some(d.tx.clone()),
                deposit_block: Some(d.block),
                deposited_at: d.at.map(|t| t.to_rfc3339()),
                deposited_usdc: Some(format!("{:.2}", dep_usdc)),
                deposited_nock: Some(format!("{:.4}", dep_nock)),
                deposit_liquidity: Some(d.liquidity.to_string()),
                current_liquidity,
                ..Default::default()
            };
            if d.liquidity != p.liquidity {
                return LadderSinceDeposit {
                    status: "liquidity_changed".to_string(),
                    reason: Some(format!(
                        "liquidity was added or partly withdrawn since the deposit (deposited {}, now {}): that receipt is no longer the whole baseline, so nothing bought is reported",
                        d.liquidity, p.liquidity
                    )),
                    ..base
                };
            }
            let bought = nock_now - dep_nock;
            let spent = dep_usdc - usdc_now;
            LadderSinceDeposit {
                status: "verified".to_string(),
                net_nock_bought: Some(signed(bought, 4)),
                net_usdc_spent: Some(signed(spent, 2)),
                avg_buy_price_usd: (bought > 0.0 && spent > 0.0).then(|| format!("{:.8}", spent / bought)),
                ..base
            }
        }
    }
}

/// Every tracked ladder's reading, in positions order. Empty with no ladder. Pure.
pub(crate) fn ladder_readings(raw: &RawOnChainData) -> Vec<LadderReading> {
    raw.positions
        .iter()
        .filter_map(|p| raw.ladders.get(&p.position_id).map(|tag| ladder_reading(raw, p, tag)))
        .collect()
}

/// Stamp `role` and `rung` on the snapshot positions that are ladders. Pure.
pub(crate) fn apply_ladder_roles(positions: &mut [Position], ladders: &HashMap<String, LadderTag>) {
    for pos in positions.iter_mut() {
        if let Some(tag) = ladders.get(&pos.position_id) {
            pos.role = Some(LADDER_ROLE.to_string());
            pos.rung = tag.rung.clone();
        }
    }
}

/// The ladders of a stored snapshot, from its positions' `role` (#167), for code that
/// rebuilds reads from a snapshot (the simulator's stored book, #166). Pure.
pub(crate) fn ladders_from_snapshot(positions: &[Position]) -> HashMap<String, LadderTag> {
    positions
        .iter()
        .filter(|p| p.role.as_deref() == Some(LADDER_ROLE))
        .map(|p| (p.position_id.clone(), LadderTag { config_id: String::new(), rung: p.rung.clone(), deposit_tx: None }))
        .collect()
}

/// A ladder's net buys since its deposit, for the whole-stack cost basis (#167): Some((net
/// NOCK, net USDC)) when they can enter it, or the reason it is left out. `as_of` is the
/// opening basis's instant. Pure.
pub(crate) fn ladder_buys_for_basis(r: &LadderReading, as_of: DateTime<Utc>) -> std::result::Result<(f64, f64), String> {
    let sd = &r.since_deposit;
    if sd.status != "verified" {
        return Err(format!("{}: no verified deposit baseline ({})", r.position_id, sd.status));
    }
    let Some(at) = sd.deposited_at.as_deref().and_then(|t| DateTime::parse_from_rfc3339(t).ok()).map(|t| t.with_timezone(&Utc)) else {
        return Err(format!("{}: the deposit block's time was not read, so it cannot be placed against the opening basis", r.position_id));
    };
    if at <= as_of {
        return Err(format!(
            "{}: deposited at or before the opening basis ({}), so what it bought cannot be split from the opening figures",
            r.position_id,
            as_of.to_rfc3339()
        ));
    }
    let n = sd.net_nock_bought.as_deref().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    let u = sd.net_usdc_spent.as_deref().and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
    if !(n > 0.0 && u > 0.0) {
        return Err(format!("{}: no net buy since its deposit ({} NOCK for ${})", r.position_id, signed(n, 4), signed(u, 2)));
    }
    Ok((n, u))
}
