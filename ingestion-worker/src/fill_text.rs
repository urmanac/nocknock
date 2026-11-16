//! Plain-text rendering of the accumulation plan's fill detection (issue #118).
//!
//! This file is compiled into both binaries: the worker (`mod fill_text`) and the stdio
//! MCP server (`#[path = "../fill_text.rs"] mod fill_text`), which cannot import from
//! main.rs. One function, one text, so the two MCP transports cannot drift. It depends
//! on serde_json only and reads the snapshot's `accumulation_plan` object.

use serde_json::Value;

/// Lines for `accumulation_plan.fill_detection`, each ending in a newline. Empty when
/// the snapshot carries no detection result (a snapshot from before v0.6.0): nothing is
/// claimed about a check that was never reported.
///
/// "Could not check" (`unavailable`) and "checked, nothing found" (`ok` with no offers)
/// are different sentences. The text lists offers; it offers no way to confirm one,
/// because MCP is read-only.
pub fn fill_detection_lines(plan: &Value) -> String {
    let Some(fd) = plan.get("fill_detection").filter(|v| !v.is_null()) else {
        return String::new();
    };
    let text = |v: &Value, k: &str| -> Option<String> {
        v.get(k).and_then(|x| x.as_str()).filter(|x| !x.is_empty()).map(|x| x.to_string())
    };
    let list = |k: &str| -> Vec<Value> { fd.get(k).and_then(|v| v.as_array()).cloned().unwrap_or_default() };
    let status = text(fd, "status").unwrap_or_else(|| "unknown".to_string());
    let detail = text(fd, "detail").unwrap_or_else(|| "no reason given".to_string());
    let last = text(fd, "last_checked_at");
    let back = text(fd, "checked_back_to");
    let offers = list("offers");
    let dismissed = list("dismissed");

    let checked = last.clone().map(|t| format!("checked {t}")).unwrap_or_else(|| "checked".to_string());
    let back_to = back.map(|b| format!(" back to {b}")).unwrap_or_default();
    let head = match status.as_str() {
        "ok" if offers.is_empty() => format!("{checked}{back_to}; no uncounted swap found"),
        "ok" => format!("{checked}{back_to}"),
        "partial" => format!("{checked}{back_to}, incompletely: {detail}"),
        "unavailable" => format!(
            "could not check this cycle ({detail}); last successful check: {}. This is not \"none found\": record a buy by its transaction hash meanwhile",
            last.unwrap_or_else(|| "none on record".to_string())
        ),
        "not_run" => format!("not run ({detail})"),
        other => format!("status {other} ({detail})"),
    };
    let mut out = format!("• Swap detection: {head}\n");

    let line = |o: &Value| -> String {
        let g = |k: &str| text(o, k).unwrap_or_else(|| "n/a".to_string());
        format!(
            "{} · {} · ${} USDC → {} NOCK at ${}",
            g("tx_hash"), g("confirmed_at"), g("usdc_spent"), g("nock_received"), g("effective_price_usd")
        )
    };
    if !offers.is_empty() {
        out.push_str(&format!(
            "• {} detected swap(s) not counted yet. Confirm each in the dashboard or mark it \"not part of the plan\" (MCP is read-only):\n",
            offers.len()
        ));
    }
    for o in &offers {
        let cost = match (o.get("execution_cost_bps").and_then(|b| b.as_i64()), text(o, "reference_spot_usd")) {
            (Some(bps), Some(spot)) => format!("{bps:+} bps against spot ${spot}"),
            _ => "cost over spot unavailable".to_string(),
        };
        let timing = match text(o, "timing").as_deref() {
            Some("early") => "early (before the open tranche came due)",
            Some("on_schedule") => "on schedule",
            _ => "timing n/a",
        };
        out.push_str(&format!("   ? {} · {} · {}\n", line(o), cost, timing));
    }
    for d in &dismissed {
        out.push_str(&format!("   ✗ not part of the plan (dismissed, restorable): {}\n", line(d)));
    }
    out
}
