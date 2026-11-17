//! Collectible gauge rewards in the MCP text (issue #142).
//!
//! One source file for both MCP transports: the worker's HTTP endpoint (`mod reward_text`
//! in main.rs) and the stdio binary, which cannot import from main.rs and includes this
//! file by path (src/bin/mcp.rs). It depends on nothing but serde_json and closed_text.rs,
//! which both binaries compile at their crate root.
//!
//! The text states what is collectible and where it is collected. It never says to
//! claim, sell, hold or accumulate AERO, and it never prints a figure the snapshot does
//! not carry: a reward the worker could not read reads "not read" with its reason, and a
//! total that could not be formed reads "not totalled" with its reason. Neither reads as
//! 0. A snapshot that states no collectible rewards at all (one from before #142, which
//! the worker serves for the first minutes after an upgrade) gets no reward line: its
//! text is what it was, with the unclaimed AERO it does carry on the AERO lines.

use serde_json::Value;

/// What a line says for a reward figure the snapshot does not carry. Only reachable for
/// a total missing beside a stated rewards figure, which the worker never produces.
pub const REWARD_NOT_REPORTED: &str = "not reported";

fn str_field<'a>(v: &'a Value, key: &str) -> Option<&'a str> {
    v.get(key).and_then(|x| x.as_str()).filter(|x| !x.trim().is_empty())
}

fn global_str<'a>(data: &'a Value, key: &str) -> Option<&'a str> {
    data.pointer("/derived_metrics/global_status").and_then(|g| str_field(g, key))
}

/// Where a reward is collected: "collectible from gauge 0x… in the Aero app", or
/// without the gauge when the snapshot names none. A read zero is not described as
/// collectible: "nothing to collect in gauge 0x…".
pub fn reward_where_text(reward: &Value) -> String {
    let app = str_field(reward, "collected_in").unwrap_or("the Aero app");
    match (reward_is_zero(reward), str_field(reward, "gauge")) {
        (true, Some(gauge)) => format!("nothing to collect in gauge {gauge}"),
        (true, None) => "nothing to collect".to_string(),
        (false, Some(gauge)) => format!("collectible from gauge {gauge} in {app}"),
        (false, None) => format!("collectible in {app}"),
    }
}

/// "17.654015 AERO ($7.75)", or with "(USD value unavailable)" for an unpriced reward.
/// None when the reward was not read: there is no amount to print.
pub fn reward_amount_text(reward: &Value) -> Option<String> {
    let amount = str_field(reward, "amount")?;
    let symbol = str_field(reward, "token_symbol").unwrap_or("AERO");
    Some(match str_field(reward, "usd") {
        Some(usd) => format!("{amount} {symbol} (${usd})"),
        None => format!("{amount} {symbol} (USD value unavailable)"),
    })
}

/// One reward as a statement of fact: the amount and where it is collected, that there
/// is nothing to collect (a read zero), or that it was not read and why.
pub fn reward_statement(reward: &Value) -> String {
    match reward_amount_text(reward) {
        Some(amount) => format!("{amount}, {}", reward_where_text(reward)),
        None => format!("not read ({})", str_field(reward, "note").unwrap_or("no reason recorded")),
    }
}

/// True when the reward was read and is exactly zero: there is nothing to collect.
pub fn reward_is_zero(reward: &Value) -> bool {
    str_field(reward, "amount").and_then(|a| a.trim().parse::<f64>().ok()).is_some_and(|a| a == 0.0)
}

/// The positions that carry a structured reward (read or not).
fn reward_positions(data: &Value) -> usize {
    data.get("positions")
        .and_then(|v| v.as_array())
        .map(|ps| ps.iter().filter(|p| p.get("collectible_reward").is_some_and(|r| r.is_object())).count())
        .unwrap_or(0)
}

/// The collectible AERO rewards total, in words. None when the snapshot states neither
/// a figure nor the reason there is none: the caller then prints no reward line. A live
/// cycle always states one of the two. A snapshot from before #142 states neither, also
/// after the worker has read it from the database and serialized it again (the keys are
/// then present and null).
/// - a figure: "$7.84 USD (17.864030 AERO in tracked gauges, collected in the Aero app)";
/// - every gauge answered zero: "$0.00 USD (0.000000 AERO in tracked gauges: nothing to collect)";
/// - no staked position: "none (no staked Aero position is tracked)";
/// - not totalled this cycle: "not totalled (reason)", never a partial sum.
pub fn collectible_rewards_text(data: &Value) -> Option<String> {
    let zero = |s: &str| s.trim().parse::<f64>().is_ok_and(|a| a == 0.0);
    Some(match global_str(data, "collectible_rewards_usd") {
        Some(_) if reward_positions(data) == 0 => "none (no staked Aero position is tracked)".to_string(),
        Some(usd) => match global_str(data, "unclaimed_aero_amount") {
            Some(amount) if zero(amount) => format!("${usd} USD ({amount} AERO in tracked gauges: nothing to collect)"),
            Some(amount) => format!("${usd} USD ({amount} AERO in tracked gauges, collected in the Aero app)"),
            None => format!("${usd} USD (in tracked gauges, collected in the Aero app)"),
        },
        None => format!("not totalled ({})", global_str(data, "collectible_rewards_note")?),
    })
}

