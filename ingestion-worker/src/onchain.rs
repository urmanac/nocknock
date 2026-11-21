use crate::*;

/// Live ETH/USD spot from the canonical Uniswap v3 WETH/USDC 0.05% pool slot0
/// (`eth_price_pool` config, default DEFAULT_ETH_PRICE_POOL — factory-resolved and
/// price-verified at Phase-1 discovery). token0 = WETH (18 dec), token1 = USDC (6 dec).
/// Zero-mock: returns None when the RPC query fails — callers must degrade honestly
/// instead of valuing gas at a fabricated price.
pub(crate) async fn fetch_eth_price_usd(http_client: &Client, alchemy_url: &str, pool: &str) -> Option<f64> {
    fetch_eth_price_usd_at(http_client, alchemy_url, pool, "latest").await
}

/// ETH/USD from the price pool's slot0 at a given block tag ("latest" or a hex block
/// number; historical blocks need an archive RPC, which Alchemy provides). None on failure.
pub(crate) async fn fetch_eth_price_usd_at(http_client: &Client, alchemy_url: &str, pool: &str, block_tag: &str) -> Option<f64> {
    let payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": pool,
            "data": V3_SLOT0
        }, block_tag],
        "id": 99
    });
    if let Some(res_json) = post_rpc_with_retry(http_client, alchemy_url, &payload, 2).await {
        if let Some(hex_str) = res_json.get("result").and_then(|r| r.as_str()) {
            let clean = hex_str.trim_start_matches("0x");
            if clean.len() >= 64 {
                if let Ok(sqrt_price) = U256::from_hex_be(&clean[..64]) {
                    let raw = sqrt_price_x96_to_raw(sqrt_price); // USDC_raw per WETH_raw
                    let usdc_per_eth = raw * 1e18 / 1e6;
                    if usdc_per_eth > 0.0 {
                        return Some(usdc_per_eth);
                    }
                }
            }
        }
    }
    None
}

pub(crate) async fn fetch_raw_data(state: &Arc<AppState>) -> Result<RawOnChainData> {
    let alchemy_key = state.config.alchemy_api_keys.first().cloned().unwrap_or_default();
    let alchemy_url = format!("https://base-mainnet.g.alchemy.com/v2/{}", alchemy_key);
    fetch_raw_data_from(&state.http_client, &alchemy_url, &state.config, Utc::now()).await
}

