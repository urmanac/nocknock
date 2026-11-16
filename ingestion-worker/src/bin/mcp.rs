use anyhow::Result;
use reqwest::Client;
use serde_json::{json, Value};
use std::io::{self, BufRead, Write};
use std::time::Duration;

/// Shared with the worker (src/cost_basis_text.rs) so both transports print the same
/// cost-basis lines. This binary cannot import from main.rs, so it compiles the file.
#[path = "../cost_basis_text.rs"]
mod cost_basis_text;

/// Worker base URL when NOCKNOCK_API_URL is unset. Point the variable at a remote
/// deployment (e.g. `http://chudmax:8081`) to read it instead of a local worker.
const DEFAULT_API_BASE: &str = "http://127.0.0.1:8080";

fn api_base() -> String {
    std::env::var("NOCKNOCK_API_URL")
        .ok()
        .map(|v| v.trim().trim_end_matches('/').to_string())
        .filter(|v| !v.is_empty())
        .unwrap_or_else(|| DEFAULT_API_BASE.to_string())
}

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
                                "description": "Returns current sovereign portfolio balances, NOCK floor buffer delta, native token balance, tracked LP position totals, and the average cost of the plan's verified NOCK buys against spot (plus the whole-stack average when an opening basis is configured).",
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
                                "description": "Simulates gas expenditure vs uncollected LP fees across the tracked LP positions.",
                                "inputSchema": {
                                    "type": "object",
                                    "properties": {},
                                    "required": []
                                }
                            },
                            {
                                "name": "get_accumulation_plan",
                                "description": "Returns the dollar-cost-averaging plan: whether a NOCK tranche is due, its size, progress from receipt-verified buys, average cost, and the LP-rebuild bias.",
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
    if let Ok(resp) = client.post(format!("{}/v1/mcp", api_base())).json(&mcp_payload).timeout(Duration::from_secs(2)).send().await {
        if resp.status().is_success() {
            if let Ok(mcp_val) = resp.json::<Value>().await {
                if let Some(text) = mcp_val.pointer("/result/content/0/text").and_then(|t| t.as_str()) {
                    return text.to_string();
                }
            }
        }
    }

    // Fallback: try snapshot API
    if let Ok(resp) = client.get(format!("{}/v1/snapshot/latest", api_base())).timeout(Duration::from_secs(2)).send().await {
        let status = resp.status();
        if let Ok(snap) = resp.json::<Value>().await {
            if status.is_success() {
                if let Some(data) = snap.get("data") {
                    let body = match tool_name {
                        "get_portfolio_state" => format_portfolio_state(data),
                        "get_shot_advice" => format_shot_advice(data),
                        "simulate_claim_fees" => format_claim_simulation(data),
                        "get_accumulation_plan" => format_accumulation_plan(data),
                        _ => return format!("Unknown tool: {tool_name}"),
                    };
                    let health = worker_health(client).await;
                    return format!("{body}\n{}", snapshot_source_line(&snap, health.as_deref()));
                }
            } else if let Some(text) = no_snapshot_text(&snap) {
                // The worker is up but has no snapshot yet: report its stated cause.
                return text;
            }
        }
    }

    // 2. Worker unreachable: report that, and nothing NockNock has not read through it.
    zero_db_fallback_text(tool_name, &api_base())
}

/// The worker's /health verdict ("ok", "degraded", "starting"), if it answers.
async fn worker_health(client: &Client) -> Option<String> {
    let resp = client.get(format!("{}/health", api_base())).timeout(Duration::from_secs(2)).send().await.ok()?;
    let v = resp.json::<Value>().await.ok()?;
    v.get("status").and_then(|s| s.as_str()).map(|s| s.to_string())
}

/// When the snapshot was generated and the worker's health, in place of the old
/// "Live API" label that was printed for any snapshot however old. Pure.
fn snapshot_source_line(snap: &Value, health: Option<&str>) -> String {
    let generated = snap
        .pointer("/meta/generated_at")
        .and_then(|v| v.as_str())
        .unwrap_or("at an unknown time");
    format!(
        "Snapshot generated {generated} · worker health: {}",
        health.unwrap_or("unavailable")
    )
}

