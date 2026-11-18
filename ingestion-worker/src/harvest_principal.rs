//! Principal in a harvest receipt (#156).
//!
//! A transaction that removes liquidity and collects in one step (the Uniswap and Aero
//! apps' "remove liquidity": `decreaseLiquidity` + `collect` in one multicall) pays the
//! wallet the position's principal together with its fees. `record-harvest` books what
//! the wallet received, so the principal has to come out first, or returned liquidity
//! is booked as fee income.
//!
//! What can be split, from the receipt alone:
//! - A `DecreaseLiquidity(tokenId, liquidity, amount0, amount1)` emitted by a known
//!   position manager (the Uniswap v3 NPM, the Aero CL position manager). Its amounts are
//!   exactly the principal the decrease released. The position's token0/token1 are read
//!   from `positions(tokenId)` at the block before the transaction (the NFT is often
//!   burned in the same transaction), by the handler.
//!
//! What cannot, and is refused with nothing recorded:
//! - A Uniswap v4 `ModifyLiquidity` with a negative `liquidityDelta`: the event carries
//!   no token amounts. (A v4 fee claim is a zero-delta decrease, which is not refused.)
//! - An Aero basic (or v2-style) pool `Burn`: full-range liquidity returned.
//! - A concentrated-liquidity pool `Burn` that releases tokens to an owner that is not a
//!   known position manager: there is no `DecreaseLiquidity` to read it from.
//! - A decrease whose principal is not collected in the same transaction, or is
//!   collected to someone other than the wallet, the position manager or the contract the
//!   wallet called.
//! - Principal larger than what the wallet received of that token, including principal
//!   in a token NockNock does not book (it may have been swapped into one it does).
//! - A collect from a position that held no liquidity just before the transaction: it
//!   can only be paying out principal decreased earlier.
//!
//! Known limit: part of a position's liquidity decreased in one transaction and collected
//! in a later one while the position still holds liquidity. The later collect carries the
//! earlier principal, its receipt has no DecreaseLiquidity, and fees owed look the same,
//! so it is booked in full. Both apps decrease and collect in one transaction.
//!
//! Event topics were computed with keccak256 (viem) and checked against live Base logs on
//! 2026-11-18: tx 0x06515a23…b778 (Uniswap v3 NPM) and tx 0x7ba5de12…39cc (Aero CL
//! position manager) carry the same DecreaseLiquidity / Collect / pool Burn topics.
use crate::*;

/// `DecreaseLiquidity(uint256 indexed tokenId, uint128 liquidity, uint256 amount0, uint256 amount1)`,
/// the same on the Uniswap v3 NPM and the Aero CL (Slipstream) position manager.
pub(crate) const DECREASE_LIQUIDITY_TOPIC: &str =
    "0x26f6a048ee9138f2c0ce266f322cb99228e8d619ae2bff30c67f8dcf9d2377b4";
/// `Collect(uint256 indexed tokenId, address recipient, uint256 amount0, uint256 amount1)`,
/// the same on both position managers. In the Uniswap v3 NPM a zero recipient means the
/// manager itself (it then unwraps or sweeps to the caller).
pub(crate) const COLLECT_TOPIC: &str =
    "0x40d0efd1a53d60ecbf40971b9daf7dc90178c3aadc7aab1765632738fa8b8f01";
/// Uniswap v3 / Slipstream pool
/// `Burn(address indexed owner, int24 indexed tickLower, int24 indexed tickUpper, uint128 amount, uint256 amount0, uint256 amount1)`.
pub(crate) const CL_POOL_BURN_TOPIC: &str =
    "0x0c396cd989a39f4459b5fa1aed6a9a8dcdbc45908acfd67e028cd568da98982c";
/// Aero basic pool `Burn(address indexed sender, address indexed to, uint256 amount0, uint256 amount1)`.
pub(crate) const AERO_BASIC_BURN_TOPIC: &str =
    "0x5d624aa9c148153ab3446c1b154f660ee7701e549fe9b62dab7171b1c80e6fa2";
/// Uniswap v2-style pool `Burn(address indexed sender, uint256 amount0, uint256 amount1, address indexed to)`.
pub(crate) const V2_POOL_BURN_TOPIC: &str =
    "0xdccd412f0b1252819cb1fd330b93224ca42612892bb3f4f789976e6d81936496";
