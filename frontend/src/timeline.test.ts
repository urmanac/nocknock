/**
 * Chart timeline builders (issue #61, N1, N15). Fixture times are live chudmax rows from
 * 2026-11-14 and the #61 downtime; see docs/kb/plan-v0.5-patches-findings.md (chart-gaps).
 */
import { describe, it, expect } from 'vitest';
import {
  buildChartRows,
  buildPositionSeriesDefs,
  compositionReadout,
  compositionRow,
  finiteOrNull,
  formatGapDuration,
  gapThresholdMs,
  isGapRow,
  toChartSeries,
  withGapBreaks,
  type HistoryEntry,
} from './timeline.ts';

describe('finiteOrNull', () => {
  it('parses finite numbers and numeric strings, keeping a real 0', () => {
    expect(finiteOrNull('2390.53')).toBe(2390.53);
    expect(finiteOrNull(' 0 ')).toBe(0);
    expect(finiteOrNull('0.0000')).toBe(0);
    expect(finiteOrNull(12)).toBe(12);
    expect(finiteOrNull(0)).toBe(0);
  });

  it('never turns a missing or malformed value into 0', () => {
    for (const x of [undefined, null, '', '   ', 'NaN', 'abc', '12abc', NaN, Infinity, -Infinity, {}, [], true]) {
      expect(finiteOrNull(x)).toBeNull();
    }
  });
});

describe('gapThresholdMs', () => {
  it('is 2.5x the sync window', () => {
    expect(gapThresholdMs(300)).toBe(750000);
    expect(gapThresholdMs(60)).toBe(150000);
  });

  it('falls back to the worker default of 300 s', () => {
    for (const x of [undefined, null, 0, -300, NaN]) expect(gapThresholdMs(x)).toBe(750000);
  });
});

describe('formatGapDuration', () => {
  it('rounds to whole minutes', () => {
    expect(formatGapDuration(900000)).toBe('15m');
    expect(formatGapDuration(3600000)).toBe('1h 0m');
    expect(formatGapDuration(69600000)).toBe('19h 20m');
    expect(formatGapDuration(30147853)).toBe('8h 22m');
    expect(formatGapDuration(90000000)).toBe('1d 1h');
  });
});

describe('withGapBreaks', () => {
  it('leaves a regular cadence untouched (live chudmax rows)', () => {
    const rows = [{ t: 1794678064995 }, { t: 1794678365096 }, { t: 1794678664986 }];
    const out = withGapBreaks(rows);
    expect(out).toEqual(rows);
    expect(out[0]).toBe(rows[0]);
  });

  it('does not break for one missed cycle (live restart, 529 s)', () => {
    expect(withGapBreaks([{ t: 1794635038370 }, { t: 1794635567460 }], 300)).toHaveLength(2);
  });

  it('does not flip-flop at the one-missed-cycle span', () => {
    expect(withGapBreaks([{ t: 0 }, { t: 598058 }], 300)).toHaveLength(2);
    expect(withGapBreaks([{ t: 0 }, { t: 602068 }], 300)).toHaveLength(2);
  });

  it('uses an exclusive threshold', () => {
    expect(withGapBreaks([{ t: 0 }, { t: 750000 }], 300)).toHaveLength(2);
    expect(withGapBreaks([{ t: 0 }, { t: 750001 }], 300)).toHaveLength(3);
  });

  it('marks the #61 downtime (12:38 AM to 7:58 PM EST) with a break row and no series keys', () => {
    const before = { t: 1794548280000, portfolio: 2475.27 };
    const after = { t: 1794617880000, portfolio: 2390.53 };
    const out = withGapBreaks([before, after], 300);
    expect(out).toEqual([
      before,
      { gap: true, t: 1794583080000, time: 'no snapshots for 19h 20m', gapStart: 1794548280000, gapEnd: 1794617880000, gapMs: 69600000 },
      after,
    ]);
    expect('portfolio' in out[1]).toBe(false);
    expect(isGapRow(out[1])).toBe(true);
  });

  it('adds no gap at the start or end, and keeps the last row', () => {
    const rows = [0, 300000, 3900000, 4200000, 90000000].map((t) => ({ t }));
    const out = withGapBreaks(rows, 300);
    expect(out.map((r) => ('gap' in r ? 'gap' : r.t))).toEqual([0, 300000, 'gap', 3900000, 4200000, 'gap', 90000000]);
    expect(out[out.length - 1]).toBe(rows[4]);
  });

  it('handles a fresh start: zero rows and one row', () => {
    expect(withGapBreaks([], 300)).toEqual([]);
    const one = [{ t: 1794680164649 }];
    expect(withGapBreaks(one, 300)).toEqual(one);
  });

  it('never creates a gap from bad input', () => {
    expect(withGapBreaks([{ t: NaN }, { t: 10000000 }], 300)).toHaveLength(2);
    expect(withGapBreaks([{ t: 10000000 }, { t: 0 }], 300)).toHaveLength(2);
  });

  it('follows a configured cadence', () => {
    const out = withGapBreaks([{ t: 0 }, { t: 180000 }], 60);
    expect(out).toHaveLength(3);
    expect(out[1]).toEqual({ gap: true, t: 90000, time: 'no snapshots for 3m', gapStart: 0, gapEnd: 180000, gapMs: 180000 });
  });

  it('labels a quiet stretch literally, never as offline (finding 61-5)', () => {
    const out = withGapBreaks([{ t: 1794631109570 }, { t: 1794661257423 }]);
    expect(out[1]).toEqual({
      gap: true,
      t: 1794646183496,
      time: 'no snapshots for 8h 22m',
      gapStart: 1794631109570,
      gapEnd: 1794661257423,
      gapMs: 30147853,
    });
    expect((out[1] as { time: string }).time).not.toContain('offline');
  });
});

