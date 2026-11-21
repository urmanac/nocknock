use crate::*;

/// Uniswap's tick bounds; a position spanning both is full range (it can never go
/// out of range). Aerodrome volatile pools are modelled this way.
pub(crate) const MIN_TICK: i32 = -887272;
pub(crate) const MAX_TICK: i32 = 887272;

pub(crate) fn is_full_range(tick_lower: i32, tick_upper: i32) -> bool {
    tick_lower <= MIN_TICK && tick_upper >= MAX_TICK
}

/// Represents an unsigned 256-bit integer as (hi: u128, lo: u128)
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct U256(pub u128, pub u128);

impl serde::Serialize for U256 {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_str(&format!("0x{}", self.to_hex_be()))
    }
}

impl<'de> serde::Deserialize<'de> for U256 {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let s = String::deserialize(deserializer)?;
        U256::from_hex_be(&s).map_err(serde::de::Error::custom)
    }
}

impl U256 {
    pub fn from_hex_be(hex_str: &str) -> Result<Self> {
        let clean = hex_str.trim_start_matches("0x");
        if clean.len() > 64 {
            return Err(anyhow::anyhow!("Hex string too long for U256: {}", clean.len()));
        }
        let padded = format!("{:0>64}", clean);
        let hi = u128::from_str_radix(&padded[0..32], 16)?;
        let lo = u128::from_str_radix(&padded[32..64], 16)?;
        Ok(U256(hi, lo))
    }

    /// Wrapping subtraction modulo 2^256
    pub fn wrapping_sub(&self, other: &Self) -> Self {
        let (lo, borrow) = self.1.overflowing_sub(other.1);
        let hi = self.0.wrapping_sub(other.0).wrapping_sub(borrow as u128);
        U256(hi, lo)
    }

    /// Computes (self * liquidity) >> 128 where liquidity is u128
    pub fn mul_div_q128(&self, liquidity: u128) -> u128 {
        let a = self.1 >> 64;
        let b = self.1 & 0xFFFF_FFFF_FFFF_FFFF;
        let c = liquidity >> 64;
        let d = liquidity & 0xFFFF_FFFF_FFFF_FFFF;

        let bd = b * d;
        let ad = a * d;
        let bc = b * c;
        let ac = a * c;

        let mid = (bd >> 64) + (ad & 0xFFFF_FFFF_FFFF_FFFF) + (bc & 0xFFFF_FFFF_FFFF_FFFF);
        let hi_part = ac + (ad >> 64) + (bc >> 64) + (mid >> 64);

        self.0.wrapping_mul(liquidity).wrapping_add(hi_part)
    }

    pub fn to_hex_be(&self) -> String {
        format!("{:032x}{:032x}", self.0, self.1)
    }

    /// Lossy f64 value (magnitude only; used for human-unit pricing math).
    pub fn to_f64(&self) -> f64 {
        self.0 as f64 * 18446744073709551616.0 * 18446744073709551616.0 + self.1 as f64
    }
}

/// Uniswap sqrt-price → raw price (token1_raw per token0_raw).
pub(crate) fn sqrt_price_x96_to_raw(sqrt_price_x96: U256) -> f64 {
    let s = sqrt_price_x96.to_f64() / 79228162514264337593543950336.0; // 2^96
    s * s
}

/// sqrt price at a tick boundary: 1.0001^(tick/2).
pub(crate) fn tick_sqrt_price(tick: i32) -> f64 {
    1.0001f64.powf(tick as f64 / 2.0)
}

/// Current position amounts from liquidity + slot0 + tick bounds (raw token units).
/// In-range splits across both tokens; out-of-range positions are single-sided.
pub(crate) fn concentrated_amounts_raw(liquidity: u128, sqrt_price_x96: U256, tick_lower: i32, tick_upper: i32) -> (f64, f64) {
    let l = liquidity as f64;
    let s = sqrt_price_x96_to_raw(sqrt_price_x96).sqrt();
    let sa = tick_sqrt_price(tick_lower);
    let sb = tick_sqrt_price(tick_upper);
    concentrated_amounts_at(l, s, sa, sb)
}

