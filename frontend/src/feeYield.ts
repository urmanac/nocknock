/**
 * The fee yield (snapshot 2.10.0): swap fees our own tracked range earned per unit of
 * liquidity while in range, measured by the worker from stored snapshots
 * (derived_metrics.fee_yield, specs/core/snapshot.yaml), and the operator's configured
 * ladder shape it sizes (derived_metrics.ladder_shape). Pure helpers: a quiet line for
 * the measurement and a by-day view of its spread. Nothing here is a forecast, prefills
 * an amount or says what to do; every figure is the worker's, with its seconds beside it.
 */

export interface FeeYieldSkips {
  liquidity_changed: number;
  zero_liquidity: number;
  fees_fell: number;
  pool_inactive: number;
  out_of_range: number;
  gap: number;
  unreadable: number;
  /** Intervals with a row staked in a gauge: their fees went to the gauge. */
  staked: number;
}

export interface FeeYieldDay {
  /** A UTC date, YYYY-MM-DD; an interval counts on the date it starts. */
  date: string;
  seconds: number;
  usdc_per_liquidity_day: string;
  nock_per_liquidity_day: string;
  /** At this cycle's pool price; null when it could not be read. */
  usd_per_liquidity_day: string | null;
}

export interface FeeYieldSpan {
  bin_lo: number;
  bin_hi: number;
  seconds: number;
  usdc_per_liquidity: string;
  nock_per_liquidity: string;
}

export interface FeeYieldPool {
  pool_id: string;
  label: string;
  measured_by: string;
  fee_tier_bps: number;
  tick_spacing: number | null;
  covered_tick_lower: number;
  covered_tick_upper: number;
  status: 'measured' | 'no_intervals';
  reason: string | null;
  from: string | null;
  to: string | null;
  rows: number;
  intervals: number;
  zero_fee_gap_intervals: number;
  measured_seconds: number;
  skipped: FeeYieldSkips;
  skipped_seconds: Record<keyof FeeYieldSkips, number>;
  usdc_per_liquidity_day: string | null;
  nock_per_liquidity_day: string | null;
  pool_price_usd: string | null;
  usd_per_liquidity_day: string | null;
  usd_per_1e15_liquidity_day: string | null;
  by_day: FeeYieldDay[];
  spans: FeeYieldSpan[];
  spans_reason: string | null;
}

export interface FeeYield {
  status: 'measured' | 'unavailable';
  reason: string | null;
  computed_at: string | null;
  window_days: number;
  max_gap_seconds: number;
  day_min_seconds: number;
  refresh_seconds: number;
  constants_note: string;
  pools: FeeYieldPool[];
  excluded: Array<{ position_id: string; reason: string }>;
  statement: string;
}

/** derived_metrics.ladder_shape (2.10.0): the operator's shape, sized by the worker. The
 *  card branch reads it; only its outline is typed here. */
export interface LadderShapeRung {
  index: number;
  rung: 'cap' | 'near' | 'deep';
  share: string | null;
  /** Against the tracked rungs (2.10.0 review): what is left to deposit, if anything. */
  status: 'to_deposit' | 'deployed' | 'over' | 'passed' | 'none';
  reason: string | null;
  target_usdc: string;
  target_nock: string | null;
  deployed_usdc: string;
  deployed_nock: string | null;
  remaining_usdc: string;
  remaining_nock: string | null;
  over_by_usdc: string | null;
  counterparts: string[];
  /** What a mint of liquidity_raw pulls, rounded up (null unless to_deposit). */
  amount0_raw: string | null;
  amount1_raw: string | null;
  tick_lower: number | null;
  tick_upper: number | null;
  low_usd: string;
  high_usd: string;
  nock_per_usdc_at_low: string | null;
  nock_per_usdc_at_high: string | null;
  in_range_now: boolean | null;
  out_of_range_above_now: boolean;
  liquidity_raw: string | null;
  liquidity_human: string | null;
  usdc: string | null;
  usdc_raw: string | null;
  nock: string | null;
  nock_raw: string | null;
  nock_held_at_bottom: string;
  nock_bought_at_bottom: string;
  avg_buy_price_usd: string | null;
  fee_estimate: import('./ladder').LadderFeeEstimate;
  cap: Record<string, unknown> | null;
}

