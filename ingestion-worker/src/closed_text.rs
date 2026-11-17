//! Closed positions in the MCP text (issue #144).
//!
//! One source file for both MCP transports: the worker's HTTP endpoint (`mod closed_text`
//! in main.rs) and the stdio binary, which cannot import from main.rs and includes this
//! file by path (src/bin/mcp.rs). It depends on nothing but serde_json.
//!
//! A closed position is a configured position whose NFT no longer exists. The text
//! states that fact, that the position is in no total, and that its config entry can be
//! removed. It prints no amount and no value for it: none was read, and its last known
//! value is not a current one. A snapshot that lists no closed position (every snapshot
//! from before #144, and every one without a closed position) gets no line at all, so
//! its text is what it was.

use serde_json::Value;

fn str_field<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(|x| x.as_str()).filter(|x| !x.trim().is_empty())
}

/// The snapshot's closed positions; empty for a snapshot that lists none.
fn closed_entries(data: &Value) -> &[Value] {
    data.get("closed_positions").and_then(|v| v.as_array()).map(Vec::as_slice).unwrap_or(&[])
}

/// One closed position as a statement of fact. The reason is the worker's own sentence
/// when the snapshot carries it.
pub fn closed_position_statement(entry: &Value) -> String {
    let label = str_field(entry, "label").or_else(|| str_field(entry, "config_id")).unwrap_or("position");
    let reason = match str_field(entry, "reason") {
        Some(r) => r.to_string(),
        None => match entry.get("nft_id").and_then(|n| n.as_u64()) {
            Some(id) => format!("NFT #{id} no longer exists (withdrawn and burned, or a mistyped nft_id)"),
            None => "its NFT no longer exists (withdrawn and burned, or a mistyped nft_id)".to_string(),
        },
    };
    let mut text = format!("{label}: closed. {reason}. Not counted in any total.");
    if entry.get("supplies_spot_price").and_then(|v| v.as_bool()) == Some(true) {
        text.push_str(" Its pool still supplies the NOCK/USD spot: flag another position as is_price_source before removing its entry from config.yaml.");
    } else {
        match str_field(entry, "config_id") {
            Some(id) => text.push_str(&format!(" You can remove its entry ({id}) from config.yaml.")),
            None => text.push_str(" You can remove its entry from config.yaml."),
        }
    }
    text
}

/// The closed-positions block of a text: a header line and one line per position, each
/// ending in a newline, so the caller places the block directly before its next line.
/// Empty when the snapshot lists no closed position.
pub fn closed_positions_lines(data: &Value) -> String {
    let entries = closed_entries(data);
    if entries.is_empty() {
        return String::new();
    }
    let mut out = format!("• Closed positions ({}), not counted in any total:\n", entries.len());
    for e in entries {
        out.push_str(&format!("   ✗ {}\n", closed_position_statement(e)));
    }
    out
}
