use crate::*;

// ------------------------------------------------------------------
// Async per-position readers
// ------------------------------------------------------------------

/// The v4 config fields every read needs: the bytes32 pool id (64 hex chars, no 0x)
/// and the two currencies (40 hex chars each, no 0x), lowercase. Pure.
fn v4_pool_config(pos: &PositionConfig) -> Result<(String, String, String)> {
    let pool_id = pos
        .pool_id
        .as_deref()
        .map(|s| s.trim_start_matches("0x").to_lowercase())
        .filter(|s| s.len() == 64 && s.chars().all(|c| c.is_ascii_hexdigit()))
        .ok_or_else(|| anyhow::anyhow!("position {} missing/invalid v4 pool_id (re-run discovery)", pos.id))?;
    let currency0 = pos
        .currency0
        .as_deref()
        .map(|s| s.trim_start_matches("0x").to_lowercase())
        .ok_or_else(|| anyhow::anyhow!("position {} missing currency0 in config", pos.id))?;
    let currency1 = pos
        .currency1
        .as_deref()
        .map(|s| s.trim_start_matches("0x").to_lowercase())
        .ok_or_else(|| anyhow::anyhow!("position {} missing currency1 in config", pos.id))?;
    Ok((pool_id, currency0, currency1))
}

/// poolKeys(bytes25) -> (currency0, currency1, fee, tickSpacing, hooks): validates the
/// config's declared currency ordering and supplies the fee tier in basis points. It is
/// a read of the pool key, not of the NFT.
async fn fetch_v4_pool_fee_bps(
    http_client: &Client,
    alchemy_url: &str,
    pos: &PositionConfig,
    pool_id: &str,
    currency0: &str,
    currency1: &str,
) -> Result<u32> {
    let pk_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": UNISWAP_V4_POSM_BASE,
            "data": format!("{}{}{}", V4_POOL_KEYS, &pool_id[..50], "0".repeat(14))
        }, "latest"],
        "id": 50
    });
    let mut fee_tier_bps = 0u32;
    if let Some(res_json) = post_rpc_with_retry(http_client, alchemy_url, &pk_payload, 2).await {
        if let Some(hex_str) = res_json.get("result").and_then(|r| r.as_str()) {
            let clean = hex_str.trim_start_matches("0x");
            if clean.len() >= 160 {
                let c0 = clean[24..64].to_lowercase();
                let c1 = clean[88..128].to_lowercase();
                let fee = parse_hex_word_u128(&clean[128..192]).unwrap_or(0) as u64;
                if c0 != currency0 || c1 != currency1 {
                    return Err(anyhow::anyhow!(
                        "position {}: poolKeys currency ordering (0x{}, 0x{}) does not match config ({}, {}); re-run discovery",
                        pos.id,
                        c0,
                        c1,
                        pos.currency0.as_deref().unwrap_or(""),
                        pos.currency1.as_deref().unwrap_or("")
                    ));
                }
                fee_tier_bps = (fee / 100) as u32; // fee is in hundredths of a basis point
            }
        }
    }
    if fee_tier_bps == 0 {
        return Err(anyhow::anyhow!(
            "position {}: failed to read poolKeys fee tier from POSM",
            pos.id
        ));
    }
    Ok(fee_tier_bps)
}

/// StateView getSlot0(bytes32) -> (sqrtPriceX96, tick, ...). A read of the pool.
async fn fetch_v4_slot0(http_client: &Client, alchemy_url: &str, pos_id: &str, pool_id: &str) -> Result<(U256, i32)> {
    let slot0_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": UNISWAP_V4_STATE_VIEW_BASE,
            "data": format!("{}{}", V4_STATE_VIEW_GET_SLOT0, pool_id)
        }, "latest"],
        "id": 52
    });
    let slot0_hex = post_rpc_with_retry(http_client, alchemy_url, &slot0_payload, 2)
        .await
        .and_then(|r| r.get("result").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .ok_or_else(|| anyhow::anyhow!("position {}: StateView getSlot0 query failed", pos_id))?;
    let clean = slot0_hex.trim_start_matches("0x");
    if clean.len() < 128 {
        return Err(anyhow::anyhow!("position {}: StateView getSlot0 returned short output", pos_id));
    }
    Ok((U256::from_hex_be(&clean[..64])?, parse_hex_word_i32(&clean[64..128])?))
}

/// The pool of a CLOSED Uniswap v4 price-source entry, read for its price alone (#144).
/// Never a position: it holds nothing and is never added to the cycle's positions.
#[derive(Debug, Clone)]
pub(crate) struct ClosedPricePool(pub(crate) RawPosition);