/// Text for a 503 NO_SNAPSHOT body, carrying the worker's last ingestion error. Pure.
fn no_snapshot_text(body: &Value) -> Option<String> {
    if body.get("error").and_then(|e| e.as_str()) != Some("NO_SNAPSHOT") {
        return None;
    }
    Some(match body.get("detail").and_then(|d| d.as_str()).filter(|d| !d.trim().is_empty()) {
        Some(d) => format!("No snapshot available yet. Last ingestion error: {d}"),
        None => "No snapshot available yet; the first ingestion cycle has not completed.".to_string(),
    })
}

/// Same text as the worker's HARVEST_RULE_TEXT (main.rs): the directive engine's precedence.
const HARVEST_RULE_TEXT: &str = "BATCH HARVEST is advised once uncollected LP fees reach the fixed $5.00 threshold, and only when no DCA tranche is due (a floor DEFENSIVE LOCK, a due tranche and a range exit all outrank it).";

/// Compact plan view for the snapshot-API fallback (the worker's own /v1/mcp gives
/// the full text and is tried first).
fn format_accumulation_plan(data: &Value) -> String {
    let lp = data.pointer("/derived_metrics/lp_rebuild");
    let lp_status = lp.and_then(|l| l.get("status")).and_then(|v| v.as_str()).unwrap_or("n/a");
    let lp_gap = lp.and_then(|l| l.get("gap_usd")).and_then(|v| v.as_str()).unwrap_or("n/a");
    let lp_line = if lp_status == "no_positions" {
        "• No LP positions are tracked; NockNock suggests deposits only into positions you list under positions:".to_string()
    } else if lp_status == "no_target" {
        "• No LP target is configured (operator_profile.target_weights_bps.uniswap_lp), so NockNock suggests no rebuild.".to_string()
    } else {
        format!("• LP rebuild bias: {lp_status} (${lp_gap} short of target; rebuilding is a reposition, no net cash change)")
    };
    match data.pointer("/derived_metrics/accumulation_plan").filter(|p| !p.is_null()) {
        None => format!("🪜 Accumulation Plan\n====================\n• No accumulation_plan is configured.\n{lp_line}"),
        Some(plan) => {
            let s = |k: &str| plan.get(k).and_then(|v| v.as_str()).unwrap_or("n/a").to_string();
            let n = |k: &str| plan.get(k).and_then(|v| v.as_u64()).unwrap_or(0);
            // Same rule as the worker's text: only a due tranche is "due now".
            let next = match s("status").as_str() {
                "complete" => "none (plan complete)".to_string(),
                "tranche_due" => format!(
                    "{} of {} — ${} due now (nominal ${})",
                    n("tranche_index"), n("tranches_total"), s("tranche_usdc"), s("nominal_tranche_usdc")
                ),
                _ => format!("{} of {} — ${} when due", n("tranche_index"), n("tranches_total"), s("nominal_tranche_usdc")),
            };
            format!(
                "🪜 Accumulation Plan \"{}\"\n====================\n\
                 • Status: {}\n\
                 • Next tranche: {}\n\
                 • Next due at: {}\n\
                 • Progress: ${} of ${} spent; average ${}\n\
                 {lp_line}",
                s("plan_id"), s("status"), next,
                s("next_due_at"), s("spent_usdc"), s("budget_usdc"), s("avg_price_usd"),
            )
        }
    }
}

