use crate::*;
mod uniswap_v4;
pub(crate) use uniswap_v4::*;
mod uniswap_v3;
pub(crate) use uniswap_v3::*;
mod aero_basic;
pub(crate) use aero_basic::*;
mod aero_cl;
pub(crate) use aero_cl::*;

/// Every `protocol` a tracked position may carry. Each function that depends on the
/// protocol matches on this enum with no catch-all arm, so adding a protocol is a
/// compile error until every one of them handles it, and a value that is none of these
/// is never treated as another protocol (it used to fall through to Uniswap v4; #91).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Protocol {
    UniswapV4,
    UniswapV3,
    /// Aero "Basic Volatile" x*y=k pool: an LP-token share, full range (`aerodrome`).
    AeroBasic,
    /// Aero concentrated-liquidity (Slipstream) NFT position (`aerodrome-cl`).
    AeroCl,
}

/// The config values, for error messages.
pub(crate) const SUPPORTED_PROTOCOLS: &str = "uniswap-v4, uniswap-v3, aerodrome, aerodrome-cl";

impl Protocol {
    pub(crate) fn parse(s: &str) -> Option<Protocol> {
        match s {
            "uniswap-v4" => Some(Protocol::UniswapV4),
            "uniswap-v3" => Some(Protocol::UniswapV3),
            "aerodrome" => Some(Protocol::AeroBasic),
            AERO_CL_PROTOCOL => Some(Protocol::AeroCl),
            _ => None,
        }
    }
}

/// An unknown `protocol` is a configuration error, stated before any chain read, and so
/// are the role checks of #167 (ladder_track::validate_role_config). Pure.
pub(crate) fn validate_positions_config(positions: &[PositionConfig]) -> Result<()> {
    for c in positions {
        if Protocol::parse(&c.protocol).is_none() {
            return Err(anyhow::anyhow!(
                "Configuration error: position '{}' has unknown protocol '{}'. Supported: {}. Nothing was read; correct `positions:` in config.yaml",
                c.id,
                c.protocol,
                SUPPORTED_PROTOCOLS
            ));
        }
        // role, rung and deposit_tx (#167): an unknown role is refused like an unknown
        // protocol, and a ladder never carries a key that would steer it.
        validate_role_config(c)?;
    }
    Ok(())
}

// ------------------------------------------------------------------
// Closed positions (#144): which contract answers prove an NFT does not exist
// ------------------------------------------------------------------

/// The Aero CL (Slipstream) position manager of the Aero app on Base (ONCHAIN_INTEGRATION.md §8).
pub(crate) const AERO_CL_POSITION_MANAGER_BASE: &str = "0x4857db576ee0e45f10525ec59527bfd10beec60b";
/// The Uniswap v3 NonfungiblePositionManager on Base (ONCHAIN_INTEGRATION.md §5).
pub(crate) const UNISWAP_V3_NPM_BASE: &str = "0x03a520b32c04bf3beef7beb72e919cf822ed34f1";

/// OpenZeppelin ERC721 `ownerOf` on both NFT position managers, for an id that does not exist.
pub(crate) const ERC721_OWNER_QUERY_NONEXISTENT: &str = "ERC721: owner query for nonexistent token";
/// Slipstream `positions(uint256)` for an id that does not exist.
pub(crate) const SLIPSTREAM_POSITIONS_NONEXISTENT: &str = "ID";
/// Uniswap v3 `positions(uint256)` for an id that does not exist.
pub(crate) const UNISWAP_V3_POSITIONS_NONEXISTENT: &str = "Invalid token ID";
/// Uniswap v4 PositionManager (solmate ERC721) `ownerOf` for an id that does not exist.
pub(crate) const V4_OWNER_OF_NONEXISTENT: &str = "NOT_MINTED";

/// One position manager's own answers for a token id that does not exist, as
/// `Error(string)` reasons. Each was read on Base on 2026-11-17 for a burned id and for
/// a never-minted one (ONCHAIN_INTEGRATION.md §13).
#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct NftNonexistenceRule {
    pub(crate) manager: &'static str,
    pub(crate) positions_reason: &'static str,
    pub(crate) owner_of_reason: &'static str,
}

