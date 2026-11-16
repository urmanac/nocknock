import { describe, it, expect } from 'vitest';
import {
  COST_PANEL_TITLE,
  buildCostChart,
  costBasisView,
  differenceView,
  fillRows,
  priceText,
  spotProvenanceText,
  stepLine,
  type CostBasis,
  type CostBasisSeries,
} from './costBasis';
import { isGapRow } from './timeline';

/**
 * The plan-buys figures are the worker's output for the operator's three real fills
 * (ingestion-worker/src/tests_cost_basis.rs; receipts in ONCHAIN_INTEGRATION.md §12).
 * The opening-basis figures (10,000 NOCK for $200) are made up for the tests: round
 * numbers that are nobody's records.
 */
const planOnly: CostBasis = {
  spot_price_usd: '0.011530',
  plan_buys: {
    status: 'ok',
    reason: null,
    fills: 3,
    spent_usdc: '60.00',
    acquired_nock: '5098.5115',
    avg_cost_usd: '0.011768',
    first_fill_at: '2026-11-14T06:02:25+00:00',
    last_fill_at: '2026-11-16T03:08:37+00:00',
    value_at_spot_usd: '58.79',
    unrealized_usd: '-1.21',
    unrealized_pct: '-2.02',
    plans: [
      {
        plan_id: '2026-11-reentry',
        fills: 3,
        spent_usdc: '60.00',
        acquired_nock: '5098.5115',
        avg_cost_usd: '0.011768',
        first_fill_at: '2026-11-14T06:02:25+00:00',
        last_fill_at: '2026-11-16T03:08:37+00:00',
      },
    ],
    includes: ['Every receipt-verified buy in the accumulation ledger, across all plans.'],
    excludes: ['NOCK held before the first recorded buy.', 'Gas paid in ETH.'],
  },
  whole_stack: { status: 'not_configured', reason: 'cost_basis.opening is not set in config.' },
};

const withOpening: CostBasis = {
  ...planOnly,
  whole_stack: {
    status: 'ok',
    reason: null,
    source: 'test figures',
    as_of: '2026-11-14T12:00:00+00:00',
    opening_nock: '10000.0000',
    opening_cost_usd: '200.00',
    opening_avg_cost_usd: '0.020000',
    fills_since: 2,
    bought_since_usdc: '40.00',
    bought_since_nock: '3500.5774',
    basis_nock: '13500.5774',
    basis_cost_usd: '240.00',
    avg_cost_usd: '0.017777',
    value_at_spot_usd: '155.66',
    unrealized_usd: '-84.34',
    unrealized_pct: '-35.14',
    // A whole number of NOCK: the precision the snapshot's own total has.
    held_nock: '12000',
    includes: ['The opening figures entered in config: 10000.0000 NOCK for $200.00, source: test figures.'],
    excludes: ['Everything after the as-of that is not a recorded buy: sells, LP ranges, fees earned.'],
  },
};

