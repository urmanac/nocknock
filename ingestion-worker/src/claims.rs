use crate::*;

/// NOCK spot price from the latest snapshot (stored in token_price_series by every
/// cycle since v0.1.0). Works for historical Aerodrome-derived rows and current
/// Uniswap-derived rows alike.
pub(crate) fn nock_spot_from_snapshot(data: &SnapshotData) -> Option<f64> {
    data.derived_metrics
        .total_value_vs_baseline
        .token_price_series
        .first()
        .and_then(|s| s.parse::<f64>().ok())
        .filter(|v| *v > 0.0)
}

/// True when a position has no liquidity and no unclaimed fees. Pure.
pub(crate) fn nothing_to_collect(liquidity: &str, owed0: &str, owed1: &str) -> bool {
    let zero = |v: &str| v.parse::<f64>().map_or(false, |x| x == 0.0);
    zero(liquidity) && zero(owed0) && zero(owed1)
}

/// Gas and net totals for the claim payload. Pure. Actions that cannot execute
/// ("not_claimable") are left out, so they never null the totals of the ones that can.
/// `gas_usd` per action is None when its estimate failed; any such failure among the
/// executable actions makes the total unknown rather than understated.
pub(crate) fn claim_payload_totals(
    actions: &[ClaimFeeAction],
    gross_usd: &str,
    gas_price_and_eth_known: bool,
) -> (Option<String>, Option<String>) {
    let executable: Vec<&ClaimFeeAction> =
        actions.iter().filter(|a| a.simulation != "not_claimable").collect();
    let estimated_gas_usd = if gas_price_and_eth_known && !executable.is_empty() {
        executable
            .iter()
            .map(|a| a.gas_usd.as_deref().and_then(|g| g.parse::<f64>().ok()))
            .sum::<Option<f64>>()
            .map(|g| format!("{:.4}", g))
    } else {
        None
    };
    let net_after_gas_usd = match (gross_usd.trim(), &estimated_gas_usd) {
        (g, Some(e)) if !g.is_empty() => {
            let gross: f64 = g.parse().unwrap_or(0.0);
            Some(format!("{:.4}", gross - e.parse::<f64>().unwrap_or(0.0)))
        }
        _ => None,
    };
    (estimated_gas_usd, net_after_gas_usd)
}

/// Fill expected amounts + token-aware USD valuation from the matching snapshot
/// position and simulate the collect via eth_call. Shared by the v4 and v3 action
/// builders so both payload halves behave identically (zero-mock: degrades to None).
pub(crate) async fn finish_claim_action(
    http_client: &Client,
    alchemy_url: &str,
    wallet: &str,
    snapshot: &SnapshotEnvelope,
    action: &mut ClaimFeeAction,
    pos_id: &str,
    sim_id: i32,
    nock_spot: Option<f64>,
    eth_price: Option<f64>,
    revert_is_not_claimable: bool,
) {
    let price_for = |symbol: &str| -> Option<f64> {
        match symbol {
            "USDC" => Some(1.0),
            "NOCK" => nock_spot,
            "WETH" | "ETH" => eth_price,
            _ => None,
        }
    };
    if let Some(p) = snapshot.data.positions.iter().find(|p| p.position_id == pos_id) {
        action.expected_amount_0 = Some(p.tokens_owed_0.clone());
        action.expected_amount_1 = Some(p.tokens_owed_1.clone());
        if nothing_to_collect(&p.liquidity_amount, &p.tokens_owed_0, &p.tokens_owed_1) {
            // An empty position's collect reverts; that is not an RPC outage (issue #87).
            action.expected_usd = Some("0.0000".to_string());
            action.simulation_detail =
                Some("Nothing to collect: this position holds no liquidity and no fees".to_string());
            return;
        }
        if let (Ok(v0), Ok(v1)) = (
            p.tokens_owed_0.parse::<f64>(),
            p.tokens_owed_1.parse::<f64>(),
        ) {
            match (price_for(&p.token0.symbol), price_for(&p.token1.symbol)) {
                (Some(p0), Some(p1)) => {
                    action.expected_usd = Some(format!("{:.4}", v0 * p0 + v1 * p1));
                }
                _ => {
                    action.simulation_detail = Some(
                        "USD valuation unavailable (missing live NOCK or ETH price)".to_string(),
                    );
                }
            }
        }
    }
    let sim_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "from": wallet, "to": action.to, "data": action.data }, "latest"],
        "id": sim_id
    });
    if revert_is_not_claimable {
        // Aero claimFees(): a revert is an answer (no LP-claimable fees), not an outage.
        let (sim, detail) = aero_claim_simulation(&eth_call_allow_revert(http_client, alchemy_url, &sim_payload).await);
        action.simulation = sim.to_string();
        if detail.is_some() {
            action.simulation_detail = detail;
        }
        if action.simulation == "not_claimable" {
            action.expected_usd = None;
        }
        return;
    }
    match post_rpc_with_retry(http_client, alchemy_url, &sim_payload, 2).await {
        Some(res_json) => {
            if res_json.get("result").and_then(|r| r.as_str()).is_some() {
                action.simulation = "success".to_string();
            } else {
                action.simulation = "failed".to_string();
                action.simulation_detail = Some("collect eth_call reverted".to_string());
            }
        }
        None => {
            action.simulation_detail = Some("RPC unavailable for collect simulation".to_string());
        }
    }
}