export interface LadderShape {
  status: 'computed' | 'deployed' | 'price_at_or_below_bottom' | 'unavailable';
  reason: string | null;
  source: string;
  statement: string;
  computed_at?: string;
  total_usdc?: string;
  total_rule?: string;
  rest_usdc?: string;
  cap?: 'sized' | 'none' | 'not in the shape';
  cap_reason?: string | null;
  rungs?: LadderShapeRung[];
  combined?: Record<string, unknown>;
  gate?: Record<string, unknown>;
  nock_budget?: Record<string, unknown>;
  floor_breach_effect?: Record<string, unknown>;
}

const num = (v: string | null | undefined): number | null => {
  if (v == null) return null;
  const n = Number(v);
  return Number.isFinite(n) ? n : null;
};

/** The measuring pool for a pool id, or the first measured one. */
export function feeYieldPool(fy: FeeYield | null | undefined, poolId?: string): FeeYieldPool | null {
  if (!fy || fy.status !== 'measured') return null;
  return fy.pools.find((p) => (poolId ? p.pool_id === poolId : true)) ?? null;
}

const grouped = (n: number) => Math.round(n).toLocaleString('en-US');
const days = (secs: number) => (secs / 86400).toFixed(2);

/** Whole minutes from `computed_at` to `now`; null when either is unreadable. */
export function feeYieldAgeMinutes(fy: FeeYield, now: Date): number | null {
  if (!fy.computed_at) return null;
  const t = Date.parse(fy.computed_at);
  if (!Number.isFinite(t)) return null;
  return Math.max(0, Math.floor((now.getTime() - t) / 60000));
}

/**
 * The quiet line: "Fees measured on v4 USDC/NOCK 3.00% #3163450: $6.0811 per 1e15
 * liquidity per day while in range, over 6.33 days (1,836 intervals), measured 15 min
 * ago. A past measurement, not a forecast." Or why there is none.
 */
export function feeYieldLine(fy: FeeYield | null | undefined, now: Date, poolId?: string): string {
  if (!fy) return 'Fees per unit of liquidity: not measured in this snapshot (a worker before 2.10.0)';
  if (fy.status !== 'measured') return `Fees per unit of liquidity: not measured: ${fy.reason ?? 'no reason given'}`;
  const p = feeYieldPool(fy, poolId);
  if (!p) return 'Fees per unit of liquidity: no tracked range in this pool measures them';
  if (p.status !== 'measured') return `Fees measured on ${p.label}: none yet: ${p.reason ?? 'no interval could be counted'}`;
  const age = feeYieldAgeMinutes(fy, now);
  const rate = p.usd_per_1e15_liquidity_day ? `$${p.usd_per_1e15_liquidity_day} per 1e15 liquidity per day while in range` : 'a rate that could not be priced';
  return `Fees measured on ${p.label}: ${rate}, over ${days(p.measured_seconds)} days (${grouped(p.intervals)} intervals)${
    age == null ? '' : `, measured ${age} min ago`
  }. A past measurement, not a forecast.`;
}

export interface FeeYieldDayRow {
  date: string;
  hours: string;
  /** USD per 1e15 liquidity per day at this cycle's pool price; null when unpriced. */
  usd_per_1e15: number | null;
  /** True when the day has at least day_min_seconds measured and enters the spread. */
  counted: boolean;
}

export interface FeeYieldSpread {
  rows: FeeYieldDayRow[];
  days_counted: number;
  day_min_seconds: number;
  min: number | null;
  /** A true median: the mean of the two middle days for an even count. */
  median: number | null;
  max: number | null;
  /** The largest usd_per_1e15 of any day, for drawing bars on one scale; null with none. */
  scale_max: number | null;
}

