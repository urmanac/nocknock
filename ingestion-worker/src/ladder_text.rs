//! The ladder simulator (#166) as MCP text, shared by both transports: `ladder` (the
//! worker's HTTP MCP) and `bin/mcp.rs` (stdio) include this same file, so both print the
//! same bytes for the same body. Pure: serde_json only. Measurement words only: never
//! "deposit", "should" or "recommended".
use serde_json::{json, Value};

/// The parameters the tool takes, in the order the query is built.
const ARG_NAMES: [&str; 10] =
    ["usdc", "nock", "low", "low_pct", "high", "high_pct", "position_id", "tick_spacing", "grid_to_pct", "grid_step_pct"];

/// The `simulate_ladder` entry of tools/list, one definition for both transports. No
/// union types: `high` is a number and `high_at_spot` a boolean.
pub fn simulate_ladder_tool() -> Value {
    json!({
        "name": "simulate_ladder",
        "description": "Read-only what-if for a USDC/NOCK range (a 'ladder') below spot, from the latest stored snapshot: the liquidity and NOCK the inputs imply, NOCK bought and USDC left at each price down the grid, the full-conversion price and average buy price, the price where it is out of range above (all USDC), the stable balance at the ladder's bottom against the protected cash reserve (counting the existing NOCK ranges' USDC converting on the way down), and whether its NOCK fits the floor + tolerance budget, with its effect on the floor-crossing price. Bounds snap to a tracked pool's tick spacing (position_id) or a spacing you give (tick_spacing). Nothing is read on-chain, signed or sent; the amounts are yours.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "usdc": { "type": "number", "exclusiveMinimum": 0, "description": "USDC the ladder holds at spot." },
                "nock": { "type": "number", "minimum": 0, "description": "Optional: the NOCK you would add with it; checked against what the range needs." },
                "low": { "type": "number", "exclusiveMinimum": 0, "description": "Bottom of the range, USD per NOCK (or give low_pct)." },
                "low_pct": { "type": "number", "description": "Bottom as % of the pool price, e.g. -30." },
                "high": { "type": "number", "exclusiveMinimum": 0, "description": "Top of the range, USD per NOCK (or high_pct, or high_at_spot)." },
                "high_at_spot": { "type": "boolean", "description": "True: the top is the pool price (on a snapped grid, the usable tick at or below it, holding no NOCK)." },
                "high_pct": { "type": "number", "description": "Top as % of the pool price, e.g. 2." },
                "position_id": { "type": "string", "description": "A tracked USDC/NOCK concentrated position whose pool and tick spacing the ladder would use." },
                "tick_spacing": { "type": "integer", "minimum": 1, "description": "A tick spacing you state, instead of position_id." },
                "grid_to_pct": { "type": "number", "description": "How far down the grid goes, % of spot (-95 to -1; default -40)." },
                "grid_step_pct": { "type": "number", "description": "Grid step, % of spot (0.5 to 20; default 5)." }
            },
            "required": ["usdc"]
        }
    })
}

fn invalid(detail: String) -> Value {
    json!({ "error": "INVALID_PARAMETER", "detail": detail })
}

