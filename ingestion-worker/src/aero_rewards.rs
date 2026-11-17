use crate::*;

// ============================================================
// AERO, the fourth pillar — issue #93
// ============================================================
// Every address, selector and behaviour here was verified by live read-only calls on
// Base on 2026-11-16, pinned to block 58077673 unless another block is named
// (ONCHAIN_INTEGRATION.md §9). The unit tests in tests_aero_rewards.rs decode the raw
// responses captured there.
//
// Which token. Two ERC-20s on Base carry the symbol "AERO". The gauges of the Aero app
// (app.aero.xyz) pay `0x3f5bf3ed…a4ee` (name "Aero"): in two real claims, one per gauge
// type, that token was minted to the claimer for exactly the amount `earned` reported
// one block earlier. The older `0x940181a9…8631` (name "Aerodrome") is a different
// token, is not what these gauges pay, and is not read.
//
// How much is unclaimed. Neither gauge has `rewardToken()` (it reverts on both).
//   - Basic (volatile pool) gauge: `earned(address)`.
//     A LEGACY Aerodrome gauge answers the same `earned(address)` but pays the older
//     token, and it does have `rewardToken()`. So `rewardToken()` is asked first: a
//     gauge that names any token other than AERO_BASE is "not read", never shown or
//     priced as AERO (gauge_pays_aero).
//   - CL gauge: `earned(address,uint256[])`, the account and the token ids it staked.
//     `earned(address,uint256)` of the older Slipstream gauge reverts here, and the
//     array form reverts with "NA" for an id the account has not staked.
//
// What it is worth. AERO/USD comes from the slot0 of an AERO/USDC pool
// (`aero_price_pool`), with the same stale-pool guard as the NOCK and ETH prices: a pool
// with no in-range liquidity cannot trade, so its price is frozen. Four other AERO/USDC
// pools on Base had zero in-range liquidity at the pinned block and showed $0.64 to
// $0.80 against a live $0.418.
//
// What NockNock does with it. Wallet AERO has value and is counted in the total, like
// gas. Unclaimed AERO is not, like uncollected LP fees: it is stated beside them. There
// is no target, no rebalancing and no advice about AERO: amounts only.

/// The AERO the Aero gauges pay on Base: name "Aero", symbol "AERO", 18 decimals.
pub(crate) const AERO_BASE: &str = "0x3f5bF3EdAc59d650D8fDf9657a71F2f904A3a4Ee";
pub(crate) const AERO_DECIMALS: i32 = 18;
/// Default AERO/USD source: the Aero CL200 AERO/USDC pool (token0 AERO, token1 USDC),
/// the deepest AERO/USDC pool found at the pinned block ($3.33M USDC + 4.17M AERO).
pub(crate) const DEFAULT_AERO_PRICE_POOL: &str = "0xd0acf293547457fb4fe9ee32ca93e89497626816";

pub(crate) const ERC20_BALANCE_OF: &str = "0x70a08231"; // balanceOf(address)
pub(crate) const GAUGE_EARNED_BASIC: &str = "0x008cc262"; // earned(address)
pub(crate) const GAUGE_EARNED_CL: &str = "0xd942be03"; // earned(address,uint256[])
pub(crate) const GAUGE_REWARD_TOKEN: &str = "0xf7c618c1"; // rewardToken()

/// The deviation matrix row for the AERO pillar: a measured share, status `no_target`.
pub(crate) const AERO_BUCKET_NAME: &str = "AERO (wallet)";

pub(crate) fn address_word(address: &str) -> String {
    format!("{:0>64}", address.trim_start_matches("0x").to_lowercase())
}

/// Calldata for `balanceOf(wallet)`. Pure.
pub(crate) fn erc20_balance_of_calldata(wallet: &str) -> String {
    format!("{ERC20_BALANCE_OF}{}", address_word(wallet))
}

/// Calldata for a basic gauge's `earned(wallet)`. Pure.
pub(crate) fn gauge_earned_basic_calldata(wallet: &str) -> String {
    format!("{GAUGE_EARNED_BASIC}{}", address_word(wallet))
}

