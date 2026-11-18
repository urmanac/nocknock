use crate::*;

/// Below this a tranche is not worth advising (and rounding dust never keeps a plan open).
pub(crate) const PLAN_DUST_USDC: f64 = 1.0;

/// Why a plan is `blocked` in a cycle whose ledger read failed. A constant because the
/// notifier compares against it: that cycle's fills and spend are unknown, not zero, so
/// the next good cycle must not read as the plan having just completed (#89).
pub(crate) const PLAN_LEDGER_UNREADABLE_REASON: &str =
    "the accumulation ledger could not be read this cycle, so plan progress is unknown";

/// When the open tranche comes due by cadence alone: the last verified fill plus the
/// plan interval. None before the first fill (the first tranche is due from the start).
/// Pure. Fill detection's early / on-schedule label uses the same rounding of the
/// interval (`fill_timing`), measured from the buy before each swap.
pub(crate) fn next_tranche_due_at(cfg: &AccumulationPlanConfig, fills: &PlanFills) -> Option<DateTime<Utc>> {
    fills
        .last_at
        .map(|last| last + chrono::Duration::seconds((cfg.interval_hours * 3600.0).round() as i64))
}

/// Pure plan decision: is a tranche due right now, and for how much? The clock is an
/// argument so this is deterministic and unit-testable.
///
/// Rules: progress counts verified fills only (a failed or unsigned transaction can
/// never advance the plan); missed windows do NOT stack — after any absence exactly
/// one tranche is due and the plan simply ends later; a pause defers without
/// consuming a tranche; guardrails can shrink a tranche but never enlarge it.
pub(crate) fn plan_directive(
    cfg: &AccumulationPlanConfig,
    fills: &PlanFills,
    guards: &PlanGuards,
    ledger_unavailable: bool,
    now: DateTime<Utc>,
) -> PlanDirective {
    if !(cfg.budget_usdc > 0.0) || cfg.tranches == 0 || !(cfg.interval_hours >= 0.0) || cfg.id.trim().is_empty() {
        return PlanDirective::Invalid {
            reason: "accumulation_plan needs a non-empty id, budget_usdc > 0, tranches > 0 and interval_hours >= 0".to_string(),
        };
    }
    if cfg.cash_reserve_usdc.map_or(false, |r| !(r >= 0.0)) {
        return PlanDirective::Invalid {
            reason: "accumulation_plan.cash_reserve_usdc must be zero or positive".to_string(),
        };
    }
    if ledger_unavailable {
        return PlanDirective::Blocked {
            reason: PLAN_LEDGER_UNREADABLE_REASON.to_string(),
        };
    }
    let remaining = cfg.budget_usdc - fills.spent_usdc;
    if remaining < PLAN_DUST_USDC {
        return PlanDirective::Complete;
    }
    if let Some(starts_at) = cfg.starts_at {
        if now < starts_at {
            return PlanDirective::Pending { starts_at };
        }
    }
    if let Some(next_due_at) = next_tranche_due_at(cfg, fills) {
        if now < next_due_at {
            return PlanDirective::Waiting { next_due_at };
        }
    }
    if cfg.pause_when_overextended && guards.overextended {
        return PlanDirective::Paused {
            reason: "daily RSI(14) is at or above 70 (overextended); the tranche is deferred, not skipped".to_string(),
        };
    }
    if let Some(max_price) = cfg.max_price_usd {
        if guards.spot_price > max_price {
            return PlanDirective::Paused {
                reason: format!(
                    "spot ${:.5} is above the plan ceiling ${:.5}; the tranche is deferred, not skipped",
                    guards.spot_price, max_price
                ),
            };
        }
    }

    let nominal = cfg.budget_usdc / cfg.tranches as f64;
    // Candidate limits, smallest wins. The label names the guardrail in plain words.
    let mut usdc = nominal;
    let mut limited_by: Option<String> = None;
    let mut cap = |limit: f64, label: String| {
        if limit < usdc - 0.005 {
            usdc = limit;
            limited_by = Some(label);
        }
    };
    cap(remaining, format!("only ${:.2} of the plan budget remains", remaining));
    cap(
        guards.wallet_usdc - guards.tax_reserve_usd,
        format!(
            "wallet USDC ${:.2} less the ${:.2} virtual tax reserve",
            guards.wallet_usdc, guards.tax_reserve_usd
        ),
    );
    if cfg.respect_cash_target {
        cap(
            guards.wallet_usdc - guards.cash_target_usd,
            format!(
                "wallet USDC ${:.2} less the ${:.2} cash target",
                guards.wallet_usdc, guards.cash_target_usd
            ),
        );
    }
    if let Some(reserve) = cfg.cash_reserve_usdc {
        // The reserve is measured on the whole stable balance: wallet USDC plus USDC
        // inside NOCK LP. Moving USDC into a range therefore does not consume headroom;
        // buying NOCK with it does. USDC in a gas-side ETH/USDC range is not counted
        // (#151): it turns into WETH as ETH falls, so it is no NOCK reserve.
        let stable = guards.wallet_usdc + guards.lp_usdc;
        cap(
            stable - reserve,
            format!(
                "stable balance ${:.2} (wallet ${:.2} + NOCK LP ${:.2}) less the ${:.2} protected cash reserve",
                stable, guards.wallet_usdc, guards.lp_usdc, reserve
            ),
        );
    }
    if usdc < PLAN_DUST_USDC {
        return PlanDirective::Blocked {
            reason: format!(
                "nothing spendable: {}",
                limited_by.unwrap_or_else(|| "tranche size is below $1".to_string())
            ),
        };
    }
    PlanDirective::TrancheDue { usdc, limited_by }
}

