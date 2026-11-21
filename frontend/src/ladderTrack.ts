/**
 * Tracked ladders (#167): positions configured `role: ladder` and their readings
 * (`data.ladders[]`).
 *
 * A ladder is a USDC/NOCK range the operator deployed. It is measured, never steered: it
 * counts in every total, the floor-crossing price and the protected reserve, but it is
 * not NOCK LP (no LP target, no rebuild). These helpers are the wording of the dashboard
 * card, the position cards and the phone cockpit; they say what each ladder holds and,
 * only from its verified deposit receipt, what it bought. They never say to re-centre,
 * withdraw, add to or close one. A snapshot with no ladder renders exactly as before.
 */
import type { LadderReading } from './types/snapshot';

/** True for a position configured `role: ladder`. */
export function isLadder(pos: { role?: string | null } | null | undefined): boolean {
  return pos?.role === 'ladder';
}

/** NOCK LP: a position with a NOCK leg that is not a ladder (the worker's `is_nock_lp`). */
export function isNockLp(pos: { role?: string | null; token0?: { symbol?: string } | null; token1?: { symbol?: string } | null } | null | undefined): boolean {
  const nockLeg = pos?.token0?.symbol === 'NOCK' || pos?.token1?.symbol === 'NOCK';
  return nockLeg && !isLadder(pos);
}

/** The snapshot's ladder readings; empty for a snapshot that lists none. */
export function laddersOf(data: { ladders?: LadderReading[] | null } | null | undefined): LadderReading[] {
  return Array.isArray(data?.ladders) ? (data!.ladders as LadderReading[]) : [];
}

/** The reading of one position, by position_id. */
export function ladderReadingFor(position: { position_id?: string } | null | undefined, ladders: LadderReading[]): LadderReading | undefined {
  return position?.position_id ? ladders.find((l) => l.position_id === position.position_id) : undefined;
}

export type LadderTone = 'good' | 'neutral' | 'stale';

/**
 * The badge of a ladder, by what it holds. A ladder above its range is normal: the badge
 * is neutral, never the warn-toned "OUT OF RANGE" of a NOCK LP position.
 */
export function ladderStatus(r: { state?: string } | null | undefined): { label: string; tone: LadderTone } {
  switch (r?.state) {
    case 'holds_only_usdc':
      return { label: '● LADDER ABOVE RANGE (ALL USDC)', tone: 'neutral' };
    case 'in_range':
      return { label: '● LADDER IN RANGE (CONVERTS BOTH WAYS)', tone: 'good' };
    case 'holds_only_nock':
      return { label: '● LADDER BELOW RANGE (ALL NOCK)', tone: 'neutral' };
    case 'empty':
      return { label: '● LADDER EMPTY', tone: 'neutral' };
    case 'pool_stale':
      return { label: '● STALE POOL', tone: 'stale' };
    default:
      return { label: '● LADDER (STATE NOT REPORTED)', tone: 'neutral' };
  }
}

/** Short state words for the phone and the summary line. */
export function ladderStateWords(state: string | undefined): string {
  switch (state) {
    case 'holds_only_usdc':
      return 'above range, all USDC';
    case 'in_range':
      return 'in range';
    case 'holds_only_nock':
      return 'below range, all NOCK';
    case 'empty':
      return 'empty';
    case 'pool_stale':
      return 'stale pool';
    default:
      return 'state not reported';
  }
}

const pct = (v: string | null | undefined) => (v == null ? null : `${v.startsWith('-') ? '−' + v.slice(1) : v}%`);

/** "$0.00678699 – $0.01032932 (−67.16% / −50.01% of spot)". */
export function ladderRangeLine(r: LadderReading): string {
  // A bound at the pool's usable extremes is stated in words, never as a price.
  const low = r.low_price_usd == null ? 'no bottom' : `$${r.low_price_usd}`;
  const high = r.high_price_usd == null ? 'no top' : `$${r.high_price_usd}`;
  const lo = r.low_price_usd == null ? 'no bottom' : pct(r.low_pct_of_spot);
  const hi = r.high_price_usd == null ? 'no top' : pct(r.high_pct_of_spot);
  const vs = r.spot_price_usd == null || !lo || !hi ? ' (no spot to compare)' : ` (${lo} / ${hi} of spot)`;
  const open = r.open_end ? `; ${r.open_end}` : '';
  return `${low} – ${high}${vs}${open}`;
}

