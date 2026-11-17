use crate::*;

// ============================================================
// Aero concentrated-liquidity ("CL", Slipstream) positions — issue #91
// ============================================================
// Every address, selector and layout here was verified by live read-only calls on Base
// on 2026-11-16 (blocks 58061668, 58063800 and 58074373; ONCHAIN_INTEGRATION.md §8). The
// unit tests in tests_aero_cl.rs decode the raw responses captured at those blocks.
//
// A CL position is an NFT in Aero's Slipstream position manager. The layouts follow
// Uniswap v3 with three differences that matter here:
//   - `positions(uint256)` word 4 is the pool's `tickSpacing` (int24), not a fee tier;
//   - the pool's `slot0()` returns 6 words (no `feeProtocol`); words 0 and 1 are still
//     `sqrtPriceX96` and `tick`;
//   - the swap fee is dynamic and read from the pool's `fee()` (pips, 1e-6).
//
// Staking. The Aero app stakes the NFT in the pool's gauge on deposit, so `ownerOf` is
// the gauge. The gauge keeps a per-depositor set of token ids: `stakedContains(wallet,
// tokenId)` is the proof that the wallet staked it (the same account, and only that
// account, can `withdraw(tokenId)`). A position that is neither held by the wallet nor
// staked by it is somebody else's and fails the cycle.
//
// Fees. Swap fees on STAKED liquidity are paid to the gauge (`pool.gaugeFees()`), not to
// the LP: `collect` simulated from the gauge (the NFT's owner) returns (0, 0) and from
// the wallet reverts. The staker earns AERO emissions instead, read from the gauge by
// aero_rewards.rs (#93). So a staked position reports no fee figure and says why. An UNSTAKED
// position's fees are what `collect` simulated from the wallet returns; that read is
// required like the Uniswap fee reads, so a revert there fails the cycle.
//
// The pool. `pool_address` comes from config, so it is checked against the NFT before
// anything is valued at its price: the pool's `nft()` must be this position manager
// (which rules out a Uniswap v3 pool of the same pair and spacing, and a pool of the
// other Slipstream deployment), and its tokens and tick spacing must be the NFT's.

/// Config `protocol` value for an Aero concentrated-liquidity position.
pub(crate) const AERO_CL_PROTOCOL: &str = "aerodrome-cl";

pub(crate) const ERC721_OWNER_OF: &str = "0x6352211e"; // ownerOf(uint256)
pub(crate) const CL_POOL_TOKEN0: &str = "0x0dfe1681"; // token0()
pub(crate) const CL_POOL_TOKEN1: &str = "0xd21220a7"; // token1()
pub(crate) const CL_POOL_TICK_SPACING: &str = "0xd0c93a7c"; // tickSpacing()
pub(crate) const CL_POOL_FEE: &str = "0xddca3f43"; // fee()
pub(crate) const CL_POOL_GAUGE: &str = "0xa6f19c84"; // gauge()
pub(crate) const CL_POOL_NFT: &str = "0x47ccca02"; // nft(): the pool's position manager
pub(crate) const CL_GAUGE_STAKED_CONTAINS: &str = "0xc69deec5"; // stakedContains(address,uint256)
// Shared with Uniswap v3 (same selectors, verified on the Slipstream contracts):
// positions(uint256) V3_POSITIONS, slot0() V3_SLOT0, liquidity() V3_LIQUIDITY and
// collect((uint256,address,uint128,uint128)) via build_v3_collect_calldata.

/// What NockNock says about the fees of a position staked in its gauge. It starts with
/// "not LP-claimable" like AERO_NOT_LP_CLAIMABLE, which is what the MCP text keys on.
pub(crate) const AERO_CL_STAKED_FEES_NOTE: &str =
    "not LP-claimable (staked: swap fees on staked liquidity go to the gauge)";

/// Decoded Slipstream `positions(uint256)` return. Same 12 words as Uniswap v3 except
/// that word 4 carries the pool's tick spacing where v3 has the fee tier.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct SlipstreamPositionDecoded {
    pub(crate) token0: String,
    pub(crate) token1: String,
    pub(crate) tick_spacing: i32,
    pub(crate) tick_lower: i32,
    pub(crate) tick_upper: i32,
    pub(crate) liquidity: u128,
    /// Settled fees only; an unstaked position's unsettled fees come from `collect`.
    pub(crate) tokens_owed0: u128,
    pub(crate) tokens_owed1: u128,
}

