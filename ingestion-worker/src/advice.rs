use crate::*;

/// Pure: wallet gas and the ETH pillar from the wallet reads, the ETH inside LP and the
/// live ETH price. The pillar is wallet gas plus LP ETH/WETH legs.
pub(crate) fn gas_pillars(wallet_eth: Option<f64>, wallet_weth: Option<f64>, lp_eth: f64, eth_px: f64) -> GasPillars {
    let priced = eth_px.is_finite() && eth_px > 0.0;
    let gas_eth = match (wallet_eth, wallet_weth) {
        (Some(e), Some(w)) => Some(e + w),
        _ => None,
    };
    let total_eth = gas_eth.map(|g| g + lp_eth);
    GasPillars {
        wallet_gas_eth: gas_eth,
        wallet_gas_usd: gas_eth.filter(|_| priced).map(|g| g * eth_px),
        total_eth_amount: total_eth,
        total_eth_usd: total_eth.filter(|_| priced).map(|t| t * eth_px),
    }
}

// (The retired V4_TICK_LOWER_ONCHAIN / V4_TICK_UPPER_ONCHAIN constants and the
// single-position tick_to_display_price() were removed in v2.0.0: tick bounds now
// come from the per-position config/discovery, and display prices from
// tick_to_display_price_dec() with the pool's own token decimals.)

/// Signed basis-point distance from spot to the nearest bracket edge
/// (positive when spot is below the edge, negative when above).
pub(crate) fn bracket_distance_bps(spot: f64, lower: f64, upper: f64) -> i32 {
    if spot <= 0.0 || lower <= 0.0 || upper <= 0.0 {
        return 0;
    }
    let nearest_edge = if (spot - lower).abs() <= (spot - upper).abs() { lower } else { upper };
    (((nearest_edge - spot) / spot) * 10000.0).round() as i32
}

pub(crate) fn compute_structural_channels_sync(history: &[(DateTime<Utc>, f64)]) -> StructuralChannels {
    let mut channels = Vec::new();

    if history.len() >= 20 {
        let window = 20;
        let mut center_line = Vec::new();
        let mut upper_1sd = Vec::new();
        let mut lower_1sd = Vec::new();
        let mut upper_2sd = Vec::new();
        let mut lower_2sd = Vec::new();

        // `i` is the exclusive end of the window; iterating through len+1 means the final
        // window `[len-window..len]` includes the newest sample (off-by-one otherwise).
        for i in window..=history.len() {
            let window_slice = &history[i-window..i];
            let sum: f64 = window_slice.iter().map(|(_, v)| v).sum();
            let mean = sum / window as f64;
            let variance: f64 = window_slice.iter()
                .map(|(_, v)| (v - mean).powi(2))
                .sum::<f64>() / window as f64;
            let std_dev = variance.sqrt();

            // Label the point with the timestamp of the newest sample inside the window
            let ts = window_slice[window_slice.len() - 1].0;
            center_line.push(LinePoint { time: ts.to_rfc3339(), value: format!("{:.2}", mean) });
            upper_1sd.push(LinePoint { time: ts.to_rfc3339(), value: format!("{:.2}", mean + std_dev) });
            lower_1sd.push(LinePoint { time: ts.to_rfc3339(), value: format!("{:.2}", mean - std_dev) });
            upper_2sd.push(LinePoint { time: ts.to_rfc3339(), value: format!("{:.2}", mean + 2.0 * std_dev) });
            lower_2sd.push(LinePoint { time: ts.to_rfc3339(), value: format!("{:.2}", mean - 2.0 * std_dev) });
        }

        if !center_line.is_empty() {
            let last_idx = center_line.len() - 1;
            let break_point = if last_idx > 0 {
                Some(center_line[last_idx].time.clone())
            } else {
                None
            };

            channels.push(RegressionChannel {
                start_time: center_line.first().map(|p| p.time.clone()).unwrap_or_else(|| Utc::now().to_rfc3339()),
                end_time: None,
                center_line,
                upper_1sd,
                lower_1sd,
                upper_2sd,
                lower_2sd,
                is_active: true,
                break_point,
            });
        }
    }

    StructuralChannels { channels }
}

pub(crate) fn compute_counterfactual_sync(
    history: &[(DateTime<Utc>, f64, f64)],
    current_portfolio: f64,
    current_native: f64,
    now_str: &str,
) -> StrategyCounterfactual {
    let mut time_points = Vec::new();
    let mut active_series = Vec::new();
    let mut passive_series = Vec::new();

    for (ts, portfolio, native) in history {
        time_points.push(ts.to_rfc3339());
        active_series.push(format!("{:.2}", portfolio));
        passive_series.push(format!("{:.2}", native));
    }

    // Always include current point
    time_points.push(now_str.to_string());
    active_series.push(format!("{:.2}", current_portfolio));
    passive_series.push(format!("{:.2}", current_native));

    StrategyCounterfactual {
        time_points,
        active_portfolio_series: active_series,
        passive_baseline_series: passive_series,
    }
}

/// Claim recommendation computed from live claimable yield vs gas break-even.
/// The 2x factor mirrors the gas_to_fee margin used elsewhere; gas <= 0 means gas is
/// unknown (not free), so only a moderate recommendation is possible.
pub(crate) fn recommended_claim_action(claimable_usd: f64, gas_break_even_usd: f64) -> &'static str {
    if gas_break_even_usd > 0.0 && claimable_usd >= 2.0 * gas_break_even_usd {
        "claim_now"
    } else if claimable_usd > 0.0 {
        "claim_recommended"
    } else {
        "accumulate"
    }
}