/// Snapshot view of a plan: config + verified fills + this cycle's directive.
pub(crate) fn build_plan_status(
    cfg: &AccumulationPlanConfig,
    fills: &PlanFills,
    directive: &PlanDirective,
) -> AccumulationPlanStatus {
    let (status, tranche_usdc, next_due_at, binding_constraint) = match directive {
        PlanDirective::Invalid { reason } => ("invalid", 0.0, None, Some(reason.clone())),
        PlanDirective::Pending { starts_at } => ("pending", 0.0, Some(starts_at.to_rfc3339()), None),
        PlanDirective::TrancheDue { usdc, limited_by } => ("tranche_due", *usdc, None, limited_by.clone()),
        PlanDirective::Waiting { next_due_at } => ("waiting", 0.0, Some(next_due_at.to_rfc3339()), None),
        PlanDirective::Paused { reason } => ("paused", 0.0, None, Some(reason.clone())),
        PlanDirective::Blocked { reason } => ("blocked", 0.0, None, Some(reason.clone())),
        PlanDirective::Complete => ("complete", 0.0, None, None),
    };
    // A cycle that could not read the ledger knows nothing of the plan's progress
    // (issue #130): its amounts say "unknown", never the zero of an empty read.
    let progress_unknown =
        matches!(directive, PlanDirective::Blocked { reason } if reason == PLAN_LEDGER_UNREADABLE_REASON);
    let fills_done = fills.count.max(0) as u32;
    let complete = matches!(directive, PlanDirective::Complete);
    let tranche_index = if complete { fills_done } else { fills_done + 1 };
    let avg_price = if fills.acquired_nock > 0.0 {
        Some(fills.spent_usdc / fills.acquired_nock)
    } else {
        None
    };
    let nominal = if cfg.tranches > 0 { cfg.budget_usdc / cfg.tranches as f64 } else { 0.0 };
    AccumulationPlanStatus {
        plan_id: cfg.id.clone(),
        status: status.to_string(),
        venue: plan_venue_label(cfg),
        tranche_index,
        // A plan whose tranches were shrunk by guardrails can need more buys than planned.
        tranches_total: cfg.tranches.max(tranche_index),
        tranche_usdc: format!("{:.2}", tranche_usdc),
        nominal_tranche_usdc: format!("{:.2}", nominal),
        next_due_at,
        budget_usdc: format!("{:.2}", cfg.budget_usdc),
        spent_usdc: if progress_unknown { PLAN_PROGRESS_UNKNOWN.to_string() } else { format!("{:.2}", fills.spent_usdc) },
        remaining_usdc: if progress_unknown {
            PLAN_PROGRESS_UNKNOWN.to_string()
        } else {
            format!("{:.2}", (cfg.budget_usdc - fills.spent_usdc).max(0.0))
        },
        acquired_nock: if progress_unknown { PLAN_PROGRESS_UNKNOWN.to_string() } else { format!("{:.4}", fills.acquired_nock) },
        fills: fills.count,
        last_fill_at: fills.last_at.map(|t| t.to_rfc3339()),
        avg_price_usd: avg_price.map(|p| format!("{:.6}", p)),
        lump_sum_nock: fills
            .first_price
            .filter(|p| *p > 0.0)
            .map(|p| format!("{:.4}", cfg.budget_usdc / p)),
        binding_constraint,
        cash_reserve_usdc: cfg.cash_reserve_usdc.map(|r| format!("{:.2}", r)),
        // Attached by the caller that ran fill detection for this cycle (issue #118).
        fill_detection: None,
        // Attached by the cycle after the directive is known, and only for a due
        // tranche (issue #115).
        route_quotes: None,
        progress_unknown,
    }
}

