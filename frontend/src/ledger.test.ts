import { describe, it, expect } from 'vitest';
import { ledgerView, ledgerLiteText, ledgerRatioPct, LEDGER_TITLE, LEDGER_BOOKKEEPING_NOTE } from './ledger';
import type { LedgerSummary } from './api/client';

const empty = (ratio?: string): LedgerSummary => ({
  cumulative_harvested_usd: '0.00',
  tax_reserve_usd: '0.00',
  growth_dry_powder_usd: '0.00',
  harvest_count: 0,
  last_harvest_at: null,
  ...(ratio !== undefined ? { tax_reserve_ratio: ratio, unpriced_count: 0 } : {}),
});

const banned = /PROTECTED|DEPLOYABLE|DRY POWDER|available|60\/40|never deployable/i;

const allText = (v: ReturnType<typeof ledgerView>) =>
  [v.title, v.summary, ...v.notes, ...v.dials.flatMap((d) => [d.label, d.sub])].join(' | ');

describe('harvest ledger view (#102)', () => {
  it('with no harvests, nothing is drawn full and it says so', () => {
    const v = ledgerView(empty('0.40'));
    expect(v.dials.map((d) => d.pct)).toEqual([0, 0]);
    expect(v.dials.map((d) => d.sub)).toEqual(['no harvests yet', 'no harvests yet']);
    expect(v.summary).toBe('No harvests recorded yet');
    expect(allText(v)).not.toMatch(banned);
  });

  it('labels come from the configured ratio, not a hardcoded 60/40', () => {
    const v = ledgerView({
      cumulative_harvested_usd: '10.00',
      tax_reserve_usd: '2.50',
      growth_dry_powder_usd: '7.50',
      harvest_count: 2,
      last_harvest_at: null,
      tax_reserve_ratio: '0.25',
      unpriced_count: 0,
    });
    expect(v.dials[0].label).toBe('25% VIRTUAL TAX RESERVE');
    expect(v.dials[1].label).toBe('75% UNRESERVED HARVESTS');
    expect(v.dials[0].pct).toBeCloseTo(25);
    expect(v.dials[1].pct).toBeCloseTo(75);
    expect(v.dials[0].sub).toBe('25% of recorded harvests');
    expect(v.summary).toBe('2 verified harvests, $10.00 recorded');
    expect(v.notes).toContain(LEDGER_BOOKKEEPING_NOTE);
    expect(allText(v)).not.toMatch(banned);
    expect(allText(v)).not.toMatch(/40%|60%/);
  });

  it('no ratio configured: no percentage is named for the reserve', () => {
    const v = ledgerView({ ...empty('0.00'), cumulative_harvested_usd: '4.00', growth_dry_powder_usd: '4.00', harvest_count: 1 });
    expect(v.dials[0].label).toBe('VIRTUAL TAX RESERVE (no ratio configured)');
    expect(v.dials[1].label).toBe('UNRESERVED HARVESTS');
    expect(v.dials[1].pct).toBeCloseTo(100);
    expect(v.notes[0]).toMatch(/No tax_reserve_ratio is configured/);
  });

  it('a pre-2.4.0 snapshot says the ratio is not reported instead of guessing', () => {
    const v = ledgerView(empty());
    expect(ledgerRatioPct(empty())).toBeNull();
    expect(v.dials[0].label).toBe('VIRTUAL TAX RESERVE');
    expect(v.notes[0]).toMatch(/not reported \(snapshot predates 2\.4\.0\)/);
  });

  it('unpriced rows are named, never hidden', () => {
    const v = ledgerView({ ...empty('0.40'), harvest_count: 1, unpriced_count: 2 });
    expect(v.notes.join(' ')).toMatch(/2 harvest rows have no USD price at their block and are left out of the total/);
  });

  it('missing ledger is unavailable, not zero', () => {
    const v = ledgerView(undefined);
    expect(v.title).toBe(LEDGER_TITLE);
    expect(v.dials).toEqual([]);
    expect(v.summary).toBe('Harvest ledger unavailable');
  });

  it('Lite cockpit line follows the same rules', () => {
    expect(ledgerLiteText(undefined)).toBe('ledger: unavailable');
    expect(ledgerLiteText(empty('0.40'))).toBe('ledger: no harvests recorded yet');
    const t = ledgerLiteText({ ...empty('0.40'), cumulative_harvested_usd: '5.00', tax_reserve_usd: '2.00', growth_dry_powder_usd: '3.00', harvest_count: 1 });
    expect(t).toBe('ledger: 1 verified harvest · 40% virtual tax reserve $2.00 · 60% unreserved harvests $3.00');
    expect(t).not.toMatch(/dry powder|protected/i);
  });
});