const series = (over: Partial<CostBasisSeries> = {}): CostBasisSeries => ({
  generated_at: '2026-11-16T04:00:00+00:00',
  from: '2026-11-14T00:00:00+00:00',
  to: '2026-11-16T04:00:00+00:00',
  bucket_seconds: 900,
  spot_match_window_minutes: 60,
  spot: [
    { at: '2026-11-14T06:00:00+00:00', price_usd: '0.01240000', source: 'uniswap_rpc', is_backfilled: false },
    { at: '2026-11-14T06:15:00+00:00', price_usd: '0.01250000', source: 'uniswap_rpc', is_backfilled: false },
    { at: '2026-11-14T06:30:00+00:00', price_usd: '0.01245000', source: 'uniswap_rpc', is_backfilled: false },
    // 20 hours with nothing stored, then one lone backfilled candle, then live points again.
    { at: '2026-11-15T03:00:00+00:00', price_usd: '0.01170000', source: 'geckoterminal', is_backfilled: true },
    { at: '2026-11-16T03:00:00+00:00', price_usd: '0.01120000', source: 'uniswap_rpc', is_backfilled: false },
    { at: '2026-11-16T03:15:00+00:00', price_usd: '0.01125000', source: 'uniswap_rpc', is_backfilled: false },
  ],
  spot_error: null,
  fills: [
    {
      plan_id: '2026-11-reentry',
      tx_hash: '0x5d0c02d9929f49f71526e0c8d91d1bd99ec19f116befc285d67f6bf9412a3f0e',
      block_number: 57228848,
      confirmed_at: '2026-11-14T06:02:25+00:00',
      usdc_spent: '20',
      nock_received: '1597.9340475636968138',
      effective_price_usd: '0.01251616',
      plan_avg_cost_usd: '0.01251616',
      avg_cost_usd: '0.01251616',
      whole_stack_avg_cost_usd: null,
      spot_at_fill: { at: '2026-11-14T06:00:00+00:00', price_usd: '0.01240000', source: 'uniswap_rpc', is_backfilled: false },
    },
    {
      plan_id: '2026-11-reentry',
      tx_hash: '0x4644edd99bb2770f369862357eb5647a68a7157ca9e91ecccfce79e6964c61e8',
      block_number: 57605753,
      confirmed_at: '2026-11-15T02:58:46+00:00',
      usdc_spent: '20',
      nock_received: '1720.2181466216939446',
      effective_price_usd: '0.01162643',
      plan_avg_cost_usd: '0.01205490',
      avg_cost_usd: '0.01205490',
      whole_stack_avg_cost_usd: null,
      spot_at_fill: { at: '2026-11-15T03:00:00+00:00', price_usd: '0.01170000', source: 'geckoterminal', is_backfilled: true },
    },
    {
      plan_id: '2026-11-reentry',
      tx_hash: '0x78376f3cc9e80f2eb124a945a452df40556649e3d9983be526b2bd97586e31b9',
      block_number: 58040708,
      confirmed_at: '2026-11-16T03:08:37+00:00',
      usdc_spent: '20',
      nock_received: '1780.3592976960049083',
      effective_price_usd: '0.01123369',
      plan_avg_cost_usd: '0.01176814',
      avg_cost_usd: '0.01176814',
      whole_stack_avg_cost_usd: null,
      spot_at_fill: null,
    },
  ],
  whole_stack: { status: 'not_configured', reason: null, opening: null },
  ...over,
});

const withOpeningSeries = (): CostBasisSeries => {
  const s = series();
  s.fills[1].whole_stack_avg_cost_usd = '0.01877098';
  s.fills[2].whole_stack_avg_cost_usd = '0.01777702';
  s.whole_stack = {
    status: 'ok',
    reason: null,
    opening: { as_of: '2026-11-14T12:00:00+00:00', nock: '10000.0000', cost_usd: '200.00', avg_cost_usd: '0.02000000', source: 'test figures' },
  };
  return s;
};

const ms = (iso: string) => Date.parse(iso);

/** Words that would turn a measurement into advice (lore: "Why not buy more now?"). */
const advice = /buy more|average down|averaging down|should|opportunit|cheap|discount|bargain|accumulate|recommend|consider|underwater|\bprofit\b|\bloss\b/i;

/** Claims about what NockNock does not read: it tracks no sells and no LP activity. */
const unreadClaim = /nothing was sold|not sold|never sold|still h(?:e|o)ld|\bunrealized\b/i;

const allText = (v: ReturnType<typeof costBasisView>) =>
  [v.title, v.configError ?? '', ...v.notes, ...v.tiles.flatMap((t) => [t.label, t.value, t.sub])].join(' | ');