/// Floor clause from the snapshot's own `nock_floor_target` (never a hardcoded 100k).
fn floor_clause(global: Option<&Value>) -> String {
    let get = |k: &str| global.and_then(|g| g.get(k)).and_then(|v| v.as_str());
    let target = get("nock_floor_target").and_then(|t| t.trim().parse::<f64>().ok());
    let delta = get("nock_floor_delta").and_then(|d| d.trim().parse::<f64>().ok());
    match (target, delta) {
        (None, _) => "floor unavailable".to_string(),
        (Some(t), _) if t <= 0.0 => "no floor configured".to_string(),
        (Some(t), Some(d)) if d < 0.0 => format!("{:.0} below the {:.0} NOCK floor", -d, t),
        (Some(t), Some(d)) => format!("+{:.0} above the {:.0} NOCK floor", d, t),
        (Some(t), None) => format!("{:.0} NOCK floor; buffer unavailable", t),
    }
}

fn format_portfolio_state(data: &Value) -> String {
    let global = data.pointer("/derived_metrics/global_status");
    // Missing fields say so: never an invented 0.00 or HOLD.
    let get = |k: &str| global.and_then(|g| g.get(k)).and_then(|v| v.as_str()).filter(|v| !v.is_empty());
    let usd = |k: &str| get(k).map(|v| format!("${v} USD")).unwrap_or_else(|| "unavailable".to_string());
    let total_usd = usd("total_portfolio_usd");
    let total_nock = get("total_nock_amount").map(|v| format!("{v} NOCK")).unwrap_or_else(|| "unavailable".to_string());
    let total_usdc = get("total_usdc_amount").map(|v| format!("{v} USDC")).unwrap_or_else(|| "unavailable".to_string());
    let hwm = usd("high_water_mark_usd");
    let advice = get("shot_advice_action").unwrap_or("unavailable");
    let floor = floor_clause(global);
    // Pillars and gas (snapshot 2.3.0, issue #94). Absent fields say so, never $0.00.
    let not_reported = "not reported (snapshot predates 2.3.0)";
    let pillar = |k: &str| get(k).map(|v| format!("${v}")).unwrap_or_else(|| not_reported.to_string());
    let pillars = format!(
        "NOCK {} · USDC {} · ETH {}",
        pillar("total_native_usd"),
        pillar("total_stable_usd"),
        pillar("total_eth_usd")
    );
    let gas = match (get("wallet_gas_eth"), get("wallet_gas_usd")) {
        (Some(eth), Some(usd)) => format!("{eth} ETH (${usd}), counted in the total; not cash, not deployable"),
        (Some(eth), None) => format!("{eth} ETH (USD value not reported), counted in the total; not cash, not deployable"),
        _ => not_reported.to_string(),
    };
    // Cost basis (issue #98): the worker's own lines, compiled from the same source file.
    let cost_basis = cost_basis_text::cost_basis_lines(data);

    format!(
        "📈 NockNock Sovereign Portfolio State\n\
         =========================================\n\
         • Total Value: {total_usd}\n\
         • Pillars: {pillars}\n\
         • Sovereign NOCK Stack: {total_nock} ({floor})\n\
         • Stable Stack: {total_usdc}\n\
         • Gas held (wallet ETH+WETH): {gas}\n\
         • High-Water Mark: {hwm}\n\
         {cost_basis}\n\
         • Tactical Directive: {advice}"
    )
}

