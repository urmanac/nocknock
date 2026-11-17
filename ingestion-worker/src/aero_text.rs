//! AERO lines of the MCP portfolio text (issue #93).
//!
//! One source file for both MCP transports: the worker's HTTP endpoint (`mod aero_text`
//! in main.rs) and the stdio binary, which cannot import from main.rs and includes this
//! file by path (src/bin/mcp.rs). It depends on nothing but serde_json and reward_text.rs,
//! which both binaries compile at their crate root.
//!
//! The text states amounts. It never says to claim, sell, hold or accumulate AERO. A
//! field the snapshot does not carry (an older snapshot, or a value the worker could
//! not read) prints as "not reported", never as 0.

use serde_json::Value;

/// What a surface says for an AERO figure the snapshot does not carry.
pub const AERO_NOT_REPORTED: &str = "not reported";

fn global_str<'a>(data: &'a Value, key: &str) -> Option<&'a str> {
    data.pointer("/derived_metrics/global_status")
        .and_then(|g| g.get(key))
        .and_then(|v| v.as_str())
        .filter(|v| !v.trim().is_empty())
}

/// The AERO entry of the "Pillars:" line: "$12.34", or "not reported".
pub fn aero_pillar_text(data: &Value) -> String {
    global_str(data, "wallet_aero_usd")
        .map(|v| format!("${v}"))
        .unwrap_or_else(|| AERO_NOT_REPORTED.to_string())
}

/// "Aero USDC/NOCK" or "Aero CL WETH/NOCK #2914490", from a snapshot position.
fn position_name(p: &Value) -> String {
    let sym = |k: &str| p.pointer(&format!("/{k}/symbol")).and_then(|v| v.as_str()).unwrap_or("?").to_string();
    let pair = format!("{}/{}", sym("token0"), sym("token1"));
    match p.get("protocol").and_then(|v| v.as_str()) {
        Some("aerodrome-cl") => format!("Aero CL {pair} #{}", p.get("nft_id").and_then(|v| v.as_u64()).unwrap_or(0)),
        Some("aerodrome") => format!("Aero {pair}"),
        Some(other) => format!("{other} {pair}"),
        None => pair,
    }
}

/// The AERO bullet lines of the portfolio text, newline-joined (no trailing newline):
/// what the wallet holds, what is unclaimed in the tracked gauges (with one row per
/// position), the price, and the stated reason for anything not read.
pub fn aero_lines(data: &Value) -> String {
    let amount_usd = |amount: &str, usd: Option<&str>| match usd {
        Some(u) => format!("{amount} AERO (${u})"),
        None => format!("{amount} AERO (USD value not reported)"),
    };
    let held = match global_str(data, "wallet_aero_amount") {
        Some(a) => format!(
            "{}, counted in the total; not cash, not deployable, no target",
            amount_usd(a, global_str(data, "wallet_aero_usd"))
        ),
        None => AERO_NOT_REPORTED.to_string(),
    };

    // One row per position that carries an AERO statement of its own.
    let mut rows = String::new();
    let (mut stated, mut read) = (0usize, 0usize);
    for p in data.get("positions").and_then(|v| v.as_array()).into_iter().flatten() {
        let s = |k: &str| p.get(k).and_then(|v| v.as_str()).filter(|v| !v.trim().is_empty());
        let row = match (s("unclaimed_aero"), s("unclaimed_aero_note")) {
            (Some(a), _) => {
                read += 1;
                let amount = amount_usd(a, s("unclaimed_aero_usd"));
                // Where it is collected (issue #142), when the snapshot says so.
                match p.get("collectible_reward").filter(|r| r.is_object()) {
                    Some(reward) => format!("{amount}, {}", crate::reward_text::reward_where_text(reward)),
                    None => amount,
                }
            }
            (None, Some(note)) => format!("not read ({note})"),
            (None, None) => continue,
        };
        stated += 1;
        rows.push_str(&format!("\n   - {}: {row}", position_name(p)));
    }
    let unclaimed = match global_str(data, "unclaimed_aero_amount") {
        Some(_) if stated == 0 => "none: no staked Aero position is tracked".to_string(),
        // Rows exist but none is an amount: the total would be a sum over nothing.
        Some(_) if read == 0 => "no gauge was read".to_string(),
        Some(a) => format!(
            "{} in tracked gauges, not counted in the total",
            amount_usd(a, global_str(data, "unclaimed_aero_usd"))
        ),
        None => AERO_NOT_REPORTED.to_string(),
    };
    let price = global_str(data, "aero_price_usd")
        .map(|p| format!("${p}"))
        .unwrap_or_else(|| AERO_NOT_REPORTED.to_string());
    let mut out = format!(
        "• AERO held (wallet): {held}\n• Unclaimed AERO: {unclaimed}{rows}\n• AERO/USD: {price}"
    );
    if let Some(note) = global_str(data, "aero_note") {
        out.push_str(&format!("\n• AERO not read: {note}"));
    }
    out
}
