/**
 * The ladder suggestion (issue #168): `derived_metrics.ladder_suggestion`, a top-of-page
 * event the worker measures every cycle. These helpers are the card's wording and the
 * geometry of its picture; the worker computes every figure.
 *
 * The card needs no input: it states whether a NOCK ladder fits the operator's own
 * rules, what it would do in plain words, and the numbered deposits with their links.
 * It shows only while the snapshot is fresh. NockNock signs and sends nothing, and the
 * links prefill no amount.
 */
import type { SyncStatus } from './hooks/useFreshness';

export interface LadderToken {
  symbol: string;
  address: string;
  decimals: number;
}

export interface LadderRung {
  index: number;
  of: number;
  rung: 'near' | 'deep' | null;
  pool: {
    position_id: string;
    label: string;
    protocol: string;
    pool_id: string;
    token0: LadderToken;
    token1: LadderToken;
    fee_tier_bps: number;
    fee_raw: number | null;
    tick_spacing: number | null;
    hooks: string | null;
  };
  tick_lower: number;
  tick_upper: number;
  bounds_usd: { low: string; high: string };
  bounds_nock_per_usdc: { low: string | null; high: string | null };
  usdc: { amount: string; raw: string; decimals: number };
  nock: { amount: string; raw: string; decimals: number };
  deep_link: { url: string | null; prefilled: string[]; not_prefilled: string[]; reason: string | null; format_note?: string | null };
  would_sign: string;
}

export interface LadderOutcome {
  kind: 'fall_inside' | 'fall_to_bottom' | 'rise' | string;
  text: string;
}

export interface LadderPictureData {
  spot_usd: string | null;
  bottom_usd: string;
  bottom_requested_usd: string;
  reserve_usdc: string;
  reserve_label: string;
  bars: { index: number; low_usd: string; high_usd: string; usdc: string; label: string }[];
}

export interface LadderCandidateRow {
  tick: number | null;
  price_usd: string | null;
  pct_of_spot: string | null;
  usdc_ceiling: string | null;
  nock_at_bottom: string | null;
  nock_per_100_usdc: string;
  avg_buy_price_usd: string | null;
}

export interface LadderSuggestion {
  id: string;
  tier: string;
  status: string;
  shown: boolean;
  headline: string | null;
  reason: string | null;
  sentence: string | null;
  missing: string[];
  rungs: LadderRung[];
  outcomes: LadderOutcome[];
  picture: LadderPictureData | null;
  candidates: { rows: LadderCandidateRow[]; to_pct?: string; step_pct?: string; reason?: string | null } | null;
  after_deposit: { kind: string; is?: string; text: string; config_snippets?: string[] } | null;
  statement: string;
}

export const LADDER_TITLE = 'Ladder suggestion (a measurement, not advice)';

/** The snapshot's suggestion, or null from a worker that predates it. */
export function ladderSuggestionOf(data: any): LadderSuggestion | null {
  const l = data?.derived_metrics?.ladder_suggestion;
  return l && typeof l === 'object' && typeof l.status === 'string' ? (l as LadderSuggestion) : null;
}

export type LadderCardMode = 'ready' | 'stale' | 'line';

/** What the card shows: the whole ladder, a stale notice, or one line saying why not. */
export function ladderCardView(s: LadderSuggestion, syncStatus: SyncStatus | undefined): { mode: LadderCardMode; headline: string; line: string | null } {
  if (s.status === 'ready' && s.shown) {
    if (syncStatus === 'stale') {
      return { mode: 'stale', headline: 'Ladder: not shown while the snapshot is stale.', line: 'It shows again once the worker reads the chain.' };
    }
    return { mode: 'ready', headline: s.headline ?? 'Ladder ready.', line: null };
  }
  return { mode: 'line', headline: s.headline ?? 'Ladder: not measured this cycle.', line: s.reason };
}

/** A deposit link is used only on https to the Uniswap web app or the Aero app. */
export function ladderDepositUrl(url: string | null | undefined): string | null {
  if (!url) return null;
  try {
    const u = new URL(url);
    if (u.protocol !== 'https:') return null;
    return u.hostname === 'app.uniswap.org' || u.hostname === 'app.aero.xyz' ? url : null;
  } catch {
    return null;
  }
}

