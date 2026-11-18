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
    // AERO is the fourth pillar (issue #93): wallet AERO, so the four add up to the total.
    let pillars = format!(
        "NOCK {} · USDC {} · ETH {} · AERO {}",
        pillar("total_native_usd"),
        pillar("total_stable_usd"),
        pillar("total_eth_usd"),
        aero_pillar_text(data)
    );
    let aero = aero_lines(data);
    // Collectible earnings (issue #142): LP fees and gauge rewards, labelled, and their sum.
    let collectible = collectible_earnings_line(data);
    let gas = match (get("wallet_gas_eth"), get("wallet_gas_usd")) {
        (Some(eth), Some(usd)) => format!("{eth} ETH (${usd}), counted in the total; not cash, not deployable"),
        (Some(eth), None) => format!("{eth} ETH (USD value not reported), counted in the total; not cash, not deployable"),
        _ => not_reported.to_string(),
    };
    // Cost basis (issue #98): the same lines in both MCP transports (cost_basis_text.rs).
    let cost_basis = cost_basis_lines(data);
    // Closed positions (issue #144): stated, never counted. Empty when there are none.
    let closed = closed_positions_lines(data);

    format!(
        "📈 NockNock Sovereign Portfolio State\n\
         =========================================\n\
         • Total Value: {total_usd}\n\
         • Pillars: {pillars}\n\
         • Sovereign NOCK Stack: {total_nock} ({floor})\n\
         • Stable Stack: {total_usdc}\n\
         • Gas held (wallet ETH+WETH): {gas}\n\
         {aero}\n\
         {collectible}\
         {closed}\
         • High-Water Mark: {hwm}\n\
         {cost_basis}\n\
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
    // Gauge rewards and the total (issue #142). "Uncollected Claimable Fees" stays LP fees.
    let collectible = collectible_advice_lines(data);
    // Closed positions (issue #144): stated beside the directive, which they do not
    // change. Empty when there are none.
    let closed = closed_positions_lines(data);

    format!(
        "🎯 Tactical Shot Advice Engine\n\
         ===============================\n\
         • Recommended Action: {action}\n\
         • Decision Matrix Details: {details}\n\
         • Uncollected Claimable Fees: {claimable_usd}\n\
         • Net of two claims' gas: {gas_delta}\n\
         {collectible}\
         {closed}\
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
    // The rebuild judges NOCK-pair LP only (#151); gas-side ETH/USDC ranges are measured.
    let status = s(lp, "status");
    let lp_line = if status == "no_positions" {
        "• No NOCK LP positions are tracked; NockNock suggests deposits only into NOCK positions you list under positions:".to_string()
    } else if status == "no_target" {
        format!(
            "• NOCK LP holds ${}. No LP target is configured (operator_profile.target_weights_bps.uniswap_lp), so NockNock suggests no rebuild.",
            s(lp, "actual_usd")
        )
    } else if status == "under_target" {
        format!(
            "• NOCK LP rebuild bias: under_target (holds ${} of a ${} target, ${} short). Rebuilding ranges is a reposition: no net cash change.",
            s(lp, "actual_usd"), s(lp, "target_usd"), s(lp, "gap_usd")
        )
    } else if status == "over_target" {
        // gap_usd is clamped at zero above target; the overage is stated, never "$0.00 short".
        let over = match (s(lp, "actual_usd").parse::<f64>(), s(lp, "target_usd").parse::<f64>()) {
            (Ok(a), Ok(t)) => format!(", over target by ${:.2}", (a - t).max(0.0)),
            _ => String::new(),
        };
        format!(
            "• NOCK LP rebuild bias: over_target (holds ${} of a ${} target{}). NockNock never suggests unwinding a range.",
            s(lp, "actual_usd"), s(lp, "target_usd"), over
        )
    } else {
        format!(
            "• NOCK LP rebuild bias: {} (holds ${} of a ${} target, within tolerance).",
            status, s(lp, "actual_usd"), s(lp, "target_usd")
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
    // Closed positions (issue #144): after the rebuild lines, which list every tracked
    // position and no closed one. Empty when there are none.
    let closed = closed_positions_trailer(data);
    let plan = data.pointer("/derived_metrics/accumulation_plan").filter(|p| !p.is_null());
    let Some(_) = plan else {
        return format!(
            "🪜 Accumulation Plan\n====================\n• No accumulation_plan is configured.\n{lp_line}{closed}"
        );
    };
    let n = |k: &str| plan.and_then(|p| p.get(k)).and_then(|x| x.as_u64()).unwrap_or(0);
    let fills = plan.and_then(|p| p.get("fills")).and_then(|x| x.as_i64()).unwrap_or(0);
    // Detected swaps and whether the check ran (issue #118); empty for older snapshots.
    let detection = plan.map(fill_detection_lines).unwrap_or_default();
    // Route quotes for a due tranche (issue #115); empty unless the snapshot has them.
    let quotes = plan.map(route_quote_lines).unwrap_or_default();
    // The operator's venue label, when set; none is chosen for them (issue #132).
    let venue_label = plan
        .and_then(|p| p.get("venue"))
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
        .map(|v| format!(" (your venue label: {v})"))
        .unwrap_or_default();
    // A cycle that could not read the ledger knows no progress (issue #130).
    if plan.and_then(|p| p.get("progress_unknown")).and_then(|x| x.as_bool()) == Some(true) {
        return format!(
            "🪜 Accumulation Plan \"{}\"{}\n====================\n\
             • Status: {}\n\
             • Next tranche: unknown\n\
             • Progress: unknown of ${} (the ledger was not read this cycle)\n\
             • Constraint: {}\n\
             {detection}{lp_line}{closed}",
            s(plan, "plan_id"), venue_label, s(plan, "status"), s(plan, "budget_usdc"), s(plan, "binding_constraint"),
        );
    }
    format!(
        "🪜 Accumulation Plan \"{}\"{}\n====================\n\
         • Status: {}\n\
         • Next tranche: {}\n\
         • Next due at: {}\n\
         • Progress: ${} of ${} spent over {} verified buy(s); ${} remaining\n\
         • Acquired: {} NOCK at average ${}\n\
         • Lump-sum counterfactual (whole budget at first fill price): {} NOCK\n\
         • Constraint: {}\n\
         • Protected cash reserve: ${}\n\
         {detection}{quotes}{lp_line}{closed}",
        s(plan, "plan_id"), venue_label, s(plan, "status"),
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

/// `simulate_claim_fees` from a snapshot alone: no live gas estimate for a reward.
/// The text is built by reward_text.rs, the one source both transports compile. The
/// HTTP transport calls that function with its live gas statements (mcp_http.rs); this
/// form is what the tests and the golden files pin.
#[cfg(test)]
pub fn format_claim_simulation_text(data: &serde_json::Value) -> String {
    claim_simulation_text(data, &[])
}