/// Parse the Slipstream position manager's `positions(uint256)` return:
/// (nonce, operator, token0, token1, tickSpacing, tickLower, tickUpper, liquidity,
///  feeGrowthInside0Last, feeGrowthInside1Last, tokensOwed0, tokensOwed1).
/// The word layout is Uniswap v3's, so the v3 decoder does the work; only the meaning
/// of word 4 differs, and it is never reported as a fee tier.
pub(crate) fn decode_slipstream_positions_return(hex_str: &str) -> Result<SlipstreamPositionDecoded> {
    let v3 = decode_v3_positions_return(hex_str)?;
    let tick_spacing = i32::try_from(v3.fee)
        .ok()
        .filter(|ts| *ts > 0)
        .ok_or_else(|| anyhow::anyhow!("positions() tickSpacing word is not a positive int24: {}", v3.fee))?;
    Ok(SlipstreamPositionDecoded {
        token0: v3.token0,
        token1: v3.token1,
        tick_spacing,
        tick_lower: v3.tick_lower,
        tick_upper: v3.tick_upper,
        liquidity: v3.liquidity,
        tokens_owed0: v3.tokens_owed0,
        tokens_owed1: v3.tokens_owed1,
    })
}

/// Parse a Slipstream pool's `slot0()` return: (sqrtPriceX96, tick, observationIndex,
/// observationCardinality, observationCardinalityNext, unlocked). Six words: Uniswap
/// v3's `feeProtocol` is absent. Only the first two are used.
pub(crate) fn decode_slipstream_slot0(hex_str: &str) -> Result<(U256, i32)> {
    let clean = hex_str.trim_start_matches("0x");
    if clean.len() < 128 {
        return Err(anyhow::anyhow!("slot0() returned {} chars, expected >= 128", clean.len()));
    }
    let sqrt_price_x96 = U256::from_hex_be(&clean[..64])?;
    if sqrt_price_x96 == U256(0, 0) {
        return Err(anyhow::anyhow!("slot0() sqrtPriceX96 is zero: the pool is not initialized"));
    }
    Ok((sqrt_price_x96, parse_hex_word_i32(&clean[64..128])?))
}

/// The low 20 bytes of an ABI word, as a lowercase 0x address.
pub(crate) fn address_from_word(hex_str: &str) -> Result<String> {
    let clean = hex_str.trim_start_matches("0x");
    if clean.len() < 64 || !clean.is_char_boundary(64) {
        return Err(anyhow::anyhow!("address word returned {} chars, expected 64", clean.len()));
    }
    Ok(format!("0x{}", clean[24..64].to_lowercase()))
}

fn same_address(a: &str, b: &str) -> bool {
    a.trim_start_matches("0x").eq_ignore_ascii_case(b.trim_start_matches("0x"))
}

/// Who holds the position NFT, as far as `ownerOf` alone can say.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum NftHolder {
    /// The tracked wallet holds the NFT itself: the position is not staked.
    Wallet,
    /// The pool's gauge holds it. Whether the WALLET staked it is a separate read.
    Gauge(String),
}

/// Step 1 of the ownership check. Pure. The NFT must be held by the wallet or by the
/// pool's own gauge; anything else is not the operator's position. `configured_gauge`
/// is the optional `gauge_address` from config: when set it must be the pool's gauge.
pub(crate) fn classify_nft_holder(
    cfg_id: &str,
    nft_id: u64,
    wallet: &str,
    owner: &str,
    pool_gauge: &str,
    configured_gauge: Option<&str>,
) -> Result<NftHolder> {
    if let Some(cfg_gauge) = configured_gauge.filter(|g| !g.trim().is_empty()) {
        if !same_address(cfg_gauge, pool_gauge) {
            return Err(anyhow::anyhow!(
                "Configuration error: position {cfg_id}: gauge_address {cfg_gauge} is not the pool's gauge ({pool_gauge}); remove gauge_address or correct it"
            ));
        }
    }
    if same_address(owner, wallet) {
        return Ok(NftHolder::Wallet);
    }
    if same_address(pool_gauge, NATIVE_ETH) {
        return Err(anyhow::anyhow!(
            "position {cfg_id}: NFT #{nft_id} is held by {owner}, not by the tracked wallet {wallet}, and its pool has no gauge it could be staked in; it is not counted"
        ));
    }
    if same_address(owner, pool_gauge) {
        return Ok(NftHolder::Gauge(pool_gauge.to_lowercase()));
    }
    Err(anyhow::anyhow!(
        "position {cfg_id}: NFT #{nft_id} is held by {owner}, which is neither the tracked wallet {wallet} nor the pool's gauge {pool_gauge}; it is not counted"
    ))
}

