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
    let measured = match (get("reserve_stable_usdc"), get("reserve_headroom_usdc")) {
        (Some(stable), Some(h)) => match h.strip_prefix('-') {
            Some(below) => format!("wallet + NOCK LP USDC ${stable}, ${below} below it"),
            None => format!("wallet + NOCK LP USDC ${stable}, ${h} above it"),
        },
        _ => "not measured this cycle".to_string(),
    };
    let dry = get("dry_powder_usdc").map(|d| format!("; liquid dry powder ${d}")).unwrap_or_default();
    format!("• Protected cash reserve: ${reserve} ({measured}{dry}); kept for buying lower, never drawn by a plan buy\n")
}
