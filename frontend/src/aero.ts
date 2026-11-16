/**
 * Pure display helpers for AERO, the fourth pillar (issue #93).
 *
 * Nothing here computes a number. Each helper restates what the worker read: the
 * wallet's AERO (`wallet_aero_amount` / `wallet_aero_usd`, counted in the total), the
 * AERO earned in tracked gauges and not claimed (`unclaimed_aero_amount` /
 * `unclaimed_aero_usd` and the per-position `unclaimed_aero*` fields, NOT counted in the
 * total, like uncollected LP fees) and the AERO/USD price.
 *
 * Two rules the wording keeps:
 * - Amounts only. No helper says to claim, sell, hold or accumulate AERO, and AERO has
 *   no target.
 * - A field the snapshot does not carry (an older snapshot, or a value the worker could
 *   not read this cycle) reads "not reported" or "—", never 0.
 */

import { positionLabel } from './positions.ts';

export const AERO_NOT_REPORTED = 'not reported';

/** A snapshot string field, or null when it is absent, null or blank. */
function field(obj: any, key: string): string | null {
  const v = obj?.[key];
  if (v === undefined || v === null) return null;
  const s = String(v).trim();
  return s === '' ? null : s;
}

function amountUsd(amount: string, usd: string | null): string {
  return usd === null ? `${amount} AERO (USD value not reported)` : `${amount} AERO ($${usd})`;
}

/** "AERO held (wallet): 1.359979 AERO ($0.57), counted in the total; …". */
export function aeroHeldLine(global: any): string {
  const amount = field(global, 'wallet_aero_amount');
  if (amount === null) return `AERO held (wallet): ${AERO_NOT_REPORTED}`;
  return `AERO held (wallet): ${amountUsd(amount, field(global, 'wallet_aero_usd'))}, counted in the total; not cash, not deployable, no target`;
}

export interface AeroPositionRow {
  positionId: string;
  label: string;
  /** "0.121026 AERO ($0.05)" or "not read (reason)". */
  text: string;
  /** True when the gauge was read (an amount, possibly 0); false for a stated reason. */
  read: boolean;
}

/** One row per position that carries an AERO statement of its own. */
export function aeroPositionRows(positions: any[] | undefined | null): AeroPositionRow[] {
  const rows: AeroPositionRow[] = [];
  for (const p of positions ?? []) {
    const amount = field(p, 'unclaimed_aero');
    const note = field(p, 'unclaimed_aero_note');
    if (amount === null && note === null) continue;
    rows.push({
      positionId: String(p.position_id ?? rows.length),
      label: positionLabel(p),
      text: amount !== null ? amountUsd(amount, field(p, 'unclaimed_aero_usd')) : `not read (${note})`,
      read: amount !== null,
    });
  }
  return rows;
}

/**
 * The unclaimed total, in words. The total covers the gauges that were read; when none
 * was, a "0" would be a sum over nothing, so it is not shown as an amount.
 */
export function unclaimedAeroText(global: any, positions: any[] | undefined | null): string {
  const amount = field(global, 'unclaimed_aero_amount');
  if (amount === null) return AERO_NOT_REPORTED;
  const rows = aeroPositionRows(positions);
  if (rows.length === 0) return 'none: no staked Aero position is tracked';
  if (!rows.some((r) => r.read)) return 'no gauge was read';
  return `${amountUsd(amount, field(global, 'unclaimed_aero_usd'))} in tracked gauges, not counted in the total`;
}

export function unclaimedAeroLine(global: any, positions: any[] | undefined | null): string {
  return `Unclaimed AERO: ${unclaimedAeroText(global, positions)}`;
}

/** "AERO/USD: $0.417608", or "AERO/USD: not reported". */
export function aeroPriceLine(global: any): string {
  const price = field(global, 'aero_price_usd');
  return `AERO/USD: ${price === null ? AERO_NOT_REPORTED : `$${price}`}`;
}

/** The worker's stated reason for an AERO figure it could not read; null when none. */
export function aeroNoteLine(global: any): string | null {
  const note = field(global, 'aero_note');
  return note === null ? null : `AERO not read: ${note}`;
}

/** A position card's AERO line; null for a position with nothing to say about AERO. */
export function positionAeroText(p: any): string | null {
  const amount = field(p, 'unclaimed_aero');
  if (amount !== null) {
    const usd = field(p, 'unclaimed_aero_usd');
    return `Unclaimed AERO: ${amount}${usd === null ? ' (USD value not reported)' : ` ($${usd})`}; not counted in the total`;
  }
  const note = field(p, 'unclaimed_aero_note');
  return note === null ? null : `Unclaimed AERO: not read (${note})`;
}

/** Subtitle of the AERO stat card: its USD value and what is unclaimed beside it. */
export function aeroStatSubtitle(global: any, positions: any[] | undefined | null): string {
  if (field(global, 'wallet_aero_amount') === null) return AERO_NOT_REPORTED;
  const usd = field(global, 'wallet_aero_usd');
  const held = usd === null ? 'USD value not reported' : `$${usd} in the wallet, counted in the total`;
  return `${held} · unclaimed: ${unclaimedAeroText(global, positions)}`;
}

/** One sentence for the LP summary pill, replacing "rewards … are not read yet (#93)". */
export function aeroRewardsSentence(global: any, positions: any[] | undefined | null): string {
  const text = unclaimedAeroText(global, positions);
  if (text === AERO_NOT_REPORTED) return `Unclaimed AERO on staked Aero LP: ${AERO_NOT_REPORTED}.`;
  if (text.startsWith('none:')) return 'No staked Aero position is tracked, so no AERO is read from a gauge.';
  if (text === 'no gauge was read') return 'Unclaimed AERO on staked Aero LP: no gauge was read (see each position).';
  return `Unclaimed AERO on staked Aero LP: ${text}.`;
}

/**
 * Wallet AERO for the Portfolio Composition readout: "AERO $0.57 (wallet; not cash)".
 * The panel's Total includes wallet AERO, and its "other legs" residual has AERO taken
 * out (timeline.ts compositionReadout), so the readout states it; otherwise the Total
 * would exceed the parts shown with no explanation. Null when the snapshot does not
 * report it or it rounds to nothing (under $0.01).
 */
export function compositionAeroText(global: any): string | null {
  const usd = field(global, 'wallet_aero_usd');
  if (usd === null) return null;
  const n = Number(usd);
  if (!Number.isFinite(n) || n < 0.01) return null;
  return `AERO $${usd} (wallet; not cash)`;
}

/**
 * What the composition readout's "other legs" residual is the Total minus. It names
 * AERO whenever the snapshot reports wallet AERO, because the residual then has it
 * subtracted; an older snapshot keeps the wording it had.
 */
export function compositionResidualLabel(global: any, ethReported: boolean): string {
  const aero = field(global, 'wallet_aero_usd');
  const aeroReported = aero !== null && Number.isFinite(Number(aero));
  const base = `Total − NOCK − USDC${ethReported ? ' − ETH' : ''}${aeroReported ? ' − AERO' : ''}`;
  return ethReported ? base : `${base}, e.g. LP WETH`;
}
