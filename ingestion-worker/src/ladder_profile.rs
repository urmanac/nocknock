//! The ladder shape (issue #169, operator 2026-11-21): every deployed NOCK range as a bar on
//! a logarithmic USD-per-NOCK axis, its height the capital density `capital / ln(high/low)`,
//! so on that axis each bar's AREA is its capital and the stacked outline is the depth
//! profile ("so that we can approximate the bottle shape and judge whether it looks
//! bottle-y"). `derived_metrics.ladder_profile`, built from the assembled snapshot only.
//!
//! - A ladder rung's capital is what it holds as all USDC at its top
//!   (`vs_top.usdc_at_top`), which does not change with the price; any other range's is its
//!   value now. Each bar says which.
//! - A rung whose capital is not known is listed with `capital: null` and is drawn as an
//!   outline, never with a guessed height.
//! - A full-range position has no finite width: it is listed with its value, not drawn.
//! - The view window is the ladder zone plus a margin; a wider range is clipped there, with
//!   where it continues, and its density is always computed on its full width.
//!
//! Absent without a tracked ladder. Pure.
use crate::*;
use serde_json::{json, Value};

/// The margin around the ladder zone, as a factor on the log axis (5% each side).
pub(crate) const PROFILE_VIEW_MARGIN: f64 = 1.05;
pub(crate) const PROFILE_STATEMENT: &str = "Each bar spans a range's low to high price on a logarithmic axis; its height is capital / ln(high/low), so its area is its capital. A ladder rung's capital is its value as all USDC at its top, which does not change with the price; any other range's is its value now. A measurement of what is deployed, not a target.";

fn f(s: &str) -> Option<f64> {
    s.trim().parse::<f64>().ok().filter(|v| v.is_finite())
}

fn m2(v: f64) -> String {
    format!("{:.2}", (v * 100.0).round() / 100.0 + 0.0)
}

fn p8(v: f64) -> String {
    format!("{:.8}", v)
}

/// USD per NOCK at a tick of a USDC/NOCK pool, either order. None for another pair. Pure.
fn usd_at(p: &Position, tick: i32) -> Option<f64> {
    if is_nock_token(&p.token1) && p.token0.symbol == "USDC" {
        Some(tick_to_display_price_dec(tick, p.token0.decimals, p.token1.decimals))
    } else if is_nock_token(&p.token0) && p.token1.symbol == "USDC" {
        Some(1.0001f64.powi(tick) * 10f64.powi(p.token0.decimals - p.token1.decimals))
    } else {
        None
    }
}

fn short_name(p: &Position, rung: Option<&str>) -> String {
    match rung {
        Some(r) => r.to_string(),
        None if p.nft_id > 0 => format!("#{}", p.nft_id),
        None => "range".to_string(),
    }
}

