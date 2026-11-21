import { describe, expect, it } from 'vitest';
import { existsSync, readFileSync } from 'node:fs';
import { buildRungsQuery } from './ladder';
import { THE_BOTTLE_CONFIGURED, TWO_RUNGS_MEASURED } from './ladderRungs.fixture';
import { SPOT_741_SNAPPED } from './ladder.fixture';

describe('ladder of rungs and the configured shape (2.10.0)', () => {
  it('the fixtures are the worker golden file, body for body', () => {
    const golden = new URL('../../ingestion-worker/testdata/ladder/golden_ladder_rungs.json', import.meta.url);
    if (!existsSync(golden)) return;
    const doc = JSON.parse(readFileSync(golden, 'utf8'));
    expect(TWO_RUNGS_MEASURED).toEqual(doc.two_rungs_measured.body);
    expect(THE_BOTTLE_CONFIGURED).toEqual(doc.the_bottle_configured.body);
  });

  it('builds the indexed query and the configured-shape query the worker reads', () => {
    expect(
      buildRungsQuery(
        [
          { rung: 'near', usdc: '100', high: 'spot', low: '0.012' },
          { rung: 'deep', usdc: '500', high: '0.011', low: '0.01' },
        ],
        { positionId: 'uniswap-v4:base:3163450' },
      ).toString(),
    ).toBe('r1_rung=near&r1_usdc=100&r1_low=0.012&r1_high=spot&r2_rung=deep&r2_usdc=500&r2_low=0.01&r2_high=0.011&position_id=uniswap-v4%3Abase%3A3163450');
    expect(buildRungsQuery('configured', { positionId: 'p' }).toString()).toBe('shape=configured&position_id=p');
    expect(buildRungsQuery('configured', { shapeUsdc: '766.84' }).toString()).toBe('shape=configured&shape_usdc=766.84');
    expect(() => buildRungsQuery([])).toThrow('1 to 4 rungs');
    expect(() => buildRungsQuery([{ rung: '' as never, usdc: '1' }])).toThrow('r1_rung is required');
  });

  it('reads the per-rung fee estimate beside its in-range share and replay bounds', () => {
    const near = TWO_RUNGS_MEASURED.rungs![0];
    expect(near.fee_estimate.per_day_in_range_usd).toBe('18.23');
    expect(near.fee_estimate.in_range_share_stored_history?.share_pct).toBe('0.11');
    expect(near.fee_estimate.stored_history_replay?.inside_upper_usd).toBe('1.31');
    expect(near.round_trip?.usdc_back).toBe('100.00');
    expect(TWO_RUNGS_MEASURED.rungs![1].if_pulled_at_fill?.nock_kept).toBe('47685.2789');
    expect(TWO_RUNGS_MEASURED.combined?.gate_price_usd).toBe('0.01002405');
    // The one-rung body carries its own estimate (here: no measurement in that fixture).
    expect(SPOT_741_SNAPPED.fee_estimate?.status).toBe('unavailable');
  });

  it('the bottle: a cap in the band containing spot, then 15/85, adding up to the gated total', () => {
    const b = THE_BOTTLE_CONFIGURED;
    expect(b.rungs!.map((r) => r.rung)).toEqual(['cap', 'near', 'deep']);
    const cap = b.rungs![0];
    expect([cap.bounds.tick_lower, cap.bounds.tick_upper]).toEqual([273600, 273900]);
    expect(cap.cap?.bound_by).toBe('lp_nock_budget');
    expect(cap.sizing.nock_needed_at_spot).toBe('86.7891');
    const sum = b.rungs!.reduce((a, r) => a + Number(r.sizing.usdc_at_spot), 0);
    expect(sum).toBeCloseTo(766.99, 1);
    expect(b.combined?.total_usdc).toBe('766.99');
    expect(b.reserve_check.margin_usdc).toBe('0.00');
  });

  it('the trade-off tables never rank or mark a row', () => {
    const t = TWO_RUNGS_MEASURED.tradeoff!;
    expect(t.statement).toBe('Rows are measurements at your depths; NockNock does not choose the split or the depths.');
    const shares = t.split!.rows.map((r) => Number(r.first_rung_share_pct));
    expect(shares).toEqual([...shares].sort((a, b) => a - b));
    for (const r of t.split!.rows) {
      expect(r).not.toHaveProperty('yours');
      expect(r).not.toHaveProperty('rank');
    }
    const all = JSON.stringify([TWO_RUNGS_MEASURED.tradeoff, TWO_RUNGS_MEASURED.rungs, TWO_RUNGS_MEASURED.combined]).toLowerCase();
    expect(all).not.toMatch(/best|should|recommend|suggest|deposit/);
  });
});
