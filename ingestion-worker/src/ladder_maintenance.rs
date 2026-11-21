//! Ladder maintenance (issue #169): what each tracked ladder rung's rule says this cycle,
//! gated by the operator's own restraint settings. A measurement, never a directive.
//!
//! The operator's words (2026-11-21): "I know how to keep it in my pants! But abstractly we
//! need a 'keep it in your pants' model at some level." So every suggestion here is gated by
//! `operator_profile.ladder_maintenance`, and none of its keys has a default: a rule whose
//! settings are not all set is off, and the block names the settings that turn it on.
//!
//! The rules follow the rung's role (decisions 2026-11-20):
//! - near: trades both ways for fees and scalps. After a rally leaves it out of range above
//!   (all USDC) past every gate, a move back to just below the pool price is stated, keeping
//!   its width in ticks and its USDC. Gates: `min_hours_out_of_range`, `cooldown_hours`,
//!   `hysteresis_pct` (the price must be that far above its top; once stated, the move
//!   stands until the price is back in its range) and `max_gas_payback_days` (the move's gas
//!   against the pool's measured in-range fees for the moved range).
//! - deep: accumulates near the ladder's bottom. After it converted at least
//!   `deep_pull_fill_fraction` of its USDC and the price recovered `deep_pull_recovery_pct`
//!   above the lowest price seen while it held NOCK, a pull (removing it) is stated, so a
//!   rebound does not sell the cheap NOCK back. Gates: those two, `cooldown_hours` and
//!   `hysteresis_pct` (once stated, the pull stands until the price falls that far below the
//!   recovery point, or makes a new low). A deep rung is never moved, never chased down and
//!   never pulled before both of its settings say so.
//! - deep, fully converted (the price at or below its bottom): reported, with no setting.
//!   NockNock never suggests a lower ladder ("we can wait it out").
//! - cap: out of range above means it sold its NOCK, by design. Stated; NockNock never
//!   suggests putting NOCK back into it (the ladder suggestion card alone offers a cap).
//!
//! History: the clocks are carried from the previous snapshot (in memory from cycle to cycle,
//! from the latest stored snapshot after a restart), the pattern closed.rs uses for its
//! first-seen time. No table: a lost database restarts them. A gap longer than the worker's
//! sync window plus the /health grace restarts the out-of-range clock and any standing
//! suggestion, so a gap can delay a suggestion and never makes one early. Pure: the snapshot,
//! the settings, the carried block, the previous snapshot's time and `now` are passed in.
use crate::*;
use serde_json::{json, Value};

pub(crate) const LM_KEY: &str = "operator_profile.ladder_maintenance";
pub(crate) const LM_STATEMENT: &str = "Measurements against your own restraint settings (operator_profile.ladder_maintenance, no defaults). NockNock builds no transaction and signs and sends nothing; whether and when is your decision.";
pub(crate) const LM_HISTORY: &str = "The clocks (out of range since, the last suggestion, the lowest price and the most converted a deep rung saw) are carried from the previous snapshot: in memory from cycle to cycle, and from the latest stored snapshot after a restart. A gap longer than the sync window plus 90 s restarts the out-of-range clock and any standing suggestion; the lowest price and the most converted are kept, with a note that the price was not watched in the gap. No table holds them: a lost database restarts them.";
pub(crate) const NO_LOWER_LADDER: &str = "NockNock does not suggest a lower ladder: the reserve kept below the ladder's bottom stays where it is, and the rung waits.";
pub(crate) const CAP_NO_REFILL: &str = "NockNock does not suggest putting NOCK back into it; a cap is offered only by the ladder suggestion card.";
pub(crate) const MOVE_TX_COUNT: f64 = 2.0;
pub(crate) const MOVE_GAS_BASIS: &str = "two transactions (remove the rung, then mint the moved range) at the worker's per-transaction estimate (150,000 gas plus the Base L1 buffer); a v4 mint can use more";
pub(crate) const PULL_GAS_BASIS: &str = "one transaction (remove the rung) at the worker's per-transaction estimate (150,000 gas plus the Base L1 buffer)";
pub(crate) const NEAR_KEYS: [&str; 4] = ["min_hours_out_of_range", "cooldown_hours", "hysteresis_pct", "max_gas_payback_days"];
pub(crate) const DEEP_KEYS: [&str; 4] = ["deep_pull_fill_fraction", "deep_pull_recovery_pct", "cooldown_hours", "hysteresis_pct"];
pub(crate) const ALL_KEYS: [&str; 6] = [
    "min_hours_out_of_range",
    "cooldown_hours",
    "hysteresis_pct",
    "max_gas_payback_days",
    "deep_pull_fill_fraction",
    "deep_pull_recovery_pct",
];

/// One setting as the profile states it. Pure.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Setting {
    Unset,
    Valid(f64),
    Invalid(String),
}

fn raw_setting(cfg: Option<&LadderMaintenanceConfig>, key: &str) -> Option<f64> {
    let c = cfg?;
    match key {
        "min_hours_out_of_range" => c.min_hours_out_of_range,
        "cooldown_hours" => c.cooldown_hours,
        "hysteresis_pct" => c.hysteresis_pct,
        "max_gas_payback_days" => c.max_gas_payback_days,
        "deep_pull_fill_fraction" => c.deep_pull_fill_fraction,
        "deep_pull_recovery_pct" => c.deep_pull_recovery_pct,
        _ => None,
    }
}

