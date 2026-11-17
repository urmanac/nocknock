use crate::*;

// ============================================================
// Gauge rewards as collectible earnings — issue #142
// ============================================================
// A staked Aero position's earnings are its gauge reward. #93 read the amount; this
// module reports it as what it is: collectible earnings, with the token, the amount,
// its USD value, the gauge it is collected from and how (the Aero app).
//
// What does not change. Unclaimed AERO stays out of `total_portfolio_usd`.
// `total_fees_claimable_usd` keeps meaning LP fees only, and BATCH HARVEST, the
// notifier and every threshold keep reading LP fees only: the operator has set no rule
// for rewards. Nothing here is advice. "Collectible" states an amount and a place.
//
// The claim calls. NockNock builds no transaction for a reward. It may state what the
// claim would cost in gas, and only for a call proven on-chain to pay what `earned`
// reports (ONCHAIN_INTEGRATION.md §9, "The claim call, proven"):
//   - CL gauge 0x3bf70c88…60fd: `0x09902477` (wallet, wallet, [tokenId]). The tracked
//     wallet's own claim, tx 0x2d2b609b…a16e in block 58618046, minted exactly
//     `earned(wallet, [2914490])` of the block before (17.654015228100787903 AERO).
//   - Basic gauge 0xc82782ee…3526: `0x8ed86282` (wallet, wallet). A real claim by
//     another account (tx 0x189f6ddf…b8f8) minted exactly its `earned`, and the same
//     call simulated from the tracked wallet with `eth_simulateV1` at block 58619904
//     mints exactly `earned(wallet)` at that block.
// Any other gauge is "claim call not verified for this gauge": no gas figure is shown.

/// How a gauge reward is collected. NockNock sends nothing; the operator uses the app.
pub(crate) const REWARD_COLLECTED_IN: &str = "the Aero app";
/// The Aero app. Never the retired aerodrome.finance.
pub(crate) const AERO_APP_URL: &str = "https://app.aero.xyz";
/// `earnings_kind` of a fee row whose reported earnings are the gauge reward. It is
/// deliberately not one of the harvest words (claim_now, claim_recommended, accumulate).
pub(crate) const EARNINGS_KIND_GAUGE_REWARD: &str = "gauge_reward";

/// The basic gauge's claim: selector `0x8ed86282`, two address words (both the wallet).
pub(crate) const GAUGE_CLAIM_BASIC: &str = "0x8ed86282";
/// The CL gauge's claim: selector `0x09902477`, two address words and a token id array.
pub(crate) const GAUGE_CLAIM_CL: &str = "0x09902477";
/// The gauges whose claim call was proven to pay what `earned` reports, lower case.
pub(crate) const VERIFIED_BASIC_CLAIM_GAUGES: [&str; 1] = ["0xc82782ee66d82c4d19db0429652091a085083526"];
pub(crate) const VERIFIED_CL_CLAIM_GAUGES: [&str; 1] = ["0x3bf70c880adc8370917994ab0df68e763b7a60fd"];

/// How long a request for the claim simulation may spend on live gas estimates. The
/// stdio MCP binary waits two seconds for the worker's answer.
pub(crate) const REWARD_GAS_BUDGET_MS: u64 = 1500;

/// One position's gauge reward as collectible earnings, from the cycle's read. Pure.
/// The same strings as `positions[].unclaimed_aero*` (aero_position_fields), so the two
/// can never disagree. A gauge that was not read has no amount and no USD value.
pub(crate) fn collectible_reward(entry: &AeroUnclaimed, price_usd: Option<f64>) -> CollectibleReward {
    let (amount, usd, note) = aero_position_fields(entry, price_usd);
    CollectibleReward {
        status: if amount.is_some() { "read" } else { "not_read" }.to_string(),
        token_symbol: "AERO".to_string(),
        token_address: AERO_BASE.to_string(),
        amount,
        usd,
        gauge: entry.gauge.clone(),
        collected_in: REWARD_COLLECTED_IN.to_string(),
        collect_url: AERO_APP_URL.to_string(),
        note,
    }
}

/// The reward of one position in this cycle's reads, if it has an entry. Pure.
pub(crate) fn collectible_reward_for(reads: &AeroReads, position_id: &str) -> Option<CollectibleReward> {
    let entry = reads.unclaimed.as_ref()?.iter().find(|e| e.position_id == position_id)?;
    Some(collectible_reward(entry, reads.price_usd))
}

