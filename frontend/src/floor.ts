/**
 * Pure display helpers for the floor and the market regime (v0.5.1). They separate
 * "the worker reported no rule" (floor "0", regime "unknown") from "the field is
 * missing", so a configured "no floor" is never shown as UNKNOWN and a missing value
 * is never shown as a number.
 */

export type FloorState =
  /** The field is absent or unparsable. */
  | { kind: 'unavailable' }
  /** The worker reported "0": the operator configured no floor. */
  | { kind: 'none' }
  /** A floor is set. `delta` is total NOCK minus the floor, or null when unknown. */
  | { kind: 'set'; target: number; delta: number | null };

function parseNum(v: string | number | undefined | null): number | null {
  if (v === undefined || v === null) return null;
  const s = String(v).trim().replace(/,/g, '');
  if (s === '') return null;
  const n = Number(s);
  return Number.isFinite(n) ? n : null;
}

export function floorState(
  target: string | undefined | null,
  delta: string | undefined | null,
  totalNock?: string | undefined | null,
): FloorState {
  const t = parseNum(target);
  if (t === null) return { kind: 'unavailable' };
  if (t <= 0) return { kind: 'none' };
  const d = parseNum(delta);
  if (d !== null) return { kind: 'set', target: t, delta: d };
  const total = parseNum(totalNock);
  return { kind: 'set', target: t, delta: total !== null ? total - t : null };
}

/** "+13,863 above the 100,000 floor" or "100,000 below the 100,000 floor". */
export function floorDeltaText(delta: number, target: number): string {
  const t = Math.round(target).toLocaleString('en-US');
  if (delta >= 0) return `+${Math.round(delta).toLocaleString('en-US')} above the ${t} floor`;
  return `${Math.round(-delta).toLocaleString('en-US')} below the ${t} floor`;
}

/** One-line floor summary for the dashboard. */
export function floorSummaryText(state: FloorState): string {
  if (state.kind === 'unavailable') return 'floor target unavailable';
  if (state.kind === 'none') return 'No floor configured';
  if (state.delta === null) return `${Math.round(state.target).toLocaleString('en-US')} NOCK floor; buffer unavailable`;
  return floorDeltaText(state.delta, state.target);
}

/** Regime badge text: UNKNOWN when the worker computed no daily RSI. */
export function regimeLabel(regime: string | undefined | null): string {
  if (regime === 'overextended') return 'OVEREXTENDED';
  if (regime === 'normal') return 'NORMAL';
  if (regime === 'unknown') return 'UNKNOWN (no daily RSI yet)';
  return '—';
}

/** Badge tone: only a measured regime gets a colour; unknown is neutral. */
export function regimeTone(regime: string | undefined | null): 'warn' | 'ok' | 'neutral' {
  if (regime === 'overextended') return 'warn';
  if (regime === 'normal') return 'ok';
  return 'neutral';
}

// ============================================================
// The floor-crossing price (snapshot 2.9.0, issue #165)
// ============================================================

/** Where one level is crossed (snapshot FloorCrossing). */
export interface FloorCrossing {
  level_nock: string;
  /** crosses | already_below | never */
  status: string;
  multiple?: string | null;
  price_usd?: string | null;
  pct_above_spot?: string | null;
}

/** One NOCK-pair position in the figure (snapshot FloorBreachPosition). */
export interface FloorBreachPosition {
  position_id: string;
  label: string;
  full_range: boolean;
  nock_now: string;
  nock_at_crossing?: string | null;
  sold_out_at_multiple?: string | null;
  sold_out_at_price_usd?: string | null;
  note?: string | null;
}

/** derived_metrics.floor_breach: how far NOCK can rise before the ranges sell the stack below the floor. */
export interface FloorBreach {
  /** crosses | already_below | never | no_floor | unavailable */
  status: string;
  floor_nock: string;
  spot_price_usd?: string | null;
  crossing?: FloorCrossing | null;
  tolerated?: FloorCrossing | null;
  nock_now?: string | null;
  wallet_nock?: string | null;
  fees_nock?: string | null;
  nock_if_all_sold?: string | null;
  largest_seller?: string | null;
  first_to_sell_out?: string | null;
  positions: FloorBreachPosition[];
  assumption: string;
  reason?: string | null;
}

function nockText(v: string | null | undefined): string | null {
  const n = parseNum(v);
  return n === null ? null : Math.round(n).toLocaleString('en-US');
}

function remainText(fb: FloorBreach): string | null {
  const all = nockText(fb.nock_if_all_sold);
  if (all === null) return null;
  const w = nockText(fb.wallet_nock);
  const f = nockText(fb.fees_nock);
  return w !== null && f !== null ? `${all} NOCK (${w} wallet + ${f} uncollected fees)` : `${all} NOCK`;
}

function toleratedText(t: FloorCrossing | null | undefined): string {
  if (!t) return '';
  const level = nockText(t.level_nock) ?? t.level_nock;
  if (t.status === 'crosses' && t.price_usd) return ` · tolerated ${level} crossed at $${t.price_usd} (+${t.pct_above_spot}%)`;
  if (t.status === 'already_below') return ` · tolerated ${level}: already below`;
  return ` · tolerated ${level}: never crossed`;
}

/**
 * The measured line under the Floor Health tile (#165). A measurement of the deployed
 * ranges, never an instruction. Null when no floor is configured or the snapshot
 * predates 2.9.0 (no field): the tile already says what it knows.
 */
export function floorBreachLine(fb: FloorBreach | null | undefined): string | null {
  if (!fb || fb.status === 'no_floor') return null;
  if (fb.status === 'unavailable') return `Floor-crossing price unavailable: ${fb.reason ?? 'no reason given'}`;
  const remain = remainText(fb);
  const tolerated = toleratedText(fb.tolerated);
  if (fb.status === 'already_below') return `Below the floor at today's price${tolerated}`;
  if (fb.status === 'never') {
    return `No rise crosses the floor: ${remain ?? 'the NOCK left'} remain if every range sells out${tolerated}`;
  }
  if (fb.status !== 'crosses' || !fb.crossing?.price_usd) return `Floor-crossing price unavailable: status ${fb.status}`;
  const first = fb.first_to_sell_out ? fb.positions.find((p) => p.label === fb.first_to_sell_out) : undefined;
  let who = '';
  if (fb.first_to_sell_out) {
    who = `; ${fb.first_to_sell_out} sells out first${first?.sold_out_at_price_usd ? ` (at $${first.sold_out_at_price_usd})` : ''}`;
  } else if (fb.positions.some((p) => p.full_range)) {
    who = '; no range sells out (only full-range pools hold NOCK)';
  }
  const allSold = remain ? ` · if every range sells out, ${remain} remain` : '';
  return `Floor crossed if NOCK ≥ $${fb.crossing.price_usd} (+${fb.crossing.pct_above_spot}%)${who}${allSold}${tolerated}`;
}