/// Step 2 of the ownership check. Pure. `staked_contains_hex` is the gauge's answer to
/// `stakedContains(wallet, tokenId)`; only an explicit true proves the wallet staked it.
pub(crate) fn require_staked_by_wallet(
    cfg_id: &str,
    nft_id: u64,
    wallet: &str,
    gauge: &str,
    staked_contains_hex: &str,
) -> Result<()> {
    let clean = staked_contains_hex.trim_start_matches("0x");
    if clean.len() < 64 {
        return Err(anyhow::anyhow!(
            "position {cfg_id}: gauge {gauge} stakedContains() returned short output"
        ));
    }
    match parse_hex_word_u128(&clean[32..64]) {
        Ok(1) if clean[..32].bytes().all(|b| b == b'0') => Ok(()),
        Ok(0) if clean[..32].bytes().all(|b| b == b'0') => Err(anyhow::anyhow!(
            "position {cfg_id}: NFT #{nft_id} is staked in gauge {gauge}, but not by the tracked wallet {wallet} (stakedContains is false); it belongs to another account and is not counted"
        )),
        _ => Err(anyhow::anyhow!(
            "position {cfg_id}: gauge {gauge} stakedContains() returned a non-boolean word"
        )),
    }
}

/// The two uint256 words `collect` returns (amount0, amount1), in raw token units.
pub(crate) fn decode_collect_return(hex_str: &str) -> Result<(u128, u128)> {
    let clean = hex_str.trim_start_matches("0x");
    if clean.len() < 128 {
        return Err(anyhow::anyhow!("collect() returned {} chars, expected >= 128", clean.len()));
    }
    let word = |i: usize| -> Result<u128> {
        let w = &clean[i * 64..(i + 1) * 64];
        if !w[..32].bytes().all(|b| b == b'0') {
            return Err(anyhow::anyhow!("collect() amount does not fit 128 bits"));
        }
        parse_hex_word_u128(&w[32..])
    };
    Ok((word(0)?, word(1)?))
}

/// What one cycle learned about the position's fees. There is no "could not be read"
/// case: an unstaked position's `collect` simulation is a required read, and a revert
/// there fails the cycle (`collect_reverted_error`) rather than carrying a zero.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum AeroClFees {
    /// Unstaked: what `collect` simulated from the wallet would pay, in raw units.
    Claimable(u128, u128),
    /// Staked in the gauge: swap fees go to the gauge. The AERO earned instead is read
    /// separately (aero_rewards.rs, #93).
    StakedNotLpClaimable,
}

impl AeroClFees {
    /// The reason shown in place of a fee figure; None when the fees were read.
    pub(crate) fn note(&self) -> Option<&'static str> {
        match self {
            AeroClFees::Claimable(..) => None,
            AeroClFees::StakedNotLpClaimable => Some(AERO_CL_STAKED_FEES_NOTE),
        }
    }
}

/// The cycle error for an unstaked position whose `collect` simulation reverts. The
/// wallet holds the NFT, so the call should pay out; a revert means its fees cannot be
/// read, and an unread fee is never published as 0 (the Uniswap adapters fail the
/// cycle on a failed fee read in the same way). Pure.
pub(crate) fn collect_reverted_error(cfg_id: &str, nft_id: u64, npm: &str, wallet: &str) -> anyhow::Error {
    anyhow::anyhow!(
        "position {cfg_id}: collect() for NFT #{nft_id} on position manager {npm}, simulated from the tracked wallet {wallet} that holds it, reverted; its claimable fees cannot be read, so the cycle fails rather than report them as 0"
    )
}

