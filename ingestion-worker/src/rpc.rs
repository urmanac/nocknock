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
    Ok(match classify_eth_call_outcome(v)? {
        CallOutcome::Returned(hex) => Some(hex),
        CallOutcome::Reverted(_) => None,
    })
}

/// What a node answered to an `eth_call` (#144). A revert keeps its revert data, so a
/// caller can tell the contract's own "this token id does not exist" from any other
/// revert. No answer at all (transport, HTTP status, a JSON-RPC error that is not an
/// execution revert) is an `Err`, never one of these.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum CallOutcome {
    /// Returned data, hex without `0x`.
    Returned(String),
    /// Execution reverted. The revert data as the node sent it (hex without `0x`,
    /// lowercase; `Some("")` for a revert with empty data). `None` when the node said
    /// "reverted" and sent no well-formed `data` field: the reason is then unknown.
    Reverted(Option<String>),
}

/// Pure classification of an `eth_call` JSON-RPC response, keeping the revert data.
/// A revert is error code 3 or a message that says "revert" (as before #144).
pub(crate) fn classify_eth_call_outcome(v: &serde_json::Value) -> Result<CallOutcome> {
    if let Some(hex) = v.get("result").and_then(|r| r.as_str()) {
        return Ok(CallOutcome::Returned(hex.trim_start_matches("0x").to_string()));
    }
    if let Some(err) = v.get("error") {
        let code = err.get("code").and_then(|c| c.as_i64()).unwrap_or(0);
        let msg = err.get("message").and_then(|m| m.as_str()).unwrap_or("").to_lowercase();
        if code == 3 || msg.contains("revert") {
            // Only a hex string is revert data. Anything else (absent, an object, text)
            // leaves the reason unknown, which no caller may read as a known reason.
            let data = err
                .get("data")
                .and_then(|d| d.as_str())
                .and_then(|d| d.strip_prefix("0x"))
                .filter(|d| d.len() % 2 == 0 && d.bytes().all(|b| b.is_ascii_hexdigit()))
                .map(|d| d.to_lowercase());
            return Ok(CallOutcome::Reverted(data));
        }
        return Err(anyhow::anyhow!("RPC error {code}: {msg}"));
    }
    Err(anyhow::anyhow!("response has neither result nor error"))
}

/// A decoded revert reason (#144). Only `Message` can ever match an allow-list entry.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum RevertReason {
    /// `Error(string)` (selector 0x08c379a0), strictly ABI-decoded.
    Message(String),
    /// `Panic(uint256)` (selector 0x4e487b71): the panic code, hex.
    Panic(String),
    /// A custom error: its 4-byte selector, hex with `0x`.
    Custom(String),
    /// The call reverted with no data at all (`revert()`, a failed `require` without a
    /// message, or a call to an address without that function).
    Empty,
    /// The node reported a revert and sent no revert data.
    NoData,
    /// Revert data that is not a well-formed `Error(string)`, `Panic` or selector.
    Malformed,
}

pub(crate) const REVERT_ERROR_STRING_SELECTOR: &str = "08c379a0"; // Error(string)
pub(crate) const REVERT_PANIC_SELECTOR: &str = "4e487b71"; // Panic(uint256)

