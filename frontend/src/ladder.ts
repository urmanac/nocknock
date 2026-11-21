/**
 * The ladder simulator (#166): a read-only what-if for a USDC/NOCK range that is mostly
 * USDC and sits below spot. The worker works it out from the stored snapshot
 * (GET /v1/simulate/ladder, specs/core/api.yaml); this file builds the query, checks the
 * form the way the worker does, and words the result as measurements. Nothing here
 * prefills an amount or says what to do.
 */
import type { Position } from './types/snapshot';

export interface LadderBounds {
  low_usd: string;
  high_usd: string;
  low_pct: string;
  high_pct: string;
  tick_lower: number | null;
  tick_upper: number | null;
  high_rule?: string;
  /** A ladder of rungs (2.10.0): what the top-level figures span. */
  describes?: string;
}

export interface LadderGridRow {
  price_usd: string;
  pool_price_usd: string;
  pct_of_spot: string;
  ladder_nock: string;
  ladder_nock_acquired: string;
  ladder_usdc: string;
  existing_nock_lp_usdc: string | null;
  existing_nock_lp_nock: string | null;
  stable_usdc: string | null;
  reserve_headroom_usdc: string | null;
  total_tracked_nock: string | null;
  markers: string[];
  /** A ladder of rungs only (2.10.0): each rung here, the rungs in range and their
   *  summed fee per day (null with est_fee_reason). */
  rungs?: Array<{ index: number; ladder_usdc: string; ladder_nock: string; in_range: boolean }>;
  rungs_in_range?: number[];
  est_fee_usd_per_day?: string | null;
  est_fee_usd_per_day_diluted?: string | null;
  est_fee_reason?: string | null;
}

/** Fees per day while in range (2.10.0): measured on our own tracked range, always with
 *  the in-range share of stored history and the replay upper bounds beside it. */
export interface LadderFeeEstimate {
  status: 'measured' | 'unavailable';
  reason: string | null;
  measured_by: string | null;
  measured_label: string | null;
  measured_at: string | null;
  measurement_age_seconds: number | null;
  measured_seconds: number | null;
  measured_days: string | null;
  intervals: number | null;
  skipped: Record<string, number> | null;
  skipped_seconds: Record<string, number> | null;
  rate: { usdc_per_liquidity_day: string; nock_per_liquidity_day: string; usd_per_1e15_liquidity_day: string; valued_at_pool_price_usd: string } | null;
  per_day_in_range_usd: string | null;
  per_day_in_range_usdc_leg: string | null;
  per_day_in_range_nock_leg: string | null;
  per_100_usd_per_day_in_range_usd: string | null;
  per_day_in_range_diluted_usd: string | null;
  dilution: { active_liquidity_raw: string | null; new_liquidity_raw: string; factor: string; share_of_active_pct: string } | null;
  dilution_reason: string | null;
  spread: {
    days_counted: number;
    day_min_seconds: number;
    min_usd_per_day: string | null;
    median_usd_per_day: string | null;
    max_usd_per_day: string | null;
    days: Array<{ date: string; seconds: number; usd_per_day: string; counted: boolean }>;
    note: string;
  } | null;
  band_measured: { usd_per_day: string | null; usd_total_upper: string; seconds: string; days: string } | null;
  band_reason: string | null;
  in_range_share_stored_history: { share_pct: string; seconds: string; of_seconds: string; edge_seconds: string; note: string } | null;
  in_range_share_reason: string | null;
  stored_history_replay: {
    inside_upper_usd: string;
    edge_upper_usd: string;
    inside_seconds: string;
    edge_seconds: string;
    measured_days: string;
    undiluted: boolean;
    note: string;
  } | null;
  replay_reason: string | null;
  coverage: { low_usd: string; high_usd: string; whole: boolean; note: string } | null;
  constants: { window_days: number | null; max_gap_seconds: number | null; day_min_seconds: number | null; band_min_seconds: number; note: string };
  assumptions: string[];
}

