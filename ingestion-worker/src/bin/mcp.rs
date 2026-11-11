use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::time::Duration;

const DEFAULT_BASE_RPC: &str = "https://mainnet.base.org";
#[allow(dead_code)]
const LOCAL_API_URL: &str = "http://127.0.0.1:8081/v1/snapshot/latest";
const WALLET_ADDR: &str = "0xeE114cC97eC5adeB34bA5645e315B497D8487a45";
#[allow(dead_code)]
const WALLET_PADDED: &str = "000000000000000000000000ee114cc97ec5adeb34ba5645e315b497d8487a45";

#[tokio::main]
async fn main() -> Result<()> {
    let client = Client::builder()
        .timeout(Duration::from_secs(5))
        .build()?;

    let stdin = io::stdin();
    let stdout = io::stdout();
    let mut handle = stdout.lock();

    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(_) => break,
        };

        if line.trim().is_empty() {
            continue;
        }

        let req: Value = match serde_json::from_str(&line) {
            Ok(v) => v,
            Err(_) => continue,
        };

        let method = req.get("method").and_then(|m| m.as_str()).unwrap_or_default();
        let id = req.get("id").cloned();

        match method {
            "initialize" => {
                let resp = json!({
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
                });
                writeln!(handle, "{}", resp)?;
                handle.flush()?;
            }
            "notifications/initialized" => {
                // Client confirmed initialization; no response needed
            }
            "ping" => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {}
                });
                writeln!(handle, "{}", resp)?;
                handle.flush()?;
            }
            "tools/list" => {
                let resp = json!({
                    "jsonrpc": "2.0",
                    "id": id,
                    "result": {
                        "tools": [
                            {
                                "name": "get_portfolio_state",
                                "description": "Returns current sovereign portfolio balances, NOCK floor buffer delta, native token balance, and active tranche allocations.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {},
                                    "required": []
                                }
                            },
                            {
                                "name": "get_shot_advice",
                                "description": "Returns the active tactical directive (e.g. HOLD & ACCRETE, BATCH HARVEST SHOT, DEFENSIVE LOCK) and break-even gas economics.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {},
                                    "required": []
                                }
                            },
                            {
                                "name": "simulate_claim_fees",
                                "description": "Simulates gas expenditure vs uncollected yield across Aerodrome and Uniswap V4 positions.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {},
                                    "required": []
                                }
                            }
                        ]
                    }
                });
                writeln!(handle, "{}", resp)?;
                handle.flush()?;
            }
            "tools/call" => {
                let tool_name = req
                    .get("params")
                    .and_then(|p| p.get("name"))
                    .and_then(|n| n.as_str())
                    .unwrap_or_default();

                let result_text = handle_tool_call(&client, tool_name).await;
                let resp = json!({
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
                });
                writeln!(handle, "{}", resp)?;
                handle.flush()?;
            }
            _ => {
                if let Some(req_id) = id {
                    let err_resp = json!({
                        "jsonrpc": "2.0",
                        "id": req_id,
                        "error": {
                            "code": -32601,
                            "message": format!("Method not found: {method}")
                        }
                    });
                    writeln!(handle, "{}", err_resp)?;
                    handle.flush()?;
                }
            }
        }
    }

    Ok(())
}

async fn handle_tool_call(client: &Client, tool_name: &str) -> String {
    let port = std::env::var("PORT").unwrap_or_else(|_| "8081".to_string());
    let api_base = std::env::var("NOCKNOCK_API_URL")
        .or_else(|_| std::env::var("API_URL"))
        .unwrap_or_else(|_| format!("http://127.0.0.1:{port}/v1"));
    let mcp_url = format!("{api_base}/mcp");
    let snapshot_url = format!("{api_base}/snapshot/latest");

    // 1. Try local Tier 1 HTTP MCP endpoint first
    let mcp_payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": tool_name
        }
    });
    if let Ok(resp) = client.post(&mcp_url).json(&mcp_payload).timeout(Duration::from_secs(2)).send().await {
        if resp.status().is_success() {
            if let Ok(mcp_val) = resp.json::<Value>().await {
                if let Some(text) = mcp_val.pointer("/result/content/0/text").and_then(|t| t.as_str()) {
                    return text.to_string();
                }
            }
        }
    }

    // Fallback: try snapshot API
    if let Ok(resp) = client.get(&snapshot_url).timeout(Duration::from_secs(2)).send().await {
        if resp.status().is_success() {
            if let Ok(snap) = resp.json::<Value>().await {
                if let Some(data) = snap.get("data") {
                    return match tool_name {
                        "get_portfolio_state" => format_portfolio_state(data),
                        "get_shot_advice" => format_shot_advice(data),
                        "simulate_claim_fees" => format_claim_simulation(data),
                        _ => format!("Unknown tool: {tool_name}"),
                    };
                }
            }
        }
    }

    // 2. Fallback to direct decentralized Base RPC (Zero-DB mode)
    handle_direct_rpc_fallback(client, tool_name).await
}

