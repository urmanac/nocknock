//! Cost-basis lines for the MCP portfolio text (issue #98).
//!
//! This file is compiled into both binaries: the worker (`mod cost_basis_text` in
//! main.rs, used by the HTTP MCP transport) and the stdio MCP binary (`#[path]` in
//! bin/mcp.rs, used by its snapshot fallback). One source, so the two transports cannot
//! drift. It depends on serde_json only.
//!
//! The lines state what the NOCK cost and what it is worth at spot. They never suggest
//! buying, selling or averaging down.
use serde_json::Value;

fn text<'a>(v: Option<&'a Value>, key: &str) -> Option<&'a str> {
    v.and_then(|o| o.get(key)).and_then(|x| x.as_str()).filter(|s| !s.is_empty())
}

/// "At spot $S <subject> is worth $V: $D below cost (-P%)." or the reason there is no
/// comparison. `subject` names the NOCK being valued. The difference is the snapshot's
/// own signed figure, never recomputed.
fn at_spot_sentence(layer: Option<&Value>, spot: Option<&str>, subject: &str) -> String {
    let (Some(spot), Some(value), Some(diff)) =
        (spot, text(layer, "value_at_spot_usd"), text(layer, "unrealized_usd"))
    else {
        return "No spot price in this snapshot, so no comparison with cost.".to_string();
    };
    let pct = text(layer, "unrealized_pct")
        .map(|p| if p.starts_with('-') { format!(" ({p}%)") } else { format!(" (+{p}%)") })
        .unwrap_or_default();
    let relation = match diff.trim().parse::<f64>() {
        Ok(d) if d.abs() < 0.005 => "equal to cost".to_string(),
        Ok(d) if d < 0.0 => format!("${} below cost{pct}", diff.trim().trim_start_matches('-')),
        Ok(_) => format!("${} above cost{pct}", diff.trim()),
        Err(_) => "difference unavailable".to_string(),
    };
    format!("At spot ${spot} {subject} is worth ${value}: {relation}.")
}

fn plan_buys_line(cb: &Value, spot: Option<&str>) -> String {
    let layer = cb.get("plan_buys");
    let label = "• Plan buys average cost";
    match text(layer, "status") {
        Some("ok") => {
            let get = |k: &str| text(layer, k).unwrap_or("unavailable");
            let fills = layer.and_then(|l| l.get("fills")).and_then(|x| x.as_i64()).unwrap_or(0);
            format!(
                "{label}: ${} per NOCK over {fills} verified buy(s) (${} for {} NOCK). {}",
                get("avg_cost_usd"),
                get("spent_usdc"),
                get("acquired_nock"),
                at_spot_sentence(layer, spot, "that NOCK")
            )
        }
        Some("no_fills") => format!("{label}: no verified buys recorded yet"),
        Some("unavailable") => {
            format!("{label}: unavailable ({})", text(layer, "reason").unwrap_or("no reason reported"))
        }
        _ => format!("{label}: not reported by this snapshot"),
    }
}

fn whole_stack_line(cb: &Value, spot: Option<&str>) -> String {
    let layer = cb.get("whole_stack");
    let label = "• Whole-stack average cost";
    match text(layer, "status") {
        Some("ok") => {
            let get = |k: &str| text(layer, k).unwrap_or("unavailable");
            let since = layer.and_then(|l| l.get("fills_since")).and_then(|x| x.as_i64()).unwrap_or(0);
            // The value at spot is of the NOCK the basis covers, which is not a reading of
            // the wallet. The NOCK read now goes beside it so the two can be compared.
            let held = match text(layer, "held_nock") {
                Some(h) => format!(" NockNock reads {h} NOCK held now (wallet plus tracked LP)."),
                None => " The NOCK held now is not reported by this snapshot.".to_string(),
            };
            format!(
                "{label}: ${} per NOCK on {} NOCK (${}). Operator-entered opening basis: {} NOCK for ${} as of {}, source: {}; plus {since} verified buy(s) since. {}{held} Not tracked after the as-of: sells, LP rebalancing, fees earned, unrecorded buys.",
                get("avg_cost_usd"),
                get("basis_nock"),
                get("basis_cost_usd"),
                get("opening_nock"),
                get("opening_cost_usd"),
                get("as_of"),
                get("source"),
                at_spot_sentence(layer, spot, "the NOCK this basis covers")
            )
        }
        Some("not_configured") => format!(
            "{label}: not configured (cost_basis.opening is unset; NockNock does not derive it from the chain)"
        ),
        Some("invalid") => {
            format!("{label}: configuration error. {}", text(layer, "reason").unwrap_or("no reason reported"))
        }
        Some("unavailable") => {
            format!("{label}: unavailable ({})", text(layer, "reason").unwrap_or("no reason reported"))
        }
        _ => format!("{label}: not reported by this snapshot"),
    }
}

/// The two cost-basis lines of the portfolio text, from the snapshot's own
/// `derived_metrics.cost_basis`. A snapshot without the block says so: never $0.
pub fn cost_basis_lines(data: &Value) -> String {
    let Some(cb) = data.pointer("/derived_metrics/cost_basis").filter(|v| !v.is_null()) else {
        return "• Cost basis: not reported by this snapshot".to_string();
    };
    let spot = text(Some(cb), "spot_price_usd");
    format!("{}\n{}", plan_buys_line(cb, spot), whole_stack_line(cb, spot))
}
