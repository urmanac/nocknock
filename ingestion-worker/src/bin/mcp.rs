use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::time::Duration;

const DEFAULT_BASE_RPC: &str = "https://mainnet.base.org";
const LOCAL_API_URL: &str = "http://127.0.0.1:8080/v1/snapshot/latest";
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
                                "description": "Returns current sovereign portfolio balances, NOCK floor buffer delta, native token balance, and tracked Uniswap LP position totals.",
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
                                "description": "Simulates gas expenditure vs uncollected yield across the tracked Uniswap v3/v4 positions.",
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
    // 1. Try local Tier 1 HTTP MCP endpoint first
    let mcp_payload = json!({
        "jsonrpc": "2.0",
        "id": 1,
        "method": "tools/call",
        "params": {
            "name": tool_name
        }
    });
    if let Ok(resp) = client.post("http://127.0.0.1:8080/v1/mcp").json(&mcp_payload).timeout(Duration::from_secs(2)).send().await {
        if resp.status().is_success() {
            if let Ok(mcp_val) = resp.json::<Value>().await {
                if let Some(text) = mcp_val.pointer("/result/content/0/text").and_then(|t| t.as_str()) {
                    return text.to_string();
                }
            }
        }
    }

    // Fallback: try snapshot API
    if let Ok(resp) = client.get(LOCAL_API_URL).timeout(Duration::from_secs(2)).send().await {
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
    // Direct decentralized query to Base mainnet. The Aerodrome tranche was liquidated
    // on 2026-10-23; the tracked LP is now Uniswap-only, so the NOCK spot comes from
    // the tracked v4 NOCK/USDC pool slot0 (discovery-verified identifiers).
    const NOCK_USDC_POOL_ID: &str = "0x91a9ef0a8dd231b99fa53a2a145a9e8bca7897e0d85852107abb997916f9f99c";
    const NOCK_TOKEN: &str = "0x9B5E262cF9bb04869ab40b19AF91D2dc85761722";

    let slot0 = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": "0xa3c0c9b65bad0b08107aa264b0f3db444b867a71", // Uniswap v4 StateView
            "data": format!("0xc815641c{}", NOCK_USDC_POOL_ID.trim_start_matches("0x"))
        }, "latest"],
        "id": 1
    });

    let nock_balance_call = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{
            "to": NOCK_TOKEN,
            "data": format!("0x70a082310000000000000000{}", WALLET_ADDR.trim_start_matches("0x"))
        }, "latest"],
        "id": 2
    });

    let mut spot_price: Option<f64> = None;
    let mut nock_balance: Option<f64> = None;

    if let Ok(resp) = client.post(DEFAULT_BASE_RPC).json(&slot0).send().await {
        if let Ok(val) = resp.json::<Value>().await {
            if let Some(hex_str) = val.get("result").and_then(|r| r.as_str()) {
                let clean = hex_str.trim_start_matches("0x");
                if clean.len() >= 64 {
                    if let Ok(s) = u128::from_str_radix(&clean[..64], 16) {
                        // token0 = USDC (6 dec), token1 = NOCK (16 dec):
                        // raw = NOCK_raw per USDC_raw -> USDC per NOCK = 1e10 / raw
                        let raw = (s as f64 / 79228162514264337593543950336.0).powi(2);
                        if raw > 0.0 {
                            spot_price = Some(1e10 / raw);
                        }
                    }
                }
            }
        }
    }

    if let Ok(resp) = client.post(DEFAULT_BASE_RPC).json(&nock_balance_call).send().await {
        if let Ok(val) = resp.json::<Value>().await {
            if let Some(hex_str) = val.get("result").and_then(|r| r.as_str()) {
                if let Ok(v) = u128::from_str_radix(hex_str.trim_start_matches("0x"), 16) {
                    nock_balance = Some(v as f64 / 1e16);
                }
            }
        }
    }

    let price_str = spot_price
        .map(|p| format!("${:.6} USD", p))
        .unwrap_or_else(|| "Unavailable (RPC query failed)".to_string());
    let nock_str = nock_balance
        .map(|b| format!("{:.2} NOCK", b))
        .unwrap_or_else(|| "Unavailable".to_string());
    let nock_usd_str = match (nock_balance, spot_price) {
        (Some(b), Some(p)) => format!("~${:.2} USD", b * p),
        _ => "Unavailable".to_string(),
    };

    let advice = "ACCRUE (fee claims need the worker's simulated Golden Path payload; open the cockpit)";

    match tool_name {
        "get_portfolio_state" => format!(
            "📈 NockNock Sovereign State (Decentralized Direct RPC Mode)\n\
             ===========================================================\n\
             • NOCK Spot Price: {price_str}\n\
             • Wallet NOCK Balance: {nock_str} ({nock_usd_str})\n\
             • LP Positions: Uniswap-only portfolio (full breakdown needs the worker/DB)\n\
             • Mode: Zero-DB Edge RPC Fallback (Database and Worker Offline)\n\
             • Status: Sovereign RPC Connected directly to Base mainnet."
        ),
        "get_shot_advice" => format!(
            "🎯 Tactical Shot Advice Engine (Decentralized RPC Mode)\n\
             =======================================================\n\
             • Recommended Action: {advice}\n\
             • NOCK Spot Price: {price_str}\n\
             • Gas Estimate: unavailable in zero-DB mode\n\
             • Mode: Zero-DB Edge RPC Fallback"
        ),
        "simulate_claim_fees" => format!(
            "⚡ Fee Claim Simulation (Decentralized RPC Mode)\n\
             ===============================================\n\
             • Unclaimed LP fees: not simulated in zero-DB mode (no fabricated estimates)\n\
             • NOCK Spot Price: {price_str}\n\
             • Estimated Gas Cost: unavailable in zero-DB mode\n\
             • Claim decision needs the worker's pre-simulated payload — open the cockpit"
        ),
        _ => format!("Unknown tool: {tool_name}"),
    }
}
