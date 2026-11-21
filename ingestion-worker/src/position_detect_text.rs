//! Positions found on-chain in the MCP text (v0.9.0; #145 for Uniswap v4).
//!
//! One source file for both MCP transports: the worker's HTTP endpoint (`mod
//! position_detect_text` in main.rs) and the stdio binary, which cannot import from
//! main.rs and includes this file by path (src/bin/mcp.rs). It depends on nothing but
//! serde_json, and reads only `derived_metrics.position_detection` of a snapshot, so the
//! two transports print the same lines.
//!
//! It states whether new positions were looked for and how far back, which positions are
//! tracked from the chain, and every position that is found and NOT counted, with its
//! value: a total that leaves a position out says so here. It never says to track,
//! dismiss, deposit or withdraw. A snapshot stored before the field existed gets no line.
use serde_json::Value;

fn s<'a>(v: &'a Value, ptr: &str) -> Option<&'a str> {
    v.pointer(ptr).and_then(|x| x.as_str()).filter(|x| !x.trim().is_empty())
}

fn n(v: &Value, ptr: &str) -> Option<u64> {
    v.pointer(ptr).and_then(|x| x.as_u64())
}

fn short(h: &str) -> String {
    if h.chars().count() > 12 { format!("{}…", h.chars().take(10).collect::<String>()) } else { h.to_string() }
}

/// A one-tap choice in words, as the dashboard labels its button.
pub fn choice_words(choice: &str) -> String {
    match choice {
        "lp" => "TRACK AS LP".to_string(),
        "dismiss" => "NOT NOW".to_string(),
        "untrack" => "UNTRACK".to_string(),
        c => match c.strip_prefix("ladder:") {
            Some(rung) => format!("TRACK AS {} RUNG", rung.to_uppercase()),
            None => c.to_uppercase(),
        },
    }
}

/// One found position as a statement of fact: what it is, what it holds and why it was
/// not tracked by itself.
pub fn found_statement(p: &Value) -> String {
    let id = n(p, "/nft_id").map(|i| format!("#{i}")).unwrap_or_else(|| "#?".to_string());
    let ticks = match (p.get("tick_lower").and_then(|v| v.as_i64()), p.get("tick_upper").and_then(|v| v.as_i64())) {
        (Some(lo), Some(hi)) => format!(", ticks {lo}..{hi}"),
        _ => String::new(),
    };
    let bounds = match (s(p, "/bounds_usd/low"), s(p, "/bounds_usd/high")) {
        (Some(lo), Some(hi)) => format!(" (${lo} to ${hi} per NOCK)"),
        _ => String::new(),
    };
    let holds = match (s(p, "/holds/token0/amount"), s(p, "/holds/token0/symbol"), s(p, "/holds/token1/amount"), s(p, "/holds/token1/symbol")) {
        (Some(a0), Some(s0), Some(a1), Some(s1)) => format!(" It holds {a0} {s0} + {a1} {s1}"),
        _ => String::new(),
    };
    let sentence = |note: &str| {
        let mut c = note.chars();
        match c.next() {
            Some(first) => format!(" {}{}.", first.to_uppercase(), c.as_str()),
            None => String::new(),
        }
    };
    let value = match (s(p, "/value_usd"), s(p, "/value_note")) {
        (Some(v), _) if !holds.is_empty() => format!(", worth ${v}."),
        (Some(v), _) => format!(" It is worth ${v}."),
        (None, Some(note)) if !holds.is_empty() => format!(".{}", sentence(note)),
        (None, Some(note)) => sentence(note),
        (None, None) if !holds.is_empty() => ".".to_string(),
        (None, None) => String::new(),
    };
    let why: Vec<&str> = p.get("why_not_automatic").and_then(|w| w.as_array()).into_iter().flatten().filter_map(|x| x.as_str()).collect();
    let why = if why.is_empty() { String::new() } else { format!(" Not tracked by itself: {}.", why.join("; ")) };
    format!("{} {id}{ticks}{bounds}.{holds}{value}{why}", s(p, "/pool_label").unwrap_or("position"))
}

fn list_total(list: &Value) -> String {
    let count = n(list, "/count").unwrap_or(0);
    let noun = if count == 1 { "position" } else { "positions" };
    match (s(list, "/value_usd"), s(list, "/value_note")) {
        (Some(v), _) => format!("{count} {noun} worth ${v}"),
        (None, Some(note)) => format!("{count} {noun} ({note})"),
        (None, None) => format!("{count} {noun}"),
    }
}

