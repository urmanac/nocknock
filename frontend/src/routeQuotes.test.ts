/**
 * Route quote display helpers (v0.7.0, issue #115). The quotes are the real comparison
 * read on Base at block 58081910 (see routeQuotes.fixture.ts).
 */
import { describe, it, expect } from 'vitest';
import type { RouteQuotes, VenueQuote } from './types/snapshot';
import { REAL_ROUTE_QUOTES } from './routeQuotes.fixture';
import {
  COST_OVER_QUOTE_CAVEAT,
  ROUTE_QUOTE_CAVEAT,
  costOverQuote,
  costOverSpotText,
  hopLine,
  newerQuotes,
  nockAmount,
  quoteAge,
  quoteBlockLine,
  swapLinkLabel,
  swapPageUrl,
  venueCard,
} from './routeQuotes';

const clone = (): RouteQuotes => JSON.parse(JSON.stringify(REAL_ROUTE_QUOTES)) as RouteQuotes;
const [aero, uniswap] = REAL_ROUTE_QUOTES.venues;

describe('swap links', () => {
  it('go to each venue\'s own swap page and prefill nothing', () => {
    expect(swapPageUrl('Aero', aero.swap_url)).toBe('https://app.aero.xyz/swap');
    expect(swapPageUrl('Uniswap', uniswap.swap_url)).toBe('https://app.uniswap.org/swap');
    for (const v of REAL_ROUTE_QUOTES.venues) {
      const url = new URL(swapPageUrl(v.venue, v.swap_url) as string);
      expect(['app.aero.xyz', 'app.uniswap.org']).toContain(url.hostname);
      expect(url.search).toBe('');
      expect(url.hash).toBe('');
    }
  });

  it('never follow a link to another host, and never to the retired Aerodrome app', () => {
    expect(swapPageUrl('Aero', 'https://aerodrome.finance/swap')).toBe('https://app.aero.xyz/swap');
    expect(swapPageUrl('Aero', 'https://app.aero.xyz.evil.example/swap')).toBe('https://app.aero.xyz/swap');
    expect(swapPageUrl('Uniswap', 'http://app.uniswap.org/swap')).toBe('https://app.uniswap.org/swap');
    expect(swapPageUrl('Uniswap', 'javascript:alert(1)')).toBe('https://app.uniswap.org/swap');
    expect(swapPageUrl('Aero', 'not a url')).toBe('https://app.aero.xyz/swap');
    expect(swapPageUrl('Aero', null)).toBe('https://app.aero.xyz/swap');
    // A venue NockNock does not know gets no link rather than a guessed one.
    expect(swapPageUrl('Elsewhere', 'https://swap.example/')).toBeNull();
    expect(swapPageUrl('Elsewhere', undefined)).toBeNull();
  });

  it('are labelled as swap pages, not deposits', () => {
    expect(swapLinkLabel('Aero')).toBe('OPEN AERO SWAP PAGE');
    expect(swapLinkLabel('Uniswap')).toBe('OPEN UNISWAP SWAP PAGE');
    expect(swapLinkLabel('Aero').toLowerCase()).not.toContain('deposit');
  });
});

describe('quote block and age', () => {
  it('names the block and its time', () => {
    expect(quoteBlockLine(REAL_ROUTE_QUOTES)).toBe('block 58,081,910 · 2026-11-16 05:25:57 UTC');
    expect(quoteBlockLine({ block_number: 58081910, quoted_at: null })).toBe('block 58,081,910');
    expect(quoteBlockLine({ block_number: null, quoted_at: null })).toBe('no block was read');
  });

  it('says how old the quotes are, never a negative age', () => {
    const at = Date.parse('2026-11-16T05:25:57Z');
    expect(quoteAge(REAL_ROUTE_QUOTES.quoted_at, at + 2_000)).toBe('read just now');
    expect(quoteAge(REAL_ROUTE_QUOTES.quoted_at, at + 40_000)).toBe('read 40 s ago');
    expect(quoteAge(REAL_ROUTE_QUOTES.quoted_at, at + 4 * 60_000)).toBe('read 4 min ago');
    expect(quoteAge(REAL_ROUTE_QUOTES.quoted_at, at + 5 * 3600_000)).toBe('read 5 h ago');
    expect(quoteAge(REAL_ROUTE_QUOTES.quoted_at, at - 30_000)).toBe('read just now');
    expect(quoteAge(null, at)).toBeNull();
    expect(quoteAge('not a time', at)).toBeNull();
  });
});

