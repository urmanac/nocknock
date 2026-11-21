//! The ladder simulator (#166) as MCP text, shared by both transports: `ladder` (the
//! worker's HTTP MCP) and `bin/mcp.rs` (stdio) include this same file, so both print the
//! same bytes for the same body. Pure: serde_json only. Measurement words only: never
//! "deposit", "should", "best", "suggest" or "recommended".
use serde_json::{json, Value};

/// The parameters the tool takes, in the order the query is built.
const ARG_NAMES: [&str; 10] =
    ["usdc", "nock", "low", "low_pct", "high", "high_pct", "position_id", "tick_spacing", "grid_to_pct", "grid_step_pct"];
/// The parameters the flat form shares with a ladder of rungs.
const SHARED_ARG_NAMES: [&str; 4] = ["position_id", "tick_spacing", "grid_to_pct", "grid_step_pct"];
/// One rung's arguments inside `rungs`, in the order the query is built.
const RUNG_ARG_NAMES: [&str; 7] = ["rung", "usdc", "nock", "low", "low_pct", "high", "high_pct"];
/// The most rungs one call simulates (ladder.rs LADDER_MAX_RUNGS; a display limit).
const MAX_RUNGS: usize = 4;

/// The `simulate_ladder` entry of tools/list, one definition for both transports. No
/// union types: `high` is a number and `high_at_spot` a boolean.
pub fn simulate_ladder_tool() -> Value {
    json!({
        "name": "simulate_ladder",
        "description": "Read-only what-if for a USDC/NOCK range (a 'ladder') below spot, or a ladder of up to 4 rungs (cap, near and deep) in one pool, from the latest stored snapshot: the liquidity and NOCK the inputs imply, NOCK bought and USDC left at each price down the grid, the full-conversion price and average buy price, the price where it is out of range above (all USDC), the stable balance at the reserve gate (the lower of the deepest bottom and your configured ladder bottom) against the protected cash reserve (counting the existing NOCK ranges' USDC converting on the way down), whether its NOCK fits the floor + tolerance budget, its effect on the floor-crossing price, and estimated fees per day while in range, measured from your own tracked range (with the in-range share of stored history and replay upper bounds beside it). With rungs it also draws trade-off tables (split and depths) as measurements, unranked. shape: 'configured' runs your own operator_profile.ladder_rungs. Bounds snap to a tracked pool's tick spacing (position_id) or a spacing you give (tick_spacing). Nothing is read on-chain, signed or sent; the amounts are yours.",
        "inputSchema": {
            "type": "object",
            "properties": {
                "usdc": { "type": "number", "exclusiveMinimum": 0, "description": "One-rung form: USDC the ladder holds at spot." },
                "nock": { "type": "number", "minimum": 0, "description": "One-rung form, optional: the NOCK you would add with it; checked against what the range needs." },
                "low": { "type": "number", "exclusiveMinimum": 0, "description": "One-rung form: bottom of the range, USD per NOCK (or give low_pct)." },
                "low_pct": { "type": "number", "description": "One-rung form: bottom as % of the pool price, e.g. -30." },
                "high": { "type": "number", "exclusiveMinimum": 0, "description": "One-rung form: top of the range, USD per NOCK (or high_pct, or high_at_spot)." },
                "high_at_spot": { "type": "boolean", "description": "One-rung form: true when the top is the pool price (on a snapped grid, the usable tick at or below it, holding no NOCK)." },
                "high_pct": { "type": "number", "description": "One-rung form: top as % of the pool price, e.g. 2." },
                "rungs": {
                    "type": "array",
                    "maxItems": MAX_RUNGS,
                    "description": "A ladder of rungs in one pool, instead of the one-rung arguments: each {rung: cap|near|deep (required), usdc, nock?, low or low_pct, high or high_pct or high_at_spot}.",
                    "items": {
                        "type": "object",
                        "properties": {
                            "rung": { "type": "string", "enum": ["cap", "near", "deep"], "description": "The rung's role; required, never assumed." },
                            "usdc": { "type": "number", "exclusiveMinimum": 0 },
                            "nock": { "type": "number", "minimum": 0 },
                            "low": { "type": "number", "exclusiveMinimum": 0 },
                            "low_pct": { "type": "number" },
                            "high": { "type": "number", "exclusiveMinimum": 0 },
                            "high_pct": { "type": "number" },
                            "high_at_spot": { "type": "boolean" }
                        },
                        "required": ["rung", "usdc"]
                    }
                },
                "shape": { "type": "string", "enum": ["configured"], "description": "'configured': your own operator_profile.ladder_rungs, instead of rungs or the one-rung arguments." },
                "shape_usdc": { "type": "number", "exclusiveMinimum": 0, "description": "With shape: the total USDC to size it to (default: the most the shared reserve gate allows)." },
                "split_step_pct": { "type": "number", "description": "With rungs: the split table's step, % of the total (5 to 50; display default 10)." },
                "position_id": { "type": "string", "description": "A tracked USDC/NOCK concentrated position whose pool and tick spacing the ladder would use." },
                "tick_spacing": { "type": "integer", "minimum": 1, "description": "A tick spacing you state, instead of position_id." },
                "grid_to_pct": { "type": "number", "description": "How far down the grid goes, % of spot (-95 to -1; default -40)." },
                "grid_step_pct": { "type": "number", "description": "Grid step, % of spot (0.5 to 20; default 5)." }
            }
        }
    })
}