/// Decode revert data (hex without `0x`; `None` = the node sent none). Pure. The
/// `Error(string)` decode is strict: offset 0x20, a length that fits the data exactly,
/// zero padding and valid UTF-8. Anything looser is `Malformed`, so a truncated or
/// padded payload can never be read as a known reason.
pub(crate) fn decode_revert_reason(data: Option<&str>) -> RevertReason {
    let Some(data) = data else {
        return RevertReason::NoData;
    };
    let data = data.trim_start_matches("0x").to_lowercase();
    if data.is_empty() {
        return RevertReason::Empty;
    }
    if data.len() < 8 || data.len() % 2 != 0 || !data.bytes().all(|b| b.is_ascii_hexdigit()) {
        return RevertReason::Malformed;
    }
    let (selector, body) = data.split_at(8);
    let small_word = |w: &str| -> Option<usize> {
        if w.len() != 64 || !w[..48].bytes().all(|b| b == b'0') {
            return None;
        }
        usize::from_str_radix(&w[48..], 16).ok()
    };
    match selector {
        REVERT_ERROR_STRING_SELECTOR => {
            if body.len() < 128 || small_word(&body[..64]) != Some(0x20) {
                return RevertReason::Malformed;
            }
            let Some(len) = small_word(&body[64..128]) else {
                return RevertReason::Malformed;
            };
            let text = &body[128..];
            // The length word is the contract's (or the node's) to choose: bound it by
            // the data that is actually there before any arithmetic on it, so a length
            // of 2^63 or more is Malformed and never an overflow or a slice panic.
            if len > text.len() / 2 {
                return RevertReason::Malformed;
            }
            let padded = len.div_ceil(32) * 32;
            if text.len() != padded * 2 || !text[len * 2..].bytes().all(|b| b == b'0') {
                return RevertReason::Malformed;
            }
            let bytes: Option<Vec<u8>> =
                (0..len).map(|i| u8::from_str_radix(&text[i * 2..i * 2 + 2], 16).ok()).collect();
            match bytes.and_then(|b| String::from_utf8(b).ok()) {
                Some(s) => RevertReason::Message(s),
                None => RevertReason::Malformed,
            }
        }
        REVERT_PANIC_SELECTOR if body.len() == 64 => {
            let code = body.trim_start_matches('0');
            RevertReason::Panic(format!("0x{}", if code.is_empty() { "0" } else { code }))
        }
        REVERT_PANIC_SELECTOR => RevertReason::Malformed,
        _ => RevertReason::Custom(format!("0x{selector}")),
    }
}

impl RevertReason {
    /// The reason in words, for an error message or the evidence of a closed position.
    pub(crate) fn describe(&self) -> String {
        match self {
            RevertReason::Message(m) => format!("Error(\"{m}\")"),
            RevertReason::Panic(code) => format!("Panic({code})"),
            RevertReason::Custom(selector) => format!("custom error {selector}"),
            RevertReason::Empty => "empty revert data".to_string(),
            RevertReason::NoData => "no revert data from the node".to_string(),
            RevertReason::Malformed => "revert data that does not decode".to_string(),
        }
    }
}

/// `eth_call` that keeps the revert data (#144). Used where a revert may be the
/// contract's own answer AND which answer it is matters.
///   Ok(Returned) = data · Ok(Reverted) = the contract reverted · Err = no answer.
/// Three attempts, like `post_rpc_with_retry`: a transport failure, an HTTP error
/// status, an unparseable body and a JSON-RPC error that is not a revert are all
/// retried and end as `Err`. The error text never contains the URL.
pub(crate) async fn eth_call_outcome(client: &Client, url: &str, payload: &serde_json::Value) -> Result<CallOutcome> {
    let mut last = String::from("no response");
    for attempt in 0..3u32 {
        match client.post(url).json(payload).send().await {
            Ok(resp) if resp.status().is_success() => match resp.json::<serde_json::Value>().await {
                Ok(v) => match classify_eth_call_outcome(&v) {
                    Ok(outcome) => return Ok(outcome),
                    Err(e) => last = e.to_string(),
                },
                Err(e) => last = format!("unparseable response: {}", e.without_url()),
            },
            Ok(resp) => last = format!("HTTP {}", resp.status()),
            // reqwest appends "for url (<url>)", and the URL holds the API key.
            Err(e) => last = format!("network error: {}", e.without_url()),
        }
        if attempt < 2 {
            tokio::time::sleep(tokio::time::Duration::from_millis(150 * (1 << attempt))).await;
        }
    }
    Err(anyhow::anyhow!(last))
}

/// The latest block's number and time (#144), for reads that must all be made at one
/// current block. `Err` when the header could not be read or carries no number or time:
/// never a substitute block. The error text never contains the URL.
pub(crate) async fn latest_block_header(client: &Client, url: &str) -> Result<(i64, DateTime<Utc>)> {
    let payload = json!({ "jsonrpc": "2.0", "id": 88, "method": "eth_getBlockByNumber", "params": ["latest", false] });
    let resp = post_rpc_with_retry(client, url, &payload, 2)
        .await
        .ok_or_else(|| anyhow::anyhow!("the node's latest block could not be read (eth_getBlockByNumber gave no answer)"))?;
    parse_block_header(&resp)
        .ok_or_else(|| anyhow::anyhow!("the node's answer for its latest block carries no readable number and time"))
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
