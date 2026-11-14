/**
 * v0.5.2 surfaces: first-run errors and Aero fees that cannot be claimed.
 */
import { describe, it, expect, vi, afterEach } from 'vitest';
import { api, describeApiError } from './client';
import { velocityRowText } from '../positions';

afterEach(() => {
  vi.unstubAllGlobals();
});

describe('first-run errors', () => {
  it('shows the last ingestion error from a 503 NO_SNAPSHOT, not "Worker unreachable"', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response(
      JSON.stringify({
        error: 'NO_SNAPSHOT',
        message: 'No snapshot available yet',
        detail: 'No NOCK/USD spot: exactly one configured position must have is_price_source: true',
      }),
      { status: 503, headers: { 'content-type': 'application/json' } },
    )));
    let caught: unknown;
    try {
      await api.latestSnapshot();
    } catch (e) {
      caught = e;
    }
    const text = describeApiError(caught);
    expect(text).toBe(
      'No snapshot yet. Last ingestion error: No NOCK/USD spot: exactly one configured position must have is_price_source: true',
    );
    expect(text).not.toContain('unreachable');
  });

  it('says the first cycle has not completed when there is no error yet', async () => {
    vi.stubGlobal('fetch', vi.fn(async () => new Response(
      JSON.stringify({ error: 'NO_SNAPSHOT', message: 'No snapshot available yet' }),
      { status: 503, headers: { 'content-type': 'application/json' } },
    )));
    let caught: unknown;
    try {
      await api.claimFeesPayload();
    } catch (e) {
      caught = e;
    }
    expect(describeApiError(caught)).toBe('No snapshot yet: the first ingestion cycle has not completed.');
  });

  it('keeps a worker-answered error as-is and calls only a network failure unreachable', () => {
    const answered = Object.assign(new Error('No Alchemy RPC key configured'), { status: 503 });
    expect(describeApiError(answered)).toBe('No Alchemy RPC key configured');
    expect(describeApiError(new TypeError('Failed to fetch'))).toBe('Worker unreachable: Failed to fetch');
  });
});

describe('velocity row', () => {
  it('marks a reverting Aero pool as not LP-claimable, never "$0 earned"', () => {
    const text = velocityRowText({
      name: 'Aero USDC/NOCK 0.30% (full range)',
      claimable: 0,
      gasBreakEven: 0.0037,
      ratio: 0,
      action: 'not_claimable',
    });
    expect(text).toBe(
      'Aero USDC/NOCK 0.30% (full range): not LP-claimable (claimFees() reverts); AERO rewards not read (#93)',
    );
    expect(text).not.toContain('earned');
  });

  it('keeps the gas economics for claimable pools', () => {
    const text = velocityRowText({ name: 'Uniswap V4', claimable: 0.12, gasBreakEven: 0.0014, ratio: 85.7, action: 'claim_now' });
    expect(text).toBe('Uniswap V4: CLAIM_NOW ($0.12 earned / gas $0.0014 — 86x ROI)');
  });
});
