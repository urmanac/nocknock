use crate::*;
mod uniswap_v4;
pub(crate) use uniswap_v4::*;
mod uniswap_v3;
pub(crate) use uniswap_v3::*;
mod aero_basic;
pub(crate) use aero_basic::*;

/// Stable identifier shared by config rules and RawPosition.position_id.
pub(crate) fn position_key(c: &PositionConfig) -> String {
    if c.protocol == "aerodrome" {
        format!("aerodrome:{}:{}", c.network, c.pool_address.clone().unwrap_or_default().to_lowercase())
    } else {
        format!("{}:{}:{}", c.protocol, c.network, c.nft_id)
    }
}

/// Where the operator goes to make a suggested deposit.
///
/// - Uniswap: the position's page in the Uniswap web app.
/// - Aero (protocol `aerodrome`): the pool's deposit page in the Aero app,
///   `https://app.aero.xyz/deposit/<chain id>-<pool>`. Operator instruction 2026-11-14:
///   never send anyone to the retired aerodrome.finance app.
/// - A config `deposit_url` overrides both.
pub(crate) fn deposit_url_for(p: &RawPosition, rule: &RebuildRule) -> String {
    if let Some(url) = rule.deposit_url.as_ref().filter(|u| !u.trim().is_empty()) {
        return url.clone();
    }
    if p.protocol == "aerodrome" {
        match p.network.as_str() {
            "base" => format!("https://app.aero.xyz/deposit/8453-{}", p.pool_id),
            // Unknown chain id: the pool list, rather than a link that may not resolve.
            _ => "https://app.aero.xyz/liquidity/pools".to_string(),
        }
    } else {
        format!(
            "https://app.uniswap.org/positions/{}/base/{}",
            if p.protocol == "uniswap-v3" { "v3" } else { "v4" },
            p.nft_id
        )
    }
}

pub(crate) fn position_label(p: &RawPosition) -> String {
    if p.protocol == "aerodrome" {
        return format!(
            "Aero {}/{} {:.2}% (full range)",
            p.token0.symbol,
            p.token1.symbol,
            p.fee_tier_bps as f64 / 100.0
        );
    }
    format!(
        "{} {}/{} {:.2}% #{}",
        if p.protocol == "uniswap-v3" { "v3" } else { "v4" },
        p.token0.symbol,
        p.token1.symbol,
        p.fee_tier_bps as f64 / 100.0,
        p.nft_id
    )
}

/// In-range (active) liquidity of the price-source pool: StateView `getLiquidity(poolId)`
/// for Uniswap v4, the pool's own `liquidity()` for Uniswap v3.
pub(crate) async fn fetch_pool_in_range_liquidity(
    http_client: &Client,
    alchemy_url: &str,
    src: &RawPosition,
) -> Result<u128> {
    let (to, data) = if src.protocol == "aerodrome" {
        // A volatile pool is live while it has LP supply: totalSupply().
        (src.pool_id.clone(), "0x18160ddd".to_string())
    } else if src.protocol == "uniswap-v3" {
        (src.pool_id.clone(), V3_LIQUIDITY.to_string())
    } else {
        (
            UNISWAP_V4_STATE_VIEW_BASE.to_string(),
            format!("{}{}", V4_STATE_VIEW_GET_LIQUIDITY, src.pool_id.trim_start_matches("0x")),
        )
    };
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
