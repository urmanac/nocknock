/**
 * Position display helpers (v2.0.0 Uniswap-only portfolio).
 *
 * Shared by the Dashboard position cards / chart legends and the contract tests,
 * so the rendering conventions stay pinned to the discovery fixtures.
 */

/** Series colors for the per-position stacked chart. */
export const POSITION_COLORS = ['#38bdf8', '#a855f7', '#f59e0b', '#ec4899', '#14b8a6', '#f97316'];

/** Human label for a tracked position: pair + protocol badge + fee tier. */
export function positionLabel(p: any): string {
  if (!p) return 'Position';
  const proto =
    // The `aerodrome` protocol key is kept for ledger continuity; the venue is Aero now.
    p.protocol === 'uniswap-v4' ? 'v4'
      : p.protocol === 'uniswap-v3' ? 'v3'
      : p.protocol === 'aerodrome' ? 'Aero'
      // Aero concentrated liquidity (#91): "CL200" is how the Aero app names the pool.
      : p.protocol === 'aerodrome-cl' ? `Aero CL${Number.isFinite(p.tick_spacing) ? p.tick_spacing : ''}`
      : p.protocol;
  const fee = (p.fee_tier_bps / 100).toFixed(2).replace(/\.?0+$/, '');
  return `${p.token0.symbol}/${p.token1.symbol} ${proto} · ${fee}%`;
}

/**
 * One-line summary of the tracked LP positions, computed from the positions themselves.
 * Replaces a line that hardcoded "0% realized losses" and called every position a
 * "concentrated Uniswap position", counting a dead pool as in range.
 */
export function positionsSummary(
  positions: Array<{ in_range: boolean; full_range?: boolean; pool_active?: boolean | null }>,
): string {
  const n = positions.length;
  if (n === 0) return 'No LP positions tracked.';
  const full = positions.filter((p) => p.full_range).length;
  const stale = positions.filter((p) => !p.full_range && p.pool_active === false).length;
  const ranged = positions.filter((p) => !p.full_range);
  const inRange = ranged.filter((p) => p.in_range && p.pool_active !== false).length;
  const out = ranged.length - inRange - stale;
  const parts: string[] = [];
  if (ranged.length > 0) {
    const detail = [`${inRange} in range`];
    if (out > 0) detail.push(`${out} out of range`);
    if (stale > 0) detail.push(`${stale} on a stale pool`);
    parts.push(`${ranged.length} ranged (${detail.join(', ')})`);
  }
  if (full > 0) parts.push(`${full} full range`);
  return `${n} LP position${n === 1 ? '' : 's'} tracked live: ${parts.join(', ')}.`;
}

/**
 * Stat card value: '—' unless the value parses to a finite number (a known 0 renders
 * as 0). Replaces a formatter that printed 'NaN NOCK' for a zero stack.
 */
export function statValueText(value: string | undefined | null, prefix = '', suffix = ''): string {
  if (value === undefined || value === null) return '—';
  const n = parseFloat(String(value).replace(/,/g, ''));
  if (!Number.isFinite(n)) return '—';
  return `${prefix}${n.toLocaleString('en-US', { maximumFractionDigits: 2 })}${suffix}`;
}

/** Protocol pill text for a position card. */
export function protocolBadge(protocol: string): string {
  if (protocol === 'uniswap-v4') return 'UNISWAP V4';
  if (protocol === 'uniswap-v3') return 'UNISWAP V3';
  if (protocol === 'aerodrome') return 'AERO';
  if (protocol === 'aerodrome-cl') return 'AERO CL';
  return String(protocol ?? '').toUpperCase();
}

/**
 * Readable range string. The API's lower_price/upper_price are token0-per-token1
 * (documented in snapshot.yaml); when that value is sub-cent tiny (ETH-per-NOCK ≈
 * 0.0000081) the display inverts to token1-per-token0 so numbers stay readable —
 * matching the operator's Uniswap-app convention for both screenshot shapes.
 */
