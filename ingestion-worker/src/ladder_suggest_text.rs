//! The ladder suggestion's MCP lines (issue #168), shared by both transports: the worker's
//! /v1/mcp (mcp_http.rs) and the stdio binary (bin/mcp.rs, by #[path]). Pure: a snapshot's
//! `data` as JSON and the worker's health verdict, nothing else.
//!
//! With a configured shape (operator_profile.ladder_rungs) the suggestion carries
//! `shape.rungs[]`: every rung of the shape, in order. The rungs left to deposit print as
//! numbered deposit blocks (the same numbers in both transports and on the card); a rung
//! that is deployed, passed or over prints its one stated line.
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
        // Each excluded pool's reason, so "no pool can hold one" says why.
        if status == "no_target_pool" {
            for e in l.get("excluded").and_then(|e| e.as_array()).into_iter().flatten() {
                out.push_str(&format!("  - {}: {}\n", s(e, "/position_id").unwrap_or("?"), s(e, "/reason").unwrap_or("?")));
            }
        }
        // A configured shape with nothing to deposit still states each rung.
        for row in l.pointer("/shape/rungs").and_then(|r| r.as_array()).into_iter().flatten() {
            if let Some(t) = s(row, "/text") {
                out.push_str(&format!("  {t}\n"));
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
    let limit = if s(l, "/gates/reserve/binding") == Some("wallet") { "wallet USDC binds" } else { "the reserve binds" };
    out.push_str(&format!(
        "  Reserve ${} holds at ${} ({limit}); NOCK budget: {}.\n",
        s(l, "/numbers/cash_reserve_usdc").unwrap_or("?"),
        s(l, "/gates/reserve/read_at_usd").unwrap_or("?"),
        s(l, "/gates/nock_budget/status").unwrap_or("unknown"),
    ));
    if let Some(t) = s(l, "/wallet_after/text") {
        out.push_str(&format!("  {t}\n"));
    }
    let stated: Vec<&str> = l.pointer("/stated/lines").and_then(|v| v.as_array()).into_iter().flatten().filter_map(|x| x.as_str()).collect();
    if !stated.is_empty() {
        out.push_str(&format!("  Stated, not gates: {}\n", stated.join(" ")));
    }
    let assumptions: Vec<&str> = l.get("assumptions").and_then(|v| v.as_array()).into_iter().flatten().filter_map(|x| x.as_str()).collect();
    if !assumptions.is_empty() {
        out.push_str(&format!("  Assumptions: {}\n", assumptions.join(" ")));
    }
    out.push_str(&rung_lines(l));
    out.push_str(&after_deposit_line(l.get("after_deposit")));
    out.push_str(&format!("  {}\n", s(l, "/statement").unwrap_or("")));
    out
}

/// One numbered block per rung: "Deposit 1 of 2 …", its range as ticks, USD and NOCK per
/// USDC, the amounts and the link. With a configured shape every rung of the shape is
/// listed in order: a deposit block for a rung left to deposit, the stated line for one
/// that is deployed, passed or over. Pure.
pub fn rung_lines(l: &Value) -> String {
    let rungs = l.get("rungs").and_then(|r| r.as_array()).cloned().unwrap_or_default();
    let total = rungs.len();
    let mut out = String::new();
    match l.pointer("/shape/rungs").and_then(|r| r.as_array()) {
        Some(rows) => {
            for row in rows {
                match rungs.iter().position(|r| r.get("index") == row.get("index")) {
                    Some(i) => out.push_str(&deposit_block(&rungs[i], i + 1, total)),
                    None => {
                        if let Some(t) = s(row, "/text") {
                            out.push_str(&format!("  {t}\n"));
                        }
                    }
                }
            }
        }
        None => {
            for (i, r) in rungs.iter().enumerate() {
                out.push_str(&deposit_block(r, i + 1, total));
            }
        }
    }
    // The link-format note once, after the links.
    if let Some(note) = rungs.iter().find_map(|r| s(r, "/deep_link/format_note")) {
        out.push_str(&format!("  {note}\n"));
    }
    out
}

/// "cap" reads "(cap)"; near and deep read "(near rung)" and "(deep rung)". Pure.
pub fn rung_name(r: &Value) -> String {
    match s(r, "/rung") {
        Some("cap") => " (cap)".to_string(),
        Some(n) => format!(" ({n} rung)"),
        None => String::new(),
    }
}

/// One deposit. A rung of a configured shape carries `summary`, its own number in the
/// shape (`index` of `of`), the exact mint, the NOCK limit when it holds NOCK and its fee
/// estimate; a rung without one prints as #168 shipped it. Pure.
pub fn deposit_block(r: &Value, position: usize, total: usize) -> String {
    let int = |k: &str| r.get(k).and_then(|v| v.as_i64()).map(|v| v.to_string()).unwrap_or_else(|| "?".into());
    let mut out = String::new();
    match s(r, "/summary") {
        Some(summary) => {
            out.push_str(&format!(
                "  Deposit {} of {}{}: {summary}. A new position in {}, ticks {} to {} (${} to ${}; {} to {} NOCK per USDC).\n",
                int("index"),
                int("of"),
                rung_name(r),
                s(r, "/pool/label").unwrap_or("?"),
                int("tick_lower"),
                int("tick_upper"),
                s(r, "/bounds_usd/low").unwrap_or("?"),
                s(r, "/bounds_usd/high").unwrap_or("?"),
                s(r, "/bounds_nock_per_usdc/low").unwrap_or("?"),
                s(r, "/bounds_nock_per_usdc/high").unwrap_or("?"),
            ));
            out.push_str(&format!(
                "    Exact mint: liquidity {}, {} raw USDC and {} raw NOCK.\n",
                s(r, "/mint/liquidity_raw").unwrap_or("?"),
                s(r, "/usdc/raw").unwrap_or("?"),
                s(r, "/nock/raw").unwrap_or("?"),
            ));
        }
        None => {
            let name = s(r, "/rung").map(|n| format!(" ({n} rung)")).unwrap_or_default();
            out.push_str(&format!(
                "  Deposit {position} of {total}{name}: {} USDC + {} NOCK as a new position in {}, ticks {} to {} (${} to ${}; {} to {} NOCK per USDC).\n",
                s(r, "/usdc/amount").unwrap_or("?"),
                s(r, "/nock/amount").unwrap_or("?"),
                s(r, "/pool/label").unwrap_or("?"),
                int("tick_lower"),
                int("tick_upper"),
                s(r, "/bounds_usd/low").unwrap_or("?"),
                s(r, "/bounds_usd/high").unwrap_or("?"),
                s(r, "/bounds_nock_per_usdc/low").unwrap_or("?"),
                s(r, "/bounds_nock_per_usdc/high").unwrap_or("?"),
            ));
        }
    }
    match s(r, "/deep_link/url") {
        Some(url) => {
            out.push_str(&format!("    Deposit link ({}): {url}\n", prefilled_words(r)));
            if let Some(a) = s(r, "/deep_link/amount_to_enter") {
                out.push_str(&format!("    {a}\n"));
            }
        }
        None => out.push_str(&format!("    Deposit link: none ({})\n", s(r, "/deep_link/reason").unwrap_or("not built"))),
    }
    if let Some(limit) = s(r, "/nock_limit/text") {
        out.push_str(&format!("    {limit}\n"));
    }
    if s(r, "/summary").is_some() {
        out.push_str(&format!("    {}\n", fee_estimate_line(r.get("fee_estimate"))));
    }
    out
}

/// 1836 -> "1,836". Pure.
fn grouped_int(v: i64) -> String {
    let digits = v.abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if v < 0 { format!("-{out}") } else { out }
}

/// A rung's fee estimate as one line (2.10.0), the same words as the dashboard's
/// `feeEstimateLine` (frontend/src/feeYield.ts). The figure is never printed without its
/// sample size, the in-range share of stored history and the replay bound; where one is
/// missing its reason is printed in its place, and an unmeasured estimate prints its
/// reason, never a zero. Pure.
pub fn fee_estimate_line(fe: Option<&Value>) -> String {
    let Some(fe) = fe.filter(|v| v.is_object()) else {
        return "Fees while in range: not stated in this body.".to_string();
    };
    let reason = |ptr: &str| s(fe, ptr).unwrap_or("no reason given").to_string();
    let per_day = s(fe, "/per_day_in_range_usd");
    let mut lines: Vec<String> = Vec::new();
    match (s(fe, "/status"), per_day) {
        (Some("measured"), Some(per_day)) => {
            let sample = match (s(fe, "/measured_days"), fe.get("intervals").and_then(|v| v.as_i64())) {
                (Some(days), Some(n)) => format!("over {days} days ({} intervals) on {}", grouped_int(n), s(fe, "/measured_label").unwrap_or("the measuring range")),
                _ => "sample size not stated".to_string(),
            };
            lines.push(format!("Fees while in range (an estimate): ${per_day}/day, measured {sample}"));
            lines.push(match s(fe, "/in_range_share_stored_history/share_pct") {
                Some(pct) => format!("In range {pct}% of the measured time"),
                None => format!("In range over stored history: not stated ({})", reason("/in_range_share_reason")),
            });
            lines.push(match (s(fe, "/stored_history_replay/inside_upper_usd"), s(fe, "/stored_history_replay/edge_upper_usd")) {
                (Some(inside), Some(edge)) => format!("Replayed over it: at most ${inside} inside and ${edge} more at its edges (no lower bound)"),
                _ => format!("Replay: not stated ({})", reason("/replay_reason")),
            });
            if fe.pointer("/coverage/whole").and_then(|v| v.as_bool()) == Some(false) {
                if let Some(note) = s(fe, "/coverage/note") {
                    lines.push(note.to_string());
                }
            }
        }
        _ => lines.push(format!("Fees while in range: not estimated: {}", reason("/reason"))),
    }
    let joined = lines.join(". ");
    if joined.ends_with('.') { joined } else { format!("{joined}.") }
}

/// What a deposit link prefills, in words, from the worker's own list. Pure.
pub fn prefilled_words(r: &Value) -> String {
    let list: Vec<&str> = r.pointer("/deep_link/prefilled").and_then(|v| v.as_array()).into_iter().flatten().filter_map(|x| x.as_str()).collect();
    let has = |k: &str| list.contains(&k);
    if list.is_empty() {
        return "nothing prefilled".to_string();
    }
    let mut parts = Vec::new();
    if has("pair") || has("fee and tick spacing") {
        parts.push("pool");
    }
    if has("range ticks") {
        parts.push("range");
    }
    if has("amount") {
        parts.push("amount");
    }
    let joined = match parts.as_slice() {
        [a] => a.to_string(),
        [a, b] => format!("{a} and {b}"),
        [a, b, c] => format!("{a}, {b} and {c}"),
        _ => parts.join(", "),
    };
    format!("{joined} prefilled")
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