/// The collectible totals of one cycle. Each is None when it cannot be stated.
#[derive(Debug, Clone, PartialEq, Default)]
pub(crate) struct CollectibleTotals {
    /// Gauge rewards in USD, as printed (2 decimals).
    pub(crate) rewards_usd: Option<String>,
    /// LP fees plus gauge rewards, the sum of the two printed figures.
    pub(crate) total_usd: Option<String>,
    /// Why the two above are None, for every cause a live cycle can produce.
    pub(crate) note: Option<String>,
}

/// Collectible gauge rewards and the total collectible figure. Pure.
/// - The rewards figure is the AERO pillar's unclaimed USD: stated only when every
///   gauge was read and every non-zero reward is priced. Otherwise it is None with the
///   reason, never a partial sum and never 0.
/// - It is the tracked gauges' summed amount at the cycle's price, the same figure as
///   `global_status.unclaimed_aero_usd`, so one amount of AERO never prints with two
///   dollar values. It is NOT the sum of the rounded per-position `usd` strings and can
///   differ from that sum by a cent per position.
/// - The total is the printed LP-fee figure plus the printed rewards figure, so the
///   three numbers a surface shows always add up to the cent.
/// - `lp_fees_usd` is the caller's LP-fee total; it is only added, never changed.
pub(crate) fn collectible_totals(reads: &AeroReads, pillar: &AeroPillar, lp_fees_usd: f64) -> CollectibleTotals {
    let Some(entries) = reads.unclaimed.as_ref() else {
        // Nothing was read at all (test fixtures only): unknown, with no live reason.
        return CollectibleTotals::default();
    };
    let unread: Vec<String> = entries
        .iter()
        .filter(|e| e.amount.is_none())
        .map(|e| format!("{} ({})", e.position_id, e.note.as_deref().unwrap_or("no reason recorded")))
        .collect();
    if !unread.is_empty() {
        return CollectibleTotals {
            note: Some(format!("gauge reward not read for {}", unread.join("; "))),
            ..Default::default()
        };
    }
    let Some(rewards) = pillar.unclaimed_usd else {
        let amount = pillar.unclaimed_amount.unwrap_or(0.0);
        let why = reads.price_note.as_deref().filter(|n| !n.trim().is_empty()).unwrap_or("AERO/USD price unavailable");
        return CollectibleTotals {
            note: Some(format!("{amount:.6} AERO in tracked gauges has no USD value: {why}")),
            ..Default::default()
        };
    };
    let rewards_usd = format!("{:.2}", rewards);
    let printed = |s: &str| s.parse::<f64>().unwrap_or(0.0);
    let total = printed(&format!("{:.2}", lp_fees_usd)) + printed(&rewards_usd);
    CollectibleTotals { rewards_usd: Some(rewards_usd), total_usd: Some(format!("{:.2}", total)), note: None }
}

/// The three row fields that say what a not-LP-claimable position earns. Pure.
/// - `lp_claimable`: the row carries a fee action, so it gets no reward and no kind
///   (an AERO amount beside claim_now or accumulate would read as advice about AERO).
/// - otherwise the reason alone, and the reward when the position has one.
pub(crate) fn fee_row_earnings(
    lp_claimable: bool,
    fees_note: Option<&str>,
    reward: Option<CollectibleReward>,
) -> (Option<String>, Option<CollectibleReward>, Option<String>) {
    if lp_claimable {
        return (None, None, None);
    }
    let kind = reward.as_ref().map(|_| EARNINGS_KIND_GAUGE_REWARD.to_string());
    (kind, reward, Some(fees_note_text(fees_note)))
}

/// Calldata of the claim call proven for this gauge, or None when none is proven. Pure.
/// Used ONLY as the input of a read-only `eth_estimateGas`; it is never returned to a
/// client and never put in the claim payload.
pub(crate) fn verified_reward_claim_call(protocol: &str, gauge: &str, wallet: &str, nft_id: u64) -> Option<String> {
    let gauge = gauge.trim().to_lowercase();
    let w = address_word(wallet);
    match Protocol::parse(protocol) {
        Some(Protocol::AeroBasic) if VERIFIED_BASIC_CLAIM_GAUGES.contains(&gauge.as_str()) => {
            Some(format!("{GAUGE_CLAIM_BASIC}{w}{w}"))
        }
        Some(Protocol::AeroCl) if VERIFIED_CL_CLAIM_GAUGES.contains(&gauge.as_str()) => {
            // (address, address, uint256[]): the array's offset is three words (0x60).
            Some(format!("{GAUGE_CLAIM_CL}{w}{w}{:064x}{:064x}{:064x}", 0x60, 1, nft_id))
        }
        Some(Protocol::AeroBasic) | Some(Protocol::AeroCl) | Some(Protocol::UniswapV4) | Some(Protocol::UniswapV3) | None => None,
    }
}