export function positionRangeText(p: any): string {
  // A volatile x*y=k pool covers every price: there is no range to print or to leave.
  if (p.full_range) return 'Full range (cannot go out of range)';
  const lower = parseFloat(p.lower_price);
  const upper = parseFloat(p.upper_price);
  const fmt = (v: number) =>
    v >= 1000
      ? Math.round(v).toLocaleString()
      : v.toPrecision(4).replace(/\.?0+$/, '');
  if (lower > 0 && lower < 0.001) {
    return `${fmt(1 / upper)} – ${fmt(1 / lower)} ${p.token1.symbol} per ${p.token0.symbol}`;
  }
  return `${fmt(lower)} – ${fmt(upper)} ${p.token0.symbol} per ${p.token1.symbol}`;
}

/**
 * Range badge for a position card (2.2.0). A pool with no in-range liquidity has a
 * frozen price, so "in range" judged against that pool's own tick says nothing about
 * the market: show STALE POOL instead of a reassuring green pill.
 */
export function rangeStatus(p: { in_range: boolean; pool_active?: boolean | null; full_range?: boolean }): {
  label: string;
  tone: 'good' | 'warn' | 'stale';
} {
  if (p.pool_active === false) return { label: '● STALE POOL', tone: 'stale' };
  if (p.full_range) return { label: '● FULL RANGE', tone: 'good' };
  return p.in_range ? { label: '● IN RANGE', tone: 'good' } : { label: '● OUT OF RANGE', tone: 'warn' };
}

/**
 * Wallet NOCK in TOKEN UNITS for the floor chart (issue #85).
 *
 * `safe_haven_series` is the wallet's NOCK valued in USD; plotting it against the
 * 100,000 NOCK floor drew a 95,599-NOCK stack at 1,095. Snapshots from v0.5.0 carry
 * `wallet_nock_series` directly. Older snapshots do not, so the units are recovered
 * exactly from the same snapshot's own numbers (USD value ÷ its spot price). When
 * neither is available the point is null: a gap in the line, never a guess.
 */
export function walletNockUnits(
  ag: { wallet_nock_series?: string[]; safe_haven_series?: string[] } | null | undefined,
  i: number,
  spotUsd: number | null | undefined,
): number | null {
  const direct = parseFloat(ag?.wallet_nock_series?.[i] ?? '');
  if (Number.isFinite(direct)) return direct;
  const usd = parseFloat(ag?.safe_haven_series?.[i] ?? '');
  if (Number.isFinite(usd) && spotUsd && spotUsd > 0) return usd / spotUsd;
  return null;
}

/** Velocity panel row for one fee contract. A pool whose claimFees() reverts is not
 *  "$0 earned": its fees are not LP-claimable. A row that carries the worker's own
 *  `note` prints it: the reason (v0.6.0, #91) followed by the position's unclaimed AERO
 *  or why that was not read (#93). A snapshot from before #93 has no note for an Aero
 *  basic pool; its AERO is then "not reported", never shown as 0. A claimable row
 *  states its fee action and gas economics only: an AERO amount beside CLAIM_NOW or
 *  ACCUMULATE would read as advice about AERO, so no note is printed there (the worker
 *  sends none; the amount is on the position card and the pillar lines). */
export function velocityRowText(g: {
  name: string;
  claimable: number;
  gasBreakEven: number;
  ratio: number;
  action: string;
  note?: string | null;
}): string {
  if (g.action === 'not_claimable') {
    const note = (g.note ?? '').trim();
    return `${g.name}: ${note || 'not LP-claimable (claimFees() reverts); unclaimed AERO not reported'}`;
  }
  return `${g.name}: ${String(g.action).toUpperCase()} ($${g.claimable} earned / gas $${g.gasBreakEven} — ${Number(g.ratio).toFixed(0)}x ROI)`;
}

/**
 * True when a position has a NOCK leg (token0 or token1 is NOCK): the same symbol rule
 * as the worker's `is_nock_pair` (#151). Only these are NOCK LP. A range with no NOCK
 * leg (a gas-side WETH/USDC range) is measured, never steered: it is in the totals but
 * not on the NOCK price chart, the NOCK stack chart or the NOCK goal.
 */
export function hasNockLeg(pos: { token0?: { symbol?: string } | null; token1?: { symbol?: string } | null } | null | undefined): boolean {
  return pos?.token0?.symbol === 'NOCK' || pos?.token1?.symbol === 'NOCK';
}

