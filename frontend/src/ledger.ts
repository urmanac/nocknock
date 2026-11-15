/**
 * Pure display helpers for the harvest ledger panel (issue #102, snapshot 2.4.0).
 *
 * The split is bookkeeping inside one wallet: NockNock never moves or locks anything, so
 * no label may say "protected", "deployable" or "available". Every percentage comes from
 * the configured `tax_reserve_ratio` and the recorded harvests, never from a hardcoded
 * 60/40, and with no harvests nothing is drawn as full.
 */
import type { LedgerSummary } from './api/client';

export interface LedgerDial {
  label: string;
  usd: string;
  /** Bar fill, 0–100: this side's share of the recorded harvest total (0 with none). */
  pct: number;
  sub: string;
}

export interface LedgerView {
  title: string;
  dials: LedgerDial[];
  /** One line on what the ledger holds, e.g. "No harvests recorded yet". */
  summary: string;
  notes: string[];
}

export const LEDGER_TITLE = 'Harvest Ledger (Virtual Tax Reserve)';
export const LEDGER_BOOKKEEPING_NOTE =
  'Bookkeeping inside one wallet: nothing is set aside on-chain, and NockNock does not stop you spending it.';

const num = (s: string | undefined | null): number | null => {
  if (s == null || s === '') return null;
  const v = Number(s);
  return Number.isFinite(v) ? v : null;
};

/** The configured ratio as a whole percentage, or null when the snapshot does not report it. */
export function ledgerRatioPct(ledger: LedgerSummary): number | null {
  const r = num(ledger.tax_reserve_ratio);
  return r == null ? null : Math.round(r * 100);
}

export function ledgerView(ledger: LedgerSummary | null | undefined): LedgerView {
  if (!ledger) {
    return { title: LEDGER_TITLE, dials: [], summary: 'Harvest ledger unavailable', notes: [] };
  }
  const total = num(ledger.cumulative_harvested_usd) ?? 0;
  const tax = num(ledger.tax_reserve_usd) ?? 0;
  const rest = num(ledger.growth_dry_powder_usd) ?? 0;
  const count = ledger.harvest_count ?? 0;
  const pct = ledgerRatioPct(ledger);
  const share = (v: number) => (total > 0 ? (v / total) * 100 : 0);
  const sub = (v: number) => (count > 0 && total > 0 ? `${share(v).toFixed(0)}% of recorded harvests` : 'no harvests yet');

  const reserveLabel =
    pct == null ? 'VIRTUAL TAX RESERVE' : pct > 0 ? `${pct}% VIRTUAL TAX RESERVE` : 'VIRTUAL TAX RESERVE (no ratio configured)';
  const restLabel = pct != null && pct > 0 ? `${100 - pct}% UNRESERVED HARVESTS` : 'UNRESERVED HARVESTS';

  const notes: string[] = [];
  if (pct == null) {
    notes.push('Tax reserve ratio not reported (snapshot predates 2.4.0).');
  } else if (pct === 0) {
    notes.push('No tax_reserve_ratio is configured in operator_profile, so no harvest is booked to a reserve.');
  } else {
    notes.push(`operator_profile.tax_reserve_ratio books ${pct}% of each recorded harvest to the virtual tax reserve.`);
  }
  const unpriced = ledger.unpriced_count ?? 0;
  if (unpriced > 0) {
    notes.push(
      `${unpriced} harvest row${unpriced === 1 ? ' has' : 's have'} no USD price at ${unpriced === 1 ? 'its' : 'their'} block and ${unpriced === 1 ? 'is' : 'are'} left out of the total.`,
    );
  }
  notes.push(LEDGER_BOOKKEEPING_NOTE);

  const summary =
    count === 0
      ? 'No harvests recorded yet'
      : `${count} verified harvest${count === 1 ? '' : 's'}, $${total.toFixed(2)} recorded${
          ledger.last_harvest_at ? `, last ${new Date(ledger.last_harvest_at).toLocaleString()}` : ''
        }`;

  return {
    title: LEDGER_TITLE,
    dials: [
      { label: reserveLabel, usd: tax.toFixed(2), pct: share(tax), sub: sub(tax) },
      { label: restLabel, usd: rest.toFixed(2), pct: share(rest), sub: sub(rest) },
    ],
    summary,
    notes,
  };
}

/** One line for the Lite cockpit. */
export function ledgerLiteText(ledger: LedgerSummary | null | undefined): string {
  if (!ledger) return 'ledger: unavailable';
  if ((ledger.harvest_count ?? 0) === 0) return 'ledger: no harvests recorded yet';
  const v = ledgerView(ledger);
  const [reserve, rest] = v.dials;
  return `ledger: ${ledger.harvest_count} verified harvest${ledger.harvest_count === 1 ? '' : 's'} · ${reserve.label.toLowerCase()} $${reserve.usd} · ${rest.label.toLowerCase()} $${rest.usd}`;
}
