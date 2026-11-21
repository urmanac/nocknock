//! The standing protected cash reserve as one line of text (#162), shared by both MCP
//! transports: `mcp_text` (HTTP) and `bin/mcp.rs` (stdio) include this same file.
use serde_json::Value;

/// "• Protected cash reserve: $950.00 (...)\n"; a stated problem when the configured
/// reserve is invalid (`reserve_note`); "" when no reserve is set.
pub fn reserve_line(global: Option<&Value>) -> String {
    let get = |k: &str| global.and_then(|g| g.get(k)).and_then(|v| v.as_str()).filter(|v| !v.is_empty());
    let Some(reserve) = get("cash_reserve_usdc") else {
        return match get("reserve_note") {
            Some(note) => format!("• Protected cash reserve: not applied: {note}\n"),
            None => String::new(),
        };
    };
    // Tracked ladders (#167) are named when they hold USDC; otherwise the line is as before.
    let what = match get("reserve_ladder_usdc").filter(|l| l.trim().parse::<f64>().map_or(false, |v| v >= 0.005)) {
        Some(l) => format!("wallet + NOCK LP and ladder USDC (${l} in ladders)"),
        None => "wallet + NOCK LP USDC".to_string(),
    };
    let measured = match (get("reserve_stable_usdc"), get("reserve_headroom_usdc")) {
        (Some(stable), Some(h)) => match h.strip_prefix('-') {
            Some(below) => format!("{what} ${stable}, ${below} below it"),
            None => format!("{what} ${stable}, ${h} above it"),
        },
        _ => "not measured this cycle".to_string(),
    };
    let dry = get("dry_powder_usdc").map(|d| format!("; liquid dry powder ${d}")).unwrap_or_default();
    format!("• Protected cash reserve: ${reserve} ({measured}{dry}); kept for buying lower, never drawn by a plan buy\n")
}