/// Calldata for a CL gauge's `earned(wallet, [nft_id])`: the account word, the offset of
/// the dynamic array (0x40), its length (1) and the token id. Pure.
pub(crate) fn gauge_earned_cl_calldata(wallet: &str, nft_id: u64) -> String {
    format!("{GAUGE_EARNED_CL}{}{:064x}{:064x}{:064x}", address_word(wallet), 0x40, 1, nft_id)
}

/// One uint256 ABI word that must fit 128 bits (a token amount). Pure.
pub(crate) fn decode_amount_word(hex_str: &str, what: &str) -> Result<u128> {
    let clean = hex_str.trim_start_matches("0x");
    if clean.len() < 64 || !clean.is_char_boundary(64) {
        return Err(anyhow::anyhow!("{what} returned {} hex chars, expected 64", clean.len()));
    }
    if !clean[..32].bytes().all(|b| b == b'0') {
        return Err(anyhow::anyhow!("{what} does not fit 128 bits"));
    }
    parse_hex_word_u128(&clean[32..64]).map_err(|e| anyhow::anyhow!("{what} is not a hex quantity: {e}"))
}

/// A raw AERO amount in human units.
pub(crate) fn aero_units(raw: u128) -> f64 {
    raw as f64 / 10f64.powi(AERO_DECIMALS)
}

/// AERO/USD from an AERO/USDC pool's reads. Pure. The pool comes from config, so it is
/// checked before its price is used:
/// - its tokens must be AERO and USDC, in either order (a pool of another pair, or of
///   the other "AERO", would price the wrong thing);
/// - it must be initialized and have in-range liquidity, or its price is frozen.
pub(crate) fn aero_price_from_pool_reads(
    pool: &str,
    token0_hex: &str,
    token1_hex: &str,
    slot0_hex: &str,
    liquidity_hex: &str,
) -> Result<f64> {
    let token0 = address_from_word(token0_hex).map_err(|e| anyhow::anyhow!("AERO price pool {pool} token0(): {e}"))?;
    let token1 = address_from_word(token1_hex).map_err(|e| anyhow::anyhow!("AERO price pool {pool} token1(): {e}"))?;
    let is = |a: &str, b: &str| a.eq_ignore_ascii_case(b);
    let aero_is_token0 = if is(&token0, AERO_BASE) && is(&token1, USDC_BASE) {
        true
    } else if is(&token0, USDC_BASE) && is(&token1, AERO_BASE) {
        false
    } else {
        return Err(anyhow::anyhow!(
            "Configuration error: aero_price_pool {pool} is not an AERO/USDC pool (token0 {token0}, token1 {token1}; AERO is {AERO_BASE}). Set aero_price_pool to an AERO/USDC pool"
        ));
    };
    let slot0 = slot0_hex.trim_start_matches("0x");
    if slot0.len() < 64 {
        return Err(anyhow::anyhow!("AERO price pool {pool} slot0() returned {} hex chars, expected >= 64", slot0.len()));
    }
    let sqrt_price_x96 = U256::from_hex_be(&slot0[..64])?;
    if sqrt_price_x96 == U256(0, 0) {
        return Err(anyhow::anyhow!("AERO price pool {pool} is not initialized (sqrtPriceX96 is zero)"));
    }
    let liquidity = decode_amount_word(liquidity_hex, "AERO price pool liquidity()")?;
    if liquidity == 0 {
        return Err(anyhow::anyhow!(
            "Stale AERO/USD price: price pool {pool} has zero in-range liquidity, so its price is frozen. \
             Set aero_price_pool to an AERO/USDC pool with active liquidity."
        ));
    }
    let raw = sqrt_price_x96_to_raw(sqrt_price_x96); // token1_raw per token0_raw
    let scale = 10f64.powi(AERO_DECIMALS - 6);
    // token0 AERO: raw = USDC_raw per AERO_raw. token0 USDC: raw = AERO_raw per USDC_raw.
    let price = if aero_is_token0 { raw * scale } else { scale / raw };
    if price.is_finite() && price > 0.0 {
        Ok(price)
    } else {
        Err(anyhow::anyhow!("AERO price pool {pool} gave no usable price"))
    }
}

