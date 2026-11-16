/**
 * Display helpers added with Aero concentrated-liquidity positions (v0.6.0, issue #91):
 * what a position card says about fees and staking, and which side of its range a
 * ranged position is on. Pure, so the wording is pinned by aeroCl.test.ts.
 *
 * Nothing here computes a number. Each helper restates what the worker already read:
 * `fees_note` (why there is no LP-claimable fee figure), `staked_in_gauge` (the gauge
 * holding the NFT for this wallet) and the position's own ticks and amounts.
 */

/** Config/snapshot `protocol` value for an Aero concentrated-liquidity position. */
export const AERO_CL_PROTOCOL = 'aerodrome-cl';

/** Both Aero protocols: the basic volatile pool and the concentrated (CL) position. */
export function isAeroProtocol(protocol: string | undefined | null): boolean {
  return protocol === 'aerodrome' || protocol === AERO_CL_PROTOCOL;
}

/** '0x3bf70c880adc8370917994ab0df68e763b7a60fd' → '0x3bf7…60fd'. */
export function shortAddress(address: string): string {
  const a = String(address ?? '');
  return a.length > 12 ? `${a.slice(0, 6)}…${a.slice(-4)}` : a;
}

/**
 * The fees part of a position card. When the worker says the fee figures are not a
 * read of LP-claimable fees, its reason is shown in their place: a staked Aero CL
 * position earns no swap fees for the LP, and "$0.000000" would read as "$0 earned".
 */
export function positionFeesText(p: { fees_usd?: string | null; fees_note?: string | null }): string {
  const note = (p.fees_note ?? '').trim();
  if (note) return `Fees: ${note}`;
  if (p.fees_usd === undefined || p.fees_usd === null || String(p.fees_usd).trim() === '') {
    return 'Fees: unavailable';
  }
  return `Fees: $${p.fees_usd}`;
}

/** Staking line for a position card; null when the wallet holds the NFT itself. */
export function stakingText(p: { staked_in_gauge?: string | null }): string | null {
  const gauge = (p.staked_in_gauge ?? '').trim();
  if (!gauge) return null;
  return `Staked in gauge ${shortAddress(gauge)} for this wallet`;
}

export type RangeSide =
  /** Spans every price; there is no side to be on. */
  | 'full_range'
  /** A ranged position that holds nothing: neither in nor out of range. */
  | 'empty'
  /** The pool's price is inside the range: the position holds both tokens. */
  | 'in_range'
  /** Pool tick below the range: the position holds only token0. */
  | 'all_token0'
  /** Pool tick at or above the range's upper tick: the position holds only token1. */
  | 'all_token1'
  /** Out of range, but neither the pool tick nor the amounts say which side. */
  | 'out_unknown';

interface RangedPosition {
  in_range: boolean;
  full_range?: boolean;
  lower_tick?: number;
  upper_tick?: number;
  amount0?: string;
  amount1?: string;
  token0?: { symbol: string };
  token1?: { symbol: string };
  pool_active?: boolean | null;
  liquidity_amount?: string;
}

/**
 * A ranged position that holds nothing: its liquidity is 0 (the worker's own rule,
 * lp_rebuild.rs: "neither in nor out of range") or, when the liquidity is not given,
 * both of its amounts read 0. Unparsable or missing amounts are not "empty".
 */
export function isEmptyRangedPosition(p: RangedPosition): boolean {
  if (p.full_range) return false;
  const liquidity = (p.liquidity_amount ?? '').trim();
  if (/^\d+$/.test(liquidity)) return /^0+$/.test(liquidity);
  const a0 = parseFloat(p.amount0 ?? '');
  const a1 = parseFloat(p.amount1 ?? '');
  return Number.isFinite(a0) && Number.isFinite(a1) && a0 === 0 && a1 === 0;
}

/**
 * Which side of its range a position is on. The pool's current tick decides when it is
 * known (`pool_mechanics[i].tick_current`, index-aligned with `positions`); otherwise
 * the position's own amounts do, since an out-of-range position holds a single token.
 * Never guessed: with neither, the side is 'out_unknown'.
 */
export function rangeSide(p: RangedPosition, tickCurrent?: number | null): RangeSide {
  if (p.full_range) return 'full_range';
  // Before the side: the worker's in_range flag compares ticks only, so an emptied
  // position can still read "in range" while holding neither token.
  if (isEmptyRangedPosition(p)) return 'empty';
  if (p.in_range) return 'in_range';
  if (
    typeof tickCurrent === 'number' &&
    Number.isFinite(tickCurrent) &&
    typeof p.lower_tick === 'number' &&
    typeof p.upper_tick === 'number'
  ) {
    if (tickCurrent < p.lower_tick) return 'all_token0';
    if (tickCurrent >= p.upper_tick) return 'all_token1';
  }
  const a0 = parseFloat(p.amount0 ?? '');
  const a1 = parseFloat(p.amount1 ?? '');
  if (Number.isFinite(a0) && Number.isFinite(a1)) {
    if (a0 > 0 && a1 === 0) return 'all_token0';
    if (a1 > 0 && a0 === 0) return 'all_token1';
  }
  return 'out_unknown';
}

/**
 * The side in words, from NOCK's point of view when NOCK is one of the tokens: a range
 * left behind by a rising NOCK has sold all of it; one left behind by a falling NOCK
 * holds only NOCK. A stale pool's tick is frozen, so no side is claimed for it.
 */
export function rangeSideText(p: RangedPosition, tickCurrent?: number | null): string {
  if (p.pool_active === false) {
    return 'stale pool: no in-range liquidity, so its price is frozen and range status says nothing about the market';
  }
  const side = rangeSide(p, tickCurrent);
  const s0 = p.token0?.symbol ?? 'token0';
  const s1 = p.token1?.symbol ?? 'token1';
  if (side === 'full_range') return 'full range (cannot go out of range)';
  if (side === 'empty') return `empty: holds no ${s0} and no ${s1}, so it is neither in nor out of range`;
  if (side === 'in_range') return `in range, holding ${s0} and ${s1}`;
  if (side === 'out_unknown') return 'out of range';
  const [held, other] = side === 'all_token0' ? [s0, s1] : [s1, s0];
  if (held === 'NOCK') {
    return `out of range below: NOCK is priced under this range, which now holds only NOCK`;
  }
  if (other === 'NOCK') {
    return `out of range above: NOCK is priced over this range, which has sold all its NOCK and holds only ${held}`;
  }
  return `out of range: holds only ${held}`;
}

/**
 * Whether the range line should be drawn as good (in range / full range), as a warning
 * (out of range), or neutral (an empty position: nothing is at stake either way).
 */
export function rangeSideTone(
  p: RangedPosition,
  tickCurrent?: number | null,
): 'good' | 'warn' | 'stale' | 'neutral' {
  if (p.pool_active === false) return 'stale';
  const side = rangeSide(p, tickCurrent);
  if (side === 'empty') return 'neutral';
  return side === 'in_range' || side === 'full_range' ? 'good' : 'warn';
}
