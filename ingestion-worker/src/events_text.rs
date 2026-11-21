//! The calls to action and the ladder maintenance in MCP text (issue #169), shared by both
//! transports: the worker's /v1/mcp (mcp_http.rs) and the stdio binary (bin/mcp.rs, by
//! #[path]). Pure: a snapshot's `data` as JSON and the worker's health verdict.
use serde_json::Value;

fn st<'a>(v: &'a Value, ptr: &str) -> Option<&'a str> {
    v.pointer(ptr).and_then(|x| x.as_str())
}

/// get_shot_advice's text with the events section after it (both transports). Pure.
pub fn with_events(advice: &str, data: &Value, health: Option<&str>) -> String {
    let lines = events_lines(data, health);
    if lines.is_empty() {
        advice.to_string()
    } else {
        format!("{advice}\n{}", lines.trim_end())
    }
}

/// The `• Top of page`, `• Quiet` and `• Ladder maintenance` lines. A snapshot stored before
/// 2.11.0 says the events are not reported, never "nothing needs you". Pure.
pub fn events_lines(data: &Value, health: Option<&str>) -> String {
    let Some(events) = data.pointer("/derived_metrics/events").and_then(|v| v.as_array()) else {
        return "• Top of page: not reported (the snapshot predates 2.11.0).\n".to_string();
    };
    let mut out = String::new();
    let stale = match health {
        Some("ok") => String::new(),
        Some(h) => format!(" As of the last snapshot; the worker's health is {h}, so it may be out of date."),
        None => " As of the last snapshot; the worker's health was not read.".to_string(),
    };
    let top: Vec<&Value> = events.iter().filter(|e| st(e, "/tier") == Some("top")).collect();
    if top.is_empty() {
        out.push_str(&format!("• Top of page: nothing needs you.{stale}\n"));
    } else {
        out.push_str(&format!("• Top of page ({}):{stale}\n", top.len()));
        for e in top {
            out.push_str(&format!(
                "  – {}: {} (since {})\n",
                st(e, "/title").unwrap_or("?"),
                st(e, "/text").unwrap_or(""),
                st(e, "/since").unwrap_or("?")
            ));
            if let Some(w) = st(e, "/would_sign") {
                out.push_str(&format!("    Would sign: {w}\n"));
            }
            if let Some(links) = e.get("links").and_then(|l| l.as_array()) {
                for l in links {
                    if let (Some(label), Some(url)) = (st(l, "/label"), st(l, "/url")) {
                        out.push_str(&format!("    {label}: {url}\n"));
                    }
                }
            }
        }
    }
    // The directive is the advice above: not repeated among the quiet lines.
    let quiet: Vec<&Value> = events.iter().filter(|e| st(e, "/tier") == Some("quiet") && st(e, "/kind") != Some("directive")).collect();
    if !quiet.is_empty() {
        out.push_str(&format!("• Quiet ({}): optional, shown where they are on the dashboard.\n", quiet.len()));
        for e in quiet {
            out.push_str(&format!("  – {}: {}\n", st(e, "/title").unwrap_or("?"), st(e, "/text").unwrap_or("")));
        }
    }
    out.push_str(&maintenance_lines(data));
    out.push_str(&profile_lines(data));
    out
}

/// One line per tracked ladder rung: its rule's status and text. Empty without a ladder.
/// Pure.
pub fn maintenance_lines(data: &Value) -> String {
    let Some(m) = data.pointer("/derived_metrics/ladder_maintenance").filter(|v| v.is_object()) else {
        return String::new();
    };
    let rungs = m.get("rungs").and_then(|r| r.as_array()).cloned().unwrap_or_default();
    if rungs.is_empty() {
        return String::new();
    }
    let mut out = format!("• Ladder maintenance ({} rung{}): {}\n", rungs.len(), if rungs.len() == 1 { "" } else { "s" }, st(m, "/statement").unwrap_or(""));
    for r in &rungs {
        out.push_str(&format!(
            "  – {} ({}): {}\n",
            st(r, "/short_label").unwrap_or("?"),
            st(r, "/status").unwrap_or("?"),
            st(r, "/text").unwrap_or("")
        ));
    }
    out
}

fn price4(v: f64) -> String {
    if !(v > 0.0) || !v.is_finite() {
        return "?".to_string();
    }
    let digits = (3 - v.log10().floor() as i32).max(0) as usize;
    format!("{:.*}", digits, v)
}

fn num(v: &Value, k: &str) -> Option<f64> {
    v.get(k).and_then(|x| x.as_str()).and_then(|s| s.parse::<f64>().ok())
}

/// The ladder shape as a short table (#169): one line per bar, low to high, with its capital
/// and what that capital is; the markers; what is not drawn. Empty without the block. Pure.
pub fn profile_lines(data: &Value) -> String {
    let Some(p) = data.pointer("/derived_metrics/ladder_profile").filter(|v| v.is_object()) else {
        return String::new();
    };
    let mut bars: Vec<&Value> = p.get("bars").and_then(|b| b.as_array()).map(|a| a.iter().collect()).unwrap_or_default();
    bars.sort_by(|a, b| num(a, "low_usd").unwrap_or(0.0).partial_cmp(&num(b, "low_usd").unwrap_or(0.0)).unwrap_or(std::cmp::Ordering::Equal));
    let mut out = "• Ladder shape (each bar on a log price axis; height capital / ln(high/low), so area = capital):\n".to_string();
    for b in bars {
        let capital = match st(b, "/capital_usd") {
            Some(c) => format!("${c} ({})", if st(b, "/capital_basis") == Some("usdc_at_top") { "all USDC at its top" } else { "value now" }),
            None => "capital unknown".to_string(),
        };
        out.push_str(&format!(
            "  – {} ({}): ${}–${}, {capital}, density {}\n",
            st(b, "/name").unwrap_or("?"),
            st(b, "/label").unwrap_or(""),
            price4(num(b, "low_usd").unwrap_or(0.0)),
            price4(num(b, "high_usd").unwrap_or(0.0)),
            st(b, "/density").unwrap_or("unknown")
        ));
    }
    let mut marks = Vec::new();
    if let Some(s) = num(p, "spot_usd") {
        marks.push(format!("spot ${}", price4(s)));
    }
    if let Some(b) = num(p, "ladder_bottom_usd") {
        marks.push(format!("ladder bottom ${}", price4(b)));
    }
    if let Some(r) = st(p, "/reserve_note") {
        marks.push(r.to_string());
    }
    if !marks.is_empty() {
        out.push_str(&format!("  Markers: {}.\n", marks.join("; ")));
    }
    for fr in p.get("full_range").and_then(|v| v.as_array()).into_iter().flatten() {
        out.push_str(&format!("  Not drawn: {}.\n", st(fr, "/text").unwrap_or("")));
    }
    out
}