fn format_shot_advice(data: &Value) -> String {
    let global = data.pointer("/derived_metrics/global_status");
    let get = |k: &str| global.and_then(|g| g.get(k)).and_then(|v| v.as_str()).filter(|v| !v.is_empty());
    let action = get("shot_advice_action").unwrap_or("unavailable");
    let details = get("shot_advice_details").unwrap_or("unavailable");
    let claimable_usd = get("total_fees_claimable_usd").map(|v| format!("${v} USD")).unwrap_or_else(|| "unavailable".to_string());
    // Signed as the worker reports it: a negative net is never shown as +$0.00.
    let gas_delta = get("gas_to_fee_delta_usd")
        .map(|v| match v.trim().strip_prefix('-') {
            Some(abs) => format!("-${abs} USD"),
            None => format!("+${v} USD"),
        })
        .unwrap_or_else(|| "unavailable".to_string());

    format!(
        "🎯 Tactical Shot Advice Engine\n\
         ===============================\n\
         • Recommended Action: {action}\n\
         • Decision Matrix Details: {details}\n\
         • Uncollected Claimable Fees: {claimable_usd}\n\
         • Net of two claims' gas: {gas_delta}\n\
         • Harvest rule: {HARVEST_RULE_TEXT}"
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
            if amount.starts_with("not LP-claimable") {
                // An Aero pool whose claimFees() reverts: nothing to claim, no gas to count.
                details.push_str(&format!("  - {symbol}: {amount}\n"));
                continue;
            }
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

/// The worker is unreachable on both its MCP endpoint and its snapshot API. Say so,
/// and report nothing the worker has not read: no wallet, no pool price, no directive.
fn zero_db_fallback_text(tool_name: &str, api_base: &str) -> String {
    let header = match tool_name {
        "get_portfolio_state" => "📈 NockNock Sovereign State (worker unreachable)",
        "get_shot_advice" => "🎯 Tactical Shot Advice Engine (worker unreachable)",
        "simulate_claim_fees" => "⚡ Fee Claim Simulation (worker unreachable)",
        "get_accumulation_plan" => "🪜 Accumulation Plan (worker unreachable)",
        _ => return format!("Unknown tool: {tool_name}"),
    };
    format!(
        "{header}\n\
         =========================================\n\
         • The NockNock worker at {api_base} is unreachable (set NOCKNOCK_API_URL to a running deployment).\n\
         • Nothing is reported that the worker has not read: no balances, prices, positions or directive."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn from_zero_snapshot() -> Value {
        json!({
            "derived_metrics": {
                "global_status": {
                    "total_portfolio_usd": "500.00",
                    "total_nock_amount": "0",
                    "nock_floor_delta": "0",
                    "nock_floor_target": "0",
                    "total_usdc_amount": "500.00",
                    "high_water_mark_usd": "500.00",
                    "shot_advice_action": "BUILD FROM ZERO (No Buy Plan Configured)",
                    "total_fees_claimable_usd": "0.00",
                    "gas_to_fee_delta_usd": "0.00"
                },
                "accumulation_plan": null,
                "lp_rebuild": { "status": "no_positions", "actual_usd": "0.00", "target_usd": "0.00", "gap_usd": "0.00", "suggestions": [], "excluded": [] }
            },
            "contract_fees": []
        })
    }

    #[test]
    fn test_fallback_state_reports_gas_and_pillars() {
        let mut data = from_zero_snapshot();
        let g = &mut data["derived_metrics"]["global_status"];
        g["total_native_usd"] = json!("0.00");
        g["total_stable_usd"] = json!("500.00");
        g["total_eth_usd"] = json!("30.00");
        g["wallet_gas_eth"] = json!("0.010000");
        g["wallet_gas_usd"] = json!("30.00");
        let text = format_portfolio_state(&data);
        assert!(text.contains("Pillars: NOCK $0.00 · USDC $500.00 · ETH $30.00"), "{text}");
        assert!(text.contains("Gas held (wallet ETH+WETH): 0.010000 ETH ($30.00), counted in the total; not cash, not deployable"), "{text}");
        // A pre-2.3.0 snapshot: not reported, never $0.00.
        let old = format_portfolio_state(&from_zero_snapshot());
        assert!(old.contains("Gas held (wallet ETH+WETH): not reported (snapshot predates 2.3.0)"), "{old}");
        assert!(old.contains("ETH not reported (snapshot predates 2.3.0)"), "{old}");
    }

    #[test]
    fn test_fallback_formatters_use_the_snapshot_floor_and_invent_nothing() {
        let data = from_zero_snapshot();
        let state = format_portfolio_state(&data);
        assert!(state.contains("Sovereign NOCK Stack: 0 NOCK (no floor configured)"), "{state}");
        assert!(!state.contains("100k"));

        let mut below = from_zero_snapshot();
        below["derived_metrics"]["global_status"]["nock_floor_target"] = json!("100000");
        below["derived_metrics"]["global_status"]["nock_floor_delta"] = json!("-100000");
        let text = format_portfolio_state(&below);
        assert!(text.contains("100000 below the 100000 NOCK floor"), "{text}");
        assert!(!text.contains("above"));

        let plan = format_accumulation_plan(&data);
        assert!(plan.contains("No accumulation_plan is configured"));
        assert!(plan.contains("No LP positions are tracked"));
        assert!(!plan.contains("rebuilding is a reposition"));

        let empty = format_shot_advice(&json!({}));
        assert!(empty.contains("Recommended Action: unavailable"));
        assert!(!empty.contains("HOLD") && !empty.contains("Accreting") && !empty.contains("0.00"));
        let empty_state = format_portfolio_state(&json!({}));
        assert!(empty_state.contains("Tactical Directive: unavailable"));
        assert!(empty_state.contains("floor unavailable"));
        assert!(!empty_state.contains("HOLD") && !empty_state.contains("0.00"));
    }

    #[test]
    fn test_fallback_shot_advice_states_the_real_rule_and_a_signed_net() {
        let mut data = from_zero_snapshot();
        data["derived_metrics"]["global_status"]["gas_to_fee_delta_usd"] = json!("-0.01");
        let text = format_shot_advice(&data);
        assert!(text.contains("fixed $5.00 threshold"), "{text}");
        assert!(!text.contains("2× live gas"), "{text}");
        assert!(text.contains("-$0.01 USD") && !text.contains("+$-"), "{text}");

        let mut no_target = from_zero_snapshot();
        no_target["derived_metrics"]["lp_rebuild"]["status"] = json!("no_target");
        let plan = format_accumulation_plan(&no_target);
        assert!(plan.contains("No LP target is configured"), "{plan}");
        assert!(!plan.contains("short of target"), "{plan}");
    }

    #[test]
    fn test_fallback_plan_text_never_says_due_now_while_waiting() {
        let data = json!({ "derived_metrics": { "accumulation_plan": {
            "plan_id": "p", "status": "waiting", "tranche_index": 2, "tranches_total": 5,
            "tranche_usdc": "0.00", "nominal_tranche_usdc": "20.00", "next_due_at": "2026-11-16T12:00:00+00:00",
            "spent_usdc": "20.00", "budget_usdc": "100.00", "avg_price_usd": "0.011530"
        }, "lp_rebuild": { "status": "on_target", "gap_usd": "0.00" } } });
        let text = format_accumulation_plan(&data);
        assert!(text.contains("Next tranche: 2 of 5 — $20.00 when due"), "{text}");
        assert!(!text.contains("due now"));
    }

    #[test]
    fn test_fallback_shows_snapshot_time_and_health_not_live_api() {
        let data = from_zero_snapshot();
        for text in [format_portfolio_state(&data), format_shot_advice(&data)] {
            assert!(!text.contains("Live API") && !text.contains("Tier 1 Connected"), "{text}");
        }
        let snap = json!({ "meta": { "generated_at": "2026-11-14T06:00:00+00:00" }, "data": data });
        assert_eq!(
            snapshot_source_line(&snap, Some("ok")),
            "Snapshot generated 2026-11-14T06:00:00+00:00 · worker health: ok"
        );
        assert!(snapshot_source_line(&json!({}), None).contains("worker health: unavailable"));
        // The harvest rule states the engine's precedence.
        assert!(format_shot_advice(&data).contains("only when no DCA tranche is due"));
    }

    #[test]
    fn test_fallback_no_snapshot_reports_the_last_ingestion_error() {
        let body = json!({ "error": "NO_SNAPSHOT", "message": "No snapshot available yet", "detail": "No Alchemy key configured" });
        assert_eq!(
            no_snapshot_text(&body).unwrap(),
            "No snapshot available yet. Last ingestion error: No Alchemy key configured"
        );
        let bare = json!({ "error": "NO_SNAPSHOT", "message": "No snapshot available yet" });
        assert!(no_snapshot_text(&bare).unwrap().contains("first ingestion cycle has not completed"));
        assert!(no_snapshot_text(&json!({ "error": "NOT_FOUND" })).is_none());
    }

    #[test]
    fn test_fallback_claim_text_marks_non_claimable_aero_pool() {
        let data = json!({ "contract_fees": [
            { "fee_token_symbol": "USDC+NOCK (Aerodrome)", "amount": "not LP-claimable (claimFees() reverts); AERO rewards not read (#93)",
              "amount_usd": "0.000000", "claimable": false, "gas_break_even_usd": "0.0037" },
            { "fee_token_symbol": "USDC+NOCK (Uniswap V4 #1)", "amount": "0.1 USDC + 2 NOCK",
              "amount_usd": "0.12", "claimable": true, "gas_break_even_usd": "0.0014" }
        ] });
        let text = format_claim_simulation(&data);
        assert!(text.contains("USDC+NOCK (Aerodrome): not LP-claimable (claimFees() reverts); AERO rewards not read (#93)\n"), "{text}");
        assert!(text.contains("Estimated Gas: ~$0.0014 USD"), "{text}");
    }

    #[test]
    fn test_zero_db_fallback_reports_nothing_unread() {
        for tool in ["get_portfolio_state", "get_shot_advice", "simulate_claim_fees", "get_accumulation_plan"] {
            let text = zero_db_fallback_text(tool, "http://127.0.0.1:8080");
            assert!(text.contains("unreachable") && text.contains("NOCKNOCK_API_URL"), "{text}");
            assert!(text.contains("http://127.0.0.1:8080"));
            assert!(!text.to_lowercase().contains("0xee114cc9"));
            assert!(!text.contains("Spot Price") && !text.contains("ACCRUE"));
        }
        // The headers the MCP playtest pins survive.
        assert!(zero_db_fallback_text("get_portfolio_state", "x").contains("NockNock Sovereign State"));
        assert!(zero_db_fallback_text("get_shot_advice", "x").contains("Tactical Shot Advice Engine"));
        assert!(zero_db_fallback_text("simulate_claim_fees", "x").contains("Fee Claim Simulation"));
        assert_eq!(zero_db_fallback_text("nope", "x"), "Unknown tool: nope");
    }

    #[test]
    fn test_fallback_state_prints_the_cost_basis_lines_the_worker_prints() {
        // The block as the worker emits it (issue #98); figures from tests_cost_basis.rs.
        let mut data = from_zero_snapshot();
        data["derived_metrics"]["cost_basis"] = json!({
            "spot_price_usd": "0.011530",
            "plan_buys": {
                "status": "ok", "reason": null, "fills": 3, "spent_usdc": "60.00", "acquired_nock": "5098.5115",
                "avg_cost_usd": "0.011768", "value_at_spot_usd": "58.79", "unrealized_usd": "-1.21", "unrealized_pct": "-2.02"
            },
            "whole_stack": { "status": "not_configured", "reason": "cost_basis.opening is not set in config." }
        });
        let text = format_portfolio_state(&data);
        // Same source file as the worker's HTTP transport, so the lines are identical.
        assert!(text.contains(&cost_basis_text::cost_basis_lines(&data)), "{text}");
        assert!(
            text.contains("• Plan buys average cost: $0.011768 per NOCK over 3 verified buy(s) ($60.00 for 5098.5115 NOCK). At spot $0.011530 that NOCK is worth $58.79: $1.21 below cost (-2.02%)."),
            "{text}"
        );
        assert!(text.contains("• Whole-stack average cost: not configured"), "{text}");
        // A snapshot from a worker without the field says so; it prints no $0 basis.
        let old = format_portfolio_state(&from_zero_snapshot());
        assert!(old.contains("• Cost basis: not reported by this snapshot"), "{old}");
        assert!(!old.contains("average cost: $0"), "{old}");
    }
}