/// A setting validated: hours, cooldown and hysteresis are 0 or more; payback days and the
/// recovery are above 0; the fill fraction is above 0 and at most 1. Pure.
pub(crate) fn maintenance_setting(cfg: Option<&LadderMaintenanceConfig>, key: &str) -> Setting {
    let Some(v) = raw_setting(cfg, key) else { return Setting::Unset };
    let ok = v.is_finite()
        && match key {
            "min_hours_out_of_range" | "cooldown_hours" | "hysteresis_pct" => v >= 0.0,
            "deep_pull_fill_fraction" => v > 0.0 && v <= 1.0,
            _ => v > 0.0,
        };
    if ok {
        Setting::Valid(v)
    } else {
        let rule = match key {
            "min_hours_out_of_range" | "cooldown_hours" | "hysteresis_pct" => "valid values are 0 or more",
            "deep_pull_fill_fraction" => "valid values are above 0 and at most 1",
            _ => "valid values are above 0",
        };
        Setting::Invalid(format!("{LM_KEY}.{key} is {v}: {rule}"))
    }
}

fn val(cfg: Option<&LadderMaintenanceConfig>, key: &str) -> f64 {
    match maintenance_setting(cfg, key) {
        Setting::Valid(v) => v,
        _ => f64::NAN,
    }
}

/// One rule's state from its settings: on, off (names the settings that turn it on) or
/// invalid (names the values). Pure.
pub(crate) fn rule_state(cfg: Option<&LadderMaintenanceConfig>, rule: &str, keys: &[&str]) -> Value {
    let mut missing = Vec::new();
    let mut invalid = Vec::new();
    for k in keys {
        match maintenance_setting(cfg, k) {
            Setting::Unset => missing.push(format!("{LM_KEY}.{k}")),
            Setting::Invalid(r) => invalid.push(r),
            Setting::Valid(_) => {}
        }
    }
    let (status, text) = if !invalid.is_empty() {
        ("invalid", format!("The {rule} rule is off: {}.", invalid.join("; ")))
    } else if !missing.is_empty() {
        ("off", format!("The {rule} rule is off until you set {}.", join_and(&missing)))
    } else {
        ("on", format!("The {rule} rule is on, with your settings."))
    };
    json!({ "status": status, "needs": missing, "invalid": invalid, "text": text })
}

fn join_and(items: &[String]) -> String {
    match items.len() {
        0 => String::new(),
        1 => items[0].clone(),
        n => format!("{} and {}", items[..n - 1].join(", "), items[n - 1]),
    }
}

/// The clocks one rung carries from cycle to cycle.
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub(crate) struct RungClock {
    /// When it was first seen holding only USDC above its range, this stretch.
    pub(crate) above_since: Option<String>,
    /// When the standing suggestion for it was first stated.
    pub(crate) suggested_since: Option<String>,
    /// When its last suggestion cleared.
    pub(crate) last_cleared_at: Option<String>,
    /// A deep rung holding NOCK: the lowest pool price and the most converted seen.
    pub(crate) episode: Option<Episode>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub(crate) struct Episode {
    pub(crate) started_at: String,
    pub(crate) low_price_usd: String,
    pub(crate) low_at: String,
    /// The most of its USDC converted to NOCK, percent, `{:.2}`.
    pub(crate) peak_converted_pct: String,
    /// Set when the price was not watched for a while during the episode.
    pub(crate) gap_note: Option<String>,
}

/// What the cycle carries in: the previous `ladder_maintenance` block, the previous
/// snapshot's time and the longest gap that still counts as watched.
pub(crate) struct Carry<'a> {
    pub(crate) prev: Option<&'a Value>,
    pub(crate) prev_at: Option<DateTime<Utc>>,
    pub(crate) gap_limit_seconds: i64,
}

fn parse_t(s: Option<&str>) -> Option<DateTime<Utc>> {
    s.and_then(|t| DateTime::parse_from_rfc3339(t).ok()).map(|t| t.with_timezone(&Utc))
}

fn hours_between(a: DateTime<Utc>, b: DateTime<Utc>) -> f64 {
    (b - a).num_seconds() as f64 / 3600.0
}

fn f(s: &str) -> Option<f64> {
    s.trim().parse::<f64>().ok().filter(|v| v.is_finite())
}

fn fo(s: Option<&String>) -> Option<f64> {
    s.and_then(|v| f(v))
}

fn m2(v: f64) -> String {
    let r = (v * 100.0).round() / 100.0 + 0.0;
    format!("{:.2}", r)
}

fn m4(v: f64) -> String {
    let r = (v * 10000.0).round() / 10000.0 + 0.0;
    format!("{:.4}", r)
}

fn p8(v: f64) -> String {
    format!("{:.8}", v)
}

fn num_plain(v: f64) -> String {
    // Settings echoed as the operator wrote them: 24, 0.5, 2.5.
    let s = format!("{v}");
    if s.contains('e') { format!("{v:.6}") } else { s }
}

fn hrs(v: f64) -> String {
    format!("{:.1} h", v.max(0.0))
}

fn utc_text(t: DateTime<Utc>) -> String {
    t.format("%Y-%m-%d %H:%M UTC").to_string()
}

fn int_grouped(v: i64) -> String {
    let s = v.abs().to_string();
    let mut out = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            out.push(',');
        }
        out.push(c);
    }
    if v < 0 { format!("-{out}") } else { out }
}

fn nock_words(v: f64) -> String {
    int_grouped(v.round() as i64)
}

fn rung_name(rung: Option<&str>) -> &'static str {
    match rung {
        Some("cap") => "Cap",
        Some("near") => "Near rung",
        Some("deep") => "Deep rung",
        _ => "Ladder",
    }
}

fn short_label(r: &LadderReading, p: Option<&Position>) -> String {
    match p {
        Some(p) if p.nft_id > 0 => format!("{} #{}", rung_name(r.rung.as_deref()), p.nft_id),
        _ => format!("{} {}", rung_name(r.rung.as_deref()), r.label),
    }
}

/// USD per NOCK rises with the tick when NOCK is token0; falls with it when NOCK is token1.
fn nock_is_token1(p: &Position) -> bool {
    is_nock_token(&p.token1)
}

