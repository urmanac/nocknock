/**
 * Fill detection display helpers and offer actions (v0.6.0, issue #118).
 * The offers below are the operator's real plan buys on Base (see
 * ingestion-worker/testdata/fills and ONCHAIN_INTEGRATION.md §8).
 */
import { describe, it, expect, vi, afterEach } from 'vitest';
import { api } from './api/client';
import type { AccumulationEvent } from './api/client';
import type { FillDetection, FillOffer } from './types/snapshot';
import {
  blockTime,
  confirmOffer,
  costOverSpot,
  detectionSummary,
  dismissOffer,
  fillsForPlan,
  restoreOffer,
  shortHash,
  swapLine,
  timingLabel,
  txUrl,
  type FillOfferClient,
} from './fillOffers';

afterEach(() => {
  vi.unstubAllGlobals();
});

const BUY_3 = '0x78376f3cc9e80f2eb124a945a452df40556649e3d9983be526b2bd97586e31b9';

const offer: FillOffer = {
  tx_hash: BUY_3,
  confirmed_at: '2026-11-16T03:08:37+00:00',
  block_number: 58040708,
  usdc_spent: '20',
  nock_received: '1780.3592976960049083',
  effective_price_usd: '0.01123369',
  timing: 'on_schedule',
  reference_spot_usd: null,
  reference_spot_source: null,
  execution_cost_bps: null,
};

const detection = (over: Partial<FillDetection>): FillDetection => ({
  status: 'ok',
  detail: null,
  checked_back_to: '2026-11-14T00:00:00+00:00',
  last_checked_at: '2026-11-16T04:00:00+00:00',
  offers: [],
  dismissed: [],
  ...over,
});

describe('detection state', () => {
  it('says "checked, none found" only when the check ran', () => {
    const s = detectionSummary(detection({}));
    expect(s.heading).toBe('CHECKED');
    expect(s.tone).toBe('ok');
    expect(s.text).toBe('Wallet checked at 2026-11-16 04:00 UTC, back to 2026-11-14 00:00 UTC: no uncounted swap found.');
  });

  it('counts open offers without calling them counted', () => {
    expect(detectionSummary(detection({ offers: [offer] })).text).toContain('1 swap found, not counted yet');
    expect(detectionSummary(detection({ offers: [offer, offer] })).text).toContain('2 swaps found, not counted yet');
  });

  it('renders unavailable as "could not check", never as "none found"', () => {
    const s = detectionSummary(
      detection({
        status: 'unavailable',
        detail: "the wallet's transfer history could not be read (alchemy_getAssetTransfers gave no answer)",
        checked_back_to: null,
        last_checked_at: '2026-11-16T03:55:00+00:00',
      }),
    );
    expect(s.heading).toBe('COULD NOT CHECK');
    expect(s.tone).toBe('warn');
    expect(s.text).toContain('Could not check the wallet for swaps this cycle');
    expect(s.text).toContain('Last successful check: 2026-11-16 03:55 UTC');
    expect(s.text).toContain('record a buy by its transaction hash below');
    expect(s.text).not.toContain('no uncounted swap found');
    // No successful check on record is said, not shown as a time.
    const never = detectionSummary(detection({ status: 'unavailable', detail: 'x', last_checked_at: null }));
    expect(never.text).toContain('Last successful check: none on record');
  });

  it('distinguishes an incomplete check and a check that did not run', () => {
    const partial = detectionSummary(
      detection({ status: 'partial', detail: '1 transaction(s) could not be read and will be tried again next cycle' }),
    );
    expect(partial.heading).toBe('CHECKED, INCOMPLETELY');
    expect(partial.text).toContain('but not completely: 1 transaction(s) could not be read');
    const off = detectionSummary(
      detection({ status: 'not_run', detail: 'swap detection is turned off (fill_detection.enabled is false)', last_checked_at: null }),
    );
    expect(off.heading).toBe('NOT CHECKED');
    expect(off.text).toBe('Swap detection did not run: swap detection is turned off (fill_detection.enabled is false).');
    expect(off.text).not.toContain('no uncounted swap found');
  });

  it('claims nothing for a snapshot that reports no detection', () => {
    for (const absent of [undefined, null]) {
      const s = detectionSummary(absent);
      expect(s.tone).toBe('muted');
      expect(s.text).toContain('older than v0.6.0');
      expect(s.text).not.toContain('no uncounted swap found');
    }
  });
});