/// The allow-list for the `positions()` + `ownerOf()` proof. A manager that is not
/// listed has no rule, so a missing NFT on it fails the cycle as before: a fork of the
/// same contract may answer alike, but that was not read. Uniswap v4 has its own proof
/// (closed::v4_closure_verdict) and an Aero basic pool has no NFT. Pure.
pub(crate) fn nft_nonexistence_rule(protocol: &str, manager: &str) -> Option<NftNonexistenceRule> {
    let rule = match Protocol::parse(protocol) {
        Some(Protocol::AeroCl) => NftNonexistenceRule {
            manager: AERO_CL_POSITION_MANAGER_BASE,
            positions_reason: SLIPSTREAM_POSITIONS_NONEXISTENT,
            owner_of_reason: ERC721_OWNER_QUERY_NONEXISTENT,
        },
        Some(Protocol::UniswapV3) => NftNonexistenceRule {
            manager: UNISWAP_V3_NPM_BASE,
            positions_reason: UNISWAP_V3_POSITIONS_NONEXISTENT,
            owner_of_reason: ERC721_OWNER_QUERY_NONEXISTENT,
        },
        Some(Protocol::UniswapV4) | Some(Protocol::AeroBasic) | None => return None,
    };
    manager
        .trim()
        .trim_start_matches("0x")
        .eq_ignore_ascii_case(rule.manager.trim_start_matches("0x"))
        .then_some(rule)
}

/// Can a CLOSED position that is flagged `is_price_source` go on supplying the NOCK/USD
/// spot from its pool? Only where the price reads never needed the NFT. Pure.
/// - Uniswap v4: yes. The price is StateView `getSlot0(poolId)` and the pair is the
///   config's own currencies, checked against `poolKeys`: the same reads as for a live
///   position (uniswap_v4::fetch_v4_closed_price_pool).
/// - Uniswap v3 and Aero CL: no. The pool's tokens (and, for Aero CL, that the pool is
///   the NFT's pool at all) are read from the NFT, which no longer exists. The cycle
///   fails with a configuration message instead of pricing from an unchecked pool.
/// - Aero basic has no NFT and is never closed.
pub(crate) fn closed_price_source_keeps_pricing(protocol: &str) -> bool {
    match Protocol::parse(protocol) {
        Some(Protocol::UniswapV4) => true,
        Some(Protocol::UniswapV3) | Some(Protocol::AeroCl) | Some(Protocol::AeroBasic) | None => false,
    }
}

/// A closed position's label, from its config entry alone: the venue as a live label
/// names it, the pair when config states it (Uniswap v4 lists its currencies), the NFT
/// id and the config id. No fee tier and no amounts: those were reads of the live
/// position. Pure.
pub(crate) fn closed_position_label(cfg: &PositionConfig) -> String {
    let venue = match Protocol::parse(&cfg.protocol) {
        Some(Protocol::UniswapV4) => "v4",
        Some(Protocol::UniswapV3) => "v3",
        Some(Protocol::AeroCl) => "Aero CL",
        Some(Protocol::AeroBasic) => "Aero",
        None => cfg.protocol.as_str(),
    };
    let pair = match (cfg.currency0.as_deref().map(token_info), cfg.currency1.as_deref().map(token_info)) {
        (Some(Ok(t0)), Some(Ok(t1))) => format!(" {}/{}", t0.symbol, t1.symbol),
        _ => String::new(),
    };
    // A ladder (#167) says so: its closing is the operator pulling a rung.
    let role = if is_ladder_config(cfg) { ", ladder" } else { "" };
    format!("{venue}{pair} #{} ({}{role})", cfg.nft_id, cfg.id)
}