/** One rung of a ladder of rungs (2.10.0). */
export interface LadderRung {
  index: number;
  rung: 'cap' | 'near' | 'deep';
  inputs: Record<string, unknown>;
  bounds: LadderBounds;
  sizing: {
    liquidity_raw: string;
    liquidity_human: string;
    usdc_at_spot: string;
    nock_needed_at_spot: string;
    nock_given: string | null;
    nock_check: 'not_given' | 'matches';
    usdc_raw: string;
    nock_raw: string;
  };
  bottom: { price_usd: string; pct_of_spot: string; nock_held: string; nock_bought: string; avg_buy_price_usd: string | null };
  top: { price_usd: string; pct_of_spot: string; usdc_held: string; out_of_range_above_now: boolean };
  max_usdc_with_the_others_as_given: string | null;
  fee_estimate: LadderFeeEstimate;
  round_trip: { down_to_usd: string; back_to_usd: string; usdc_start: string; nock_start: string; usdc_back: string; nock_back: string; text: string } | null;
  if_pulled_at_fill: { at_usd: string; nock_kept: string; nock_bought: string; avg_price_usd: string | null; note: string } | null;
  cap?: { tick_lower: number; tick_upper: number; pool_tick: number; band_rule: string; lp_nock_budget: string | null; nock: string; usdc: string; bound_by: string; note: string };
}

export interface LadderCombined {
  rung_count: number;
  max_rungs: number;
  max_rungs_note: string;
  total_usdc: string;
  total_nock_needed: string;
  deepest_bottom_usd: string;
  nock_held_at_deepest_bottom: string;
  nock_bought_at_deepest_bottom: string;
  avg_buy_price_usd: string | null;
  deep_rungs: number;
  nock_kept_if_deep_rungs_pulled: string;
  nock_bought_by_deep_rungs: string;
  avg_kept_price_usd: string | null;
  max_usdc_total: string | null;
  overlaps: Array<{ a: number; b: number; low_usd: string; high_usd: string }>;
  gate_price_usd: string;
  gate_price_rule: string;
}

/** Measurement tables, in the order of the quantity varied, never ranked. */
export interface LadderTradeoff {
  split: { step_pct: number; step_note: string; varies: string; rows: Array<Record<string, unknown>> } | null;
  split_reason: string | null;
  near_depth: { rows: Array<Record<string, unknown>>; skipped_rows: unknown[]; ticks_between: number; max_rows: number; max_rows_note: string; truncated: boolean } | null;
  near_depth_reason: string | null;
  deep_top: { rows: Array<Record<string, unknown>>; skipped_rows: unknown[]; ticks_between: number; max_rows: number; max_rows_note: string; truncated: boolean } | null;
  deep_top_reason: string | null;
  statement: string;
  order: string;
  columns_note: string;
}

export interface LadderCrossingView {
  status: string;
  crossing: { level_nock: string; price_usd: string | null; pct_above_spot: string | null; multiple: string | null; status: string } | null;
  tolerated: { level_nock: string; price_usd: string | null; pct_above_spot: string | null; multiple: string | null; status: string } | null;
  reason: string | null;
  /** Tracked NOCK at spot: the same before and after (the ladder's NOCK leaves the wallet). */
  nock_now?: string | null;
}