/// Aero CL claim action, from the snapshot's own statement about the position. Pure.
/// Returns true when the action is settled without a simulation:
/// - staked in its gauge: "not_claimable" with the reason (nothing to dispatch);
/// - not in the latest snapshot: left "unavailable" with the reason, never simulated
///   blind (staked or not is unknown);
/// - any other position whose snapshot row carries a `fees_note`: the note says its
///   fee figures are not a read of LP-claimable fees, so they are never copied into
///   the action as expected amounts. It is left "unavailable" with the note and no
///   expected amounts or USD. (No adapter emits this today: an unstaked position's
///   failed fee read fails the cycle. The arm keeps the rule "a note means no figure"
///   true for stored snapshots and for notes added later.)
/// Returns false for an unstaked position whose fees were read, which is simulated
/// like a Uniswap v3 collect.
pub(crate) fn aero_cl_mark_not_claimable(action: &mut ClaimFeeAction, snapshot_position: Option<&Position>) -> bool {
    match snapshot_position {
        None => {
            action.simulation = "unavailable".to_string();
            action.simulation_detail =
                Some("Position is not in the latest snapshot yet; whether it is staked is unknown".to_string());
            true
        }
        Some(p) if p.staked_in_gauge.is_some() => {
            action.simulation = "not_claimable".to_string();
            action.simulation_detail =
                Some(fees_note_text(p.fees_note.as_deref().or(Some(AERO_CL_STAKED_FEES_NOTE))));
            action.expected_usd = None;
            true
        }
        Some(p) if p.fees_note.as_deref().is_some_and(|n| !n.trim().is_empty()) => {
            action.simulation = "unavailable".to_string();
            action.simulation_detail = p.fees_note.clone();
            action.expected_amount_0 = None;
            action.expected_amount_1 = None;
            action.expected_usd = None;
            true
        }
        Some(_) => false,
    }
}