/// The NOCK/USD price reads of a v4 config entry whose NFT is gone. They are the same
/// two reads a live v4 position's price comes from, and neither needs the NFT: poolKeys
/// (the pool key is the config's pair, in order) and StateView getSlot0. The caller
/// applies the same stale-pool guard as for a live price source.
pub(crate) async fn fetch_v4_closed_price_pool(
    http_client: &Client,
    alchemy_url: &str,
    pos: &PositionConfig,
    wallet: &str,
) -> Result<ClosedPricePool> {
    let (pool_id, currency0, currency1) = v4_pool_config(pos)?;
    let fee_tier_bps = fetch_v4_pool_fee_bps(http_client, alchemy_url, pos, &pool_id, &currency0, &currency1).await?;
    let (sqrt_price_x96, tick) = fetch_v4_slot0(http_client, alchemy_url, &pos.id, &pool_id).await?;
    Ok(ClosedPricePool(RawPosition {
        position_id: position_key(pos),
        network: pos.network.clone(),
        protocol: "uniswap-v4".to_string(),
        wallet: wallet.to_string(),
        nft_id: pos.nft_id,
        pool_id: format!("0x{}", pool_id),
        npm_address: None,
        fee_tier_bps,
        token0: token_info(&format!("0x{}", currency0))?,
        token1: token_info(&format!("0x{}", currency1))?,
        // Nothing is held: the NFT does not exist. Only the pool's price is used.
        liquidity: 0,
        sqrt_price_x96,
        tick,
        tick_lower: pos.tick_lower.unwrap_or(0),
        tick_upper: pos.tick_upper.unwrap_or(0),
        amount0: 0.0,
        amount1: 0.0,
        fee0: 0.0,
        fee1: 0.0,
        in_range: false,
        is_price_source: true,
    }))
}