/// Plan-less entry point (unit tests and callers without an accumulation plan).
#[allow(dead_code)]
pub(crate) fn compute_derived_metrics(raw: &RawOnChainData, prev_hwm: f64, profile: &OperatorProfile, structural_channels: StructuralChannels, counterfactual_history: &[(DateTime<Utc>, f64, f64)], ledger: &LedgerAggregate, regime: &MarketRegime) -> DerivedMetrics {
    compute_derived_metrics_with_plan(raw, prev_hwm, profile, structural_channels, counterfactual_history, ledger, regime, &PlanContext::none())
}

/// Where spot stands against the top of the price-source range: the position, the top
/// in display units (USD per NOCK) and whether spot is at or above it. One definition
/// for the RANGE EXIT directive and for the range-exit push (#89), so they cannot
/// disagree. None when there is no ranged price-source position, when that range holds
/// no liquidity (an emptied range has nothing to exit: it earns nothing in or out of
/// range), or when spot or the top is not a positive number.
pub(crate) fn price_source_range_top(raw: &RawOnChainData) -> Option<(&RawPosition, f64, bool)> {
    let spot = raw.spot_price_nock_usd;
    raw.positions
        .iter()
        .find(|p| p.is_price_source && !is_full_range(p.tick_lower, p.tick_upper))
        .filter(|p| p.liquidity > 0)
        .map(|p| (p, tick_to_display_price_dec(p.tick_lower, p.token0.decimals, p.token1.decimals)))
        .filter(|(_, top)| spot > 0.0 && *top > 0.0)
        .map(|(p, top)| (p, top, spot >= top))
}

#[allow(clippy::too_many_arguments)]
pub(crate) fn compute_derived_metrics_with_plan(raw: &RawOnChainData, prev_hwm: f64, profile: &OperatorProfile, structural_channels: StructuralChannels, counterfactual_history: &[(DateTime<Utc>, f64, f64)], ledger: &LedgerAggregate, regime: &MarketRegime, plan: &PlanContext) -> DerivedMetrics {
    compute_derived_metrics_with_fills(raw, prev_hwm, profile, structural_channels, counterfactual_history, ledger, regime, plan, None)
}