describe('cost basis summary (#98)', () => {
  it('states the plan-buys average and the difference at spot as facts', () => {
    const v = costBasisView(planOnly);
    expect(v.title).toBe(COST_PANEL_TITLE);
    expect(v.title).toBe('Average cost vs spot');
    const [plan, spot, diff, whole] = v.tiles;
    expect(plan).toMatchObject({ label: 'PLAN BUYS · AVERAGE COST', value: '$0.01177' });
    expect(plan.sub).toBe('3 verified buys · $60.00 for 5,098.51 NOCK');
    expect(spot).toMatchObject({ label: 'SPOT', value: '$0.01153' });
    expect(diff.label).toBe('PLAN BUYS · VALUE AT SPOT LESS COST');
    expect(diff.value).toBe('−$1.21 (−2.02%)');
    expect(diff.tone).toBe('below');
    expect(diff.sub).toBe(
      '5,098.51 NOCK is worth $58.79 at spot $0.01153; it cost $60.00. This values the NOCK these buys acquired; NockNock does not track whether it was later sold or moved into LP.',
    );
    expect(whole).toMatchObject({ label: 'WHOLE STACK · AVERAGE COST', value: 'not configured' });
    expect(whole.sub).toMatch(/cannot derive this from the chain/);
    expect(whole.sub).toMatch(/cost_basis\.opening/);
    expect(v.configError).toBeNull();
    expect(allText(v)).not.toMatch(advice);
    // Sells are not read, so no text may say the NOCK was kept.
    expect(allText(v)).not.toMatch(unreadClaim);
  });

  it('with an opening basis, the whole-stack tile names the source, the instant and the parts', () => {
    const v = costBasisView(withOpening);
    const whole = v.tiles[3];
    expect(whole.value).toBe('$0.01778');
    expect(whole.sub).toBe(
      'On 13,500.58 NOCK costing $240.00: your opening basis (10,000 NOCK for $200.00, source: test figures, as of 2026-11-14T12:00:00+00:00) plus 2 verified buys since. At spot that NOCK is worth $155.66: −$84.34 (−35.14%) against its cost. NockNock reads 12,000 NOCK held now (wallet plus tracked LP).',
    );
    expect(v.notes.some((n) => n.startsWith('Whole stack counts:'))).toBe(true);
    expect(v.notes.some((n) => n.startsWith('Whole stack leaves out:') && /sells/.test(n))).toBe(true);
    expect(v.notes.some((n) => n.startsWith('Plan buys leave out:') && /Gas paid in ETH/.test(n))).toBe(true);
    expect(allText(v)).not.toMatch(advice);
    expect(allText(v)).not.toMatch(unreadClaim);
  });

  it('a configuration error is shown with its reason and no whole-stack number', () => {
    const reason = 'cost_basis.opening cannot be used: cost_usd is missing (what that NOCK cost you in total, in USD).';
    const v = costBasisView({ ...planOnly, whole_stack: { status: 'invalid', reason } });
    expect(v.configError).toBe(reason);
    expect(v.tiles[3]).toMatchObject({ value: 'configuration error', sub: reason });
    expect(v.tiles[0].value).toBe('$0.01177');
  });

  it('no buys and no spot are each said plainly, never as $0', () => {
    const none = costBasisView({
      spot_price_usd: '0.011530',
      plan_buys: { status: 'no_fills', fills: 0 },
      whole_stack: { status: 'not_configured' },
    });
    expect(none.tiles[0]).toMatchObject({ value: '—', sub: 'No verified buys recorded yet' });
    expect(none.tiles[2]).toMatchObject({ value: '—', sub: 'Needs at least one verified buy' });

    const noSpot = costBasisView({
      spot_price_usd: null,
      plan_buys: { ...planOnly.plan_buys, value_at_spot_usd: null, unrealized_usd: null, unrealized_pct: null },
      whole_stack: { status: 'not_configured' },
    });
    expect(noSpot.tiles[1].value).toBe('—');
    expect(noSpot.tiles[2]).toMatchObject({ value: '—', sub: 'No spot price in this snapshot, so no comparison' });
    for (const v of [none, noSpot]) expect(allText(v)).not.toMatch(/\$0\.00/);
  });

  it('an unread ledger is "unavailable" on both plan-buys tiles, never "no buys"', () => {
    const reason = 'the accumulation ledger could not be read this cycle';
    const unread = costBasisView({
      spot_price_usd: '0.011530',
      // fills is null in this state: the count was not read.
      plan_buys: { status: 'unavailable', reason, fills: null },
      whole_stack: { status: 'unavailable', reason, source: 'x' },
    });
    expect(unread.tiles[0]).toMatchObject({ label: 'PLAN BUYS · AVERAGE COST', value: 'unavailable', sub: reason });
    expect(unread.tiles[2]).toMatchObject({ label: 'PLAN BUYS · VALUE AT SPOT LESS COST', value: 'unavailable', sub: reason });
    expect(unread.tiles[3].value).toBe('unavailable');
    // Nothing on the panel reads as "there are no buys" or as a zero.
    expect(allText(unread)).not.toMatch(/verified buy|\$0\.00/i);

    // Unavailable with no reason given, and a snapshot with no plan-buys block at all.
    const noReason = costBasisView({ plan_buys: { status: 'unavailable' }, whole_stack: { status: 'not_configured' } });
    expect(noReason.tiles[2]).toMatchObject({ value: 'unavailable', sub: 'no reason reported' });
    const noBlock = costBasisView({ whole_stack: { status: 'not_configured' } } as unknown as CostBasis);
    expect(noBlock.tiles[0]).toMatchObject({ value: '—', sub: 'not reported by this snapshot' });
    expect(noBlock.tiles[2]).toMatchObject({ value: '—', sub: 'not reported by this snapshot' });
  });

  it('a snapshot without the block is not reported, with no tiles', () => {
    for (const cb of [undefined, null]) {
      const v = costBasisView(cb);
      expect(v.tiles).toEqual([]);
      expect(v.notes[0]).toMatch(/not reported by this snapshot/);
    }
  });

  it('difference text is signed from the worker figure', () => {
    expect(differenceView('-1.21', '-2.02')).toEqual({ text: '−$1.21 (−2.02%)', tone: 'below' });
    expect(differenceView('41.97', '69.95')).toEqual({ text: '+$41.97 (+69.95%)', tone: 'above' });
    expect(differenceView('1234.5', null)).toEqual({ text: '+$1,234.50', tone: 'above' });
    expect(differenceView('-0.00', '-0.00')).toEqual({ text: '$0.00', tone: 'even' });
    expect(differenceView(null, null)).toEqual({ text: '—', tone: 'none' });
    expect(differenceView('', '1')).toEqual({ text: '—', tone: 'none' });
    expect(priceText(undefined)).toBe('—');
    expect(priceText('0.0115296')).toBe('$0.01153');
  });
});