/// The operator's venue label, or None when none is set (or it is blank). Pure.
pub(crate) fn plan_venue_label(cfg: &AccumulationPlanConfig) -> Option<String> {
    cfg.venue.as_deref().map(str::trim).filter(|v| !v.is_empty()).map(str::to_string)
}

/// What a plan amount reads when the ledger could not be read (issue #130).
pub(crate) const PLAN_PROGRESS_UNKNOWN: &str = "unknown";

/// Whether this plan status was built without a ledger read (issue #130). Pure.
pub(crate) fn plan_progress_unknown(ps: Option<&AccumulationPlanStatus>) -> bool {
    ps.is_some_and(|p| p.progress_unknown)
}

/// Advice text for an active (not complete) plan.
pub(crate) fn plan_advice(ps: &AccumulationPlanStatus, spot_price: f64) -> (String, String) {
    // No ledger read: no progress sentence and no "(n of N)", which would be the zero of
    // an empty read presented as the plan's (issue #130).
    if ps.progress_unknown {
        let details = format!(
            "Plan \"{}\": progress is unknown because {}. Nothing is paced until the ledger can be read again.",
            ps.plan_id,
            PLAN_LEDGER_UNREADABLE_REASON.trim_end_matches(", so plan progress is unknown")
        );
        return ("DCA BLOCKED (ledger unreadable)".to_string(), details);
    }
    let progress = format!(
        "Plan \"{}\": ${} of ${} deployed over {} verified buy(s){}.",
        ps.plan_id,
        ps.spent_usdc,
        ps.budget_usdc,
        ps.fills,
        ps.avg_price_usd
            .as_ref()
            .map(|p| format!(", average cost ${}", p))
            .unwrap_or_default()
    );
    let progress = match &ps.cash_reserve_usdc {
        Some(r) => format!("{} The plan never spends the stable balance (wallet + LP USDC) below ${}.", progress, r),
        None => progress,
    };
    let n = format!("{} of {}", ps.tranche_index, ps.tranches_total);
    let (action, details) = match ps.status.as_str() {
        "tranche_due" => (
            format!("DCA TRANCHE DUE ({n})"),
            format!(
                "Buy ${} of NOCK (spot ${:.5}{}).{} {} {}",
                ps.tranche_usdc,
                spot_price,
                // The operator's own label, never a venue NockNock chose (issue #132).
                ps.venue.as_deref().map(|v| format!("; your plan's venue label: {v}")).unwrap_or_default(),
                ps.binding_constraint
                    .as_ref()
                    .map(|c| format!(" Sized below the nominal ${} tranche: {}.", ps.nominal_tranche_usdc, c))
                    .unwrap_or_default(),
                progress,
                after_buying_sentence(ps)
            ),
        ),
        "waiting" => (
            format!("DCA ON SCHEDULE ({n} next)"),
            format!(
                "Nothing to buy yet: the next ${} tranche is due at {}. {}",
                ps.nominal_tranche_usdc,
                ps.next_due_at.clone().unwrap_or_default(),
                progress
            ),
        ),
        "pending" => (
            "DCA PLAN PENDING".to_string(),
            format!(
                "The plan starts at {}. {}",
                ps.next_due_at.clone().unwrap_or_default(),
                progress
            ),
        ),
        "paused" => (
            format!("DCA PAUSED ({n})"),
            format!(
                "Tranche held back: {}. {}",
                ps.binding_constraint.clone().unwrap_or_default(),
                progress
            ),
        ),
        "blocked" => (
            format!("DCA BLOCKED ({n})"),
            format!(
                "Tranche cannot be funded: {}. {}",
                ps.binding_constraint.clone().unwrap_or_default(),
                progress
            ),
        ),
        _ => (
            "DCA PLAN INVALID".to_string(),
            format!(
                "The configured plan cannot run: {}.",
                ps.binding_constraint.clone().unwrap_or_default()
            ),
        ),
    };
    // A detected swap that is not counted yet (issue #118) is not written here: the
    // directive engine leads the final text with it once, whichever arm wraps this one
    // (lead_with_open_offer_notice; issue #135).
    (action, details)
}

