//! Exact integer mint amounts (review of #166 rungs, finding 4): the liquidity a mint of
//! given token amounts gets and the amounts that liquidity pulls, with Uniswap's own
//! integer formulas (TickMath.getSqrtRatioAtTick, LiquidityAmounts.getLiquidityFor*,
//! SqrtPriceMath.getAmount*Delta rounding up), so a transaction builder can use
//! `liquidity_raw` with `amount*_raw` as maxima that never revert for a shortfall and
//! never exceed the amounts stated. No floats. A small unsigned big integer, enough for
//! 512-bit intermediates; no crate is added.
use std::cmp::Ordering;

/// An unsigned integer, little-endian 64-bit limbs, no trailing zero limbs.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Big(Vec<u64>);

impl Big {
    pub(crate) fn zero() -> Big {
        Big(Vec::new())
    }
    pub(crate) fn from_u128(v: u128) -> Big {
        let mut b = Big(vec![v as u64, (v >> 64) as u64]);
        b.trim();
        b
    }
    fn trim(&mut self) {
        while self.0.last() == Some(&0) {
            self.0.pop();
        }
    }
    pub(crate) fn is_zero(&self) -> bool {
        self.0.is_empty()
    }
    /// From big-endian hex (with or without 0x).
    pub(crate) fn from_hex(h: &str) -> Option<Big> {
        let h = h.trim().trim_start_matches("0x");
        if h.is_empty() || !h.chars().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let mut out = Big::zero();
        for c in h.chars() {
            out = out.shl(4).add(&Big::from_u128(c.to_digit(16)? as u128));
        }
        Some(out)
    }
    pub(crate) fn to_u128(&self) -> Option<u128> {
        match self.0.len() {
            0 => Some(0),
            1 => Some(self.0[0] as u128),
            2 => Some(self.0[0] as u128 | ((self.0[1] as u128) << 64)),
            _ => None,
        }
    }
    pub(crate) fn to_dec(&self) -> String {
        if self.is_zero() {
            return "0".to_string();
        }
        let mut digits = Vec::new();
        let mut v = self.clone();
        let ten18 = Big::from_u128(1_000_000_000_000_000_000);
        while !v.is_zero() {
            let (q, r) = v.divmod(&ten18);
            digits.push(r.to_u128().unwrap_or(0));
            v = q;
        }
        let mut s = format!("{}", digits.pop().unwrap_or(0));
        while let Some(d) = digits.pop() {
            s.push_str(&format!("{d:018}"));
        }
        s
    }
    fn bits(&self) -> usize {
        match self.0.last() {
            None => 0,
            Some(top) => (self.0.len() - 1) * 64 + (64 - top.leading_zeros() as usize),
        }
    }
    fn bit(&self, i: usize) -> bool {
        self.0.get(i / 64).map_or(false, |w| (w >> (i % 64)) & 1 == 1)
    }
    pub(crate) fn add(&self, o: &Big) -> Big {
        let n = self.0.len().max(o.0.len());
        let mut out = Vec::with_capacity(n + 1);
        let mut carry = 0u128;
        for i in 0..n {
            let s = *self.0.get(i).unwrap_or(&0) as u128 + *o.0.get(i).unwrap_or(&0) as u128 + carry;
            out.push(s as u64);
            carry = s >> 64;
        }
        if carry > 0 {
            out.push(carry as u64);
        }
        let mut b = Big(out);
        b.trim();
        b
    }
    /// self - o; panics on underflow (callers compare first).
    pub(crate) fn sub(&self, o: &Big) -> Big {
        assert!(self.cmp(o) != Ordering::Less, "Big::sub underflow");
        let mut out = Vec::with_capacity(self.0.len());
        let mut borrow = 0i128;
        for i in 0..self.0.len() {
            let mut d = self.0[i] as i128 - *o.0.get(i).unwrap_or(&0) as i128 - borrow;
            borrow = if d < 0 {
                d += 1i128 << 64;
                1
            } else {
                0
            };
            out.push(d as u64);
        }
        let mut b = Big(out);
        b.trim();
        b
    }
    pub(crate) fn mul(&self, o: &Big) -> Big {
        if self.is_zero() || o.is_zero() {
            return Big::zero();
        }
        let mut out = vec![0u64; self.0.len() + o.0.len()];
        for (i, &a) in self.0.iter().enumerate() {
            let mut carry = 0u128;
            for (j, &b) in o.0.iter().enumerate() {
                let t = a as u128 * b as u128 + out[i + j] as u128 + carry;
                out[i + j] = t as u64;
                carry = t >> 64;
            }
            out[i + o.0.len()] = carry as u64;
        }
        let mut b = Big(out);
        b.trim();
        b
    }
    pub(crate) fn shl(&self, n: usize) -> Big {
        if self.is_zero() {
            return Big::zero();
        }
        let (w, b) = (n / 64, n % 64);
        let mut out = vec![0u64; w];
        let mut carry = 0u64;
        for &x in &self.0 {
            out.push(if b == 0 { x } else { (x << b) | carry });
            carry = if b == 0 { 0 } else { x >> (64 - b) };
        }
        if carry > 0 {
            out.push(carry);
        }
        let mut r = Big(out);
        r.trim();
        r
    }
    pub(crate) fn shr(&self, n: usize) -> Big {
        let (w, b) = (n / 64, n % 64);
        if w >= self.0.len() {
            return Big::zero();
        }
        let src = &self.0[w..];
        let mut out = Vec::with_capacity(src.len());
        for i in 0..src.len() {
            let lo = src[i] >> b;
            let hi = if b == 0 { 0 } else { src.get(i + 1).map_or(0, |x| x << (64 - b)) };
            out.push(lo | hi);
        }
        let mut r = Big(out);
        r.trim();
        r
    }
    /// (quotient, remainder); panics on division by zero.
    pub(crate) fn divmod(&self, d: &Big) -> (Big, Big) {
        assert!(!d.is_zero(), "Big::divmod by zero");
        if self.cmp(d) == Ordering::Less {
            return (Big::zero(), self.clone());
        }
        let mut q = vec![0u64; self.0.len()];
        let mut r = Big::zero();
        for i in (0..self.bits()).rev() {
            r = r.shl(1);
            if self.bit(i) {
                r = r.add(&Big::from_u128(1));
            }
            if r.cmp(d) != Ordering::Less {
                r = r.sub(d);
                q[i / 64] |= 1 << (i % 64);
            }
        }
        let mut qb = Big(q);
        qb.trim();
        (qb, r)
    }
    pub(crate) fn mul_div(&self, b: &Big, d: &Big) -> Big {
        self.mul(b).divmod(d).0
    }
    pub(crate) fn mul_div_up(&self, b: &Big, d: &Big) -> Big {
        let (q, r) = self.mul(b).divmod(d);
        if r.is_zero() { q } else { q.add(&Big::from_u128(1)) }
    }
    pub(crate) fn div_up(&self, d: &Big) -> Big {
        let (q, r) = self.divmod(d);
        if r.is_zero() { q } else { q.add(&Big::from_u128(1)) }
    }
}