pub(crate) async fn fetch_v4_position(
    http_client: &Client,
    alchemy_url: &str,
    pos: &PositionConfig,
    wallet: &str,
) -> Result<PositionRead<RawPosition>> {
    let (pool_id, currency0, currency1) = v4_pool_config(pos)?;
    let tick_lower = pos
        .tick_lower
        .ok_or_else(|| anyhow::anyhow!("position {} missing tick_lower in config", pos.id))?;
    let tick_upper = pos
        .tick_upper
        .ok_or_else(|| anyhow::anyhow!("position {} missing tick_upper in config", pos.id))?;

    // 0. poolKeys(bytes25) -> (currency0, currency1, fee, tickSpacing, hooks);
    //    validates the config's declared currency ordering and supplies the fee tier.
    let fee_tier_bps = fetch_v4_pool_fee_bps(http_client, alchemy_url, pos, &pool_id, &currency0, &currency1).await?;

    // 1. getPositionLiquidity(uint256)
    let liq_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": UNISWAP_V4_POSM_BASE,
            "data": format!("{}{:064x}", V4_GET_POSITION_LIQUIDITY, pos.nft_id)
        }, "latest"],
        "id": 51
    });
    let liquidity = post_rpc_with_retry(http_client, alchemy_url, &liq_payload, 2)
        .await
        .and_then(|r| r.get("result").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .and_then(|s| u128::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        .ok_or_else(|| {
            anyhow::anyhow!(
                "position {}: failed to query getPositionLiquidity from POSM",
                pos.id
            )
        })?;

    // 1b. Does the NFT exist? (#144) getPositionLiquidity answers 0 for a burned or
    //     never-minted id exactly as it does for an NFT that exists and is empty, so
    //     before #144 a burned v4 NFT read as a live position holding nothing. Zero
    //     liquidity is therefore followed by ownerOf: an owner means the NFT exists
    //     (empty, still a live position); Error("NOT_MINTED") together with an empty
    //     getPoolAndPositionInfo means it does not, and the position is closed. Any
    //     other outcome fails the cycle. Liquidity above zero is itself proof that the
    //     NFT exists (a burn clears it), so that path makes no extra read.
    if liquidity == 0 {
        let outcome = |selector: &'static str, id: u64, what: &'static str| {
            let cfg_id = pos.id.clone();
            let data = format!("{}{:064x}", selector, pos.nft_id);
            async move {
                let payload = json!({
                    "jsonrpc": "2.0", "method": "eth_call", "id": id,
                    "params": [{ "to": UNISWAP_V4_POSM_BASE, "data": data }, "latest"]
                });
                eth_call_outcome(http_client, alchemy_url, &payload)
                    .await
                    .map_err(|e| anyhow::anyhow!("position {}: POSM {} query failed: {e}", cfg_id, what))
            }
        };
        match outcome(ERC721_OWNER_OF, 55, "ownerOf()").await? {
            CallOutcome::Returned(_) => {}
            reverted => {
                let info = outcome(V4_GET_POOL_AND_POSITION_INFO, 56, "getPoolAndPositionInfo()").await?;
                match v4_closure_verdict(&reverted, &info) {
                    ClosureVerdict::Closed(evidence) => {
                        return Ok(PositionRead::Closed(ClosedProof {
                            position_manager: UNISWAP_V4_POSM_BASE.to_string(),
                            evidence,
                        }));
                    }
                    ClosureVerdict::NotProven(why) => {
                        return Err(anyhow::anyhow!(
                            "position {}: NFT #{} reads zero liquidity and whether it exists on PositionManager {} could not be established ({why})",
                            pos.id, pos.nft_id, UNISWAP_V4_POSM_BASE
                        ));
                    }
                }
            }
        }
    }

    // 2. StateView getSlot0(bytes32) -> (sqrtPriceX96, tick, ...)
    let (sqrt_price_x96, tick) = fetch_v4_slot0(http_client, alchemy_url, &pos.id, &pool_id).await?;

    // 3. Uncollected fees: getPositionInfo(poolId, posm, tickLower, tickUpper, tokenId)
    //    + getFeeGrowthInside(poolId, tickLower, tickUpper), existing verified selectors.
    let pos_info_data = format!(
        "{}{}{}{}{}{:064x}",
        V4_STATE_VIEW_GET_POSITION_INFO,
        pool_id,
        word_addr(UNISWAP_V4_POSM_BASE),
        int24_calldata_word(tick_lower),
        int24_calldata_word(tick_upper),
        pos.nft_id
    );
    let pos_info_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": UNISWAP_V4_STATE_VIEW_BASE,
            "data": pos_info_data
        }, "latest"],
        "id": 53
    });
    let fg_data = format!(
        "{}{}{}{}",
        V4_STATE_VIEW_GET_FEE_GROWTH_INSIDE,
        pool_id,
        int24_calldata_word(tick_lower),
        int24_calldata_word(tick_upper)
    );
    let fg_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": UNISWAP_V4_STATE_VIEW_BASE,
            "data": fg_data
        }, "latest"],
        "id": 54
    });
    let (fg0_last, fg1_last, fg0_curr, fg1_curr) = match (
        post_rpc_with_retry(http_client, alchemy_url, &pos_info_payload, 2).await,
        post_rpc_with_retry(http_client, alchemy_url, &fg_payload, 2).await,
    ) {
        (Some(res_pos), Some(res_fg)) => {
            let hex_pos = res_pos.get("result").and_then(|r| r.as_str()).unwrap_or("");
            let hex_fg = res_fg.get("result").and_then(|r| r.as_str()).unwrap_or("");
            let cp = hex_pos.trim_start_matches("0x");
            let cf = hex_fg.trim_start_matches("0x");
            if cp.len() < 192 || cf.len() < 128 {
                return Err(anyhow::anyhow!(
                    "position {}: StateView getPositionInfo/getFeeGrowthInside returned short output",
                    pos.id
                ));
            }
            (
                U256::from_hex_be(&cp[64..128])?,
                U256::from_hex_be(&cp[128..192])?,
                U256::from_hex_be(&cf[0..64])?,
                U256::from_hex_be(&cf[64..128])?,
            )
        }
        _ => {
            return Err(anyhow::anyhow!(
                "position {}: failed to query StateView fee growth",
                pos.id
            ));
        }
    };
    let fee0_raw = fg0_curr.wrapping_sub(&fg0_last).mul_div_q128(liquidity);
    let fee1_raw = fg1_curr.wrapping_sub(&fg1_last).mul_div_q128(liquidity);

    let token0 = token_info(&format!("0x{}", currency0))?;
    let token1 = token_info(&format!("0x{}", currency1))?;
    let (amount0_raw, amount1_raw) = concentrated_amounts_raw(liquidity, sqrt_price_x96, tick_lower, tick_upper);
    let dec0 = 10f64.powi(token0.decimals);
    let dec1 = 10f64.powi(token1.decimals);

    tracing::info!(
        "v4 position {} (NFT #{}): liquidity={}, tick={}, in_range={}, fees raw {}/{}",
        pos.id, pos.nft_id, liquidity, tick, tick_lower <= tick && tick <= tick_upper, fee0_raw, fee1_raw
    );

    Ok(PositionRead::Live(RawPosition {
        position_id: format!("uniswap-v4:{}:{}", pos.network, pos.nft_id),
        network: pos.network.clone(),
        protocol: "uniswap-v4".to_string(),
        wallet: wallet.to_string(),
        nft_id: pos.nft_id,
        pool_id: format!("0x{}", pool_id),
        npm_address: None,
        fee_tier_bps,
        token0,
        token1,
        liquidity,
        sqrt_price_x96,
        tick,
        tick_lower,
        tick_upper,
        amount0: amount0_raw / dec0,
        amount1: amount1_raw / dec1,
        fee0: fee0_raw as f64 / dec0,
        fee1: fee1_raw as f64 / dec1,
        in_range: tick_lower <= tick && tick <= tick_upper,
        is_price_source: pos.is_price_source,
    }))
}