export interface LadderSimulation {
  snapshot: {
    snapshot_id: string;
    generated_at: string;
    version: string;
    age_seconds: number;
    age_minutes: number;
    worker_health: string;
    spot_usd: string;
    spot_source: string;
  };
  inputs: Record<string, unknown>;
  pool: {
    position_id: string | null;
    label: string;
    pool_price_usd: string;
    pool_price_vs_spot_pct: string;
    tick_spacing: number | null;
    spacing_source: 'position' | 'caller' | null;
    snapped: boolean;
    note: string;
  };
  bounds: LadderBounds;
  top_neighbours: Array<{
    tick: number;
    price_usd: string;
    pct_of_spot: string;
    nock_needed: string | null;
    fits_budget: boolean | null;
    selected: boolean;
  }> | null;
  top_neighbours_reason: string | null;
  sizing: {
    /** Null for a ladder of rungs: liquidity is per rung (2.10.0). */
    liquidity_raw: string | null;
    liquidity_human: string | null;
    usdc_at_spot: string;
    nock_needed_at_spot: string;
    nock_given: string | null;
    nock_check: 'not_given' | 'matches' | 'per_rung';
    describes?: string;
  };
  grid: LadderGridRow[];
  bottom: { price_usd: string; pct_of_spot: string; nock_held: string; nock_bought: string; avg_buy_price_usd: string | null; describes?: string };
  top: {
    describes?: string;
    price_usd: string;
    pct_of_spot: string;
    multiple_of_spot: string;
    usdc_held: string;
    out_of_range_above_now: boolean;
    avg_sell_price_usd: string | null;
  };
  reserve_check: {
    status: 'holds' | 'breached' | 'not_configured' | 'invalid' | 'unknown';
    gate: string;
    cash_reserve_usdc: string | null;
    stable_at_spot_usdc: string | null;
    headroom_at_spot_usdc: string | null;
    existing_ranges_usdc_at_bottom: string | null;
    stable_at_bottom_usdc: string | null;
    margin_usdc: string | null;
    max_usdc_for_this_range: string | null;
    at_top: { price_usd: string; out_of_range_above_now: boolean; ladder_usdc: string; stable_usdc: string | null; headroom_usdc: string | null; note: string };
    existing_ranges_below_bottom: { price_usd: string; pct_of_spot: string; stable_usdc: string; headroom_usdc: string | null; note: string } | null;
    existing_ranges_below_bottom_reason: string | null;
    accumulation_plan_unspent: { usdc: string; note: string | null; is: string } | null;
    accumulation_plan_unspent_reason: string | null;
    tax_reserve_usd: { usd: string; is: string } | null;
    reason: string | null;
    /** Where the gate is read (2.10.0): always for rungs; for one rung only when
     *  operator_profile.ladder_bottom_usd is set. */
    gate_price_usd?: string;
    gate_price_rule?: string;
  };
  floor_check: {
    status: 'fits' | 'exceeds' | 'unknown';
    lp_nock_budget: string | null;
    budget_note: string | null;
    nock_needed: string;
    rebuild_suggested_nock: string;
    rebuild_note: string;
    max_usdc_within_budget: string | null;
    /** A ladder of rungs only (2.10.0). */
    nock_needed_by_rung?: Array<{ index: number; nock_needed: string }>;
    floor_breach_effect: {
      status: string;
      before: LadderCrossingView | null;
      after: LadderCrossingView | null;
      reason: string;
      assumption: string | null;
    };
    reason: string | null;
  };
  /** One rung (2.10.0): the fee estimate. Null for a ladder of rungs (per rung there). */
  fee_estimate?: LadderFeeEstimate | null;
  fee_estimate_reason?: string;
  /** A ladder of rungs and shape=configured only (2.10.0). */
  rungs?: LadderRung[];
  combined?: LadderCombined;
  tradeoff?: LadderTradeoff | null;
  tradeoff_reason?: string | null;
  assumptions: string[];
  statement: string;
}

/** One rung of an indexed query (2.10.0): the operator's own values, none defaulted. */
export interface LadderRungQuery {
  rung: 'cap' | 'near' | 'deep';
  usdc: string;
  nock?: string;
  low?: string;
  low_pct?: string;
  /** A price, or 'spot'. */
  high?: string;
  high_pct?: string;
}

/** The GET /v1/simulate/ladder query for a ladder of rungs (r1_..r4_), or for the
 *  operator's configured shape (`shape: true`). Throws on a rung with no role or more
 *  than four rungs, the worker's own 400s. */
