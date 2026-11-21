use crate::*;

/// Live gas statements for the rewards in a snapshot, for `simulate_claim_fees`.
/// Read-only and bounded (REWARD_GAS_BUDGET_MS). Without a wallet or an RPC key every
/// reward says why it has no estimate; nothing is guessed.
async fn simulation_reward_gas(state: &AppState, data: &SnapshotData) -> Vec<RewardGas> {
    let unavailable = |why: &str| -> Vec<RewardGas> {
        data.positions
            .iter()
            .map(|p| match reward_gas_request(p, "") {
                Ok(_) => RewardGas::Failed(why.to_string()),
                Err(gas) => gas,
            })
            .collect()
    };
    let Some(wallet) = state.config.tracked_wallets.first() else {
        return unavailable("no tracked wallet is configured");
    };
    let Some(rpc_url) = request_rpc_url(&state.config.alchemy_api_keys) else {
        return unavailable("no RPC key is configured");
    };
    let eth_price = data
        .derived_metrics
        .global_status
        .eth_price_usd
        .as_deref()
        .and_then(|p| p.trim().parse::<f64>().ok());
    estimate_reward_claim_gas(
        &state.http_client,
        &rpc_url,
        wallet,
        &data.positions,
        eth_price,
        GasPrice::ReadNow,
        std::time::Duration::from_millis(REWARD_GAS_BUDGET_MS),
    )
    .await
}

pub(crate) async fn mcp_http_handler(
    Extension(state): Extension<Arc<AppState>>,
    headers: HeaderMap,
    Json(payload): Json<serde_json::Value>,
) -> impl IntoResponse {
    // Validate Bearer token if configured
    if let Some(ref required_token) = state.config.mcp_bearer_token {
        let auth_header = headers.get("authorization").and_then(|h| h.to_str().ok());
        let provided_token = auth_header.and_then(|h| {
            h.strip_prefix("Bearer ")
                .or_else(|| h.strip_prefix("bearer "))
        });
        if provided_token != Some(required_token.as_str()) {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({
                    "jsonrpc": "2.0",
                    "id": payload.get("id"),
                    "error": {
                        "code": -32001,
                        "message": "Unauthorized: Invalid or missing Bearer token"
                    }
                })),
            );
        }
    }

    let method = payload.get("method").and_then(|m| m.as_str()).unwrap_or_default();
    let id = payload.get("id").cloned();

    let response = match method {
        "initialize" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "protocolVersion": "2024-11-05",
                "serverInfo": {
                    "name": "nocknock-mcp",
                    "version": env!("CARGO_PKG_VERSION")
                },
                "capabilities": {
                    "tools": {}
                }
            }
        }),
        "ping" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {}
        }),
        "tools/list" => json!({
            "jsonrpc": "2.0",
            "id": id,
            "result": {
                "tools": [
                    {
                        "name": "get_portfolio_state",
                        "description": "Returns current sovereign portfolio balances, NOCK floor buffer delta, the floor-crossing price (how far NOCK can rise before the deployed ranges sell the stack below the floor, a measurement), native token balance, active tranche allocations, and the average cost of the plan's verified NOCK buys against spot (plus the whole-stack average when an opening basis is configured).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {},
                            "required": []
                        }
                    },
                    {
                        "name": "get_shot_advice",
                        "description": "Returns the active tactical directive (e.g. HOLD & ACCRETE, BATCH HARVEST SHOT, DEFENSIVE LOCK) and break-even gas economics, with uncollected LP fees, collectible AERO rewards and their total as separate labelled lines. The harvest rule counts LP fees only.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {},
                            "required": []
                        }
                    },
                    {
                        "name": "simulate_claim_fees",
                        "description": "Simulates gas expenditure vs uncollected LP fees across the tracked LP positions, and states each staked Aero position's collectible gauge reward: the amount, the gauge it is collected from (in the Aero app) and a gas estimate only where the claim call is verified on-chain. LP fees, AERO rewards and their total are separate labelled figures. Reports amounts only: no advice about AERO, and no transaction is built for a reward.",
                        "inputSchema": {
                            "type": "object",
                            "properties": {},
                            "required": []
                        }
                    },
                    {
                        "name": "get_accumulation_plan",
                        "description": "Returns the dollar-cost-averaging plan: whether a NOCK tranche is due, its size, progress from receipt-verified buys, average cost, swaps found on-chain that await the operator's confirmation (listed only; this tool confirms nothing), the LP-rebuild bias and, while a tranche is due, read-only route quotes for that amount on Aero and Uniswap (a comparison, not a recommendation).",
                        "inputSchema": {
                            "type": "object",
                            "properties": {},
                            "required": []
                        }
                    }
                ]
            }
        }),
        "tools/call" => {
            let tool_name = payload
                .get("params")
                .and_then(|p| p.get("name"))
                .and_then(|n| n.as_str())
                .unwrap_or_default();

            // Cloned so the lock is not held across the live gas estimate below.
            let latest: Option<SnapshotEnvelope> = state.latest_snapshot.lock().await.clone();
            let last_error: Option<String> =
                state.last_ingest_error.lock().ok().and_then(|slot| slot.clone()).map(|(_, e)| e);
            let result_text = if let Some(ref env) = latest {
                let data_val = serde_json::to_value(&env.data).unwrap_or_default();
                let body = match tool_name {
                    "get_portfolio_state" => format_portfolio_state_text(&data_val),
                    "get_shot_advice" => format_shot_advice_text(&data_val),
                    // Gauge rewards (#142): the gas of collecting one is estimated now,
                    // read-only, and only for a claim call proven for that gauge.
                    "simulate_claim_fees" => {
                        let gas = simulation_reward_gas(&state, &env.data).await;
                        claim_simulation_text(&data_val, &gas)
                    }
                    "get_accumulation_plan" => format_accumulation_plan_text(&data_val),
                    _ => format!("Unknown tool: {tool_name}"),
                };
                let now = Utc::now();
                let lag = now.signed_duration_since(env.meta.generated_at).num_seconds().max(0);
                let health = health_status(true, lag, state.config.sync_window_seconds as i64, last_error.is_some());
                format!("{body}\n{}", snapshot_source_line(env.meta.generated_at, now, health))
            } else {
                no_snapshot_mcp_text(last_error.as_deref())
            };

            json!({
                "jsonrpc": "2.0",
                "id": id,
                "result": {
                    "content": [
                        {
                            "type": "text",
                            "text": result_text
                        }
                    ]
                }
            })
        }
        _ => json!({
            "jsonrpc": "2.0",
            "id": id,
            "error": {
                "code": -32601,
                "message": format!("Method not found: {method}")
            }
        }),
    };

    (StatusCode::OK, Json(response))
}