describe('toChartSeries', () => {
  it('marks a row with gaps on both sides as isolated, and takes ticks from real rows only', () => {
    const rows = [0, 300000, 5000000, 9000000, 9300000].map((t) => ({ t, v: 1 }) as { t: number; v: number; isolated?: boolean });
    const s = toChartSeries(rows, 300);
    expect(s.rows.map((r) => (isGapRow(r) ? 'gap' : r.t))).toEqual([0, 300000, 'gap', 5000000, 'gap', 9000000, 9300000]);
    expect(s.gaps).toHaveLength(2);
    expect(s.ticks).toEqual([0, 300000, 5000000, 9000000, 9300000]);
    const real = s.rows.filter((r) => !isGapRow(r)) as Array<{ t: number; isolated?: boolean }>;
    expect(real.map((r) => r.isolated)).toEqual([false, false, true, false, false]);
  });

  it('sorts rows by time, drops rows without a time, and marks a single row isolated', () => {
    const s = toChartSeries([{ t: 300000 }, { t: NaN }, { t: 0 }] as Array<{ t: number; isolated?: boolean }>);
    expect(s.rows.map((r) => r.t)).toEqual([0, 300000]);
    const single = toChartSeries([{ t: 5 }] as Array<{ t: number; isolated?: boolean }>);
    expect((single.rows[0] as { isolated?: boolean }).isolated).toBe(true);
    expect(toChartSeries([]).rows).toEqual([]);
  });
});

// ---------------------------------------------------------------------------
// Dataset builders
// ---------------------------------------------------------------------------

const V4 = 'uniswap-v4:base:3163450';
const V3 = 'uniswap-v3:base:6336662';
const AERO = 'aerodrome:base:0x770f1e4d2a8b9c7e6f5a4b3c2d1e0f9a8b7c6d5e';

function position(id: string, protocol: string, fee: number, t0 = 'USDC', t1 = 'NOCK') {
  return { position_id: id, protocol, fee_tier_bps: fee, token0: { symbol: t0 }, token1: { symbol: t1 } };
}

const LATEST_POSITIONS = [
  position(V4, 'uniswap-v4', 300),
  position(V3, 'uniswap-v3', 100, 'WETH'),
  position(AERO, 'aerodrome', 100),
];