export function buildRungsQuery(
  rungs: LadderRungQuery[] | 'configured',
  opts: { positionId?: string; shapeUsdc?: string; splitStepPct?: string } = {},
): URLSearchParams {
  const q = new URLSearchParams();
  if (rungs === 'configured') {
    q.set('shape', 'configured');
    if (opts.shapeUsdc && opts.shapeUsdc.trim() !== '') q.set('shape_usdc', opts.shapeUsdc.trim());
  } else {
    if (rungs.length === 0 || rungs.length > 4) throw new Error('a ladder of rungs has 1 to 4 rungs');
    rungs.forEach((r, i) => {
      if (!['cap', 'near', 'deep'].includes(r.rung)) throw new Error(`r${i + 1}_rung is required (cap, near or deep)`);
      const n = `r${i + 1}_`;
      q.set(`${n}rung`, r.rung);
      for (const k of ['usdc', 'nock', 'low', 'low_pct', 'high', 'high_pct'] as const) {
        const v = r[k];
        if (v != null && v.trim() !== '') q.set(`${n}${k}`, v.trim());
      }
    });
    if (opts.splitStepPct && opts.splitStepPct.trim() !== '') q.set('split_step_pct', opts.splitStepPct.trim());
  }
  if (opts.positionId) q.set('position_id', opts.positionId);
  return q;
}

/** A 400 or 422 body: kept whole, so the sub-code, nock_needed and bounds are shown. */
export interface LadderErrorBody {
  error: 'INVALID_PARAMETER' | 'LADDER_NOT_SIMULATED' | string;
  code?: string;
  detail: string;
  nock_needed?: string;
  bounds?: LadderBounds;
}

export type LadderOutcome =
  | { ok: true; sim: LadderSimulation }
  | { ok: false; status: number; error: LadderErrorBody };

export interface LadderForm {
  usdc: string;
  /** Empty: the worker works out the NOCK the range needs. */
  nock: string;
  lowMode: 'usd' | 'pct';
  low: string;
  highMode: 'usd' | 'pct' | 'spot';
  high: string;
  /** A tracked position_id, '' for unsnapped, or 'spacing' for a stated tick spacing. */
  pool: string;
  tickSpacing: string;
  gridToPct: string;
}

/** The empty form: no amount is ever prefilled (the operator's numbers are never defaulted). */
export const EMPTY_LADDER_FORM: LadderForm = {
  usdc: '',
  nock: '',
  lowMode: 'usd',
  low: '',
  highMode: 'spot',
  high: '',
  pool: '',
  tickSpacing: '',
  gridToPct: '',
};

const isNum = (v: string) => v.trim() !== '' && Number.isFinite(Number(v));

/** The worker's 400 rules, checked before asking it. Null when the form can be sent. */
export function validateLadderForm(f: LadderForm): string | null {
  if (!isNum(f.usdc) || Number(f.usdc) <= 0) return 'USDC must be a number above 0';
  if (f.nock.trim() !== '' && (!isNum(f.nock) || Number(f.nock) < 0)) return 'NOCK must be empty or a number of 0 or more';
  if (!isNum(f.low)) return 'the bottom must be a number';
  if (f.lowMode === 'usd' && Number(f.low) <= 0) return 'the bottom must be above $0';
  if (f.lowMode === 'pct' && Number(f.low) <= -100) return 'the bottom must be above -100%';
  if (f.highMode !== 'spot') {
    if (!isNum(f.high)) return 'the top must be a number, or spot';
    if (f.highMode === 'usd' && Number(f.high) <= 0) return 'the top must be above $0';
    if (f.highMode === 'pct' && Number(f.high) <= -100) return 'the top must be above -100%';
    if (f.lowMode === f.highMode && Number(f.low) >= Number(f.high)) return 'the bottom must be below the top';
  }
  if (f.pool === 'spacing') {
    const s = Number(f.tickSpacing);
    if (!Number.isInteger(s) || s < 1 || s > 32767) return 'the tick spacing must be a whole number from 1 to 32767';
  }
  if (f.gridToPct.trim() !== '' && (!isNum(f.gridToPct) || Number(f.gridToPct) < -95 || Number(f.gridToPct) > -1)) {
    return 'the grid depth must be from -95% to -1%';
  }
  return null;
}

