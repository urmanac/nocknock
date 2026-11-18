use crate::*;

/// How far NOCK must rise against the pool's other token before the range has sold all
/// of its NOCK. NOCK as token1 is exhausted at the lower tick, as token0 at the upper.
/// Pure.
pub(crate) fn nock_sell_out_multiple(p: &RawPosition) -> Option<f64> {
    if is_full_range(p.tick_lower, p.tick_upper) {
        return None; // sells gradually forever; never empties
    }
    let ticks = if is_nock_token(&p.token1) {
        p.tick - p.tick_lower
    } else if is_nock_token(&p.token0) {
        p.tick_upper - p.tick
    } else {
        return None;
    };
    if ticks <= 0 {
        return None; // already past the edge: the range holds no NOCK
    }
    Some(1.0001f64.powi(ticks))
}

/// Smallest deposit worth suggesting. Below this a suggestion is noise (a $2 top-up
/// costs attention and gas for nothing) and the position is listed as not fundable.
pub(crate) const LP_SUGGESTION_MIN_USD: f64 = 5.0;

/// Why a position with an ETH or WETH leg is never filled (issue #94): wallet ETH and
/// WETH are gas, counted in the total but never deployed. Its share goes to the others.
pub(crate) const LP_GAS_LEG_REASON: &str =
    "needs ETH/WETH, which is gas, not investable capital: NockNock never funds an ETH leg from wallet gas, so this position's share goes to the other eligible positions";

/// Why an Aero CL position is never sized for a rebuild deposit (issue #91). Observed
/// 2026-11-16: the Aero app's deposit link for a CL pool starts a NEW custom-range
/// deposit, which mints another NFT that NockNock does not track; and adding liquidity
/// to an existing NFT (staked in its gauge or not) has not been verified. A suggestion
/// to "add to NFT #N" would send capital out of the tracked totals and the floor reading.
pub(crate) const LP_AERO_CL_REASON: &str =
    "Aero CL: not sized. The Aero app's deposit link opens a new position (a different NFT, which NockNock would not track), and adding to an existing NFT is not verified";

/// Human amounts of token0/token1 that one USD of value buys in this position's range
/// at the pool's current price. None when the range has no value at this price or a
/// leg cannot be priced. Pure.
pub(crate) fn amounts_per_usd(p: &RawPosition, nock_usd: f64, eth_usd: f64) -> Option<(f64, f64)> {
    // Any liquidity works as the probe: amounts are linear in L.
    let (r0, r1) = concentrated_amounts_raw(1_000_000_000_000_000_000, p.sqrt_price_x96, p.tick_lower, p.tick_upper);
    let a0 = r0 / 10f64.powi(p.token0.decimals);
    let a1 = r1 / 10f64.powi(p.token1.decimals);
    let p0 = leg_usd_price(&p.token0, nock_usd, eth_usd);
    let p1 = leg_usd_price(&p.token1, nock_usd, eth_usd);
    if (a0 > 0.0 && p0 <= 0.0) || (a1 > 0.0 && p1 <= 0.0) {
        return None;
    }
    let value = a0 * p0 + a1 * p1;
    if !(value > 0.0) || !value.is_finite() {
        return None;
    }
    Some((a0 / value, a1 / value))
}

