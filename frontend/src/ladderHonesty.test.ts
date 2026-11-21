import { describe, expect, it } from 'vitest';
import { feeEstimateLines } from './feeYield';
import { ladderGateWhere, ladderSummaryLines, type LadderFeeEstimate, type LadderSimulation } from './ladder';
import { SPOT_741_SNAPPED } from './ladder.fixture';
import { THE_BOTTLE_CONFIGURED, TWO_RUNGS_MEASURED } from './ladderRungs.fixture';

/** Every fee estimate in a body: the one-rung estimate and each rung's. */
const estimates = (sim: LadderSimulation): LadderFeeEstimate[] =>
  [sim.fee_estimate, ...(sim.rungs ?? []).map((r) => r.fee_estimate)].filter((f): f is LadderFeeEstimate => f != null);

describe('the honesty rules the helpers keep (2.10.0 review)', () => {
  it('never prints a fee per day without its sample size, in-range share and replay bound', () => {
    let printed = 0;
    for (const sim of [TWO_RUNGS_MEASURED, THE_BOTTLE_CONFIGURED, SPOT_741_SNAPPED]) {
      for (const fe of estimates(sim)) {
        const lines = feeEstimateLines(fe);
        if (!lines[0].includes('/day')) continue;
        printed += 1;
        expect(lines[0]).toMatch(/measured over \d+\.\d\d days \([\d,]+ intervals\) on /);
        expect(lines[1]).toMatch(/^In range [\d.]+% of the measured time$|^In range over stored history: not stated \(.+\)$/);
        expect(lines[2]).toMatch(/^Replayed over it: at most \$[\d.]+ inside|^Replay: not stated \(.+\)$/);
      }
    }
    expect(printed).toBeGreaterThanOrEqual(5);
    expect(feeEstimateLines(TWO_RUNGS_MEASURED.rungs![0].fee_estimate)).toEqual([
      'Fees while in range (an estimate): $18.23/day, measured over 6.33 days (1,836 intervals) on v4 USDC/NOCK 3.00% #3163450',
      'In range 0.11% of the measured time',
      'Replayed over it: at most $1.31 inside and $4.70 more at its edges (no lower bound)',
    ]);
  });

  it('renders the reason in place of a missing share or replay, and of an unavailable estimate', () => {
    const fe = structuredClone(TWO_RUNGS_MEASURED.rungs![0].fee_estimate);
    fe.in_range_share_stored_history = null;
    fe.in_range_share_reason = 'unsnapped: the range\'s bounds are not on the pool\'s tick grid';
    fe.stored_history_replay = null;
    fe.replay_reason = 'this snapshot was read back from the database';
    const lines = feeEstimateLines(fe);
    expect(lines[1]).toBe("In range over stored history: not stated (unsnapped: the range's bounds are not on the pool's tick grid)");
    expect(lines[2]).toBe('Replay: not stated (this snapshot was read back from the database)');
    // No measurement: the reason, never $0.00.
    const none = feeEstimateLines(SPOT_741_SNAPPED.fee_estimate);
    expect(none).toEqual(['Fees while in range: not estimated: this snapshot has no fee_yield (2.10.0): fees are not measured in it']);
    expect(none.join(' ')).not.toContain('$0.00');
    const noSample = feeEstimateLines({ ...fe, measured_days: null, intervals: null });
    expect(noSample[0]).toContain('sample size not stated');
  });

  it('says where the reserve gate is read: at the bottom, or at the gate price with its rule', () => {
    expect(ladderGateWhere(SPOT_741_SNAPPED)).toBe('at the bottom');
    expect(ladderSummaryLines(SPOT_741_SNAPPED).find((l) => l.startsWith('Reserve:'))).toContain('stable $976.45 at the bottom against');
    const gated = structuredClone(SPOT_741_SNAPPED);
    gated.reserve_check.gate_price_usd = '0.01000000';
    gated.reserve_check.gate_price_rule = 'your ladder bottom (operator_profile.ladder_bottom_usd $0.01000000), below the deepest rung\'s bottom';
    const line = ladderSummaryLines(gated).find((l) => l.startsWith('Reserve:'))!;
    expect(line).toContain('at the gate $0.01000000 (your ladder bottom (operator_profile.ladder_bottom_usd $0.01000000), below the deepest rung');
    expect(line).not.toContain('at the bottom');
    // A ladder of rungs always states its gate.
    expect(ladderGateWhere(TWO_RUNGS_MEASURED)).toContain('at the gate $0.01002405 (the deepest rung');
  });

  it('every trade-off row is gated at one price and shows share and replay beside a fee', () => {
    const rows = TWO_RUNGS_MEASURED.tradeoff!.split!.rows as Array<Record<string, any>>;
    expect(new Set(rows.map((r) => r.gate_price_usd))).toEqual(new Set(['0.01002405']));
    for (const r of rows) {
      for (const x of r.per_rung) {
        if (x.fee_per_day_in_range_usd != null) {
          expect(x.in_range_share_pct).not.toBeNull();
          expect(x.replay_inside_upper_usd).not.toBeNull();
          expect(x.replay_edge_upper_usd).not.toBeNull();
        }
      }
    }
  });
});