/** A stored history row (slimmed: one point per series). */
function historyRow(
  generatedAt: string,
  opts: {
    total?: string;
    native?: string;
    stable?: string;
    walletNock?: string;
    spot?: string;
    positions?: Array<[string, string]>;
    staleCounterfactual?: boolean;
  } = {},
): HistoryEntry {
  return {
    meta: { generated_at: generatedAt },
    data: {
      derived_metrics: {
        global_status: {
          total_portfolio_usd: opts.total ?? '2390.53',
          total_native_usd: opts.native ?? '1312.88',
          total_stable_usd: opts.stable ?? '1033.71',
        },
        asset_aggregation_series: {
          time_points: [generatedAt],
          safe_haven_series: ['1102.25'],
          wallet_nock_series: [opts.walletNock ?? '95598.98'],
          active_protocol_series: ['0'],
          positions_nock: (opts.positions ?? [[V4, '9363.08'], [V3, '10958.96'], [AERO, '0']]).map(([position_id, v]) => ({
            position_id,
            series: [v],
          })),
        },
        total_value_vs_baseline: {
          time_points: [generatedAt],
          portfolio_value_series: [opts.total ?? '2390.53'],
          token_price_series: [opts.spot ?? '0.01153'],
          high_water_mark_series: ['2475.27'],
        },
        // The slimmed payload keeps the OLDEST counterfactual point (~8 h stale, N1).
        strategy_counterfactual: opts.staleCounterfactual
          ? {
              time_points: ['2026-11-14T10:10:57.466343+00:00'],
              active_portfolio_series: ['2474.90'],
              passive_baseline_series: ['1441.90'],
            }
          : undefined,
      },
    },
  };
}

function latestData(extra: any = {}) {
  return {
    positions: LATEST_POSITIONS,
    derived_metrics: {
      global_status: {
        total_portfolio_usd: '2390.56',
        total_native_usd: '1312.98',
        total_stable_usd: '1033.61',
      },
      asset_aggregation_series: {
        time_points: ['2026-11-14T18:36:04.000000Z'],
        safe_haven_series: ['1102.25'],
        wallet_nock_series: ['95598.98'],
        active_protocol_series: ['0'],
        positions_nock: [V4, V3, AERO].map((position_id) => ({ position_id, series: ['1'] })),
      },
      total_value_vs_baseline: {
        time_points: ['2026-11-14T18:36:04.000000Z'],
        portfolio_value_series: ['2390.56'],
        token_price_series: ['0.01153'],
        high_water_mark_series: ['2475.27'],
      },
      ...extra,
    },
  };
}

describe('buildChartRows: from zero', () => {
  it('0 history rows and no snapshot gives empty datasets, never invented points', () => {
    const r = buildChartRows([], undefined);
    for (const s of [r.assetSeries, r.valueVsBaseline, r.composition, r.channel]) {
      expect(s.rows).toEqual([]);
      expect(s.gaps).toEqual([]);
    }
    expect(r.positionSeriesDefs).toEqual([]);
    expect(r.spansMultipleDays).toBe(false);
    expect(r.fromHistory).toBe(false);
  });

  it('1 history row falls back to the latest snapshot, drawn as an isolated point', () => {
    const r = buildChartRows([historyRow('2026-11-14T18:31:04Z')], latestData(), 300, '2026-11-14T18:36:04.000000Z');
    expect(r.fromHistory).toBe(false);
    expect(r.assetSeries.rows).toHaveLength(1);
    expect(r.assetSeries.rows[0]).toMatchObject({ t: Date.parse('2026-11-14T18:36:04Z'), isolated: true, safeHaven: 95599 });
    expect(r.composition.rows).toEqual([
      { t: Date.parse('2026-11-14T18:36:04Z'), active: 2390.56, passive: 1312.98, stable: 1033.61, eth: null, isolated: true },
    ]);
  });

  it('zero balances plot as real zeros, not gaps', () => {
    const zero = (t: string) =>
      historyRow(t, { total: '0.00', native: '0.00', stable: '0.00', walletNock: '0.0000', positions: [] });
    const r = buildChartRows([zero('2026-11-14T18:00:00Z'), zero('2026-11-14T18:05:00Z')], { positions: [], derived_metrics: {} });
    expect(r.fromHistory).toBe(true);
    expect(r.positionSeriesDefs).toEqual([]);
    expect(r.assetSeries.rows.map((row) => (row as any).safeHaven)).toEqual([0, 0]);
    expect(r.assetSeries.rows.map((row) => (row as any).totalNock)).toEqual([0, 0]);
    expect(r.composition.rows.map((row) => (row as any).active)).toEqual([0, 0]);
    expect(r.composition.rows.map((row) => (row as any).stable)).toEqual([0, 0]);
    expect(r.assetSeries.gaps).toEqual([]);
  });

  it('a missing value in a real row is null, not 0', () => {
    const a = historyRow('2026-11-14T18:00:00Z');
    const b = historyRow('2026-11-14T18:05:00Z');
    delete b.data.derived_metrics.total_value_vs_baseline.token_price_series;
    delete b.data.derived_metrics.global_status.total_stable_usd;
    const r = buildChartRows([a, b], latestData());
    expect((r.valueVsBaseline.rows[1] as any).tokenPrice).toBeNull();
    expect((r.composition.rows[1] as any).stable).toBeNull();
  });
});