/// Size the LP rebuild: split the LP target across eligible tracked positions and work
/// out what each deposit takes. Pure — no I/O, unit-testable.
///
/// Eligibility: the pool must have in-range liquidity (a dead pool's price is frozen,
/// so a deposit there would sit alone at a stale price), and the position must be in
/// range unless its config makes an exception, and it must not need ETH/WETH: wallet
/// ETH is gas, not investable capital (issue #94), so a position with an ETH or WETH
/// leg is excluded and its share is redistributed to the eligible positions. An Aero CL
/// position is excluded whatever its pair (LP_AERO_CL_REASON, issue #91). Funding:
/// a suggestion is scaled down, never up, to what the wallet can supply; the limit is
/// named.
pub(crate) fn lp_rebuild_suggestions(
    raw: &RawOnChainData,
    lp_target_usd: f64,
    funds: RebuildFunds,
) -> (Vec<LpSuggestion>, Vec<LpExclusion>) {
    let nock_usd = raw.spot_price_nock_usd;
    let eth_usd = raw.live_eth_price_usd;
    let mut excluded = Vec::new();
    let mut eligible: Vec<(&RawPosition, Option<u32>)> = Vec::new();
    for p in &raw.positions {
        // A range with no NOCK leg (a gas-side ETH/USDC range, #151) is not NOCK LP:
        // measured, never steered. It is neither a template nor an exclusion.
        if !is_nock_pair(p) {
            continue;
        }
        let rule = raw.rebuild_rules.get(&p.position_id).cloned().unwrap_or_default();
        let mut skip = |reason: &str| {
            excluded.push(LpExclusion {
                position_id: p.position_id.clone(),
                nft_id: p.nft_id,
                label: position_label(p),
                reason: reason.to_string(),
            })
        };
        if rule.weight_bps == Some(0) {
            skip("excluded in config (rebuild_weight_bps: 0)");
            continue;
        }
        match raw.pool_liquidity.get(&p.position_id).and_then(|v| v.parse::<u128>().ok()) {
            None => {
                skip("pool liquidity could not be read this cycle");
                continue;
            }
            Some(0) => {
                skip("pool has no in-range liquidity, so its price is frozen; a deposit would sit alone at a stale price");
                continue;
            }
            Some(_) => {}
        }
        if !p.in_range && !rule.when_out_of_range {
            skip("position is out of range (out-of-range positions are not filled by default)");
            continue;
        }
        if is_eth_token(&p.token0) || is_eth_token(&p.token1) {
            skip(LP_GAS_LEG_REASON);
            continue;
        }
        // After the gas rule, which applies to every protocol and is the more basic
        // reason for a WETH pair: an Aero CL position without an ETH leg is still not
        // sized, because no verified way to add to its NFT exists (issue #91).
        if Protocol::parse(&p.protocol) == Some(Protocol::AeroCl) {
            skip(LP_AERO_CL_REASON);
            continue;
        }
        eligible.push((p, rule.weight_bps));
    }

    // Weights: explicit bps where given; positions without one share what is left equally.
    // When every eligible position has an explicit weight and they sum to less than the
    // whole, they are scaled up in proportion: the share of an excluded position (an
    // ETH/WETH one, a dead pool) goes to the eligible ones, so the LP target can be met.
    let explicit: u32 = eligible.iter().filter_map(|(_, w)| *w).sum();
    let unweighted = eligible.iter().filter(|(_, w)| w.is_none()).count() as f64;
    let leftover = (10000.0 - explicit as f64).max(0.0);
    let explicit_scale = if unweighted == 0.0 && explicit > 0 && explicit < 10000 {
        10000.0 / explicit as f64
    } else {
        1.0
    };

    // Pass 1: what each position wants, and what that takes of each token.
    struct Want<'a> {
        p: &'a RawPosition,
        target_usd: f64,
        current_usd: f64,
        want_usd: f64,
        per0: f64,
        per1: f64,
    }
    let mut wants: Vec<Want> = Vec::new();
    for (p, w) in eligible {
        let weight = match w {
            Some(bps) => bps as f64 * explicit_scale,
            None => leftover / unweighted,
        } / 10000.0;
        let p0 = leg_usd_price(&p.token0, nock_usd, eth_usd);
        let p1 = leg_usd_price(&p.token1, nock_usd, eth_usd);
        let current_usd = p.amount0 * p0 + p.amount1 * p1;
        let target_usd = lp_target_usd * weight;
        let want_usd = target_usd - current_usd;
        if want_usd < LP_SUGGESTION_MIN_USD {
            continue; // at, above or within dust of its share: never suggest unwinding
        }
        match amounts_per_usd(p, nock_usd, eth_usd) {
            Some((per0, per1)) => wants.push(Want { p, target_usd, current_usd, want_usd, per0, per1 }),
            None => excluded.push(LpExclusion {
                position_id: p.position_id.clone(),
                nft_id: p.nft_id,
                label: position_label(p),
                reason: "the range holds no value at the current price, or a leg cannot be priced".to_string(),
            }),
        }
    }

    // Never suggest more than the aggregate gap. Positions already above their share
    // are left alone (no unwinding), so the others' shares can add up to more than
    // what LP as a whole is short by; scale them down together so a completed rebuild
    // lands on the LP target instead of overshooting it.
    // NOCK LP only (#151), the same sum the rebuild status was judged on.
    let lp_now: f64 = raw
        .positions
        .iter()
        .filter(|p| is_nock_pair(p))
        .map(|p| {
            p.amount0 * leg_usd_price(&p.token0, nock_usd, eth_usd)
                + p.amount1 * leg_usd_price(&p.token1, nock_usd, eth_usd)
        })
        .sum();
    let lp_gap = (lp_target_usd - lp_now).max(0.0);
    let total_want: f64 = wants.iter().map(|w| w.want_usd).sum();
    if total_want > lp_gap && total_want > 0.0 {
        let k = lp_gap / total_want;
        for w in wants.iter_mut() {
            w.want_usd *= k;
            // Report the share actually being filled, so the client does not show
            // "$128 of $162 wanted" for a deposit that is exactly what is wanted.
            w.target_usd = w.current_usd + w.want_usd;
        }
    }

    // Pass 2: total demand per funding source, and how far each must be scaled back.
    // Every position drawing on a short source is scaled by the same factor, so the
    // outcome does not depend on the order positions are listed in config.
    #[derive(Clone, Copy, PartialEq)]
    enum Src {
        Nock,
        Usdc,
        Unfundable,
    }
    // ETH/WETH legs never reach here (excluded at eligibility): gas is not a source.
    let src_of = |t: &PositionTokenInfo| {
        if is_nock_token(t) {
            Src::Nock
        } else if is_usdc_token(t) {
            Src::Usdc
        } else {
            Src::Unfundable
        }
    };
    let (mut need_nock, mut need_usdc) = (0.0f64, 0.0f64);
    for w in &wants {
        for (tok, per) in [(&w.p.token0, w.per0), (&w.p.token1, w.per1)] {
            let amt = per * w.want_usd;
            match src_of(tok) {
                Src::Nock => need_nock += amt,
                Src::Usdc => need_usdc += amt,
                Src::Unfundable => {}
            }
        }
    }
    let factor = |avail: f64, need: f64| if need > 0.0 { (avail.max(0.0) / need).min(1.0) } else { 1.0 };
    // (scale factor, plain-words reason) per source.
    let nock_f = (
        factor(funds.nock, need_nock),
        if !funds.floor_set {
            format!("only {:.0} NOCK is in the wallet", funds.nock.max(0.0))
        } else if funds.floor_tolerance_nock > 0.0 {
            format!(
                "only {:.0} NOCK may go into LP: what sits above the hard floor plus the {:.0} tolerated below it (a range sells its NOCK as price rises)",
                funds.nock.max(0.0),
                funds.floor_tolerance_nock
            )
        } else {
            format!(
                "only {:.0} NOCK sits above the hard floor (a range can sell its NOCK as price rises, so floor NOCK is never placed in LP)",
                funds.nock.max(0.0)
            )
        },
    );
    let usdc_f = (
        factor(funds.usdc, need_usdc),
        format!(
            "only ${:.2} of wallet USDC is free after the cash target, tax reserve and remaining DCA tranches",
            funds.usdc.max(0.0)
        ),
    );
    let mut suggestions = Vec::new();
    for w in wants {
        let p = w.p;
        let mut scale = 1.0f64;
        let mut limited_by: Option<String> = None;
        for (tok, per) in [(&p.token0, w.per0), (&p.token1, w.per1)] {
            if per <= 0.0 {
                continue;
            }
            let (f, why) = match src_of(tok) {
                Src::Nock => (nock_f.0, nock_f.1.clone()),
                Src::Usdc => (usdc_f.0, usdc_f.1.clone()),
                Src::Unfundable => (0.0, format!("{} is not a token the engine can fund", tok.symbol)),
            };
            if f < scale - 1e-9 {
                scale = f;
                limited_by = Some(why);
            }
        }
        let add_usd = w.want_usd * scale;
        if add_usd < LP_SUGGESTION_MIN_USD {
            excluded.push(LpExclusion {
                position_id: p.position_id.clone(),
                nft_id: p.nft_id,
                label: position_label(p),
                reason: format!("cannot be funded: {}", limited_by.unwrap_or_else(|| "amount below $1".to_string())),
            });
            continue;
        }
        let (amt0, amt1) = (w.per0 * add_usd, w.per1 * add_usd);
        let fmt_amt = |tok: &PositionTokenInfo, amt: f64| -> String {
            if is_eth_token(tok) {
                format!("{:.6}", amt)
            } else if is_usdc_token(tok) {
                format!("{:.2}", amt)
            } else {
                format!("{:.0}", amt)
            }
        };
        suggestions.push(LpSuggestion {
            position_id: p.position_id.clone(),
            nft_id: p.nft_id,
            protocol: p.protocol.clone(),
            label: position_label(p),
            target_usd: format!("{:.2}", w.target_usd),
            current_usd: format!("{:.2}", w.current_usd),
            add_usd: format!("{:.2}", add_usd),
            token0_symbol: p.token0.symbol.clone(),
            amount0: fmt_amt(&p.token0, amt0),
            token1_symbol: p.token1.symbol.clone(),
            amount1: fmt_amt(&p.token1, amt1),
            limited_by,
            nock_sold_out_at_multiple: nock_sell_out_multiple(p).map(|m| format!("{:.2}", m)),
            url: deposit_url_for(p, &raw.rebuild_rules.get(&p.position_id).cloned().unwrap_or_default()),
        });
    }
    (suggestions, excluded)
}