/// The pool's answer to `nft()`, which must name the configured position manager.
/// `nft_call` is the outcome of the eth_call: Some(hex) = returned data, None = the
/// call reverted (a Uniswap v3 pool has no `nft()`). Pure.
pub(crate) fn pool_position_manager(cfg_id: &str, pool: &str, npm: &str, nft_call: Option<&str>) -> Result<String> {
    let not_a_pool = |why: &str| {
        anyhow::anyhow!(
            "Configuration error: position {cfg_id}: pool_address {pool} is not an Aero CL pool of position manager {npm} ({why}); correct pool_address"
        )
    };
    let hex = nft_call.ok_or_else(|| not_a_pool("its nft() call reverts, as on a Uniswap v3 pool"))?;
    address_from_word(hex).map_err(|_| not_a_pool("its nft() call returned no address"))
}

/// The pool named in config must be the NFT's pool, or the amounts would be computed
/// at another pool's price. Three things identify it: the pool's own position manager
/// (`nft()`) is the one the NFT lives on, and its tokens and tick spacing are the
/// NFT's. Tokens and spacing alone do not: the Uniswap v3 WETH/NOCK 1% pool also has
/// tick spacing 200, and the legacy Slipstream deployment has same-pair, same-spacing
/// pools of its own (verified, ONCHAIN_INTEGRATION.md §8). Pure.
#[allow(clippy::too_many_arguments)]
pub(crate) fn check_pool_is_the_nfts_pool(
    cfg: &PositionConfig,
    npm: &str,
    pool: &str,
    decoded: &SlipstreamPositionDecoded,
    pool_nft: &str,
    pool_token0: &str,
    pool_token1: &str,
    pool_tick_spacing: i32,
) -> Result<()> {
    if !same_address(pool_nft, npm) {
        return Err(anyhow::anyhow!(
            "Configuration error: position {}: pool_address {} is not a pool of position manager {} (its nft() is {}); NFT #{} cannot be valued at its price",
            cfg.id, pool, npm, pool_nft, cfg.nft_id
        ));
    }
    if !same_address(&decoded.token0, pool_token0)
        || !same_address(&decoded.token1, pool_token1)
        || decoded.tick_spacing != pool_tick_spacing
    {
        return Err(anyhow::anyhow!(
            "Configuration error: position {}: pool_address {} ({}/{} tick spacing {}) is not the pool of NFT #{} ({}/{} tick spacing {})",
            cfg.id, pool, pool_token0, pool_token1, pool_tick_spacing,
            cfg.nft_id, decoded.token0, decoded.token1, decoded.tick_spacing
        ));
    }
    Ok(())
}