/// Uniswap v4 PoolManager
/// `ModifyLiquidity(bytes32 indexed id, address indexed sender, int24 tickLower, int24 tickUpper, int256 liquidityDelta, bytes32 salt)`.
pub(crate) const V4_MODIFY_LIQUIDITY_TOPIC: &str =
    "0xf208f4912782fd25c7f114ca3723a2d5dd6f3bcc3ac8db5af63baa85f711d5ec";

/// Position managers whose DecreaseLiquidity amounts are read as principal. A manager
/// that is not listed is not guessed at: its pool Burn is refused instead.
pub(crate) const PRINCIPAL_POSITION_MANAGERS: [&str; 2] = [UNISWAP_V3_NPM_BASE, AERO_CL_POSITION_MANAGER_BASE];

fn is_principal_manager(addr: &str) -> bool {
    let a = addr.trim().trim_start_matches("0x");
    PRINCIPAL_POSITION_MANAGERS
        .iter()
        .any(|m| a.eq_ignore_ascii_case(m.trim_start_matches("0x")))
}

/// Liquidity removed through a known position manager: the principal it released.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LiquidityDecrease {
    /// The position manager that emitted the event (lowercase, 0x-prefixed).
    pub(crate) manager: String,
    /// The position NFT's id, as the 64-hex-digit topic word (no 0x).
    pub(crate) token_id_word: String,
    pub(crate) amount0: u128,
    pub(crate) amount1: u128,
}

/// A collect through a known position manager: what it paid out, and to whom.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct LiquidityCollect {
    pub(crate) manager: String,
    pub(crate) token_id_word: String,
    /// Lowercase, 0x-prefixed.
    pub(crate) recipient: String,
    pub(crate) amount0: u128,
    pub(crate) amount1: u128,
}

/// The decreases and collects of known position managers in one receipt.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct ReceiptLiquidity {
    pub(crate) decreases: Vec<LiquidityDecrease>,
    pub(crate) collects: Vec<LiquidityCollect>,
}

/// Why a receipt's principal cannot be split from its fees. Nothing is recorded.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PrincipalRefusal {
    pub(crate) code: &'static str,
    pub(crate) detail: String,
}

impl PrincipalRefusal {
    fn not_separable(detail: String) -> Self {
        PrincipalRefusal { code: "PRINCIPAL_NOT_SEPARABLE", detail }
    }
}

fn data_words(log: &serde_json::Value) -> Vec<String> {
    let data = log.get("data").and_then(|d| d.as_str()).unwrap_or("").trim_start_matches("0x");
    if data.len() % 64 != 0 {
        return Vec::new();
    }
    (0..data.len() / 64).map(|i| data[i * 64..(i + 1) * 64].to_lowercase()).collect()
}

/// A uint256 word that must fit a u128 (every token amount NockNock books does).
fn word_u128(word: &str) -> Option<u128> {
    let w = word.trim_start_matches("0x");
    if w.len() != 64 || !w[..32].chars().all(|c| c == '0') {
        return None;
    }
    u128::from_str_radix(&w[32..], 16).ok()
}

fn is_zero_word(word: &str) -> bool {
    word.trim_start_matches("0x").chars().all(|c| c == '0')
}

fn token_id_topic(topics: &[serde_json::Value]) -> Option<String> {
    topics
        .get(1)
        .and_then(|t| t.as_str())
        .map(|t| t.trim_start_matches("0x").to_lowercase())
        .filter(|t| t.len() == 64)
}

