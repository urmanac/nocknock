use crate::*;

/// What NockNock says about an Aero pool whose claimFees() reverts: its swap fees are
/// not paid to LPs that way. The AERO a staked LP earns instead is read from the gauge
/// and stated after this reason (aero_rewards.rs, issue #93).
pub(crate) const AERO_NOT_LP_CLAIMABLE: &str = "not LP-claimable (claimFees() reverts)";

/// Map an Aero claimFees() eth_call to (simulation, detail). Pure. A revert means the
/// pool pays no LP-claimable fees ("not_claimable"); only a transport failure is
/// "unavailable".
pub(crate) fn aero_claim_simulation(result: &Result<Option<String>>) -> (&'static str, Option<String>) {
    match result {
        Ok(Some(_)) => ("success", None),
        Ok(None) => ("not_claimable", Some(AERO_NOT_LP_CLAIMABLE.to_string())),
        Err(e) => ("unavailable", Some(format!("RPC unavailable for claimFees simulation: {e}"))),
    }
}

/// Pure core of the Aerodrome read (issue #88): turn pool reserves, LP supply, the
/// wallet's LP balance and its claimable fees into a RawPosition. A volatile pool is
/// x*y=k across every price, so it is modelled as a full-range position: the wallet
/// owns `lp_balance / total_supply` of each reserve, it is always in range, and its
/// price is reserve1/reserve0. Unit-testable.
#[allow(clippy::too_many_arguments)]
pub(crate) fn aerodrome_position_from_reads(
    cfg: &PositionConfig,
    wallet: &str,
    pool: &str,
    token0: PositionTokenInfo,
    token1: PositionTokenInfo,
    reserve0_raw: u128,
    reserve1_raw: u128,
    total_supply: u128,
    lp_balance: u128,
    fee0_raw: u128,
    fee1_raw: u128,
) -> Result<RawPosition> {
    if reserve0_raw == 0 || reserve1_raw == 0 || total_supply == 0 {
        return Err(anyhow::anyhow!(
            "position {}: Aerodrome pool {} has empty reserves or no LP supply",
            cfg.id,
            pool
        ));
    }
    let share = lp_balance as f64 / total_supply as f64;
    let amount0 = reserve0_raw as f64 * share / 10f64.powi(token0.decimals);
    let amount1 = reserve1_raw as f64 * share / 10f64.powi(token1.decimals);
    // sqrtPriceX96 equivalent of the pool's spot, so the shared price and sizing math
    // (nock_spot_from_positions, amounts_per_usd) works unchanged.
    let raw_price = reserve1_raw as f64 / reserve0_raw as f64; // token1_raw per token0_raw
    let sqrt_x96 = raw_price.sqrt() * 79228162514264337593543950336.0; // * 2^96
    let two128 = 340282366920938463463374607431768211456.0f64;
    let hi = (sqrt_x96 / two128).floor();
    let sqrt_price_x96 = U256(hi as u128, (sqrt_x96 - hi * two128) as u128);
    let tick = (raw_price.ln() / 1.0001f64.ln()).floor() as i32;
    Ok(RawPosition {
        position_id: position_key(cfg),
        network: cfg.network.clone(),
        protocol: "aerodrome".to_string(),
        wallet: wallet.to_string(),
        nft_id: 0,
        pool_id: pool.to_lowercase(),
        npm_address: None,
        fee_tier_bps: cfg.fee_bps.unwrap_or(30),
        token0: token0.clone(),
        token1: token1.clone(),
        liquidity: lp_balance,
        sqrt_price_x96,
        tick,
        tick_lower: MIN_TICK,
        tick_upper: MAX_TICK,
        amount0,
        amount1,
        fee0: fee0_raw as f64 / 10f64.powi(token0.decimals),
        fee1: fee1_raw as f64 / 10f64.powi(token1.decimals),
        in_range: true,
        is_price_source: cfg.is_price_source,
    })
}