/** The link's label, from what the worker says it prefills. */
export function ladderLinkLabel(r: LadderRung): string {
  const url = ladderDepositUrl(r.deep_link?.url);
  if (!url) return '';
  const host = new URL(url).hostname;
  if (host === 'app.aero.xyz') return 'OPEN POOL DEPOSIT ON AERO.XYZ ↗ (nothing prefilled)';
  return (r.deep_link.prefilled ?? []).includes('range ticks')
    ? 'OPEN NEW POSITION IN UNISWAP ↗ (range prefilled, amount not)'
    : 'OPEN UNISWAP ↗ (amount not prefilled)';
}

/** "Deposit 1 of 2 (near rung): $115.12 USDC + 0 NOCK" */
export function rungTitle(r: LadderRung): string {
  const name = r.rung ? ` (${r.rung} rung)` : '';
  const nock = Number(r.nock?.amount);
  return `Deposit ${r.index} of ${r.of}${name}: $${r.usdc.amount} USDC + ${Number.isFinite(nock) && nock > 0 ? r.nock.amount : '0'} NOCK`;
}

/** The range in both forms: USD per NOCK and NOCK per USDC (the Uniswap app's form). */
export function rungRangeLines(r: LadderRung): string[] {
  const lines = [`$${r.bounds_usd.low} to $${r.bounds_usd.high} per NOCK`];
  if (r.bounds_nock_per_usdc?.low && r.bounds_nock_per_usdc?.high) {
    lines.push(`${r.bounds_nock_per_usdc.low} to ${r.bounds_nock_per_usdc.high} NOCK per USDC (the app's form)`);
  }
  lines.push(`ticks ${r.tick_lower} to ${r.tick_upper} in ${r.pool.label}`);
  return lines;
}

/** The setting the card is missing, in one plain sentence. */
export function missingSettingSentence(s: LadderSuggestion): string | null {
  if (!s.missing?.length) return null;
  const words = s.missing.map((k) =>
    k === 'operator_profile.ladder_bottom_usd'
      ? 'your bottom price, operator_profile.ladder_bottom_usd'
      : k === 'operator_profile.cash_reserve_usdc'
        ? 'your protected reserve, operator_profile.cash_reserve_usdc'
        : k,
  );
  return `NockNock needs ${words.join(' and ')}, in config.yaml; it does not choose these numbers.`;
}

/** One short line per candidate bottom (no bottom set): a measurement, none picked. */
export function candidateLines(s: LadderSuggestion): string[] {
  return (s.candidates?.rows ?? []).slice(0, 8).map((r) => {
    const amount = r.usdc_ceiling ? `up to $${r.usdc_ceiling} → about ${r.nock_at_bottom} NOCK` : `${r.nock_per_100_usdc} NOCK per $100`;
    return `$${r.price_usd} (${r.pct_of_spot}%): ${amount}, average $${r.avg_buy_price_usd}`;
  });
}

export interface PictureGeometry {
  width: number;
  height: number;
  x: (usd: number) => number;
  min: number;
  max: number;
  spotX: number | null;
  bottomX: number;
  bars: { x: number; w: number; y: number; label: string; low: number; high: number }[];
}

/** A linear USD-per-NOCK axis from below the bottom to above spot. Pure. */
export function pictureGeometry(p: LadderPictureData, width = 520, height = 150): PictureGeometry | null {
  const bottom = Number(p.bottom_usd);
  const spot = p.spot_usd != null ? Number(p.spot_usd) : NaN;
  const highs = p.bars.map((b) => Number(b.high_usd)).filter(Number.isFinite);
  const top = Math.max(...highs, Number.isFinite(spot) ? spot : -Infinity);
  if (!Number.isFinite(bottom) || !Number.isFinite(top) || !(top > bottom)) return null;
  const span = top - bottom;
  const min = bottom - span * 0.25;
  const max = top + span * 0.1;
  const left = 8;
  const right = width - 8;
  const x = (usd: number) => left + ((usd - min) / (max - min)) * (right - left);
  const bars = p.bars.map((b, i) => {
    const low = Number(b.low_usd);
    const high = Number(b.high_usd);
    return { x: x(low), w: Math.max(2, x(high) - x(low)), y: 34 + i * 30, label: b.label, low, high };
  });
  return { width, height, x, min, max, spotX: Number.isFinite(spot) ? x(spot) : null, bottomX: x(bottom), bars };
}