/// Scan a receipt for liquidity returned to its owner. Returns the decreases and collects
/// of known position managers (no decrease: an ordinary fee claim), or a refusal when the
/// receipt returns principal that cannot be read from it. Pure.
pub(crate) fn scan_liquidity_decreases(receipt: &serde_json::Value) -> Result<ReceiptLiquidity, PrincipalRefusal> {
    let mut out = ReceiptLiquidity::default();
    let Some(logs) = receipt.get("logs").and_then(|l| l.as_array()) else {
        return Ok(out);
    };
    for log in logs {
        let Some(topics) = log.get("topics").and_then(|t| t.as_array()) else {
            continue;
        };
        let Some(t0) = topics.first().and_then(|t| t.as_str()) else {
            continue;
        };
        let address = log.get("address").and_then(|a| a.as_str()).unwrap_or("").to_lowercase();
        let words = data_words(log);

        if t0.eq_ignore_ascii_case(DECREASE_LIQUIDITY_TOPIC) {
            if !is_principal_manager(&address) {
                // A DecreaseLiquidity from another contract says nothing on its own; the
                // pool Burn it causes is what is checked below.
                continue;
            }
            let (Some(token_id_word), true) = (token_id_topic(topics), words.len() == 3) else {
                return Err(PrincipalRefusal::not_separable(format!(
                    "a DecreaseLiquidity event from position manager {address} could not be read"
                )));
            };
            let (Some(amount0), Some(amount1)) = (word_u128(&words[1]), word_u128(&words[2])) else {
                return Err(PrincipalRefusal::not_separable(format!(
                    "a DecreaseLiquidity amount from position manager {address} does not fit 128 bits"
                )));
            };
            out.decreases.push(LiquidityDecrease { manager: address, token_id_word, amount0, amount1 });
        } else if t0.eq_ignore_ascii_case(COLLECT_TOPIC) {
            if !is_principal_manager(&address) {
                continue;
            }
            let (Some(token_id_word), true) = (token_id_topic(topics), words.len() == 3) else {
                return Err(PrincipalRefusal::not_separable(format!(
                    "a Collect event from position manager {address} could not be read"
                )));
            };
            let (Some(amount0), Some(amount1)) = (word_u128(&words[1]), word_u128(&words[2])) else {
                return Err(PrincipalRefusal::not_separable(format!(
                    "a Collect amount from position manager {address} does not fit 128 bits"
                )));
            };
            let recipient = format!("0x{}", &words[0][24..]);
            out.collects.push(LiquidityCollect { manager: address, token_id_word, recipient, amount0, amount1 });
        } else if t0.eq_ignore_ascii_case(CL_POOL_BURN_TOPIC) {
            // Burn(owner, tickLower, tickUpper | amount, amount0, amount1). A Burn owned by
            // a known manager is read from that manager's DecreaseLiquidity instead.
            let owner = topics
                .get(1)
                .and_then(|t| t.as_str())
                .map(|t| t.trim_start_matches("0x"))
                .filter(|t| t.len() == 64)
                .map(|t| format!("0x{}", &t[24..]))
                .unwrap_or_default();
            let releases_tokens = words.len() != 3 || !is_zero_word(&words[1]) || !is_zero_word(&words[2]);
            if releases_tokens && !is_principal_manager(&owner) {
                return Err(PrincipalRefusal::not_separable(format!(
                    "the transaction removes concentrated liquidity from pool {address} for owner {owner}, which is not a position manager NockNock reads principal from"
                )));
            }
        } else if t0.eq_ignore_ascii_case(AERO_BASIC_BURN_TOPIC) || t0.eq_ignore_ascii_case(V2_POOL_BURN_TOPIC) {
            return Err(PrincipalRefusal::not_separable(format!(
                "the transaction removes full-range liquidity from pool {address}; what it returns is principal, not fees (an Aero basic pool's fees are claimed with claimFees)"
            )));
        } else if t0.eq_ignore_ascii_case(V4_MODIFY_LIQUIDITY_TOPIC) {
            // Data: tickLower, tickUpper, liquidityDelta (int256), salt. Negative = removed.
            let negative = words
                .get(2)
                .and_then(|w| w.chars().next())
                .is_some_and(|c| c.to_digit(16).is_some_and(|d| d >= 8));
            if words.len() != 4 || negative {
                return Err(PrincipalRefusal::not_separable(format!(
                    "the transaction removes Uniswap v4 liquidity (ModifyLiquidity from {address}); the event carries no token amounts, so principal cannot be told apart from fees"
                )));
            }
        }
    }
    Ok(out)
}

fn same_addr(a: &str, b: &str) -> bool {
    a.trim().trim_start_matches("0x").eq_ignore_ascii_case(b.trim().trim_start_matches("0x"))
}

/// Can a collect's recipient be where the wallet's inflow came from? The wallet itself;
/// the position manager (a zero recipient in the v3 NPM means the manager), which then
/// unwraps or sweeps; or the contract the wallet itself called (a router collecting on
/// its behalf). Anywhere else, the collect pays someone other than the wallet.
fn collect_reaches_wallet(c: &LiquidityCollect, receipt: &serde_json::Value, wallet: &str) -> bool {
    let zero = "0x0000000000000000000000000000000000000000";
    let field = |k: &str| receipt.get(k).and_then(|v| v.as_str()).unwrap_or("").to_string();
    let wallet_called = same_addr(&field("from"), wallet) && !field("to").is_empty();
    same_addr(&c.recipient, wallet)
        || same_addr(&c.recipient, &c.manager)
        || same_addr(&c.recipient, zero)
        || (wallet_called && same_addr(&c.recipient, &field("to")))
}