/// Pure core of the Aero CL read: turn the decoded NFT, the pool's price and fee and
/// the custody result into a RawPosition and its side facts. Unit-testable. Checks
/// again that the pool is the NFT's pool, so no caller can skip it.
#[allow(clippy::too_many_arguments)]
pub(crate) fn aero_cl_position_from_reads(
    cfg: &PositionConfig,
    wallet: &str,
    npm: &str,
    pool: &str,
    decoded: &SlipstreamPositionDecoded,
    pool_nft: &str,
    pool_token0: &str,
    pool_token1: &str,
    pool_tick_spacing: i32,
    pool_fee_pips: u32,
    sqrt_price_x96: U256,
    tick: i32,
    staked_in_gauge: Option<String>,
    fees: AeroClFees,
) -> Result<(RawPosition, PositionFacts)> {
    check_pool_is_the_nfts_pool(cfg, npm, pool, decoded, pool_nft, pool_token0, pool_token1, pool_tick_spacing)?;
    if decoded.tick_lower >= decoded.tick_upper {
        return Err(anyhow::anyhow!(
            "position {}: NFT #{} decoded with tickLower {} >= tickUpper {}",
            cfg.id, cfg.nft_id, decoded.tick_lower, decoded.tick_upper
        ));
    }
    let token0 = token_info(&decoded.token0)?;
    let token1 = token_info(&decoded.token1)?;
    // Shared with Uniswap v3: amounts from liquidity, ticks and the pool's sqrt price.
    let (amount0_raw, amount1_raw) =
        concentrated_amounts_raw(decoded.liquidity, sqrt_price_x96, decoded.tick_lower, decoded.tick_upper);
    let dec0 = 10f64.powi(token0.decimals);
    let dec1 = 10f64.powi(token1.decimals);
    // A staked position has no LP-claimable fee: it is carried as 0 here AND flagged in
    // the facts, so no surface prints it as "$0 earned" (build_contract_fees,
    // PositionFacts, claims::aero_cl_mark_not_claimable).
    let (fee0_raw, fee1_raw) = match fees {
        AeroClFees::Claimable(f0, f1) => (f0, f1),
        AeroClFees::StakedNotLpClaimable => (0, 0),
    };
    let raw = RawPosition {
        position_id: position_key(cfg),
        network: cfg.network.clone(),
        protocol: AERO_CL_PROTOCOL.to_string(),
        wallet: wallet.to_string(),
        nft_id: cfg.nft_id,
        pool_id: pool.to_lowercase(),
        npm_address: Some(npm.to_string()),
        // pool.fee() is in pips (1e-6): 9700 = 0.97% = 97 bps. It is dynamic on Aero
        // pools, so this is the fee at this cycle's read.
        fee_tier_bps: pool_fee_pips / 100,
        token0,
        token1,
        liquidity: decoded.liquidity,
        sqrt_price_x96,
        tick,
        tick_lower: decoded.tick_lower,
        tick_upper: decoded.tick_upper,
        amount0: amount0_raw / dec0,
        amount1: amount1_raw / dec1,
        fee0: fee0_raw as f64 / dec0,
        fee1: fee1_raw as f64 / dec1,
        in_range: tick_in_range(tick, decoded.tick_lower, decoded.tick_upper),
        is_price_source: cfg.is_price_source,
    };
    let facts = PositionFacts {
        tick_spacing: Some(decoded.tick_spacing),
        staked_in_gauge,
        fees_note: fees.note().map(|n| n.to_string()),
    };
    Ok((raw, facts))
}