/** The GET /v1/simulate/ladder query for a valid form. */
export function buildLadderQuery(f: LadderForm): URLSearchParams {
  const q = new URLSearchParams();
  q.set('usdc', f.usdc.trim());
  if (f.nock.trim() !== '') q.set('nock', f.nock.trim());
  q.set(f.lowMode === 'usd' ? 'low' : 'low_pct', f.low.trim());
  if (f.highMode === 'spot') q.set('high', 'spot');
  else q.set(f.highMode === 'usd' ? 'high' : 'high_pct', f.high.trim());
  if (f.pool === 'spacing') q.set('tick_spacing', f.tickSpacing.trim());
  else if (f.pool !== '') q.set('position_id', f.pool);
  if (f.gridToPct.trim() !== '') q.set('grid_to_pct', f.gridToPct.trim());
  return q;
}

export interface LadderPoolOption {
  value: string;
  label: string;
  disabled: boolean;
  reason: string | null;
}

/** The pools a ladder can snap to: tracked USDC/NOCK positions; the others say why not. */
export function ladderPoolOptions(positions: Position[] | null | undefined): LadderPoolOption[] {
  const opts: LadderPoolOption[] = [
    { value: '', label: 'Unsnapped (no pool)', disabled: false, reason: null },
    { value: 'spacing', label: 'A tick spacing I state', disabled: false, reason: null },
  ];
  for (const p of positions ?? []) {
    const syms = [p.token0.symbol, p.token1.symbol];
    if (!(syms.includes('USDC') && syms.includes('NOCK'))) continue;
    const label = `${p.protocol} ${p.token0.symbol}/${p.token1.symbol} #${p.nft_id || p.pool_id.slice(0, 10)}`;
    if (p.full_range) {
      opts.push({ value: p.position_id, label, disabled: true, reason: 'a basic Aero pool has no ranges: its deposits are full range' });
    } else if (p.tick_spacing == null) {
      opts.push({ value: p.position_id, label, disabled: true, reason: 'tick spacing not read for this pool' });
    } else {
      opts.push({ value: p.position_id, label: `${label} (tick spacing ${p.tick_spacing})`, disabled: false, reason: null });
    }
  }
  return opts;
}

const grouped = (v: string | null | undefined) => {
  const n = Number(v);
  return v == null || !Number.isFinite(n) ? 'unknown' : Math.round(n).toLocaleString('en-US');
};
const signedUsd = (h: string) => (h.startsWith('-') ? `-$${h.slice(1)}` : `+$${h}`);

/** A grid row's NOCK movement: "1,234 bought", or "4,194 sold" above spot (never a
 *  negative purchase), as the MCP text says it. */
export function ladderNockMoved(acquired: string): string {
  const n = Number(acquired);
  if (!Number.isFinite(n)) return 'unknown';
  const r = Math.round(n) + 0; // never "-0"
  return r < 0 ? `${(-r).toLocaleString('en-US')} sold` : `${r.toLocaleString('en-US')} bought`;
}

/** "From the snapshot generated … (N min ago) · worker health: ok": its age and health. */
export function ladderSnapshotLine(sim: LadderSimulation): string {
  const s = sim.snapshot;
  return `From the snapshot generated ${s.generated_at} (${s.age_minutes} min ago) · worker health: ${s.worker_health}`;
}