fn usd_at_tick(p: &Position, tick: i32) -> f64 {
    if nock_is_token1(p) {
        tick_to_display_price_dec(tick, p.token0.decimals, p.token1.decimals)
    } else {
        1.0001f64.powi(tick) * 10f64.powi(p.token0.decimals - p.token1.decimals)
    }
}

/// The moved near rung: its width in ticks kept, its top at the usable tick at or below the
/// pool price (so it holds only USDC when opened). None when it would not move. Pure.
pub(crate) fn moved_ticks(p: &Position, pool_tick: i32, spacing: i32) -> Option<(i32, i32)> {
    if spacing <= 0 {
        return None;
    }
    let width = p.upper_tick - p.lower_tick;
    let (lo, hi) = if nock_is_token1(p) {
        // Higher tick = lower USD: the top is the lower tick, at or above the pool tick.
        let top = pool_tick.div_euclid(spacing) * spacing + if pool_tick.rem_euclid(spacing) == 0 { 0 } else { spacing };
        (top, top + width)
    } else {
        let top = pool_tick.div_euclid(spacing) * spacing;
        (top - width, top)
    };
    (lo != p.lower_tick || hi != p.upper_tick).then_some((lo, hi))
}

/// Raw liquidity of a range holding only `usdc` (human units) above it in USD. Pure.
pub(crate) fn liquidity_for_usdc(p: &Position, lo: i32, hi: i32, usdc: f64) -> Option<f64> {
    let (sa, sb) = (tick_sqrt_price(lo), tick_sqrt_price(hi));
    if !(sb > sa) || !(usdc > 0.0) {
        return None;
    }
    let l = if nock_is_token1(p) {
        let raw = usdc * 10f64.powi(p.token0.decimals);
        raw / (1.0 / sa - 1.0 / sb)
    } else {
        let raw = usdc * 10f64.powi(p.token1.decimals);
        raw / (sb - sa)
    };
    (l.is_finite() && l > 0.0).then_some(l)
}

/// The per-transaction gas estimate in USD this cycle (contract_fees, 150,000 gas plus the
/// L1 buffer). None when no row states it. Pure.
pub(crate) fn gas_per_tx_usd(data: &SnapshotData) -> Option<f64> {
    data.contract_fees.iter().find_map(|c| f(&c.gas_break_even_usd)).filter(|g| *g > 0.0)
}

/// The pool's measured in-range fees per raw unit of liquidity per day in USD, or why not.
fn fee_rate(data: &SnapshotData, pool_id: &str) -> std::result::Result<(f64, String), String> {
    let Some(fy) = data.derived_metrics.fee_yield.as_ref() else {
        return Err("no fee measurement is in this snapshot".to_string());
    };
    let Some(pool) = fy.pools.iter().find(|p| p.pool_id.eq_ignore_ascii_case(pool_id)) else {
        return Err("no range of ours measures fees in this pool".to_string());
    };
    match pool.usd_per_liquidity_day.as_deref().and_then(f).filter(|v| *v > 0.0) {
        Some(v) => Ok((v, pool.label.clone())),
        None => Err(pool.reason.clone().unwrap_or_else(|| "the pool's fees per unit of liquidity are not measured".to_string())),
    }
}

fn gate(name: &str, setting: String, measured: String, passes: Option<bool>, text: String) -> Value {
    json!({ "gate": name, "setting": setting, "measured": measured, "passes": passes, "text": text })
}

/// Fill `derived_metrics.ladder_maintenance` on an assembled snapshot. None without a
/// tracked ladder. Pure.
pub(crate) fn ladder_maintenance_block(
    data: &SnapshotData,
    cfg: Option<&LadderMaintenanceConfig>,
    carry: &Carry,
    now: DateTime<Utc>,
) -> Option<Value> {
    if data.ladders.is_empty() {
        return None;
    }
    let continuous = carry.prev.is_some()
        && carry.prev_at.map_or(false, |t| (now - t).num_seconds() <= carry.gap_limit_seconds && now >= t);
    let near_rule = rule_state(cfg, "near-rung move", &NEAR_KEYS);
    let deep_rule = rule_state(cfg, "deep-rung pull", &DEEP_KEYS);
    let mut settings = serde_json::Map::new();
    for k in ALL_KEYS {
        settings.insert(k.to_string(), raw_setting(cfg, k).map(|v| json!(num_plain(v))).unwrap_or(Value::Null));
    }
    let rungs: Vec<Value> = data
        .ladders
        .iter()
        .map(|r| {
            let prev_clock: Option<RungClock> = carry
                .prev
                .and_then(|p| p.get("rungs"))
                .and_then(|v| v.as_array())
                .and_then(|a| a.iter().find(|e| e.get("position_id").and_then(|x| x.as_str()) == Some(r.position_id.as_str())))
                .and_then(|e| e.get("clock"))
                .and_then(|c| serde_json::from_value(c.clone()).ok());
            rung_entry(data, r, cfg, &near_rule, &deep_rule, prev_clock, continuous, carry.prev_at, now)
        })
        .collect();
    Some(json!({
        "settings_key": LM_KEY,
        "settings": settings,
        "rules": { "near_move": near_rule, "deep_pull": deep_rule, "deep_bottom_report": { "status": "on", "needs": [], "invalid": [], "text": "A deep rung fully converted at its bottom is reported; there is no setting." } },
        "history": LM_HISTORY,
        "watched_since_previous": continuous,
        "gap_limit_seconds": carry.gap_limit_seconds,
        "rungs": rungs,
        "statement": LM_STATEMENT,
    }))
}