/// The pool a closed position's config entry names, lowercase. Pure.
pub(crate) fn closed_position_pool(cfg: &PositionConfig) -> Option<String> {
    let named = match Protocol::parse(&cfg.protocol) {
        Some(Protocol::UniswapV4) => cfg.pool_id.as_deref(),
        Some(Protocol::UniswapV3) | Some(Protocol::AeroCl) | Some(Protocol::AeroBasic) | None => cfg.pool_address.as_deref(),
    };
    named.map(str::trim).filter(|p| !p.is_empty()).map(str::to_lowercase)
}

/// Stable identifier shared by config rules and RawPosition.position_id.
pub(crate) fn position_key(c: &PositionConfig) -> String {
    match Protocol::parse(&c.protocol) {
        Some(Protocol::AeroBasic) => {
            format!("aerodrome:{}:{}", c.network, c.pool_address.clone().unwrap_or_default().to_lowercase())
        }
        // NFT positions, and an unknown protocol (rejected by validate_positions_config
        // before it can be read): the key is the config's own strings, nothing inferred.
        Some(Protocol::UniswapV4) | Some(Protocol::UniswapV3) | Some(Protocol::AeroCl) | None => {
            format!("{}:{}:{}", c.protocol, c.network, c.nft_id)
        }
    }
}

/// Where the operator goes to make a suggested deposit.
///
/// - Uniswap: the position's page in the Uniswap web app.
/// - Aero (protocols `aerodrome` and `aerodrome-cl`): the pool's deposit page in the
///   Aero app, `https://app.aero.xyz/deposit/<chain id>-<pool>`. Operator instruction
///   2026-11-14: never send anyone to the retired aerodrome.finance app.
/// - A config `deposit_url` overrides both.
/// - An unknown protocol has no link (empty), never another protocol's.
pub(crate) fn deposit_url_for(p: &RawPosition, rule: &RebuildRule) -> String {
    if let Some(url) = rule.deposit_url.as_ref().filter(|u| !u.trim().is_empty()) {
        return url.clone();
    }
    match Protocol::parse(&p.protocol) {
        Some(Protocol::AeroBasic) | Some(Protocol::AeroCl) => match p.network.as_str() {
            "base" => format!("https://app.aero.xyz/deposit/8453-{}", p.pool_id),
            // Unknown chain id: the pool list, rather than a link that may not resolve.
            _ => "https://app.aero.xyz/liquidity/pools".to_string(),
        },
        Some(Protocol::UniswapV3) => format!("https://app.uniswap.org/positions/v3/base/{}", p.nft_id),
        Some(Protocol::UniswapV4) => format!("https://app.uniswap.org/positions/v4/base/{}", p.nft_id),
        None => String::new(),
    }
}

