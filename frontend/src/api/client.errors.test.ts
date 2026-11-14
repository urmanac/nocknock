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

// ---------------- LP rebuild suggestion display (v0.5.0, issue #83) ----------------
import { suggestionLegs } from '../accumulation';
import { rangeStatus } from '../positions';

describe('LP rebuild suggestion display', () => {
  it('lists both legs of a deposit and drops a zero leg', () => {
    expect(suggestionLegs({ amount0: '78.20', token0_symbol: 'USDC', amount1: '8730', token1_symbol: 'NOCK' })).toBe(
      '78.20 USDC + 8,730 NOCK',
    );
    expect(suggestionLegs({ amount0: '0.000000', token0_symbol: 'ETH', amount1: '10070', token1_symbol: 'NOCK' })).toBe(
      '10,070 NOCK',
    );
    expect(suggestionLegs({ amount0: '0.016400', token0_symbol: 'WETH', amount1: '0', token1_symbol: 'NOCK' })).toBe(
      '0.016400 WETH',
    );
  });

  it('never shows a dead pool as in range', () => {
    expect(rangeStatus({ in_range: true, pool_active: false })).toEqual({ label: '● STALE POOL', tone: 'stale' });
    expect(rangeStatus({ in_range: true, pool_active: true }).tone).toBe('good');
    expect(rangeStatus({ in_range: false, pool_active: true }).tone).toBe('warn');
    // Unknown pool state (older snapshot, failed read) falls back to the tick-based answer.
    expect(rangeStatus({ in_range: true }).label).toBe('● IN RANGE');
    expect(rangeStatus({ in_range: true, pool_active: null }).tone).toBe('good');
  });
});

// ---------------- NOCK stack chart units (issue #85) ----------------
import { walletNockUnits } from '../positions';

describe('wallet NOCK for the floor chart', () => {
  it('uses token units when the snapshot carries them', () => {
    // The 2026-11-14 values: 95,598.98 NOCK worth $1,094.58 at $0.01145.
    const ag = { wallet_nock_series: ['95598.9782'], safe_haven_series: ['1094.58'] };
    expect(walletNockUnits(ag, 0, 0.01145)).toBeCloseTo(95598.9782, 3);
  });

  it('recovers units for older snapshots from their own USD value and spot', () => {
    // Pre-0.5.0 snapshot: only the USD series exists. Never plot 1,094 as NOCK.
    const nock = walletNockUnits({ safe_haven_series: ['1094.58'] }, 0, 0.01145);
    expect(nock).not.toBeNull();
    expect(Math.round(nock!)).toBe(95597);
    expect(nock!).toBeGreaterThan(90000);
  });

  it('leaves a gap rather than guessing when units cannot be known', () => {
    expect(walletNockUnits({ safe_haven_series: ['1094.58'] }, 0, null)).toBeNull();
    expect(walletNockUnits({ safe_haven_series: ['1094.58'] }, 0, 0)).toBeNull();
    expect(walletNockUnits(undefined, 0, 0.01145)).toBeNull();
    expect(walletNockUnits({ wallet_nock_series: [] }, 0, 0.01145)).toBeNull();
  });
});
