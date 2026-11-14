/**
 * Micro-tests — API client error mapping + record-harvest retry semantics.
 *
 * Covers the 0.2.x triage fixes (TRIAGE.md Phase E):
 * - E5: 409 ALREADY_RECORDED must classify as an idempotent success, and the
 *   client must keep the HTTP status on thrown errors.
 * - E6: the human-readable `detail` from the backend must surface before the
 *   machine error code.
 */
import { describe, it, expect, vi, afterEach } from 'vitest';
import { api } from './client';
import { classifyRecordError } from '../components/LiteCockpit';

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('client error mapping', () => {
  it('prefers the human-readable detail over the machine error code and keeps status', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response(
      JSON.stringify({
        error: 'RECEIPT_NOT_FOUND',
        detail: 'Transaction not found on-chain. If the wallet shows it approved, it may still be pending broadcast in the wallet.',
      }),
      { status: 404, headers: { 'content-type': 'application/json' } },
    )));
    let caught: (Error & { status?: number }) | undefined;
    try {
      await api.recordHarvest('aerodrome', `0x${'1'.repeat(64)}`);
    } catch (e) {
      caught = e as Error & { status?: number };
    }
    expect(caught).toBeInstanceOf(Error);
    expect(caught!.message).toContain('pending broadcast');
    expect(caught!.status).toBe(404);
  });

  it('falls back to the HTTP status when the error body is not JSON', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response('nope', { status: 500 })));
    let caught: (Error & { status?: number }) | undefined;
    try {
      await api.health();
    } catch (e) {
      caught = e as Error & { status?: number };
    }
    expect(caught!.message).toBe('HTTP 500');
    expect(caught!.status).toBe(500);
  });
});

describe('classifyRecordError (record-harvest retry semantics)', () => {
  it('treats 409 ALREADY_RECORDED as recorded (idempotent success)', () => {
    const e = Object.assign(new Error('ALREADY_RECORDED'), { status: 409 });
    expect(classifyRecordError(e)).toBe('recorded');
  });

  it('treats 404 RECEIPT_NOT_FOUND as awaiting_broadcast (pending-broadcast quirk)', () => {
    const e = Object.assign(new Error('Transaction not found on-chain.'), { status: 404 });
    expect(classifyRecordError(e)).toBe('awaiting_broadcast');
  });

  it('treats anything else as failed', () => {
    expect(classifyRecordError(new Error('DB_ERROR'))).toBe('failed');
    expect(classifyRecordError(Object.assign(new Error('boom'), { status: 500 }))).toBe('failed');
  });
});

// ---------------- Accumulation plan helpers (v0.5.0, issue #79) ----------------
import { dcaVsLumpSumNock, isTxHash, lpRebuildActive, planProgressPct, timeUntil } from '../accumulation';

describe('accumulation plan display helpers', () => {
  it('reports budget progress and clamps it to 0–100', () => {
    expect(planProgressPct({ spent_usdc: '200.00', budget_usdc: '500.00' })).toBe(40);
    expect(planProgressPct({ spent_usdc: '600.00', budget_usdc: '500.00' })).toBe(100);
    expect(planProgressPct({ spent_usdc: '10.00', budget_usdc: '0.00' })).toBe(0);
  });

  it('describes time until the next tranche without inventing one', () => {
    const now = new Date('2026-11-15T12:00:00Z');
    expect(timeUntil('2026-11-16T02:05:00Z', now)).toBe('in 14h 05m');
    expect(timeUntil('2026-11-18T14:00:00Z', now)).toBe('in 3d 2h');
    expect(timeUntil('2026-11-15T12:20:00Z', now)).toBe('in 20m');
    expect(timeUntil('2026-11-15T11:00:00Z', now)).toBe('now');
    expect(timeUntil(null, now)).toBeNull();
    expect(timeUntil('not-a-date', now)).toBeNull();
  });

  it('compares averaging with the lump-sum counterfactual only once a fill exists', () => {
    // Budget 500 at average 0.0117647 -> 42,500 NOCK on pace; lump sum bought 40,000.
    const ahead = dcaVsLumpSumNock({ avg_price_usd: '0.011765', lump_sum_nock: '40000.0000', budget_usdc: '500.00' });
    expect(ahead).not.toBeNull();
    expect(Math.round(ahead!)).toBe(2499);
    // Averaging can lose: a higher average cost shows a negative number, not a hidden one.
    const behind = dcaVsLumpSumNock({ avg_price_usd: '0.013000', lump_sum_nock: '40000.0000', budget_usdc: '500.00' });
    expect(behind!).toBeLessThan(0);
    expect(dcaVsLumpSumNock({ avg_price_usd: null, lump_sum_nock: null, budget_usdc: '500.00' })).toBeNull();
  });

  it('shows the LP rebuild bias only when LP is under target', () => {
    const lp = { target_usd: '371.29', actual_usd: '0.00', gap_usd: '371.29', target_weight_bps: 1500 };
    expect(lpRebuildActive({ ...lp, status: 'under_target' })).toBe(true);
    expect(lpRebuildActive({ ...lp, status: 'on_target' })).toBe(false);
    expect(lpRebuildActive(undefined)).toBe(false);
  });

  it('validates transaction hashes before calling the worker', () => {
    const h = 'a'.repeat(64);
    expect(isTxHash(`0x${h}`)).toBe(true);
    expect(isTxHash(h)).toBe(true);
    expect(isTxHash(`0x${h.slice(1)}`)).toBe(false);
    expect(isTxHash('0xnothex')).toBe(false);
  });

  it('posts only the hash when recording a buy; the worker supplies the amounts', async () => {
    const fetchMock = vi.fn().mockResolvedValue({
      ok: true,
      status: 200,
      json: async () => ({ recorded: true, duplicate: false, usdc_spent: '100', nock_received: '8100' }),
    });
    vi.stubGlobal('fetch', fetchMock);
    const res = await api.recordAccumulation('0x' + 'b'.repeat(64));
    expect(res.recorded).toBe(true);
    const [url, init] = fetchMock.mock.calls[0];
    expect(String(url)).toContain('/execute/record-accumulation');
    expect(init.method).toBe('POST');
    expect(JSON.parse(init.body)).toEqual({ tx_hash: '0x' + 'b'.repeat(64) });
    vi.unstubAllGlobals();
  });
});