#[allow(clippy::too_many_arguments)]
fn rung_entry(
    data: &SnapshotData,
    r: &LadderReading,
    cfg: Option<&LadderMaintenanceConfig>,
    near_rule: &Value,
    deep_rule: &Value,
    prev: Option<RungClock>,
    continuous: bool,
    prev_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
) -> Value {
    let pos = data.positions.iter().find(|p| p.position_id == r.position_id);
    let label = short_label(r, pos);
    let rung = r.rung.as_deref();
    let state = r.state.as_str();
    let now_s = now.to_rfc3339();
    let prev = prev.unwrap_or_default();
    // A gap restarts the out-of-range clock and any standing suggestion; the rest is kept.
    let mut clock = RungClock {
        above_since: if continuous { prev.above_since.clone() } else { None },
        suggested_since: if continuous { prev.suggested_since.clone() } else { None },
        last_cleared_at: prev.last_cleared_at.clone(),
        episode: prev.episode.clone(),
    };
    if state == "holds_only_usdc" {
        if clock.above_since.is_none() {
            clock.above_since = Some(now_s.clone());
        }
    } else {
        clock.above_since = None;
    }
    let pool_price = fo(r.pool_price_usd.as_ref());
    let top = fo(r.high_price_usd.as_ref());
    let bottom = fo(r.low_price_usd.as_ref());
    let usdc_now = f(&r.usdc_now).unwrap_or(0.0);
    let nock_now = f(&r.nock_now).unwrap_or(0.0);
    let fees = f(&r.fees_usd).unwrap_or(0.0);
    let deposited_at = parse_t(r.since_deposit.deposited_at.as_deref());

    // The deep rung's episode: open while it holds NOCK, closed when it is all USDC again.
    if rung == Some("deep") {
        let holds_nock = matches!(state, "in_range" | "holds_only_nock");
        if holds_nock {
            let converted = if state == "holds_only_nock" { 100.0 } else { fo(r.vs_top.converted_pct.as_ref()).unwrap_or(0.0) };
            let px = pool_price.unwrap_or(f64::NAN);
            let gap_note = (!continuous && clock.episode.is_some()).then(|| {
                format!(
                    "the price was not watched from {} to {}: the low may have been lower and more may have converted",
                    prev_at.map(utc_text).unwrap_or_else(|| "an unknown time".to_string()),
                    utc_text(now)
                )
            });
            clock.episode = Some(match clock.episode.take() {
                Some(mut e) => {
                    let low = f(&e.low_price_usd).unwrap_or(f64::INFINITY);
                    if px.is_finite() && px < low {
                        e.low_price_usd = p8(px);
                        e.low_at = now_s.clone();
                    }
                    let peak = f(&e.peak_converted_pct).unwrap_or(0.0);
                    if converted > peak {
                        e.peak_converted_pct = format!("{:.2}", converted);
                    }
                    if gap_note.is_some() {
                        e.gap_note = gap_note;
                    }
                    e
                }
                None => Episode {
                    started_at: now_s.clone(),
                    low_price_usd: if px.is_finite() { p8(px) } else { String::new() },
                    low_at: now_s.clone(),
                    peak_converted_pct: format!("{:.2}", converted),
                    gap_note: None,
                },
            });
        } else if state != "pool_stale" {
            clock.episode = None;
        }
    } else {
        clock.episode = None;
    }

    let mut out = json!({
        "position_id": r.position_id,
        "label": r.label,
        "short_label": label,
        "rung": r.rung,
        "state": r.state,
        "pool_price_usd": r.pool_price_usd,
        "top_usd": r.high_price_usd,
        "bottom_usd": r.low_price_usd,
        "usdc_now": r.usdc_now,
        "nock_now": r.nock_now,
        "fees_earned_usd": r.fees_usd,
        "tick_lower": pos.map(|p| p.lower_tick),
        "tick_upper": pos.map(|p| p.upper_tick),
        "rule": Value::Null,
        "status": "not_applicable",
        "gates": [],
        "move": Value::Null,
        "pull": Value::Null,
        "event": Value::Null,
        "text": "",
    });
    let mut suggested = false;
    match rung {
        Some("near") => {
            out["rule"] = json!("near_move");
            match state {
                "holds_only_usdc" => {
                    let (status, text, gates, mv, event, is_suggested) =
                        near_move(data, pos, cfg, near_rule, &clock, pool_price, top, usdc_now, fees, deposited_at, now, &label);
                    out["status"] = json!(status);
                    out["text"] = json!(text);
                    out["gates"] = json!(gates);
                    out["move"] = mv;
                    out["event"] = event;
                    suggested = is_suggested;
                }
                "in_range" => {
                    out["status"] = json!("trading");
                    out["text"] = json!(format!(
                        "{label} is in range: it trades both ways, buying NOCK as the price dips and selling it back as it rises (the scalp), and earns fees. Nothing to do."
                    ));
                }
                "holds_only_nock" => {
                    out["status"] = json!("holds_nock");
                    out["text"] = json!(format!(
                        "{label} holds only NOCK ({} NOCK): it bought on the way down to its bottom ${}. A rebound into it sells that NOCK back for USDC (the scalp). Nothing to do.",
                        nock_words(nock_now),
                        bottom.map(price4).unwrap_or_else(|| "?".to_string())
                    ));
                }
                _ => {
                    out["text"] = json!(format!("{label}: {}. No rule applies.", r.state_text));
                }
            }
        }
        Some("deep") => {
            out["rule"] = json!("deep_pull");
            let (status, text, gates, pull, event, is_suggested) =
                deep_rung(data, r, pos, cfg, deep_rule, &clock, pool_price, top, bottom, usdc_now, nock_now, fees, deposited_at, now, &label);
            out["status"] = json!(status);
            out["text"] = json!(text);
            out["gates"] = json!(gates);
            out["pull"] = pull;
            out["event"] = event;
            suggested = is_suggested;
        }
        Some("cap") => {
            out["rule"] = json!("cap_statement");
            match state {
                "holds_only_usdc" => {
                    out["status"] = json!("sold");
                    let text = format!(
                        "{label} is out of range above: it sold its NOCK, as a cap does when the price rises through its band, and holds ${} USDC (${} of uncollected fees). {CAP_NO_REFILL}",
                        m2(usdc_now),
                        m2(fees)
                    );
                    out["event"] = json!({
                        "kind": "ladder_cap_sold",
                        "tier": "quiet",
                        "title": "LADDER: CAP SOLD ITS NOCK",
                        "what_changed": format!("The price rose above {label}'s band, so it sold its NOCK."),
                        "text": text,
                        "numbers": { "usdc_now": m2(usdc_now), "nock_now": m4(nock_now), "fees_earned_usd": m2(fees), "top_usd": r.high_price_usd, "pool_price_usd": r.pool_price_usd },
                        "links": [],
                        "would_sign": Value::Null,
                    });
                    out["text"] = json!(text);
                }
                "in_range" => {
                    out["status"] = json!("in_range");
                    out["text"] = json!(format!("{label} is in range: it holds NOCK and USDC and earns fees. Nothing to do."));
                }
                "holds_only_nock" => {
                    out["status"] = json!("holds_nock");
                    out["text"] = json!(format!("{label} holds only NOCK ({} NOCK): the price is below its band. Nothing to do.", nock_words(nock_now)));
                }
                _ => {
                    out["text"] = json!(format!("{label}: {}. No rule applies.", r.state_text));
                }
            }
        }
        _ => {
            out["text"] = json!(format!(
                "{}: its rung is not set (cap, near or deep), so no maintenance rule applies to it.",
                r.label
            ));
        }
    }
    // The standing suggestion's clock: stamped on entry, cleared (and the cooldown started)
    // when it ends.
    if suggested {
        if clock.suggested_since.is_none() {
            clock.suggested_since = Some(now_s.clone());
        }
    } else if prev.suggested_since.is_some() {
        clock.suggested_since = None;
        clock.last_cleared_at = Some(now_s.clone());
    }
    out["clock"] = serde_json::to_value(&clock).unwrap_or(Value::Null);
    if let Some(since) = clock.suggested_since.as_ref() {
        if out["event"].is_object() {
            out["event"]["since"] = json!(since);
        }
    }
    out
}

