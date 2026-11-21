import { describe, expect, it } from 'vitest';
import { reserveText } from './reserve.ts';

describe('the standing cash reserve line (#162)', () => {
  it('states the headroom from the operator numbers of 2026-11-21', () => {
    const g = { cash_reserve_usdc: '950.00', reserve_stable_usdc: '1768.18', reserve_headroom_usdc: '818.18' };
    expect(reserveText(g)).toBe('$950.00 protected reserve (wallet + NOCK LP USDC): $818.18 above it');
  });

  it('says below, never a negative "above"', () => {
    const g = { cash_reserve_usdc: '950.00', reserve_headroom_usdc: '-150.00' };
    expect(reserveText(g)).toBe('$950.00 protected reserve (wallet + NOCK LP USDC): $150.00 below it');
  });

  it('is absent with no reserve and honest without a measurement', () => {
    expect(reserveText({})).toBeNull();
    expect(reserveText(null)).toBeNull();
    expect(reserveText({ cash_reserve_usdc: '' })).toBeNull();
    expect(reserveText({ cash_reserve_usdc: '950.00' })).toBe('$950.00 protected reserve: not measured this cycle');
  });
});