/// The total collectible figure (LP fees plus AERO rewards), in words.
pub fn total_collectible_text(data: &Value) -> String {
    match global_str(data, "total_collectible_usd") {
        Some(usd) => format!("${usd} USD"),
        None => match global_str(data, "collectible_rewards_note") {
            Some(_) => "not totalled (the AERO rewards part is not totalled)".to_string(),
            None => REWARD_NOT_REPORTED.to_string(),
        },
    }
}

/// The two labelled lines of a text that also prints the LP fees: the rewards and the
/// total, each ending in a newline. Empty when the snapshot states no rewards.
fn rewards_and_total_lines(data: &Value, total_label: &str) -> String {
    match collectible_rewards_text(data) {
        Some(rewards) => {
            format!("• Collectible AERO rewards: {rewards}\n• {total_label}: {}\n", total_collectible_text(data))
        }
        None => String::new(),
    }
}

/// The two labelled lines `get_shot_advice` prints beside "Uncollected Claimable Fees"
/// (which stays LP fees only). Each line ends in a newline, so the caller places the
/// block directly before its next line. Empty when the snapshot states no rewards.
pub fn collectible_advice_lines(data: &Value) -> String {
    rewards_and_total_lines(data, "Total collectible (LP fees + AERO rewards)")
}

/// The one line `get_portfolio_state` prints after the AERO lines: both parts, labelled,
/// and their sum. None of it is in the portfolio total. The line ends in a newline;
/// empty when the snapshot states no rewards.
pub fn collectible_earnings_line(data: &Value) -> String {
    let rewards = match (global_str(data, "collectible_rewards_usd"), global_str(data, "collectible_rewards_note")) {
        (Some(usd), _) => format!("${usd}"),
        (None, Some(note)) => format!("not totalled ({note})"),
        (None, None) => return String::new(),
    };
    let lp = global_str(data, "total_fees_claimable_usd")
        .map(|v| format!("${v}"))
        .unwrap_or_else(|| "unavailable".to_string());
    let total = match global_str(data, "total_collectible_usd") {
        Some(usd) => format!("; total collectible ${usd}"),
        None => String::new(),
    };
    format!("• Collectible earnings (not counted in the total): LP fees {lp}; AERO rewards {rewards}{total}\n")
}

/// What is known about the gas of collecting one reward.
// The stdio binary compiles this file too and never makes a live estimate, so it
// constructs only `NotRequested`.
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq)]
pub enum RewardGas {
    /// No live estimate was made (the text was built from a stored snapshot alone).
    NotRequested,
    /// The claim call is not verified on-chain for this gauge, so no figure is shown.
    NotVerified,
    /// `eth_estimateGas` of the verified claim call, sent from the tracked wallet.
    Estimated { units: u128, usd: Option<f64> },
    /// The estimate was asked for and did not come back; the reason, in words.
    Failed(String),
}

/// The gas part of a reward row. A figure appears only for `Estimated`.
pub fn reward_gas_text(gas: &RewardGas, nothing_to_collect: bool) -> String {
    if nothing_to_collect {
        return "nothing to collect, gas not estimated".to_string();
    }
    match gas {
        RewardGas::NotRequested => "gas not estimated (no live estimate was made for this text)".to_string(),
        RewardGas::NotVerified => "gas not estimated (claim call not verified for this gauge)".to_string(),
        RewardGas::Estimated { units, usd: Some(usd) } => format!("gas est. {units} units (~${usd:.4})"),
        RewardGas::Estimated { units, usd: None } => format!("gas est. {units} units (USD value unavailable)"),
        RewardGas::Failed(why) => format!("gas not estimated ({why})"),
    }
}

/// One reward for a row of the claim simulation: the statement, then the gas statement
/// when there is an amount to collect. A reward that was not read, and a read zero
/// (whose statement already says there is nothing to collect), carry no gas statement.
fn reward_row_text(reward: &Value, gas: &RewardGas) -> String {
    let statement = reward_statement(reward);
    if reward_amount_text(reward).is_none() || reward_is_zero(reward) {
        return statement;
    }
    format!("{statement} | {}", reward_gas_text(gas, false))
}

