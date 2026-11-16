/**
 * Static renders of the route quotes in the accumulation plan panel (v0.7.0, issue
 * #115). The frontend tests have no DOM, so these check the markup the components
 * produce. The quotes are the real comparison read on Base at block 58081910.
 */
import { describe, it, expect } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import type { AccumulationEvent } from '../api/client';
import type { AccumulationPlanStatus, RouteQuotes as RouteQuotesData } from '../types/snapshot';
import { REAL_ROUTE_QUOTES } from '../routeQuotes.fixture';
import { AccumulationPlanPanel } from './AccumulationPlanPanel';
import { PlanFillsTable } from './PlanFills';
import { RouteQuotes, RouteQuotesView } from './RouteQuotes';

const NOW = Date.parse('2026-11-16T05:29:57Z');
const count = (html: string, needle: string) => html.split(needle).length - 1;
const text = (html: string) => html.replace(/<[^>]+>/g, '').replace(/&#x27;/g, "'").replace(/&amp;/g, '&');

const plan = (over: Partial<AccumulationPlanStatus> = {}): AccumulationPlanStatus => ({
  plan_id: '2026-11-reentry',
  status: 'tranche_due',
  venue: 'Aero',
  tranche_index: 4,
  tranches_total: 5,
  tranche_usdc: '20.00',
  nominal_tranche_usdc: '20.00',
  next_due_at: null,
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
  route_quotes: REAL_ROUTE_QUOTES,
  ...over,
});

const unavailable: RouteQuotesData = {
  status: 'unavailable',
  detail: 'no Alchemy RPC key is configured',
  amount_usdc: '20.00',
  block_number: null,
  quoted_at: null,
  reference_spot_usd: null,
  venues: [
    { venue: 'Aero', status: 'unavailable', reason: 'no Alchemy RPC key is configured', swap_url: 'https://app.aero.xyz/swap' },
    { venue: 'Uniswap', status: 'unavailable', reason: 'no Alchemy RPC key is configured', swap_url: 'https://app.uniswap.org/swap' },
  ],
  more_nock_venue: null,
  more_nock_by_pct: null,
  comparison: 'Neither venue could be quoted, so there is nothing to compare.',
};

describe('route quotes', () => {
  it('shows the two venues side by side with NOCK out, price, cost, route and block', () => {
    const html = renderToStaticMarkup(<RouteQuotesView quotes={REAL_ROUTE_QUOTES} nowMs={NOW} onRefresh={() => undefined} />);
    const plain = text(html);
    expect(count(html, 'data-testid="venue-quote"')).toBe(2);
    expect(plain).toContain('ROUTE QUOTES · $20.00 USDC → NOCK · READ-ONLY');
    expect(plain).toContain('block 58,081,910 · 2026-11-16 05:25:57 UTC · read 4 min ago');
    expect(plain).toContain('AERO');
    expect(plain).toContain('1,773.0768 NOCK');
    // The card and the worker's comparison sentence below it give the same figure.
    expect(plain).toContain('1773.0768 NOCK against 1772.2085 on Uniswap');
    expect(plain).not.toContain('1,773.0769');
    expect(plain).toContain('$0.011280 per NOCK · 0.06% under the reference spot $0.011287');
    expect(plain).toContain('USDC → USDT → WETH → NOCK');
    expect(plain).toContain('WETH → NOCK · Aero concentrated · fee 0.97% · 0x957256c9…0874');
    expect(plain).toContain('UNISWAP');
    expect(plain).toContain('1,772.2085 NOCK');
    expect(plain).toContain('WETH → NOCK · Uniswap v3 · fee 1% · 0x71a4a6f6…ad9a');
    // The exact amount is kept on the figure.
    expect(html).toContain('title="1773.0768618618146014"');
    expect(html).toContain('>REFRESH QUOTES<');
  });

  it('marks the venue that returns more NOCK, as the worker stated it', () => {
    const html = renderToStaticMarkup(<RouteQuotesView quotes={REAL_ROUTE_QUOTES} nowMs={NOW} />);
    expect(count(html, 'data-testid="returns-more"')).toBe(1);
    const aeroCard = html.split('data-testid="venue-quote"')[1];
    expect(aeroCard).toContain('RETURNS MORE NOCK');
    expect(text(html)).toContain(
      'Aero returns more NOCK by 0.05% for $20.00 USDC at block 58081910: 1773.0768 NOCK against 1772.2085 on Uniswap.',
    );
    // No refresh handler, no button.
    expect(html).not.toContain('REFRESH QUOTES');
  });

  it('links each venue\'s swap page, prefills nothing and never links Aerodrome', () => {
    const html = renderToStaticMarkup(<RouteQuotesView quotes={REAL_ROUTE_QUOTES} nowMs={NOW} />);
    expect(html).toContain('href="https://app.aero.xyz/swap"');
    expect(html).toContain('href="https://app.uniswap.org/swap"');
    expect(html).toContain('OPEN AERO SWAP PAGE');
    expect(html).toContain('OPEN UNISWAP SWAP PAGE');
    expect(count(text(html), 'Nothing is prefilled: enter $20.00 USDC → NOCK there yourself.')).toBe(2);
    const hrefs = [...html.matchAll(/href="([^"]+)"/g)].map((m) => m[1]);
    expect(hrefs).toHaveLength(2);
    for (const h of hrefs) {
      expect(['app.aero.xyz', 'app.uniswap.org']).toContain(new URL(h).hostname);
      expect(h).not.toContain('?');
    }
    expect(html).not.toContain('aerodrome.finance');
    expect(html.toLowerCase()).not.toContain('deposit');
  });

  it('says plainly what a quote is and that the wallet app may route differently', () => {
    const html = renderToStaticMarkup(<RouteQuotesView quotes={REAL_ROUTE_QUOTES} nowMs={NOW} />);
    expect(count(html, 'data-testid="quote-caveat"')).toBe(1);
    const plain = text(html);
    expect(plain).toContain('estimates as of that time, not a promise');
    expect(plain).toContain("The wallet app's own router may pick a different route.");
    expect(plain).toContain('it is not a recommendation to buy');
  });

  it('shows an unavailable venue with its reason and no number', () => {
    const html = renderToStaticMarkup(<RouteQuotesView quotes={unavailable} nowMs={NOW} />);
    const plain = text(html);
    expect(count(plain, 'UNAVAILABLE')).toBe(2);
    expect(count(plain, 'no Alchemy RPC key is configured')).toBe(2);
    expect(plain).toContain('no block was read');
    expect(plain).toContain('Neither venue could be quoted, so there is nothing to compare.');
    expect(plain).not.toMatch(/\d NOCK/);
    expect(html).not.toContain('data-testid="returns-more"');
    // The links stay, so the operator can still open either venue.
    expect(html).toContain('href="https://app.aero.xyz/swap"');
    expect(html).toContain('href="https://app.uniswap.org/swap"');
  });

  it('with one venue quoted shows it and marks neither', () => {
    const one: RouteQuotesData = {
      ...REAL_ROUTE_QUOTES,
      status: 'partial',
      venues: [REAL_ROUTE_QUOTES.venues[0], unavailable.venues[1]],
      more_nock_venue: null,
      more_nock_by_pct: null,
      comparison: 'Only Aero could be quoted at block 58081910; with one quote there is nothing to compare.',
    };
    const html = renderToStaticMarkup(<RouteQuotesView quotes={one} nowMs={NOW} />);
    expect(text(html)).toContain('1,773.0768 NOCK');
    expect(count(text(html), 'UNAVAILABLE')).toBe(1);
    expect(html).not.toContain('data-testid="returns-more"');
    expect(text(html)).toContain('Only Aero could be quoted at block 58081910');
  });

  it('keeps the quotes and their block when a refresh fails', () => {
    const html = renderToStaticMarkup(
      <RouteQuotesView quotes={REAL_ROUTE_QUOTES} nowMs={NOW} error="No tranche is due." onRefresh={() => undefined} />,
    );
    expect(text(html)).toContain('Could not refresh the quotes: No tranche is due.');
    expect(text(html)).toContain('block 58,081,910');
    const busy = renderToStaticMarkup(<RouteQuotesView quotes={REAL_ROUTE_QUOTES} nowMs={NOW} busy onRefresh={() => undefined} />);
    expect(busy).toContain('>QUOTING…<');
    expect(busy).toMatch(/<button[^>]*disabled/);
  });

  it('renders nothing without quotes', () => {
    expect(renderToStaticMarkup(<RouteQuotes quotes={null} />)).toBe('');
    expect(renderToStaticMarkup(<RouteQuotes />)).toBe('');
  });
});

describe('plan panel', () => {
  it('shows the quotes only while a tranche is due', () => {
    const due = renderToStaticMarkup(<AccumulationPlanPanel plan={plan()} />);
    expect(count(due, 'data-testid="route-quotes"')).toBe(1);
    expect(due.indexOf('data-testid="route-quotes"')).toBeLessThan(due.indexOf('RECORD A BUY BY TRANSACTION HASH'));

    // Due, but no quotes in the snapshot (quoting off, or an older worker): no block.
    const off = renderToStaticMarkup(<AccumulationPlanPanel plan={plan({ route_quotes: null })} />);
    expect(off).not.toContain('data-testid="route-quotes"');
    const older = plan();
    delete older.route_quotes;
    expect(renderToStaticMarkup(<AccumulationPlanPanel plan={older} />)).not.toContain('data-testid="route-quotes"');

    // Not due: stale quotes left in a payload are never drawn.
    for (const status of ['waiting', 'paused', 'blocked', 'pending', 'complete', 'invalid'] as const) {
      const html = renderToStaticMarkup(
        <AccumulationPlanPanel plan={plan({ status, tranche_usdc: '0.00', next_due_at: '2026-11-17T03:08:37+00:00' })} />,
      );
      expect(html, status).not.toContain('data-testid="route-quotes"');
    }
    expect(renderToStaticMarkup(<AccumulationPlanPanel plan={null} />)).not.toContain('data-testid="route-quotes"');
  });
});

describe('recorded buys', () => {
  const base: AccumulationEvent = {
    plan_id: '2026-11-reentry',
    tx_hash: '0x78376f3cc9e80f2eb124a945a452df40556649e3d9983be526b2bd97586e31b9',
    venue: 'Aero',
    usdc_spent: '20',
    nock_received: '1764.2',
    effective_price_usd: '0.01133623',
    block_number: 58081920,
    confirmed_at: '2026-11-16T05:25:59+00:00',
    reference_spot_usd: null,
    reference_spot_source: null,
    execution_cost_bps: null,
  };
  const quoted: AccumulationEvent = {
    ...base,
    quote_venue: 'Aero',
    quote_route: 'USDC → USDT → WETH → NOCK',
    quote_usdc: '20.00',
    quote_nock_out: '1773.0768618618146014',
    quote_effective_price_usd: '0.01127983',
    quote_block_number: 58081910,
    quoted_at: '2026-11-16T05:25:57+00:00',
    quote_cost_bps: 50,
  };
  const unquoted: AccumulationEvent = {
    ...base,
    tx_hash: '0x5d0c02d9929f49f71526e0c8d91d1bd99ec19f116befc285d67f6bf9412a3f0e',
    confirmed_at: '2026-11-14T06:02:25+00:00',
    block_number: 57228848,
  };

  it('show each fill\'s cost over its quote with the quote\'s block, or that none is on record', () => {
    const html = renderToStaticMarkup(<PlanFillsTable planId="2026-11-reentry" events={[unquoted, quoted]} />);
    expect(html).toContain('COST OVER QUOTE');
    const cells = html.split('data-testid="cost-over-quote"').slice(1).map((c) => text(c.slice(c.indexOf('>') + 1, c.indexOf('</td>'))));
    expect(cells).toEqual([
      'no quote on record',
      '+50 bps (0.50%) over the quotebest quote: Aero, $20.00 for 1,773.0768 NOCK at block 58,081,910 (2026-11-16 05:25:57 UTC)',
    ]);
    expect(text(html)).toContain('The market can move between the quote\'s block and the swap');
  });
});
