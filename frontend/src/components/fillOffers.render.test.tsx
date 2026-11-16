/**
 * Static renders of the accumulation plan panel's fill detection (v0.6.0, issue #118).
 * The frontend tests have no DOM, so these check the markup the components produce.
 */
import { describe, it, expect } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import type { AccumulationEvent } from '../api/client';
import type { AccumulationPlanStatus, FillDetection, FillOffer } from '../types/snapshot';
import { AccumulationPlanPanel } from './AccumulationPlanPanel';
import { FillOffers } from './FillOffers';
import { PlanFillsTable } from './PlanFills';

const BUY_2 = '0x4644edd99bb2770f369862357eb5647a68a7157ca9e91ecccfce79e6964c61e8';
const BUY_3 = '0x78376f3cc9e80f2eb124a945a452df40556649e3d9983be526b2bd97586e31b9';

const offer3: FillOffer = {
  tx_hash: BUY_3,
  confirmed_at: '2026-11-16T03:08:37+00:00',
  block_number: 58040708,
  usdc_spent: '20',
  nock_received: '1780.3592976960049083',
  effective_price_usd: '0.01123369',
  timing: 'on_schedule',
  reference_spot_usd: '0.01120000',
  reference_spot_source: 'market_price_history uniswap_rpc at 2026-11-16T03:05:00+00:00',
  execution_cost_bps: 30,
};

const offer2: FillOffer = {
  tx_hash: BUY_2,
  confirmed_at: '2026-11-15T02:58:46+00:00',
  block_number: 57605753,
  usdc_spent: '20',
  nock_received: '1720.2181466216939446',
  effective_price_usd: '0.01162643',
  timing: 'early',
  reference_spot_usd: null,
  reference_spot_source: null,
  execution_cost_bps: null,
};

const detection = (over: Partial<FillDetection> = {}): FillDetection => ({
  status: 'ok',
  detail: null,
  checked_back_to: '2026-11-14T00:00:00+00:00',
  last_checked_at: '2026-11-16T04:00:00+00:00',
  offers: [],
  dismissed: [],
  ...over,
});

const plan = (over: Partial<AccumulationPlanStatus> = {}): AccumulationPlanStatus => ({
  plan_id: '2026-11-reentry',
  status: 'tranche_due',
  venue: 'Aero',
  tranche_index: 3,
  tranches_total: 5,
  tranche_usdc: '20.00',
  nominal_tranche_usdc: '20.00',
  next_due_at: null,
  budget_usdc: '100.00',
  spent_usdc: '40.00',
  remaining_usdc: '60.00',
  acquired_nock: '3318.1522',
  fills: 2,
  last_fill_at: '2026-11-15T02:58:46+00:00',
  avg_price_usd: '0.012055',
  lump_sum_nock: '7989.6702',
  binding_constraint: null,
  cash_reserve_usdc: '950.00',
  fill_detection: detection(),
  ...over,
});

const HASH_FIELD = 'aria-label="Transaction hash of a NOCK purchase"';
const count = (html: string, needle: string) => html.split(needle).length - 1;

