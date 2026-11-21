//! Tracked ladders (#167) in the MCP text.
//!
//! One source file for both MCP transports: the worker's HTTP endpoint (`mod
//! ladder_track_text` in main.rs, called from mcp_text.rs) and the stdio binary, which
//! cannot import from main.rs and includes this file by path (src/bin/mcp.rs). It depends
//! on nothing but serde_json.
//!
//! A ladder is measured, never steered. The text states what each ladder holds, where its
//! range is against spot, and what it bought since its deposit only when the deposit
//! receipt was verified; otherwise the reason. It never says to re-centre, withdraw, add
//! to or close a ladder. A snapshot with no ladder (every snapshot from before #167 and
//! every one without a `role: ladder` position) gets no line at all, so its text is what
//! it was.
use serde_json::Value;

fn s<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(|x| x.as_str()).filter(|x| !x.trim().is_empty())
}

fn ladder_entries(data: &Value) -> &[Value] {
    data.get("ladders").and_then(|v| v.as_array()).map(Vec::as_slice).unwrap_or(&[])
}

/// The words for a ladder's state, as the dashboard prints them.
pub fn ladder_state_words(state: &str) -> &'static str {
    match state {
        "holds_only_usdc" => "price above the range: all USDC",
        "in_range" => "in range: converts both ways",
        "holds_only_nock" => "price below the range: all NOCK",
        "empty" => "empty: no liquidity",
        "pool_stale" => "stale pool: frozen price",
        _ => "state not reported",
    }
}

/// "since deposit (tx 0xafe4…, block N): net X NOCK bought for net $Y spent, average $Z",
/// or the status and its reason.
pub fn since_deposit_text(sd: Option<&Value>) -> String {
    let Some(sd) = sd else { return "since deposit: not reported".to_string() };
    let status = s(sd, "status").unwrap_or("not reported");
    if status == "verified" {
        let tx = s(sd, "deposit_tx").map(|t| format!("{}…", &t[..t.len().min(10)])).unwrap_or_default();
        let block = sd.get("deposit_block").and_then(|b| b.as_i64()).map(|b| format!(", block {b}")).unwrap_or_default();
        let avg = match s(sd, "avg_buy_price_usd") {
            Some(a) => format!(", average ${a}"),
            None => "; no average price (it has not net bought NOCK with USDC)".to_string(),
        };
        format!(
            "since deposit (tx {tx}{block}): net {} NOCK bought for net ${} USDC spent{avg}",
            s(sd, "net_nock_bought").unwrap_or("?"),
            s(sd, "net_usdc_spent").unwrap_or("?"),
        )
    } else {
        format!("since deposit: {status}: {}", s(sd, "reason").unwrap_or("no reason reported"))
    }
}

/// One ladder as two to four lines of fact.
pub fn ladder_statement(l: &Value) -> String {
    let label = s(l, "label").or_else(|| s(l, "position_id")).unwrap_or("ladder");
    let rung = match s(l, "rung") {
        Some(r) => format!("rung {r}"),
        None => "rung unset".to_string(),
    };
    let pct = match (s(l, "low_pct_of_spot"), s(l, "high_pct_of_spot")) {
        (Some(a), Some(b)) => format!(" ({a}% / {b}% of spot)"),
        _ => " (no spot to compare)".to_string(),
    };
    let vs = l.get("vs_top");
    let g = |k: &str| vs.and_then(|v| s(v, k)).unwrap_or("?");
    let curve_avg = vs
        .and_then(|v| s(v, "curve_avg_price_usd"))
        .map(|a| format!(", curve average ${a}"))
        .unwrap_or_default();
    format!(
        "   – {label} ({rung}): ${} – ${}{pct}; {}; holds ${} USDC + {} NOCK\n\
         \u{20}    against its all-USDC top (${} USDC): {} NOCK held, ${} converted ({}%){curve_avg}; all NOCK at the bottom: {} NOCK\n\
         \u{20}    {}; uncollected fees ${}\n",
        s(l, "low_price_usd").unwrap_or("?"),
        s(l, "high_price_usd").unwrap_or("?"),
        ladder_state_words(s(l, "state").unwrap_or("")),
        s(l, "usdc_now").unwrap_or("?"),
        s(l, "nock_now").unwrap_or("?"),
        g("usdc_at_top"),
        g("nock_held_vs_top"),
        g("usdc_converted_vs_top"),
        vs.and_then(|v| s(v, "converted_pct")).unwrap_or("?"),
        g("nock_at_bottom"),
        since_deposit_text(l.get("since_deposit")),
        s(l, "fees_usd").unwrap_or("?"),
    )
}

/// The ladders block of get_portfolio_state; "" when the snapshot lists none.
pub fn ladders_lines(data: &Value) -> String {
    let entries = ladder_entries(data);
    if entries.is_empty() {
        return String::new();
    }
    let mut out = "• Ladders (measured, not NOCK LP; counted in the totals, the reserve and the floor-crossing price):\n".to_string();
    for l in entries {
        out.push_str(&ladder_statement(l));
    }
    out
}

/// The note after the rebuild lines; "" when the snapshot lists no ladder.
pub fn ladder_rebuild_note(data: &Value) -> String {
    let n = ladder_entries(data).len();
    if n == 0 {
        return String::new();
    }
    format!("\n• Ladders are not rebuild templates and are not in the NOCK LP target ({n} tracked, measured only).")
}

/// The ladder part of the whole-stack cost-basis line; "" when the block has none.
pub fn ladder_basis_clause(whole_stack: Option<&Value>) -> String {
    let Some(lb) = whole_stack.and_then(|w| w.get("ladder_buys_since")).filter(|v| v.is_object()) else {
        return String::new();
    };
    let counted = lb.get("ladders_counted").and_then(|x| x.as_i64()).unwrap_or(0);
    let left = lb.get("not_counted").and_then(|x| x.as_array()).map(|a| a.len()).unwrap_or(0);
    let total = counted + left as i64;
    format!(
        " Plus the net buys of {counted} of {total} tracked ladder{} since a verified deposit: {} NOCK for ${}.",
        if total == 1 { "" } else { "s" },
        s(lb, "nock").unwrap_or("?"),
        s(lb, "usdc").unwrap_or("?"),
    )
}