describe('buildChartRows: 2 rows and a gap', () => {
  it('2 rows a cycle apart: one line, real time on the x axis, no gap', () => {
    const r = buildChartRows(
      [historyRow('2026-11-14T17:41:04.995794Z'), historyRow('2026-11-14T17:46:05.096807Z')],
      latestData(),
    );
    expect(r.fromHistory).toBe(true);
    expect(r.valueVsBaseline.rows.map((row) => row.t)).toEqual([1794678064995, 1794678365096]);
    expect(r.valueVsBaseline.ticks).toEqual([1794678064995, 1794678365096]);
    expect(r.valueVsBaseline.gaps).toEqual([]);
    expect(r.valueVsBaseline.rows.every((row) => !(row as any).isolated)).toBe(true);
  });

  it('a 19h 20m outage breaks every history dataset and is described', () => {
    const r = buildChartRows(
      [historyRow('2026-11-13T05:38:00.000Z', { total: '2475.27' }), historyRow('2026-11-14T00:58:00.000Z')],
      latestData(),
    );
    for (const s of [r.assetSeries, r.valueVsBaseline, r.composition]) {
      expect(s.rows).toHaveLength(3);
      expect(isGapRow(s.rows[1])).toBe(true);
      expect(s.gaps.map((g) => g.time)).toEqual(['no snapshots for 19h 20m']);
      expect(s.ticks).toEqual([1794548280000, 1794617880000]);
    }
    // No series key on the break row, so every Line and stacked Area breaks there.
    const gapRow = r.assetSeries.rows[1] as Record<string, unknown>;
    for (const k of ['safeHaven', 'totalNock', ...r.positionSeriesDefs.map((d) => d.key)]) expect(k in gapRow).toBe(false);
    expect('active' in (r.composition.rows[1] as object)).toBe(false);
    // Both edges of the outage stand alone: they get dots, not a vanishing line.
    expect((r.composition.rows[0] as any).isolated).toBe(true);
    expect((r.composition.rows[2] as any).isolated).toBe(true);
  });

  it('accepts a configured sync window', () => {
    const rows = [historyRow('2026-11-14T18:00:00Z'), historyRow('2026-11-14T18:03:00Z')];
    expect(buildChartRows(rows, latestData(), 60).valueVsBaseline.gaps).toHaveLength(1);
    expect(buildChartRows(rows, latestData(), 300).valueVsBaseline.gaps).toHaveLength(0);
  });
});

describe('buildChartRows: composition from each row\'s own totals (N1)', () => {
  it('ignores the stale slimmed counterfactual point and reads global_status', () => {
    const r = buildChartRows(
      [
        historyRow('2026-11-14T18:01:05.086105Z', { staleCounterfactual: true }),
        historyRow('2026-11-14T18:06:05.086105Z', { staleCounterfactual: true }),
      ],
      latestData(),
    );
    expect(r.composition.rows[1]).toMatchObject({ active: 2390.53, passive: 1312.88, stable: 1033.71, eth: null });
  });

  it('compositionRow reads total_eth_usd when the snapshot carries it', () => {
    const row = compositionRow({ global_status: { total_portfolio_usd: '2413.60', total_native_usd: '1312.88', total_stable_usd: '1033.71', total_eth_usd: '67.01' } }, 1);
    expect(row.eth).toBe(67.01);
    expect(compositionRow({}, 1)).toEqual({ t: 1, active: null, passive: null, stable: null, eth: null });
  });
});