/// Every decrease's principal must be collected in the same transaction, towards the
/// wallet: one or more Collects of the same position, all of them reaching the wallet,
/// paying at least the principal. Otherwise the principal was left owed or paid
/// elsewhere, and taking it out of the wallet's inflow would take it out of fees. Pure.
pub(crate) fn check_decreases_collected(
    liq: &ReceiptLiquidity,
    receipt: &serde_json::Value,
    wallet: &str,
) -> Result<(), PrincipalRefusal> {
    let mut seen: Vec<(&str, &str)> = Vec::new();
    for d in &liq.decreases {
        let key = (d.manager.as_str(), d.token_id_word.as_str());
        if seen.contains(&key) {
            continue;
        }
        seen.push(key);
        let same = |m: &str, id: &str| same_addr(m, key.0) && id == key.1;
        let (mut p0, mut p1) = (0u128, 0u128);
        for x in liq.decreases.iter().filter(|x| same(&x.manager, &x.token_id_word)) {
            p0 = p0.saturating_add(x.amount0);
            p1 = p1.saturating_add(x.amount1);
        }
        let collects: Vec<&LiquidityCollect> = liq.collects.iter().filter(|c| same(&c.manager, &c.token_id_word)).collect();
        let position = format!("position 0x{} on {}", key.1.trim_start_matches('0'), key.0);
        if let Some(c) = collects.iter().find(|c| !collect_reaches_wallet(c, receipt, wallet)) {
            return Err(PrincipalRefusal::not_separable(format!(
                "the transaction collects {position} to {}, not to the wallet",
                c.recipient
            )));
        }
        let (c0, c1) = collects
            .iter()
            .fold((0u128, 0u128), |(a, b), c| (a.saturating_add(c.amount0), b.saturating_add(c.amount1)));
        if collects.is_empty() || c0 < p0 || c1 < p1 {
            return Err(PrincipalRefusal::not_separable(format!(
                "the transaction decreases {position} but does not collect its principal in the same transaction"
            )));
        }
    }
    Ok(())
}

/// Collects towards the wallet of positions this transaction did not decrease. The
/// handler reads each one's liquidity just before the transaction: a position that held
/// no liquidity can only be paying out principal decreased in an earlier transaction. Pure.
pub(crate) fn collect_only_positions(
    liq: &ReceiptLiquidity,
    receipt: &serde_json::Value,
    wallet: &str,
) -> Vec<(String, String)> {
    let mut out: Vec<(String, String)> = Vec::new();
    for c in &liq.collects {
        let decreased = liq
            .decreases
            .iter()
            .any(|d| same_addr(&d.manager, &c.manager) && d.token_id_word == c.token_id_word);
        let paid = c.amount0 > 0 || c.amount1 > 0;
        let key = (c.manager.clone(), c.token_id_word.clone());
        if !decreased && paid && collect_reaches_wallet(c, receipt, wallet) && !out.contains(&key) {
            out.push(key);
        }
    }
    out
}

/// What the receipt transfers to the wallet, per token (lowercase address), whether or
/// not NockNock books the token. Pure.
pub(crate) fn wallet_token_inflows(receipt: &serde_json::Value, wallet: &str) -> Vec<(String, u128)> {
    let mut out: Vec<(String, u128)> = Vec::new();
    let wallet_word = format!("000000000000000000000000{}", wallet.trim_start_matches("0x").to_lowercase());
    let Some(logs) = receipt.get("logs").and_then(|l| l.as_array()) else {
        return out;
    };
    for log in logs {
        let topics = log.get("topics").and_then(|t| t.as_array()).cloned().unwrap_or_default();
        let is_transfer = topics.len() == 3
            && topics[0].as_str().is_some_and(|t| t.eq_ignore_ascii_case(TRANSFER_EVENT_TOPIC))
            && topics[2].as_str().is_some_and(|t| t.trim_start_matches("0x").eq_ignore_ascii_case(&wallet_word));
        if !is_transfer {
            continue;
        }
        let words = data_words(log);
        let (Some(token), Some(amount)) = (
            log.get("address").and_then(|a| a.as_str()).map(|a| a.to_lowercase()),
            words.first().and_then(|w| word_u128(w)),
        ) else {
            continue;
        };
        match out.iter_mut().find(|(t, _)| *t == token) {
            Some((_, sum)) => *sum = sum.saturating_add(amount),
            None => out.push((token, amount)),
        }
    }
    out
}

