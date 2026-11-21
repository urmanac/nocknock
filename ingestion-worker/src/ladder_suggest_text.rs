//! The ladder suggestion's MCP lines (issue #168), shared by both transports: the worker's
//! /v1/mcp (mcp_http.rs) and the stdio binary (bin/mcp.rs, by #[path]). Pure: a snapshot's
//! `data` as JSON and the worker's health verdict, nothing else.
use serde_json::Value;

fn s<'a>(v: &'a Value, ptr: &str) -> Option<&'a str> {
    v.pointer(ptr).and_then(|x| x.as_str())
}

/// Whether the card may show: the worker's health verdict is "ok" (the snapshot is within
/// its sync window and the last cycle did not fail). Anything else, or no verdict, is not
/// fresh. Pure.
pub fn ladder_fresh(health: Option<&str>) -> bool {
    health == Some("ok")
}

/// get_shot_advice's text with the ladder section after it (both transports). Pure.
pub fn with_ladder_suggestion(advice: &str, data: &Value, health: Option<&str>) -> String {
    let lines = ladder_suggestion_lines(data, health);
    if lines.is_empty() {
        advice.to_string()
    } else {
        format!("{advice}\n{}", lines.trim_end())
    }
}

/// The `• Ladder suggestion:` section of get_shot_advice. Empty for a snapshot stored before
/// the field existed (2.9.0). Pure.
pub fn ladder_suggestion_lines(data: &Value, health: Option<&str>) -> String {
    let Some(l) = data.pointer("/derived_metrics/ladder_suggestion").filter(|v| v.is_object()) else {
        return String::new();
    };
    let status = s(l, "/status").unwrap_or("unavailable");
    let reason = s(l, "/reason").unwrap_or("");
    if status != "ready" {
        let mut out = format!("• Ladder suggestion: {} ({status}) {reason}\n", s(l, "/headline").unwrap_or("Ladder: not shown."));
        if status == "needs_setting" {
            if let Some(rows) = l.pointer("/candidates/rows").and_then(|r| r.as_array()).filter(|r| !r.is_empty()) {
                out.push_str(&format!(
                    "  Candidate bottoms (a measurement, none picked; grid to {}% in {}% steps):\n",
                    s(l, "/candidates/to_pct").unwrap_or("?"),
                    s(l, "/candidates/step_pct").unwrap_or("?")
                ));
                for r in rows.iter().take(8) {
                    let ceiling = match s(r, "/usdc_ceiling") {
                        Some(c) => format!("reserve allows ${c}, {} NOCK at the bottom", s(r, "/nock_at_bottom").unwrap_or("?")),
                        None => "no reserve set, no amount".to_string(),
                    };
                    out.push_str(&format!(
                        "  - ${} ({}%, tick {}): {ceiling}; {} NOCK per $100; average ${}\n",
                        s(r, "/price_usd").unwrap_or("?"),
                        s(r, "/pct_of_spot").unwrap_or("?"),
                        r.get("tick").and_then(|t| t.as_i64()).map(|t| t.to_string()).unwrap_or_else(|| "?".to_string()),
                        s(r, "/nock_per_100_usdc").unwrap_or("?"),
                        s(r, "/avg_buy_price_usd").unwrap_or("?"),
                    ));
                }
            }
        }
        return out;
    }
    if !ladder_fresh(health) {
        return format!(
            "• Ladder suggestion: not shown: the snapshot is not fresh (worker health: {}). It shows again on a fresh cycle.\n",
            health.unwrap_or("unavailable")
        );
    }
    let mut out = format!(
        "• Ladder suggestion (a measurement, not advice): {} {}\n",
        s(l, "/headline").unwrap_or(""),
        s(l, "/sentence").unwrap_or("")
    );
    for o in l.get("outcomes").and_then(|o| o.as_array()).into_iter().flatten() {
        if let Some(t) = s(o, "/text") {
            out.push_str(&format!("  - {t}\n"));
        }
    }
    out.push_str(&format!(
        "  Reserve ${} holds at the bottom; NOCK budget: {}.\n",
        s(l, "/numbers/cash_reserve_usdc").unwrap_or("?"),
        s(l, "/gates/nock_budget/status").unwrap_or("unknown"),
    ));
    out.push_str(&rung_lines(l));
    out.push_str(&after_deposit_line(l.get("after_deposit")));
    out.push_str(&format!("  {}\n", s(l, "/statement").unwrap_or("")));
    out
}

/// One numbered block per rung: "Deposit 1 of 2 …", its range as ticks, USD and NOCK per
/// USDC, the amounts and the link. Pure.
pub fn rung_lines(l: &Value) -> String {
    let rungs = l.get("rungs").and_then(|r| r.as_array()).cloned().unwrap_or_default();
    let total = rungs.len();
    let mut out = String::new();
    for (i, r) in rungs.iter().enumerate() {
        let name = s(r, "/rung").map(|n| format!(" ({n} rung)")).unwrap_or_default();
        out.push_str(&format!(
            "  Deposit {} of {total}{name}: {} USDC + {} NOCK into {}, ticks {} to {} (${} to ${}; {} to {} NOCK per USDC).\n",
            i + 1,
            s(r, "/usdc/amount").unwrap_or("?"),
            s(r, "/nock/amount").unwrap_or("?"),
            s(r, "/pool/label").unwrap_or("?"),
            r.get("tick_lower").and_then(|v| v.as_i64()).map(|v| v.to_string()).unwrap_or_else(|| "?".into()),
            r.get("tick_upper").and_then(|v| v.as_i64()).map(|v| v.to_string()).unwrap_or_else(|| "?".into()),
            s(r, "/bounds_usd/low").unwrap_or("?"),
            s(r, "/bounds_usd/high").unwrap_or("?"),
            s(r, "/bounds_nock_per_usdc/low").unwrap_or("?"),
            s(r, "/bounds_nock_per_usdc/high").unwrap_or("?"),
        ));
        match s(r, "/deep_link/url") {
            Some(url) => out.push_str(&format!("    Deposit link (no amount prefilled): {url}\n")),
            None => out.push_str(&format!("    Deposit link: none ({})\n", s(r, "/deep_link/reason").unwrap_or("not built"))),
        }
    }
    out
}

/// The `after_deposit` field, whatever its kind. Today only `manual_config`, the fallback
/// until new position NFTs are detected on-chain. Pure.
pub fn after_deposit_line(a: Option<&Value>) -> String {
    match a.and_then(|a| s(a, "/kind")) {
        Some("manual_config") => format!("  After you deposit (current fallback): {}\n", a.and_then(|a| s(a, "/text")).unwrap_or("")),
        Some(other) => format!("  After you deposit: {}\n", a.and_then(|a| s(a, "/text")).unwrap_or(other)),
        None => String::new(),
    }
}