/// Where the operator would open a NEW position (a ladder, #168) in the pool of the
/// tracked snapshot position `p`, with the range `tick_lower..tick_upper` in pool ticks.
/// With `usdc_amount`, the USDC side of the deposit is prefilled as `depositState`
/// (operator decision 2026-11-21, reversing v0.7.0's "nothing prefilled": the operator
/// does not type into forms); any other side is left to the app. Returns the URL and
/// what it prefills.
///
/// - Uniswap v4 on Base: the web app's new-position page,
///   `https://app.uniswap.org/positions/add/new`, with the pair, the pool key's fee and
///   tick spacing, the hooks address when it is not zero, and `priceRangeState`
///   {priceInverted: false, fullRange: false, minTick, maxTick} in pool ticks. The format
///   is read from the app's own source, not documented: the card prints the ticks and
///   prices as text as well. It needs the pool key's unrounded fee and hooks (`fee_raw`,
///   `hooks`), never a fee rebuilt from `fee_tier_bps`.
/// - Aero CL on Base: the pool's deposit page, nothing prefilled (no format is known).
/// - Anything else: no link, with the reason.
pub(crate) fn new_position_url(p: &Position, tick_lower: i32, tick_upper: i32, usdc_amount: Option<&str>) -> std::result::Result<(String, Vec<&'static str>), String> {
    if p.network != "base" {
        return Err(format!("no new-position link is known for network {}", p.network));
    }
    match Protocol::parse(&p.protocol) {
        Some(Protocol::UniswapV4) => {
            let fee = p.fee_raw.ok_or("the pool's fee was not read this cycle")?;
            let spacing = p.tick_spacing.ok_or("the pool's tick spacing was not read this cycle")?;
            let hooks = p.hooks.as_deref().ok_or("the pool's hooks address was not read this cycle")?;
            let mut params: Vec<(&str, String)> = vec![
                ("chain", "base".to_string()),
                ("protocolVersion", "v4".to_string()),
                ("currencyA", p.token0.address.clone()),
                ("currencyB", p.token1.address.clone()),
                ("fee", json!({ "feeAmount": fee, "tickSpacing": spacing, "isDynamic": false }).to_string()),
            ];
            if hooks.trim_start_matches("0x").chars().any(|c| c != '0') {
                params.push(("hook", hooks.to_string()));
            }
            params.push((
                "priceRangeState",
                json!({ "priceInverted": false, "fullRange": false, "minTick": tick_lower, "maxTick": tick_upper }).to_string(),
            ));
            // TOKEN0 / TOKEN1 are currencyA / currencyB.
            let usdc_field = if is_usdc_token(&p.token0) { Some("TOKEN0") } else if is_usdc_token(&p.token1) { Some("TOKEN1") } else { None };
            let amount = usdc_amount.zip(usdc_field);
            if let Some((a, f)) = amount {
                params.push(("depositState", {
                    let mut amounts = serde_json::Map::new();
                    amounts.insert(f.to_string(), json!(a));
                    json!({ "exactField": f, "exactAmounts": amounts }).to_string()
                }));
            }
            params.push(("step", "1".to_string()));
            let url = reqwest::Url::parse_with_params("https://app.uniswap.org/positions/add/new", &params)
                .map_err(|e| format!("the link could not be built: {e}"))?;
            let mut prefilled = vec!["pair", "fee and tick spacing", "range ticks"];
            if amount.is_some() {
                prefilled.push("amount");
            }
            Ok((url.to_string(), prefilled))
        }
        Some(Protocol::AeroCl) => Ok((format!("https://app.aero.xyz/deposit/8453-{}", p.pool_id), vec![])),
        Some(Protocol::AeroBasic) => Err("a basic Aero pool is full range; it cannot hold a ladder".to_string()),
        Some(Protocol::UniswapV3) => Err("no new-position link is built for a Uniswap v3 pool".to_string()),
        None => Err(format!("unknown protocol {}", p.protocol)),
    }
}

pub(crate) fn position_label(p: &RawPosition) -> String {
    let fee_pct = p.fee_tier_bps as f64 / 100.0;
    match Protocol::parse(&p.protocol) {
        Some(Protocol::AeroBasic) => {
            format!("Aero {}/{} {:.2}% (full range)", p.token0.symbol, p.token1.symbol, fee_pct)
        }
        Some(Protocol::AeroCl) => {
            format!("Aero CL {}/{} {:.2}% #{}", p.token0.symbol, p.token1.symbol, fee_pct, p.nft_id)
        }
        Some(Protocol::UniswapV3) => {
            format!("v3 {}/{} {:.2}% #{}", p.token0.symbol, p.token1.symbol, fee_pct, p.nft_id)
        }
        Some(Protocol::UniswapV4) => {
            format!("v4 {}/{} {:.2}% #{}", p.token0.symbol, p.token1.symbol, fee_pct, p.nft_id)
        }
        None => format!(
            "{} {}/{} #{} (unsupported protocol)",
            p.protocol, p.token0.symbol, p.token1.symbol, p.nft_id
        ),
    }
}

/// Venue name used in fee rows ("USDC+NOCK (Uniswap V4 #1)"). Pure.
pub(crate) fn protocol_display_name(protocol: &str) -> String {
    match Protocol::parse(protocol) {
        Some(Protocol::UniswapV4) => "Uniswap V4".to_string(),
        Some(Protocol::UniswapV3) => "Uniswap V3".to_string(),
        Some(Protocol::AeroBasic) => "Aerodrome".to_string(),
        Some(Protocol::AeroCl) => "Aero CL".to_string(),
        None => protocol.to_string(),
    }
}

