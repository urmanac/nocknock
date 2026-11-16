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

/// An unknown `protocol` is a configuration error, stated before any chain read. Pure.
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
    }
    Ok(())
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