/// One cycle's reads against a given RPC URL. `fetch_raw_data` builds the URL from the
/// configured key; the tests of #144 point this at a local node that answers with real
/// captured responses. It reads only `http_client` and `config`. `now` is the cycle's
/// clock: the one thing it decides is whether the block of a closed position's proof is
/// current (closed::ProofClock), so the tests pass the recorded block's own time.
pub(crate) async fn fetch_raw_data_from(
    http_client: &Client,
    alchemy_url: &str,
    config: &AppConfig,
    now: DateTime<Utc>,
) -> Result<RawOnChainData> {
    let proof_clock = ProofClock::for_cycle(now, config.sync_window_seconds);
    let Some(wallet_addr) = config.tracked_wallets.first() else {
        return Err(anyhow::anyhow!(
            "No tracked wallet configured: set tracked_wallets in config.yaml to the address NockNock should read"
        ));
    };

    // A position with an unknown protocol is a configuration error, stated before any
    // chain read (issue #91): it is never read as another protocol.
    validate_positions_config(&config.positions)?;

    // 0. Fetch live ETH/USD spot price from the canonical Uniswap v3 WETH/USDC pool slot0.
    //    (The former Aerodrome vAMM-WETH/USDC source is retired along with the liquidated
    //    Aerodrome tranche.) Zero-mock invariant: a failed query fails the cycle.
    let eth_price_pool = config.eth_price_pool.clone().unwrap_or_else(|| DEFAULT_ETH_PRICE_POOL.to_string());
    let eth_slot0_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": eth_price_pool,
            "data": V3_SLOT0
        }, "latest"],
        "id": 99
    });
    let mut live_eth_price_usd = 0.0f64;
    let mut eth_price_found = false;
    if let Some(res_json) = post_rpc_with_retry(http_client, alchemy_url, &eth_slot0_payload, 2).await {
        if let Some(hex_str) = res_json.get("result").and_then(|r| r.as_str()) {
            let clean = hex_str.trim_start_matches("0x");
            if clean.len() >= 64 {
                if let Ok(sqrt_price) = U256::from_hex_be(&clean[..64]) {
                    // token0 = WETH (18 dec), token1 = USDC (6 dec):
                    // raw price = USDC_raw per WETH_raw -> human USDC per ETH
                    let usdc_per_eth = sqrt_price_x96_to_raw(sqrt_price) * 1e18 / 1e6;
                    if usdc_per_eth > 0.0 {
                        live_eth_price_usd = usdc_per_eth;
                        eth_price_found = true;
                    }
                }
            }
        }
    }
    if !eth_price_found {
        return Err(anyhow::anyhow!(
            "Failed to query live ETH/USD spot price from the v3 WETH/USDC pool ({}) slot0",
            eth_price_pool
        ));
    }
    // 0b. Stale-price guard (issue #94), as for the NOCK spot: wallet gas and LP WETH are
    //     valued in the total, so a frozen pool's price would be a mock number.
    let eth_liq_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "to": eth_price_pool, "data": V3_LIQUIDITY }, "latest"],
        "id": 98
    });
    let eth_pool_liquidity = post_rpc_with_retry(http_client, alchemy_url, &eth_liq_payload, 2)
        .await
        .and_then(|r| r.get("result").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .and_then(|s| {
            let clean = s.trim_start_matches("0x");
            if clean.len() < 64 {
                return None;
            }
            u128::from_str_radix(&clean[clean.len() - 32..], 16).ok()
        })
        .ok_or_else(|| {
            anyhow::anyhow!("Failed to query in-range liquidity of the ETH/USD price pool ({})", eth_price_pool)
        })?;
    check_eth_price_pool_liquidity(&eth_price_pool, eth_pool_liquidity)?;

    // 1. Fetch live gas price (eth_gasPrice) — required input, fail explicitly on error
    let gas_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_gasPrice",
        "params": [],
        "id": 10
    });
    let mut live_gas_cost_usd = 0.0f64;
    let mut gas_price_found = false;
    if let Some(res_json) = post_rpc_with_retry(http_client, alchemy_url, &gas_payload, 2).await {
        if let Some(hex_str) = res_json.get("result").and_then(|r| r.as_str()) {
            let clean = hex_str.trim_start_matches("0x");
            if let Ok(wei) = u128::from_str_radix(clean, 16) {
                let eth_gas_cost = (wei as f64 / 1e18) * 150_000.0;
                // The +$0.002 add covers Base L1 data-fee volatility (documented buffer,
                // not a mock price). No upper clamp: a silent cap understated real gas
                // during fee spikes and biased the claim decision toward "claim now".
                let usd_gas_cost = (eth_gas_cost * live_eth_price_usd) + 0.002;
                live_gas_cost_usd = usd_gas_cost.max(0.002);
                gas_price_found = true;
            }
        }
    }
    if !gas_price_found {
        return Err(anyhow::anyhow!("Failed to query live gas price from RPC"));
    }

    // 2. Per-position reads (v2.0.0): every configured Uniswap position is read live;
    //    there are no Aerodrome LP-share reads anymore (tranche liquidated 2026-10-23).
    if config.positions.is_empty() {
        return Err(anyhow::anyhow!(
            "No positions configured; add entries under `positions:` in config.yaml (see ONCHAIN_INTEGRATION.md §5)"
        ));
    }
    let mut raw_positions: Vec<RawPosition> = Vec::new();
    let mut fees_not_lp_claimable: Vec<String> = Vec::new();
    let mut position_facts: HashMap<String, PositionFacts> = HashMap::new();
    // Positions proven closed this cycle (#144): their NFT does not exist. They are
    // never pushed to `raw_positions`, so nothing downstream counts or reads them.
    let mut closed_positions: Vec<ClosedPosition> = Vec::new();
    // The config entries that are still live, in config order: the only ones whose
    // gauges are read and whose rebuild rules apply.
    let mut live_configs: Vec<PositionConfig> = Vec::new();
    for pos in &config.positions {
        let read: PositionRead<RawPosition> = match Protocol::parse(&pos.protocol) {
            Some(Protocol::UniswapV4) => {
                // The pool's tick spacing (#166) travels as a position fact, like Aero CL's.
                match fetch_v4_position(http_client, alchemy_url, pos, wallet_addr, proof_clock).await? {
                    PositionRead::Live((raw, facts)) => {
                        if facts != PositionFacts::default() {
                            position_facts.insert(raw.position_id.clone(), facts);
                        }
                        PositionRead::Live(raw)
                    }
                    PositionRead::Closed(proof) => PositionRead::Closed(proof),
                }
            }
            Some(Protocol::UniswapV3) => {
                fetch_v3_position(http_client, alchemy_url, pos, wallet_addr, proof_clock).await?
            }
            Some(Protocol::AeroBasic) => {
                // No NFT, so never closed: an LP-token balance of zero is a live read.
                let (raw, fees_claimable) =
                    fetch_aerodrome_position(http_client, alchemy_url, pos, wallet_addr).await?;
                if !fees_claimable {
                    fees_not_lp_claimable.push(raw.position_id.clone());
                }
                PositionRead::Live(raw)
            }
            Some(Protocol::AeroCl) => {
                // Ownership is checked inside: an NFT the wallet neither holds nor staked
                // in the pool's gauge fails the cycle here (issue #91).
                match fetch_aero_cl_position(http_client, alchemy_url, pos, wallet_addr, proof_clock).await? {
                    PositionRead::Live((raw, facts)) => {
                        if facts.fees_note.is_some() {
                            fees_not_lp_claimable.push(raw.position_id.clone());
                        }
                        position_facts.insert(raw.position_id.clone(), facts);
                        PositionRead::Live(raw)
                    }
                    PositionRead::Closed(proof) => PositionRead::Closed(proof),
                }
            }
            None => {
                // Unreachable after validate_positions_config; kept so that an unknown
                // protocol can never reach an adapter.
                return Err(anyhow::anyhow!(
                    "Configuration error: position '{}' has unknown protocol '{}'. Supported: {}",
                    pos.id,
                    pos.protocol,
                    SUPPORTED_PROTOCOLS
                ));
            }
        };
        match read {
            PositionRead::Live(raw) => {
                raw_positions.push(raw);
                live_configs.push(pos.clone());
            }
            PositionRead::Closed(proof) => {
                tracing::info!(
                    "position {}: closed ({}); it is not counted and its config entry can be removed",
                    pos.id,
                    closed_reason(pos.nft_id, &proof.position_manager)
                );
                closed_positions.push(closed_position_entry(pos, proof, false));
            }
        }
    }
    // Every configured position is closed: there is nothing to report positions from,
    // as with no positions configured at all (issue #119 is not papered over here).
    if raw_positions.is_empty() {
        return Err(all_positions_closed_error(&closed_positions));
    }
    // 2b. Ladders (#167): live entries only (a closed ladder is in no total and has no
    //     reading). Each must be a USDC/NOCK concentrated range, checked on what was read.
    let ladders: HashMap<String, LadderTag> =
        live_configs.iter().filter(|c| is_ladder_config(c)).map(|c| (position_key(c), ladder_tag(c))).collect();
    check_ladder_pairs(&raw_positions, &ladders, &position_facts)?;

    // 3. NOCK/USD spot from the designated price-source position pool (zero-mock:
    //    fail the cycle when missing or unparseable).
    //    When the flagged position is CLOSED (#144) no price is guessed. A Uniswap v4
    //    entry's pool goes on supplying it through the same two reads as before
    //    (poolKeys and getSlot0 need no NFT). A v3 or Aero CL entry fails the cycle
    //    with a configuration message, because its pool was identified through the NFT.
    //    A live flagged position always comes first.
    let mut closed_price_pool: Option<ClosedPricePool> = None;
    if !raw_positions.iter().any(|p| p.is_price_source) {
        // The first flagged config entry that is closed, with its index in the list.
        let closed_source = config.positions.iter().filter(|c| c.is_price_source).find_map(|c| {
            let key = position_key(c);
            closed_positions.iter().position(|x| x.position_id == key).map(|i| (c, i))
        });
        if let Some((cfg, i)) = closed_source {
            if !closed_price_source_keeps_pricing(&cfg.protocol) {
                return Err(closed_price_source_error(cfg, &closed_positions[i].position_manager));
            }
            closed_price_pool = Some(fetch_v4_closed_price_pool(http_client, alchemy_url, cfg, wallet_addr).await?);
            closed_positions[i].supplies_spot_price = true;
        }
    }
    let price_source: Option<&RawPosition> = raw_positions
        .iter()
        .find(|p| p.is_price_source)
        .or(closed_price_pool.as_ref().map(|pool| &pool.0));
    let spot_price = price_source
        .and_then(|src| nock_spot_from_positions(std::slice::from_ref(src)))
        .ok_or_else(|| {
            anyhow::anyhow!(
                "No NOCK/USD spot: exactly one configured position must have is_price_source: true \
                 and quote NOCK against USDC"
            )
        })?;

    // 3b. Stale-price guard (zero-mock): a pool with no in-range liquidity cannot
    //     trade, so its sqrtPrice stays frozen wherever the last liquidity was pulled.
    //     Publishing that as "live" spot is a data-integrity bug — fail the cycle.
    if let Some(src) = price_source {
        let pool_liquidity =
            fetch_pool_in_range_liquidity(http_client, alchemy_url, src).await?;
        check_price_source_liquidity(&src.position_id, &src.pool_id, pool_liquidity)?;
    }

    // 4. Fetch live token balances for tracked wallet

    // 5. Fetch live token balances for tracked wallet
    let balances_payload = json!({
        "jsonrpc": "2.0",
        "method": "alchemy_getTokenBalances",
        "params": [
            wallet_addr,
            [
                "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913", // USDC
                "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722"  // NOCK
            ]
        ],
        "id": 2
    });

    let balances_json = post_rpc_with_retry(http_client, alchemy_url, &balances_payload, 2)
        .await
        .ok_or_else(|| anyhow::anyhow!("Failed to query live token balances from Alchemy RPC"))?;
    let (nock_balance, usdc_balance) = parse_wallet_token_balances(&balances_json)?;

    let nock_balance_usd = nock_balance * spot_price;
    let now_str = Utc::now().to_rfc3339();

    // 5. Snapshot-level builders (sync, unit-testable): USD pricing, pool mechanics,
    //    and per-position contract fees.
    // 5a. Pool activity per tracked position (issue #83). A failed read leaves the
    //     key absent: unknown, never assumed live or dead.
    let mut pool_liquidity: HashMap<String, String> = HashMap::new();
    for p in &raw_positions {
        match fetch_pool_in_range_liquidity(http_client, alchemy_url, p).await {
            Ok(l) => {
                pool_liquidity.insert(p.position_id.clone(), l.to_string());
            }
            Err(e) => tracing::warn!("pool liquidity read failed for {}: {e}", p.position_id),
        }
    }
    // 5b. ETH and WETH balances: the gas pillar (issue #94). Counted in the total, so both
    //     reads are required, like NOCK and USDC: a failed read fails the cycle.
    let eth_balance_payload = json!({
        "jsonrpc": "2.0", "method": "eth_getBalance", "params": [wallet_addr, "latest"], "id": 60
    });
    let wallet_eth = post_rpc_with_retry(http_client, alchemy_url, &eth_balance_payload, 2)
        .await
        .and_then(|r| r.get("result").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .and_then(|h| u128::from_str_radix(h.trim_start_matches("0x"), 16).ok())
        .map(|wei| wei as f64 / 1e18);
    let weth_balance_payload = json!({
        "jsonrpc": "2.0", "method": "eth_call", "id": 61,
        "params": [{
            "to": WETH_BASE,
            "data": format!("0x70a08231000000000000000000000000{}", wallet_addr.trim_start_matches("0x").to_lowercase())
        }, "latest"]
    });
    let wallet_weth = post_rpc_with_retry(http_client, alchemy_url, &weth_balance_payload, 2)
        .await
        .and_then(|r| r.get("result").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .filter(|h| h.len() > 2)
        .and_then(|h| u128::from_str_radix(h.trim_start_matches("0x"), 16).ok())
        .map(|wei| wei as f64 / 1e18);
    let (gas_eth, gas_weth) = require_gas_balances(wallet_eth, wallet_weth)?;
    // 5b'. AERO, the fourth pillar (issue #93): wallet AERO (required, counted in the
    //      total), unclaimed AERO per staked Aero position (a failed gauge read is stated
    //      on that position, never 0) and the AERO/USD price (required only while the
    //      wallet holds AERO; otherwise a failed read degrades with its reason).
    let aero_price_pool =
        config.aero_price_pool.clone().unwrap_or_else(|| DEFAULT_AERO_PRICE_POOL.to_string());
    let aero = fetch_aero_reads(
        http_client,
        alchemy_url,
        wallet_addr,
        &aero_price_pool,
        // Live entries only (#144): a closed position's gauge is not read. A CL gauge's
        // earned() reverts with "NA" for a token id the wallet no longer has staked.
        &live_configs,
        &position_facts,
    )
    .await?;
    // 5b''. Each ladder's baseline (#167): its deposit_tx receipt, checked against the
    //       position. Never fails the cycle: a failed read is `unverified` and retried.
    let mut ladder_baselines: HashMap<String, LadderBaselineRead> = HashMap::new();
    for p in raw_positions.iter() {
        if let Some(tag) = ladders.get(&p.position_id) {
            let read = read_ladder_baseline(http_client, alchemy_url, wallet_addr, p, tag).await;
            ladder_baselines.insert(p.position_id.clone(), read);
        }
    }
    // 5c. Per-position rebuild rules from config, keyed like RawPosition.position_id.
    //     Live entries only: a closed position takes no share of the LP target.
    let rebuild_rules: HashMap<String, RebuildRule> = live_configs
        .iter()
        .map(|c| {
            (
                position_key(c),
                RebuildRule {
                    weight_bps: c.rebuild_weight_bps,
                    when_out_of_range: c.rebuild_when_out_of_range,
                    deposit_url: c.deposit_url.clone(),
                },
            )
        })
        .collect();

    let positions =
        snapshot_positions_from(&raw_positions, spot_price, live_eth_price_usd, &pool_liquidity, &position_facts, &aero);
    let mut pool_mechanics = build_pool_mechanics(&raw_positions, spot_price, live_eth_price_usd);
    stamp_active_liquidity(&mut pool_mechanics, &raw_positions, &pool_liquidity);
    let contract_fees = build_contract_fees(&positions, live_gas_cost_usd, &fees_not_lp_claimable);

    tracing::info!(
        "Live position reads: {} position(s), NOCK spot ${:.6}, ETH/USD ${:.2}, AERO/USD {}",
        positions.len(),
        spot_price,
        live_eth_price_usd,
        aero.price_usd.map(|p| format!("${:.4}", p)).unwrap_or_else(|| "unavailable".to_string())
    );

    Ok(RawOnChainData {
        wallet_balances: vec![
            WalletBalance {
                wallet_address: wallet_addr.clone(),
                token_symbol: "NOCK".to_string(),
                token_address: "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722".to_string(),
                amount: format!("{:.4}", nock_balance),
                amount_usd: format!("{:.2}", nock_balance_usd),
                updated_at: now_str.clone(),
            },
            WalletBalance {
                wallet_address: wallet_addr.clone(),
                token_symbol: "USDC".to_string(),
                token_address: "0x833589fCD6eDb6E08f4c7C32D4f71b54bdA02913".to_string(),
                amount: format!("{:.4}", usdc_balance),
                amount_usd: format!("{:.2}", usdc_balance),
                updated_at: now_str.clone(),
            },
            // Gas rows (2.3.0) come after NOCK and USDC, which keep indexes 0 and 1.
            WalletBalance {
                wallet_address: wallet_addr.clone(),
                token_symbol: "ETH".to_string(),
                token_address: NATIVE_ETH.to_string(),
                amount: format!("{:.6}", gas_eth),
                amount_usd: format!("{:.2}", gas_eth * live_eth_price_usd),
                updated_at: now_str.clone(),
            },
            WalletBalance {
                wallet_address: wallet_addr.clone(),
                token_symbol: "WETH".to_string(),
                token_address: WETH_BASE.to_string(),
                amount: format!("{:.6}", gas_weth),
                amount_usd: format!("{:.2}", gas_weth * live_eth_price_usd),
                updated_at: now_str.clone(),
            },
            // AERO (v0.6.0, #93) comes last: earlier rows keep their indexes.
            aero_wallet_balance(wallet_addr, aero.wallet_aero.unwrap_or(0.0), aero.price_usd, &now_str),
        ],
        pool_mechanics,
        concentrated_liquidity: vec![], // DEPRECATED since v2.0.0 — see `positions`
        contract_fees,
        positions: raw_positions,
        spot_price_nock_usd: spot_price,
        live_gas_cost_usd,
        live_eth_price_usd,
        pool_liquidity,
        wallet_eth: Some(gas_eth),
        wallet_weth: Some(gas_weth),
        rebuild_rules,
        fees_not_lp_claimable,
        position_facts,
        aero,
        closed_positions,
        ladders,
        ladder_baselines,
    })
}

pub(crate) const USDC_BASE_LC: &str = "0x833589fcd6edb6e08f4c7c32d4f71b54bda02913";
pub(crate) const NOCK_BASE_LC: &str = "0x9b5e262cf9bb04869ab40b19af91d2dc85761722";

/// Wallet NOCK and USDC from an `alchemy_getTokenBalances` response, in human units.
/// Every token must be present with a readable balance: a missing entry, an error, or a
/// null `tokenBalance` fails the cycle instead of reading as 0 (issue #94, N14). Pure.
pub(crate) fn parse_wallet_token_balances(res_json: &serde_json::Value) -> Result<(f64, f64)> {
    let balances = res_json
        .get("result")
        .and_then(|r| r.get("tokenBalances"))
        .and_then(|t| t.as_array())
        .ok_or_else(|| anyhow::anyhow!("Failed to query live token balances from Alchemy RPC: no tokenBalances in the response"))?;
    let read = |want: &str, symbol: &str, decimals: i32| -> Result<f64> {
        let entry = balances
            .iter()
            .find(|b| {
                b.get("contractAddress").and_then(|a| a.as_str()).map(|a| a.to_lowercase()).as_deref() == Some(want)
            })
            .ok_or_else(|| anyhow::anyhow!("Wallet {symbol} balance missing from the token balance response"))?;
        if let Some(err) = entry.get("error").filter(|e| !e.is_null()) {
            return Err(anyhow::anyhow!("Wallet {symbol} balance read failed: {err}"));
        }
        let hex = entry
            .get("tokenBalance")
            .and_then(|h| h.as_str())
            .ok_or_else(|| anyhow::anyhow!("Wallet {symbol} balance is null in the token balance response; not read as 0"))?;
        let clean = hex.trim_start_matches("0x");
        let raw = if clean.is_empty() {
            0u128 // "0x" is an explicit zero balance
        } else {
            u128::from_str_radix(clean, 16)
                .map_err(|_| anyhow::anyhow!("Wallet {symbol} balance '{hex}' is not a hex quantity"))?
        };
        Ok(raw as f64 / 10f64.powi(decimals))
    };
    let usdc = read(USDC_BASE_LC, "USDC", 6)?;
    let nock = read(NOCK_BASE_LC, "NOCK", 16)?;
    Ok((nock, usdc))
}

/// The wallet's ETH and WETH (gas) balances are counted in the total, so both reads
/// are required: a failed read fails the cycle rather than counting gas as zero. Pure.
pub(crate) fn require_gas_balances(eth: Option<f64>, weth: Option<f64>) -> Result<(f64, f64)> {
    match (eth, weth) {
        (Some(e), Some(w)) => Ok((e, w)),
        (e, w) => Err(anyhow::anyhow!(
            "Failed to read wallet {} (gas) balance: gas is counted in the portfolio total, so the cycle fails rather than counting it as 0",
            match (e.is_none(), w.is_none()) {
                (true, true) => "ETH and WETH",
                (true, false) => "ETH",
                _ => "WETH",
            }
        )),
    }
}

/// The ETH/USD price pool must have in-range liquidity, or its price is frozen and would
/// value gas and LP WETH at a stale number (issue #94). Pure — unit-testable.
pub(crate) fn check_eth_price_pool_liquidity(pool: &str, in_range_liquidity: u128) -> Result<()> {
    if in_range_liquidity == 0 {
        return Err(anyhow::anyhow!(
            "Stale ETH/USD price: price pool {} has zero in-range liquidity, so its price is frozen. \
             Set eth_price_pool to a WETH/USDC pool with active liquidity.",
            pool
        ));
    }
    Ok(())
}

/// A price-source pool must have active liquidity, otherwise its price is frozen
/// and must never be presented as live spot. Pure — unit-testable.
pub(crate) fn check_price_source_liquidity(position_id: &str, pool_id: &str, in_range_liquidity: u128) -> Result<()> {
    if in_range_liquidity == 0 {
        return Err(anyhow::anyhow!(
            "Stale NOCK/USD spot: price-source pool {} (position {}) has zero in-range liquidity, \
             so its price is frozen. Move is_price_source to a pool with active liquidity.",
            pool_id,
            position_id
        ));
    }
    Ok(())
}