/// Fire-and-forget directive-transition notifications (issue #28). Delivery
/// failures are logged and never fatal to the ingestion cycle.
/// The instant from which buys may count toward a plan: the configured `starts_at`
/// when the operator set one (which may deliberately reach back to include earlier
/// buys), otherwise the moment the worker first saw the plan. Pure — unit-testable.
pub(crate) fn plan_anchor(starts_at: Option<DateTime<Utc>>, registered_at: DateTime<Utc>) -> DateTime<Utc> {
    starts_at.unwrap_or(registered_at)
}

/// Register the plan id on first sight and return when that was. Idempotent.
pub(crate) async fn plan_registered_at(db: &PgPool, plan_id: &str) -> std::result::Result<DateTime<Utc>, sqlx::Error> {
    sqlx::query("INSERT INTO accumulation_plans (plan_id) VALUES ($1) ON CONFLICT (plan_id) DO NOTHING")
        .bind(plan_id)
        .execute(db)
        .await?;
    let row = sqlx::query("SELECT registered_at FROM accumulation_plans WHERE plan_id = $1")
        .bind(plan_id)
        .fetch_one(db)
        .await?;
    row.try_get::<DateTime<Utc>, _>("registered_at")
}

/// Load the configured accumulation plan together with its receipt-verified fills.
/// A ledger read failure is surfaced (plan reports `blocked`) rather than treated as
/// "no buys yet", which would re-advise a tranche that was already bought.
pub(crate) async fn load_plan_context(state: &Arc<AppState>) -> PlanContext {
    let now = Utc::now();
    let Some(cfg) = state.config.accumulation_plan.clone() else {
        return PlanContext { cfg: None, fills: PlanFills::default(), ledger_unavailable: false, now };
    };
    if let Err(e) = plan_registered_at(&state.db, &cfg.id).await {
        tracing::error!("Accumulation plan registry unavailable: {e}");
        return PlanContext { cfg: Some(cfg), fills: PlanFills::default(), ledger_unavailable: true, now };
    }
    let row = sqlx::query(
        "SELECT COUNT(*) AS fills, \
                COALESCE(SUM(usdc_spent::numeric)::float8, 0) AS spent, \
                COALESCE(SUM(nock_received::numeric)::float8, 0) AS acquired, \
                MAX(confirmed_at) AS last_at, \
                (SELECT effective_price_usd::float8 FROM accumulation_events \
                  WHERE plan_id = $1 ORDER BY confirmed_at ASC, id ASC LIMIT 1) AS first_price \
         FROM accumulation_events WHERE plan_id = $1",
    )
    .bind(&cfg.id)
    .fetch_one(&state.db)
    .await;
    match row {
        Ok(r) => PlanContext {
            fills: PlanFills {
                count: r.try_get::<i64, _>("fills").unwrap_or(0),
                spent_usdc: r.try_get::<f64, _>("spent").unwrap_or(0.0),
                acquired_nock: r.try_get::<f64, _>("acquired").unwrap_or(0.0),
                last_at: r.try_get::<Option<DateTime<Utc>>, _>("last_at").ok().flatten(),
                first_price: r.try_get::<Option<f64>, _>("first_price").ok().flatten(),
            },
            cfg: Some(cfg),
            ledger_unavailable: false,
            now,
        },
        Err(e) => {
            tracing::error!("Accumulation ledger query failed: {e}");
            PlanContext { cfg: Some(cfg), fills: PlanFills::default(), ledger_unavailable: true, now }
        }
    }
}