/// The claim call to estimate for a snapshot position, or why there is none. Pure.
/// Ok(calldata, gauge) only for a read, non-zero reward on a gauge with a proven call.
pub(crate) fn reward_gas_request(p: &Position, wallet: &str) -> std::result::Result<(String, String), RewardGas> {
    let Some(reward) = p.collectible_reward.as_ref() else {
        return Err(RewardGas::NotRequested);
    };
    let amount = reward.amount.as_deref().and_then(|a| a.trim().parse::<f64>().ok());
    // Not read, or nothing to collect: the text says so without a gas figure.
    if !amount.is_some_and(|a| a > 0.0) {
        return Err(RewardGas::NotRequested);
    }
    let Some(gauge) = reward.gauge.as_deref() else {
        return Err(RewardGas::NotVerified);
    };
    match verified_reward_claim_call(&p.protocol, gauge, wallet, p.nft_id) {
        Some(data) => Ok((data, gauge.to_string())),
        None => Err(RewardGas::NotVerified),
    }
}

/// One `eth_estimateGas` answer as a gas statement. Pure. `call` is Ok(Some(hex)) for a
/// quantity, Ok(None) for a revert, Err when the node did not answer. The error text
/// comes from rpc.rs, which never lets a request URL through.
pub(crate) fn reward_gas_from_estimate(
    call: &Result<Option<String>>,
    gas_price_wei: Option<u128>,
    eth_price_usd: Option<f64>,
) -> RewardGas {
    match call {
        Ok(Some(hex)) => match u128::from_str_radix(hex.trim_start_matches("0x"), 16) {
            Ok(units) if units > 0 => {
                // Both live inputs or no USD figure: never a default gas or ETH price.
                let usd = match (gas_price_wei, eth_price_usd.filter(|p| p.is_finite() && *p > 0.0)) {
                    (Some(gp), Some(ep)) => Some((units as f64) * (gp as f64) / 1e18 * ep),
                    _ => None,
                };
                RewardGas::Estimated { units, usd }
            }
            _ => RewardGas::Failed("the estimate was not a gas quantity".to_string()),
        },
        Ok(None) => RewardGas::Failed("the claim call reverts when estimated from the wallet".to_string()),
        Err(e) => RewardGas::Failed(format!("the estimate was not answered: {e}")),
    }
}

/// Where a request's gas price comes from.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) enum GasPrice {
    /// The caller has made no `eth_gasPrice` read for this request: make one here.
    ReadNow,
    /// The caller's own `eth_gasPrice` read for this request: the price (None when that
    /// read failed). It is used as it is and not read again, so one response never
    /// prices its LP actions and its reward rows at two gas prices.
    Known(Option<u128>),
}

