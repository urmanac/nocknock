/**
 * Static renders of the accumulation plan panel for the v0.8.1 burn-down. The frontend
 * tests have no DOM, so these check the markup the panel produces.
 */
import { describe, it, expect } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import type { AccumulationPlanStatus } from '../types/snapshot';
import { AccumulationPlanPanel } from './AccumulationPlanPanel';
import { planProgressPct } from '../accumulation';

const text = (html: string) => html.replace(/<[^>]+>/g, '').replace(/&#x27;/g, "'").replace(/&amp;/g, '&');

const plan = (over: Partial<AccumulationPlanStatus> = {}): AccumulationPlanStatus => ({
  plan_id: '2026-11-reentry',
  status: 'waiting',
  venue: 'Aero',
  tranche_index: 4,
  tranches_total: 5,
  tranche_usdc: '0.00',
  nominal_tranche_usdc: '20.00',
  next_due_at: '2026-11-17T03:08:37+00:00',
  budget_usdc: '100.00',
  spent_usdc: '60.00',
  remaining_usdc: '40.00',
  acquired_nock: '5098.5114',
  fills: 3,
  last_fill_at: '2026-11-16T03:08:37+00:00',
  avg_price_usd: '0.011768',
  lump_sum_nock: '7989.6702',
  binding_constraint: null,
  cash_reserve_usdc: null,
  fill_detection: null,
  route_quotes: null,
  ...over,
});

/** What a worker that could not read the ledger publishes (issue #130). */
const unread = plan({
  status: 'blocked',
  tranche_index: 1,
  spent_usdc: 'unknown',
  remaining_usdc: 'unknown',
  acquired_nock: 'unknown',
  fills: 0,
  last_fill_at: null,
  avg_price_usd: null,
  lump_sum_nock: null,
  next_due_at: null,
  binding_constraint: 'the accumulation ledger could not be read this cycle, so plan progress is unknown',
  progress_unknown: true,
});

describe('#130: a cycle that could not read the plan ledger', () => {
  it('shows the progress as unknown, never $0.00 or 0 buys', () => {
    const t = text(renderToStaticMarkup(<AccumulationPlanPanel plan={unread} />));
    expect(t).toContain('unknown of $100.00');
    expect(t).toContain('Verified buys and the remainder are unknown');
    expect(t).toContain('plan progress is unknown');
    expect(t).not.toContain('$0.00');
    expect(t).not.toContain('0 verified buys');
    expect(t).not.toContain('$unknown');
    expect(t).not.toContain('No verified buys yet');
  });

  it('draws no progress bar for unknown progress', () => {
    expect(planProgressPct(unread)).toBeNull();
    expect(planProgressPct(plan())).toBe(60);
    const html = renderToStaticMarkup(<AccumulationPlanPanel plan={unread} />);
    expect(html).not.toContain('width:0%');
  });

  it('leaves a plan whose ledger was read unchanged', () => {
    const t = text(renderToStaticMarkup(<AccumulationPlanPanel plan={plan()} />));
    expect(t).toContain('$60.00 of $100.00');
    expect(t).toContain('3 verified buys · $40.00 remaining');
  });
});

describe('#132: the plan venue is the operator\'s label or nothing', () => {
  it('shows no venue when none is configured', () => {
    for (const venue of [null, undefined]) {
      const t = text(renderToStaticMarkup(<AccumulationPlanPanel plan={plan({ venue })} />));
      expect(t).toContain('PLAN “2026-11-reentry”');
      expect(t).not.toContain('AERO');
      expect(t).not.toContain('VENUE');
    }
  });

  it('shows a configured venue as the operator\'s label', () => {
    const t = text(renderToStaticMarkup(<AccumulationPlanPanel plan={plan({ venue: 'Aero' })} />));
    expect(t).toContain('PLAN “2026-11-reentry” · YOUR VENUE LABEL: AERO');
  });
});