/// MCP `arguments` as the GET /v1/simulate/ladder query: numbers or numeric strings,
/// `high_at_spot: true` as `high=spot`. Err is a 400 body. Pure.
pub fn ladder_query_from_args(args: Option<&Value>) -> Result<Vec<(String, String)>, Value> {
    let empty = serde_json::Map::new();
    let obj = match args {
        None | Some(Value::Null) => &empty,
        Some(Value::Object(m)) => m,
        Some(_) => return Err(invalid("arguments must be an object".to_string())),
    };
    let mut keys: Vec<&String> = obj.keys().collect();
    keys.sort();
    for k in keys {
        if !ARG_NAMES.contains(&k.as_str()) && k != "high_at_spot" {
            return Err(invalid(format!("unknown argument '{k}'")));
        }
    }
    let at_spot = match obj.get("high_at_spot") {
        None | Some(Value::Null) | Some(Value::Bool(false)) => false,
        Some(Value::Bool(true)) => true,
        Some(other) => return Err(invalid(format!("high_at_spot must be true or false, not {other}"))),
    };
    if at_spot && obj.get("high").map_or(false, |v| !v.is_null()) {
        return Err(invalid("give high or high_at_spot, not both".to_string()));
    }
    let mut out = Vec::new();
    for k in ARG_NAMES {
        if k == "high" && at_spot {
            out.push(("high".to_string(), "spot".to_string()));
            continue;
        }
        let v = match obj.get(k) {
            None | Some(Value::Null) => continue,
            Some(Value::Number(n)) => n.to_string(),
            Some(Value::String(s)) => s.trim().to_string(),
            Some(other) => return Err(invalid(format!("{k} must be a number, not {other}"))),
        };
        out.push((k.to_string(), v));
    }
    Ok(out)
}

/// "65,563" from 65563.49. Pure.
fn grouped(v: f64) -> String {
    let n = v.round() as i64 + 0;
    let digits = n.abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if n < 0 { format!("-{out}") } else { out }
}

fn s<'a>(v: &'a Value, ptr: &str) -> Option<&'a str> {
    v.pointer(ptr).and_then(|x| x.as_str())
}

fn num(v: &Value, ptr: &str) -> Option<f64> {
    s(v, ptr).and_then(|x| x.parse::<f64>().ok())
}

/// "+$26.45" / "-$151.37". Pure.
fn signed_usd(h: &str) -> String {
    match h.strip_prefix('-') {
        Some(abs) => format!("-${abs}"),
        None => format!("+${h}"),
    }
}

fn nock_txt(v: &Value, ptr: &str) -> String {
    num(v, ptr).map(grouped).unwrap_or_else(|| "unknown".to_string())
}