fn invalid(detail: String) -> Value {
    json!({ "error": "INVALID_PARAMETER", "detail": detail })
}

/// One argument value as a query value: a number or a numeric string.
fn arg_value(k: &str, v: &Value) -> Result<Option<String>, Value> {
    match v {
        Value::Null => Ok(None),
        Value::Number(n) => Ok(Some(n.to_string())),
        Value::String(s) => Ok(Some(s.trim().to_string())),
        other => Err(invalid(format!("{k} must be a number, not {other}"))),
    }
}

fn at_spot_flag(k: &str, v: Option<&Value>) -> Result<bool, Value> {
    match v {
        None | Some(Value::Null) | Some(Value::Bool(false)) => Ok(false),
        Some(Value::Bool(true)) => Ok(true),
        Some(other) => Err(invalid(format!("{k} must be true or false, not {other}"))),
    }
}

/// MCP `arguments` as the GET /v1/simulate/ladder query: numbers or numeric strings,
/// `high_at_spot: true` as `high=spot`, and a `rungs` array as the indexed `r1_..`
/// parameters. Err is a 400 body. Pure.
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
        if !ARG_NAMES.contains(&k.as_str()) && !["high_at_spot", "rungs", "shape", "shape_usdc", "split_step_pct"].contains(&k.as_str()) {
            return Err(invalid(format!("unknown argument '{k}'")));
        }
    }
    let rungs = match obj.get("rungs") {
        None | Some(Value::Null) => None,
        Some(Value::Array(a)) => Some(a),
        Some(other) => return Err(invalid(format!("rungs must be an array of rung objects, not {other}"))),
    };
    let mut out = Vec::new();
    if let Some(rungs) = rungs {
        if let Some(k) = ["usdc", "nock", "low", "low_pct", "high", "high_pct", "high_at_spot"].iter().find(|k| obj.get(**k).map_or(false, |v| !v.is_null())) {
            return Err(invalid(format!("'{k}' is the one-rung form; with rungs give every rung's values inside rungs, not both")));
        }
        if rungs.is_empty() {
            return Err(invalid("rungs is empty: give at least one rung".to_string()));
        }
        if rungs.len() > MAX_RUNGS {
            return Err(invalid(format!("at most {MAX_RUNGS} rungs (a display limit) are simulated in one call; {} were given", rungs.len())));
        }
        for (i, r) in rungs.iter().enumerate() {
            let n = i + 1;
            let Value::Object(m) = r else {
                return Err(invalid(format!("rungs[{i}] must be an object")));
            };
            let mut rk: Vec<&String> = m.keys().collect();
            rk.sort();
            for k in rk {
                if !RUNG_ARG_NAMES.contains(&k.as_str()) && k != "high_at_spot" {
                    return Err(invalid(format!("unknown argument '{k}' in rungs[{i}]")));
                }
            }
            let at_spot = at_spot_flag(&format!("rungs[{i}].high_at_spot"), m.get("high_at_spot"))?;
            if at_spot && m.get("high").map_or(false, |v| !v.is_null()) {
                return Err(invalid(format!("rungs[{i}]: give high or high_at_spot, not both")));
            }
            for k in RUNG_ARG_NAMES {
                let name = format!("r{n}_{k}");
                if k == "high" && at_spot {
                    out.push((name, "spot".to_string()));
                    continue;
                }
                if let Some(v) = m.get(k) {
                    if let Some(v) = arg_value(&format!("rungs[{i}].{k}"), v)? {
                        out.push((name, v));
                    }
                }
            }
        }
    } else {
        let at_spot = at_spot_flag("high_at_spot", obj.get("high_at_spot"))?;
        if at_spot && obj.get("high").map_or(false, |v| !v.is_null()) {
            return Err(invalid("give high or high_at_spot, not both".to_string()));
        }
        for k in ARG_NAMES {
            if SHARED_ARG_NAMES.contains(&k) {
                continue;
            }
            if k == "high" && at_spot {
                out.push(("high".to_string(), "spot".to_string()));
                continue;
            }
            if let Some(v) = obj.get(k) {
                if let Some(v) = arg_value(k, v)? {
                    out.push((k.to_string(), v));
                }
            }
        }
    }
    for k in ["shape", "shape_usdc", "split_step_pct"].iter().copied().chain(SHARED_ARG_NAMES) {
        if let Some(v) = obj.get(k) {
            if let Some(v) = arg_value(k, v)? {
                out.push((k.to_string(), v));
            }
        }
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
    if b.get("rungs").map_or(false, |r| r.is_array()) {
        return rungs_text(b);
    }
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
            // Tracked ladders (#167) are existing ranges too; named only when there are any.
            let ladders = b
                .pointer("/reserve_check/existing_ladders")
                .and_then(|v| v.as_array())
                .map(|a| a.len())
                .filter(|n| *n > 0)
                .map(|n| format!(", {n} tracked ladder{} among them", if n == 1 { "" } else { "s" }))
                .unwrap_or_default();
            format!(
                "• Reserve: stable ${} at the bottom against the ${} reserve: {verdict} (this range takes at most ${}; the existing NOCK ranges hold ${} USDC there{ladders}). {}\n",
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
    // Fees (2.10.0): absent in a body from before the fee estimate.
    if let Some(fe) = b.get("fee_estimate").filter(|v| !v.is_null()) {
        let oor = b.pointer("/top/out_of_range_above_now").and_then(|v| v.as_bool()).unwrap_or(false);
        for l in fee_lines(fe, "• ", (s(b, "/top/price_usd").unwrap_or("?"), s(b, "/top/pct_of_spot").unwrap_or("?"), oor)) {
            out.push_str(&l);
            out.push('\n');
        }
    }
    let assumptions: Vec<String> = b
        .get("assumptions")
        .and_then(|a| a.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(|x| x.to_string())).collect())
        .unwrap_or_default();
    out.push_str(&format!("• Assumptions: {}\n", assumptions.join(" ")));
    if let Some(fa) = b.pointer("/fee_estimate/assumptions").and_then(|a| a.as_array()).filter(|_| b.pointer("/fee_estimate/status").and_then(|v| v.as_str()) == Some("measured")) {
        let fa: Vec<&str> = fa.iter().filter_map(|x| x.as_str()).collect();
        out.push_str(&format!("• Fee estimate assumptions: {}\n", fa.join(" ")));
    }
    out.push_str(&format!("• {}\n", or("/statement", "")));
    out.push_str(&snapshot_line(b));
    out
}

