//! The WETH/NOCK bet card's MCP lines (issue #181), shared by both transports: the worker's
//! /v1/mcp (mcp_http.rs) and the stdio binary (bin/mcp.rs, by #[path]). Pure: a snapshot's
//! `data` as JSON and the worker's health verdict, nothing else.
use serde_json::Value;

fn s<'a>(v: &'a Value, ptr: &str) -> Option<&'a str> {
    v.pointer(ptr).and_then(|x| x.as_str())
}

/// get_shot_advice's text with the bet section after it (both transports). Pure.
pub fn with_bet_suggestion(advice: &str, data: &Value, health: Option<&str>) -> String {
    let lines = bet_suggestion_lines(data, health);
    if lines.is_empty() {
        advice.to_string()
    } else {
        format!("{advice}\n{}", lines.trim_end())
    }
}

/// The `• WETH/NOCK bet:` section of get_shot_advice. Empty when the snapshot carries no bet
/// card (operator_profile.nock_bets not set, or a snapshot stored before 2.12.0). Pure.
pub fn bet_suggestion_lines(data: &Value, health: Option<&str>) -> String {
    let Some(b) = data.pointer("/derived_metrics/bet_suggestion").filter(|v| v.is_object()) else {
        return String::new();
    };
    let status = s(b, "/status").unwrap_or("unavailable");
    let headline = s(b, "/headline").unwrap_or("Bets: not measured this cycle.");
    let mut out = String::new();
    if status == "ready" && health != Some("ok") {
        out.push_str(&format!(
            "• WETH/NOCK bet: not shown: the snapshot is not fresh (worker health: {}). It shows again on a fresh cycle.\n",
            health.unwrap_or("unknown")
        ));
    } else if status == "ready" {
        out.push_str(&format!("• WETH/NOCK bet: {headline}\n"));
        if let Some(p) = b.get("pool").filter(|p| p.is_object()) {
            out.push_str(&format!(
                "  Pool: {} ({}), tick {}, ${} per NOCK at the pool.\n",
                s(p, "/label").unwrap_or("the bet pool"),
                s(p, "/address").unwrap_or("?"),
                p.get("tick").and_then(|t| t.as_i64()).map(|t| t.to_string()).unwrap_or_else(|| "?".to_string()),
                s(p, "/usd_per_nock").map(trim_price).unwrap_or_else(|| "?".to_string()),
            ));
        }
        if let Some(r) = s(b, "/suggestion/range_text") {
            out.push_str(&format!("  Range: {r}\n"));
        }
        out.push_str(&format!(
            "  Value: ${} (WETH ${} + NOCK ${}).\n",
            s(b, "/suggestion/value_usd").unwrap_or("?"),
            s(b, "/suggestion/weth_usd").unwrap_or("?"),
            s(b, "/suggestion/nock_usd").unwrap_or("?"),
        ));
        if let Some(a) = s(b, "/suggestion/link/amounts_to_enter") {
            out.push_str(&format!("  {a}.\n"));
        }
        if let Some(u) = s(b, "/suggestion/link/url") {
            out.push_str(&format!("  Deposit link (pool and range filled in; the app takes no amount in a link): {u}\n"));
        }
        for o in b.pointer("/suggestion/outcomes").and_then(|o| o.as_array()).into_iter().flatten() {
            if let Some(t) = s(o, "/text") {
                out.push_str(&format!("  - {t}\n"));
            }
        }
    } else {
        let reason = s(b, "/reason").map(|r| format!(" {r}")).unwrap_or_default();
        out.push_str(&format!("• WETH/NOCK bet: {headline} ({status}){reason}\n"));
    }
    // The gates, as measured (each one line).
    for g in b.get("gates").and_then(|g| g.as_array()).into_iter().flatten() {
        if let Some(t) = s(g, "/text") {
            let mark = match s(g, "/status") {
                Some("pass") => "ok",
                Some("fail") => "no",
                _ => "-",
            };
            out.push_str(&format!("  [{mark}] {t}\n"));
        }
    }
    if let Some(t) = s(b, "/floor_crossing/text") {
        out.push_str(&format!("  Floor-crossing: {t}\n"));
    }
    if status == "ready" && health == Some("ok") {
        if let Some(t) = s(b, "/after_deposit") {
            out.push_str(&format!("  After you deposit: {t}\n"));
        }
    }
    // Done things are quiet lines: each tracked bet, and what the gauge holds uncounted.
    for line in b.get("bets").and_then(|v| v.as_array()).into_iter().flatten() {
        if let Some(t) = s(line, "/line") {
            out.push_str(&format!("  · {t}\n"));
        }
    }
    for f in b.get("found_not_counted").and_then(|v| v.as_array()).into_iter().flatten() {
        let dismissed = f.get("dismissed").and_then(|v| v.as_bool()) == Some(true);
        if let Some(t) = s(f, "/line") {
            out.push_str(&format!("  · {}{t}\n", if dismissed { "(set aside) " } else { "" }));
        }
    }
    out
}

/// A USD-per-NOCK price string to 4 significant digits.
fn trim_price(p: &str) -> String {
    match p.trim().parse::<f64>() {
        Ok(v) if v.is_finite() && v > 0.0 => {
            let dp = (3 - v.log10().floor() as i32).max(0) as usize;
            format!("{:.*}", dp, v)
        }
        _ => p.to_string(),
    }
}