/// The body of `concentrated_amounts_raw` on f64 inputs: liquidity `l`, the pool's sqrt
/// price `s` in raw units (sqrt(token1_raw / token0_raw)) and the range's edges `sa`, `sb`.
/// Split out so the floor-breach price (#165) can evaluate a range at a moved price; the
/// expressions are unchanged.
pub(crate) fn concentrated_amounts_at(l: f64, s: f64, sa: f64, sb: f64) -> (f64, f64) {
    if s <= sa {
        // Entirely token0: L * (1/sa - 1/sb)
        (l * (1.0 / sa - 1.0 / sb), 0.0)
    } else if s >= sb {
        // Entirely token1: L * (sb - sa)
        (0.0, l * (sb - sa))
    } else {
        let amount0 = l * (sb - s) / (s * sb);
        let amount1 = l * (s - sa);
        (amount0, amount1)
    }
}

/// Human display price for a tick in token0-per-token1 units:
/// 1.0001^(-tick) * 10^(dec1 - dec0). For USDC(6)/NOCK(16) this is USDC per NOCK
/// (the historical bracket convention); for ETH(18)/NOCK(16) it is ETH per NOCK.
pub(crate) fn tick_to_display_price_dec(tick: i32, dec0: i32, dec1: i32) -> f64 {
    1.0001f64.powi(-tick) * 10f64.powi(dec1 - dec0)
}

pub(crate) fn parse_hex_word_u128(hex_word: &str) -> Result<u128> {
    u128::from_str_radix(hex_word.trim_start_matches("0x"), 16)
        .map_err(|e| anyhow::anyhow!("bad hex word: {e}"))
}

/// A signed 32-bit value (an `int24` tick, say) from an ABI word. Negative values arrive
/// sign-extended across the whole 32-byte word (`ff…fe98` is -360), so the value is the
/// low four bytes read as two's complement; the bytes above them must be that sign
/// extension, or the word is not a small signed integer at all. (Issue #91: this used to
/// parse the whole word as u32 and so rejected every negative tick.)
pub(crate) fn parse_hex_word_i32(hex_word: &str) -> Result<i32> {
    let clean = hex_word.trim_start_matches("0x");
    if clean.is_empty() || clean.len() > 64 || !clean.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(anyhow::anyhow!("bad hex word: '{clean}'"));
    }
    if clean.len() <= 8 {
        let v = u32::from_str_radix(clean, 16).map_err(|e| anyhow::anyhow!("bad hex word: {e}"))?;
        return Ok(v as i32);
    }
    let (high, low) = clean.split_at(clean.len() - 8);
    let v = u32::from_str_radix(low, 16).map_err(|e| anyhow::anyhow!("bad hex word: {e}"))? as i32;
    let fill = if v < 0 { b'f' } else { b'0' };
    // A short (unpadded) positive word has no sign extension to check beyond zeros.
    if !high.bytes().all(|b| b.to_ascii_lowercase() == fill) {
        return Err(anyhow::anyhow!("hex word does not fit a signed 32-bit integer: '{clean}'"));
    }
    Ok(v)
}

/// ABI-encode a signed tick (`int24`) as a 32-byte calldata word, sign-extended:
/// -360 becomes `ff…fe98`. (`{:064x}` of `tick as u32` zero-pads a negative tick, which
/// the contract rejects or reads as a different tick.)
pub(crate) fn int24_calldata_word(tick: i32) -> String {
    if tick < 0 {
        format!("{}{:08x}", "f".repeat(56), tick as u32)
    } else {
        format!("{:064x}", tick as u32)
    }
}

/// A position is in range while the pool's tick is at or above its lower tick and
/// below its upper tick (the upper bound is exclusive: at `tick == tick_upper` the
/// range holds only token1).
pub(crate) fn tick_in_range(tick: i32, tick_lower: i32, tick_upper: i32) -> bool {
    tick_lower <= tick && tick < tick_upper
}

/// Exact decimal formatting: raw integer amount -> human string, no float rounding.
/// "120033" @6dp -> "0.120033"; "54468217823004916" @16dp -> "5.4468217823004916".
pub(crate) fn format_units(amount: u128, decimals: i32) -> String {
    let digits = amount.to_string();
    let d = decimals.max(0) as usize;
    let s = if digits.len() <= d {
        format!("0.{}{}", "0".repeat(d - digits.len()), digits)
    } else {
        format!("{}.{}", &digits[..digits.len() - d], &digits[digits.len() - d..])
    };
    if s.contains('.') {
        let trimmed = s.trim_end_matches('0').trim_end_matches('.');
        if !trimmed.is_empty() {
            return trimmed.to_string();
        }
    }
    s
}