/// The simulation as text. Pure.
pub fn ladder_text(b: &Value) -> String {
    let or = |ptr: &str, d: &str| s(b, ptr).unwrap_or(d).to_string();
    let mut out = String::from("🧮 Ladder Simulation (what-if, read-only)\n=========================================\n");

    let snapped = b.pointer("/pool/snapped").and_then(|v| v.as_bool()).unwrap_or(false);
    let pool_line = if snapped {
        format!(
            "snapped to {} (tick spacing {}{}; top: {})",
            or("/pool/label", "?"),
            b.pointer("/pool/tick_spacing").map(|v| v.to_string()).unwrap_or_default(),
            if s(b, "/pool/spacing_source") == Some("caller") { ", given by you" } else { "" },
            or("/bounds/high_rule", "?")
        )
    } else {
        "unsnapped: no position_id or tick_spacing was given".to_string()
    };
    out.push_str(&format!(
        "• Range: ${} – ${} ({}% / {}% of the pool price ${}), {}\n",
        or("/bounds/low_usd", "?"),
        or("/bounds/high_usd", "?"),
        or("/bounds/low_pct", "?"),
        or("/bounds/high_pct", "?"),
        or("/pool/pool_price_usd", "?"),
        pool_line
    ));
    if s(b, "/pool/pool_price_vs_spot_pct").map_or(false, |p| p != "0.00") {
        out.push_str(&format!(
            "• Pool price ${} is {}% from spot ${}; every pool is moved by the same % from its own price\n",
            or("/pool/pool_price_usd", "?"),
            or("/pool/pool_price_vs_spot_pct", "?"),
            or("/snapshot/spot_usd", "?")
        ));
    }
    if let Some(note) = s(b, "/pool/note").filter(|_| snapped) {
        out.push_str(&format!("• Pool: {note}\n"));
    }
    let check = match s(b, "/sizing/nock_check") {
        Some("matches") => format!("; the {} NOCK given matches", or("/sizing/nock_given", "?")),
        _ => String::new(),
    };
    out.push_str(&format!(
        "• Sizing: ${} USDC and {} NOCK at spot (liquidity {}{check})\n",
        or("/sizing/usdc_at_spot", "?"),
        nock_txt(b, "/sizing/nock_needed_at_spot"),
        or("/sizing/liquidity_human", "?")
    ));
    match b.get("top_neighbours").and_then(|v| v.as_array()) {
        Some(rows) => {
            let parts: Vec<String> = rows
                .iter()
                .map(|r| {
                    let need = match num(r, "/nock_needed") {
                        Some(n) => format!("needs {} NOCK", grouped(n)),
                        None => "is the bottom's own tick".to_string(),
                    };
                    let fit = match r.get("fits_budget").and_then(|v| v.as_bool()) {
                        Some(true) => " (fits the budget)",
                        Some(false) => " (exceeds the budget)",
                        None => "",
                    };
                    format!("${} ({}%) {need}{fit}", or_v(r, "/price_usd"), or_v(r, "/pct_of_spot"))
                })
                .collect();
            out.push_str(&format!("• Tops next to spot: {}\n", parts.join("; ")));
        }
        None => {
            if let Some(why) = s(b, "/top_neighbours_reason") {
                out.push_str(&format!("• Tops next to spot: none listed ({why})\n"));
            }
        }
    }
    out.push_str("• Price grid (NOCK bought by the ladder, USDC left in it; stable balance and reserve headroom):\n");
    for row in b.get("grid").and_then(|v| v.as_array()).cloned().unwrap_or_default() {
        let stable = match (s(&row, "/stable_usdc"), s(&row, "/reserve_headroom_usdc")) {
            (Some(st), Some(h)) => format!("; stable ${st} (reserve {})", signed_usd(h)),
            (Some(st), None) => format!("; stable ${st}"),
            _ => "; stable unknown".to_string(),
        };
        let markers: Vec<String> = row
            .get("markers")
            .and_then(|m| m.as_array())
            .map(|m| m.iter().filter_map(|x| x.as_str().map(|x| x.to_string())).collect())
            .unwrap_or_default();
        let mark = if markers.is_empty() { String::new() } else { format!(" [{}]", markers.join(", ")) };
        // Above spot the ladder sells the NOCK it held at spot: said as sold, never as a
        // negative purchase.
        let moved = match num(&row, "/ladder_nock_acquired") {
            Some(n) if n.round() < 0.0 => format!("{} NOCK sold", grouped(-n)),
            Some(n) => format!("{} NOCK bought", grouped(n)),
            None => "unknown NOCK bought".to_string(),
        };
        out.push_str(&format!(
            "  {}% ${}: {moved}, ${} USDC left{stable}{mark}\n",
            or_v(&row, "/pct_of_spot"),
            or_v(&row, "/price_usd"),
            or_v(&row, "/ladder_usdc"),
        ));
    }
    out.push_str(&format!(
        "• Bottom: fully NOCK at ${} ({}%): {} NOCK held, {} bought, average ${}\n",
        or("/bottom/price_usd", "?"),
        or("/bottom/pct_of_spot", "?"),
        nock_txt(b, "/bottom/nock_held"),
        nock_txt(b, "/bottom/nock_bought"),
        or("/bottom/avg_buy_price_usd", "unknown"),
    ));
    let top = if b.pointer("/top/out_of_range_above_now").and_then(|v| v.as_bool()) == Some(true) {
        format!(
            "• Top: ${} ({}%): out of range above now, all USDC (${}); it enters range when NOCK falls to ${}\n",
            or("/top/price_usd", "?"),
            or("/top/pct_of_spot", "?"),
            or("/top/usdc_held", "?"),
            or("/top/price_usd", "?"),
        )
    } else {
        format!(
            "• Top: out of range above at ${} ({}%): ${} USDC; the NOCK held at spot sells at an average ${}\n",
            or("/top/price_usd", "?"),
            or("/top/pct_of_spot", "?"),
            or("/top/usdc_held", "?"),
            or("/top/avg_sell_price_usd", "unknown"),
        )
    };
    out.push_str(&top);

    // Reserve.
    let rc = |p: &str| s(b, &format!("/reserve_check/{p}")).map(|x| x.to_string());
    let reserve = match rc("status").as_deref() {
        Some("holds") | Some("breached") => {
            let margin = rc("margin_usdc").unwrap_or_default();
            let verdict = match margin.strip_prefix('-') {
                Some(abs) => format!("breached: below it by ${abs}"),
                None => format!("holds by ${margin}"),
            };
            format!(
                "• Reserve: stable ${} at the bottom against the ${} reserve: {verdict} (this range takes at most ${}; the existing NOCK ranges hold ${} USDC there). {}\n",
                rc("stable_at_bottom_usdc").unwrap_or_default(),
                rc("cash_reserve_usdc").unwrap_or_default(),
                rc("max_usdc_for_this_range").unwrap_or_else(|| "unknown".to_string()),
                rc("existing_ranges_usdc_at_bottom").unwrap_or_default(),
                rc("gate").unwrap_or_default(),
            )
        }
        Some(st) => format!(
            "• Reserve: {st}: {}{}\n",
            rc("reason").unwrap_or_default(),
            rc("stable_at_bottom_usdc").map(|x| format!("; stable at the bottom ${x}")).unwrap_or_default()
        ),
        None => "• Reserve: unknown\n".to_string(),
    };
    out.push_str(&reserve);
    if let Some(bb) = b.pointer("/reserve_check/existing_ranges_below_bottom").filter(|v| !v.is_null()) {
        let h = s(bb, "/headroom_usdc").map(|h| format!(" (reserve {})", signed_usd(h))).unwrap_or_default();
        out.push_str(&format!(
            "• Below the bottom: at ${} ({}%) stable ${}{h}. {}\n",
            or_v(bb, "/price_usd"),
            or_v(bb, "/pct_of_spot"),
            or_v(bb, "/stable_usdc"),
            or_v(bb, "/note"),
        ));
    }
    if let Some(t) = b.pointer("/reserve_check/at_top") {
        let h = s(t, "/headroom_usdc").map(|h| format!(" (reserve {})", signed_usd(h))).unwrap_or_default();
        let st = s(t, "/stable_usdc").map(|x| format!("stable ${x}{h} at ${}. ", or_v(t, "/price_usd"))).unwrap_or_default();
        out.push_str(&format!("• At the top: {st}{}\n", or_v(t, "/note")));
    }
    let plan = match b.pointer("/reserve_check/accumulation_plan_unspent").filter(|v| !v.is_null()) {
        Some(p) => format!("the accumulation plan's unspent ${}", or_v(p, "/usdc")),
        None => format!("no accumulation plan ({})", or("/reserve_check/accumulation_plan_unspent_reason", "not configured")),
    };
    let tax = b
        .pointer("/reserve_check/tax_reserve_usd/usd")
        .and_then(|v| v.as_str())
        .map(|t| format!("the ${t} tax reserve"))
        .unwrap_or_else(|| "the tax reserve is not reported".to_string());
    out.push_str(&format!("• Stated, not gates: {plan}; {tax}\n"));

    // Floor budget.
    let fc = |p: &str| s(b, &format!("/floor_check/{p}")).map(|x| x.to_string());
    let budget_note = fc("budget_note").map(|n| format!(" ({n})")).unwrap_or_default();
    let floor = match fc("status").as_deref() {
        Some(st @ ("fits" | "exceeds")) => {
            let cap = fc("max_usdc_within_budget").map(|m| format!("; ${m} is the most USDC this range holds within it")).unwrap_or_default();
            format!(
                "• Floor budget: needs {} of the {} NOCK that may go into LP{budget_note}: {st}{cap}. {}\n",
                nock_txt(b, "/floor_check/nock_needed"),
                fc("lp_nock_budget").map(|v| v.parse::<f64>().map(grouped).unwrap_or(v)).unwrap_or_default(),
                fc("rebuild_note").map(|n| format!("{}{}", n[..1].to_uppercase(), &n[1..])).unwrap_or_default(),
            )
        }
        _ => format!("• Floor budget: unknown: {}\n", fc("reason").unwrap_or_default()),
    };
    out.push_str(&floor);
    let fe = |p: &str| s(b, &format!("/floor_check/floor_breach_effect/{p}")).map(|x| x.to_string());
    let crossing = |which: &str| -> String {
        let base = format!("/floor_check/floor_breach_effect/{which}");
        match s(b, &format!("{base}/crossing/price_usd")) {
            Some(p) => format!("${p} ({}%)", s(b, &format!("{base}/crossing/pct_above_spot")).map(|x| format!("+{x}")).unwrap_or_default()),
            None => s(b, &format!("{base}/status")).unwrap_or("unknown").to_string(),
        }
    };
    match fe("status").as_deref() {
        Some("unknown") | None => out.push_str(&format!("• Floor-crossing price: unknown: {}\n", fe("reason").unwrap_or_default())),
        Some("no_floor") => out.push_str("• Floor-crossing price: no floor is set\n"),
        Some(st) => out.push_str(&format!(
            "• Floor-crossing price: {st}: {} (without the ladder {}, with it {})\n",
            fe("reason").unwrap_or_default(),
            crossing("before"),
            crossing("after"),
        )),
    }
    let assumptions: Vec<String> = b
        .get("assumptions")
        .and_then(|a| a.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(|x| x.to_string())).collect())
        .unwrap_or_default();
    out.push_str(&format!("• Assumptions: {}\n", assumptions.join(" ")));
    out.push_str(&format!("• {}\n", or("/statement", "")));
    out.push_str(&snapshot_line(b));
    out
}