impl PartialOrd for Big {
    fn partial_cmp(&self, o: &Big) -> Option<Ordering> {
        Some(self.cmp(o))
    }
}
impl Ord for Big {
    fn cmp(&self, o: &Big) -> Ordering {
        self.0.len().cmp(&o.0.len()).then_with(|| {
            for i in (0..self.0.len()).rev() {
                match self.0[i].cmp(&o.0[i]) {
                    Ordering::Equal => continue,
                    x => return x,
                }
            }
            Ordering::Equal
        })
    }
}

fn q96() -> Big {
    Big::from_u128(1).shl(96)
}

/// TickMath.getSqrtRatioAtTick (Uniswap v3 and v4), exactly. None outside the tick range.
pub(crate) fn sqrt_ratio_at_tick(tick: i32) -> Option<Big> {
    const MAX_TICK: i32 = 887272;
    if tick.abs() > MAX_TICK {
        return None;
    }
    let abs = tick.unsigned_abs();
    let consts: [(u32, u128); 19] = [
        (0x2, 0xfff97272373d413259a46990580e213a),
        (0x4, 0xfff2e50f5f656932ef12357cf3c7fdcc),
        (0x8, 0xffe5caca7e10e4e61c3624eaa0941cd0),
        (0x10, 0xffcb9843d60f6159c9db58835c926644),
        (0x20, 0xff973b41fa98c081472e6896dfb254c0),
        (0x40, 0xff2ea16466c96a3843ec78b326b52861),
        (0x80, 0xfe5dee046a99a2a811c461f1969c3053),
        (0x100, 0xfcbe86c7900a88aedcffc83b479aa3a4),
        (0x200, 0xf987a7253ac413176f2b074cf7815e54),
        (0x400, 0xf3392b0822b70005940c7a398e4b70f3),
        (0x800, 0xe7159475a2c29b7443b29c7fa6e889d9),
        (0x1000, 0xd097f3bdfd2022b8845ad8f792aa5825),
        (0x2000, 0xa9f746462d870fdf8a65dc1f90e061e5),
        (0x4000, 0x70d869a156d2a1b890bb3df62baf32f7),
        (0x8000, 0x31be135f97d08fd981231505542fcfa6),
        (0x10000, 0x9aa508b5b7a84e1c677de54f3e99bc9),
        (0x20000, 0x5d6af8dedb81196699c329225ee604),
        (0x40000, 0x2216e584f5fa1ea926041bedfe98),
        (0x80000, 0x48a170391f7dc42444e8fa2),
    ];
    let mut ratio = if abs & 1 != 0 { Big::from_u128(0xfffcb933bd6fad37aa2d162d1a594001) } else { Big::from_u128(1).shl(128) };
    for (bit, c) in consts {
        if abs & bit != 0 {
            ratio = ratio.mul(&Big::from_u128(c)).shr(128);
        }
    }
    if tick > 0 {
        let max = Big::from_u128(1).shl(256).sub(&Big::from_u128(1));
        ratio = max.divmod(&ratio).0;
    }
    let (q, r) = ratio.divmod(&Big::from_u128(1).shl(32));
    Some(if r.is_zero() { q } else { q.add(&Big::from_u128(1)) })
}