/// GET /v1/execute/claim-fees/payload — unsigned fee-claim transactions for the
/// sovereign wallet, derived from live on-chain state. Clients (Android app, web
/// cockpit) dispatch these to the paired wallet for signing; they never build
/// calldata themselves.
pub(crate) async fn get_claim_fees_payload_handler(Extension(state): Extension<Arc<AppState>>) -> impl IntoResponse {
    let wallet = match state.config.tracked_wallets.first().cloned() {
        Some(w) => w,
        None => {
            return (StatusCode::SERVICE_UNAVAILABLE, Json(json!({
                "error": "NO_TRACKED_WALLET",
                "detail": "No tracked wallet configured; cannot build claim transactions"
            }))).into_response();
        }
    };
    let alchemy_key = state.config.alchemy_api_keys.first().cloned().unwrap_or_default();
    if alchemy_key.is_empty() || alchemy_key == "YOUR_ALCHEMY_API_KEY" {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(json!({
            "error": "No Alchemy RPC key configured; cannot simulate or estimate claim transactions"
        }))).into_response();
    }
    let alchemy_url = format!("https://base-mainnet.g.alchemy.com/v2/{}", alchemy_key);

    // Live snapshot is the source of truth for claimable amounts (zero-mock)
    let snapshot = state.latest_snapshot.lock().await.clone();
    let snapshot = match snapshot {
        Some(s) => s,
        None => {
            let last_error = state.last_ingest_error.lock().ok().and_then(|slot| slot.clone());
            return (
                StatusCode::SERVICE_UNAVAILABLE,
                Json(no_snapshot_body(last_error.as_ref().map(|(_, e)| e.as_str()))),
            )
                .into_response();
        }
    };
    let gross_usd = snapshot.data.derived_metrics.global_status.total_fees_claimable_usd.clone();

    let deadline = (Utc::now().timestamp() as u64) + 600;
    let mut actions: Vec<ClaimFeeAction> = Vec::new();

    // --- Per-position Uniswap v4 collect actions ---
    // Closed positions (#144) get no action: the snapshot lists them as closed, their
    // NFT does not exist and there is nothing to collect from it.
    let live_configs = live_position_configs(&state.config.positions, &snapshot.data.closed_positions);
    let v4_configs: Vec<&PositionConfig> =
        live_configs.iter().copied().filter(|p| p.protocol == "uniswap-v4").collect();
    let v3_configs: Vec<&PositionConfig> =
        live_configs.iter().copied().filter(|p| p.protocol == "uniswap-v3").collect();
    let aero_configs: Vec<&PositionConfig> =
        live_configs.iter().copied().filter(|p| p.protocol == "aerodrome").collect();
    let aero_cl_configs: Vec<&PositionConfig> =
        live_configs.iter().copied().filter(|p| p.protocol == AERO_CL_PROTOCOL).collect();
    // A position with an unknown protocol gets no claim action built for some other
    // protocol, and is not silently left out either (issue #91).
    if let Err(e) = validate_positions_config(&state.config.positions) {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(json!({
            "error": "INVALID_POSITION_CONFIG",
            "detail": e.to_string()
        }))).into_response();
    }
    if v4_configs.is_empty() && v3_configs.is_empty() && aero_configs.is_empty() && aero_cl_configs.is_empty() {
        return (StatusCode::SERVICE_UNAVAILABLE, Json(json!({
            "error": "NO_CLAIMABLE_POSITIONS",
            "detail": "No LP positions configured; cannot build claim transactions"
        }))).into_response();
    }

    // Live prices for token-aware USD valuation (degrade to None, never fabricate).
    let eth_price_pool = state.config.eth_price_pool.clone().unwrap_or_else(|| DEFAULT_ETH_PRICE_POOL.to_string());
    let eth_price = fetch_eth_price_usd(&state.http_client, &alchemy_url, &eth_price_pool).await;
    let nock_spot = nock_spot_from_snapshot(&snapshot.data);

    for (idx, pos) in v4_configs.iter().enumerate() {
        let (Some(c0), Some(c1)) = (pos.currency0.as_deref(), pos.currency1.as_deref()) else {
            return (StatusCode::SERVICE_UNAVAILABLE, Json(json!({
                "error": "NO_POSITION_CONFIG",
                "detail": format!("Position {} is missing currency0/currency1 in config", pos.id)
            }))).into_response();
        };
        let v4_data = build_v4_collect_calldata(pos.nft_id, c0, c1, &wallet, deadline);
        let mut action = ClaimFeeAction {
            platform: "uniswap-v4".to_string(),
            label: format!("Collect Uniswap V4 LP fees ({} #{})", pos.id, pos.nft_id),
            to: UNISWAP_V4_POSM_BASE.to_string(),
            data: v4_data,
            value: "0x0".to_string(),
            expected_amount_0: None,
            expected_amount_1: None,
            expected_usd: None,
            gas_estimate: None,
            gas_usd: None,
            simulation: "unavailable".to_string(),
            simulation_detail: None,
        };
        let pos_id = format!("uniswap-v4:{}:{}", pos.network, pos.nft_id);
        finish_claim_action(&state.http_client, &alchemy_url, &wallet, &snapshot,
            &mut action, &pos_id, 20 + idx as i32, nock_spot, eth_price, false).await;
        actions.push(action);
    }

    // --- Per-position Uniswap v3 collect actions ---
    // v3 fees accrue in the canonical NPM (tokensOwed); collect((uint256,address,
    // uint128,uint128)) with type(uint128).max maxes sweeps them to the wallet. Emitted
    // only for positions whose config carries the canonical npm_address (Phase-1
    // verified on Base); positions without one stay read-only (advice: ACCUMULATE).
    for (idx, pos) in v3_configs.iter().enumerate() {
        let Some(npm) = pos.npm_address.as_deref() else {
            continue;
        };
        let v3_data = build_v3_collect_calldata(pos.nft_id, &wallet);
        let mut action = ClaimFeeAction {
            platform: "uniswap-v3".to_string(),
            label: format!("Collect Uniswap V3 LP fees ({} #{})", pos.id, pos.nft_id),
            to: npm.to_string(),
            data: v3_data,
            value: "0x0".to_string(),
            expected_amount_0: None,
            expected_amount_1: None,
            expected_usd: None,
            gas_estimate: None,
            gas_usd: None,
            simulation: "unavailable".to_string(),
            simulation_detail: None,
        };
        let pos_id = format!("uniswap-v3:{}:{}", pos.network, pos.nft_id);
        finish_claim_action(&state.http_client, &alchemy_url, &wallet, &snapshot,
            &mut action, &pos_id, 20 + (v4_configs.len() + idx) as i32, nock_spot, eth_price, false).await;
        actions.push(action);
    }

    // --- Aerodrome volatile-pool fee claims (issue #88) ---
    // claimFees() on the pool pays the caller's accrued swap fees; selector 0xd294f093
    // verified against this pool (ONCHAIN_INTEGRATION.md §2).
    for (idx, pos) in aero_configs.iter().enumerate() {
        let Some(pool) = pos.pool_address.as_deref() else {
            continue;
        };
        let mut action = ClaimFeeAction {
            platform: "aerodrome".to_string(),
            label: format!("Claim Aero LP fees ({})", pos.id),
            to: pool.to_string(),
            data: "0xd294f093".to_string(),
            value: "0x0".to_string(),
            expected_amount_0: None,
            expected_amount_1: None,
            expected_usd: None,
            gas_estimate: None,
            gas_usd: None,
            simulation: "unavailable".to_string(),
            simulation_detail: None,
        };
        let pos_id = position_key(pos);
        finish_claim_action(&state.http_client, &alchemy_url, &wallet, &snapshot,
            &mut action, &pos_id, 40 + idx as i32, nock_spot, eth_price, true).await;
        // Not LP-claimable: say what the position earns instead and where it is
        // collected (#142). No transaction is built for it.
        state_reward_on_not_claimable(&mut action, snapshot.data.positions.iter().find(|p| p.position_id == pos_id));
        actions.push(action);
    }

    // --- Aero CL positions (issue #91) ---
    // An unstaked NFT's fees are collected from the Aero CL position manager with the
    // same collect((uint256,address,uint128,uint128)) call as Uniswap v3 (verified on
    // that contract, ONCHAIN_INTEGRATION.md §8). A STAKED NFT has nothing to collect:
    // swap fees on staked liquidity go to the gauge, and collect() from the wallet
    // reverts because the gauge holds the NFT. The snapshot says which it is.
    for (idx, pos) in aero_cl_configs.iter().enumerate() {
        let Some(npm) = pos.npm_address.as_deref() else {
            continue;
        };
        let pos_id = position_key(pos);
        let mut action = ClaimFeeAction {
            platform: AERO_CL_PROTOCOL.to_string(),
            label: format!("Collect Aero CL LP fees ({} #{})", pos.id, pos.nft_id),
            to: npm.to_string(),
            data: build_v3_collect_calldata(pos.nft_id, &wallet),
            value: "0x0".to_string(),
            expected_amount_0: None,
            expected_amount_1: None,
            expected_usd: None,
            gas_estimate: None,
            gas_usd: None,
            simulation: "unavailable".to_string(),
            simulation_detail: None,
        };
        let snap_pos = snapshot.data.positions.iter().find(|p| p.position_id == pos_id);
        if !aero_cl_mark_not_claimable(&mut action, snap_pos) {
            finish_claim_action(&state.http_client, &alchemy_url, &wallet, &snapshot,
                &mut action, &pos_id, 60 + idx as i32, nock_spot, eth_price, false).await;
        }
        state_reward_on_not_claimable(&mut action, snap_pos);
        actions.push(action);
    }

    // --- Gas estimation per action (live; degrades to None, never fabricated) ---
    let gas_payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_gasPrice",
        "params": [],
        "id": 22
    });
    let gas_price_wei: Option<u128> = match post_rpc_with_retry(&state.http_client, &alchemy_url, &gas_payload, 2).await {
        Some(res_json) => res_json
            .get("result")
            .and_then(|r| r.as_str())
            .and_then(|s| u128::from_str_radix(s.trim_start_matches("0x"), 16).ok()),
        None => None,
    };
    for (idx, action) in actions.iter_mut().enumerate() {
        // A pool whose claimFees() reverts has nothing to execute: no estimate, and it
        // stays out of the totals (claim_payload_totals).
        if action.simulation == "not_claimable" {
            continue;
        }
        let est_payload = json!({
            "jsonrpc": "2.0",
            "method": "eth_estimateGas",
            "params": [{ "from": wallet, "to": action.to, "data": action.data, "value": action.value }, "latest"],
            "id": 30 + idx
        });
        match post_rpc_with_retry(&state.http_client, &alchemy_url, &est_payload, 2).await {
            Some(res_json) => match res_json.get("result").and_then(|r| r.as_str()).and_then(|s| u128::from_str_radix(s.trim_start_matches("0x"), 16).ok()) {
                Some(units) => {
                    action.gas_estimate = Some(format!("0x{:x}", units));
                    // Zero-mock: gas USD requires BOTH the live gas price and the live
                    // ETH/USD spot; if either is unavailable the USD valuation degrades
                    // to None instead of being valued at a fabricated price.
                    if let (Some(gp), Some(ep)) = (gas_price_wei, eth_price) {
                        let usd = (units as f64) * (gp as f64) / 1e18 * ep;
                        action.gas_usd = Some(format!("{:.4}", usd));
                    }
                }
                None => {
                    action.simulation_detail = Some(action.simulation_detail.clone().unwrap_or_default()
                        + "; gas estimation failed");
                }
            },
            None => {}
        }
    }
    let (estimated_gas_usd, net_after_gas_usd) =
        claim_payload_totals(&actions, &gross_usd, gas_price_wei.is_some() && eth_price.is_some());

    // --- Gauge rewards (#142): reported beside the LP fees, never as an action ---
    // A gas figure is estimated only for a claim call proven for that gauge
    // (rewards.rs); it is a read-only eth_estimateGas from the wallet. The rows carry
    // no calldata, so nothing here can be dispatched.
    let reward_gas = estimate_reward_claim_gas(
        &state.http_client,
        &alchemy_url,
        &wallet,
        &snapshot.data.positions,
        eth_price,
        // The gas price read above for the LP actions: one price for the whole payload.
        GasPrice::Known(gas_price_wei),
        std::time::Duration::from_millis(REWARD_GAS_BUDGET_MS),
    )
    .await;
    let rewards = claim_reward_rows(&snapshot.data.positions, &reward_gas);
    let global = &snapshot.data.derived_metrics.global_status;

    let payload = ClaimFeesPayload {
        chain_id: 8453,
        from: wallet,
        generated_at: Utc::now().to_rfc3339(),
        gross_usd: Some(gross_usd.clone()),
        estimated_gas_usd,
        net_after_gas_usd,
        actions,
        collectible_lp_fees_usd: Some(gross_usd),
        collectible_rewards_usd: global.collectible_rewards_usd.clone(),
        collectible_rewards_note: global.collectible_rewards_note.clone(),
        total_collectible_usd: global.total_collectible_usd.clone(),
        rewards,
    };
    Json(payload).into_response()
}