describe('compositionReadout', () => {
  it('reports total_stable_usd, not total minus native (which also holds LP WETH)', () => {
    const r = compositionReadout({ total_portfolio_usd: '2390.53', total_native_usd: '1312.88', total_stable_usd: '1033.71' });
    expect(r.stableUsd).toBe(1033.71);
    expect(r.pctOfNative).toBeCloseTo(78.735, 2);
    expect(r.otherUsd).toBe(43.94);
  });

  it('a USDC-only newcomer has no percentage of native value, not 0.0%', () => {
    const r = compositionReadout({ total_portfolio_usd: '500.00', total_native_usd: '0.00', total_stable_usd: '500.00' });
    expect(r.stableUsd).toBe(500);
    expect(r.pctOfNative).toBeNull();
    expect(r.otherUsd).toBe(0);
  });

  it('unknown fields stay unknown', () => {
    expect(compositionReadout(undefined)).toEqual({ stableUsd: null, nativeUsd: null, totalUsd: null, pctOfNative: null, ethUsd: null, otherUsd: null });
  });

  it('2.3.0: ETH is its own pillar, so nothing is left over and ETH is never called USDC', () => {
    // Live 2026-11-14 figures plus ~0.0125 ETH of wallet gas.
    const r = compositionReadout({
      total_portfolio_usd: '2413.57', total_native_usd: '1312.88', total_stable_usd: '1033.71', total_eth_usd: '66.98',
    });
    expect(r.ethUsd).toBe(66.98);
    expect(r.stableUsd).toBe(1033.71);
    expect(r.otherUsd).toBe(0);
  });
});

describe('LP series keyed by position_id (N15)', () => {
  // Live row 16:14:59.513104Z, from before the retired v4 ranges were dropped.
  const OLD_LIST: Array<[string, string]> = [
    ['uniswap-v4:base:3163431', '0'],
    ['uniswap-v4:base:3163432', '0'],
    [V4, '9363.08'],
    [V3, '10958.96'],
    ['aerodrome:base:0x85f1aa00bb11cc22dd33ee44ff5566778899aabb', '0'],
  ];

  it('keeps each position under its own series when the tracked list changes', () => {
    const r = buildChartRows(
      [
        historyRow('2026-11-14T16:14:59.513104Z', { positions: OLD_LIST }),
        historyRow('2026-11-14T16:19:59.513104Z'),
      ],
      latestData(),
    );
    const key = (id: string) => r.positionSeriesDefs.find((d) => d.positionId === id)!.key;
    const old = r.assetSeries.rows[0] as any;
    expect(old[key(V4)]).toBe(9363);
    expect(old[key(V3)]).toBe(10959);
    expect(key(AERO) in old).toBe(false); // not tracked then: a break, not a 0
    expect(old.totalNock).toBe(115921); // matches the row's own total_nock_amount
    const now = r.assetSeries.rows[1] as any;
    expect(now[key(V4)]).toBe(9363);
    expect(now[key(AERO)]).toBe(0);
    // Retired and never holding NOCK in history: no band at all.
    expect(r.positionSeriesDefs.some((d) => d.positionId === 'uniswap-v4:base:3163431')).toBe(false);
  });

  it('lists current positions first with their labels, then retired ids that held NOCK', () => {
    // The retired #3163432 held NOCK in an earlier row; #3163431 and the Aero pool only
    // ever read 0, so they get no flat-zero band.
    const defs = buildPositionSeriesDefs(latestData().derived_metrics, LATEST_POSITIONS, [
      historyRow('2026-11-14T15:00:00Z', { positions: [['uniswap-v4:base:3163432', '4200.00']] }),
      historyRow('2026-11-14T16:14:59Z', { positions: OLD_LIST }),
    ]);
    expect(defs.map((d) => d.key)).toEqual(['lp0', 'lp1', 'lp2', 'lp3']);
    expect(defs.slice(0, 3).map((d) => [d.positionId, d.retired])).toEqual([[V4, false], [V3, false], [AERO, false]]);
    expect(defs[0].name).toBe('USDC/NOCK v4 · 3%');
    expect(defs.slice(3).map((d) => d.name)).toEqual(['uniswap-v4 #3163432 (no longer tracked)']);
    expect(defs[3].retired).toBe(true);
  });

  it('keeps a current position whose series reads 0 (it is tracked, not retired)', () => {
    const defs = buildPositionSeriesDefs(latestData().derived_metrics, LATEST_POSITIONS, [
      historyRow('2026-11-14T16:14:59Z', { positions: [[AERO, '0']] }),
    ]);
    expect(defs.map((d) => d.positionId)).toEqual([V4, V3, AERO]);
  });
});