fn format_portfolio_state(data: &Value) -> String {
    let global = data.pointer("/derived_metrics/global_status");
    let total_usd = global.and_then(|g| g.get("total_portfolio_usd")).and_then(|v| v.as_str()).unwrap_or("0.00");
    let total_nock = global.and_then(|g| g.get("total_nock_amount")).and_then(|v| v.as_str()).unwrap_or("0");
    let nock_delta = global.and_then(|g| g.get("nock_floor_delta")).and_then(|v| v.as_str()).unwrap_or("0");
    let total_usdc = global.and_then(|g| g.get("total_usdc_amount")).and_then(|v| v.as_str()).unwrap_or("0.00");
    let hwm = global.and_then(|g| g.get("high_water_mark_usd")).and_then(|v| v.as_str()).unwrap_or("0.00");
    let advice = global.and_then(|g| g.get("shot_advice_action")).and_then(|v| v.as_str()).unwrap_or("HOLD");

    let delta_prefix = if nock_delta.starts_with('-') { "" } else { "+" };

    format!(
        "📈 NockNock Sovereign Portfolio State (Live API)\n\
         =========================================\n\
         • Total Value: ${total_usd} USD\n\
         • Sovereign NOCK Stack: {total_nock} NOCK (Floor buffer delta: {delta_prefix}{nock_delta} above 100k)\n\
         • Stable Stack: {total_usdc} USDC\n\
         • High-Water Mark: ${hwm} USD\n\
         • Tactical Directive: {advice}\n\
         • Mode: Tier 1 Connected (Verified Persistent Snapshot)"
    )
}

fn format_shot_advice(data: &Value) -> String {
    let global = data.pointer("/derived_metrics/global_status");
    let action = global.and_then(|g| g.get("shot_advice_action")).and_then(|v| v.as_str()).unwrap_or("HOLD");
    let details = global.and_then(|g| g.get("shot_advice_details")).and_then(|v| v.as_str()).unwrap_or("Accreting");
    let claimable_usd = global.and_then(|g| g.get("total_fees_claimable_usd")).and_then(|v| v.as_str()).unwrap_or("0.00");
    let gas_delta = global.and_then(|g| g.get("gas_to_fee_delta_usd")).and_then(|v| v.as_str()).unwrap_or("0.00");

    format!(
        "🎯 Tactical Shot Advice Engine\n\
         ===============================\n\
         • Recommended Action: {action}\n\
         • Decision Matrix Details: {details}\n\
         • Uncollected Claimable Fees: ${claimable_usd} USD\n\
         • Net Gas-to-Fee Profitability: +${gas_delta} USD\n\
         • Execution rule: claim advised when claimable yield ≥ 2× live gas break-even."
    )
}

fn format_claim_simulation(data: &Value) -> String {
    let fees = data.get("contract_fees").and_then(|f| f.as_array());
    let mut details = String::new();
    let mut total_claimable_usd = 0.0f64;
    let mut gas_usd: Option<f64> = None;

    if let Some(list) = fees {
        for fee in list {
            let symbol = fee.get("fee_token_symbol").and_then(|s| s.as_str()).unwrap_or("Pool");
            let amount = fee.get("amount").and_then(|a| a.as_str()).unwrap_or("0");
            let amount_usd = fee.get("amount_usd").and_then(|a| a.as_str()).unwrap_or("0.00");
            let gas_str = fee.get("gas_break_even_usd").and_then(|g| g.as_str()).unwrap_or_default();
            match gas_str.parse::<f64>() {
                Ok(g) => {
                    // Live per-contract gas break-even; take the max as representative.
                    gas_usd = Some(match gas_usd {
                        Some(acc) => acc.max(g),
                        None => g,
                    });
                    details.push_str(&format!("  - {symbol}: {amount} (~${amount_usd} USD | Gas Est: ${:.4})\n", g));
                }
                Err(_) => {
                    details.push_str(&format!("  - {symbol}: {amount} (~${amount_usd} USD | Gas Est: unavailable)\n"));
                }
            }
            if let Ok(usd_val) = amount_usd.parse::<f64>() {
                total_claimable_usd += usd_val;
            }
        }
    }

    let (gas_line, net_line) = match gas_usd {
        Some(g) => (
            format!("~${:.4} USD", g),
            format!("${:.4} USD", (total_claimable_usd - g).max(0.0)),
        ),
        None => (
            "unavailable".to_string(),
            "unknown (live gas estimate unavailable)".to_string(),
        ),
    };
    format!(
        "⚡ Fee Claim Simulation\n\
         =======================\n\
         • Total Gross Claimable: ${:.4} USD\n\
         • Estimated Gas: {}\n\
         • Net Accretion if Harvested: {}\n\
         Pool Breakdown:\n{}",
        total_claimable_usd, gas_line, net_line, details
    )
}