/// Fill `derived_metrics.ladder_profile`. None without a tracked ladder. Pure.
pub(crate) fn ladder_profile_block(data: &SnapshotData) -> Option<Value> {
    if data.ladders.is_empty() {
        return None;
    }
    let mut bars: Vec<Value> = Vec::new();
    let mut full_range: Vec<Value> = Vec::new();
    let mut not_drawn: Vec<Value> = Vec::new();
    for p in &data.positions {
        if !(is_nock_token(&p.token0) || is_nock_token(&p.token1)) {
            continue;
        }
        let ladder = data.ladders.iter().find(|l| l.position_id == p.position_id);
        let rung = ladder.and_then(|l| l.rung.clone());
        let label = ladder.map(|l| l.label.clone()).unwrap_or_else(|| {
            data.derived_metrics
                .floor_breach
                .as_ref()
                .and_then(|fb| fb.positions.iter().find(|x| x.position_id == p.position_id).map(|x| x.label.clone()))
                .unwrap_or_else(|| position_label_of(p))
        });
        if p.full_range {
            full_range.push(json!({ "position_id": p.position_id, "label": label, "value_usd": p.amount_usd, "text": format!("{label} ${}: full range, no finite width, not drawn", p.amount_usd) }));
            continue;
        }
        let (Some(a), Some(b)) = (usd_at(p, p.lower_tick), usd_at(p, p.upper_tick)) else {
            not_drawn.push(json!({ "position_id": p.position_id, "label": label, "reason": "not a USDC/NOCK pair: its bounds are not USD prices" }));
            continue;
        };
        let (low, high) = if a < b { (a, b) } else { (b, a) };
        let (capital, basis) = match ladder {
            Some(l) => (l.vs_top.usdc_at_top.as_deref().and_then(f), "usdc_at_top"),
            None => (f(&p.amount_usd), "value_now"),
        };
        let width = (high / low).ln();
        let density = capital.filter(|_| width > 0.0).map(|c| c / width);
        let role = match (ladder.is_some(), rung.as_deref()) {
            (true, Some(r)) => r.to_string(),
            (true, None) => "ladder".to_string(),
            _ => "other".to_string(),
        };
        let name = short_name(p, rung.as_deref());
        bars.push(json!({
            "position_id": p.position_id,
            "label": label,
            "name": name,
            "role": role,
            "low_usd": p8(low),
            "high_usd": p8(high),
            "capital_usd": capital.map(m2),
            "capital_basis": basis,
            "density": density.map(|d| format!("{:.6}", d)),
            "status": if capital.is_some() { "measured" } else { "capital_unknown" },
            "text": match capital {
                Some(c) => format!("{name} ${}", m2(c)),
                None => format!("{name}: capital unknown"),
            },
        }));
    }
    let num = |b: &Value, k: &str| b[k].as_str().and_then(f);
    // The view: the ladder zone (its rungs, the ladder bottom, spot) plus a margin.
    let spot = data.ladders.iter().find_map(|l| l.spot_price_usd.as_deref().and_then(f));
    let bottom = data.derived_metrics.global_status.ladder_bottom_usd.as_deref().and_then(f);
    let ladder_bars: Vec<&Value> = bars.iter().filter(|b| b["role"] != "other").collect();
    let zone_lo = ladder_bars.iter().filter_map(|b| num(b, "low_usd")).chain(bottom).fold(f64::INFINITY, f64::min);
    let zone_hi = ladder_bars.iter().filter_map(|b| num(b, "high_usd")).chain(spot).fold(0.0, f64::max);
    let (view_lo, view_hi) = if zone_lo.is_finite() && zone_hi > zone_lo { (zone_lo / PROFILE_VIEW_MARGIN, zone_hi * PROFILE_VIEW_MARGIN) } else { (0.0, 0.0) };
    for b in bars.iter_mut() {
        let (lo, hi) = (num(b, "low_usd").unwrap_or(0.0), num(b, "high_usd").unwrap_or(0.0));
        b["clipped_low"] = json!(lo < view_lo);
        b["clipped_high"] = json!(hi > view_hi);
        b["clip_text"] = match (lo < view_lo, hi > view_hi) {
            (true, true) => json!(format!("continues from ${} to ${}", price4(lo), price4(hi))),
            (true, false) => json!(format!("← continues to ${}", price4(lo))),
            (false, true) => json!(format!("continues to ${} →", price4(hi))),
            _ => Value::Null,
        };
    }
    // The stacked depth: between consecutive bar edges, the sum of the densities covering it.
    let mut edges: Vec<f64> = bars.iter().flat_map(|b| [num(b, "low_usd"), num(b, "high_usd")]).flatten().collect();
    edges.sort_by(|a, b| a.partial_cmp(b).unwrap());
    edges.dedup_by(|a, b| (*a - *b).abs() <= 1e-12 * b.abs());
    let mut segments = Vec::new();
    for w in edges.windows(2) {
        let (lo, hi) = (w[0], w[1]);
        let mid = (lo * hi).sqrt();
        let covering: Vec<&Value> = bars.iter().filter(|b| num(b, "low_usd").unwrap_or(f64::INFINITY) <= mid && mid <= num(b, "high_usd").unwrap_or(0.0)).collect();
        if covering.is_empty() {
            continue;
        }
        let unknown = covering.iter().any(|b| b["density"].is_null());
        let total: f64 = covering.iter().filter_map(|b| num(b, "density")).sum();
        segments.push(json!({
            "low_usd": p8(lo),
            "high_usd": p8(hi),
            "density": format!("{:.6}", total),
            "includes_unknown": unknown,
            "position_ids": covering.iter().map(|b| b["position_id"].clone()).collect::<Vec<_>>(),
        }));
    }
    Some(json!({
        "axis": "usd_per_nock_log",
        "view_low_usd": (view_hi > 0.0).then(|| p8(view_lo)),
        "view_high_usd": (view_hi > 0.0).then(|| p8(view_hi)),
        "spot_usd": spot.map(p8),
        "ladder_bottom_usd": bottom.map(p8),
        "cash_reserve_usdc": data.derived_metrics.global_status.cash_reserve_usdc,
        "reserve_note": data.derived_metrics.global_status.cash_reserve_usdc.as_ref().map(|r| format!("${r} reserve kept for prices below the ladder bottom")),
        "bars": bars,
        "segments": segments,
        "full_range": full_range,
        "not_drawn": not_drawn,
        "statement": PROFILE_STATEMENT,
    }))
}

fn position_label_of(p: &Position) -> String {
    format!("{}/{} #{}", p.token0.symbol, p.token1.symbol, p.nft_id)
}