/// Principal of one token, summed over the transaction's decreases.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct PrincipalAmount {
    /// Lowercase, 0x-prefixed token address.
    pub(crate) token_address: String,
    pub(crate) amount_raw: u128,
}

/// Pair each decrease's amounts with its position's tokens and sum them per token.
/// `pairs[i]` is decrease `i`'s (token0, token1). Pure.
pub(crate) fn principal_by_token(
    decreases: &[LiquidityDecrease],
    pairs: &[(String, String)],
) -> Result<Vec<PrincipalAmount>, PrincipalRefusal> {
    if decreases.len() != pairs.len() {
        return Err(PrincipalRefusal::not_separable(
            "the tokens of every decreased position could not be read".to_string(),
        ));
    }
    let mut out: Vec<PrincipalAmount> = Vec::new();
    for (d, (token0, token1)) in decreases.iter().zip(pairs) {
        for (token, amount) in [(token0, d.amount0), (token1, d.amount1)] {
            if amount == 0 {
                continue;
            }
            let token = format!("0x{}", token.trim().trim_start_matches("0x").to_lowercase());
            match out.iter_mut().find(|p| p.token_address == token) {
                Some(p) => {
                    p.amount_raw = p.amount_raw.checked_add(amount).ok_or_else(|| {
                        PrincipalRefusal::not_separable(format!("principal of {token} overflows 128 bits"))
                    })?
                }
                None => out.push(PrincipalAmount { token_address: token, amount_raw: amount }),
            }
        }
    }
    Ok(out)
}

/// What a receipt paid the wallet, split into fees (booked) and principal (left out).
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct FeeSplit {
    /// Ledger rows: what the wallet received less principal, rows that reach zero dropped.
    pub(crate) fees: Vec<ParsedHarvestTransfer>,
    /// Principal left out, per ledger row it was taken from (same token, symbol, decimals).
    pub(crate) principal: Vec<ParsedHarvestTransfer>,
}

fn same_token(a: &str, b: &str) -> bool {
    a.trim_start_matches("0x").eq_ignore_ascii_case(b.trim_start_matches("0x"))
}

/// Is this a token NockNock books (so its principal must be found in the wallet's inflow)?
fn is_booked_token(token: &str) -> bool {
    [USDC_BASE, NOCK_BASE, WETH_BASE, AERO_BASE].iter().any(|t| same_token(t, token))
}