/**
 * The position a tick bracket belongs to (#152). Since 2.8.2 a bracket names its
 * position (`position_id`); an older snapshot's brackets are one per position, in
 * positions order, so the index is the fallback.
 */
export function bracketPosition<P extends { position_id?: string }>(
  bracket: { position_id?: string | null; [k: string]: unknown } | null | undefined,
  index: number,
  positions: P[],
): P | undefined {
  const id = bracket?.position_id;
  if (id) return positions.find((p) => p.position_id === id);
  return positions[index];
}

/** The tick bracket of a position (#152): by id, else by its index in positions. */
export function positionBracket<B extends { position_id?: string | null; [k: string]: unknown }>(
  position: { position_id?: string } | null | undefined,
  positions: Array<{ position_id?: string }>,
  brackets: B[],
): B | undefined {
  if (!position) return undefined;
  const byId = position.position_id ? brackets.find((b) => b.position_id === position.position_id) : undefined;
  if (byId) return byId;
  // An older snapshot's brackets carry no id: they are index-aligned with positions.
  if (brackets.some((b) => b.position_id)) return undefined;
  const i = positions.indexOf(position);
  return i >= 0 ? brackets[i] : undefined;
}

export interface NockBracketRow {
  name: string;
  lower: number;
  upper: number;
  displayLower: string;
  displayUpper: string;
  liquidity: number;
  inRange: boolean;
  distance: number | null;
  converted: boolean;
  fullRange: boolean;
}

/**
 * Rows for the Tick Bracket Hypervisor, a chart in USD per NOCK (#152). Each bracket is
 * paired with its position by `position_id` (index for older snapshots). A bracket whose
 * position has no NOCK leg is left out: its prices are token0 per token1 of another pair
 * (WETH per USDC), not a NOCK price. A WETH/NOCK bracket is ETH per NOCK and is converted
 * to USD with the live ETH/USD; without one it keeps native units rather than a made-up
 * price. Full-range positions have no bracket to draw.
 */
export function nockBracketRows(
  brackets: any[] | null | undefined,
  positions: any[] | null | undefined,
  ethUsd: number | null | undefined,
  spot: number,
): NockBracketRow[] {
  const list = positions ?? [];
  const rows: NockBracketRow[] = [];
  (brackets ?? []).forEach((b: any, i: number) => {
    const pos = bracketPosition(b, i, list);
    if (pos && !hasNockLeg(pos)) return;
    if (b?.full_range) return;
    const ethDenominated = !!pos && ['ETH', 'WETH'].includes(pos.token0?.symbol) && pos.token1?.symbol === 'NOCK';
    const converted = ethDenominated && ethUsd != null;
    const fx = converted ? (ethUsd as number) : 1;
    const lower = parseFloat(b.lower_price) * fx;
    const upper = parseFloat(b.upper_price) * fx;
    // Distance to the nearest bracket edge in the same (USD/NOCK) units; falls back to
    // the worker's bps when a row stays in native units.
    const distance = converted && spot > 0 && lower > 0
      ? Math.min(Math.max(spot - lower, 0), Math.max(upper - spot, 0)) / spot * 100
      : (b.distance_bps != null ? b.distance_bps / 100 : null);
    rows.push({
      name: positionLabel(pos),
      lower,
      upper,
      displayLower: b.lower_price,
      displayUpper: b.upper_price,
      liquidity: parseFloat(b.liquidity_usd),
      inRange: b.in_range,
      distance,
      converted,
      fullRange: !!b.full_range,
    });
  });
  return rows;
}

/**
 * How to name the tracked ranges with no NOCK leg, the worker's rule
 * (`non_nock_range_kind`): "ETH/USDC" only when every one of them is an ETH-or-WETH /
 * USDC pair, otherwise "non-NOCK".
 */
export function nonNockRangeKind(positions: any[] | null | undefined): 'ETH/USDC' | 'non-NOCK' {
  const allEthUsdc = (positions ?? [])
    .filter((p) => p && !hasNockLeg(p))
    .every((p) => isEthUsdcPair(p));
  return allEthUsdc ? 'ETH/USDC' : 'non-NOCK';
}

/**
 * True for a pair of ETH or WETH with USDC, in either order: the ranges drawn on the
 * ETH/USDC bracket chart (#160). The same symbol rule as `nonNockRangeKind`.
 */
