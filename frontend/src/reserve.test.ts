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

  it('reads -0.00 as zero, above the reserve (#162 review)', () => {
    expect(reserveText({ cash_reserve_usdc: '950.00', reserve_headroom_usdc: '-0.00' })).toBe('$950.00 protected reserve (wallet + NOCK LP USDC): $0.00 above it');
  });

  it('names a reserve that is not applied', () => {
    expect(reserveText({ reserve_note: 'operator_profile.cash_reserve_usdc must be zero or positive' })).toBe(
      'protected reserve not applied: operator_profile.cash_reserve_usdc must be zero or positive',
    );
  });
});