async fn handle_direct_rpc_fallback(client: &Client, tool_name: &str) -> String {
    // Direct decentralized query to Base node
    let aero_reserves = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": "0x85f1aa3a70fedd1c52705c15baed143e675cd626",
            "data": "0x0902f1ac"
        }, "latest"],
        "id": 1
    });

    let aero_claim = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "from": WALLET_ADDR,
            "to": "0x85f1aa3a70fedd1c52705c15baed143e675cd626",
            "data": "0xd294f093"
        }, "latest"],
        "id": 2
    });

    let mut spot_price: Option<f64> = None;
    let mut claim_usdc = 0.0f64;
    let mut claim_nock = 0.0f64;

    if let Ok(resp) = client.post(DEFAULT_BASE_RPC).json(&aero_reserves).send().await {
        if let Ok(val) = resp.json::<Value>().await {
            if let Some(hex_str) = val.get("result").and_then(|r| r.as_str()) {
                let clean = hex_str.trim_start_matches("0x");
                if clean.len() >= 128 {
                    if let (Ok(r0), Ok(r1)) = (
                        u128::from_str_radix(&clean[..64], 16),
                        u128::from_str_radix(&clean[64..128], 16),
                    ) {
                        let usdc = r0 as f64 / 1e6;
                        let nock = r1 as f64 / 1e16;
                        if nock > 0.0 {
                            spot_price = Some(usdc / nock);
                        }
                    }
                }
            }
        }
    }

    if let Ok(resp) = client.post(DEFAULT_BASE_RPC).json(&aero_claim).send().await {
        if let Ok(val) = resp.json::<Value>().await {
            if let Some(hex_str) = val.get("result").and_then(|r| r.as_str()) {
                let clean = hex_str.trim_start_matches("0x");
                if clean.len() >= 128 {
                    if let (Ok(c0), Ok(c1)) = (
                        u128::from_str_radix(&clean[..64], 16),
                        u128::from_str_radix(&clean[64..128], 16),
                    ) {
                        claim_usdc = c0 as f64 / 1e6;
                        claim_nock = c1 as f64 / 1e16;
                    }
                }
            }
        }
    }

    let price_str = spot_price
        .map(|p| format!("${:.6} USD", p))
        .unwrap_or_else(|| "Unavailable (RPC query failed)".to_string());

    let claim_usd = spot_price.map(|p| claim_usdc + claim_nock * p);
    let claim_usd_str = claim_usd
        .map(|c| format!("${:.4} USD", c))
        .unwrap_or_else(|| "Pending spot price".to_string());

    // Zero-DB mode has no live gas feed, so the ≥2× gas claim decision cannot be made
    // here. Report the accrual state honestly instead of a fabricated $0.005 gas or a
    // hardcoded $5.00 trigger.
    let advice = "ACCRUE (live gas unavailable in zero-DB mode; open the cockpit for a live claim decision)";

    match tool_name {
        "get_portfolio_state" => format!(
            "📈 NockNock Sovereign State (Decentralized Direct RPC Mode)\n\
             ===========================================================\n\
             • NOCK Spot Price: {price_str}\n\
             • Uncollected Aerodrome Yield: {claim_usd_str}\n\
             • Mode: Zero-DB Edge RPC Fallback (Database and Worker Offline)\n\
             • Status: Sovereign RPC Connected directly to Base mainnet."
        ),
        "get_shot_advice" => format!(
            "🎯 Tactical Shot Advice Engine (Decentralized RPC Mode)\n\
             =======================================================\n\
             • Recommended Action: {advice}\n\
             • Active Accrued Yield: {claim_usd_str}\n\
             • Gas Estimate: unavailable in zero-DB mode\n\
             • Mode: Zero-DB Edge RPC Fallback"
        ),
        "simulate_claim_fees" => format!(
            "⚡ Fee Claim Simulation (Decentralized RPC Mode)\n\
             ===============================================\n\
             • Uncollected Aerodrome Fees: {:.5} USDC + {:.4} NOCK\n\
             • Total USD Value: {claim_usd_str}\n\
             • Estimated Gas Cost: unavailable in zero-DB mode\n\
             • Net Yield: {} (claim decision needs live gas — open the cockpit)",
            claim_usdc,
            claim_nock,
            claim_usd
                .map(|c| format!("${:.4} USD (gross)", c))
                .unwrap_or_else(|| "Pending spot price".to_string())
        ),
        _ => format!("Unknown tool: {tool_name}"),
    }
}