/// "1,836" from a JSON number. Pure.
fn grouped_v(v: &Value, ptr: &str) -> String {
    v.pointer(ptr).and_then(|x| x.as_f64().or_else(|| x.as_str().and_then(|s| s.parse().ok()))).map(grouped).unwrap_or_else(|| "?".to_string())
}

/// The fee estimate's lines (#166 rungs): the estimate, always beside the in-range share
/// of stored history and the replay upper bounds, then the measurement inside the range
/// and the daily spread. `top` is (price, pct, out of range above now). Pure.
fn fee_lines(fe: &Value, lead: &str, top: (&str, &str, bool)) -> Vec<String> {
    let mut out = Vec::new();
    if s(fe, "/status") != Some("measured") {
        out.push(format!("{lead}Fees while in range: not estimated: {}", s(fe, "/reason").unwrap_or("unknown")));
        return out;
    }
    let dil = match (s(fe, "/per_day_in_range_diluted_usd"), s(fe, "/dilution/share_of_active_pct")) {
        (Some(d), Some(sh)) => format!(", ${d} diluted (it would be {sh}% of the pool's active liquidity)"),
        _ => format!(", diluted unknown ({})", s(fe, "/dilution_reason").unwrap_or("not stated")),
    };
    let age = fe.pointer("/measurement_age_seconds").and_then(|v| v.as_i64()).map(|a| format!(", measured {} min ago", a / 60)).unwrap_or_default();
    out.push(format!(
        "{lead}Fees while in range (an estimate): ${}/day{dil}; ${} per $100 of its value at spot per day; measured on {} over {} days ({} intervals{age}) at ${} per 1e15 liquidity per day, the NOCK leg valued at the pool price ${}: the measuring range's average fee per unit of liquidity times this range's liquidity, not a measurement inside it",
        s(fe, "/per_day_in_range_usd").unwrap_or("?"),
        s(fe, "/per_100_usd_per_day_in_range_usd").unwrap_or("?"),
        s(fe, "/measured_label").unwrap_or("?"),
        s(fe, "/measured_days").unwrap_or("?"),
        grouped_v(fe, "/intervals"),
        s(fe, "/rate/usd_per_1e15_liquidity_day").unwrap_or("?"),
        s(fe, "/rate/valued_at_pool_price_usd").unwrap_or("?"),
    ));
    if top.2 {
        out.push(format!("{lead}It earns nothing until the price falls to ${} ({}%)", top.0, top.1));
    }
    match fe.get("in_range_share_stored_history").filter(|v| !v.is_null()) {
        Some(sh) => out.push(format!(
            "{lead}In range over stored history: {}% of the measured time ({} s of {} s); replayed over it, at most ${} from intervals wholly inside and at most ${} more from intervals at its edges ({} s), undiluted, no lower bound",
            or_v(sh, "/share_pct"),
            grouped_v(sh, "/seconds"),
            grouped_v(sh, "/of_seconds"),
            s(fe, "/stored_history_replay/inside_upper_usd").unwrap_or("?"),
            s(fe, "/stored_history_replay/edge_upper_usd").unwrap_or("?"),
            grouped_v(sh, "/edge_seconds"),
        )),
        None => out.push(format!("{lead}In range over stored history: not stated ({})", s(fe, "/in_range_share_reason").unwrap_or("unknown"))),
    }
    let band = match (fe.get("band_measured").filter(|v| !v.is_null()), s(fe, "/band_reason")) {
        (Some(b), _) if s(b, "/usd_per_day").is_some() => format!("${}/day over {} days (at most ${} in all)", or_v(b, "/usd_per_day"), or_v(b, "/days"), or_v(b, "/usd_total_upper")),
        (Some(b), why) => format!("at most ${} over {} s; {}", or_v(b, "/usd_total_upper"), grouped_v(b, "/seconds"), why.unwrap_or("")),
        (None, why) => format!("none ({})", why.unwrap_or("unknown")),
    };
    out.push(format!("{lead}Measured inside this range: {band}"));
    if let Some(sp) = fe.get("spread").filter(|v| !v.is_null()) {
        let days: Vec<String> = sp
            .get("days")
            .and_then(|d| d.as_array())
            .map(|d| {
                d.iter()
                    .map(|x| {
                        format!(
                            "{} ${} ({} s{})",
                            or_v(x, "/date"),
                            or_v(x, "/usd_per_day"),
                            grouped_v(x, "/seconds"),
                            if x.get("counted").and_then(|c| c.as_bool()) == Some(true) { "" } else { ", not counted" }
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        let counted = sp.get("days_counted").and_then(|v| v.as_u64()).unwrap_or(0);
        let spread = if counted == 0 {
            "no day was measured long enough".to_string()
        } else {
            format!(
                "min ${}, median ${}, max ${} per day",
                or_v(sp, "/min_usd_per_day"),
                or_v(sp, "/median_usd_per_day"),
                or_v(sp, "/max_usd_per_day")
            )
        };
        out.push(format!(
            "{lead}Daily spread ({counted} UTC days with at least {} s measured): {spread}; every day: {}",
            grouped_v(sp, "/day_min_seconds"),
            days.join(", ")
        ));
    }
    if fe.pointer("/coverage/whole").and_then(|v| v.as_bool()) == Some(false) {
        out.push(format!("{lead}{}", s(fe, "/coverage/note").unwrap_or("")));
    }
    out
}

/// A ladder of rungs as text. Pure.
fn rungs_text(b: &Value) -> String {
    let or = |ptr: &str, d: &str| s(b, ptr).unwrap_or(d).to_string();
    let mut out = String::from("🧮 Ladder Simulation (what-if, read-only)\n=========================================\n");
    let rungs = b.get("rungs").and_then(|r| r.as_array()).cloned().unwrap_or_default();
    let snapped = b.pointer("/pool/snapped").and_then(|v| v.as_bool()).unwrap_or(false);
    let pool_line = if snapped {
        format!(
            "snapped to {} (tick spacing {}{})",
            or("/pool/label", "?"),
            b.pointer("/pool/tick_spacing").map(|v| v.to_string()).unwrap_or_default(),
            if s(b, "/pool/spacing_source") == Some("caller") { ", given by you" } else { "" },
        )
    } else {
        "unsnapped: no position_id or tick_spacing was given".to_string()
    };
    out.push_str(&format!("• Ladder of {} rung{} at the pool price ${}, {pool_line}\n", rungs.len(), if rungs.len() == 1 { "" } else { "s" }, or("/pool/pool_price_usd", "?")));
    if let Some(sh) = b.pointer("/inputs/shape").filter(|v| !v.is_null()) {
        let parts: Vec<String> = sh
            .get("rungs")
            .and_then(|r| r.as_array())
            .map(|r| {
                r.iter()
                    .map(|x| match s(x, "/rung") {
                        Some("cap") => "cap: the tick band containing spot".to_string(),
                        _ => format!(
                            "{} {} from {} down to ${}",
                            or_v(x, "/rung"),
                            or_v(x, "/share"),
                            if s(x, "/high") == Some("spot") { "spot".to_string() } else { format!("${}", or_v(x, "/high")) },
                            or_v(x, "/low_usd")
                        ),
                    })
                    .collect()
            })
            .unwrap_or_default();
        out.push_str(&format!(
            "• Shape: your operator_profile.ladder_rungs ({}), sized to ${} USDC: {}\n",
            parts.join("; "),
            or_v(sh, "/total_usdc"),
            or_v(sh, "/total_rule")
        ));
    }
    if let Some(note) = s(b, "/pool/note").filter(|_| snapped) {
        out.push_str(&format!("• Pool: {note}\n"));
    }
    for r in &rungs {
        let role = s(r, "/rung").unwrap_or("?");
        let idx = r.get("index").map(|v| v.to_string()).unwrap_or_default();
        out.push_str(&format!(
            "• r{idx} {role}: ${} – ${} ({}% / {}%), ${} USDC and {} NOCK at spot (liquidity {}; top: {})\n",
            or_v(r, "/bounds/low_usd"),
            or_v(r, "/bounds/high_usd"),
            or_v(r, "/bounds/low_pct"),
            or_v(r, "/bounds/high_pct"),
            or_v(r, "/sizing/usdc_at_spot"),
            nock_txt(r, "/sizing/nock_needed_at_spot"),
            or_v(r, "/sizing/liquidity_human"),
            or_v(r, "/bounds/high_rule"),
        ));
        out.push_str(&format!(
            "  – Bottom: fully NOCK at ${}: {} NOCK held, {} bought, average ${}\n",
            or_v(r, "/bottom/price_usd"),
            nock_txt(r, "/bottom/nock_held"),
            nock_txt(r, "/bottom/nock_bought"),
            s(r, "/bottom/avg_buy_price_usd").unwrap_or("unknown"),
        ));
        let oor = r.pointer("/top/out_of_range_above_now").and_then(|v| v.as_bool()).unwrap_or(false);
        let top = (s(r, "/top/price_usd").unwrap_or("?"), s(r, "/top/pct_of_spot").unwrap_or("?"), oor);
        if let Some(fe) = r.get("fee_estimate") {
            for l in fee_lines(fe, "  – ", top) {
                out.push_str(&l);
                out.push('\n');
            }
        }
        if let Some(t) = s(r, "/round_trip/text") {
            out.push_str(&format!("  – Round trip: {t}\n"));
        }
        if let Some(p) = r.get("if_pulled_at_fill").filter(|v| !v.is_null()) {
            out.push_str(&format!(
                "  – If pulled at fill (at ${}): {} NOCK kept, {} bought at an average ${}. {}\n",
                or_v(p, "/at_usd"),
                nock_txt(p, "/nock_kept"),
                nock_txt(p, "/nock_bought"),
                s(p, "/avg_price_usd").unwrap_or("unknown"),
                or_v(p, "/note"),
            ));
        }
        if let Some(m) = s(r, "/max_usdc_with_the_others_as_given") {
            out.push_str(&format!("  – The reserve gate allows this rung at most ${m} with the other rungs as given\n"));
        }
    }
    let overlaps: Vec<String> = b
        .pointer("/combined/overlaps")
        .and_then(|o| o.as_array())
        .map(|o| o.iter().map(|x| format!("r{} and r{} at ${} – ${}", x["a"], x["b"], or_v(x, "/low_usd"), or_v(x, "/high_usd"))).collect())
        .unwrap_or_default();
    out.push_str(&format!(
        "• Overlaps: {}\n",
        if overlaps.is_empty() { "none".to_string() } else { format!("{} (separate positions; their liquidity adds)", overlaps.join("; ")) }
    ));
    out.push_str(&format!(
        "• Combined: ${} USDC and {} NOCK at spot; at the deepest bottom ${}: {} NOCK held, {} bought, average ${}; kept if the {} deep rung(s) are pulled at fill: {} NOCK, bought at an average ${}\n",
        or("/combined/total_usdc", "?"),
        nock_txt(b, "/combined/total_nock_needed"),
        or("/combined/deepest_bottom_usd", "?"),
        nock_txt(b, "/combined/nock_held_at_deepest_bottom"),
        nock_txt(b, "/combined/nock_bought_at_deepest_bottom"),
        or("/combined/avg_buy_price_usd", "unknown"),
        b.pointer("/combined/deep_rungs").map(|v| v.to_string()).unwrap_or_default(),
        nock_txt(b, "/combined/nock_kept_if_deep_rungs_pulled"),
        or("/combined/avg_kept_price_usd", "unknown"),
    ));
    out.push_str("• Price grid (NOCK bought by the ladder, USDC left in it; rungs in range and their estimated fees per day; stable balance and reserve headroom):\n");
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
        let moved = match num(&row, "/ladder_nock_acquired") {
            Some(n) if n.round() < 0.0 => format!("{} NOCK sold", grouped(-n)),
            Some(n) => format!("{} NOCK bought", grouped(n)),
            None => "unknown NOCK bought".to_string(),
        };
        let inr: Vec<String> = row.get("rungs_in_range").and_then(|v| v.as_array()).map(|a| a.iter().map(|i| format!("r{i}")).collect()).unwrap_or_default();
        let fee = match (s(&row, "/est_fee_usd_per_day"), s(&row, "/est_fee_usd_per_day_diluted")) {
            _ if inr.is_empty() => "no rung in range".to_string(),
            (Some(f), Some(d)) => format!("{} in range, fees ${f}/day (${d} diluted)", inr.join("+")),
            (Some(f), None) => format!("{} in range, fees ${f}/day", inr.join("+")),
            _ => format!("{} in range, fees not estimated", inr.join("+")),
        };
        out.push_str(&format!(
            "  {}% ${}: {moved}, ${} USDC left; {fee}{stable}{mark}\n",
            or_v(&row, "/pct_of_spot"),
            or_v(&row, "/price_usd"),
            or_v(&row, "/ladder_usdc"),
        ));
    }
    // Reserve, budget and floor-crossing: the flat form's lines, read at the shared gate.
    let rc = |p: &str| s(b, &format!("/reserve_check/{p}")).map(|x| x.to_string());
    match rc("status").as_deref() {
        Some("holds") | Some("breached") => {
            let margin = rc("margin_usdc").unwrap_or_default();
            let verdict = match margin.strip_prefix('-') {
                Some(abs) => format!("breached: below it by ${abs}"),
                None => format!("holds by ${margin}"),
            };
            out.push_str(&format!(
                "• Reserve (shared): stable ${} at the gate ${} ({}) against the ${} reserve: {verdict} (the rungs take at most ${} in all; the existing NOCK ranges hold ${} USDC there)\n",
                rc("stable_at_bottom_usdc").unwrap_or_default(),
                rc("gate_price_usd").unwrap_or_default(),
                rc("gate_price_rule").unwrap_or_default(),
                rc("cash_reserve_usdc").unwrap_or_default(),
                rc("max_usdc_for_this_range").unwrap_or_else(|| "unknown".to_string()),
                rc("existing_ranges_usdc_at_bottom").unwrap_or_default(),
            ));
        }
        Some(st) => out.push_str(&format!("• Reserve (shared): {st}: {}\n", rc("reason").unwrap_or_default())),
        None => out.push_str("• Reserve (shared): unknown\n"),
    }
    if let Some(bb) = b.pointer("/reserve_check/existing_ranges_below_bottom").filter(|v| !v.is_null()) {
        let h = s(bb, "/headroom_usdc").map(|h| format!(" (reserve {})", signed_usd(h))).unwrap_or_default();
        out.push_str(&format!("• Below the gate: at ${} ({}%) stable ${}{h}. {}\n", or_v(bb, "/price_usd"), or_v(bb, "/pct_of_spot"), or_v(bb, "/stable_usdc"), or_v(bb, "/note")));
    }
    let fc = |p: &str| s(b, &format!("/floor_check/{p}")).map(|x| x.to_string());
    let per: Vec<String> = b
        .pointer("/floor_check/nock_needed_by_rung")
        .and_then(|v| v.as_array())
        .map(|a| a.iter().map(|x| format!("r{} {}", x["index"], nock_txt(x, "/nock_needed"))).collect())
        .unwrap_or_default();
    match fc("status").as_deref() {
        Some(st @ ("fits" | "exceeds")) => {
            let cap = fc("max_usdc_within_budget").map(|m| format!("; ${m} in all is the most these rungs hold within it")).unwrap_or_default();
            let note = fc("budget_note").map(|n| format!(" ({n})")).unwrap_or_default();
            out.push_str(&format!(
                "• Floor budget (shared): needs {} ({}) of the {} NOCK that may go into LP{note}: {st}{cap}\n",
                nock_txt(b, "/floor_check/nock_needed"),
                per.join(", "),
                fc("lp_nock_budget").map(|v| v.parse::<f64>().map(grouped).unwrap_or(v)).unwrap_or_default(),
            ));
        }
        _ => out.push_str(&format!("• Floor budget (shared): unknown: {}\n", fc("reason").unwrap_or_default())),
    }
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
        Some(st) => out.push_str(&format!("• Floor-crossing price: {st}: {} (without the ladder {}, with it {})\n", fe("reason").unwrap_or_default(), crossing("before"), crossing("after"))),
    }
    if let Some(t) = b.get("tradeoff").filter(|v| !v.is_null()) {
        out.push_str(&format!("• Trade-off: {} {} {}\n", or_v(t, "/statement"), or_v(t, "/order"), or_v(t, "/columns_note")));
        let row_line = |row: &Value, head: String| -> String {
            let rungs: Vec<String> = row
                .get("per_rung")
                .and_then(|p| p.as_array())
                .map(|p| {
                    p.iter()
                        .map(|x| {
                            let fee = s(x, "/fee_per_day_in_range_usd").map(|f| format!("${f}/day in range")).unwrap_or_else(|| "no fee figure".to_string());
                            let share = s(x, "/in_range_share_pct").map(|v| format!("{v}%")).unwrap_or_else(|| "?".to_string());
                            let rep = match (s(x, "/replay_inside_upper_usd"), s(x, "/replay_edge_upper_usd")) {
                                (Some(i), Some(e)) => format!("replay ≤${i} inside, ≤${e} edges"),
                                _ => "no replay".to_string(),
                            };
                            format!("r{} ${}: {fee}, in range {share} of stored history, {rep}", x["index"], or_v(x, "/usdc"))
                        })
                        .collect()
                })
                .unwrap_or_default();
            let margin = s(row, "/reserve_margin_usdc").map(signed_usd).unwrap_or_else(|| "unknown".to_string());
            format!(
                "    {head}: {}; {} NOCK at the deepest bottom, average ${}; {} kept if deep rungs are pulled; reserve {} ({margin}); NOCK {} ({})\n",
                rungs.join("; "),
                nock_txt(row, "/nock_at_deepest_bottom"),
                s(row, "/avg_buy_price_usd").unwrap_or("unknown"),
                nock_txt(row, "/nock_kept_if_deep_rungs_pulled"),
                or_v(row, "/reserve_status"),
                nock_txt(row, "/nock_needed"),
                or_v(row, "/budget_status"),
            )
        };
        match t.get("split").filter(|v| !v.is_null()) {
            Some(sp) => {
                out.push_str(&format!("  Split ({}; step {}%, {}):\n", or_v(sp, "/varies"), sp["step_pct"], or_v(sp, "/step_note")));
                for row in sp.get("rows").and_then(|r| r.as_array()).cloned().unwrap_or_default() {
                    out.push_str(&row_line(&row, format!("r1 {}%", or_v(&row, "/first_rung_share_pct"))));
                }
            }
            None => out.push_str(&format!("  Split: none ({})\n", s(t, "/split_reason").unwrap_or("?"))),
        }
        for (key, label, col) in [("near_depth", "Near rung's bottom", "near_low_usd"), ("deep_top", "Deep rung's top", "deep_high_usd")] {
            match t.get(key).filter(|v| !v.is_null()) {
                Some(tb) => {
                    let trunc = if tb.get("truncated").and_then(|v| v.as_bool()) == Some(true) {
                        format!(", the first {} of {} usable ticks", tb["max_rows"], tb["ticks_between"])
                    } else {
                        String::new()
                    };
                    out.push_str(&format!("  {label} at each usable tick{trunc}:\n"));
                    for row in tb.get("rows").and_then(|r| r.as_array()).cloned().unwrap_or_default() {
                        out.push_str(&row_line(&row, format!("${}", or_v(&row, &format!("/{col}")))));
                    }
                }
                None => out.push_str(&format!("  {label}: none ({})\n", s(t, &format!("/{key}_reason")).unwrap_or("?"))),
            }
        }
    }
    let assumptions: Vec<String> = b
        .get("assumptions")
        .and_then(|a| a.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(|x| x.to_string())).collect())
        .unwrap_or_default();
    out.push_str(&format!("• Assumptions: {}\n", assumptions.join(" ")));
    let fee_assumptions: Vec<String> = rungs
        .first()
        .and_then(|r| r.pointer("/fee_estimate/assumptions"))
        .and_then(|a| a.as_array())
        .map(|a| a.iter().filter_map(|x| x.as_str().map(|x| x.to_string())).collect())
        .unwrap_or_default();
    if !fee_assumptions.is_empty() {
        out.push_str(&format!("• Fee estimate assumptions: {}\n", fee_assumptions.join(" ")));
    }
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