/// As above, with this cycle's fill-detection result (issue #118). The result is
/// attached to the plan status, and the plan's advice text reads it from there; the
/// function stays pure, since detection's I/O happened before the call.
#[allow(clippy::too_many_arguments)]
pub(crate) fn compute_derived_metrics_with_fills(raw: &RawOnChainData, prev_hwm: f64, profile: &OperatorProfile, structural_channels: StructuralChannels, counterfactual_history: &[(DateTime<Utc>, f64, f64)], ledger: &LedgerAggregate, regime: &MarketRegime, plan: &PlanContext, fill_detection: Option<&FillDetection>) -> DerivedMetrics {
    // 1. Spot price: live NOCK/USD from the designated price-source position pool
    //    (zero-mock: the cycle fails upstream when the read fails).
    let spot_price = raw.spot_price_nock_usd;

    // 2. LP aggregates across all tracked positions (v2.0.0; replaces tranche 1/2 math).
    let mut lp_nock = 0.0f64;
    let mut lp_usdc = 0.0f64;
    let mut lp_usd = 0.0f64;
    let mut lp_fee_nock = 0.0f64;
    let mut lp_fee_usd = 0.0f64;
    // ETH/WETH inside tracked LP: the LP part of the ETH pillar (issue #94).
    let mut lp_eth = 0.0f64;
    // Per-position (address, label, fee_usd, LP-claimable, reason) tuples for the velocity
    // panel, with what a not-LP-claimable position earns instead (#142): the kind, the
    // gauge reward and the reason alone.
    #[allow(clippy::type_complexity)]
    let mut fee_entries: Vec<(String, String, f64, bool, Option<String>, (Option<String>, Option<CollectibleReward>, Option<String>))> = Vec::new();
    for p in &raw.positions {
        let p0 = leg_usd_price(&p.token0, spot_price, raw.live_eth_price_usd);
        let p1 = leg_usd_price(&p.token1, spot_price, raw.live_eth_price_usd);
        lp_usd += p.amount0 * p0 + p.amount1 * p1;
        if is_usdc_token(&p.token0) { lp_usdc += p.amount0; }
        if is_usdc_token(&p.token1) { lp_usdc += p.amount1; }
        if is_nock_token(&p.token0) { lp_nock += p.amount0; }
        if is_nock_token(&p.token1) { lp_nock += p.amount1; }
        if is_eth_token(&p.token0) { lp_eth += p.amount0; }
        if is_eth_token(&p.token1) { lp_eth += p.amount1; }
        let fee_usd = p.fee0 * p0 + p.fee1 * p1;
        lp_fee_usd += fee_usd;
        if is_nock_token(&p.token0) { lp_fee_nock += p.fee0; }
        if is_nock_token(&p.token1) { lp_fee_nock += p.fee1; }
        // Protocol-specific naming lives in adapters/mod.rs. A position with a
        // fees_note (an Aero CL position, #91) has no LP-claimable fee figure.
        let facts = raw.position_facts.get(&p.position_id);
        let fees_note = facts.and_then(|f| f.fees_note.clone());
        let lp_claimable = fees_note.is_none() && !raw.fees_not_lp_claimable.contains(&p.position_id);
        // The row's note (#93): the position's unclaimed AERO, or why it was not read,
        // after the reason its fees are not LP-claimable. Amounts only. A row whose
        // fees ARE LP-claimable carries a fee action (claim_now, accumulate), and an
        // AERO amount beside that word would read as advice about AERO, so such a row
        // gets no AERO statement: the amount is on the position itself
        // (positions[].unclaimed_aero) and in the pillar lines. With no AERO statement
        // the note is what it was before #93.
        // The reward as the row's earnings (#142): only on a row with no fee action.
        let earnings = fee_row_earnings(lp_claimable, fees_note.as_deref(), collectible_reward_for(&raw.aero, &p.position_id));
        let note = fee_row_note(lp_claimable, fees_note, aero_statement_for(&raw.aero, &p.position_id));
        fee_entries.push((
            fee_contract_for(
                &p.protocol,
                p.npm_address.as_deref(),
                &p.pool_id,
                facts.and_then(|f| f.staked_in_gauge.as_deref()),
            ),
            velocity_label(p),
            fee_usd,
            lp_claimable,
            note,
            earnings,
        ));
    }

    let total_fees_usd = lp_fee_usd;
    // Zero-mock invariant: no fabricated gas fallback. A zero value means the gas query
    // was unavailable (only reachable in unit tests via default RawOnChainData) — derived
    // efficiency/action values then treat gas as unknown, never as free.
    let gas_cost = raw.live_gas_cost_usd;

    // 3. Sovereign Treasury (wallet) and totals
    let wallet_nock = raw.wallet_balances.iter()
        .find(|w| w.token_symbol == "NOCK")
        .and_then(|w| w.amount.parse().ok())
        .unwrap_or(0.0);
    let t3_nock = wallet_nock;
    let t3_usd = t3_nock * spot_price;

    let wallet_usdc: f64 = raw.wallet_balances.iter()
        .find(|w| w.token_symbol == "USDC")
        .and_then(|w| w.amount.parse().ok())
        .unwrap_or(0.0);

    // Gas pillar (issue #94): wallet ETH + WETH, counted in the total at the live ETH
    // price, but never cash, never dry powder, never a funding source and never under a
    // target. Unread (tests only) or unpriced gas is null, never zero.
    let eth_px = raw.live_eth_price_usd;
    let eth_priced = eth_px.is_finite() && eth_px > 0.0;
    let pillars = gas_pillars(raw.wallet_eth, raw.wallet_weth, lp_eth, eth_px);
    let wallet_gas_usd = pillars.wallet_gas_usd.unwrap_or(0.0);

    // AERO pillar (issue #93): wallet AERO, counted in the total at the live AERO price
    // like gas, and like gas never cash, never dry powder, never a funding source and
    // never under a target. Unclaimed AERO in the gauges is NOT in the total, exactly as
    // uncollected LP fees are not: it is stated beside them and counts once claimed.
    // NockNock gives no advice about AERO; no directive below reads these figures.
    let aero = aero_pillar(&raw.aero);
    let wallet_aero_usd = aero.wallet_usd.unwrap_or(0.0);
    // Collectible earnings (#142): LP fees and gauge rewards side by side and their sum.
    // Reported only. Nothing below reads these: the total, the harvest threshold and the
    // directive are computed exactly as before.
    let collectible = collectible_totals(&raw.aero, &aero, total_fees_usd);

    // One definition of the portfolio: every pillar, gas and AERO included. Targets,
    // weights, the high-water mark and history all use it.
    let total_portfolio_usd = lp_usd + t3_usd + wallet_usdc + wallet_gas_usd + wallet_aero_usd;
    // NOCK only (wallet + LP); ETH is its own pillar (total_eth_usd).
    let total_native_usd = (lp_nock + t3_nock) * spot_price;
    let total_stable_usd = lp_usdc + wallet_usdc;

    let floor = profile.treasury_floor_nock;
    let now_str = Utc::now().to_rfc3339();
    let total_nock = lp_nock + t3_nock + lp_fee_nock;
    let nock_floor_delta = total_nock - floor;
    let current_hwm = prev_hwm.max(total_portfolio_usd);

    // Liquid dry powder (issue #63): only wallet USDC is spendable. USDC inside LP is
    // working capital (a USDC/NOCK range converts it to NOCK as the price falls), so it
    // neither covers the virtual tax reserve nor counts as dry powder: the same rule
    // plan_directive's tax cap and the LP rebuild's liquid floor already use.
    let tax_reserve_usd = ledger.total_usd * profile.tax_reserve_ratio;
    let tax_guardrail_blocked = tax_reserve_usd > 0.0 && wallet_usdc < tax_reserve_usd;
    // Cash target (issue #79) and tax reserve overlap: the larger one stays in the wallet.
    let cash_target_usd = total_portfolio_usd * (profile.target_weights_bps.cash as f64 / 10000.0);
    let deployable_usdc = (wallet_usdc - tax_reserve_usd.max(cash_target_usd)).max(0.0);
    let lp_usdc_note = if lp_usdc >= 0.005 {
        format!(" The ${:.2} of USDC inside LP is working capital, not dry powder: it stays in its ranges.", lp_usdc)
    } else {
        String::new()
    };
    // Only rules the operator set are named: an unset cash weight or tax ratio is no rule,
    // never a default deduction.
    let cash_target_set = profile.target_weights_bps.cash > 0;
    let tax_ratio_set = profile.tax_reserve_ratio > 0.0;
    let dry_powder_amount = if deployable_usdc >= PLAN_DUST_USDC { format!("${:.2}", deployable_usdc) } else { "none".to_string() };
    let dry_powder_sentence = match (cash_target_set, tax_ratio_set) {
        (true, true) => format!(
            "Liquid dry powder: {} (wallet USDC ${:.2} less the larger of the ${:.2} cash target and the ${:.2} virtual tax reserve).",
            dry_powder_amount, wallet_usdc, cash_target_usd, tax_reserve_usd
        ),
        (true, false) => format!(
            "Liquid dry powder: {} (wallet USDC ${:.2} less the ${:.2} cash target).",
            dry_powder_amount, wallet_usdc, cash_target_usd
        ),
        (false, true) => format!(
            "Liquid dry powder: {} (wallet USDC ${:.2} less the ${:.2} virtual tax reserve).",
            dry_powder_amount, wallet_usdc, tax_reserve_usd
        ),
        (false, false) => format!(
            "Liquid dry powder: {} (wallet USDC; no cash target or tax reserve ratio is configured).",
            dry_powder_amount
        ),
    };
    // ETH is gas: stated as a fact when the wallet has none, never sized.
    let gas_note = match (raw.wallet_eth, raw.wallet_weth) {
        (Some(e), Some(w)) if e <= 0.0 && w <= 0.0 => " The wallet holds no ETH; every action on Base pays its gas in ETH.",
        _ => "",
    };
    let own_plan_hint = "add an accumulation_plan with your own budget_usdc, tranches and interval_hours";

    // Accumulation plan (issue #79): pure decision over config + verified fills.
    let plan_status: Option<AccumulationPlanStatus> = plan.cfg.as_ref().map(|cfg| {
        let guards = PlanGuards {
            overextended: regime.overextended,
            spot_price,
            wallet_usdc,
            lp_usdc,
            tax_reserve_usd,
            cash_target_usd,
        };
        let directive = plan_directive(cfg, &plan.fills, &guards, plan.ledger_unavailable, plan.now);
        let mut status = build_plan_status(cfg, &plan.fills, &directive);
        status.fill_detection = fill_detection.cloned();
        status
    });
    // While a plan exists it owns the buy decision: the lump-sum dip arms are retired
    // (a completed plan must not be followed by "deploy everything that is left").
    let plan_configured = plan_status.is_some();
    let active_plan = plan_status.as_ref().filter(|p| p.status != "complete");
    // An operator who opts out of the RSI pause keeps buying through an overextension.
    let plan_overrides_cooldown = active_plan.is_some()
        && plan.cfg.as_ref().map_or(false, |c| !c.pause_when_overextended);
    let mut lp_rebuild = lp_rebuild_status(lp_usd, total_portfolio_usd, profile.target_weights_bps.uniswap_lp);
    // Nothing tracked: there is no range to rebuild, so no suggestion and no bias text
    // (a user who never held LP is not told to "re-establish" one).
    if raw.positions.is_empty() {
        lp_rebuild.status = "no_positions".to_string();
    }
    if lp_rebuild.status == "under_target" {
        let lp_target_usd = total_portfolio_usd * (profile.target_weights_bps.uniswap_lp as f64 / 10000.0);
        // USDC that must stay liquid: the cash target, or the tax reserve plus the
        // accumulation plan's unspent budget, whichever is larger.
        let plan_remaining = active_plan
            .and_then(|p| p.remaining_usdc.parse::<f64>().ok())
            .unwrap_or(0.0);
        let liquid_floor = cash_target_usd.max(tax_reserve_usd + plan_remaining);
        let floor_tolerance_nock = floor * (profile.lp_floor_tolerance_bps as f64 / 10000.0);
        let funds = RebuildFunds {
            // A range sells its NOCK as price rises, so in a large enough rally NOCK
            // placed in LP is converted away entirely. LP may hold the NOCK above the
            // hard floor plus the operator's tolerated overage, which bounds how far
            // below the floor a full sell-out of every range could leave the stack.
            nock: wallet_nock - floor + floor_tolerance_nock,
            floor_tolerance_nock,
            floor_set: floor > 0.0,
            usdc: wallet_usdc - liquid_floor,
        };
        let (suggestions, excluded) = lp_rebuild_suggestions(raw, lp_target_usd, funds);
        // Worst case for the floor: every range sells all its NOCK, so only wallet
        // NOCK not being deposited remains.
        let suggested_nock: f64 = suggestions
            .iter()
            .map(|sg| {
                (if sg.token0_symbol == "NOCK" { sg.amount0.parse::<f64>().unwrap_or(0.0) } else { 0.0 })
                    + (if sg.token1_symbol == "NOCK" { sg.amount1.parse::<f64>().unwrap_or(0.0) } else { 0.0 })
            })
            .sum();
        lp_rebuild.worst_case_nock = Some(format!("{:.0}", wallet_nock - suggested_nock));
        lp_rebuild.floor_tolerance_nock = Some(format!("{:.0}", floor_tolerance_nock));
        lp_rebuild.suggestions = suggestions;
        lp_rebuild.excluded = excluded;
    }

    // Dynamic Shot Advice Engine (v0.5.1). Every figure comes from config or the chain;
    // NockNock never sizes a buy outside the operator's own accumulation plan.
    let (lp_summary, engine_active) = lp_positions_summary(raw);
    // RANGE EXIT reports the price-source range's own value, never the whole LP book.
    let range_exit: Option<(String, String)> = price_source_range_top(raw)
        .filter(|(_, _, above)| *above)
        .map(|(p, top, _)| {
            let p0 = leg_usd_price(&p.token0, spot_price, raw.live_eth_price_usd);
            let p1 = leg_usd_price(&p.token1, spot_price, raw.live_eth_price_usd);
            let pos_usd = p.amount0 * p0 + p.amount1 * p1;
            (
                "RANGE EXIT FLASHPOINT (Above Range)".to_string(),
                format!(
                    "Spot ${:.4} is at or above the top of the price-source range (${:.4}). That range ({}) holds ${:.2} ({:.2} {} + {:.2} {}) and earns no fees unless the price falls back into it. Whether to leave it, withdraw it or open a new range is your decision; NockNock does not choose range bounds.",
                    spot_price,
                    top,
                    position_label(p),
                    pos_usd,
                    p.amount0,
                    p.token0.symbol,
                    p.amount1,
                    p.token1.symbol
                ),
            )
        });
    // BATCH HARVEST: the split comes from the configured ratio, and the text says what the
    // harvest ledger actually books (USDC, NOCK and WETH/ETH, each valued at the claim's block).
    // AERO is deliberately not named here (#93): this directive is about LP fees, and
    // no directive may read as advice to claim AERO. The ledger books AERO all the same.
    let harvest: Option<(String, String)> = (total_fees_usd >= 5.00).then(|| {
        let gas_efficiency = if gas_cost > 0.0 {
            format!("{:.0}x", total_fees_usd / gas_cost)
        } else {
            "unknown".to_string()
        };
        let r = profile.tax_reserve_ratio * 100.0;
        let booking = if tax_ratio_set {
            format!(
                "NockNock books {:.0}% of the USDC, NOCK and WETH/ETH received to the virtual tax reserve and {:.0}% to growth",
                r,
                100.0 - r
            )
        } else {
            "NockNock records the USDC, NOCK and WETH/ETH received; no tax_reserve_ratio is configured, so none of it is booked to a virtual tax reserve".to_string()
        };
        (
            "BATCH HARVEST SHOT (Execute Claim)".to_string(),
            format!(
                "Uncollected LP fees (${:.2}) have reached the fixed $5.00 harvest threshold ({} gas efficiency). After harvesting, record the transaction: {}, each valued at the claim's block. Bookkeeping only; nothing moves on-chain.",
                total_fees_usd,
                gas_efficiency,
                booking
            ),
        )
    });
    // A plan with a tranche due (or a config error to report) leads. A plan with nothing
    // due (waiting, pending, paused, blocked) yields to a range exit or a due harvest (N11).
    let plan_leads = active_plan.filter(|p| !matches!(p.status.as_str(), "waiting" | "pending" | "paused" | "blocked"));
    let exit_or_harvest = range_exit.clone().or_else(|| harvest.clone());
    let wallet_label = raw
        .wallet_balances
        .first()
        .map(|w| format!("The tracked wallet {}", w.wallet_address))
        .unwrap_or_else(|| "The tracked wallet".to_string());
    let dip_zone = spot_price > 0.0 && spot_price < 0.020;

    let (advice_action, advice_details) = if floor > 0.0 && total_nock < floor {
        let deficit = format!(
            "Total NOCK ({:.0}) is below the {:.0} floor by {:.0} NOCK.",
            total_nock,
            floor,
            floor - total_nock
        );
        match active_plan {
            // The plan stays visible: a tranche transition changes the action, so it notifies.
            Some(ps) => {
                let (plan_action, plan_details) = plan_advice(ps, spot_price);
                (
                    format!("DEFENSIVE LOCK (Floor Recovery) · {}", plan_action),
                    format!("{} Your accumulation plan is the recovery path: {}", deficit, plan_details),
                )
            }
            // A configured plan that is complete is not "no plan": say so, by its id.
            None => match plan_status.as_ref() {
                Some(done) => (
                    "DEFENSIVE LOCK (Floor Recovery)".to_string(),
                    format!(
                        "{} Your accumulation plan \"{}\" is complete. NockNock does not size recovery buys: extend its budget_usdc or add a new plan with your own budget, tranches and interval to have them paced.",
                        deficit, done.plan_id
                    ),
                ),
                None => (
                    "DEFENSIVE LOCK (Floor Recovery)".to_string(),
                    format!(
                        "{} NockNock does not size recovery buys: {} to have them paced.",
                        deficit, own_plan_hint
                    ),
                ),
            },
        }
    } else if !plan_configured && lp_usd + t3_usd + wallet_usdc <= 0.0 && total_nock <= 0.0 {
        // Gas alone is not something to track or deploy: a gas-only wallet is still empty.
        // AERO is treated the same way here (#93), so it never changes which directive
        // is chosen; the amount held is stated, with no advice about it.
        // The test adds up the parts that are neither gas nor AERO. It must not subtract
        // gas and AERO back out of the total: (gas + aero) - gas - aero leaves a float
        // residue, which made a gas-and-AERO-only wallet miss this branch.
        let gas_held = match pillars.wallet_gas_eth {
            Some(g) if g > 0.0 => format!(
                " It holds {:.6} ETH/WETH for gas, counted in the total; gas is not cash and is never deployed.",
                g
            ),
            _ => String::new(),
        };
        let aero_held = match aero.wallet_amount {
            Some(a) if a > 0.0 => format!(" It holds {:.6} AERO, counted in the total; AERO is not cash and is never deployed.", a),
            _ => String::new(),
        };
        (
            "NOTHING TO TRACK YET (Empty Wallet)".to_string(),
            format!(
                "{} holds no NOCK, no USDC and no LP value that NockNock reads. Once it is funded NockNock reports it here. To have NOCK buys paced, {}; set operator_profile.treasury_floor_nock only if you want a floor alarm.{}{}{}",
                wallet_label, own_plan_hint, gas_note, gas_held, aero_held
            ),
        )
    } else if regime.overextended && !plan_overrides_cooldown {
        let cooldown = format!(
            "NOCK spot (${:.5}) is overextended: daily RSI(14) {} is at or above 70.{} NockNock advises no sale and no buy while it stays there.",
            spot_price,
            regime.rsi_1d.map(|r| format!("{:.0}", r)).unwrap_or_else(|| "unavailable".to_string()),
            if active_plan.is_some() {
                " Your accumulation plan pauses buys until it cools."
            } else {
                ""
            }
        );
        // A cooldown has nothing to buy, so a range exit or a due harvest outranks it
        // (N11); the overextension fact stays in the details.
        match exit_or_harvest.as_ref() {
            Some((a, d)) => (a.clone(), format!("{} {}", d, cooldown)),
            None => ("OVEREXTENDED (Daily RSI ≥ 70, No Buy Advised)".to_string(), cooldown),
        }
    } else if let Some(ps) = plan_leads {
        plan_advice(ps, spot_price)
    } else if let Some(adv) = exit_or_harvest.clone() {
        // Outranks a plan with nothing due (N11) and every planless arm below, none of
        // which has anything to buy.
        adv
    } else if let Some(ps) = active_plan {
        plan_advice(ps, spot_price)
    } else if !plan_configured && dip_zone && tax_guardrail_blocked {
        (
            "TAX RESERVE SHORT (No Buy Advised)".to_string(),
            format!(
                "Spot price (${:.5}) is below the $0.020 accumulation line, but the virtual tax reserve (${:.2}) exceeds liquid wallet USDC (${:.2}).{} No buy is advised while wallet USDC is below the reserve.",
                spot_price, tax_reserve_usd, wallet_usdc, lp_usdc_note
            ),
        )
    } else if !plan_configured && total_nock <= 0.0 && wallet_usdc >= PLAN_DUST_USDC {
        (
            "BUILD FROM ZERO (No Buy Plan Configured)".to_string(),
            format!(
                "{} holds ${:.2} of USDC and no NOCK.{} NockNock does not choose amounts for you: {} (optionally cash_reserve_usdc and max_price_usd) and it will pace each NOCK buy and verify it on-chain.{}",
                wallet_label, wallet_usdc, lp_usdc_note, own_plan_hint, gas_note
            ),
        )
    } else if !plan_configured && dip_zone && deployable_usdc >= PLAN_DUST_USDC {
        // Last arm before HOLD: it must never hide a range exit or a due harvest.
        (
            "DIP ZONE (No Buy Plan Configured)".to_string(),
            format!(
                "Spot ${:.5} is below the $0.020 accumulation line and the wallet holds ${:.2} of liquid USDC. {}{} NockNock does not size buys outside a plan: {} and it will pace them.{}",
                spot_price, wallet_usdc, dry_powder_sentence, lp_usdc_note, own_plan_hint, gas_note
            ),
        )
    } else {
        let floor_text = if floor > 0.0 {
            format!("Total NOCK held is {:.0} (+{:.0} above the {:.0} floor).", total_nock, nock_floor_delta, floor)
        } else {
            format!("Total NOCK held is {:.0}; no floor configured.", total_nock)
        };
        // The "HOLD & ACCRETE" prefix stays (apply_lp_rebuild_bias keys idle detection on
        // it); the parenthetical states only what the positions show.
        let label = if engine_active {
            "HOLD & ACCRETE (Dual Growth Engine Active)"
        } else if raw.positions.is_empty() {
            "HOLD & ACCRETE (No LP Tracked)"
        } else if lp_usd <= 0.0 {
            "HOLD & ACCRETE (LP Empty)"
        } else {
            "HOLD & ACCRETE (No Ranged LP Confirmed In Range)"
        };
        let dip_note = if !plan_configured && dip_zone {
            format!(
                " Spot (${:.5}) is below the $0.020 accumulation line. NockNock does not size buys outside a plan: {} to have them paced.",
                spot_price, own_plan_hint
            )
        } else {
            String::new()
        };
        (
            label.to_string(),
            format!(
                "{} {}{} {}{}{}",
                floor_text,
                lp_summary,
                if raw.positions.is_empty() { String::new() } else { format!(" ${:.2} of LP fees uncollected.", total_fees_usd) },
                dry_powder_sentence,
                lp_usdc_note,
                dip_note
            ),
        )
    };

    let (advice_action, advice_details) = apply_lp_rebuild_bias(advice_action, advice_details, &lp_rebuild);
    // A detected swap waiting for an answer is said in every arm, not only the plan's
    // own (issue #118). The title is untouched, so the notifier does not fire for it.
    let advice_details = lead_with_open_offer_notice(plan_status.as_ref(), advice_details);

    DerivedMetrics {
        global_status: GlobalStatus {
            total_portfolio_usd: format!("{:.2}", total_portfolio_usd),
            total_native_usd: format!("{:.2}", total_native_usd),
            total_stable_usd: format!("{:.2}", total_stable_usd),
            total_fees_accrued_usd: format!("{:.2}", total_fees_usd),
            total_fees_claimable_usd: format!("{:.2}", total_fees_usd),
            // Signed: fees less two claims' gas. A negative net is shown, never clamped to 0.
            gas_to_fee_delta_usd: format!("{:.2}", total_fees_usd - (gas_cost * 2.0)),
            high_water_mark_usd: format!("{:.2}", current_hwm),
            // The cycle replaces this with the peak row's own time when the mark was set
            // earlier (resolve_high_water_mark_date); here it is a new mark.
            high_water_mark_date: now_str.clone(),
            total_nock_amount: format!("{:.0}", total_nock),
            nock_floor_delta: format!("{:.0}", nock_floor_delta),
            nock_floor_target: format!("{:.0}", floor),
            total_usdc_amount: format!("{:.2}", total_stable_usd),
            shot_advice_action: advice_action,
            shot_advice_details: advice_details,
            regime: regime_label(&regime).to_string(),
            rsi_14: regime.rsi_5m.map(|r| format!("{:.1}", r)),
            rsi_14_1d: regime.rsi_1d.map(|r| format!("{:.1}", r)),
            eth_price_usd: eth_priced.then(|| format!("{:.2}", eth_px)),
            wallet_gas_eth: pillars.wallet_gas_eth.map(|v| format!("{:.6}", v)),
            wallet_gas_usd: pillars.wallet_gas_usd.map(|v| format!("{:.2}", v)),
            total_eth_amount: pillars.total_eth_amount.map(|v| format!("{:.6}", v)),
            total_eth_usd: pillars.total_eth_usd.map(|v| format!("{:.2}", v)),
            aero_price_usd: aero.price_usd.map(|v| format!("{:.6}", v)),
            wallet_aero_amount: aero.wallet_amount.map(|v| format!("{:.6}", v)),
            wallet_aero_usd: aero.wallet_usd.map(|v| format!("{:.2}", v)),
            unclaimed_aero_amount: aero.unclaimed_amount.map(|v| format!("{:.6}", v)),
            unclaimed_aero_usd: aero.unclaimed_usd.map(|v| format!("{:.2}", v)),
            aero_note: aero.note.clone(),
            collectible_rewards_usd: collectible.rewards_usd.clone(),
            collectible_rewards_note: collectible.note.clone(),
            total_collectible_usd: collectible.total_usd.clone(),
            // Closed positions (#144) are stated here and counted nowhere above: they
            // are not in raw.positions, which every figure of this function is built from.
            closed_positions_count: (!raw.closed_positions.is_empty()).then_some(raw.closed_positions.len() as u32),
            closed_positions_note: closed_positions_note(&raw.closed_positions),
        },
        asset_aggregation_series: AssetAggregationSeries {
            time_points: vec![now_str.clone()],
            safe_haven_series: vec![format!("{:.2}", t3_usd)],
            wallet_nock_series: vec![format!("{:.4}", t3_nock)],
            active_protocol_series: vec![format!("{:.2}", lp_usd)],
            positions_nock: raw
                .positions
                .iter()
                .map(|p| {
                    let nock_amt = if is_nock_token(&p.token0) {
                        p.amount0
                    } else if is_nock_token(&p.token1) {
                        p.amount1
                    } else {
                        0.0
                    };
                    PositionNockSeries {
                        position_id: p.position_id.clone(),
                        series: vec![format!("{:.2}", nock_amt)],
                    }
                })
                .collect(),
        },
        total_value_vs_baseline: TotalValueVsBaseline {
            time_points: vec![now_str.clone()],
            portfolio_value_series: vec![format!("{:.2}", total_portfolio_usd)],
            token_price_series: vec![format!("{:.5}", spot_price)],
            high_water_mark_series: vec![format!("{:.2}", current_hwm)],
        },
        tick_bracket_hypervisor: TickBracketHypervisor {
            current_price: format!("{:.5}", spot_price),
            // Zero-mock: one bracket per tracked position, straight from its live ticks.
            brackets: raw
                .positions
                .iter()
                .map(|p| {
                    let p0 = leg_usd_price(&p.token0, spot_price, raw.live_eth_price_usd);
                    let p1 = leg_usd_price(&p.token1, spot_price, raw.live_eth_price_usd);
                    let pos_usd = p.amount0 * p0 + p.amount1 * p1;
                    // Display orientation: bigger tick -> lower price (token0-per-token1).
                    let lower_disp = tick_to_display_price_dec(p.tick_upper, p.token0.decimals, p.token1.decimals);
                    let upper_disp = tick_to_display_price_dec(p.tick_lower, p.token0.decimals, p.token1.decimals);
                    let full_range = is_full_range(p.tick_lower, p.tick_upper);
                    TickBracket {
                        lower_price: if full_range { "0".to_string() } else { fmt_price(lower_disp) },
                        upper_price: if full_range { "0".to_string() } else { fmt_price(upper_disp) },
                        liquidity_usd: format!("{:.2}", pos_usd),
                        in_range: p.in_range,
                        full_range,
                        // Spot is USD/NOCK; brackets are token0-per-NOCK. Only USD-
                        // denominated token0 (USDC) yields an honest distance.
                        distance_bps: if full_range {
                            None
                        } else if p.token0.symbol == "USDC" {
                            Some(bracket_distance_bps(spot_price, lower_disp, upper_disp))
                        } else {
                            None
                        },
                    }
                })
                .collect(),
        },
        portfolio_deviation_matrix: PortfolioDeviationMatrix {
            allocations: {
                let calc_status = |deviation_bps: i32, target_weight_bps: i32| -> String {
                    let tol = (target_weight_bps as f64 * 0.20).max(100.0) as i32;
                    // An unset weight (0) is no target: a measurement, never "over-funded".
                    if target_weight_bps <= 0 {
                        "no_target".to_string()
                    } else if deviation_bps > tol {
                        "over_funded".to_string()
                    } else if deviation_bps < -tol {
                        "under_funded".to_string()
                    } else {
                        "on_target".to_string()
                    }
                };

                let w = &profile.target_weights_bps;
                // An empty portfolio has no weights: 0/0 is never computed.
                let weight = |usd: f64| -> i32 {
                    if total_portfolio_usd > 0.0 { ((usd / total_portfolio_usd) * 10000.0) as i32 } else { 0 }
                };
                let t3_weight = weight(t3_usd);
                let t3_dev = t3_weight - w.treasury;

                let lp_weight = weight(lp_usd);
                let lp_dev = lp_weight - w.uniswap_lp;

                let usdc_weight = weight(wallet_usdc);
                let usdc_dev = usdc_weight - w.cash;

                // Gas has no target: shown as a measured share, never under pressure.
                let gas_weight = weight(wallet_gas_usd);
                // Nor has AERO (#93): the fourth pillar is measured, never steered.
                let aero_weight = weight(wallet_aero_usd);

                vec![
                    AllocationDeviation {
                        bucket_name: "Sovereign Treasury".to_string(),
                        target_weight_bps: w.treasury,
                        actual_weight_bps: t3_weight,
                        deviation_bps: t3_dev,
                        status: calc_status(t3_dev, w.treasury),
                    },
                    AllocationDeviation {
                        bucket_name: "LP (all tracked positions)".to_string(),
                        target_weight_bps: w.uniswap_lp,
                        actual_weight_bps: lp_weight,
                        deviation_bps: lp_dev,
                        status: calc_status(lp_dev, w.uniswap_lp),
                    },
                    AllocationDeviation {
                        bucket_name: "Liquid USDC (Escrow/Vault)".to_string(),
                        target_weight_bps: w.cash,
                        actual_weight_bps: usdc_weight,
                        deviation_bps: usdc_dev,
                        status: calc_status(usdc_dev, w.cash),
                    },
                    AllocationDeviation {
                        bucket_name: "Gas (wallet ETH+WETH)".to_string(),
                        target_weight_bps: 0,
                        actual_weight_bps: gas_weight,
                        deviation_bps: gas_weight,
                        status: "no_target".to_string(),
                    },
                    AllocationDeviation {
                        bucket_name: AERO_BUCKET_NAME.to_string(),
                        target_weight_bps: 0,
                        actual_weight_bps: aero_weight,
                        deviation_bps: aero_weight,
                        status: "no_target".to_string(),
                    },
                ]
            },
        },
        structural_channels,
        strategy_counterfactual: compute_counterfactual_sync(
            counterfactual_history,
            total_portfolio_usd,
            total_native_usd,
            &now_str,
        ),
        sovereign_ledger: SovereignLedger {
            cumulative_harvested_usd: format!("{:.2}", ledger.total_usd),
            tax_reserve_usd: format!("{:.2}", ledger.total_usd * profile.tax_reserve_ratio),
            growth_dry_powder_usd: format!("{:.2}", ledger.total_usd * (1.0 - profile.tax_reserve_ratio)),
            harvest_count: ledger.harvest_count,
            last_harvest_at: ledger.last_at.map(|dt| dt.to_rfc3339()),
            tax_reserve_ratio: format!("{:.2}", profile.tax_reserve_ratio),
            unpriced_count: ledger.unpriced_count,
        },
        accumulation_plan: plan_status,
        lp_rebuild,
        // Filled in by the ingestion cycle from the accumulation ledger (cost_basis.rs);
        // this function reads no ledger rows.
        cost_basis: None,
        velocity_gas_sandbox: VelocityGasSandbox {
            contracts: fee_entries
                .iter()
                .map(|(addr, name, fee_usd, lp_claimable, note, (earnings_kind, reward, lp_fees_note))| FeeCollectionIndex {
                    contract_address: addr.clone(),
                    contract_name: name.clone(),
                    claimable_usd: format!("{:.2}", fee_usd),
                    gas_break_even_usd: format!("{:.4}", gas_cost),
                    efficiency_ratio: if gas_cost > 0.0 {
                        (fee_usd / gas_cost * 100.0).round() / 100.0
                    } else {
                        0.0
                    },
                    // A reverting claimFees() is not "$0 earned, accumulate" (#93).
                    recommended_action: if *lp_claimable {
                        recommended_claim_action(*fee_usd, gas_cost).to_string()
                    } else {
                        "not_claimable".to_string()
                    },
                    note: note.clone(),
                    earnings_kind: earnings_kind.clone(),
                    reward: reward.clone(),
                    lp_fees_note: lp_fees_note.clone(),
                })
                .collect(),
        },
    }
}