/// LP weight vs target, with the same tolerance band as the deviation matrix.
pub(crate) fn lp_rebuild_status(lp_usd: f64, total_portfolio_usd: f64, target_weight_bps: i32) -> LpRebuild {
    let target_usd = total_portfolio_usd * (target_weight_bps as f64 / 10000.0);
    let weight_bps = if total_portfolio_usd > 0.0 {
        ((lp_usd / total_portfolio_usd) * 10000.0) as i32
    } else {
        0
    };
    let dev = weight_bps - target_weight_bps;
    let tol = (target_weight_bps as f64 * 0.20).max(100.0) as i32;
    // No LP target set: nothing to measure against, so no rebuild is ever suggested.
    let status = if target_weight_bps <= 0 {
        "no_target"
    } else if total_portfolio_usd <= 0.0 {
        "on_target"
    } else if dev < -tol {
        "under_target"
    } else if dev > tol {
        "over_target"
    } else {
        "on_target"
    };
    LpRebuild {
        status: status.to_string(),
        target_usd: format!("{:.2}", target_usd),
        actual_usd: format!("{:.2}", lp_usd),
        gap_usd: format!("{:.2}", (target_usd - lp_usd).max(0.0)),
        target_weight_bps,
        suggestions: Vec::new(),
        excluded: Vec::new(),
        floor_tolerance_nock: None,
        worst_case_nock: None,
    }
}

