
// ============================================================
// Golden Path: claim-fees payload engine (v0.2.0)
// ============================================================
// Encoding verified against Base mainnet via eth_call simulation on 2026-10-12
// (helix-specs task 000661 verification/v4collect_sim.py; golden vectors in
// verification/sdk_full.txt match @uniswap/v4-sdk@2.4.1 V4PositionPlanner output
// byte-for-byte). Do not "fix" these constants without re-running that simulation.

pub(crate) const UNISWAP_V4_POSM_BASE: &str = "0x7c5f5a4bbd8fd63184577525326123b519429bdc";
pub(crate) const POSM_MODIFY_LIQUIDITIES_SELECTOR: &str = "dd46508f"; // modifyLiquidities(bytes,uint256)
pub(crate) const V4_ACTION_DECREASE_LIQUIDITY: u8 = 0x01; // liquidity 0 => collect fees only
pub(crate) const V4_ACTION_TAKE_PAIR: u8 = 0x11; // unified Actions enum as deployed on Base
pub(crate) const UNISWAP_V3_COLLECT_SELECTOR: &str = "fc6f7865"; // collect((uint256,address,uint128,uint128)) —
    // Phase-1 ground truth: the NPM collect staticcall used to validate the v3 fee math

pub(crate) fn word_addr(addr: &str) -> String {
    format!("000000000000000000000000{}", addr.trim_start_matches("0x").to_lowercase())
}

pub(crate) fn word_u256(v: u64) -> String {
    format!("{:064x}", v)
}

/// ABI-encode a dynamic `bytes` member: 32-byte length word + data padded to 32-byte multiples.
pub(crate) fn abi_encode_bytes(b_hex: &str) -> String {
    let clean = b_hex.trim_start_matches("0x");
    let pad_pairs = ((64 - (clean.len() % 64)) % 64) / 2;
    format!("{:064x}{}{}", clean.len() / 2, clean, "00".repeat(pad_pairs))
}

/// ABI-encode a `bytes[]` array at its own position: [length, offsets..., contents...].
/// Offsets are in bytes, relative to the start of the array (after the length word).
pub(crate) fn abi_encode_bytes_array(items: &[String]) -> String {
    let n = items.len();
    let mut offsets = String::new();
    let mut contents = String::new();
    let mut acc = 0usize;
    for it in items {
        offsets.push_str(&word_u256((n * 32 + acc) as u64));
        acc += 32 + ((it.len() / 2 + 31) / 32) * 32;
    }
    for it in items {
        contents.push_str(&abi_encode_bytes(it));
    }
    format!("{:064x}{}{}", n, offsets, contents)
}

/// Build the V4 PositionManager `unlockData` for a fee collect:
/// strict abi.encode(bytes actions, bytes[] params) where actions = [DECREASE_LIQUIDITY,
/// TAKE_PAIR] and params are flat member encodings. Byte-for-byte identical to
/// @uniswap/v4-sdk V4PositionPlanner.finalize() for the same inputs (golden vector test).
pub(crate) fn build_v4_collect_unlock_data(token_id: u64, currency0: &str, currency1: &str, recipient: &str) -> String {
    let actions_hex = format!("{:02x}{:02x}", V4_ACTION_DECREASE_LIQUIDITY, V4_ACTION_TAKE_PAIR);
    // DECREASE_LIQUIDITY flat params: [tokenId u256, liquidity u256 (=0 collects fees),
    // amount0Min u128, amount1Min u128, hookData offset (head-relative, incl. offset word
    // => 5*32 = 0xa0), hookData length (=0)]
    let decrease = format!(
        "{}{}{}{}{}{}",
        word_u256(token_id),
        word_u256(0),
        word_u256(0),
        word_u256(0),
        word_u256(0xa0),
        word_u256(0)
    );
    // TAKE_PAIR flat params: [currency0, currency1, recipient] — all static, 3 words
    let take_pair = format!(
        "{}{}{}",
        word_addr(currency0),
        word_addr(currency1),
        word_addr(recipient)
    );
    let params = vec![decrease, take_pair];
    let actions_len_padded_bytes = ((actions_hex.len() / 2 + 31) / 32) * 32;
    let params_offset = 0x40 + 32 + actions_len_padded_bytes;
    format!(
        "0x{}{}{}{}",
        word_u256(0x40),
        word_u256(params_offset as u64),
        abi_encode_bytes(&actions_hex),
        abi_encode_bytes_array(&params)
    )
}

/// Full calldata for PositionManager.modifyLiquidities(bytes,uint256):
/// [selector, 0x40 (offset to unlockData), deadline, bytesLen, unlockData...]
pub(crate) fn build_v4_collect_calldata(token_id: u64, currency0: &str, currency1: &str, recipient: &str, deadline: u64) -> String {
    format!(
        "0x{}{}{}{}",
        POSM_MODIFY_LIQUIDITIES_SELECTOR,
        word_u256(0x40),
        word_u256(deadline),
        abi_encode_bytes(&build_v4_collect_unlock_data(token_id, currency0, currency1, recipient))
    )
}

/// Canonical Uniswap v3 NPM collect: [selector, tokenId, recipient, amount0Max,
/// amount1Max] with type(uint128).max maxes — sweeps all accrued fees + tokensOwed
/// to the recipient. Only valid on the canonical NPM (Phase-1-verified on Base).
pub(crate) fn build_v3_collect_calldata(token_id: u64, recipient: &str) -> String {
    format!(
        "0x{}{}{}{}{}",
        UNISWAP_V3_COLLECT_SELECTOR,
        word_u256(token_id),
        word_addr(recipient),
        format!("{:064x}", u128::MAX),
        format!("{:064x}", u128::MAX),
    )
}

/// Single-action variant: unlockData with only DECREASE_LIQUIDITY (collects fees to
/// the posm ledger without sweeping them out). Live-verified companion to the full
/// collect; useful for isolating decode vs take failures. Kept (test-exercised via
/// golden vector) even though production paths use the full collect variant.
#[allow(dead_code)]
pub(crate) fn build_v4_collect_unlock_data_decrease_only(token_id: u64) -> String {
    let actions_hex = format!("{:02x}", V4_ACTION_DECREASE_LIQUIDITY);
    let decrease = format!(
        "{}{}{}{}{}{}",
        word_u256(token_id),
        word_u256(0),
        word_u256(0),
        word_u256(0),
        word_u256(0xa0),
        word_u256(0)
    );
    let params = vec![decrease];
    let actions_len_padded_bytes = ((actions_hex.len() / 2 + 31) / 32) * 32;
    let params_offset = 0x40 + 32 + actions_len_padded_bytes;
    format!(
        "0x{}{}{}{}",
        word_u256(0x40),
        word_u256(params_offset as u64),
        abi_encode_bytes(&actions_hex),
        abi_encode_bytes_array(&params)
    )
}
