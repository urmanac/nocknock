use crate::*;

pub(crate) async fn post_rpc_with_retry(
    client: &Client,
    url: &str,
    payload: &serde_json::Value,
    max_retries: usize,
) -> Option<serde_json::Value> {
    for attempt in 0..=max_retries {
        match client.post(url).json(payload).send().await {
            Ok(resp) => {
                if resp.status().is_success() {
                    if let Ok(json_val) = resp.json::<serde_json::Value>().await {
                        if let Some(err) = json_val.get("error") {
                            let msg = err.get("message").and_then(|m| m.as_str()).unwrap_or("RPC error");
                            tracing::warn!("RPC returned error (attempt {}/{}): {}", attempt + 1, max_retries + 1, msg);
                        } else {
                            return Some(json_val);
                        }
                    }
                } else if resp.status().as_u16() == 429 {
                    tracing::warn!("Rate limited (429) on RPC (attempt {}/{})", attempt + 1, max_retries + 1);
                } else {
                    tracing::warn!("HTTP status {} on RPC (attempt {}/{})", resp.status(), attempt + 1, max_retries + 1);
                }
            }
            Err(e) => {
                // reqwest prints the request URL in its errors, and the RPC URL carries the API
                // key: never let it reach a log line or a caller.
                tracing::warn!("Network error calling RPC (attempt {}/{}): {}", attempt + 1, max_retries + 1, e.without_url());
            }
        }

        if attempt < max_retries {
            let base_delay_ms = 100u64 * (1 << attempt);
            let jitter_ms = (Utc::now().timestamp_subsec_millis() as u64) % 75;
            tokio::time::sleep(tokio::time::Duration::from_millis(base_delay_ms + jitter_ms)).await;
        }
    }
    None
}

/// `eth_call` that distinguishes "the contract reverted" from "the node did not answer".
/// `post_rpc_with_retry` folds both into `None`, which is right for reads that must
/// succeed but wrong where a revert is itself the answer.
///   Ok(Some(hex)) = returned data (without 0x) · Ok(None) = reverted · Err = no answer.
pub(crate) async fn eth_call_allow_revert(
    client: &Client,
    url: &str,
    payload: &serde_json::Value,
) -> Result<Option<String>> {
    let mut last = String::from("no response");
    for attempt in 0..3u32 {
        match client.post(url).json(payload).send().await {
            Ok(resp) if resp.status().is_success() => match resp.json::<serde_json::Value>().await {
                Ok(v) => return classify_eth_call_response(&v),
                Err(e) => last = format!("unparseable response: {}", e.without_url()),
            },
            Ok(resp) => last = format!("HTTP {}", resp.status()),
            // The error text is returned to callers that publish it (/health, snapshot notes,
            // MCP text). reqwest appends "for url (<url>)", and the URL holds the API key.
            Err(e) => last = format!("network error: {}", e.without_url()),
        }
        tokio::time::sleep(tokio::time::Duration::from_millis(150 * (1 << attempt))).await;
    }
    Err(anyhow::anyhow!(last))
}

/// Pure part of `eth_call_allow_revert`. JSON-RPC error code 3 is "execution reverted";
/// nodes also say so in the message. Any other error is not a revert.
pub(crate) fn classify_eth_call_response(v: &serde_json::Value) -> Result<Option<String>> {
    if let Some(hex) = v.get("result").and_then(|r| r.as_str()) {
        return Ok(Some(hex.trim_start_matches("0x").to_string()));
    }
    if let Some(err) = v.get("error") {
        let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
        let msg = err.get("message").and_then(|m| m.as_str()).unwrap_or("").to_lowercase();
        if code == 3 || msg.contains("revert") {
            return Ok(None);
        }
        return Err(anyhow::anyhow!("RPC error {code}: {msg}"));
    }
    Err(anyhow::anyhow!("response has neither result nor error"))
}

/// Single-word (or selected-word) eth_call helper for v3 pool reads.
pub(crate) async fn post_rpc_word(
    http_client: &Client,
    alchemy_url: &str,
    to: &str,
    data: &str,
    word_index: usize,
    id: u64,
) -> Result<U256> {
    let payload = json!({
        "jsonrpc": "2.0",
        "method": "eth_call",
        "params": [{ "to": to, "data": if data.starts_with("0x") { data.to_string() } else { format!("0x{}", data) } }, "latest"],
        "id": id
    });
    let res = post_rpc_with_retry(http_client, alchemy_url, &payload, 2)
        .await
        .ok_or_else(|| anyhow::anyhow!("eth_call to {} failed", to))?;
    let hex_str = res
        .get("result")
        .and_then(|v| v.as_str())
        .ok_or_else(|| anyhow::anyhow!("eth_call to {} returned no result", to))?;
    let clean = hex_str.trim_start_matches("0x");
    let start = word_index * 64;
    if clean.len() < start + 64 {
        return Err(anyhow::anyhow!(
            "eth_call to {} returned short output ({} chars, need word {})",
            to,
            clean.len(),
            word_index
        ));
    }
    U256::from_hex_be(&clean[start..start + 64])
}