/// The reason alone why a row's LP fees are not LP-claimable: the worker's own
/// `lp_fees_note`, or (a snapshot from before #142) the part of `amount` before the
/// AERO statement.
fn lp_fees_reason<'a>(fee: &'a Value, amount: &'a str) -> &'a str {
    str_field(fee, "lp_fees_note").unwrap_or_else(|| amount.split("; ").next().unwrap_or(amount))
}

/// `simulate_claim_fees`, for both transports. `reward_gas[i]` belongs to
/// `contract_fees[i]`; a missing entry is `NotRequested`.
///
/// LP fees and gauge rewards are never merged into one figure: "Collectible LP fees",
/// "Collectible AERO rewards" and "Total collectible" are three labelled lines. The
/// "Estimated Gas" and "Net Accretion if Harvested" lines keep their wording and their
/// meaning (LP fee claims only) and sit directly under the LP fees they belong to.
/// A snapshot that states no rewards (one from before #142) gets the LP lines alone.
pub fn claim_simulation_text(data: &Value, reward_gas: &[RewardGas]) -> String {
    let fees = data.get("contract_fees").and_then(|f| f.as_array());
    let mut details = String::new();
    let mut total_claimable_usd = 0.0f64;
    let mut gas_usd: Option<f64> = None;
    let (mut claimable_rows, mut not_claimable_rows) = (0usize, 0usize);

    for (i, fee) in fees.into_iter().flatten().enumerate() {
        let symbol = fee.get("fee_token_symbol").and_then(|s| s.as_str()).unwrap_or("Pool");
        let amount = fee.get("amount").and_then(|a| a.as_str()).unwrap_or("0");
        let amount_usd = fee.get("amount_usd").and_then(|a| a.as_str()).unwrap_or("0.00");
        let gas_str = fee.get("gas_break_even_usd").and_then(|g| g.as_str()).unwrap_or_default();
        let reward = fee.get("reward").filter(|r| r.is_object());
        let row_gas = reward_gas.get(i).unwrap_or(&RewardGas::NotRequested);
        if amount.starts_with("not LP-claimable") {
            // No LP fee figure to print: an Aero pool whose claimFees() reverts, or an
            // Aero CL position staked in its gauge. Its earnings are the gauge reward,
            // stated first; the worker's own reason for the LP fees follows. A snapshot
            // without the structured reward prints the worker's `amount` as it always did.
            match reward {
                Some(r) => details.push_str(&format!(
                    "  - {symbol}: reward {}; LP fees {}\n",
                    reward_row_text(r, row_gas),
                    lp_fees_reason(fee, amount)
                )),
                None => details.push_str(&format!("  - {symbol}: {amount}\n")),
            }
            not_claimable_rows += 1;
            continue;
        }
        claimable_rows += 1;
        match gas_str.parse::<f64>() {
            Ok(g) => {
                // Per-contract break-evens come from the live gas query; take the max
                // as the representative claim cost. Never fabricate a constant.
                gas_usd = Some(match gas_usd {
                    Some(acc) => acc.max(g),
                    None => g,
                });
                details.push_str(&format!("  - {symbol}: {amount} (~${amount_usd} USD | Gas Est: ${:.4})\n", g));
            }
            Err(_) => {
                details.push_str(&format!("  - {symbol}: {amount} (~${amount_usd} USD | Gas Est: unavailable)\n"));
            }
        }
        if let Ok(usd_val) = amount_usd.parse::<f64>() {
            total_claimable_usd += usd_val;
        }
        // A position with LP-claimable fees and a gauge reward: the reward on its own line.
        if let Some(r) = reward {
            details.push_str(&format!("      gauge reward: {}\n", reward_row_text(r, row_gas)));
        }
    }

    let (gas_line, net_line) = match gas_usd {
        Some(g) => (
            format!("~${:.4} USD", g),
            format!("${:.4} USD", (total_claimable_usd - g).max(0.0)),
        ),
        // Every row is one the LP cannot claim: there is no claim to cost, which is
        // not the same as a gas estimate that could not be read.
        None if claimable_rows == 0 && not_claimable_rows > 0 => (
            "n/a (nothing LP-claimable)".to_string(),
            "n/a (nothing LP-claimable)".to_string(),
        ),
        None => (
            "unavailable".to_string(),
            "unknown (live gas estimate unavailable)".to_string(),
        ),
    };
    format!(
        "⚡ Fee Claim Simulation\n\
         =======================\n\
         • Collectible LP fees: ${:.4} USD\n\
         • Estimated Gas: {}\n\
         • Net Accretion if Harvested: {}\n\
         {}\
         Pool Breakdown:\n{}{}",
        total_claimable_usd,
        gas_line,
        net_line,
        rewards_and_total_lines(data, "Total collectible (LP fees + AERO rewards, before gas)"),
        details,
        // Closed positions (#144): no row above, no figure here; the fact alone.
        crate::closed_text::closed_positions_lines(data)
    )
}