/// The wallet's AERO balance is counted in the total, so its read is required: a failed
/// read fails the cycle rather than counting AERO as zero (as for gas, issue #94). Pure.
pub(crate) fn require_wallet_aero(balance: Option<f64>) -> Result<f64> {
    balance.ok_or_else(|| {
        anyhow::anyhow!(
            "Failed to read wallet AERO balance: AERO is counted in the portfolio total, so the cycle fails rather than counting it as 0"
        )
    })
}

/// Required versus optional, decided as #94 did for gas. Pure.
/// - The wallet holds AERO: its value is in the total, so the price is required and a
///   failed read fails the cycle (never AERO at $0, never a stale price).
/// - The wallet holds none: the price only values unclaimed AERO, which is not in the
///   total. A failed read degrades: no price, with the reason, and the cycle goes on.
pub(crate) fn resolve_aero_price(wallet_aero: f64, price: Result<f64>) -> Result<(Option<f64>, Option<String>)> {
    match price {
        Ok(p) => Ok((Some(p), None)),
        Err(e) if wallet_aero > 0.0 => Err(anyhow::anyhow!(
            "Failed to read the AERO/USD price ({e}): the wallet holds {wallet_aero:.6} AERO, which is counted in the portfolio total, so the cycle fails rather than counting it as 0"
        )),
        Err(e) => Ok((None, Some(format!("AERO/USD price unavailable: {e}")))),
    }
}

/// Does this gauge pay AERO? Decided from its `rewardToken()` eth_call before `earned`
/// is trusted. Pure. Err is the reason the gauge's figure must not be read as AERO.
/// - It reverted: the gauge has no `rewardToken()`. That is the Aero app's gauge (both
///   of its types revert here), which was shown to pay AERO_BASE by what real claims
///   minted (ONCHAIN_INTEGRATION.md §9). Proceed.
/// - It returned AERO_BASE: proceed.
/// - It returned any other token: a different gauge. A legacy Aerodrome gauge answers
///   `earned(address)` too, but in the older token `0x940181a9…8631`; publishing that
///   figure as AERO, priced from the AERO pool, would be a wrong number.
/// - The node did not answer, or the answer is not an address: not known, so not read.
pub(crate) fn gauge_pays_aero(gauge: &str, reward_token_call: &Result<Option<String>>) -> std::result::Result<(), String> {
    match reward_token_call {
        Ok(None) => Ok(()),
        Ok(Some(hex)) => match address_from_word(hex) {
            Ok(token) if token.eq_ignore_ascii_case(AERO_BASE) => Ok(()),
            Ok(token) => Err(format!(
                "gauge {gauge} pays {token} (its rewardToken()), not AERO {}",
                AERO_BASE.to_lowercase()
            )),
            Err(e) => Err(format!("gauge {gauge} rewardToken() is not an address: {e}")),
        },
        Err(e) => Err(format!("gauge {gauge} rewardToken() read failed: {e}")),
    }
}

/// The entry for a gauge whose figure is not read as AERO (see gauge_pays_aero). Pure.
pub(crate) fn unclaimed_not_read(position_id: &str, gauge: &str, note: String) -> AeroUnclaimed {
    AeroUnclaimed { position_id: position_id.to_string(), gauge: Some(gauge.to_string()), amount: None, note: Some(note) }
}

/// One gauge's answer to `earned`, as an entry for the cycle. Pure. `call` is the
/// outcome of the eth_call: Ok(Some(hex)) = returned data, Ok(None) = it reverted,
/// Err = the node did not answer. Only returned data is an amount; a revert or a failed
/// read is "not read" with its reason, never 0.
pub(crate) fn unclaimed_from_earned_call(position_id: &str, gauge: &str, call: &Result<Option<String>>) -> AeroUnclaimed {
    let (amount, note) = match call {
        Ok(Some(hex)) => match decode_amount_word(hex, "earned()") {
            Ok(raw) => (Some(aero_units(raw)), None),
            Err(e) => (None, Some(format!("gauge {gauge} {e}"))),
        },
        Ok(None) => (None, Some(format!("gauge {gauge} earned() reverted"))),
        Err(e) => (None, Some(format!("gauge {gauge} earned() read failed: {e}"))),
    };
    AeroUnclaimed { position_id: position_id.to_string(), gauge: Some(gauge.to_string()), amount, note }
}