fn ignored_words(d: &Value) -> String {
    let labels = [
        ("in_config", "in config.yaml"),
        ("not_whitelisted", "not on the token whitelist"),
        ("empty", "empty"),
        ("not_owned", "no longer in the wallet"),
        ("hooks", "in a hooked pool"),
        ("unsupported", "with a token NockNock cannot read in a position yet"),
        ("gone", "burned"),
    ];
    let parts: Vec<String> = labels.iter().filter_map(|(k, words)| n(d, &format!("/ignored/{k}")).filter(|c| *c > 0).map(|c| format!("{c} {words}"))).collect();
    if parts.is_empty() { String::new() } else { format!(" Not listed: {}.", parts.join(", ")) }
}

/// The `• Positions found on-chain` block: a header line, then one line per position
/// tracked from the chain, per position found and not counted, and per position set
/// aside. Each line ends in a newline. Empty for a snapshot without the field.
pub fn position_detection_lines(data: &Value) -> String {
    let Some(d) = data.pointer("/derived_metrics/position_detection").filter(|v| v.is_object()) else {
        return String::new();
    };
    let status = s(d, "/status").unwrap_or("unavailable");
    let offers = d.pointer("/found_not_counted/positions").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let dismissed = d.pointer("/dismissed/positions").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let tracked = d.get("tracked").and_then(|v| v.as_array()).cloned().unwrap_or_default();
    let coverage = s(d, "/scan/text").unwrap_or("");
    let head = match status {
        "ok" => format!("checked. {coverage}"),
        // The reason names the gap; the coverage sentence would only repeat it.
        "partial" => format!("checked in part: {}.", s(d, "/reason").unwrap_or("something is missing")),
        "not_run" => format!("not looked for: {}.", s(d, "/reason").unwrap_or("no reason reported")),
        _ => s(d, "/reason").unwrap_or("New positions could not be checked this cycle.").to_string(),
    };
    let nothing = if offers.is_empty() && dismissed.is_empty() && tracked.is_empty() && status == "ok" { " None is found that is not already counted." } else { "" };
    let mut out = format!("• Positions found on-chain ({status}): {}{nothing}{}\n", head.trim_end(), ignored_words(d));
    for t in &tracked {
        let what = match (s(t, "/role"), s(t, "/rung")) {
            (Some("ladder"), Some(rung)) => format!("as a ladder ({rung} rung)"),
            (Some("ladder"), None) => "as a ladder (rung not set)".to_string(),
            _ => "as LP (never a rebuild target)".to_string(),
        };
        let how = match s(t, "/source") {
            Some("detected") => "tracked by itself: it matched a suggested rung",
            _ => "tracked by your tap",
        };
        let mint = s(t, "/deposit_tx").map(|tx| format!(" (mint {})", short(tx))).unwrap_or_default();
        let state = match s(t, "/state") {
            Some("counted") => "Counted in the totals.".to_string(),
            Some(other) => format!("Not counted ({}): {}", other.replace('_', " "), s(t, "/note").unwrap_or("no reason reported.")),
            None => "State not reported.".to_string(),
        };
        out.push_str(&format!(
            "   ✓ Tracked from the chain: NFT #{} {what}, {how}{mint}. {state}\n",
            n(t, "/nft_id").map(|i| i.to_string()).unwrap_or_else(|| "?".to_string())
        ));
    }
    if !offers.is_empty() {
        out.push_str(&format!("   ! Found on-chain, NOT counted in any total: {}.\n", list_total(&d["found_not_counted"])));
        for p in &offers {
            let choices: Vec<String> = p.get("choices").and_then(|c| c.as_array()).into_iter().flatten().filter_map(|x| x.as_str()).map(choice_words).collect();
            let taps = if choices.is_empty() { String::new() } else { format!(" One tap in the dashboard: {}.", choices.join(", ")) };
            out.push_str(&format!("     - {}{taps}\n", found_statement(p)));
        }
    }
    if !dismissed.is_empty() {
        out.push_str(&format!("   - Set aside by you, not counted: {}.\n", list_total(&d["dismissed"])));
        for p in &dismissed {
            out.push_str(&format!("     - {} RESTORE puts it back on offer.\n", found_statement(p)));
        }
    }
    out
}

/// The same block for a text whose last line does not end in a newline: it starts on a
/// new line and ends without one. Empty for a snapshot without the field.
pub fn position_detection_trailer(data: &Value) -> String {
    let block = position_detection_lines(data);
    if block.is_empty() {
        return block;
    }
    format!("\n{}", block.trim_end_matches('\n'))
}

/// get_shot_advice's text with the block after it, only when it has something to say
/// that bears on the totals: a position found and not counted, or a check that did not
/// run or is incomplete. With everything read and nothing found the advice is unchanged.
pub fn with_position_notice(advice: &str, data: &Value) -> String {
    let Some(d) = data.pointer("/derived_metrics/position_detection").filter(|v| v.is_object()) else {
        return advice.to_string();
    };
    let found = n(d, "/found_not_counted/count").unwrap_or(0) > 0;
    if s(d, "/status") == Some("ok") && !found {
        return advice.to_string();
    }
    format!("{advice}{}", position_detection_trailer(data))
}
