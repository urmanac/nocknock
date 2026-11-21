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
}

export interface LadderCrossingView {
  status: string;
  crossing: { level_nock: string; price_usd: string | null; pct_above_spot: string | null; multiple: string | null; status: string } | null;
  tolerated: { level_nock: string; price_usd: string | null; pct_above_spot: string | null; multiple: string | null; status: string } | null;
  reason: string | null;
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
    liquidity_raw: string;
    liquidity_human: string;
    usdc_at_spot: string;
    nock_needed_at_spot: string;
    nock_given: string | null;
    nock_check: 'not_given' | 'matches';
  };
  grid: LadderGridRow[];
  bottom: { price_usd: string; pct_of_spot: string; nock_held: string; nock_bought: string; avg_buy_price_usd: string | null };
  top: {
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
  };
  floor_check: {
    status: 'fits' | 'exceeds' | 'unknown';
    lp_nock_budget: string | null;
    budget_note: string | null;
    nock_needed: string;
    rebuild_suggested_nock: string;
    rebuild_note: string;
    max_usdc_within_budget: string | null;
    floor_breach_effect: {
      status: string;
      before: LadderCrossingView | null;
      after: LadderCrossingView | null;
      reason: string;
      assumption: string | null;
    };
    reason: string | null;
  };
  assumptions: string[];
  statement: string;
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
    lines.push(`Floor budget: needs ${grouped(fc.nock_needed)} of the ${grouped(fc.lp_nock_budget)} NOCK that may go into LP: ${fc.status}${cap}`);
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