/// The entry for an Aero basic position with no `gauge_address`: nothing can be read,
/// and that is said rather than shown as 0. Pure.
pub(crate) fn unclaimed_without_gauge(position_id: &str) -> AeroUnclaimed {
    AeroUnclaimed {
        position_id: position_id.to_string(),
        gauge: None,
        amount: None,
        note: Some("no gauge_address is configured for this position".to_string()),
    }
}

/// The AERO pillar from one cycle's reads. Pure.
/// - A zero amount is worth $0 whatever the price, so it needs none.
/// - The unclaimed total is stated only when every entry has an amount. If any gauge
///   read failed, or a position has no gauge to read (an Aero basic pool without
///   `gauge_address`), the total is None: a partial sum would understate it, and a 0
///   would say "nothing unclaimed" about something that was never read. The note names
///   each such position. Some(0.0) therefore means either that every gauge answered 0
///   or that no tracked position earns AERO (no entries).
pub(crate) fn aero_pillar(reads: &AeroReads) -> AeroPillar {
    let price = reads.price_usd.filter(|p| p.is_finite() && *p > 0.0);
    let usd = |amount: Option<f64>| match (amount, price) {
        (Some(a), _) if a == 0.0 => Some(0.0),
        (Some(a), Some(p)) => Some(a * p),
        _ => None,
    };
    let unclaimed_amount = reads.unclaimed.as_ref().and_then(|entries| {
        let mut sum = 0.0;
        for e in entries {
            sum += e.amount?;
        }
        Some(sum)
    });
    let mut notes: Vec<String> = Vec::new();
    if price.is_none() {
        if let Some(n) = reads.price_note.as_ref().filter(|n| !n.trim().is_empty()) {
            notes.push(n.clone());
        }
    }
    for e in reads.unclaimed.iter().flatten().filter(|e| e.amount.is_none()) {
        notes.push(format!(
            "unclaimed AERO not read for {} ({})",
            e.position_id,
            e.note.as_deref().unwrap_or("no reason recorded")
        ));
    }
    AeroPillar {
        wallet_amount: reads.wallet_aero,
        wallet_usd: usd(reads.wallet_aero),
        unclaimed_amount,
        unclaimed_usd: usd(unclaimed_amount),
        price_usd: price,
        note: if notes.is_empty() { None } else { Some(notes.join("; ")) },
    }
}

/// The three optional `positions[]` fields for one position's entry. Pure.
pub(crate) fn aero_position_fields(entry: &AeroUnclaimed, price_usd: Option<f64>) -> (Option<String>, Option<String>, Option<String>) {
    match entry.amount {
        Some(a) => {
            let usd = if a == 0.0 {
                Some(0.0)
            } else {
                price_usd.filter(|p| p.is_finite() && *p > 0.0).map(|p| a * p)
            };
            (Some(format!("{:.6}", a)), usd.map(|u| format!("{:.2}", u)), None)
        }
        None => (None, None, Some(entry.note.clone().unwrap_or_else(|| "not read".to_string()))),
    }
}

/// One position's AERO statement, from the same strings its snapshot row carries. Pure.
/// Amounts only: it never says to claim, sell, hold or accumulate.
pub(crate) fn aero_statement(amount: Option<&str>, usd: Option<&str>, note: Option<&str>) -> Option<String> {
    match (amount, usd, note.filter(|n| !n.trim().is_empty())) {
        (Some(a), Some(u), _) => Some(format!("unclaimed AERO {a} (${u})")),
        (Some(a), None, _) => Some(format!("unclaimed AERO {a} (USD value unavailable)")),
        (None, _, Some(n)) => Some(format!("unclaimed AERO not read ({n})")),
        (None, _, None) => None,
    }
}

/// The statement for a position, straight from the cycle's reads (velocity panel). Pure.
pub(crate) fn aero_statement_for(reads: &AeroReads, position_id: &str) -> Option<String> {
    let entry = reads.unclaimed.as_ref()?.iter().find(|e| e.position_id == position_id)?;
    let (amount, usd, note) = aero_position_fields(entry, reads.price_usd);
    aero_statement(amount.as_deref(), usd.as_deref(), note.as_deref())
}

