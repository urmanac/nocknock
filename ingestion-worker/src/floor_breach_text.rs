//! The floor-crossing price (#165) as one line of text, shared by both MCP transports:
//! `mcp_text` (HTTP) and `bin/mcp.rs` (stdio) compile this same file. A measurement of
//! the deployed ranges, never an instruction.
use serde_json::Value;

/// The `derived_metrics.floor_breach` line for get_portfolio_state:
/// "• Floor crossing: ...\n". "" when no floor is configured (the stack line already says
/// so). A snapshot without the field says it predates 2.9.0, never "safe".
pub fn floor_breach_line(fb: Option<&Value>) -> String {
    let Some(fb) = fb.filter(|v| v.is_object()) else {
        return "• Floor crossing: not reported (snapshot predates 2.9.0)\n".to_string();
    };
    let s = |v: &Value, k: &str| v.get(k).and_then(|x| x.as_str()).filter(|x| !x.is_empty()).map(str::to_string);
    let status = s(fb, "status").unwrap_or_default();
    let floor = s(fb, "floor_nock").unwrap_or_else(|| "?".to_string());
    match status.as_str() {
        "no_floor" => return String::new(),
        "unavailable" => {
            let reason = s(fb, "reason").unwrap_or_else(|| "no reason given".to_string());
            return format!("• Floor crossing: unavailable: {reason}\n");
        }
        _ => {}
    }
    let crossing = fb.get("crossing").filter(|v| v.is_object());
    let price = |c: &Value| {
        format!(
            "NOCK ≥ ${} ({}x spot, +{}%)",
            s(c, "price_usd").unwrap_or_default(),
            s(c, "multiple").unwrap_or_default(),
            s(c, "pct_above_spot").unwrap_or_default()
        )
    };
    let remain = match (s(fb, "nock_if_all_sold"), s(fb, "wallet_nock"), s(fb, "fees_nock")) {
        (Some(all), Some(w), Some(f)) => format!("{all} NOCK would remain ({w} wallet + {f} uncollected fees)"),
        (Some(all), _, _) => format!("{all} NOCK would remain"),
        _ => "the NOCK that would remain is not reported".to_string(),
    };
    let tolerated = fb.get("tolerated").filter(|v| v.is_object()).map(|t| {
        let level = s(t, "level_nock").unwrap_or_default();
        match s(t, "status").as_deref() {
            Some("crosses") => format!(" Tolerated level {level} NOCK: crossed at {}.", price(t)),
            Some("already_below") => format!(" Tolerated level {level} NOCK: already below at spot."),
            _ => format!(" Tolerated level {level} NOCK: never crossed by a rise."),
        }
    });
    let tolerated = tolerated.unwrap_or_default();
    let positions: Vec<&Value> = fb.get("positions").and_then(|v| v.as_array()).map(|a| a.iter().collect()).unwrap_or_default();
    let by_label = |label: &str| positions.iter().copied().find(|p| s(p, "label").as_deref() == Some(label));
    let first = match s(fb, "first_to_sell_out") {
        Some(label) => {
            let at = by_label(&label)
                .map(|p| {
                    format!(
                        " at NOCK ≥ ${} ({}x spot)",
                        s(p, "sold_out_at_price_usd").unwrap_or_default(),
                        s(p, "sold_out_at_multiple").unwrap_or_default()
                    )
                })
                .unwrap_or_default();
            format!(" First range to sell out: {label}{at}.")
        }
        None if positions.iter().any(|p| p.get("full_range").and_then(|v| v.as_bool()) == Some(true)) => {
            " No range sells out: only full-range pools hold NOCK, and they sell gradually and never empty.".to_string()
        }
        None => String::new(),
    };
    const TAIL: &str = " Each pool moves with spot; WETH/NOCK ranges hold ETH/USD constant. A measurement, not an instruction.";
    match status.as_str() {
        "crosses" => {
            let at = crossing.map(price).unwrap_or_else(|| "a price not reported".to_string());
            let seller = s(fb, "largest_seller")
                .map(|label| {
                    let amounts = by_label(&label)
                        .and_then(|p| Some(format!(" ({} → {} NOCK)", s(p, "nock_now")?, s(p, "nock_at_crossing")?)))
                        .unwrap_or_default();
                    format!("; {label} sells the most by then{amounts}")
                })
                .unwrap_or_default();
            format!(
                "• Floor crossing: {at} would take the stack below the {floor} NOCK floor{seller}.{first} If every range sold out, {remain}.{tolerated}{TAIL}\n"
            )
        }
        "never" => format!(
            "• Floor crossing: none. If every range sold out, {remain}, at or above the {floor} NOCK floor.{first}{tolerated}{TAIL}\n"
        ),
        "already_below" => format!("• Floor crossing: already below the {floor} NOCK floor at spot.{tolerated}\n"),
        other => format!("• Floor crossing: unavailable: unknown status '{other}'\n"),
    }
}