describe('average cost vs spot chart (#98)', () => {
  it('breaks the spot line at gaps and never adds a point', () => {
    const chart = buildCostChart(series())!;
    expect(chart.domain).toEqual([ms('2026-11-14T00:00:00Z'), ms('2026-11-16T04:00:00Z')]);
    expect(chart.ticks).toEqual([
      ms('2026-11-14T00:00:00Z'),
      ms('2026-11-14T13:00:00Z'),
      ms('2026-11-15T02:00:00Z'),
      ms('2026-11-15T15:00:00Z'),
      ms('2026-11-16T04:00:00Z'),
    ]);
    const real = chart.spot.filter((r) => !isGapRow(r)) as Array<{ t: number; spot: number; isolated?: boolean }>;
    // Exactly the six stored points, at their own times and prices.
    expect(real.map((r) => r.t)).toEqual(series().spot!.map((p) => ms(p.at)));
    expect(real.map((r) => r.spot)).toEqual([0.0124, 0.0125, 0.01245, 0.0117, 0.0112, 0.01125]);
    // Two gaps: 06:30 → 03:00 next day (20h 30m) and 03:00 → 03:00 (24h).
    expect(chart.gaps.map((g) => g.time)).toEqual(['no stored price for 20h 30m', 'no stored price for 1d 0h']);
    expect(chart.spot.filter(isGapRow).length).toBe(2);
    // The lone candle between the gaps is isolated (drawn as a dot); the runs are not.
    expect(real.map((r) => r.isolated)).toEqual([false, false, false, true, false, false]);
    expect(chart.spotPoints).toBe(6);
    expect(chart.backfilledPoints).toBe(1);
    expect(chart.spotNote).toBeNull();
    expect(spotProvenanceText(chart)).toBe(
      '6 stored spot points · 5 read by the worker, 1 backfilled from third-party candles · 2 gaps shown, not filled.',
    );
  });

  it('points 2.5 buckets apart or closer stay connected', () => {
    const s = series({
      bucket_seconds: 900,
      spot: [
        { at: '2026-11-14T06:00:00+00:00', price_usd: '0.0124', source: 'uniswap_rpc', is_backfilled: false },
        { at: '2026-11-14T06:37:30+00:00', price_usd: '0.0125', source: 'uniswap_rpc', is_backfilled: false }, // exactly 2.5 buckets
        { at: '2026-11-14T07:15:01+00:00', price_usd: '0.0126', source: 'uniswap_rpc', is_backfilled: false }, // one second more
      ],
    });
    const chart = buildCostChart(s)!;
    expect(chart.gaps.length).toBe(1);
    expect(chart.gaps[0].gapStart).toBe(ms('2026-11-14T06:37:30Z'));
  });

  it('draws the plan-buys average as steps at each fill, carried to the end of the range', () => {
    const chart = buildCostChart(series())!;
    expect(chart.planAvg).toEqual([
      { t: ms('2026-11-14T06:02:25Z'), v: 0.01251616 },
      { t: ms('2026-11-15T02:58:46Z'), v: 0.0120549 },
      { t: ms('2026-11-16T03:08:37Z'), v: 0.01176814 },
      { t: ms('2026-11-16T04:00:00Z'), v: 0.01176814 },
    ]);
    // Every fill is marked at its own time and the price actually paid.
    expect(chart.fills.map((f) => [f.t, f.price])).toEqual([
      [ms('2026-11-14T06:02:25Z'), 0.01251616],
      [ms('2026-11-15T02:58:46Z'), 0.01162643],
      [ms('2026-11-16T03:08:37Z'), 0.01123369],
    ]);
    expect(chart.wholeAvg).toEqual([]);
    const [lo, hi] = chart.yDomain!;
    expect(lo).toBeLessThan(0.0112);
    expect(hi).toBeGreaterThan(0.01251616);
    expect(lo).toBeGreaterThan(0.010);
  });

  it('draws the whole-stack line only when an opening basis is set, starting at its as-of', () => {
    const chart = buildCostChart(withOpeningSeries())!;
    expect(chart.wholeAvg).toEqual([
      { t: ms('2026-11-14T12:00:00Z'), v: 0.02 },
      { t: ms('2026-11-15T02:58:46Z'), v: 0.01877098 },
      { t: ms('2026-11-16T03:08:37Z'), v: 0.01777702 },
      { t: ms('2026-11-16T04:00:00Z'), v: 0.01777702 },
    ]);
    expect(chart.yDomain![1]).toBeGreaterThan(0.02);
    // An invalid or absent opening basis draws nothing for the whole stack, even if a
    // fill carried a stray value.
    const stray = withOpeningSeries();
    stray.whole_stack = { status: 'invalid', reason: 'bad', opening: null };
    expect(buildCostChart(stray)!.wholeAvg).toEqual([]);
  });

  it('a range that starts after the first fills carries the average in force, and marks only fills in range', () => {
    const chart = buildCostChart(series({ from: '2026-11-15T12:00:00+00:00' }))!;
    expect(chart.planAvg).toEqual([
      { t: ms('2026-11-15T12:00:00Z'), v: 0.0120549 },
      { t: ms('2026-11-16T03:08:37Z'), v: 0.01176814 },
      { t: ms('2026-11-16T04:00:00Z'), v: 0.01176814 },
    ]);
    expect(chart.fills.length).toBe(1);
  });

  it('stepLine: empty before any change, flat when every change precedes the range', () => {
    expect(stepLine([], 0, 100)).toEqual([]);
    expect(stepLine([{ t: 200, v: 1 }], 0, 100)).toEqual([]);
    expect(stepLine([{ t: -5, v: 2 }], 0, 100)).toEqual([
      { t: 0, v: 2 },
      { t: 100, v: 2 },
    ]);
    expect(stepLine([{ t: 100, v: 3 }], 0, 100)).toEqual([{ t: 100, v: 3 }]);
    expect(stepLine([{ t: 10, v: NaN }, { t: 20, v: 4 }], 0, 100)).toEqual([
      { t: 20, v: 4 },
      { t: 100, v: 4 },
    ]);
  });

  it('an unreadable spot history says so and still draws the buys', () => {
    const chart = buildCostChart(series({ spot: null, spot_error: 'the stored price history could not be read' }))!;
    expect(chart.spot).toEqual([]);
    expect(chart.spotNote).toBe('Spot history unavailable: the stored price history could not be read.');
    expect(chart.planAvg.length).toBe(4);
    expect(spotProvenanceText(chart)).toBe(chart.spotNote);
    const empty = buildCostChart(series({ spot: [], fills: [] }))!;
    expect(empty.spotNote).toBe('No stored spot price in this range.');
    expect(empty.yDomain).toBeNull();
    expect(empty.planAvg).toEqual([]);
  });

  it('a spot point that does not parse is left out, never drawn at 0', () => {
    const chart = buildCostChart(
      series({
        spot: [
          { at: '2026-11-14T06:00:00+00:00', price_usd: '', source: 'uniswap_rpc', is_backfilled: false },
          { at: 'not a time', price_usd: '0.0124', source: 'uniswap_rpc', is_backfilled: false },
          { at: '2026-11-14T06:15:00+00:00', price_usd: '0', source: 'uniswap_rpc', is_backfilled: false },
          { at: '2026-11-14T06:30:00+00:00', price_usd: '0.0125', source: 'uniswap_rpc', is_backfilled: false },
        ],
      }),
    )!;
    expect(chart.spotPoints).toBe(1);
    expect(chart.yDomain![0]).toBeGreaterThan(0.01);
  });

  it('no series, or a range that does not parse, draws nothing', () => {
    expect(buildCostChart(null)).toBeNull();
    expect(buildCostChart(undefined)).toBeNull();
    expect(buildCostChart(series({ from: 'x' }))).toBeNull();
    expect(buildCostChart(series({ from: '2026-11-16T04:00:00+00:00' }))).toBeNull();
    expect(spotProvenanceText(null)).toBe('');
  });
});

