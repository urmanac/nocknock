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