/// One-line summary of the tracked LP positions, built from the positions themselves
/// (issue #95; a port of the frontend `positionsSummary`). A pool reading zero in-range
/// liquidity counts as stale; a pool whose read failed is not called stale. It claims
/// nothing about yield or profit and loss.
///
/// The flag is true only when at least one ranged position holds liquidity, is in
/// range, and sits on a pool read live this cycle: the only case in which the old
/// "(Dual Growth Engine Active)" label is a statement of fact.
///
/// Only NOCK-pair positions are the NOCK engine (#151). Ranges with no NOCK leg (gas-side
/// ETH/USDC ranges) are named in a closing clause as measured only, and never make the
/// engine active.
pub(crate) fn lp_positions_summary(raw: &RawOnChainData) -> (String, bool) {
    let pool_active = |p: &RawPosition| -> Option<bool> {
        raw.pool_liquidity
            .get(&p.position_id)
            .and_then(|v| v.parse::<u128>().ok())
            .map(|l| l > 0)
    };
    let nock: Vec<&RawPosition> = raw.positions.iter().filter(|p| is_nock_pair(p)).collect();
    let other: Vec<&RawPosition> = raw.positions.iter().filter(|p| !is_nock_pair(p)).collect();
    let other_clause = if other.is_empty() {
        String::new()
    } else {
        let m = other.len();
        let all_eth_usdc = other.iter().all(|p| {
            (is_eth_token(&p.token0) && is_usdc_token(&p.token1)) || (is_usdc_token(&p.token0) && is_eth_token(&p.token1))
        });
        let in_range = other
            .iter()
            .filter(|p| p.in_range && p.liquidity > 0 && pool_active(p) != Some(false))
            .count();
        format!(
            " +{} {} range{}, measured only ({} in range).",
            m,
            if all_eth_usdc { "ETH/USDC" } else { "non-NOCK" },
            if m == 1 { "" } else { "s" },
            in_range
        )
    };
    let n = nock.len();
    if n == 0 {
        if other.is_empty() {
            return ("No LP positions tracked.".to_string(), false);
        }
        return (format!("No NOCK LP positions tracked.{}", other_clause), false);
    }
    let full = nock.iter().filter(|p| is_full_range(p.tick_lower, p.tick_upper)).count();
    let ranged: Vec<&RawPosition> = nock.iter().copied().filter(|p| !is_full_range(p.tick_lower, p.tick_upper)).collect();
    // A ranged position holding no liquidity is empty: neither in nor out of range.
    let empty = ranged.iter().filter(|p| p.liquidity == 0).count();
    let held: Vec<&&RawPosition> = ranged.iter().filter(|p| p.liquidity > 0).collect();
    let stale = held.iter().filter(|p| pool_active(p) == Some(false)).count();
    let in_range = held.iter().filter(|p| p.in_range && pool_active(p) != Some(false)).count();
    let out = held.len() - in_range - stale;
    // A failed pool read (None) does not disprove activity: the same rule the in-range
    // count above uses, so one RPC hiccup cannot flip the HOLD label.
    let engine_active = ranged
        .iter()
        .any(|p| p.in_range && p.liquidity > 0 && pool_active(p) != Some(false));
    let mut parts: Vec<String> = Vec::new();
    if !ranged.is_empty() {
        let mut detail = vec![format!("{} in range", in_range)];
        if out > 0 {
            detail.push(format!("{} out of range", out));
        }
        if stale > 0 {
            detail.push(format!("{} on a stale pool", stale));
        }
        if empty > 0 {
            detail.push(format!("{} empty", empty));
        }
        parts.push(format!("{} ranged ({})", ranged.len(), detail.join(", ")));
    }
    if full > 0 {
        parts.push(format!("{} full range", full));
    }
    (
        format!(
            "{} {}LP position{} tracked: {}.{}",
            n,
            if other.is_empty() { "" } else { "NOCK " },
            if n == 1 { "" } else { "s" },
            parts.join(", "),
            other_clause
        ),
        engine_active,
    )
}