describe('fill table (#98)', () => {
  it('shows what each buy did to the average, and the stored spot beside it', () => {
    const rows = fillRows(series());
    expect(rows.map((r) => [r.spent, r.nock, r.price])).toEqual([
      ['$20.00', '1,597.93', '$0.01252'],
      ['$20.00', '1,720.22', '$0.01163'],
      ['$20.00', '1,780.36', '$0.01123'],
    ]);
    expect(rows.map((r) => `${r.avgBefore} → ${r.avgAfter}`)).toEqual([
      '— → $0.01252',
      '$0.01252 → $0.01205',
      '$0.01205 → $0.01177',
    ]);
    expect(rows.map((r) => r.spotAtFill)).toEqual(['$0.01240', '$0.01170 (backfilled)', 'none stored within 60 min']);
    expect(rows.every((r) => r.wholeAfter === '—')).toBe(true);
    expect(rows[0].planId).toBe('2026-11-reentry');
  });

  it('whole-stack column carries the blended average only for fills after the as-of', () => {
    expect(fillRows(withOpeningSeries()).map((r) => r.wholeAfter)).toEqual(['—', '$0.01877', '$0.01778']);
  });

  it('with spot history unavailable the spot column says unavailable; no series is no rows', () => {
    expect(fillRows(series({ spot: null })).map((r) => r.spotAtFill)[2]).toBe('unavailable');
    expect(fillRows(null)).toEqual([]);
  });
});
