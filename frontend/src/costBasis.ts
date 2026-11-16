/**
 * Pure display helpers for the "Average cost vs spot" panel (issue #98).
 *
 * Rules this file keeps:
 * - Two layers, never merged. "Plan buys" is what the accumulation ledger's verified
 *   fills cost. "Whole stack" exists only when the operator entered an opening basis in
 *   config; otherwise it reads "not configured" and shows no number.
 * - Every figure is the worker's own (snapshot `cost_basis`, GET /v1/cost-basis/series).
 *   A missing value is "—" or a stated reason, never 0.
 * - Measurements only. No sentence here suggests buying, selling or averaging down: the
 *   panel says where the average is and what the NOCK is worth at spot.
 * - Stored spot points are drawn as stored. Two points more than 2.5 buckets apart have a
 *   break between them; nothing is interpolated across it. The average-cost lines are
 *   steps because an average cost really is constant between buys.
 */
import {
  GAP_CADENCE_MULTIPLIER,
  formatGapDuration,
  finiteOrNull,
  timeMs,
  toChartSeries,
  isGapRow,
  type GapRow,
} from './timeline.ts';

// ---------------------------------------------------------------------------
// Types: snapshot `derived_metrics.cost_basis` (specs/core/snapshot.yaml)
// ---------------------------------------------------------------------------

export interface PlanBasis {
  plan_id: string;
  fills: number;
  spent_usdc: string;
  acquired_nock: string;
  avg_cost_usd: string;
  first_fill_at: string;
  last_fill_at: string;
}

export interface PlanBuysBasis {
  status: 'ok' | 'no_fills' | 'unavailable';
  reason?: string | null;
  fills: number;
  spent_usdc?: string | null;
  acquired_nock?: string | null;
  avg_cost_usd?: string | null;
  first_fill_at?: string | null;
  last_fill_at?: string | null;
  value_at_spot_usd?: string | null;
  unrealized_usd?: string | null;
  unrealized_pct?: string | null;
  plans?: PlanBasis[];
  includes?: string[];
  excludes?: string[];
}

export interface WholeStackBasis {
  status: 'not_configured' | 'ok' | 'invalid' | 'unavailable';
  reason?: string | null;
  source?: string | null;
  as_of?: string | null;
  opening_nock?: string | null;
  opening_cost_usd?: string | null;
  opening_avg_cost_usd?: string | null;
  fills_since?: number | null;
  bought_since_usdc?: string | null;
  bought_since_nock?: string | null;
  basis_nock?: string | null;
  basis_cost_usd?: string | null;
  avg_cost_usd?: string | null;
  value_at_spot_usd?: string | null;
  unrealized_usd?: string | null;
  unrealized_pct?: string | null;
  held_nock?: string | null;
  includes?: string[];
  excludes?: string[];
}

export interface CostBasis {
  spot_price_usd?: string | null;
  plan_buys: PlanBuysBasis;
  whole_stack: WholeStackBasis;
}

// ---------------------------------------------------------------------------
// Types: GET /v1/cost-basis/series (specs/core/api.yaml)
// ---------------------------------------------------------------------------

export interface SeriesSpotPoint {
  at: string;
  price_usd: string;
  source: string;
  is_backfilled: boolean;
}

export interface SeriesFill {
  plan_id: string;
  tx_hash: string;
  block_number: number;
  confirmed_at: string;
  usdc_spent: string;
  nock_received: string;
  effective_price_usd: string;
  plan_avg_cost_usd?: string | null;
  avg_cost_usd?: string | null;
  whole_stack_avg_cost_usd?: string | null;
  spot_at_fill?: SeriesSpotPoint | null;
}

export interface CostBasisSeries {
  generated_at: string;
  from: string;
  to: string;
  bucket_seconds: number;
  spot_match_window_minutes: number;
  spot: SeriesSpotPoint[] | null;
  spot_error?: string | null;
  fills: SeriesFill[];
  whole_stack: {
    status: 'not_configured' | 'ok' | 'invalid';
    reason?: string | null;
    opening?: { as_of: string; nock: string; cost_usd: string; avg_cost_usd: string; source: string } | null;
  };
}