/// `pool_mechanics[].platform` (a closed enum in the snapshot spec). Pure.
pub(crate) fn pool_platform(protocol: &str) -> &'static str {
    match Protocol::parse(protocol) {
        Some(Protocol::UniswapV4) => "UniswapV4",
        Some(Protocol::UniswapV3) => "UniswapV3",
        Some(Protocol::AeroBasic) | Some(Protocol::AeroCl) => "Aerodrome",
        None => "Other",
    }
}

/// The contract a position's fees are claimed from. Pure.
/// - Uniswap v4: the position manager (POSM). Uniswap v3: the position's NPM.
/// - Aero basic: the pool (`claimFees()`).
/// - Aero CL: the gauge while the NFT is staked there (the LP's earnings are the
///   gauge's AERO emissions, #93), otherwise the position manager (`collect`).
/// - Unknown protocol: the position's own pool id, never the Uniswap v4 POSM.
pub(crate) fn fee_contract_for(
    protocol: &str,
    npm_address: Option<&str>,
    pool_id: &str,
    staked_in_gauge: Option<&str>,
) -> String {
    match Protocol::parse(protocol) {
        Some(Protocol::UniswapV4) => UNISWAP_V4_POSM_BASE.to_string(),
        Some(Protocol::UniswapV3) => npm_address.unwrap_or(pool_id).to_string(),
        Some(Protocol::AeroBasic) => pool_id.to_string(),
        Some(Protocol::AeroCl) => staked_in_gauge.or(npm_address).unwrap_or(pool_id).to_string(),
        None => pool_id.to_string(),
    }
}

/// `contract_fees[].fee_token_symbol`: the pair and the venue. Pure.
pub(crate) fn fee_token_label(p: &Position) -> String {
    let venue = protocol_display_name(&p.protocol);
    match Protocol::parse(&p.protocol) {
        // An LP-token share has no NFT id to print.
        Some(Protocol::AeroBasic) => format!("{}+{} ({})", p.token0.symbol, p.token1.symbol, venue),
        Some(Protocol::UniswapV4) | Some(Protocol::UniswapV3) | Some(Protocol::AeroCl) | None => {
            format!("{}+{} ({} #{})", p.token0.symbol, p.token1.symbol, venue, p.nft_id)
        }
    }
}

/// Row name in the velocity panel. Pure.
pub(crate) fn velocity_label(p: &RawPosition) -> String {
    let uniswap = |venue: &str| {
        format!(
            "{} ({}/{} {:.2}%, #{})",
            venue,
            p.token0.symbol,
            p.token1.symbol,
            p.fee_tier_bps as f64 / 100.0,
            p.nft_id
        )
    };
    match Protocol::parse(&p.protocol) {
        Some(Protocol::UniswapV4) => uniswap("Uniswap V4"),
        Some(Protocol::UniswapV3) => uniswap("Uniswap V3"),
        Some(Protocol::AeroBasic) | Some(Protocol::AeroCl) | None => position_label(p),
    }
}

/// Where a tracked position's unclaimed AERO is read from (v0.6.0, #93).
pub(crate) enum AeroRewardSource {
    /// Not an Aero position, or an Aero CL NFT the wallet holds itself (unstaked, so it
    /// earns swap fees rather than AERO). Nothing is read and nothing is said.
    NotEarning,
    /// An Aero basic pool with no `gauge_address`: staked LP and its AERO cannot be
    /// read, which the position's row states (never 0).
    GaugeNotConfigured,
    /// The gauge to call and, given the wallet, the calldata of its `earned` read.
    Gauge { gauge: String, calldata: Box<dyn Fn(&str) -> String + Send> },
}