/** Every UTC day of the measurement, with the spread over the days counted. */
export function feeYieldByDay(pool: FeeYieldPool, dayMinSeconds: number): FeeYieldSpread {
  const rows: FeeYieldDayRow[] = pool.by_day.map((d) => {
    const usd = num(d.usd_per_liquidity_day);
    return {
      date: d.date,
      hours: (d.seconds / 3600).toFixed(1),
      usd_per_1e15: usd == null ? null : usd * 1e15,
      counted: d.seconds >= dayMinSeconds,
    };
  });
  const vals = rows.filter((r) => r.counted && r.usd_per_1e15 != null).map((r) => r.usd_per_1e15 as number).sort((a, b) => a - b);
  const k = vals.length;
  const median = k === 0 ? null : k % 2 === 1 ? vals[(k - 1) / 2] : (vals[k / 2 - 1] + vals[k / 2]) / 2;
  const all = rows.filter((r) => r.usd_per_1e15 != null).map((r) => r.usd_per_1e15 as number);
  return {
    rows,
    days_counted: k,
    day_min_seconds: dayMinSeconds,
    min: k ? vals[0] : null,
    median,
    max: k ? vals[k - 1] : null,
    scale_max: all.length ? Math.max(...all) : null,
  };
}

/** "min $3.10, median $5.29, max $7.99 per 1e15 liquidity per day over 5 UTC days with at
 *  least 20.0 h measured", or why there is no spread. */
export function feeYieldSpreadLine(s: FeeYieldSpread): string {
  const h = (s.day_min_seconds / 3600).toFixed(1);
  if (s.days_counted === 0) return `Daily spread: no UTC day has at least ${h} h measured yet`;
  const f = (v: number | null) => (v == null ? '?' : `$${v.toFixed(2)}`);
  return `Daily spread: min ${f(s.min)}, median ${f(s.median)}, max ${f(s.max)} per 1e15 liquidity per day over ${s.days_counted} UTC day${
    s.days_counted === 1 ? '' : 's'
  } with at least ${h} h measured`;
}

/** The fields of a fee estimate its lines read: the simulator's full estimate
 *  (LadderFeeEstimate) and the summary a ladder suggestion's deposit carries both fit. */
export interface FeeEstimateForLines {
  status: string;
  reason?: string | null;
  per_day_in_range_usd?: string | null;
  measured_days?: string | null;
  intervals?: number | null;
  measured_label?: string | null;
  in_range_share_stored_history?: { share_pct: string } | null;
  in_range_share_reason?: string | null;
  stored_history_replay?: { inside_upper_usd: string; edge_upper_usd: string } | null;
  replay_reason?: string | null;
  coverage?: { whole: boolean; note: string } | null;
}

/**
 * A fee estimate as lines (2.10.0). The rules the worker states are kept here too: a fee
 * per day while in range is never printed without its sample size (days and intervals),
 * the in-range share of stored history and the replay upper bound beside it; when one of
 * those is missing its reason is printed in its place; an unavailable estimate prints
 * its reason, never a zero.
 */
export function feeEstimateLines(fe: FeeEstimateForLines | null | undefined): string[] {
  if (!fe) return ['Fees while in range: not stated in this body'];
  if (fe.status !== 'measured' || fe.per_day_in_range_usd == null) {
    return [`Fees while in range: not estimated: ${fe.reason ?? 'no reason given'}`];
  }
  const sample =
    fe.measured_days != null && fe.intervals != null
      ? `over ${fe.measured_days} days (${fe.intervals.toLocaleString('en-US')} intervals) on ${fe.measured_label ?? 'the measuring range'}`
      : 'sample size not stated';
  const lines = [`Fees while in range (an estimate): $${fe.per_day_in_range_usd}/day, measured ${sample}`];
  const sh = fe.in_range_share_stored_history;
  const rp = fe.stored_history_replay;
  lines.push(
    sh
      ? `In range ${sh.share_pct}% of the measured time`
      : `In range over stored history: not stated (${fe.in_range_share_reason ?? 'no reason given'})`,
  );
  lines.push(
    rp
      ? `Replayed over it: at most $${rp.inside_upper_usd} inside and $${rp.edge_upper_usd} more at its edges (no lower bound)`
      : `Replay: not stated (${fe.replay_reason ?? 'no reason given'})`,
  );
  if (fe.coverage && !fe.coverage.whole) lines.push(fe.coverage.note);
  return lines;
}

/**
 * The same estimate as one line, for a card that gives each rung one muted line. The
 * worker prints these words too (ladder_suggest_text.rs, fee_estimate_line): the figure
 * with its sample size, in-range share and replay bound, or the reason it is not measured.
 */
export function feeEstimateLine(fe: FeeEstimateForLines | null | undefined): string {
  const joined = feeEstimateLines(fe).join('. ');
  return joined.endsWith('.') ? joined : `${joined}.`;
}