/// Standing bias (operator order, 2026-11-14): rebuilding LP tranches leaves the net
/// cash position unchanged, so it is always worth doing. When LP is under target the
/// rebuild becomes the primary directive whenever the engine would otherwise say
/// "nothing to do", and is appended to every other directive. It never outranks a due
/// tranche or a safety directive, so those transitions still notify.
pub(crate) fn apply_lp_rebuild_bias(action: String, details: String, lp: &LpRebuild) -> (String, String) {
    if lp.status != "under_target" {
        return (action, details);
    }
    let sentence = format!(
        "NOCK LP holds ${} against a ${} target ({:.0}% of portfolio), ${} short. Re-establishing ranges from NOCK and USDC already held is a reposition, not a purchase: the stable total is unchanged, only its location moves.",
        lp.actual_usd,
        lp.target_usd,
        lp.target_weight_bps as f64 / 100.0,
        lp.gap_usd
    );
    let sentence = if lp.suggestions.is_empty() {
        format!("{} No tracked position is eligible for a deposit right now.", sentence)
    } else {
        let steps: Vec<String> = lp
            .suggestions
            .iter()
            .map(|sg| {
                let mut legs = Vec::new();
                if sg.amount0.parse::<f64>().map_or(false, |a| a > 0.0) {
                    legs.push(format!("{} {}", sg.amount0, sg.token0_symbol));
                }
                if sg.amount1.parse::<f64>().map_or(false, |a| a > 0.0) {
                    legs.push(format!("{} {}", sg.amount1, sg.token1_symbol));
                }
                format!(
                    "add {} (${}) to {}{}",
                    legs.join(" + "),
                    sg.add_usd,
                    sg.label,
                    sg.nock_sold_out_at_multiple
                        .as_ref()
                        .map(|m| format!(" (sells all its NOCK by a {}x rise)", m))
                        .unwrap_or_default()
                )
            })
            .collect();
        let worst = lp
            .worst_case_nock
            .as_ref()
            .map(|w| format!(" If every range sold out, {} NOCK would remain.", w))
            .unwrap_or_default();
        format!("{} Suggested: {}.{}", sentence, steps.join("; "), worst)
    };
    // With nothing fundable the note is informational only: it must not replace the
    // directive, or "rebuild" would sit on top permanently with no step to take.
    if lp.suggestions.is_empty() {
        return (action, format!("{} Standing bias: {}", details, sentence));
    }
    let idle = action.starts_with("HOLD & ACCRETE")
        || action.starts_with("DIP ZONE")
        || action.starts_with("DCA ON SCHEDULE")
        || action.starts_with("DCA PLAN PENDING")
        || action.starts_with("DCA PAUSED")
        || action.starts_with("DCA BLOCKED");
    if idle {
        // The dip-zone crossing must still change the action (and so notify): the rebuild
        // carries it rather than swallowing it.
        let rebuild = if action.starts_with("DIP ZONE") {
            format!("REBUILD LP TRANCHES (No Net Cash Change) · {}", action)
        } else {
            "REBUILD LP TRANCHES (No Net Cash Change)".to_string()
        };
        (
            rebuild,
            format!("{} Meanwhile: {} — {}", sentence, action, details),
        )
    } else {
        (action, format!("{} Standing bias: {}", details, sentence))
    }
}
