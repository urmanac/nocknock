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
 * - No claim about what was not read. The difference at spot values the NOCK the buys
 *   acquired; NockNock does not track sells or LP activity, so the panel never says that
 *   NOCK is still held.
 * - Stored spot points are drawn as stored. Two points more than 2.5 buckets apart have a
 *   break between them; nothing is interpolated across it. The average-cost lines are
 *   steps because an average cost really is constant between buys.
 */
import {
  GAP_CADENCE_MULTIPLIER,
  formatGapDuration,
  finiteOrNull,
  gapThresholdMs,
  timeMs,
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
  /** Null when the ledger could not be read: unknown, not 0. */
  fills?: number | null;
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
  /** NOCK read this cycle, to the nearest whole NOCK. */
  held_nock?: string | null;
  /** Verified net buys of tracked ladders in the basis (2.9.0, #167); absent with none. */
  ladder_buys_since?: { ladders_counted: number; nock: string; usdc: string; not_counted?: string[] } | null;
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
  if (!p) return { label, value: '—', sub: NOT_REPORTED, tone: 'none' };
  // An unread ledger is not "no buys": say it could not be read, with the reason.
  if (p.status === 'unavailable') {
    return { label, value: 'unavailable', sub: p.reason ?? 'no reason reported', tone: 'none' };
  }
  if (p.status !== 'ok') return { label, value: '—', sub: 'Needs at least one verified buy', tone: 'none' };
  const d = differenceView(p.unrealized_usd, p.unrealized_pct);
  if (d.tone === 'none') {
    return { label, value: '—', sub: 'No spot price in this snapshot, so no comparison', tone: 'none' };
  }
  return {
    label,
    value: d.text,
    // Only what was read: the buys' NOCK valued at spot. Sells and LP activity are not
    // tracked, so nothing here says that NOCK is still held.
    sub: `${nockText(p.acquired_nock)} NOCK is worth ${usdText(p.value_at_spot_usd)} at spot ${priceText(spot)}; it cost ${usdText(p.spent_usdc)}. This values the NOCK these buys acquired; NockNock does not track whether it was later sold or moved into LP.`,
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

/** A real gap in the stored spot line. Only some carry a label, so none overprint. */
export type CostGap = GapRow & { labelled: boolean };

/** A run of backfilled third-party candles, captioned once (issue #137). */
export interface BackfillStretch {
  start: number;
  end: number;
  caption: string;
}

/** Labelled gaps closer than this share of the x range to the last label stay unlabelled. */
export const GAP_LABEL_MIN_FRACTION = 0.12;

/**
 * The candle period of the backfilled points: the median spacing between consecutive
 * backfilled points (3,600,000 for hourly candles). Null with fewer than two.
 */
export function candlePeriodMs(rows: SpotRow[]): number | null {
  const sorted = rows.filter((r) => Number.isFinite(r.t)).sort((a, b) => a.t - b.t);
  const spacings: number[] = [];
  for (let i = 1; i < sorted.length; i++) {
    if (sorted[i].backfilled && sorted[i - 1].backfilled) spacings.push(sorted[i].t - sorted[i - 1].t);
  }
  const positive = spacings.filter((d) => d > 0).sort((a, b) => a - b);
  if (positive.length === 0) return null;
  return positive[Math.floor((positive.length - 1) / 2)];
}

/**
 * Each point's own candle period (issue #137 review): for a backfilled point, the
 * smaller spacing to the backfilled point just before or after it; null for a worker
 * read. The backfill mixes daily and hourly candles, so one period for the whole range
 * would read every daily step as a gap or hide a hole between hourly candles. A
 * backfilled point with no backfilled neighbour takes `fallback`. Pure.
 */
export function localCandlePeriods(sorted: SpotRow[], fallback: number | null): Array<number | null> {
  return sorted.map((row, i) => {
    if (!row.backfilled) return null;
    const spacings: number[] = [];
    const prev = sorted[i - 1];
    const next = sorted[i + 1];
    if (prev?.backfilled && row.t - prev.t > 0) spacings.push(row.t - prev.t);
    if (next?.backfilled && next.t - row.t > 0) spacings.push(next.t - row.t);
    return spacings.length > 0 ? Math.min(...spacings) : fallback;
  });
}

function candleCaption(periodMs: number | null): string {
  if (periodMs === 3_600_000) return 'hourly third-party candles';
  if (periodMs === 86_400_000) return 'daily third-party candles';
  return periodMs ? `third-party candles every ${formatGapDuration(periodMs)}` : 'third-party candles';
}

function median(values: number[]): number | null {
  const sorted = values.filter((v) => v > 0).sort((a, b) => a - b);
  return sorted.length === 0 ? null : sorted[Math.floor((sorted.length - 1) / 2)];
}

/**
 * The stored spot points with breaks only at real gaps (issue #137). Pure.
 *
 * A gap is measured against the cadence of the points on either side: worker reads
 * come one bucket apart, backfilled candles one candle period apart, and each candle's
 * period is its own (`localCandlePeriods`), because the backfill mixes daily and hourly
 * candles. Between two worker reads the rule is the dashboard's (more than 2.5 buckets).
 * Between two candles it is more than 2.5 of the larger of their two periods. Between a
 * candle and a worker read, the candle covers its own period, so the time beyond that
 * period is held to the worker-read rule. So consecutive candles are one stretch,
 * captioned once, and only real holes get a break and count as gaps.
 * Gap labels are thinned so none overprints another: a gap starting within
 * `GAP_LABEL_MIN_FRACTION` of the x range after the last labelled one is shaded but
 * not labelled. Rows with no real neighbour are marked `isolated` (drawn as dots).
 */
export function spotChartSeries(
  spotRows: SpotRow[],
  bucketSeconds: number | null,
  domain?: [number, number],
): { rows: Array<SpotRow | CostGap>; gaps: CostGap[]; backfill: BackfillStretch[] } {
  const sorted = spotRows.filter((r) => Number.isFinite(r.t)).sort((a, b) => a.t - b.t);
  const readThreshold = gapThresholdMs(bucketSeconds);
  const periods = localCandlePeriods(sorted, candlePeriodMs(sorted));
  const periodOf = new Map<SpotRow, number | null>(sorted.map((r, i) => [r, periods[i]]));
  const span = domain ? domain[1] - domain[0] : sorted.length > 1 ? sorted[sorted.length - 1].t - sorted[0].t : 0;
  const minLabelGap = span > 0 ? span * GAP_LABEL_MIN_FRACTION : 0;

  const rows: Array<SpotRow | CostGap> = [];
  const gaps: CostGap[] = [];
  let lastLabel: number | null = null;
  sorted.forEach((row, i) => {
    if (i > 0) {
      const prev = sorted[i - 1];
      const dt = row.t - prev.t;
      const pPrev = periods[i - 1] ?? 0;
      const pRow = periods[i] ?? 0;
      let threshold: number;
      if (prev.backfilled && row.backfilled) {
        threshold = Math.max(readThreshold, Math.max(pPrev, pRow) * GAP_CADENCE_MULTIPLIER);
      } else if (prev.backfilled || row.backfilled) {
        threshold = readThreshold + (prev.backfilled ? pPrev : pRow);
      } else {
        threshold = readThreshold;
      }
      if (Number.isFinite(dt) && dt > threshold) {
        const labelled = lastLabel === null || prev.t - lastLabel >= minLabelGap;
        if (labelled) lastLabel = prev.t;
        const gap: CostGap = {
          gap: true,
          t: prev.t + Math.floor(dt / 2),
          time: `no stored price for ${formatGapDuration(dt)}`,
          gapStart: prev.t,
          gapEnd: row.t,
          gapMs: dt,
          labelled,
        };
        rows.push(gap);
        gaps.push(gap);
      }
    }
    rows.push(row);
  });
  rows.forEach((r, i) => {
    if (isGapRow(r)) return;
    const prev = rows[i - 1];
    const next = rows[i + 1];
    r.isolated = !((prev !== undefined && !isGapRow(prev)) || (next !== undefined && !isGapRow(next)));
  });

  // One caption per unbroken run of two or more candles of one period.
  const backfill: BackfillStretch[] = [];
  let run: SpotRow[] = [];
  let runSpacing: number | null = null;
  const flush = () => {
    if (run.length >= 2) {
      const period = median(run.map((r) => periodOf.get(r) ?? 0));
      backfill.push({ start: run[0].t, end: run[run.length - 1].t, caption: candleCaption(period) });
    }
    run = [];
    runSpacing = null;
  };
  for (const r of rows) {
    if (isGapRow(r) || !r.backfilled) {
      flush();
      continue;
    }
    const last = run[run.length - 1];
    if (last) {
      const spacing = r.t - last.t;
      // A change of candle period (daily to hourly) starts a stretch of its own.
      if (runSpacing !== null && (spacing > runSpacing * GAP_CADENCE_MULTIPLIER || spacing * GAP_CADENCE_MULTIPLIER < runSpacing)) {
        flush();
      } else if (runSpacing === null) {
        runSpacing = spacing;
      }
    }
    run.push(r);
  }
  flush();
  return { rows, gaps, backfill };
}

export interface CostChart {
  /** x domain in epoch ms: the range the worker served. */
  domain: [number, number];
  /** Evenly spaced x ticks across the domain, ends included. */
  ticks: number[];
  /** Stored spot points with break rows between runs. Empty when there is none. */
  spot: Array<SpotRow | CostGap>;
  /** Real gaps only (issue #137); a backfilled stretch is not a gap. */
  gaps: CostGap[];
  /** Runs of backfilled candles, one caption each. */
  backfill: BackfillStretch[];
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
  // Breaks at 2.5x the cadence of the points on either side: the bucket for worker
  // reads, the candle period for backfilled candles (issue #137).
  const { rows: spot, gaps, backfill } = spotChartSeries(spotRows, bucket, [from, to]);

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
    backfill,
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
