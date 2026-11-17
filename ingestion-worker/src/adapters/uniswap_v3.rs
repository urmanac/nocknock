use crate::*;

pub(crate) async fn fetch_v3_position(
    http_client: &Client,
    alchemy_url: &str,
    pos: &PositionConfig,
    wallet: &str,
    clock: ProofClock,
) -> Result<PositionRead<RawPosition>> {
    let npm = pos
        .npm_address
        .clone()
        .ok_or_else(|| anyhow::anyhow!("position {} missing npm_address in config", pos.id))?;
    let pool = pos
        .pool_address
        .clone()
        .ok_or_else(|| anyhow::anyhow!(
            "position {} missing pool_address in config (resolve via factory getPool at discovery)",
            pos.id
        ))?;

    // 1. positions(uint256) -> 12-word decode (self-describing: tokens/fee/ticks/liquidity).
    //    A burned or unknown id reverts. The revert is read with its reason (#144).
    //    When it is the position manager's own "no such token" answer, the proof is
    //    read: positions() AND ownerOf(), both at one current block
    //    (closed::fetch_proof_block). When both give that answer there, the position is
    //    closed and the cycle goes on. Any other outcome fails the cycle.
    let outcome = |selector: &'static str, id: u64, what: &'static str, block: String| {
        let (cfg_id, npm) = (pos.id.clone(), npm.clone());
        let data = format!("{}{:064x}", selector, pos.nft_id);
        async move {
            let payload = json!({
                "jsonrpc": "2.0", "method": "eth_call", "id": id,
                "params": [{ "to": npm, "data": data }, block]
            });
            eth_call_outcome(http_client, alchemy_url, &payload)
                .await
                .map_err(|e| anyhow::anyhow!("position {}: NPM {} query failed: {e}", cfg_id, what))
        }
    };
    let not_proven = |why: String| {
        anyhow::anyhow!(
            "position {}: NPM positions() query failed (NFT #{} on position manager {}: {why})",
            pos.id, pos.nft_id, npm
        )
    };
    let res_hex = match outcome(V3_POSITIONS, 60, "positions()", "latest".to_string()).await? {
        CallOutcome::Returned(hex) => hex,
        reverted => {
            positions_says_nonexistent(&pos.protocol, &npm, &reverted).map_err(&not_proven)?;
            let block = fetch_proof_block(http_client, alchemy_url, clock)
                .await
                .map_err(|e| not_proven(format!("positions() {}, and {e}", describe_outcome(&reverted))))?;
            let positions = outcome(V3_POSITIONS, 68, "positions()", block.tag()).await?;
            let owner_of = outcome(ERC721_OWNER_OF, 69, "ownerOf()", block.tag()).await?;
            match nft_closure_verdict(&pos.protocol, &npm, &positions, &owner_of) {
                ClosureVerdict::Closed(evidence) => {
                    tracing::info!("position {}: NFT #{} proven nonexistent at block {}", pos.id, pos.nft_id, block.number);
                    return Ok(PositionRead::Closed(ClosedProof { position_manager: npm, evidence }));
                }
                ClosureVerdict::NotProven(why) => return Err(not_proven(format!("at block {}: {why}", block.number))),
            }
        }
    };
    let decoded = decode_v3_positions_return(&res_hex)
        .map_err(|e| anyhow::anyhow!("position {}: {}", pos.id, e))?;
    if decoded.token0 == NATIVE_ETH || decoded.token1 == NATIVE_ETH {
        return Err(anyhow::anyhow!(
            "position {}: NFT #{} decoded with a zero token address",
            pos.id,
            pos.nft_id
        ));
    }

    // 2. pool.slot0() -> (sqrtPriceX96, tick, ...)
    let slot0_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "to": pool, "data": V3_SLOT0 }, "latest"],
        "id": 61
    });
    let slot0_hex = post_rpc_with_retry(http_client, alchemy_url, &slot0_payload, 2)
        .await
        .and_then(|r| r.get("result").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .ok_or_else(|| anyhow::anyhow!("position {}: pool slot0 query failed", pos.id))?;
    let clean = slot0_hex.trim_start_matches("0x");
    if clean.len() < 128 {
        return Err(anyhow::anyhow!("position {}: pool slot0 returned short output", pos.id));
    }
    let sqrt_price_x96 = U256::from_hex_be(&clean[..64])?;
    let tick = parse_hex_word_i32(&clean[64..128])?;

    // 3. v3 uncollected fees: inside = global - outside(lower) - outside(upper);
    //    owed = (inside - insideLast) * L / 2^128 + tokensOwed. TickInfo word layout:
    //    [0] liquidityGross, [1] liquidityNet, [2] feeGrowthOutside0, [3] feeGrowthOutside1
    //    (this pool returns an 8-word struct; words 4+ are fork extras we ignore).
    let fg0_g = post_rpc_word(http_client, alchemy_url, pool.as_str(), V3_FEE_GROWTH_GLOBAL0, 0, 62).await?;
    let fg1_g = post_rpc_word(http_client, alchemy_url, pool.as_str(), V3_FEE_GROWTH_GLOBAL1, 0, 63).await?;
    // TickInfo words: [2] feeGrowthOutside0, [3] feeGrowthOutside1 — BOTH bounds, BOTH legs.
    let tl_lower_tick_data = format!("{}{}", V3_TICKS, int24_calldata_word(decoded.tick_lower));
    let tu_upper_tick_data = format!("{}{}", V3_TICKS, int24_calldata_word(decoded.tick_upper));
    let fgo0_l = post_rpc_word(http_client, alchemy_url, pool.as_str(), &tl_lower_tick_data, 2, 64).await?;
    let fgo1_l = post_rpc_word(http_client, alchemy_url, pool.as_str(), &tl_lower_tick_data, 3, 66).await?;
    let fgo0_u = post_rpc_word(http_client, alchemy_url, pool.as_str(), &tu_upper_tick_data, 2, 67).await?;
    let fgo1_u = post_rpc_word(http_client, alchemy_url, pool.as_str(), &tu_upper_tick_data, 3, 68).await?;
    let inside0 = fg0_g.wrapping_sub(&fgo0_l).wrapping_sub(&fgo0_u);
    let inside1 = fg1_g.wrapping_sub(&fgo1_l).wrapping_sub(&fgo1_u);
    // Uncollected = (inside - insideLast) * L / 2^128 + tokensOwed (the settled part).
    let fee0_raw = inside0
        .wrapping_sub(&decoded.fee_growth_inside0_last)
        .mul_div_q128(decoded.liquidity)
        .saturating_add(decoded.tokens_owed0);
    let fee1_raw = inside1
        .wrapping_sub(&decoded.fee_growth_inside1_last)
        .mul_div_q128(decoded.liquidity)
        .saturating_add(decoded.tokens_owed1);

    let token0 = token_info(&decoded.token0)?;
    let token1 = token_info(&decoded.token1)?;
    let (amount0_raw, amount1_raw) =
        concentrated_amounts_raw(decoded.liquidity, sqrt_price_x96, decoded.tick_lower, decoded.tick_upper);
    let dec0 = 10f64.powi(token0.decimals);
    let dec1 = 10f64.powi(token1.decimals);

    tracing::info!(
        "v3 position {} (NFT #{}): fee={}bps, liquidity={}, tick={}, in_range={}, fees raw {}/{}",
        pos.id, pos.nft_id, decoded.fee, decoded.liquidity, tick,
        decoded.tick_lower <= tick && tick <= decoded.tick_upper, fee0_raw, fee1_raw
    );

    Ok(PositionRead::Live(RawPosition {
        position_id: format!("uniswap-v3:{}:{}", pos.network, pos.nft_id),
        network: pos.network.clone(),
        protocol: "uniswap-v3".to_string(),
        wallet: wallet.to_string(),
        nft_id: pos.nft_id,
        pool_id: pool.clone(),
        npm_address: Some(npm.clone()),
        fee_tier_bps: (decoded.fee / 100) as u32,
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
        in_range: decoded.tick_lower <= tick && tick <= decoded.tick_upper,
        is_price_source: pos.is_price_source,
    }))
}

