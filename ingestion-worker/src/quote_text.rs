//! Plain-text rendering of the due tranche's route quotes (issue #115).
//!
//! Compiled into both binaries, like fill_text.rs: the worker (`mod quote_text`) and the
//! stdio MCP server (`#[path = "../quote_text.rs"] mod quote_text`), which cannot import
//! from main.rs. One function, one text, so the two MCP transports cannot drift. It
//! depends on serde_json only and reads the snapshot's `accumulation_plan` object.

use serde_json::Value;

/// Said under every set of quotes, on every surface.
pub const ROUTE_QUOTE_CAVEAT: &str = "Quotes are contract reads at the stated block: estimates as of that time, not a promise. The wallet app's own router may pick a different route. This compares execution routes for a buy your plan already sized; it is not a recommendation to buy.";

/// Lines for `accumulation_plan.route_quotes`, each ending in a newline. Empty when the
/// snapshot carries no quotes (no tranche due, quoting off, or a snapshot from before
/// v0.7.0): nothing is said about quotes that were never requested.
///
/// A venue that could not be quoted reads "unavailable" with its reason. No number is
/// derived here: every figure is printed as the snapshot carries it.
pub fn route_quote_lines(plan: &Value) -> String {
    let Some(q) = plan.get("route_quotes").filter(|v| !v.is_null()) else {
        return String::new();
    };
    let text = |v: &Value, k: &str| -> Option<String> {
        v.get(k).and_then(|x| x.as_str()).filter(|x| !x.is_empty()).map(|x| x.to_string())
    };
    // NOCK amounts are exact 16-decimal strings; four decimals are enough to read.
    let nock = |s: &str| -> String {
        match s.split_once('.') {
            Some((whole, frac)) => format!("{whole}.{:0<4}", frac.chars().take(4).collect::<String>()),
            None => format!("{s}.0000"),
        }
    };
    let amount = text(q, "amount_usdc").unwrap_or_else(|| "n/a".to_string());
    let at = match (q.get("block_number").and_then(|b| b.as_i64()), text(q, "quoted_at")) {
        (Some(block), Some(time)) => format!("block {block} ({time})"),
        (Some(block), None) => format!("block {block}"),
        _ => "no block (nothing was read)".to_string(),
    };
    let spot = text(q, "reference_spot_usd")
        .map(|s| format!("reference spot ${s}"))
        .unwrap_or_else(|| "no reference spot".to_string());
    let mut out = format!("• Route quotes for ${amount} USDC → NOCK at {at}, {spot}:\n");

    for v in q.get("venues").and_then(|v| v.as_array()).into_iter().flatten() {
        let venue = text(v, "venue").unwrap_or_else(|| "venue n/a".to_string());
        let url = text(v, "swap_url").unwrap_or_default();
        if text(v, "status").as_deref() != Some("quoted") {
            out.push_str(&format!(
                "   {venue}: unavailable ({})\n",
                text(v, "reason").unwrap_or_else(|| "no reason given".to_string())
            ));
            continue;
        }
        let cost = text(v, "cost_over_spot_pct")
            .map(|c| {
                let signed = if c.starts_with('-') { c } else { format!("+{c}") };
                format!("{signed}% over the reference spot")
            })
            .unwrap_or_else(|| "cost over spot unavailable".to_string());
        let hops: Vec<String> = v
            .get("hops")
            .and_then(|h| h.as_array())
            .into_iter()
            .flatten()
            .map(|h| {
                format!(
                    "{}→{} {} {} fee {}",
                    text(h, "token_in").unwrap_or_else(|| "?".to_string()),
                    text(h, "token_out").unwrap_or_else(|| "?".to_string()),
                    text(h, "pool_kind").unwrap_or_else(|| "pool".to_string()),
                    text(h, "pool").unwrap_or_else(|| "n/a".to_string()),
                    text(h, "fee_pct").map(|f| format!("{f}%")).unwrap_or_else(|| "not read".to_string()),
                )
            })
            .collect();
        out.push_str(&format!(
            "   {venue}: {} NOCK at ${} ({cost}) via {}\n",
            text(v, "nock_out").map(|n| nock(&n)).unwrap_or_else(|| "n/a".to_string()),
            text(v, "effective_price_usd").unwrap_or_else(|| "n/a".to_string()),
            text(v, "route").unwrap_or_else(|| "route n/a".to_string()),
        ));
        for h in hops {
            out.push_str(&format!("      {h}\n"));
        }
        if !url.is_empty() {
            out.push_str(&format!("      swap page: {url} (enter ${amount} USDC → NOCK yourself; nothing is prefilled)\n"));
        }
    }
    if let Some(comparison) = text(q, "comparison") {
        out.push_str(&format!("   {comparison}\n"));
    }
    out.push_str(&format!("   {ROUTE_QUOTE_CAVEAT}\n"));
    out
}