describe('offer card', () => {
  it('shows the full hash, the amounts, the price, the cost and both actions', () => {
    const html = renderToStaticMarkup(<FillOffers detection={detection({ offers: [offer2, offer3] })} />);
    expect(count(html, 'data-testid="fill-offer"')).toBe(2);
    expect(html).toContain('DETECTED SWAP · NOT YET COUNTED');
    expect(html).toContain(BUY_3);
    expect(html).toContain(`https://basescan.org/tx/${BUY_3}`);
    expect(html).toContain('$20.00 USDC → 1,780.36 NOCK at $0.011234 per NOCK');
    expect(html).toContain('2026-11-16 03:08 UTC · block 58040708');
    expect(html).toContain('Execution cost: +30 bps (0.30%) over spot $0.011200');
    expect(html).toContain('market_price_history uniswap_rpc at 2026-11-16T03:05:00+00:00');
    // The reference's time is in the card's text, not only in a hover title.
    expect(html.replace(/<[^>]+>/g, '')).toContain(
      'Execution cost: +30 bps (0.30%) over spot $0.011200 · stored price from 2026-11-16 03:05 UTC, 4 min before the swap',
    );
    expect(count(html, 'data-testid="cost-caveat"')).toBe(1);
    expect(html).toContain('A price move between that stored price and the swap reads as cost');
    // The second offer has no reference price: unavailable, never a zero.
    expect(html).toContain('Execution cost: cost over spot unavailable');
    expect(html).toContain('early: less than one plan interval after the buy before it');
    expect(count(html, '>CONFIRM<')).toBe(2);
    expect(count(html, '>NOT PART OF THE PLAN<')).toBe(2);
    expect(html).toContain('2 swaps found, not counted yet');
    expect(html).not.toContain('RESTORE');
  });

  it('keeps dismissed swaps listed with a restore action', () => {
    const html = renderToStaticMarkup(<FillOffers detection={detection({ offers: [offer3], dismissed: [offer2] })} />);
    expect(count(html, 'data-testid="fill-offer"')).toBe(1);
    expect(count(html, 'data-testid="fill-dismissed"')).toBe(1);
    expect(html).toContain('NOT COUNTED · MARKED “NOT PART OF THE PLAN”');
    expect(html).toContain(BUY_2);
    expect(count(html, '>RESTORE<')).toBe(1);
    expect(count(html, '>CONFIRM<')).toBe(1);
  });

  it('shows the cost caveat only when a cost figure is shown', () => {
    const none = renderToStaticMarkup(<FillOffers detection={detection({ offers: [offer2] })} />);
    expect(none).not.toContain('data-testid="cost-caveat"');
    const dismissedOnly = renderToStaticMarkup(<FillOffers detection={detection({ dismissed: [offer3] })} />);
    expect(count(dismissedOnly, 'data-testid="cost-caveat"')).toBe(1);
  });

  it('says "could not check" for unavailable, and never "none found"', () => {
    const html = renderToStaticMarkup(
      <FillOffers
        detection={detection({
          status: 'unavailable',
          detail: 'the wallet&#x27;s transfer history could not be read',
          last_checked_at: null,
          checked_back_to: null,
        })}
      />,
    );
    expect(html).toContain('COULD NOT CHECK');
    expect(html).toContain('Last successful check: none on record');
    expect(html).not.toContain('no uncounted swap found');
    expect(html).not.toContain('data-testid="fill-offer"');

    const none = renderToStaticMarkup(<FillOffers detection={detection()} />);
    expect(none).toContain('no uncounted swap found');
    const old = renderToStaticMarkup(<FillOffers detection={null} />);
    expect(old).toContain('SWAP DETECTION NOT REPORTED');
    expect(old).not.toContain('no uncounted swap found');
  });

  it('still offers swaps found earlier when this cycle could not check', () => {
    const html = renderToStaticMarkup(
      <FillOffers detection={detection({ status: 'unavailable', detail: 'no answer', offers: [offer3] })} />,
    );
    expect(html).toContain('COULD NOT CHECK');
    expect(count(html, '>CONFIRM<')).toBe(1);
  });
});

describe('accumulation plan panel', () => {
  it('keeps the manual hash field in every state that had it, with the offer above it', () => {
    for (const status of ['tranche_due', 'waiting', 'pending', 'paused', 'blocked', 'complete'] as const) {
      const html = renderToStaticMarkup(
        <AccumulationPlanPanel plan={plan({ status, fill_detection: detection({ offers: [offer3] }) })} />,
      );
      expect(html, status).toContain(HASH_FIELD);
      expect(html, status).toContain('RECORD BUY');
      expect(html, status).toContain('RECORD A BUY BY TRANSACTION HASH · WORKS WITHOUT SWAP DETECTION');
      // "Always" would be untrue: recording still needs a plan, an RPC key and a receipt.
      expect(html.toUpperCase(), status).not.toContain('ALWAYS AVAILABLE');
      expect(html, status).toContain(BUY_3);
      expect(html.indexOf('data-testid="fill-offer"'), status).toBeLessThan(html.indexOf(HASH_FIELD));
    }
    // An invalid plan has nothing to record against: no field, no offers, as before.
    const invalid = renderToStaticMarkup(
      <AccumulationPlanPanel plan={plan({ status: 'invalid', binding_constraint: 'accumulation_plan needs a non-empty id' })} />,
    );
    expect(invalid).not.toContain(HASH_FIELD);
    expect(invalid).not.toContain('DETECTED SWAP');
    // No plan at all: the panel explains how to add one.
    expect(renderToStaticMarkup(<AccumulationPlanPanel plan={null} />)).toContain('No accumulation plan is configured');
  });

  it('keeps the hash field when detection could not run or is not reported', () => {
    const down = renderToStaticMarkup(
      <AccumulationPlanPanel plan={plan({ fill_detection: detection({ status: 'unavailable', detail: 'no answer' }) })} />,
    );
    expect(down).toContain('COULD NOT CHECK');
    expect(down).toContain(HASH_FIELD);
    const old = renderToStaticMarkup(<AccumulationPlanPanel plan={plan({ fill_detection: undefined })} />);
    expect(old).toContain('SWAP DETECTION NOT REPORTED');
    expect(old).toContain(HASH_FIELD);
  });

  // Review finding on #118: the hint promised "NockNock then looks for the swap" in
  // every state, and never said to answer an open offer before buying again.
  it('words the tranche-due hint by the detection state', () => {
    const text = (p: AccumulationPlanStatus) => renderToStaticMarkup(<AccumulationPlanPanel plan={p} />).replace(/<[^>]+>/g, '');
    const BUY = 'Buy $20.00 of NOCK (tranche 3 of 5). ';
    const PROMISE = 'NockNock then looks for the swap on-chain and offers it here to confirm';

    expect(text(plan())).toContain(`${BUY}${PROMISE}; you can also record its transaction hash below.`);

    const down = text(plan({ fill_detection: detection({ status: 'unavailable', detail: 'no Alchemy RPC key is configured' }) }));
    expect(down).toContain(`${BUY}Swap detection could not check this cycle; after buying, record the transaction hash below.`);
    expect(down).not.toContain(PROMISE);

    const off = text(plan({ fill_detection: detection({ status: 'not_run', detail: 'swap detection is turned off' }) }));
    expect(off).toContain(`${BUY}Then record the transaction hash below.`);
    expect(off).not.toContain(PROMISE);

    const old = text(plan({ fill_detection: undefined }));
    expect(old).toContain(`${BUY}Then record the transaction hash below.`);
    expect(old).not.toContain(PROMISE);

    // An open offer comes first: answer it before buying the same tranche again.
    const html = renderToStaticMarkup(
      <AccumulationPlanPanel plan={plan({ fill_detection: detection({ offers: [offer3] }) })} />,
    );
    const open = html.replace(/<[^>]+>/g, '');
    expect(count(html, 'data-testid="offer-first"')).toBe(1);
    expect(open).toContain(
      `A detected swap below is not counted yet: confirm it or mark it as not part of the plan before buying again. ${BUY}`,
    );
    expect(renderToStaticMarkup(<AccumulationPlanPanel plan={plan()} />)).not.toContain('data-testid="offer-first"');
    // Only a due tranche carries the buy hint; a waiting plan does not tell anyone to buy.
    const waiting = renderToStaticMarkup(
      <AccumulationPlanPanel plan={plan({ status: 'waiting', fill_detection: detection({ offers: [offer3] }) })} />,
    );
    expect(waiting).not.toContain('data-testid="offer-first"');
    expect(waiting).not.toContain('before buying again');
  });

  it('does not count an open offer as progress', () => {
    const html = renderToStaticMarkup(
      <AccumulationPlanPanel plan={plan({ fill_detection: detection({ offers: [offer3] }) })} />,
    );
    expect(html).toContain('2 verified buys');
    expect(html).toContain('$40.00');
    expect(html).toContain('offered, not counted');
  });
});