// ---------------------------------------------------------------------------
// Formatting
// ---------------------------------------------------------------------------

export const COST_PANEL_TITLE = 'Average cost vs spot';
export const NOT_REPORTED = 'not reported by this snapshot';

/** A NOCK price: five decimals, like the rest of the dashboard. "—" when unknown. */
export function priceText(x: unknown): string {
  const v = finiteOrNull(x);
  return v === null ? '—' : `$${v.toFixed(5)}`;
}

function usdText(x: unknown): string {
  const v = finiteOrNull(x);
  return v === null ? '—' : `$${v.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
}

function nockText(x: unknown): string {
  const v = finiteOrNull(x);
  return v === null ? '—' : v.toLocaleString('en-US', { maximumFractionDigits: 2 });
}

export type DifferenceTone = 'above' | 'below' | 'even' | 'none';

export interface DifferenceView {
  /** "−$1.21 (−2.02%)", "+$41.97 (+69.95%)", "$0.00" or "—". */
  text: string;
  tone: DifferenceTone;
}

/**
 * The signed difference between value at spot and cost, as the worker reports it.
 * "below" means the NOCK is worth less at spot than it cost. A statement, not a signal.
 */
export function differenceView(unrealizedUsd: unknown, unrealizedPct: unknown): DifferenceView {
  const usd = finiteOrNull(unrealizedUsd);
  if (usd === null) return { text: '—', tone: 'none' };
  const pct = finiteOrNull(unrealizedPct);
  if (Math.abs(usd) < 0.005) return { text: '$0.00', tone: 'even' };
  const sign = usd < 0 ? '−' : '+';
  const money = `${sign}$${Math.abs(usd).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
  const share = pct === null ? '' : ` (${pct < 0 ? '−' : '+'}${Math.abs(pct).toFixed(2)}%)`;
  return { text: `${money}${share}`, tone: usd < 0 ? 'below' : 'above' };
}

// ---------------------------------------------------------------------------
// Summary tiles
// ---------------------------------------------------------------------------

export interface CostTile {
  label: string;
  value: string;
  sub: string;
  tone: DifferenceTone;
}

export interface CostBasisView {
  title: string;
  tiles: CostTile[];
  /** A configuration error to show prominently; null when there is none. */
  configError: string | null;
  /** What the figures count and leave out, in the worker's words. */
  notes: string[];
}

function planTile(p: PlanBuysBasis | undefined): CostTile {
  const label = 'PLAN BUYS · AVERAGE COST';
  if (!p) return { label, value: '—', sub: NOT_REPORTED, tone: 'none' };
  if (p.status === 'unavailable') {
    return { label, value: 'unavailable', sub: p.reason ?? 'no reason reported', tone: 'none' };
  }
  if (p.status === 'no_fills') {
    return { label, value: '—', sub: 'No verified buys recorded yet', tone: 'none' };
  }
  const n = p.fills ?? 0;
  const plans = p.plans?.length ?? 0;
  return {
    label,
    value: priceText(p.avg_cost_usd),
    sub: `${n} verified buy${n === 1 ? '' : 's'}${plans > 1 ? ` across ${plans} plans` : ''} · ${usdText(p.spent_usdc)} for ${nockText(p.acquired_nock)} NOCK`,
    tone: 'none',
  };
}

function planDifferenceTile(p: PlanBuysBasis | undefined, spot: unknown): CostTile {
  const label = 'PLAN BUYS · VALUE AT SPOT LESS COST';
  if (!p || p.status !== 'ok') return { label, value: '—', sub: 'Needs at least one verified buy', tone: 'none' };
  const d = differenceView(p.unrealized_usd, p.unrealized_pct);
  if (d.tone === 'none') {
    return { label, value: '—', sub: 'No spot price in this snapshot, so no comparison', tone: 'none' };
  }
  return {
    label,
    value: d.text,
    sub: `${nockText(p.acquired_nock)} NOCK is worth ${usdText(p.value_at_spot_usd)} at spot ${priceText(spot)}; it cost ${usdText(p.spent_usdc)}. Unrealized: nothing was sold.`,
    tone: d.tone,
  };
}

function wholeStackTile(w: WholeStackBasis | undefined): CostTile {
  const label = 'WHOLE STACK · AVERAGE COST';
  if (!w) return { label, value: '—', sub: NOT_REPORTED, tone: 'none' };
  switch (w.status) {
    case 'not_configured':
      return {
        label,
        value: 'not configured',
        sub: 'NockNock cannot derive this from the chain. To see it, enter your own opening figures under cost_basis.opening in config.yaml.',
        tone: 'none',
      };
    case 'invalid':
      return { label, value: 'configuration error', sub: w.reason ?? 'cost_basis.opening cannot be used', tone: 'none' };
    case 'unavailable':
      return { label, value: 'unavailable', sub: w.reason ?? 'no reason reported', tone: 'none' };
    default: {
      const d = differenceView(w.unrealized_usd, w.unrealized_pct);
      const since = w.fills_since ?? 0;
      const diff = d.tone === 'none' ? '' : ` At spot that NOCK is worth ${usdText(w.value_at_spot_usd)}: ${d.text} against its cost.`;
      const held = finiteOrNull(w.held_nock);
      const heldText = held === null ? '' : ` NockNock reads ${nockText(held)} NOCK held now (wallet plus tracked LP).`;
      return {
        label,
        value: priceText(w.avg_cost_usd),
        sub: `On ${nockText(w.basis_nock)} NOCK costing ${usdText(w.basis_cost_usd)}: your opening basis (${nockText(w.opening_nock)} NOCK for ${usdText(w.opening_cost_usd)}, source: ${w.source ?? 'not stated'}, as of ${w.as_of ?? 'unknown'}) plus ${since} verified buy${since === 1 ? '' : 's'} since.${diff}${heldText}`,
        // The headline is an average, not a difference: it takes no tone.
        tone: 'none',
      };
    }
  }
}

export function costBasisView(cb: CostBasis | null | undefined): CostBasisView {
  if (!cb) {
    return {
      title: COST_PANEL_TITLE,
      tiles: [],
      configError: null,
      notes: [`Cost basis ${NOT_REPORTED} (the worker predates it, or has not completed a cycle).`],
    };
  }
  const p = cb.plan_buys;
  const w = cb.whole_stack;
  const notes: string[] = [];
  for (const s of p?.includes ?? []) notes.push(`Plan buys count: ${s}`);
  if (p?.excludes?.length) notes.push(`Plan buys leave out: ${p.excludes.join(' ')}`);
  if (w?.status === 'ok') {
    for (const s of w.includes ?? []) notes.push(`Whole stack counts: ${s}`);
    if (w.excludes?.length) notes.push(`Whole stack leaves out: ${w.excludes.join(' ')}`);
  }
  return {
    title: COST_PANEL_TITLE,
    tiles: [
      planTile(p),
      { label: 'SPOT', value: priceText(cb.spot_price_usd), sub: 'NOCK/USD from the price-source pool, this snapshot', tone: 'none' },
      planDifferenceTile(p, cb.spot_price_usd),
      wholeStackTile(w),
    ],
    configError: w?.status === 'invalid' ? (w.reason ?? 'cost_basis.opening cannot be used') : null,
    notes,
  };
}

// ---------------------------------------------------------------------------
// Chart
// ---------------------------------------------------------------------------

/** A break is two stored points more than this many buckets apart (the dashboard's gap rule). */
export const SPOT_GAP_BUCKETS = GAP_CADENCE_MULTIPLIER;

export interface SpotRow {
  t: number;
  spot: number;
  backfilled: boolean;
  isolated?: boolean;
}

export interface StepRow {
  t: number;
  v: number;
}

export interface FillMarker {
  t: number;
  /** Effective price of this buy (USDC spent / NOCK received). */
  price: number;
  txHash: string;
}

export interface CostChart {
  /** x domain in epoch ms: the range the worker served. */
  domain: [number, number];
  /** Evenly spaced x ticks across the domain, ends included. */
  ticks: number[];
  /** Stored spot points with break rows between runs. Empty when there is none. */
  spot: Array<SpotRow | GapRow>;
  gaps: GapRow[];
  /** Running average of the verified buys, as a step line carried to the end of the range. */
  planAvg: StepRow[];
  /** Blended whole-stack average from the opening basis on; empty unless configured. */
  wholeAvg: StepRow[];
  fills: FillMarker[];
  /** Padded [min, max] over everything drawn, or null when nothing can be drawn. */
  yDomain: [number, number] | null;
  spotPoints: number;
  backfilledPoints: number;
  /** Why there is no spot line, when there is none. */
  spotNote: string | null;
}

/**
 * A step line through (time, value) change points, clipped to the range: it starts at the
 * later of the first change and `from` (carrying the value in force there) and ends at
 * `to`. Empty when no change point is at or before `to`.
 */
export function stepLine(changes: StepRow[], from: number, to: number): StepRow[] {
  const sorted = changes.filter((c) => Number.isFinite(c.t) && Number.isFinite(c.v)).sort((a, b) => a.t - b.t);
  const out: StepRow[] = [];
  let carried: number | null = null;
  for (const c of sorted) {
    if (c.t > to) break;
    if (c.t < from) {
      carried = c.v;
      continue;
    }
    if (out.length === 0 && carried !== null && c.t > from) out.push({ t: from, v: carried });
    out.push({ t: c.t, v: c.v });
  }
  if (out.length === 0 && carried !== null) out.push({ t: from, v: carried });
  if (out.length > 0 && out[out.length - 1].t < to) out.push({ t: to, v: out[out.length - 1].v });
  return out;
}

/** `count` evenly spaced ticks from `from` to `to`, both ends included. */
export function evenTicks(from: number, to: number, count = 5): number[] {
  const n = Math.max(2, Math.floor(count));
  return Array.from({ length: n }, (_, i) => Math.round(from + ((to - from) * i) / (n - 1)));
}

export function buildCostChart(series: CostBasisSeries | null | undefined): CostChart | null {
  if (!series) return null;
  const from = timeMs(series.from);
  const to = timeMs(series.to);
  if (from === null || to === null || to <= from) return null;

  const bucket = finiteOrNull(series.bucket_seconds);
  const spotRows: SpotRow[] = [];
  for (const p of series.spot ?? []) {
    const t = timeMs(p.at);
    const v = finiteOrNull(p.price_usd);
    if (t === null || v === null || v <= 0) continue; // an unreadable point is left out, never drawn as 0
    spotRows.push({ t, spot: v, backfilled: p.is_backfilled === true });
  }
  // toChartSeries breaks at 2.5x the cadence it is given; here the cadence is the bucket.
  const spotSeries = toChartSeries(spotRows, bucket);
  const gaps = spotSeries.gaps.map((g) => ({ ...g, time: `no stored price for ${formatGapDuration(g.gapMs)}` }));
  const spot = spotSeries.rows.map((r) => (isGapRow(r) ? gaps[spotSeries.gaps.indexOf(r)] : r));

  const fills: FillMarker[] = [];
  const planChanges: StepRow[] = [];
  const wholeChanges: StepRow[] = [];
  const opening = series.whole_stack?.status === 'ok' ? series.whole_stack.opening : null;
  if (opening) {
    const t = timeMs(opening.as_of);
    const v = finiteOrNull(opening.avg_cost_usd);
    if (t !== null && v !== null) wholeChanges.push({ t, v });
  }
  for (const f of series.fills ?? []) {
    const t = timeMs(f.confirmed_at);
    if (t === null) continue;
    const price = finiteOrNull(f.effective_price_usd);
    if (price !== null && t >= from && t <= to) fills.push({ t, price, txHash: f.tx_hash });
    const avg = finiteOrNull(f.avg_cost_usd);
    if (avg !== null) planChanges.push({ t, v: avg });
    const whole = finiteOrNull(f.whole_stack_avg_cost_usd);
    if (whole !== null && opening) wholeChanges.push({ t, v: whole });
  }
  const planAvg = stepLine(planChanges, from, to);
  const wholeAvg = stepLine(wholeChanges, from, to);

  const values = [
    ...spotRows.map((r) => r.spot),
    ...planAvg.map((r) => r.v),
    ...wholeAvg.map((r) => r.v),
    ...fills.map((f) => f.price),
  ];
  let yDomain: [number, number] | null = null;
  if (values.length > 0) {
    const lo = Math.min(...values);
    const hi = Math.max(...values);
    const pad = hi > lo ? (hi - lo) * 0.08 : hi * 0.05;
    yDomain = [Math.max(0, lo - pad), hi + pad];
  }

  const spotNote =
    series.spot === null
      ? `Spot history unavailable: ${series.spot_error ?? 'the stored price history could not be read'}.`
      : spotRows.length === 0
        ? 'No stored spot price in this range.'
        : null;

  return {
    domain: [from, to],
    ticks: evenTicks(from, to),
    spot,
    gaps,
    planAvg,
    wholeAvg,
    fills,
    yDomain,
    spotPoints: spotRows.length,
    backfilledPoints: spotRows.filter((r) => r.backfilled).length,
    spotNote,
  };
}

// ---------------------------------------------------------------------------
// Fill table: what each buy did to the average
// ---------------------------------------------------------------------------

export interface FillRow {
  txHash: string;
  planId: string;
  confirmedAt: string;
  spent: string;
  nock: string;
  price: string;
  /** Average of the verified buys before and after this one; "—" before the first. */
  avgBefore: string;
  avgAfter: string;
  /** Stored spot nearest the buy, or the reason there is none. */
  spotAtFill: string;
  /** Blended whole-stack average after this buy; "—" when not configured or not counted. */
  wholeAfter: string;
}

export function fillRows(series: CostBasisSeries | null | undefined): FillRow[] {
  if (!series) return [];
  const window = finiteOrNull(series.spot_match_window_minutes);
  let before: string | null = null;
  return (series.fills ?? []).map((f) => {
    const s = f.spot_at_fill;
    const row: FillRow = {
      txHash: f.tx_hash,
      planId: f.plan_id,
      confirmedAt: f.confirmed_at,
      spent: usdText(f.usdc_spent),
      nock: nockText(f.nock_received),
      price: priceText(f.effective_price_usd),
      avgBefore: before === null ? '—' : priceText(before),
      avgAfter: priceText(f.avg_cost_usd),
      spotAtFill: s
        ? `${priceText(s.price_usd)}${s.is_backfilled ? ' (backfilled)' : ''}`
        : series.spot === null
          ? 'unavailable'
          : `none stored within ${window === null ? 'the match window' : `${window} min`}`,
      wholeAfter: priceText(f.whole_stack_avg_cost_usd),
    };
    before = f.avg_cost_usd ?? before;
    return row;
  });
}

/** Legend line under the chart: how much of the spot line is third-party backfill. */
export function spotProvenanceText(chart: CostChart | null): string {
  if (!chart) return '';
  if (chart.spotNote) return chart.spotNote;
  const live = chart.spotPoints - chart.backfilledPoints;
  const parts = [`${chart.spotPoints} stored spot point${chart.spotPoints === 1 ? '' : 's'}`];
  if (chart.backfilledPoints > 0) {
    parts.push(`${live} read by the worker, ${chart.backfilledPoints} backfilled from third-party candles`);
  }
  if (chart.gaps.length > 0) {
    parts.push(`${chart.gaps.length} gap${chart.gaps.length === 1 ? '' : 's'} shown, not filled`);
  }
  return `${parts.join(' · ')}.`;
}