fn liquidity_for_amount0(sa: &Big, sb: &Big, amount0: &Big) -> Big {
    let intermediate = sa.mul_div(sb, &q96());
    amount0.mul_div(&intermediate, &sb.sub(sa))
}

fn liquidity_for_amount1(sa: &Big, sb: &Big, amount1: &Big) -> Big {
    amount1.mul_div(&q96(), &sb.sub(sa))
}

/// LiquidityAmounts.getLiquidityForAmounts: the most liquidity the amounts pay for,
/// rounded down.
pub(crate) fn liquidity_for_amounts(sqrt_p: &Big, sa: &Big, sb: &Big, amount0: &Big, amount1: &Big) -> Big {
    if sqrt_p <= sa {
        liquidity_for_amount0(sa, sb, amount0)
    } else if sqrt_p < sb {
        liquidity_for_amount0(sqrt_p, sb, amount0).min(liquidity_for_amount1(sa, sqrt_p, amount1))
    } else {
        liquidity_for_amount1(sa, sb, amount1)
    }
}

/// SqrtPriceMath.getAmount0Delta, rounding up.
fn amount0_up(sa: &Big, sb: &Big, l: &Big) -> Big {
    l.shl(96).mul_div_up(&sb.sub(sa), sb).div_up(sa)
}

/// SqrtPriceMath.getAmount1Delta, rounding up.
fn amount1_up(sa: &Big, sb: &Big, l: &Big) -> Big {
    l.mul_div_up(&sb.sub(sa), &q96())
}