/** "rung cap", "rung near", "rung deep" or "rung unset", and "found on-chain" for a
 *  ladder that comes from a database row (v0.9.0) rather than a config.yaml entry. */
export function ladderRungText(r: LadderReading): string {
  const rung = r.rung ? `rung ${r.rung}` : 'rung unset';
  return (r as { source?: string | null }).source ? `${rung} · found on-chain` : rung;
}

export function ladderHoldsLine(r: LadderReading): string {
  return `Holds $${r.usdc_now} USDC + ${r.nock_now} NOCK${r.value_usd ? ` ($${r.value_usd})` : ''}`;
}

/** Holdings against the range's own all-USDC top: no baseline needed, and not trades. */
export function ladderCurveLine(r: LadderReading): string {
  const v = r.vs_top;
  const avg = v.curve_avg_price_usd ? `, curve average $${v.curve_avg_price_usd}` : '';
  const top = v.usdc_at_top == null ? 'not stated' : `$${v.usdc_at_top}`;
  const conv = v.usdc_converted_vs_top == null ? 'not stated' : `$${v.usdc_converted_vs_top}`;
  const bottom = v.nock_at_bottom == null ? 'not stated (no bottom)' : `${v.nock_at_bottom} NOCK`;
  return `Against its all-USDC top (${top} USDC): ${v.nock_held_vs_top} NOCK held, ${conv} converted (${v.converted_pct ?? '—'}%)${avg}; all NOCK at the bottom: ${bottom}`;
}

/** What it bought since its deposit, from the verified receipt only; else the reason. */
export function ladderSinceDeposit(r: LadderReading): { text: string; verified: boolean } {
  const sd = r.since_deposit;
  if (sd?.status === 'verified') {
    const tx = sd.deposit_tx ? `${sd.deposit_tx.slice(0, 10)}…` : '—';
    const avg = sd.avg_buy_price_usd
      ? `, average $${sd.avg_buy_price_usd}`
      : '; no average price (it has not net bought NOCK with USDC)';
    return {
      text: `Since deposit (tx ${tx}, block ${sd.deposit_block ?? '—'}): net ${sd.net_nock_bought} NOCK bought for net $${sd.net_usdc_spent} USDC spent${avg}`,
      verified: true,
    };
  }
  const status = sd?.status ?? 'not reported';
  return { text: `Since deposit: ${status.replace(/_/g, ' ')}: ${sd?.reason ?? 'no reason reported'}`, verified: false };
}

/** One muted line for the phone: "v4 USDC/NOCK 3.00% #3240369: above range, all USDC · $1226.09 USDC + 0.0000 NOCK". */
export function ladderLiteLine(r: LadderReading): string {
  return `${r.label}: ${ladderStateWords(r.state)} · $${r.usdc_now} USDC + ${r.nock_now} NOCK`;
}

/** " and 2 ladders" for the Goal 1 line; "" with none. */
export function ladderCountClause(positions: Array<{ role?: string | null }> | null | undefined): string {
  const m = (positions ?? []).filter(isLadder).length;
  return m === 0 ? '' : ` and ${m} ladder${m === 1 ? '' : 's'}`;
}

/**
 * " +2 ladders, measured only (1 holding only USDC, 1 in range)." for the positions
 * summary, the worker's own clause and order; "" with none.
 */
export function ladderSummaryClause(positions: Array<{ position_id?: string; role?: string | null }> | null | undefined, ladders: LadderReading[]): string {
  const ladderPositions = (positions ?? []).filter(isLadder);
  if (ladderPositions.length === 0) return '';
  const states = ladderPositions.map((p) => ladderReadingFor(p, ladders)?.state ?? 'not_reported');
  const order: Array<[string, string]> = [
    ['holds_only_usdc', 'holding only USDC'],
    ['in_range', 'in range'],
    ['holds_only_nock', 'holding only NOCK'],
    ['empty', 'empty'],
    ['pool_stale', 'on a stale pool'],
    ['not_reported', 'state not reported'],
  ];
  const detail = order
    .map(([s, words]) => [states.filter((x) => x === s).length, words] as const)
    .filter(([k]) => k > 0)
    .map(([k, words]) => `${k} ${words}`);
  const n = ladderPositions.length;
  return ` +${n} ladder${n === 1 ? '' : 's'}, measured only (${detail.join(', ')}).`;
}

export const LADDERS_TITLE = 'Ladders (measured, not NOCK LP)';
export const LADDERS_COUNTED = 'Counted in the totals, the protected reserve and the floor-crossing price; not in the LP target or the rebuild.';