export function isEthUsdcPair(pos: { token0?: { symbol?: string } | null; token1?: { symbol?: string } | null } | null | undefined): boolean {
  const isEth = (t: any) => t?.symbol === 'ETH' || t?.symbol === 'WETH';
  const isUsdc = (t: any) => t?.symbol === 'USDC';
  return (isEth(pos?.token0) && isUsdc(pos?.token1)) || (isUsdc(pos?.token0) && isEth(pos?.token1));
}

export interface EthBracketRow {
  positionId: string | null;
  name: string;
  /** USD (USDC) per ETH. */
  lower: number;
  upper: number;
  liquidity: number;
  inRange: boolean;
  /** Percent from the ETH spot to the nearer edge; 0 outside; null without a spot. */
  distance: number | null;
}

/**
 * Rows for the ETH/USDC bracket chart, in USD per ETH (#160). A bracket is token0 per
 * token1: a WETH/USDC one is WETH per USDC, so it is inverted and its edges swap; a
 * USDC/WETH one is already USDC per ETH. Pairing is by `position_id` (index for older
 * snapshots), as for the NOCK chart. Full-range positions have no bracket to draw, and a
 * bracket with an edge that is not a positive number is left out rather than drawn at a
 * made-up price. These ranges are measured, never steered (#151).
 */
export function ethBracketRows(
  brackets: any[] | null | undefined,
  positions: any[] | null | undefined,
  ethUsd: number | null | undefined,
): EthBracketRow[] {
  const list = positions ?? [];
  const rows: EthBracketRow[] = [];
  (brackets ?? []).forEach((b: any, i: number) => {
    const pos = bracketPosition(b, i, list);
    if (!pos || hasNockLeg(pos) || !isEthUsdcPair(pos)) return;
    if (b?.full_range || pos.full_range) return;
    const lo = parseFloat(b?.lower_price);
    const hi = parseFloat(b?.upper_price);
    if (!(lo > 0) || !(hi > 0) || !Number.isFinite(lo) || !Number.isFinite(hi)) return;
    const ethIsToken0 = pos.token0?.symbol === 'ETH' || pos.token0?.symbol === 'WETH';
    const a = ethIsToken0 ? 1 / lo : lo;
    const c = ethIsToken0 ? 1 / hi : hi;
    const lower = Math.min(a, c);
    const upper = Math.max(a, c);
    const spot = ethUsd != null && ethUsd > 0 ? ethUsd : null;
    const distance = spot !== null
      ? Math.min(Math.max(spot - lower, 0), Math.max(upper - spot, 0)) / spot * 100
      : null;
    const liquidity = parseFloat(b?.liquidity_usd);
    rows.push({
      positionId: pos.position_id ?? null,
      name: positionLabel(pos),
      lower,
      upper,
      liquidity: Number.isFinite(liquidity) ? liquidity : 0,
      inRange: !!b?.in_range,
      distance,
    });
  });
  return rows;
}

/**
 * Tracked ETH-or-WETH / USDC ranges that are full range: no bracket to draw on the
 * ETH/USDC chart, so the chart says so instead of leaving them out silently (#160).
 */
export function fullRangeEthUsdcCount(positions: any[] | null | undefined): number {
  return (positions ?? []).filter((p) => p && !hasNockLeg(p) && isEthUsdcPair(p) && p.full_range).length;
}

/**
 * "1 non-NOCK range not drawn: not priced in NOCK or ETH"; null when every range is
 * drawn on one of the two charts. Since #160 ETH/USDC ranges are drawn on their own
 * chart, so only another pair (USDC/AERO, say) is named here.
 */
export function notDrawnRangesText(positions: any[] | null | undefined): string | null {
  const n = (positions ?? []).filter((p) => p && !hasNockLeg(p) && !isEthUsdcPair(p)).length;
  if (n === 0) return null;
  return `${n} non-NOCK range${n === 1 ? '' : 's'} not drawn: not priced in NOCK or ETH`;
}

/** Tracked ranges with no NOCK leg: not drawn on the NOCK bracket chart (#152). */
export function nonNockRangeCount(positions: any[] | null | undefined): number {
  return (positions ?? []).filter((p) => p && !hasNockLeg(p)).length;
}