/// The cooldown gate: hours since the later of the rung's deposit (the last action) and the
/// last suggestion clearing. Unknown without a deposit time. Pure.
fn cooldown_gate(cfg: Option<&LadderMaintenanceConfig>, clock: &RungClock, deposited_at: Option<DateTime<Utc>>, now: DateTime<Utc>) -> Value {
    let setting = val(cfg, "cooldown_hours");
    let cleared = parse_t(clock.last_cleared_at.as_deref());
    let Some(dep) = deposited_at else {
        return gate(
            "cooldown_hours",
            num_plain(setting),
            "unknown".to_string(),
            None,
            "cooldown: unknown, the rung's deposit time was not read (no verified deposit receipt), so the time since the last action cannot be measured".to_string(),
        );
    };
    let (from, what) = match cleared {
        Some(c) if c > dep => (c, "since the last suggestion cleared"),
        _ => (dep, "since its deposit"),
    };
    let h = hours_between(from, now);
    let pass = h >= setting;
    gate("cooldown_hours", num_plain(setting), format!("{:.1}", h.max(0.0)), Some(pass), format!("cooldown: {} {what} of your {} h", hrs(h), num_plain(setting)))
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn near_move(
    data: &SnapshotData,
    pos: Option<&Position>,
    cfg: Option<&LadderMaintenanceConfig>,
    rule: &Value,
    clock: &RungClock,
    pool_price: Option<f64>,
    top: Option<f64>,
    usdc_now: f64,
    fees: f64,
    deposited_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    label: &str,
) -> (&'static str, String, Vec<Value>, Value, Value, bool) {
    let above_since = parse_t(clock.above_since.as_deref()).unwrap_or(now);
    let hours_above = hours_between(above_since, now);
    let (Some(px), Some(top), Some(p)) = (pool_price, top, pos) else {
        return ("unknown", format!("{label} is out of range above, but its pool price or range is not known this cycle: no move is stated."), vec![], Value::Null, Value::Null, false);
    };
    let pct_above = (px / top - 1.0) * 100.0;
    if !(pct_above > 0.0) {
        return ("unknown", format!("{label} reads as holding only USDC, but its pool price ${} is not above its top ${}: no move is stated.", price4(px), price4(top)), vec![], Value::Null, Value::Null, false);
    }
    let head = format!(
        "{label} has been out of range above since {} ({}): it holds ${} USDC and 0 NOCK and earns nothing until NOCK falls to its top ${}. The pool price ${} is {:.2}% above that top.",
        utc_text(above_since),
        hrs(hours_above),
        m2(usdc_now),
        price4(top),
        price4(px),
        pct_above
    );
    let status = rule["status"].as_str().unwrap_or("off");
    if status != "on" {
        return (if status == "invalid" { "invalid" } else { "off" }, format!("{head} {}", rule["text"].as_str().unwrap_or("")), vec![], Value::Null, Value::Null, false);
    }
    let pool_tick = data.pool_mechanics.iter().find(|m| m.pool_id.eq_ignore_ascii_case(&p.pool_id)).and_then(|m| m.tick_current);
    let (Some(spacing), Some(pool_tick)) = (p.tick_spacing, pool_tick) else {
        return ("unknown", format!("{head} The pool's tick or tick spacing was not read this cycle, so no move is worked out."), vec![], Value::Null, Value::Null, false);
    };
    let Some((lo, hi)) = moved_ticks(p, pool_tick, spacing) else {
        return ("waiting", format!("{head} The price is within one tick spacing of its top, so a move would not change the range."), vec![], Value::Null, Value::Null, false);
    };
    let (new_low, new_top) = if nock_is_token1(p) { (usd_at_tick(p, hi), usd_at_tick(p, lo)) } else { (usd_at_tick(p, lo), usd_at_tick(p, hi)) };
    let width = p.upper_tick - p.lower_tick;
    let liq = liquidity_for_usdc(p, lo, hi, usdc_now);
    let gas_tx = gas_per_tx_usd(data);
    let move_gas = gas_tx.map(|g| g * MOVE_TX_COUNT);
    let rate = fee_rate(data, &p.pool_id);
    let fees_day = match (&rate, liq) {
        (Ok((rate, _)), Some(l)) => Some(rate * l),
        _ => None,
    };
    let payback = match (move_gas, fees_day) {
        (Some(g), Some(fd)) if fd > 0.0 => Some(g / fd),
        _ => None,
    };
    let standing = clock.suggested_since.is_some();
    let min_h = val(cfg, "min_hours_out_of_range");
    let hyst = val(cfg, "hysteresis_pct");
    let max_pb = val(cfg, "max_gas_payback_days");
    let mut gates = vec![
        gate(
            "min_hours_out_of_range",
            num_plain(min_h),
            format!("{:.1}", hours_above),
            Some(hours_above >= min_h),
            format!("out of range above: {} of your {} h", hrs(hours_above), num_plain(min_h)),
        ),
        gate(
            "hysteresis_pct",
            num_plain(hyst),
            format!("{:.2}", pct_above),
            Some(pct_above >= hyst || standing),
            if standing {
                format!("hysteresis: the move stands until the price is back in the range ({:.2}% above its top now; it was stated at {}% or more)", pct_above, num_plain(hyst))
            } else {
                format!("hysteresis: {:.2}% above its top, your {}%", pct_above, num_plain(hyst))
            },
        ),
    ];
    let cool = cooldown_gate(cfg, clock, deposited_at, now);
    let cool_pass = standing || cool["passes"].as_bool() == Some(true);
    let mut cool = cool;
    if standing {
        cool["passes"] = json!(true);
        cool["text"] = json!(format!("{} (it is checked only before a move is first stated)", cool["text"].as_str().unwrap_or("")));
    }
    gates.push(cool);
    let gas_text = match (move_gas, fees_day, payback, &rate) {
        (Some(g), Some(fd), Some(pb), _) => format!(
            "gas against fees: ≈${} of gas ({}) against ≈${} a day of fees while in range at the pool's measured rate, repaid in {:.4} days in range, your {}",
            m4(g),
            "two transactions, an estimate",
            m4(fd),
            pb,
            num_plain(max_pb)
        ),
        (None, _, _, _) => "gas against fees: unknown, this cycle's gas price is not in the snapshot".to_string(),
        (_, _, _, Err(why)) => format!("gas against fees: unknown, {why}"),
        _ => "gas against fees: unknown, the moved range's liquidity could not be worked out".to_string(),
    };
    gates.push(gate(
        "max_gas_payback_days",
        num_plain(max_pb),
        payback.map(|v| format!("{:.4}", v)).unwrap_or_else(|| "unknown".to_string()),
        payback.map(|pb| pb <= max_pb),
        gas_text,
    ));
    let all_pass = gates.iter().all(|g| g["passes"].as_bool() == Some(true)) && cool_pass;
    let tx_link = position_page_url(p);
    let new_link = new_position_url(p, lo, hi, None).ok().map(|(u, _)| u);
    let opens_below = (new_top / px - 1.0) * 100.0;
    let mv = json!({
        "new_tick_lower": lo,
        "new_tick_upper": hi,
        "width_ticks": width,
        "new_top_usd": p8(new_top),
        "new_bottom_usd": p8(new_low),
        "opens_pct_of_price": format!("{:.2}", opens_below),
        "usdc": m2(usdc_now),
        "nock": "0",
        "liquidity_raw": liq.map(|l| format!("{:.0}", l)),
        "gas_usd": move_gas.map(m4),
        "gas_basis": MOVE_GAS_BASIS,
        "fees_per_day_in_range_usd": fees_day.map(m4),
        "fee_rate_from": rate.as_ref().ok().map(|(_, l)| l.clone()),
        "fee_rate_reason": rate.as_ref().err().cloned(),
        "payback_days_in_range": payback.map(|v| format!("{:.4}", v)),
        "fees_earned_usd": m2(fees),
        "remove_url": tx_link,
        "new_position_url": new_link,
    });
    let what = format!(
        "A move to just below the price, keeping its width ({} ticks), would be ticks {lo}..{hi}, ${} to ${}, with the same ${} USDC and 0 NOCK; it would open {:.2}% below the price, so it earns fees only once NOCK dips into it. It has earned ${} in fees so far.",
        int_grouped(width as i64),
        price4(new_low),
        price4(new_top),
        m2(usdc_now),
        -opens_below,
        m2(fees)
    );
    let gate_lines: Vec<String> = gates.iter().filter_map(|g| g["text"].as_str().map(str::to_string)).collect();
    if all_pass {
        let text = format!("{head} {what} {}. Whether and when is your decision.", capitalize(&gate_lines.join("; ")));
        let event = json!({
            "kind": "ladder_near_move",
            "tier": "top",
            "title": "LADDER: MOVE THE NEAR RUNG",
            "what_changed": format!("{label} has been out of range above past every gate you set."),
            "text": text,
            "numbers": {
                "hours_out_of_range": format!("{:.1}", hours_above),
                "pool_price_usd": p8(px),
                "top_usd": p8(top),
                "pct_above_top": format!("{:.2}", pct_above),
                "usdc": m2(usdc_now),
                "new_tick_lower": lo.to_string(),
                "new_tick_upper": hi.to_string(),
                "new_top_usd": p8(new_top),
                "new_bottom_usd": p8(new_low),
                "width_ticks": width.to_string(),
                "gas_usd": move_gas.map(m4),
                "fees_per_day_in_range_usd": fees_day.map(m4),
                "payback_days_in_range": payback.map(|v| format!("{:.4}", v)),
                "fees_earned_usd": m2(fees),
            },
            "links": links(&[("This rung in the Uniswap app (remove)", tx_link.clone()), ("The moved range in the Uniswap app (new position, no amount prefilled)", new_link.clone())]),
            "would_sign": format!(
                "Two transactions in your wallet: remove all of {label}'s liquidity (its fees come with it), then mint ticks {lo}..{hi} in the same pool with ${} USDC and 0 NOCK. NockNock builds neither and signs nothing; the new position is found on-chain after it lands.",
                m2(usdc_now)
            ),
        });
        ("suggested", text, gates, mv, event, true)
    } else {
        let failing: Vec<String> = gates.iter().filter(|g| g["passes"].as_bool() != Some(true)).filter_map(|g| g["text"].as_str().map(str::to_string)).collect();
        let text = format!("{head} {what} Not stated yet: {}.", failing.join("; "));
        ("waiting", text, gates, mv, Value::Null, false)
    }
}

#[allow(clippy::too_many_arguments, clippy::type_complexity)]
fn deep_rung(
    data: &SnapshotData,
    r: &LadderReading,
    pos: Option<&Position>,
    cfg: Option<&LadderMaintenanceConfig>,
    rule: &Value,
    clock: &RungClock,
    pool_price: Option<f64>,
    top: Option<f64>,
    bottom: Option<f64>,
    usdc_now: f64,
    nock_now: f64,
    fees: f64,
    deposited_at: Option<DateTime<Utc>>,
    now: DateTime<Utc>,
    label: &str,
) -> (&'static str, String, Vec<Value>, Value, Value, bool) {
    let state = r.state.as_str();
    let price_words = |v: Option<f64>| v.map(price4).unwrap_or_else(|| "?".to_string());
    let avg = r
        .since_deposit
        .avg_buy_price_usd
        .as_deref()
        .and_then(f)
        .or_else(|| fo(r.vs_top.curve_avg_price_usd.as_ref()))
        .or_else(|| match (fo(r.vs_top.usdc_at_top.as_ref()), fo(r.vs_top.nock_at_bottom.as_ref())) {
            (Some(u), Some(n)) if state == "holds_only_nock" && n > 0.0 => Some(u / n),
            _ => None,
        });
    let avg_words = avg.map(|a| format!(", an average of ${}", price4(a))).unwrap_or_default();
    match state {
        "holds_only_usdc" => {
            let text = format!(
                "{label} holds only USDC (${}): the price is above it, so it waits for NOCK to fall to its top ${}. It is never moved up or chased; nothing to do.",
                m2(usdc_now),
                price_words(top)
            );
            return ("waiting_for_dip", text, vec![], Value::Null, Value::Null, false);
        }
        "in_range" | "holds_only_nock" => {}
        _ => return ("not_applicable", format!("{label}: {}. No rule applies.", r.state_text), vec![], Value::Null, Value::Null, false),
    }
    let ep = clock.episode.clone().unwrap_or_default();
    let low = f(&ep.low_price_usd);
    let low_at = parse_t(Some(ep.low_at.as_str()));
    let peak = f(&ep.peak_converted_pct).unwrap_or(0.0);
    let recovery_now = match (pool_price, low) {
        (Some(px), Some(l)) if l > 0.0 => Some((px / l - 1.0) * 100.0),
        _ => None,
    };
    let bottom_report = || -> (String, Value) {
        let text = format!(
            "{label} is fully converted: the price ${} is at or below its bottom ${}, and it holds {} NOCK bought between ${} and ${}{avg_words}, and no USDC. {NO_LOWER_LADDER}",
            price_words(pool_price),
            price_words(bottom),
            nock_words(nock_now),
            price_words(top),
            price_words(bottom)
        );
        let event = json!({
            "kind": "ladder_deep_converted",
            "tier": "top",
            "title": "LADDER: DEEP RUNG FULLY CONVERTED",
            "what_changed": format!("The price reached {label}'s bottom: all of its USDC is NOCK now."),
            "text": text,
            "numbers": { "nock_now": m4(nock_now), "usdc_now": m2(usdc_now), "avg_buy_price_usd": avg.map(p8), "bottom_usd": bottom.map(p8), "top_usd": top.map(p8), "pool_price_usd": pool_price.map(p8) },
            "links": [],
            "would_sign": Value::Null,
        });
        (text, event)
    };
    let status = rule["status"].as_str().unwrap_or("off");
    let held = format!(
        "{label} holds {} NOCK + ${} USDC{avg_words}; it converted up to {:.2}% of its USDC and the lowest price seen while it held NOCK is ${}{}.",
        nock_words(nock_now),
        m2(usdc_now),
        peak,
        price_words(low),
        low_at.map(|t| format!(" ({})", utc_text(t))).unwrap_or_default()
    );
    let gap = ep.gap_note.as_ref().map(|g| format!(" Note: {g}.")).unwrap_or_default();
    if status != "on" {
        let rule_text = rule["text"].as_str().unwrap_or("");
        if state == "holds_only_nock" {
            let (text, event) = bottom_report();
            return ("fully_converted", format!("{text} {rule_text}"), vec![], Value::Null, event, false);
        }
        return (if status == "invalid" { "invalid" } else { "off" }, format!("{held}{gap} {rule_text}"), vec![], Value::Null, Value::Null, false);
    }
    let fill = val(cfg, "deep_pull_fill_fraction");
    let rec = val(cfg, "deep_pull_recovery_pct");
    let hyst = val(cfg, "hysteresis_pct");
    // A new low this cycle ends a standing pull: the recovery is measured from it again.
    let new_low_now = low_at == Some(now) && ep.started_at != ep.low_at;
    let standing = clock.suggested_since.is_some() && !new_low_now;
    let trigger = low.map(|l| l * (1.0 + rec / 100.0));
    let hold_level = trigger.map(|t| t * (1.0 - hyst / 100.0));
    let rec_pass = match (pool_price, trigger, hold_level) {
        (Some(px), Some(t), Some(h)) => Some(if standing { px >= h } else { px >= t }),
        _ => None,
    };
    let mut gates = vec![
        gate(
            "deep_pull_fill_fraction",
            num_plain(fill),
            format!("{:.4}", peak / 100.0),
            Some(peak / 100.0 >= fill - 1e-12),
            format!("converted: up to {:.2}% of its USDC, your {}%", peak, num_plain(fill * 100.0)),
        ),
        gate(
            "deep_pull_recovery_pct",
            num_plain(rec),
            recovery_now.map(|v| format!("{:.2}", v)).unwrap_or_else(|| "unknown".to_string()),
            rec_pass,
            match recovery_now {
                Some(v) => format!("recovery: the price is {:.2}% above the lowest seen, your {}%", v, num_plain(rec)),
                None => "recovery: unknown, the lowest price or the pool price is not known".to_string(),
            },
        ),
        gate(
            "hysteresis_pct",
            num_plain(hyst),
            hold_level.map(p8).unwrap_or_else(|| "unknown".to_string()),
            Some(true),
            match hold_level {
                Some(h) if standing => format!("hysteresis: the pull stands until the price falls below ${} ({}% under the recovery point) or makes a new low", price4(h), num_plain(hyst)),
                Some(h) => format!("hysteresis: once stated, the pull stands until the price falls below ${} ({}% under the recovery point) or makes a new low", price4(h), num_plain(hyst)),
                None => "hysteresis: no recovery point yet".to_string(),
            },
        ),
    ];
    let mut cool = cooldown_gate(cfg, clock, deposited_at, now);
    if standing {
        cool["passes"] = json!(true);
        cool["text"] = json!(format!("{} (it is checked only before a pull is first stated)", cool["text"].as_str().unwrap_or("")));
    }
    gates.push(cool);
    let all_pass = gates.iter().all(|g| g["passes"].as_bool() == Some(true));
    let gas = gas_per_tx_usd(data);
    let remove = pos.and_then(position_page_url);
    let pull = json!({
        "low_price_usd": low.map(p8),
        "low_at": low_at.map(|t| t.to_rfc3339()),
        "peak_converted_pct": format!("{:.2}", peak),
        "recovery_point_usd": trigger.map(p8),
        "recovery_pct_now": recovery_now.map(|v| format!("{:.2}", v)),
        "nock_now": m4(nock_now),
        "usdc_now": m2(usdc_now),
        "avg_buy_price_usd": avg.map(p8),
        "gas_usd": gas.map(m4),
        "gas_basis": PULL_GAS_BASIS,
        "fees_earned_usd": m2(fees),
        "remove_url": remove,
    });
    if all_pass {
        let text = format!(
            "{held}{gap} The price ${} is {:.2}% above that low (your {}%). A further rise into the rung sells its NOCK back for USDC; removing it now keeps the NOCK it bought. Gas ≈ ${} (one transaction, an estimate). {}. Whether and when is your decision.",
            price_words(pool_price),
            recovery_now.unwrap_or(0.0),
            num_plain(rec),
            gas.map(m4).unwrap_or_else(|| "unknown".to_string()),
            capitalize(&gates.iter().filter_map(|g| g["text"].as_str()).collect::<Vec<_>>().join("; "))
        );
        let event = json!({
            "kind": "ladder_deep_pull",
            "tier": "top",
            "title": "LADDER: PULL THE DEEP RUNG",
            "what_changed": format!("{label} bought low and the price has recovered past the point you set."),
            "text": text,
            "numbers": {
                "peak_converted_pct": format!("{:.2}", peak),
                "low_price_usd": low.map(p8),
                "pool_price_usd": pool_price.map(p8),
                "recovery_pct_now": recovery_now.map(|v| format!("{:.2}", v)),
                "nock_now": m4(nock_now),
                "usdc_now": m2(usdc_now),
                "avg_buy_price_usd": avg.map(p8),
                "gas_usd": gas.map(m4),
                "fees_earned_usd": m2(fees),
            },
            "links": links(&[("This rung in the Uniswap app (remove)", remove.clone())]),
            "would_sign": format!(
                "One transaction in your wallet: remove all of {label}'s liquidity; its {} NOCK and ${} USDC (and fees) return to the wallet. NockNock builds no transaction and signs nothing.",
                nock_words(nock_now),
                m2(usdc_now)
            ),
        });
        return ("suggested", text, gates, pull, event, true);
    }
    if state == "holds_only_nock" {
        let (text, event) = bottom_report();
        return ("fully_converted", text, gates, pull, event, false);
    }
    let failing: Vec<String> = gates.iter().filter(|g| g["passes"].as_bool() != Some(true)).filter_map(|g| g["text"].as_str().map(str::to_string)).collect();
    let text = format!("{held}{gap} It is in range, buying as the price falls. No pull is stated: {}.", failing.join("; "));
    ("holding", text, gates, pull, Value::Null, false)
}

fn capitalize(s: &str) -> String {
    let mut c = s.chars();
    match c.next() {
        Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
        None => String::new(),
    }
}

fn links(items: &[(&str, Option<String>)]) -> Value {
    Value::Array(items.iter().filter_map(|(l, u)| u.as_ref().map(|u| json!({ "label": l, "url": u }))).collect())
}