describe('venue card', () => {
  it('shows NOCK out, price, cost over spot, the route and its hops', () => {
    const card = venueCard(aero, REAL_ROUTE_QUOTES);
    expect(card.quoted).toBe(true);
    expect(card.returnsMore).toBe(true);
    expect(card.headline).toBe('1,773.0769 NOCK');
    expect(card.exactNock).toBe('1773.0768618618146014');
    expect(card.detail).toBe('$0.011280 per NOCK · 0.06% under the reference spot $0.011287');
    expect(card.route).toBe('USDC → USDT → WETH → NOCK');
    expect(card.hops).toEqual([
      'USDC → USDT · Aero concentrated · fee 0.0008% · 0xd4489908…aaf7',
      'USDT → WETH · Aero concentrated · fee 0.0468% · 0x2dbe0e26…cf7b',
      'WETH → NOCK · Aero concentrated · fee 0.97% · 0x957256c9…0874',
    ]);
    // The venue's other candidates are listed with what each returned.
    expect(card.otherRoutes).toEqual([
      'USDC → NOCK: 1,742.2508 NOCK',
      'USDC → WETH → NOCK: 1,772.5260 NOCK',
      'USDC → WETH → NOCK: 1,772.2004 NOCK',
    ]);
    expect(card.swapUrl).toBe('https://app.aero.xyz/swap');

    const other = venueCard(uniswap, REAL_ROUTE_QUOTES);
    expect(other.returnsMore).toBe(false);
    expect(other.headline).toBe('1,772.2085 NOCK');
    expect(other.detail).toBe('$0.011285 per NOCK · 0.01% under the reference spot $0.011287');
    expect(other.hops).toEqual([
      'USDC → WETH · Uniswap v3 · fee 0.05% · 0xd0b53d92…f224',
      'WETH → NOCK · Uniswap v3 · fee 1% · 0x71a4a6f6…ad9a',
    ]);
    expect(other.otherRoutes).toEqual(['USDC → NOCK: 1,716.8661 NOCK', 'USDC → ETH → NOCK: 1,692.7635 NOCK']);
  });

  it('shows only the reason for a venue that could not be quoted', () => {
    const down: VenueQuote = {
      venue: 'Uniswap',
      status: 'unavailable',
      reason: 'none of its 3 verified route(s) could be quoted: the quote request timed out',
      swap_url: 'https://app.uniswap.org/swap',
    };
    const card = venueCard(down, { more_nock_venue: null, reference_spot_usd: null });
    expect(card.quoted).toBe(false);
    expect(card.headline).toBe('UNAVAILABLE');
    expect(card.detail).toBe('none of its 3 verified route(s) could be quoted: the quote request timed out');
    expect(card.exactNock).toBe('');
    expect(card.hops).toEqual([]);
    expect(card.returnsMore).toBe(false);
    // The link stays: the operator can still open the venue.
    expect(card.swapUrl).toBe('https://app.uniswap.org/swap');
    expect(JSON.stringify(card)).not.toMatch(/\d NOCK/);
    // A "quoted" entry without an amount is not drawn as a number either.
    expect(venueCard({ ...aero, nock_out: null }, REAL_ROUTE_QUOTES).headline).toBe('UNAVAILABLE');
    expect(venueCard({ ...down, reason: null }, REAL_ROUTE_QUOTES).detail).toBe('no reason given');
  });

  it('marks a venue only when the worker named it', () => {
    const q = clone();
    q.more_nock_venue = null;
    expect(q.venues.map((v) => venueCard(v, q).returnsMore)).toEqual([false, false]);
    q.more_nock_venue = 'Uniswap';
    expect(q.venues.map((v) => venueCard(v, q).returnsMore)).toEqual([false, true]);
    // An unavailable venue is never marked, whatever the field says.
    q.venues[1] = { venue: 'Uniswap', status: 'unavailable', reason: 'x', swap_url: 'https://app.uniswap.org/swap' };
    expect(venueCard(q.venues[1], q).returnsMore).toBe(false);
  });

  it('says a fee is not read rather than drawing a default', () => {
    expect(hopLine({ ...(aero.hops ?? [])[1], fee_pct: null })).toBe(
      'USDT → WETH · Aero concentrated · fee not read · 0x2dbe0e26…cf7b',
    );
    // A Uniswap v4 pool is named by its 32-byte pool id.
    expect(
      hopLine({
        pool: '0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13',
        pool_kind: 'Uniswap v4',
        token_in: 'USDC',
        token_out: 'NOCK',
        fee_pct: '3',
      }),
    ).toBe('USDC → NOCK · Uniswap v4 · fee 3% · 0x064a5df2…6b13');
  });
});