describe('per-fill table', () => {
  const events: AccumulationEvent[] = [
    {
      plan_id: '2026-11-reentry',
      tx_hash: '0x5d0c02d9929f49f71526e0c8d91d1bd99ec19f116befc285d67f6bf9412a3f0e',
      venue: 'Aero',
      usdc_spent: '20',
      nock_received: '1597.9340475636968138',
      effective_price_usd: '0.01251616',
      block_number: 57228848,
      confirmed_at: '2026-11-14T06:02:25+00:00',
      reference_spot_usd: '0.01229000',
      reference_spot_source: 'market_price_history uniswap_rpc at 2026-11-14T06:00:00+00:00',
      execution_cost_bps: 184,
    },
    {
      plan_id: '2026-11-reentry',
      tx_hash: BUY_2,
      venue: 'Aero',
      usdc_spent: '20',
      nock_received: '1720.2181466216939446',
      effective_price_usd: '0.01162643',
      block_number: 57605753,
      confirmed_at: '2026-11-15T02:58:46+00:00',
      reference_spot_usd: null,
      reference_spot_source: null,
      execution_cost_bps: null,
    },
    { plan_id: 'another-plan', tx_hash: '0xfeed', venue: 'Aero', usdc_spent: '5', nock_received: '1', effective_price_usd: '5', block_number: 1, confirmed_at: '2026-01-01T00:00:00+00:00' },
  ];

  it('lists each fill of the plan with its cost over spot, or unavailable', () => {
    const html = renderToStaticMarkup(<PlanFillsTable planId="2026-11-reentry" events={events} />);
    expect(count(html, 'data-testid="plan-fill"')).toBe(2);
    expect(html).toContain('2026-11-14 06:02 UTC');
    expect(html).toContain('$0.012516');
    expect(html).toContain('+184 bps (1.84%) over spot $0.012290');
    // Each figure names its reference time in the cell; the footnote needs no hover.
    expect(html).toContain('stored price from 2026-11-14 06:00 UTC, 2 min before the swap');
    expect(html).toContain('A price move between that stored price and the swap reads as cost');
    expect(html).not.toContain('hover');
    expect(html).toContain('>unavailable<');
    expect(html).toContain('0x5d0c02d9…2a3f0e');
    expect(html).not.toContain('0xfeed');
  });

  it('says so when there are no fills, when loading, and when the list cannot be read', () => {
    expect(renderToStaticMarkup(<PlanFillsTable planId="p" events={[]} />)).toContain('No buys recorded for this plan yet.');
    expect(renderToStaticMarkup(<PlanFillsTable planId="p" events={null} />)).toContain('Loading recorded buys');
    const failed = renderToStaticMarkup(<PlanFillsTable planId="p" events={null} error="HTTP 500" />);
    expect(failed).toContain('Recorded buys could not be listed: HTTP 500');
    expect(failed).not.toContain('No buys recorded');
  });
});