/// Stamp each snapshot position with its unclaimed AERO, or with the reason it was not
/// read, and with the same read as its collectible reward (#142). Positions without an entry (not an Aero position, or an unstaked CL NFT) are
/// left exactly as built. Run before build_contract_fees, which prints the statement.
pub(crate) fn apply_aero_rewards(positions: &mut [Position], reads: &AeroReads) {
    for entry in reads.unclaimed.iter().flatten() {
        if let Some(p) = positions.iter_mut().find(|p| p.position_id == entry.position_id) {
            let (amount, usd, note) = aero_position_fields(entry, reads.price_usd);
            p.unclaimed_aero = amount;
            p.unclaimed_aero_usd = usd;
            p.unclaimed_aero_note = note;
            // The same read, structured as collectible earnings (#142).
            p.collectible_reward = Some(collectible_reward(entry, reads.price_usd));
        }
    }
}

/// A velocity/fee row's note. Pure.
/// - Fees not LP-claimable: the reason, then the position's AERO statement if it has one.
/// - Fees LP-claimable: never an AERO statement. That row carries a fee action
///   (claim_now, accumulate), and "unclaimed AERO" beside it would read as advice to
///   claim or accumulate AERO. The amount stays on the position and the pillar lines.
pub(crate) fn fee_row_note(lp_claimable: bool, fees_note: Option<String>, aero_statement: Option<String>) -> Option<String> {
    match (aero_statement, lp_claimable) {
        (Some(aero), false) => join_reason_and_aero(Some(fees_note_text(fees_note.as_deref())), Some(aero)),
        (_, _) => fees_note,
    }
}

/// A fee row's reason followed by the position's AERO statement, when it has one.
pub(crate) fn join_reason_and_aero(reason: Option<String>, aero: Option<String>) -> Option<String> {
    match (reason, aero) {
        (Some(r), Some(a)) => Some(format!("{r}; {a}")),
        (r, a) => r.or(a),
    }
}

/// The wallet's AERO row for `wallet_balances`. USD is exact for a zero balance and
/// otherwise needs the price, which is a required read while the wallet holds AERO.
pub(crate) fn aero_wallet_balance(wallet: &str, amount: f64, price_usd: Option<f64>, now: &str) -> WalletBalance {
    WalletBalance {
        wallet_address: wallet.to_string(),
        token_symbol: "AERO".to_string(),
        token_address: AERO_BASE.to_string(),
        amount: format!("{:.6}", amount),
        amount_usd: format!("{:.2}", amount * price_usd.unwrap_or(0.0)),
        updated_at: now.to_string(),
    }
}

/// Harvest-ledger unit price including AERO (the other tokens as harvest_unit_price). Pure.
pub(crate) fn harvest_unit_price_with_aero<'a>(
    symbol: &str,
    nock: Option<&'a (f64, String)>,
    eth: Option<&'a (f64, String)>,
    aero: Option<&'a (f64, String)>,
) -> Option<(f64, &'a str)> {
    match symbol {
        "AERO" => aero.map(|(p, s)| (*p, s.as_str())),
        _ => harvest_unit_price(symbol, nock, eth),
    }
}

/// Koinly description for a ledger row. AERO paid by a gauge is a reward, not a swap
/// fee; every other token keeps the fee-harvest wording. Pure.
pub(crate) fn harvest_row_description(token_symbol: &str, platform: &str) -> String {
    if token_symbol == "AERO" {
        format!("NockNock AERO reward claim ({platform})")
    } else {
        format!("NockNock fee harvest ({platform})")
    }
}

// ------------------------------------------------------------------
// Live reads
// ------------------------------------------------------------------

async fn eth_call_hex(http_client: &Client, rpc_url: &str, to: &str, data: &str, block_tag: &str, id: u64) -> Option<String> {
    let payload = json!({
        "jsonrpc": "2.0", "method": "eth_call", "id": id,
        "params": [{ "to": to, "data": data }, block_tag]
    });
    post_rpc_with_retry(http_client, rpc_url, &payload, 2)
        .await
        .and_then(|r| r.get("result").and_then(|v| v.as_str()).map(|s| s.to_string()))
}