/// Live gas statements for the rewards of a snapshot's positions, index-aligned with
/// `positions` (and so with `contract_fees`). Read-only: `eth_gasPrice` at most once
/// (not at all when the caller already read it), then one `eth_estimateGas` from the
/// wallet per reward that has a proven claim call. Bounded by `budget`; a call that
/// does not return in time is stated as such, never guessed.
pub(crate) async fn estimate_reward_claim_gas(
    http_client: &Client,
    rpc_url: &str,
    wallet: &str,
    positions: &[Position],
    eth_price_usd: Option<f64>,
    gas_price: GasPrice,
    budget: std::time::Duration,
) -> Vec<RewardGas> {
    let deadline = tokio::time::Instant::now() + budget;
    let requests: Vec<std::result::Result<(String, String), RewardGas>> =
        positions.iter().map(|p| reward_gas_request(p, wallet)).collect();
    if requests.iter().all(|r| r.is_err()) {
        return requests.into_iter().map(|r| r.err().unwrap_or(RewardGas::NotRequested)).collect();
    }
    let gas_price_wei: Option<u128> = match gas_price {
        GasPrice::Known(wei) => wei,
        GasPrice::ReadNow => {
            let gas_price_payload = json!({ "jsonrpc": "2.0", "method": "eth_gasPrice", "params": [], "id": 70 });
            tokio::time::timeout_at(deadline, post_rpc_with_retry(http_client, rpc_url, &gas_price_payload, 1))
                .await
                .ok()
                .flatten()
                .and_then(|r| r.get("result").and_then(|v| v.as_str()).map(|s| s.to_string()))
                .and_then(|s| u128::from_str_radix(s.trim_start_matches("0x"), 16).ok())
        }
    };
    let mut out = Vec::with_capacity(requests.len());
    for (idx, request) in requests.into_iter().enumerate() {
        out.push(match request {
            Err(gas) => gas,
            Ok((data, gauge)) => {
                let payload = json!({
                    "jsonrpc": "2.0",
                    "method": "eth_estimateGas",
                    "params": [{ "from": wallet, "to": gauge, "data": data, "value": "0x0" }, "latest"],
                    "id": 71 + idx
                });
                match tokio::time::timeout_at(deadline, eth_call_allow_revert(http_client, rpc_url, &payload)).await {
                    Ok(call) => reward_gas_from_estimate(&call, gas_price_wei, eth_price_usd),
                    Err(_) => RewardGas::Failed("the estimate did not return in time".to_string()),
                }
            }
        });
    }
    out
}

/// The RPC URL for request-time estimates, when a key is configured. Pure.
pub(crate) fn request_rpc_url(alchemy_api_keys: &[String]) -> Option<String> {
    alchemy_api_keys
        .first()
        .filter(|k| !k.trim().is_empty() && k.as_str() != "YOUR_ALCHEMY_API_KEY")
        .map(|k| format!("https://base-mainnet.g.alchemy.com/v2/{k}"))
}

/// The claim payload's reward rows: one per position that carries a reward. Pure.
/// A row has no `to` and no `data`: nothing in it can be dispatched to a wallet.
pub(crate) fn claim_reward_rows(positions: &[Position], gas: &[RewardGas]) -> Vec<ClaimRewardRow> {
    positions
        .iter()
        .enumerate()
        .filter_map(|(i, p)| {
            let r = p.collectible_reward.as_ref()?;
            let value = serde_json::to_value(r).unwrap_or_default();
            let row_gas = gas.get(i).unwrap_or(&RewardGas::NotRequested);
            let (gas_estimate, gas_usd) = match row_gas {
                RewardGas::Estimated { units, usd } => (Some(format!("0x{:x}", units)), usd.map(|u| format!("{:.4}", u))),
                _ => (None, None),
            };
            let gas_note = if r.amount.is_none() {
                "gas not estimated (the reward was not read)".to_string()
            } else {
                reward_gas_text(row_gas, reward_is_zero(&value))
            };
            Some(ClaimRewardRow {
                position_id: p.position_id.clone(),
                label: fee_token_label(p),
                status: r.status.clone(),
                token_symbol: r.token_symbol.clone(),
                amount: r.amount.clone(),
                usd: r.usd.clone(),
                gauge: r.gauge.clone(),
                collected_in: r.collected_in.clone(),
                collect_url: r.collect_url.clone(),
                note: r.note.clone(),
                gas_estimate,
                gas_usd,
                gas_note,
            })
        })
        .collect()
}

/// What a `not_claimable` claim action says after its reason: the position's reward and
/// where it is collected, or that the reward was not read. Pure. None when the position
/// carries no reward (not an Aero position, or a snapshot from before #142).
pub(crate) fn reward_detail(p: &Position) -> Option<String> {
    let r = p.collectible_reward.as_ref()?;
    Some(format!("reward {}", reward_statement(&serde_json::to_value(r).unwrap_or_default())))
}

/// Append the reward to a `not_claimable` action's detail. Pure. Any other action is
/// left exactly as built: an executable action never carries an AERO statement.
pub(crate) fn state_reward_on_not_claimable(action: &mut ClaimFeeAction, snapshot_position: Option<&Position>) {
    if action.simulation != "not_claimable" {
        return;
    }
    if let Some(detail) = snapshot_position.and_then(reward_detail) {
        action.simulation_detail = Some(match action.simulation_detail.take().filter(|d| !d.trim().is_empty()) {
            Some(reason) => format!("{reason}; {detail}"),
            None => detail,
        });
    }
}