fn or_v(v: &Value, ptr: &str) -> String {
    s(v, ptr).unwrap_or("?").to_string()
}

/// The source line, from the body's own snapshot block, in the words the other tools'
/// footer uses. Pure.
fn snapshot_line(b: &Value) -> String {
    format!(
        "Snapshot generated {} ({} min ago) · worker health: {}",
        s(b, "/snapshot/generated_at").unwrap_or("at an unknown time"),
        b.pointer("/snapshot/age_minutes").and_then(|v| v.as_i64()).map(|m| m.to_string()).unwrap_or_else(|| "?".to_string()),
        s(b, "/snapshot/worker_health").unwrap_or("unavailable"),
    )
}

/// A 400 or 422 body as text: the stated cause, and the NOCK the range needs as a
/// measured figure (never an instruction). Pure.
pub fn ladder_error_text(e: &Value) -> String {
    let what = s(e, "/code").or_else(|| s(e, "/error")).unwrap_or("ERROR");
    let mut out = format!(
        "🧮 Ladder Simulation: not simulated\n=========================================\n• {what}: {}\n",
        s(e, "/detail").unwrap_or("no detail")
    );
    if let Some(n) = s(e, "/nock_needed") {
        out.push_str(&format!("• Measured: this range needs {n} NOCK at the pool price for this USDC\n"));
    }
    if let Some(bd) = e.get("bounds").filter(|v| !v.is_null()) {
        out.push_str(&format!(
            "• Bounds: ${} – ${} ({}% / {}%)\n",
            or_v(bd, "/low_usd"),
            or_v(bd, "/high_usd"),
            or_v(bd, "/low_pct"),
            or_v(bd, "/high_pct")
        ));
    }
    out.push_str("• Nothing was read on-chain, signed or sent.");
    out
}