/// Take principal out of the merged ledger rows (one per token, from
/// `merge_harvest_transfers`). WETH principal is taken from the WETH row first, then
/// from ETH unwrapped by a position manager (how the Uniswap app pays WETH out).
/// Principal of a token NockNock does not book has no row: it is left out like that
/// token, but only if the receipt shows at least that much of it transferred to the
/// wallet (`unbooked_inflows`, from `wallet_token_inflows`). Otherwise it was swapped,
/// perhaps into a booked token whose row it would inflate, and the split is refused.
/// Principal of a booked token that exceeds what the wallet received is refused. Pure.
pub(crate) fn subtract_principal(
    rows: Vec<ParsedHarvestTransfer>,
    principal: &[PrincipalAmount],
    unbooked_inflows: &[(String, u128)],
) -> Result<FeeSplit, PrincipalRefusal> {
    let mut amounts: Vec<u128> = Vec::with_capacity(rows.len());
    for r in &rows {
        amounts.push(r.amount_raw.parse::<u128>().map_err(|_| {
            PrincipalRefusal::not_separable(format!("the {} amount {} could not be read", r.token_symbol, r.amount_raw))
        })?);
    }
    let mut taken: Vec<u128> = vec![0; rows.len()];
    for p in principal {
        let mut candidates: Vec<usize> = (0..rows.len())
            .filter(|&i| same_token(&rows[i].token_address, &p.token_address))
            .collect();
        if same_token(&p.token_address, WETH_BASE) {
            candidates.extend((0..rows.len()).filter(|&i| same_token(&rows[i].token_address, NATIVE_ETH_LEDGER_ADDRESS)));
        }
        let mut remaining = p.amount_raw;
        for i in candidates {
            let take = remaining.min(amounts[i]);
            amounts[i] -= take;
            taken[i] += take;
            remaining -= take;
            if remaining == 0 {
                break;
            }
        }
        let received_unbooked = unbooked_inflows
            .iter()
            .find(|(t, _)| same_token(t, &p.token_address))
            .map(|(_, a)| *a)
            .unwrap_or(0);
        let covered = !is_booked_token(&p.token_address) && remaining <= received_unbooked;
        if remaining > 0 && !covered {
            return Err(PrincipalRefusal {
                code: "PRINCIPAL_EXCEEDS_INFLOW",
                detail: format!(
                    "the transaction removes more {} principal than it pays the wallet ({} raw units not received): it was paid elsewhere, swapped, or left uncollected, so the fees cannot be told apart",
                    p.token_address, remaining
                ),
            });
        }
    }
    let mut fees = Vec::new();
    let mut left_out = Vec::new();
    for (i, r) in rows.into_iter().enumerate() {
        if taken[i] > 0 {
            left_out.push(ParsedHarvestTransfer {
                amount_raw: taken[i].to_string(),
                amount_human: format_units(taken[i], r.decimals),
                ..r.clone()
            });
        }
        if amounts[i] > 0 {
            fees.push(ParsedHarvestTransfer {
                amount_raw: amounts[i].to_string(),
                amount_human: format_units(amounts[i], r.decimals),
                ..r
            });
        }
    }
    Ok(FeeSplit { fees, principal: left_out })
}

/// `positions(tokenId)` calldata for a decrease's position manager.
pub(crate) fn positions_calldata(d: &LiquidityDecrease) -> String {
    format!("{V3_POSITIONS}{}", d.token_id_word)
}

/// Block tag of the state just before the transaction's block: the position may be
/// burned in the transaction itself. None for block 0 or an unknown block.
pub(crate) fn block_before_tag(block_number: i64) -> Option<String> {
    (block_number > 0).then(|| format!("0x{:x}", block_number - 1))
}

/// A fee or reward claim only pays the wallet (#157). If the receipt shows the wallet
/// paying out an ERC-20 (a 3-topic `Transfer` from the wallet: a 4-topic ERC-721
/// `Transfer` is a position NFT moving, burned on withdrawal or staked), or the
/// transaction sent ETH, it is a swap, a deposit or a purchase, not a claim. Pure.
pub(crate) fn check_is_claim(receipt: &serde_json::Value, tx_value_wei: u128, wallet: &str) -> Result<(), PrincipalRefusal> {
    let not_a_claim = |detail: String| PrincipalRefusal { code: "NOT_A_CLAIM", detail };
    if tx_value_wei > 0 {
        return Err(not_a_claim(
            "the transaction sends ETH, so it is a purchase or a deposit, not a fee claim; a NOCK buy is recorded with record-accumulation".to_string(),
        ));
    }
    let wallet_word = format!("000000000000000000000000{}", wallet.trim_start_matches("0x").to_lowercase());
    let Some(logs) = receipt.get("logs").and_then(|l| l.as_array()) else {
        return Ok(());
    };
    for log in logs {
        let Some(topics) = log.get("topics").and_then(|t| t.as_array()) else {
            continue;
        };
        let pays_out = topics.len() == 3
            && topics[0].as_str().is_some_and(|t| t.eq_ignore_ascii_case(TRANSFER_EVENT_TOPIC))
            && topics[1].as_str().is_some_and(|t| t.trim_start_matches("0x").eq_ignore_ascii_case(&wallet_word));
        if pays_out {
            let token = log.get("address").and_then(|a| a.as_str()).unwrap_or("?");
            return Err(not_a_claim(format!(
                "the wallet pays out token {token} in this transaction, so it is a swap or a deposit, not a fee claim; a NOCK buy is recorded with record-accumulation"
            )));
        }
    }
    Ok(())
}

/// The `value` of a transaction from `eth_getTransactionByHash`, in wei. None when the
/// answer has no readable value. Pure.
pub(crate) fn tx_value_wei(tx: &serde_json::Value) -> Option<u128> {
    tx.get("value")
        .and_then(|v| v.as_str())
        .and_then(|v| u128::from_str_radix(v.trim_start_matches("0x"), 16).ok())
}