describe('swap figures', () => {
  it('formats the amounts and price from the receipt figures', () => {
    expect(swapLine(offer)).toBe('$20.00 USDC → 1,780.36 NOCK at $0.011234 per NOCK');
    // Unparseable figures are shown as given, not as NaN or zero.
    expect(swapLine({ usdc_spent: 'n/a', nock_received: '', effective_price_usd: '' })).toBe('$n/a USDC →  NOCK at $ per NOCK');
  });

  it('shows the full hash to the explorer and a short form for tight spaces', () => {
    expect(shortHash(BUY_3)).toBe('0x78376f3c…6e31b9');
    expect(shortHash('0xabc')).toBe('0xabc');
    expect(txUrl(BUY_3)).toBe(`https://basescan.org/tx/${BUY_3}`);
    expect(blockTime('2026-11-16T03:08:37+00:00')).toBe('2026-11-16 03:08 UTC');
    expect(blockTime('whenever')).toBe('whenever');
  });

  it('labels a swap that landed before its tranche was due', () => {
    expect(timingLabel('early')).toBe('early: before the open tranche came due');
    expect(timingLabel('on_schedule')).toBe('on schedule');
    expect(timingLabel(undefined)).toBe('');
  });
});

describe('execution cost over spot', () => {
  it('reports the cost with its reference', () => {
    const c = costOverSpot({
      execution_cost_bps: 160,
      reference_spot_usd: '0.01250000',
      reference_spot_source: 'market_price_history uniswap_rpc at 2026-11-15T02:55:00+00:00',
    });
    expect(c.known).toBe(true);
    expect(c.text).toBe('+160 bps (1.60%) over spot $0.012500');
    expect(c.source).toContain('uniswap_rpc');
    expect(costOverSpot({ execution_cost_bps: -80, reference_spot_usd: '0.0125' }).text).toBe('−80 bps (0.80%) under spot $0.012500');
    expect(costOverSpot({ execution_cost_bps: 0, reference_spot_usd: '0.0125' }).text).toBe('at spot ($0.012500)');
  });

  it('is unavailable without a reference, never zero', () => {
    for (const f of [
      { execution_cost_bps: null, reference_spot_usd: null },
      { execution_cost_bps: undefined, reference_spot_usd: undefined },
      { execution_cost_bps: 160, reference_spot_usd: null },
      { execution_cost_bps: null, reference_spot_usd: '0.0125' },
      { execution_cost_bps: Number.NaN, reference_spot_usd: '0.0125' },
    ]) {
      const c = costOverSpot(f);
      expect(c.known).toBe(false);
      expect(c.text).toContain('unavailable');
      expect(c.text).not.toMatch(/\d+ bps/);
    }
  });
});

