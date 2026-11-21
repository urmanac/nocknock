import { describe, expect, it } from 'vitest';
import { existsSync, readFileSync } from 'node:fs';
import { feeYieldAgeMinutes, feeYieldByDay, feeYieldLine, feeYieldPool, feeYieldSpreadLine, type FeeYield } from './feeYield';
import { FEE_YIELD } from './feeYield.fixture';

const POOL = '0x064a5df2ecfcb7c05779b147aa38e4b0e91c966267fbdbc8cf16addd83636b13';
const at = (min: number) => new Date(Date.parse('2026-11-20T11:50:00Z') + min * 60000);

describe('fee yield (2.10.0)', () => {
  it('the fixture is the worker golden file', () => {
    const golden = new URL('../../ingestion-worker/testdata/fee_yield/golden_fee_yield.json', import.meta.url);
    // A frontend-only checkout has no worker directory; the copy is then all there is.
    if (!existsSync(golden)) return;
    expect(FEE_YIELD).toEqual(JSON.parse(readFileSync(golden, 'utf8')).fee_yield);
  });

  it('states the measurement as one quiet line, with its days, intervals and age', () => {
    expect(feeYieldLine(FEE_YIELD, at(15), POOL)).toBe(
      'Fees measured on v4 USDC/NOCK 3.00% #3163450: $6.0811 per 1e15 liquidity per day while in range, over 6.33 days (1,836 intervals), measured 15 min ago. A past measurement, not a forecast.',
    );
    expect(feeYieldAgeMinutes(FEE_YIELD, at(61))).toBe(61);
    // No pool given: the first measured one.
    expect(feeYieldPool(FEE_YIELD)?.measured_by).toBe('uniswap-v4:base:3163450');
    expect(feeYieldPool(FEE_YIELD, '0xother')).toBeNull();
  });

  it('says why there is no figure, never a zero', () => {
    expect(feeYieldLine(undefined, at(0))).toBe('Fees per unit of liquidity: not measured in this snapshot (a worker before 2.10.0)');
    const none: FeeYield = { ...FEE_YIELD, status: 'unavailable', reason: 'the stored history could not be read this cycle', pools: [] };
    expect(feeYieldLine(none, at(0))).toBe('Fees per unit of liquidity: not measured: the stored history could not be read this cycle');
    expect(feeYieldLine(FEE_YIELD, at(0), '0xother')).toBe('Fees per unit of liquidity: no tracked range in this pool measures them');
    const empty: FeeYield = { ...FEE_YIELD, pools: [{ ...FEE_YIELD.pools[0], status: 'no_intervals', reason: '1 stored rows of x in the window: too little history to measure' }] };
    expect(feeYieldLine(empty, at(0))).toContain('none yet: 1 stored rows');
    for (const line of [feeYieldLine(FEE_YIELD, at(0)), feeYieldLine(none, at(0))]) {
      expect(line.toLowerCase()).not.toMatch(/should|recommend|suggest|best|deposit/);
    }
  });

  it('shows every UTC day with its hours, and the spread over the days counted', () => {
    const pool = feeYieldPool(FEE_YIELD, POOL)!;
    const s = feeYieldByDay(pool, FEE_YIELD.day_min_seconds);
    expect(s.rows.map((r) => [r.date, r.hours, r.counted])).toEqual([
      ['2026-11-14', '8.5', false],
      ['2026-11-15', '24.0', true],
      ['2026-11-16', '23.9', true],
      ['2026-11-17', '19.8', false],
      ['2026-11-18', '23.9', true],
      ['2026-11-19', '23.8', true],
      ['2026-11-20', '24.0', true],
      ['2026-11-21', '4.0', false],
    ]);
    // Each day at the cycle's pool price, from the worker's own legs.
    for (const [i, d] of pool.by_day.entries()) {
      const expected = (Number(d.usdc_per_liquidity_day) + Number(d.nock_per_liquidity_day) * Number(pool.pool_price_usd)) * 1e15;
      expect(s.rows[i].usd_per_1e15).toBeCloseTo(expected, 4);
    }
    expect(s.days_counted).toBe(5);
    expect(feeYieldSpreadLine(s)).toBe('Daily spread: min $3.10, median $5.29, max $7.99 per 1e15 liquidity per day over 5 UTC days with at least 20.0 h measured');
    expect(s.scale_max).toBeCloseTo(9.7357, 3);
    // An even count takes the mean of the two middle days.
    const four = feeYieldByDay({ ...pool, by_day: pool.by_day.filter((d) => d.date !== '2026-11-20') }, FEE_YIELD.day_min_seconds);
    const mid = four.rows.filter((r) => r.counted).map((r) => r.usd_per_1e15 as number).sort((a, b) => a - b);
    expect(four.median).toBeCloseTo((mid[1] + mid[2]) / 2, 9);
    expect(feeYieldSpreadLine(feeYieldByDay({ ...pool, by_day: [] }, 72000))).toBe('Daily spread: no UTC day has at least 20.0 h measured yet');
  });
});