/// Read an Aero CL position (issue #91). Every read is required: a failed one fails the
/// cycle, as does a position the wallet neither holds nor staked, a pool that is not
/// the NFT's pool, and an unstaked position whose `collect` simulation reverts (its
/// fees cannot be read, and an unread fee is never published as zero).
/// Since #144 an NFT proven not to exist is `PositionRead::Closed` instead of an error.
pub(crate) async fn fetch_aero_cl_position(
    http_client: &Client,
    alchemy_url: &str,
    cfg: &PositionConfig,
    wallet: &str,
) -> Result<PositionRead<(RawPosition, PositionFacts)>> {
    let npm = cfg.npm_address.clone().filter(|a| !a.trim().is_empty()).ok_or_else(|| {
        anyhow::anyhow!("Configuration error: position {}: {AERO_CL_PROTOCOL} positions need npm_address (the Aero CL position manager)", cfg.id)
    })?;
    let pool = cfg.pool_address.clone().filter(|a| !a.trim().is_empty()).ok_or_else(|| {
        anyhow::anyhow!("Configuration error: position {}: {AERO_CL_PROTOCOL} positions need pool_address", cfg.id)
    })?;
    if cfg.nft_id == 0 {
        return Err(anyhow::anyhow!(
            "Configuration error: position {}: {AERO_CL_PROTOCOL} positions need nft_id (the position NFT's token id)",
            cfg.id
        ));
    }
    let read = |to: String, data: String, id: u64, what: &'static str| {
        let cfg_id = cfg.id.clone();
        async move {
            let payload = json!({
                "jsonrpc": "2.0", "method": "eth_call", "id": id,
                "params": [{ "to": to, "data": data }, "latest"]
            });
            post_rpc_with_retry(http_client, alchemy_url, &payload, 2)
                .await
                .and_then(|r| r.get("result").and_then(|v| v.as_str()).map(|s| s.to_string()))
                .ok_or_else(|| anyhow::anyhow!("position {}: Aero CL {} read failed", cfg_id, what))
        }
    };
    let ctx = |e: anyhow::Error, what: &str| anyhow::anyhow!("position {}: Aero CL {}: {}", cfg.id, what, e);
    let id_word = format!("{:064x}", cfg.nft_id);
    let wallet_word = format!("{:0>64}", wallet.trim_start_matches("0x").to_lowercase());

    // 1. The NFT: tokens, tick spacing, ticks, liquidity. A burned or unknown id
    //    reverts. That revert is read with its reason (#144): when positions() AND
    //    ownerOf() both give this position manager's own "no such token" answer, the
    //    position is closed and the cycle goes on. Any other outcome fails the cycle.
    let outcome = |data: String, id: u64, what: &'static str| {
        let (cfg_id, npm) = (cfg.id.clone(), npm.clone());
        async move {
            let payload = json!({
                "jsonrpc": "2.0", "method": "eth_call", "id": id,
                "params": [{ "to": npm, "data": data }, "latest"]
            });
            eth_call_outcome(http_client, alchemy_url, &payload)
                .await
                .map_err(|e| anyhow::anyhow!("position {}: Aero CL {} read failed: {e}", cfg_id, what))
        }
    };
    let positions_hex = match outcome(format!("{V3_POSITIONS}{id_word}"), 80, "positions()").await? {
        CallOutcome::Returned(hex) => hex,
        reverted => {
            let owner_of = outcome(format!("{ERC721_OWNER_OF}{id_word}"), 86, "ownerOf()").await?;
            match nft_closure_verdict(&cfg.protocol, &npm, &reverted, &owner_of) {
                ClosureVerdict::Closed(evidence) => {
                    return Ok(PositionRead::Closed(ClosedProof { position_manager: npm, evidence }));
                }
                ClosureVerdict::NotProven(why) => {
                    return Err(anyhow::anyhow!(
                        "position {}: Aero CL positions() read failed (NFT #{} on position manager {}: {why})",
                        cfg.id, cfg.nft_id, npm
                    ));
                }
            }
        }
    };
    let decoded = decode_slipstream_positions_return(&positions_hex).map_err(|e| ctx(e, "positions()"))?;

    // 2. The pool: identity (position manager, tokens, tick spacing), price, tick and
    //    current swap fee. nft() comes first and may revert: a pool that is not this
    //    position manager's (a Uniswap v3 pool, say) is a configuration error, said as
    //    one, while a node that does not answer is a failed read.
    let nft_payload = json!({
        "jsonrpc": "2.0", "method": "eth_call", "id": 90,
        "params": [{ "to": pool, "data": CL_POOL_NFT }, "latest"]
    });
    let nft_call = eth_call_allow_revert(http_client, alchemy_url, &nft_payload)
        .await
        .map_err(|e| anyhow::anyhow!("position {}: Aero CL pool nft() read failed: {e}", cfg.id))?;
    let pool_nft = pool_position_manager(&cfg.id, &pool, &npm, nft_call.as_deref())?;
    let pool_token0 = address_from_word(&read(pool.clone(), CL_POOL_TOKEN0.to_string(), 81, "pool token0()").await?)
        .map_err(|e| ctx(e, "pool token0()"))?;
    let pool_token1 = address_from_word(&read(pool.clone(), CL_POOL_TOKEN1.to_string(), 82, "pool token1()").await?)
        .map_err(|e| ctx(e, "pool token1()"))?;
    let word_i32 = |hex: String, what: &'static str| -> Result<i32> {
        let clean = hex.trim_start_matches("0x");
        if clean.len() < 64 {
            return Err(anyhow::anyhow!("position {}: Aero CL {} returned short output", cfg.id, what));
        }
        parse_hex_word_i32(&clean[..64]).map_err(|e| ctx(e, what))
    };
    let pool_tick_spacing =
        word_i32(read(pool.clone(), CL_POOL_TICK_SPACING.to_string(), 83, "pool tickSpacing()").await?, "pool tickSpacing()")?;
    // Before anything else is read from it: a pool that is not this NFT's pool is a
    // configuration error, said as one, rather than a failed read further down.
    check_pool_is_the_nfts_pool(cfg, &npm, &pool, &decoded, &pool_nft, &pool_token0, &pool_token1, pool_tick_spacing)?;
    let pool_fee = word_i32(read(pool.clone(), CL_POOL_FEE.to_string(), 84, "pool fee()").await?, "pool fee()")?;
    let pool_fee_pips = u32::try_from(pool_fee)
        .map_err(|_| anyhow::anyhow!("position {}: Aero CL pool fee() is negative: {}", cfg.id, pool_fee))?;
    let slot0_hex = read(pool.clone(), V3_SLOT0.to_string(), 85, "pool slot0()").await?;
    let (sqrt_price_x96, tick) = decode_slipstream_slot0(&slot0_hex).map_err(|e| ctx(e, "pool slot0()"))?;

    // 3. Custody: the wallet holds the NFT, or the pool's gauge holds it AND the gauge
    //    lists it among the wallet's stakes. Anything else is not the operator's.
    let owner = address_from_word(&read(npm.clone(), format!("{ERC721_OWNER_OF}{id_word}"), 86, "ownerOf()").await?)
        .map_err(|e| ctx(e, "ownerOf()"))?;
    let pool_gauge = address_from_word(&read(pool.clone(), CL_POOL_GAUGE.to_string(), 87, "pool gauge()").await?)
        .map_err(|e| ctx(e, "pool gauge()"))?;
    let holder = classify_nft_holder(&cfg.id, cfg.nft_id, wallet, &owner, &pool_gauge, cfg.gauge_address.as_deref())?;
    let (staked_in_gauge, fees) = match holder {
        NftHolder::Gauge(gauge) => {
            let staked_hex = read(
                gauge.clone(),
                format!("{CL_GAUGE_STAKED_CONTAINS}{wallet_word}{id_word}"),
                88,
                "gauge stakedContains()",
            )
            .await?;
            require_staked_by_wallet(&cfg.id, cfg.nft_id, wallet, &gauge, &staked_hex)?;
            (Some(gauge), AeroClFees::StakedNotLpClaimable)
        }
        NftHolder::Wallet => {
            // collect() simulated from the wallet returns exactly what a claim would pay,
            // including fees not yet settled into tokensOwed. With no liquidity and
            // nothing owed there is nothing to simulate.
            if decoded.liquidity == 0 && decoded.tokens_owed0 == 0 && decoded.tokens_owed1 == 0 {
                (None, AeroClFees::Claimable(0, 0))
            } else {
                let payload = json!({
                    "jsonrpc": "2.0", "method": "eth_call", "id": 89,
                    "params": [{ "from": wallet, "to": npm, "data": build_v3_collect_calldata(cfg.nft_id, wallet) }, "latest"]
                });
                match eth_call_allow_revert(http_client, alchemy_url, &payload).await {
                    Err(e) => {
                        return Err(anyhow::anyhow!("position {}: Aero CL collect() simulation failed: {e}", cfg.id));
                    }
                    Ok(Some(hex)) => {
                        let (f0, f1) = decode_collect_return(&hex).map_err(|e| ctx(e, "collect()"))?;
                        (None, AeroClFees::Claimable(f0, f1))
                    }
                    Ok(None) => return Err(collect_reverted_error(&cfg.id, cfg.nft_id, &npm, wallet)),
                }
            }
        }
    };

    let (raw, facts) = aero_cl_position_from_reads(
        cfg, wallet, &npm, &pool, &decoded, &pool_nft, &pool_token0, &pool_token1, pool_tick_spacing, pool_fee_pips,
        sqrt_price_x96, tick, staked_in_gauge, fees,
    )?;
    tracing::info!(
        "aero CL position {} (NFT #{}): tickSpacing={}, fee={}bps, liquidity={}, tick={}, in_range={}, staked_in={:?}, amounts {:.6} {} / {:.4} {}",
        cfg.id, cfg.nft_id, decoded.tick_spacing, raw.fee_tier_bps, raw.liquidity, raw.tick, raw.in_range,
        facts.staked_in_gauge, raw.amount0, raw.token0.symbol, raw.amount1, raw.token1.symbol
    );
    Ok(PositionRead::Live((raw, facts)))
}
