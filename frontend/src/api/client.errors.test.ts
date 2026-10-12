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
