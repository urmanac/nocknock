use crate::*;

/// MCP text when the worker holds no snapshot yet. Pure.
pub(crate) fn no_snapshot_mcp_text(last_error: Option<&str>) -> String {
    match last_error.filter(|e| !e.trim().is_empty()) {
        Some(e) => format!("No snapshot available yet. Last ingestion error: {e}"),
        None => "No snapshot available yet; the first ingestion cycle has not completed.".to_string(),
    }
}

/// Source line for MCP text: when the snapshot was generated and the worker's health
/// verdict (the same one /health reports). Pure. Replaces the old "Live API" label,
/// which was printed for any snapshot however old.
pub(crate) fn snapshot_source_line(generated_at: DateTime<Utc>, now: DateTime<Utc>, health: &str) -> String {
    let age_min = now.signed_duration_since(generated_at).num_minutes().max(0);
    format!(
        "Snapshot generated {} ({} min ago) · worker health: {}",
        generated_at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        age_min,
        health
    )
}

/// Floor clause for MCP text, from the snapshot's own `nock_floor_target` (never a
/// hardcoded 100k). "0" means no floor is configured; a missing field is unavailable.
pub fn floor_clause(global: Option<&serde_json::Value>) -> String {
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

pub fn format_portfolio_state_text(data: &serde_json::Value) -> String {
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

    format!(
        "📈 NockNock Sovereign Portfolio State\n\
         =========================================\n\
         • Total Value: {total_usd}\n\
         • Pillars: {pillars}\n\
         • Sovereign NOCK Stack: {total_nock} ({floor})\n\
         • Stable Stack: {total_usdc}\n\
         • Gas held (wallet ETH+WETH): {gas}\n\
         • High-Water Mark: {hwm}\n\
         • Tactical Directive: {advice}"
    )
}

/// The harvest rule as the directive engine applies it (derive_metrics precedence):
/// a floor DEFENSIVE LOCK, a due DCA tranche and a range exit all outrank it.
pub const HARVEST_RULE_TEXT: &str = "BATCH HARVEST is advised once uncollected LP fees reach the fixed $5.00 threshold, and only when no DCA tranche is due (a floor DEFENSIVE LOCK, a due tranche and a range exit all outrank it).";

pub fn format_shot_advice_text(data: &serde_json::Value) -> String {
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

pub fn format_accumulation_plan_text(data: &serde_json::Value) -> String {
    let s = |v: Option<&serde_json::Value>, k: &str| -> String {
        v.and_then(|p| p.get(k))
            .and_then(|x| x.as_str())
            .map(|x| x.to_string())
            .unwrap_or_else(|| "n/a".to_string())
    };
    let lp = data.pointer("/derived_metrics/lp_rebuild");
    let lp_line = if s(lp, "status") == "no_positions" {
        "• No LP positions are tracked; NockNock suggests deposits only into positions you list under positions:".to_string()
    } else if s(lp, "status") == "no_target" {
        format!(
            "• LP holds ${}. No LP target is configured (operator_profile.target_weights_bps.uniswap_lp), so NockNock suggests no rebuild.",
            s(lp, "actual_usd")
        )
    } else {
        format!(
            "• LP rebuild bias: {} (holds ${} of a ${} target, ${} short). Rebuilding ranges is a reposition: no net cash change.",
            s(lp, "status"), s(lp, "actual_usd"), s(lp, "target_usd"), s(lp, "gap_usd")
        )
    };
    let mut lp_line = lp_line;
    for sg in lp.and_then(|l| l.get("suggestions")).and_then(|v| v.as_array()).into_iter().flatten() {
        let g = |k: &str| sg.get(k).and_then(|x| x.as_str()).unwrap_or("n/a");
        lp_line.push_str(&format!(
            "\n   → add {} {} + {} {} (${}) to {}{}\n     {}",
            g("amount0"), g("token0_symbol"), g("amount1"), g("token1_symbol"), g("add_usd"), g("label"),
            sg.get("limited_by").and_then(|x| x.as_str()).map(|l| format!(" — limited: {l}")).unwrap_or_default(),
            g("url"),
        ));
    }
    for ex in lp.and_then(|l| l.get("excluded")).and_then(|v| v.as_array()).into_iter().flatten() {
        let g = |k: &str| ex.get(k).and_then(|x| x.as_str()).unwrap_or("n/a");
        lp_line.push_str(&format!("\n   ✗ {}: {}", g("label"), g("reason")));
    }
    let plan = data.pointer("/derived_metrics/accumulation_plan").filter(|p| !p.is_null());
    let Some(_) = plan else {
        return format!(
            "🪜 Accumulation Plan\n====================\n• No accumulation_plan is configured.\n{lp_line}"
        );
    };
    let n = |k: &str| plan.and_then(|p| p.get(k)).and_then(|x| x.as_u64()).unwrap_or(0);
    let fills = plan.and_then(|p| p.get("fills")).and_then(|x| x.as_i64()).unwrap_or(0);
    format!(
        "🪜 Accumulation Plan \"{}\" ({})\n====================\n\
         • Status: {}\n\
         • Next tranche: {}\n\
         • Next due at: {}\n\
         • Progress: ${} of ${} spent over {} verified buy(s); ${} remaining\n\
         • Acquired: {} NOCK at average ${}\n\
         • Lump-sum counterfactual (whole budget at first fill price): {} NOCK\n\
         • Constraint: {}\n\
         • Protected cash reserve: ${}\n\
         {lp_line}",
        s(plan, "plan_id"), s(plan, "venue"), s(plan, "status"),
        if s(plan, "status") == "complete" {
            "none (plan complete)".to_string()
        } else if s(plan, "status") != "tranche_due" {
            // Nothing is due while waiting, pending, paused or blocked: say the size,
            // not "$0.00 due now".
            format!(
                "{} of {} — ${} when due",
                n("tranche_index"), n("tranches_total"), s(plan, "nominal_tranche_usdc")
            )
        } else {
            format!(
                "{} of {} — ${} due now (nominal ${})",
                n("tranche_index"), n("tranches_total"), s(plan, "tranche_usdc"), s(plan, "nominal_tranche_usdc")
            )
        },
        s(plan, "next_due_at"),
        s(plan, "spent_usdc"), s(plan, "budget_usdc"), fills, s(plan, "remaining_usdc"),
        s(plan, "acquired_nock"), s(plan, "avg_price_usd"),
        s(plan, "lump_sum_nock"),
        s(plan, "binding_constraint"),
        s(plan, "cash_reserve_usdc"),
    )
}

pub fn format_claim_simulation_text(data: &serde_json::Value) -> String {
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
            if amount.starts_with("not LP-claimable") || amount.starts_with("fees unavailable") {
                // No fee figure to print: an Aero pool whose claimFees() reverts, an Aero CL
                // position staked in its gauge, or one whose fees could not be read. The
                // worker's own reason is shown; nothing to claim, no gas to count.
                details.push_str(&format!("  - {symbol}: {amount}\n"));
                continue;
            }
            match gas_str.parse::<f64>() {
                Ok(g) => {
                    // Per-contract break-evens come from the live gas query; take the max
                    // as the representative claim cost. Never fabricate a constant.
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