/// What a mint of `l` on [tick_lower, tick_upper) pulls at the pool's tick and sqrt
/// price, rounded up as the pool rounds it (v4 Pool.modifyLiquidity by tick).
pub(crate) fn amounts_for_liquidity(tick: i32, sqrt_p: &Big, tick_lower: i32, tick_upper: i32, l: &Big) -> Option<(Big, Big)> {
    let (sa, sb) = (sqrt_ratio_at_tick(tick_lower)?, sqrt_ratio_at_tick(tick_upper)?);
    Some(if tick < tick_lower {
        (amount0_up(&sa, &sb, l), Big::zero())
    } else if tick < tick_upper {
        // The tick decides the side; a sqrt price a rounding outside the band is clamped.
        let p = sqrt_p.clone().max(sa.clone()).min(sb.clone());
        (amount0_up(&p, &sb, l), amount1_up(&sa, &p, l))
    } else {
        (Big::zero(), amount1_up(&sa, &sb, l))
    })
}

/// The exact mint for at most `max0`/`max1` raw tokens: the liquidity (rounded down,
/// then lowered until the amounts it pulls fit) and those amounts. None when the ticks
/// are not valid or the amounts buy no liquidity.
pub(crate) fn exact_mint(tick: i32, sqrt_p: &Big, tick_lower: i32, tick_upper: i32, max0: &Big, max1: &Big) -> Option<(Big, Big, Big)> {
    if tick_lower >= tick_upper {
        return None;
    }
    let (sa, sb) = (sqrt_ratio_at_tick(tick_lower)?, sqrt_ratio_at_tick(tick_upper)?);
    // The pool decides the side by its tick; the sqrt price must agree with it.
    let p = if tick < tick_lower { sa.clone() } else if tick >= tick_upper { sb.clone() } else { sqrt_p.clone().max(sa.clone()).min(sb.clone()) };
    let mut l = liquidity_for_amounts(&p, &sa, &sb, max0, max1);
    for _ in 0..8 {
        if l.is_zero() {
            return None;
        }
        let (a0, a1) = amounts_for_liquidity(tick, sqrt_p, tick_lower, tick_upper, &l)?;
        if &a0 <= max0 && &a1 <= max1 {
            return Some((l, a0, a1));
        }
        l = l.sub(&Big::from_u128(1));
    }
    None
}

/// Raw units as a decimal string with `decimals` places.
pub(crate) fn raw_to_decimal(raw: &Big, decimals: u32) -> String {
    let s = raw.to_dec();
    let d = decimals as usize;
    if d == 0 {
        return s;
    }
    let s = format!("{s:0>width$}", width = d + 1);
    format!("{}.{}", &s[..s.len() - d], &s[s.len() - d..])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sqrt_ratio_at_tick_matches_uniswap() {
        // TickMath's MIN_SQRT_RATIO and MAX_SQRT_RATIO, 2^96 at tick 0, and tick -1.
        assert_eq!(sqrt_ratio_at_tick(-887272).unwrap().to_dec(), "4295128739");
        assert_eq!(sqrt_ratio_at_tick(887272).unwrap().to_dec(), "1461446703485210103287273052203988822378723970342");
        assert_eq!(sqrt_ratio_at_tick(0).unwrap(), Big::from_u128(1).shl(96));
        assert_eq!(sqrt_ratio_at_tick(-1).unwrap().to_dec(), "79224201403219477170569942574");
        // The 3% pool's band edge, as /tmp/nn-v090/rungs_fixture/exact_mint.py has it.
        assert_eq!(sqrt_ratio_at_tick(273600).unwrap().to_dec(), "69140267990916712638494518778479670");
        assert!(sqrt_ratio_at_tick(887273).is_none());
    }

    #[test]
    fn test_big_arithmetic_round_trips() {
        let a = Big::from_hex("0d69693142532bf9792f0b505c97d4").unwrap();
        let b = Big::from_u128(1_000_000_007);
        let (q, r) = a.mul(&b).add(&Big::from_u128(5)).divmod(&b);
        assert_eq!((q, r), (a.clone(), Big::from_u128(5)));
        assert_eq!(a.shl(70).shr(70), a);
        assert_eq!(raw_to_decimal(&Big::from_u128(1225999), 6), "1.225999");
        assert_eq!(raw_to_decimal(&Big::from_u128(5), 6), "0.000005");
    }
}