describe('positions with no NOCK leg (#151)', () => {
  const GAS = 'uniswap-v3:base:6607225';
  const GAS_CL = 'aerodrome-cl:base:3449558';
  const withGas = [...LATEST_POSITIONS, position(GAS, 'uniswap-v3', 5, 'WETH', 'USDC'), position(GAS_CL, 'aerodrome-cl', 0, 'WETH', 'USDC')];

  it('skips a current WETH/USDC range even when an older snapshot sent it a 0 series', () => {
    const latest = latestData().derived_metrics;
    const pre = {
      ...latest,
      asset_aggregation_series: {
        ...latest.asset_aggregation_series,
        positions_nock: [V4, V3, AERO, GAS, GAS_CL].map((position_id) => ({ position_id, series: ['0'] })),
      },
    };
    const defs = buildPositionSeriesDefs(pre, withGas, [
      historyRow('2026-11-17T12:00:00Z', { positions: [[V4, '9363.08'], [GAS, '0']] }),
    ]);
    expect(defs.map((d) => d.positionId)).toEqual([V4, V3, AERO]);
    expect(defs.every((d) => !d.retired)).toBe(true);
  });

  it('drops a retired WETH/USDC range whose pre-2.8.2 history is all 0', () => {
    const RETIRED_GAS = 'uniswap-v3:base:6600001';
    const defs = buildPositionSeriesDefs(latestData().derived_metrics, LATEST_POSITIONS, [
      historyRow('2026-11-17T12:00:00Z', { positions: [[V4, '9363.08'], [RETIRED_GAS, '0']] }),
      historyRow('2026-11-17T12:05:00Z', { positions: [[V4, '9363.08'], [RETIRED_GAS, '0.00']] }),
    ]);
    expect(defs.map((d) => d.positionId)).toEqual([V4, V3, AERO]);
  });

  it('keeps an id seen only in history: no current position says it has no NOCK leg', () => {
    const defs = buildPositionSeriesDefs(latestData().derived_metrics, withGas, [
      historyRow('2026-11-14T16:14:59Z', { positions: [['uniswap-v4:base:3163431', '5.00']] }),
    ]);
    expect(defs.map((d) => d.positionId)).toEqual([V4, V3, AERO, 'uniswap-v4:base:3163431']);
    expect(defs[3].retired).toBe(true);
  });
});

describe('structural channel', () => {
  it('uses real times and breaks at gaps', () => {
    const pts = (vals: string[]) =>
      ['2026-11-14T10:00:00Z', '2026-11-14T10:05:00Z', '2026-11-14T13:00:00Z'].map((time, i) => ({ time, value: vals[i] }));
    const r = buildChartRows([], latestData({
      structural_channels: {
        channels: [{
          center_line: pts(['100', '101', '102']),
          upper_1sd: pts(['110', '111', '112']),
          lower_1sd: pts(['90', '91', '92']),
          upper_2sd: pts(['120', '121', '122']),
          lower_2sd: pts(['80', '', '82']),
        }],
      },
    }));
    expect(r.channel.rows.map((row) => (isGapRow(row) ? 'gap' : row.t))).toEqual([
      Date.parse('2026-11-14T10:00:00Z'),
      Date.parse('2026-11-14T10:05:00Z'),
      'gap',
      Date.parse('2026-11-14T13:00:00Z'),
    ]);
    expect((r.channel.rows[1] as any).lower2).toBeNull();
    expect((r.channel.rows[3] as any).isolated).toBe(true);
  });
});