/// AERO/USD from the price pool at a block tag ("latest" or a hex block number; a past
/// block needs an archive RPC). Err carries the reason: a failed read, a pool that is
/// not AERO/USDC, or one with no in-range liquidity at that block.
pub(crate) async fn fetch_aero_price_usd_at(http_client: &Client, rpc_url: &str, pool: &str, block_tag: &str) -> Result<f64> {
    let read = |data: &'static str, id: u64, what: &'static str| async move {
        eth_call_hex(http_client, rpc_url, pool, data, block_tag, id)
            .await
            .ok_or_else(|| anyhow::anyhow!("AERO price pool {pool} {what} read failed"))
    };
    let token0 = read(CL_POOL_TOKEN0, 110, "token0()").await?;
    let token1 = read(CL_POOL_TOKEN1, 111, "token1()").await?;
    let slot0 = read(V3_SLOT0, 112, "slot0()").await?;
    let liquidity = read(V3_LIQUIDITY, 113, "liquidity()").await?;
    aero_price_from_pool_reads(pool, &token0, &token1, &slot0, &liquidity)
}

/// One cycle's AERO reads (issue #93): the wallet balance (required), the unclaimed
/// AERO of every tracked Aero position that has a gauge (optional: a failed read is
/// stated on that position, never 0; a gauge that names another reward token is not
/// read as AERO), and the AERO/USD price (required only while the wallet holds AERO).
pub(crate) async fn fetch_aero_reads(
    http_client: &Client,
    rpc_url: &str,
    wallet: &str,
    price_pool: &str,
    positions: &[PositionConfig],
    position_facts: &HashMap<String, PositionFacts>,
) -> Result<AeroReads> {
    let balance = eth_call_hex(http_client, rpc_url, AERO_BASE, &erc20_balance_of_calldata(wallet), "latest", 100)
        .await
        .and_then(|hex| decode_amount_word(&hex, "AERO balanceOf()").ok())
        .map(aero_units);
    let wallet_aero = require_wallet_aero(balance)?;

    let mut unclaimed: Vec<AeroUnclaimed> = Vec::new();
    for cfg in positions {
        let position_id = position_key(cfg);
        match aero_reward_source(cfg, position_facts.get(&position_id)) {
            AeroRewardSource::NotEarning => {}
            AeroRewardSource::GaugeNotConfigured => unclaimed.push(unclaimed_without_gauge(&position_id)),
            AeroRewardSource::Gauge { gauge, calldata } => {
                // Which token the gauge pays, before its `earned` is read as AERO.
                let token_payload = json!({
                    "jsonrpc": "2.0", "method": "eth_call", "id": 102,
                    "params": [{ "to": gauge, "data": GAUGE_REWARD_TOKEN }, "latest"]
                });
                let token_call = eth_call_allow_revert(http_client, rpc_url, &token_payload).await;
                let entry = match gauge_pays_aero(&gauge, &token_call) {
                    Err(note) => unclaimed_not_read(&position_id, &gauge, note),
                    Ok(()) => {
                        let payload = json!({
                            "jsonrpc": "2.0", "method": "eth_call", "id": 101,
                            "params": [{ "to": gauge, "data": calldata(wallet) }, "latest"]
                        });
                        let call = eth_call_allow_revert(http_client, rpc_url, &payload).await;
                        unclaimed_from_earned_call(&position_id, &gauge, &call)
                    }
                };
                if let Some(note) = entry.note.as_ref() {
                    tracing::warn!("position {position_id}: unclaimed AERO not read: {note}");
                }
                unclaimed.push(entry);
            }
        }
    }

    let price = fetch_aero_price_usd_at(http_client, rpc_url, price_pool, "latest").await;
    let (price_usd, price_note) = resolve_aero_price(wallet_aero, price)?;
    if let Some(note) = price_note.as_ref() {
        tracing::warn!("{note}");
    }
    Ok(AeroReads { wallet_aero: Some(wallet_aero), price_usd, price_note, unclaimed: Some(unclaimed) })
}