/// Read an Aerodrome volatile-pool position (issue #88). Selectors are the ones the
/// pre-v0.4.0 worker used against this same pool (ONCHAIN_INTEGRATION.md §2):
/// getReserves 0x0902f1ac, totalSupply 0x18160ddd, balanceOf 0x70a08231, claimFees
/// 0xd294f093 simulated from the wallet. Every read failing fails the cycle (zero-mock).
pub(crate) async fn fetch_aerodrome_position(
    http_client: &Client,
    alchemy_url: &str,
    cfg: &PositionConfig,
    wallet: &str,
) -> Result<(RawPosition, bool)> {
    let pool = cfg
        .pool_address
        .clone()
        .ok_or_else(|| anyhow::anyhow!("position {}: aerodrome positions need pool_address", cfg.id))?;
    let wallet_word = format!("000000000000000000000000{}", wallet.trim_start_matches("0x").to_lowercase());
    let call = |data: String, from: Option<String>, id: i32| {
        let mut tx = json!({ "to": pool, "data": data });
        if let Some(f) = from {
            tx["from"] = json!(f);
        }
        json!({ "jsonrpc": "2.0", "method": "eth_call", "params": [tx, "latest"], "id": id })
    };
    let read = |payload: serde_json::Value, what: &'static str| {
        let cfg_id = cfg.id.clone();
        async move {
            post_rpc_with_retry(http_client, alchemy_url, &payload, 2)
                .await
                .and_then(|r| r.get("result").and_then(|v| v.as_str()).map(|s| s.trim_start_matches("0x").to_string()))
                .ok_or_else(|| anyhow::anyhow!("position {}: Aerodrome {} read failed", cfg_id, what))
        }
    };
    let word = |hex: &str, i: usize, what: &str| -> Result<u128> {
        let start = i * 64;
        if hex.len() < start + 64 {
            return Err(anyhow::anyhow!("position {}: Aerodrome {} returned short output", cfg.id, what));
        }
        // uint112/uint256 values here fit u128; take the low 32 hex bytes.
        u128::from_str_radix(&hex[start + 32..start + 64], 16)
            .map_err(|e| anyhow::anyhow!("position {}: Aerodrome {} unparseable: {e}", cfg.id, what))
    };

    let t0 = read(call("0x0dfe1681".to_string(), None, 70), "token0").await?;
    let t1 = read(call("0xd21220a7".to_string(), None, 71), "token1").await?;
    if t0.len() < 64 || t1.len() < 64 {
        return Err(anyhow::anyhow!("position {}: Aerodrome token0/token1 returned short output", cfg.id));
    }
    let token0 = token_info(&t0[24..64])?;
    let token1 = token_info(&t1[24..64])?;
    let reserves = read(call("0x0902f1ac".to_string(), None, 72), "getReserves").await?;
    let supply = read(call("0x18160ddd".to_string(), None, 73), "totalSupply").await?;
    let balance = read(call(format!("0x70a08231{wallet_word}"), None, 74), "balanceOf").await?;
    let mut lp_balance = word(&balance, 0, "balanceOf")?;
    // LP tokens staked in the gauge still belong to the wallet (issue #88 follow-up:
    // the operator's first aero.xyz deposit was staked automatically and read as $0).
    if let Some(gauge) = cfg.gauge_address.as_ref().filter(|g| !g.trim().is_empty()) {
        let payload = json!({
            "jsonrpc": "2.0", "method": "eth_call", "id": 76,
            "params": [{ "to": gauge, "data": format!("0x70a08231{wallet_word}") }, "latest"]
        });
        let staked_hex = read(payload, "gauge balanceOf").await?;
        let staked = word(&staked_hex, 0, "gauge balanceOf")?;
        lp_balance = lp_balance.saturating_add(staked);
    }
    // claimFees() simulated from the wallet returns exactly what a claim would pay.
    // With no LP there is nothing to simulate, so the call is skipped.
    //
    // Not every volatile pool pays fees to LPs this way: on the new Aero pools
    // claimFees() reverts (checked 2026-11-14 on 0x770fE8Fc…). A revert is an answer
    // ("nothing claimable here"), so fees read as zero; only a transport failure, where
    // the node gave no answer at all, fails the cycle.
    // The bool records whether claimFees() answers at all, so the revert is shown as
    // "not LP-claimable" rather than as $0 of fees earned.
    let (fee0_raw, fee1_raw, fees_claimable) = if lp_balance > 0 {
        let payload = call("0xd294f093".to_string(), Some(wallet.to_string()), 75);
        match eth_call_allow_revert(http_client, alchemy_url, &payload).await {
            Err(e) => {
                return Err(anyhow::anyhow!("position {}: Aerodrome claimFees read failed: {e}", cfg.id));
            }
            Ok(Some(hex)) => (word(&hex, 0, "claimFees")?, word(&hex, 1, "claimFees")?, true),
            Ok(None) => {
                tracing::info!(
                    "position {}: claimFees() reverted; this pool has no LP-claimable fees",
                    cfg.id
                );
                (0, 0, false)
            }
        }
    } else {
        (0, 0, true)
    };
    let raw = aerodrome_position_from_reads(
        cfg,
        wallet,
        &pool,
        token0,
        token1,
        word(&reserves, 0, "getReserves")?,
        word(&reserves, 1, "getReserves")?,
        word(&supply, 0, "totalSupply")?,
        lp_balance,
        fee0_raw,
        fee1_raw,
    )?;
    tracing::info!(
        "aerodrome position {}: LP balance={}, amounts {:.4} {} / {:.4} {}, fees {:.6}/{:.6}",
        cfg.id, raw.liquidity, raw.amount0, raw.token0.symbol, raw.amount1, raw.token1.symbol, raw.fee0, raw.fee1
    );
    Ok((raw, fees_claimable))
}