/// Parse the 384-byte `positions(uint256)` return:
/// (nonce, operator, token0, token1, fee, tickLower, tickUpper, liquidity,
///  feeGrowthInside0Last, feeGrowthInside1Last, tokensOwed0, tokensOwed1).
pub(crate) fn decode_v3_positions_return(hex_str: &str) -> Result<V3PositionDecoded> {
    let clean = hex_str.trim_start_matches("0x");
    if clean.len() < 12 * 64 {
        return Err(anyhow::anyhow!(
            "NPM positions() returned {} chars, expected >= 768",
            clean.len()
        ));
    }
    let word = |i: usize| -> &str { &clean[i * 64..(i + 1) * 64] };
    let addr = |i: usize| -> String { format!("0x{}", &word(i)[24..]) };
    // Sign-extended int24 words (a negative tick is `ff…`): the shared decoder in evm.rs.
    let signed_tick = |i: usize| -> Result<i32> {
        parse_hex_word_i32(word(i)).map_err(|e| anyhow::anyhow!("bad tick word: {e}"))
    };
    Ok(V3PositionDecoded {
        token0: addr(2),
        token1: addr(3),
        fee: u64::from_str_radix(word(4), 16).map_err(|e| anyhow::anyhow!("bad fee word: {e}"))?,
        tick_lower: signed_tick(5)?,
        tick_upper: signed_tick(6)?,
        liquidity: u128::from_str_radix(word(7), 16).map_err(|e| anyhow::anyhow!("bad liquidity word: {e}"))?,
        fee_growth_inside0_last: U256::from_hex_be(word(8))?,
        fee_growth_inside1_last: U256::from_hex_be(word(9))?,
        tokens_owed0: u128::from_str_radix(word(10), 16).unwrap_or(0),
        tokens_owed1: u128::from_str_radix(word(11), 16).unwrap_or(0),
    })
}