describe('offer actions', () => {
  const client = (over: Partial<FillOfferClient> = {}): FillOfferClient => ({
    recordAccumulation: vi.fn().mockResolvedValue({
      recorded: true,
      duplicate: false,
      plan_id: '2026-11-reentry',
      tx_hash: BUY_3,
      usdc_spent: '20',
      nock_received: '1780.3592976960049083',
      effective_price_usd: '0.01123369',
      block_number: 58040708,
      confirmed_at: '2026-11-16T03:08:37+00:00',
    }),
    dismissFillOffer: vi.fn().mockResolvedValue({ tx_hash: BUY_3, state: 'dismissed', changed: true }),
    restoreFillOffer: vi.fn().mockResolvedValue({ tx_hash: BUY_3, state: 'offered', changed: true }),
    ingestNow: vi.fn().mockResolvedValue({}),
    ...over,
  });

  it('CONFIRM records the offered hash through the record endpoint, then refreshes', async () => {
    const c = client();
    const res = await confirmOffer(offer, c);
    expect(c.recordAccumulation).toHaveBeenCalledTimes(1);
    expect(c.recordAccumulation).toHaveBeenCalledWith(BUY_3);
    expect(c.ingestNow).toHaveBeenCalledTimes(1);
    expect(c.dismissFillOffer).not.toHaveBeenCalled();
    expect(res).toEqual({
      ok: true,
      text: 'Confirmed and verified on-chain: $20.00 USDC → 1,780.36 NOCK at $0.011234 per NOCK.',
    });
  });

  it('CONFIRM goes over the wire as the same request a pasted hash makes', async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ recorded: true, duplicate: false, usdc_spent: '20', nock_received: '1780.36', effective_price_usd: '0.01123369' }),
    });
    vi.stubGlobal('fetch', fetchMock);
    await confirmOffer(offer);
    const [url, init] = fetchMock.mock.calls[0];
    expect(String(url)).toContain('/execute/record-accumulation');
    expect(init.method).toBe('POST');
    // Only the hash is sent: the amounts on the card are never trusted by the worker.
    expect(JSON.parse(init.body)).toEqual({ tx_hash: BUY_3 });
    expect(String(fetchMock.mock.calls[1][0])).toContain('/ingest');
  });

  it('a verification failure is shown and nothing else is called', async () => {
    const c = client({
      recordAccumulation: vi.fn().mockRejectedValue(new Error('Receipt does not show USDC leaving and NOCK entering the sovereign wallet')),
    });
    const res = await confirmOffer(offer, c);
    expect(res).toEqual({ ok: false, text: 'Receipt does not show USDC leaving and NOCK entering the sovereign wallet' });
    expect(c.ingestNow).not.toHaveBeenCalled();
  });

  it('a failed refresh after recording is not a failed recording', async () => {
    const c = client({ ingestNow: vi.fn().mockRejectedValue(new Error('timeout')) });
    expect((await confirmOffer(offer, c)).ok).toBe(true);
    const dup = client({ recordAccumulation: vi.fn().mockResolvedValue({ recorded: false, duplicate: true }) });
    expect((await confirmOffer(offer, dup)).text).toBe('Already recorded: this transaction is in the ledger.');
  });

  it('NOT PART OF THE PLAN and RESTORE never touch the record endpoint', async () => {
    const c = client();
    const dismissed = await dismissOffer(offer, c);
    expect(c.dismissFillOffer).toHaveBeenCalledWith(BUY_3);
    expect(dismissed.ok).toBe(true);
    expect(dismissed.text).toContain('not part of the plan');
    const restored = await restoreOffer(offer, c);
    expect(c.restoreFillOffer).toHaveBeenCalledWith(BUY_3);
    expect(restored.text).toContain('still not counted until you confirm it');
    expect(c.recordAccumulation).not.toHaveBeenCalled();
    expect(c.ingestNow).toHaveBeenCalledTimes(2);

    const broken = client({ dismissFillOffer: vi.fn().mockRejectedValue(new Error('This buy is already in the plan ledger.')) });
    expect(await dismissOffer(offer, broken)).toEqual({ ok: false, text: 'This buy is already in the plan ledger.' });
  });

  it('dismiss and restore post the hash to the candidates endpoints', async () => {
    const fetchMock = vi.fn().mockResolvedValue({ ok: true, status: 200, json: async () => ({ tx_hash: BUY_3, state: 'dismissed', changed: true }) });
    vi.stubGlobal('fetch', fetchMock);
    await api.dismissFillOffer(BUY_3);
    await api.restoreFillOffer(BUY_3);
    await api.accumulationEvents();
    const [dismissUrl, dismissInit] = fetchMock.mock.calls[0];
    expect(String(dismissUrl)).toMatch(/\/accumulation\/candidates\/dismiss$/);
    expect(dismissInit.method).toBe('POST');
    expect(JSON.parse(dismissInit.body)).toEqual({ tx_hash: BUY_3 });
    expect(String(fetchMock.mock.calls[1][0])).toMatch(/\/accumulation\/candidates\/restore$/);
    expect(JSON.parse(fetchMock.mock.calls[1][1].body)).toEqual({ tx_hash: BUY_3 });
    expect(String(fetchMock.mock.calls[2][0])).toMatch(/\/accumulation\/events$/);
  });
});

describe('per-fill list', () => {
  const event = (plan_id: string, tx_hash: string, confirmed_at: string): AccumulationEvent => ({
    plan_id,
    tx_hash,
    venue: 'Aero',
    usdc_spent: '20',
    nock_received: '1600',
    effective_price_usd: '0.01250000',
    block_number: 1,
    confirmed_at,
  });

  it('keeps the current plan only, oldest first', () => {
    const events = [
      event('2026-11-reentry', '0xb', '2026-11-15T02:58:46+00:00'),
      event('older-plan', '0xz', '2026-10-01T00:00:00+00:00'),
      event('2026-11-reentry', '0xa', '2026-11-14T06:02:25+00:00'),
    ];
    expect(fillsForPlan(events, '2026-11-reentry').map((e) => e.tx_hash)).toEqual(['0xa', '0xb']);
    expect(events.map((e) => e.tx_hash)).toEqual(['0xb', '0xz', '0xa']);
    expect(fillsForPlan(null, 'p')).toEqual([]);
    expect(fillsForPlan(undefined, 'p')).toEqual([]);
  });
});