/// Which gauge call reads a position's unclaimed AERO. The two gauge types differ
/// (verified, ONCHAIN_INTEGRATION.md §9): a basic gauge answers `earned(address)`, a CL
/// gauge `earned(address,uint256[])` for the token ids the account staked. Pure.
/// - Aero basic: the configured `gauge_address`. It is read whether or not LP is staked
///   there now, because AERO earned before an unstake stays claimable.
/// - Aero CL: the gauge the cycle proved the wallet staked the NFT in.
pub(crate) fn aero_reward_source(cfg: &PositionConfig, facts: Option<&PositionFacts>) -> AeroRewardSource {
    match Protocol::parse(&cfg.protocol) {
        Some(Protocol::AeroBasic) => match cfg.gauge_address.as_ref().filter(|g| !g.trim().is_empty()) {
            Some(gauge) => AeroRewardSource::Gauge {
                gauge: gauge.to_lowercase(),
                calldata: Box::new(gauge_earned_basic_calldata),
            },
            None => AeroRewardSource::GaugeNotConfigured,
        },
        Some(Protocol::AeroCl) => match facts.and_then(|f| f.staked_in_gauge.clone()) {
            Some(gauge) => {
                let nft_id = cfg.nft_id;
                AeroRewardSource::Gauge {
                    gauge,
                    calldata: Box::new(move |wallet: &str| gauge_earned_cl_calldata(wallet, nft_id)),
                }
            }
            None => AeroRewardSource::NotEarning,
        },
        Some(Protocol::UniswapV4) | Some(Protocol::UniswapV3) | None => AeroRewardSource::NotEarning,
    }
}

/// What a `not LP-claimable` fee row says: the position's own reason when it has one
/// (an Aero CL position), otherwise the Aero basic pool's reverting claimFees(). Pure.
pub(crate) fn fees_note_text(note: Option<&str>) -> String {
    note.filter(|n| !n.trim().is_empty()).unwrap_or(AERO_NOT_LP_CLAIMABLE).to_string()
}

/// The eth_call (to, data) that reads a pool's in-range (active) liquidity: StateView
/// `getLiquidity(poolId)` for Uniswap v4, the pool's own `liquidity()` for Uniswap v3
/// and Aero CL (same selector, verified on the Slipstream pool), `totalSupply()` for an
/// Aero basic pool. An unknown protocol is an error, never a Uniswap v4 read. Pure.
pub(crate) fn pool_liquidity_call(src: &RawPosition) -> Result<(String, String)> {
    match Protocol::parse(&src.protocol) {
        // A volatile pool is live while it has LP supply: totalSupply().
        Some(Protocol::AeroBasic) => Ok((src.pool_id.clone(), "0x18160ddd".to_string())),
        Some(Protocol::UniswapV3) | Some(Protocol::AeroCl) => Ok((src.pool_id.clone(), V3_LIQUIDITY.to_string())),
        Some(Protocol::UniswapV4) => Ok((
            UNISWAP_V4_STATE_VIEW_BASE.to_string(),
            format!("{}{}", V4_STATE_VIEW_GET_LIQUIDITY, src.pool_id.trim_start_matches("0x")),
        )),
        None => Err(anyhow::anyhow!(
            "position {}: unknown protocol '{}' has no pool liquidity read (supported: {})",
            src.position_id,
            src.protocol,
            SUPPORTED_PROTOCOLS
        )),
    }
}

/// In-range (active) liquidity of a tracked position's pool (see pool_liquidity_call).
pub(crate) async fn fetch_pool_in_range_liquidity(
    http_client: &Client,
    alchemy_url: &str,
    src: &RawPosition,
) -> Result<u128> {
    let (to, data) = pool_liquidity_call(src)?;
    let payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "to": to, "data": data }, "latest"],
        "id": 58
    });
    post_rpc_with_retry(http_client, alchemy_url, &payload, 2)
        .await
        .and_then(|r| r.get("result").and_then(|v| v.as_str()).map(|s| s.to_string()))
        .and_then(|s| {
            let clean = s.trim_start_matches("0x");
            if clean.len() < 64 {
                return None;
            }
            // uint128 right-aligned in a 32-byte word
            u128::from_str_radix(&clean[clean.len() - 32..], 16).ok()
        })
        .ok_or_else(|| {
            anyhow::anyhow!(
                "price source {}: failed to query in-range liquidity of pool {}",
                src.position_id,
                src.pool_id
            )
        })
}