/** The result as measured lines, in the MCP text's words. */
export function ladderSummaryLines(sim: LadderSimulation): string[] {
  const lines: string[] = [];
  const b = sim.bounds;
  const pool = sim.pool.snapped
    ? `snapped to ${sim.pool.label} (tick spacing ${sim.pool.tick_spacing}${sim.pool.spacing_source === 'caller' ? ', given by you' : ''})`
    : 'unsnapped: no pool or tick spacing was given';
  lines.push(`Range: $${b.low_usd} – $${b.high_usd} (${b.low_pct}% / ${b.high_pct}% of the pool price $${sim.pool.pool_price_usd}), ${pool}`);
  lines.push(`Sizing: $${sim.sizing.usdc_at_spot} USDC and ${grouped(sim.sizing.nock_needed_at_spot)} NOCK at spot`);
  if (sim.top_neighbours) {
    lines.push(
      'Tops next to spot: ' +
        sim.top_neighbours
          .map((t) => {
            const need = t.nock_needed == null ? 'is the bottom’s own tick' : `needs ${grouped(t.nock_needed)} NOCK`;
            const fit = t.fits_budget == null ? '' : t.fits_budget ? ' (fits the budget)' : ' (exceeds the budget)';
            return `$${t.price_usd} (${t.pct_of_spot}%) ${need}${fit}`;
          })
          .join('; '),
    );
  }
  lines.push(
    `Bottom: fully NOCK at $${sim.bottom.price_usd} (${sim.bottom.pct_of_spot}%): ${grouped(sim.bottom.nock_held)} NOCK held, average $${sim.bottom.avg_buy_price_usd ?? 'unknown'}`,
  );
  lines.push(
    sim.top.out_of_range_above_now
      ? `Top: $${sim.top.price_usd} (${sim.top.pct_of_spot}%): out of range above now, all USDC ($${sim.top.usdc_held})`
      : `Top: out of range above at $${sim.top.price_usd} (${sim.top.pct_of_spot}%): $${sim.top.usdc_held} USDC`,
  );
  const rc = sim.reserve_check;
  if (rc.status === 'holds' || rc.status === 'breached') {
    const m = rc.margin_usdc ?? '';
    const verdict = m.startsWith('-') ? `breached: below it by $${m.slice(1)}` : `holds by $${m}`;
    lines.push(
      `Reserve: stable $${rc.stable_at_bottom_usdc} at the bottom against the $${rc.cash_reserve_usdc} reserve: ${verdict} (this range takes at most $${rc.max_usdc_for_this_range ?? 'unknown'})`,
    );
  } else {
    lines.push(`Reserve: ${rc.status}: ${rc.reason ?? ''}`);
  }
  if (rc.existing_ranges_below_bottom) {
    const x = rc.existing_ranges_below_bottom;
    lines.push(
      `Below the bottom: at $${x.price_usd} (${x.pct_of_spot}%) stable $${x.stable_usdc}${x.headroom_usdc ? ` (reserve ${signedUsd(x.headroom_usdc)})` : ''}; a measurement, not the gate`,
    );
  }
  const fc = sim.floor_check;
  if (fc.status === 'unknown') {
    lines.push(`Floor budget: unknown: ${fc.reason ?? ''}`);
  } else {
    const cap = fc.max_usdc_within_budget ? `; $${fc.max_usdc_within_budget} is the most USDC this range holds within it` : '';
    // The note says why the budget is not a plain one (no floor set, or negative), as the
    // MCP text does: "fits" with no floor, or a negative budget, is never shown bare.
    const note = fc.budget_note ? ` (${fc.budget_note})` : '';
    lines.push(`Floor budget: needs ${grouped(fc.nock_needed)} of the ${grouped(fc.lp_nock_budget)} NOCK that may go into LP${note}: ${fc.status}${cap}`);
  }
  const e = fc.floor_breach_effect;
  const at = (v: LadderCrossingView | null) =>
    v?.crossing?.price_usd ? `$${v.crossing.price_usd} (+${v.crossing.pct_above_spot}%)` : (v?.status ?? 'unknown');
  lines.push(
    e.status === 'unknown' || e.status === 'no_floor'
      ? `Floor-crossing price: ${e.status}: ${e.reason}`
      : `Floor-crossing price: ${e.status}: ${e.reason} (without the ladder ${at(e.before)}, with it ${at(e.after)})`,
  );
  return lines;
}

/** A 400 or 422 as measured lines. `nock_needed` is a figure, never an instruction. */
export function ladderErrorLines(err: LadderErrorBody): string[] {
  const lines = [`${err.code ?? err.error}: ${err.detail}`];
  if (err.nock_needed != null) lines.push(`Measured: this range needs ${err.nock_needed} NOCK at the pool price for this USDC`);
  if (err.bounds) lines.push(`Bounds: $${err.bounds.low_usd} – $${err.bounds.high_usd} (${err.bounds.low_pct}% / ${err.bounds.high_pct}%)`);
  return lines;
}
