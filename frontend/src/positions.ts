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
  const proto = p.protocol === 'uniswap-v4' ? 'v4' : p.protocol === 'uniswap-v3' ? 'v3' : p.protocol;
  const fee = (p.fee_tier_bps / 100).toFixed(2).replace(/\.?0+$/, '');
  return `${p.token0.symbol}/${p.token1.symbol} ${proto} · ${fee}%`;
}

/**
 * Readable range string. The API's lower_price/upper_price are token0-per-token1
 * (documented in snapshot.yaml); when that value is sub-cent tiny (ETH-per-NOCK ≈
 * 0.0000081) the display inverts to token1-per-token0 so numbers stay readable —
 * matching the operator's Uniswap-app convention for both screenshot shapes.
 */
export function positionRangeText(p: any): string {
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
export function rangeStatus(p: { in_range: boolean; pool_active?: boolean | null }): {
  label: string;
  tone: 'good' | 'warn' | 'stale';
} {
  if (p.pool_active === false) return { label: '● STALE POOL', tone: 'stale' };
  return p.in_range ? { label: '● IN RANGE', tone: 'good' } : { label: '● OUT OF RANGE', tone: 'warn' };
}