describe('cost over the reference spot', () => {
  it('is signed in words, and unavailable without a reference', () => {
    expect(costOverSpotText('3.21', '0.01128675')).toBe('3.21% over the reference spot $0.011287');
    expect(costOverSpotText('-0.06', '0.01128675')).toBe('0.06% under the reference spot $0.011287');
    expect(costOverSpotText('0.00', '0.01128675')).toBe('at the reference spot $0.011287');
    expect(costOverSpotText('1.80', null)).toBe('1.80% over the reference spot');
    expect(costOverSpotText(null, '0.01128675')).toBe('cost over spot unavailable (no reference spot)');
    expect(costOverSpotText('', '0.01128675')).toBe('cost over spot unavailable (no reference spot)');
    expect(costOverSpotText('n/a', null)).toBe('cost over spot unavailable (no reference spot)');
  });

  it('formats NOCK for reading and leaves what it cannot parse alone', () => {
    expect(nockAmount('1773.0768618618146014')).toBe('1,773.0769');
    expect(nockAmount('5')).toBe('5.0000');
    expect(nockAmount(null)).toBe('n/a');
    expect(nockAmount('')).toBe('n/a');
    expect(nockAmount('lots')).toBe('lots');
  });
});

describe('refreshed quotes', () => {
  it('replace the snapshot\'s only when read at a later block for the same amount', () => {
    const later = clone();
    later.block_number = 58083410;
    expect(newerQuotes(REAL_ROUTE_QUOTES, later)).toBe(later);
    expect(newerQuotes(later, REAL_ROUTE_QUOTES)).toBe(later);
    expect(newerQuotes(REAL_ROUTE_QUOTES, null)).toBe(REAL_ROUTE_QUOTES);
    // A refresh for another tranche size is not shown against this one.
    const otherSize = clone();
    otherSize.block_number = 58090000;
    otherSize.amount_usdc = '13.37';
    expect(newerQuotes(REAL_ROUTE_QUOTES, otherSize)).toBe(REAL_ROUTE_QUOTES);
    // A failed refresh (no block) never replaces quotes that were read.
    const failed = clone();
    failed.block_number = null;
    expect(newerQuotes(REAL_ROUTE_QUOTES, failed)).toBe(REAL_ROUTE_QUOTES);
    // No quotes in the snapshot (no tranche due): nothing is shown, refreshed or not.
    expect(newerQuotes(null, later)).toBeNull();
    expect(newerQuotes(undefined, undefined)).toBeNull();
  });
});

describe('cost over quote, per fill', () => {
  const fill = {
    quote_venue: 'Aero',
    quote_usdc: '20.00',
    quote_nock_out: '1773.0768618618146014',
    quote_effective_price_usd: '0.01127983',
    quote_block_number: 58081910,
    quoted_at: '2026-11-16T05:25:57+00:00',
    quote_cost_bps: 50,
  };

  it('names the quote it is measured against, with its block', () => {
    expect(costOverQuote(fill)).toEqual({
      known: true,
      text: '+50 bps (0.50%) over the quote',
      reference: 'best quote: Aero, $20.00 for 1,773.0769 NOCK at block 58,081,910 (2026-11-16 05:25:57 UTC)',
    });
    expect(costOverQuote({ ...fill, quote_cost_bps: -12 }).text).toBe('−12 bps (0.12%) under the quote');
    expect(costOverQuote({ ...fill, quote_cost_bps: 0 }).text).toBe('at the quote');
    expect(costOverQuote({ ...fill, quote_block_number: null, quoted_at: null }).reference).toBe(
      'best quote: Aero, $20.00 for 1,773.0769 NOCK block not recorded',
    );
  });

  it('is "no quote on record" without a stored quote, never a zero', () => {
    const none = { known: false, text: 'no quote on record', reference: '' };
    expect(costOverQuote({})).toEqual(none);
    expect(costOverQuote({ ...fill, quote_cost_bps: null })).toEqual(none);
    expect(costOverQuote({ ...fill, quote_effective_price_usd: null })).toEqual(none);
    expect(costOverQuote({ ...fill, quote_venue: null })).toEqual(none);
    // A fill recorded before v0.7.0 has none of the fields.
    expect(costOverQuote({ quote_cost_bps: undefined })).toEqual(none);
  });
});

describe('wording', () => {
  it('states what a quote is and is not', () => {
    expect(ROUTE_QUOTE_CAVEAT).toContain('estimates as of that time');
    expect(ROUTE_QUOTE_CAVEAT).toContain("The wallet app's own router may pick a different route.");
    expect(ROUTE_QUOTE_CAVEAT).toContain('it is not a recommendation to buy');
    expect(COST_OVER_QUOTE_CAVEAT).toContain('not execution cost alone');
  });

  it('the worker\'s comparison is a statement of fact', () => {
    expect(REAL_ROUTE_QUOTES.comparison).toBe(
      'Aero returns more NOCK by 0.05% for $20.00 USDC at block 58081910: 1773.0768 NOCK against 1772.2085 on Uniswap.',
    );
    expect(REAL_ROUTE_QUOTES.comparison.toLowerCase()).not.toMatch(/recommend|should|cheaper|best/);
  });
});
