/**
 * Gaps in the "Average cost vs spot" chart (issue #137). The shape is the deployment's
 * after the v0.6.0 to v0.8.0 deploy (2026-11-16): 33 hourly third-party candles backfilled
 * for 2026-11-12 to 2026-11-13, 3600 s apart, then the worker's own reads, 300 s apart.
 * The series is served with a 900 s bucket (a five-day range, at most 600 points).
 */
import { describe, it, expect } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import {
  GAP_LABEL_MIN_FRACTION,
  buildCostChart,
  candlePeriodMs,
  spotChartSeries,
  spotProvenanceText,
  type CostBasisSeries,
  type SpotRow,
} from './costBasis';
import { CostChartSvg } from './components/AverageCostPanel';
import { isGapRow } from './timeline';

const HOUR = 3_600_000;
const iso = (t: number) => new Date(t).toISOString().replace('.000Z', '+00:00');
const START = Date.parse('2026-11-12T00:00:00Z');

/** 33 hourly candles from START, then worker reads every 300 s from `readsFrom` to `readsTo`. */
function points(readsFrom: number, readsTo: number, skip: (t: number) => boolean = () => false) {
  const out: CostBasisSeries['spot'] = [];
  for (let i = 0; i < 33; i++) {
    const t = START + i * HOUR;
    if (!skip(t)) out!.push({ at: iso(t), price_usd: (0.0118 + i * 0.00001).toFixed(6), source: 'geckoterminal', is_backfilled: true });
  }
  for (let t = readsFrom; t <= readsTo; t += 300_000) {
    if (!skip(t)) out!.push({ at: iso(t), price_usd: '0.011530', source: 'uniswap_rpc', is_backfilled: false });
  }
  return out;
}

const series = (spot: CostBasisSeries['spot']): CostBasisSeries => ({
  generated_at: '2026-11-17T04:00:00+00:00',
  from: '2026-11-12T00:00:00+00:00',
  to: '2026-11-17T04:00:00+00:00',
  bucket_seconds: 900,
  spot_match_window_minutes: 60,
  spot,
  fills: [],
  whole_stack: { status: 'not_configured' },
});

const LAST_CANDLE = START + 32 * HOUR; // 2026-11-13T08:00Z
const FIRST_READ = LAST_CANDLE + 20 * 60_000; // the worker started 20 minutes later

describe('average cost chart gaps (#137)', () => {
  it('reads hourly candles as their own cadence, not as 33 gaps', () => {
    const chart = buildCostChart(series(points(FIRST_READ, Date.parse('2026-11-17T04:00:00Z'))))!;
    expect(candlePeriodMs(chart.spot.filter((r) => !isGapRow(r)) as SpotRow[])).toBe(HOUR);
    expect(chart.gaps).toEqual([]);
    expect(chart.spot.filter(isGapRow)).toEqual([]);
    // One caption for the whole backfilled stretch.
    expect(chart.backfill).toEqual([{ start: START, end: LAST_CANDLE, caption: 'hourly third-party candles' }]);
    expect(chart.backfilledPoints).toBe(33);
    expect(spotProvenanceText(chart)).not.toMatch(/gap/);
    expect(spotProvenanceText(chart)).toMatch(/33 backfilled from third-party candles\.$/);
  });

  it('still breaks at real holes, in the candles and in the reads, and counts only those', () => {
    const outage = (t: number) => t > Date.parse('2026-11-15T10:00:00Z') && t < Date.parse('2026-11-15T13:00:00Z');
    const missingCandles = (t: number) => t > START + 10 * HOUR && t < START + 16 * HOUR; // 5 candles missing
    const chart = buildCostChart(
      series(points(FIRST_READ, Date.parse('2026-11-17T04:00:00Z'), (t) => outage(t) || missingCandles(t))),
    )!;
    expect(chart.gaps.map((g) => g.time)).toEqual(['no stored price for 6h 0m', 'no stored price for 3h 0m']);
    expect(chart.gaps.every((g) => g.labelled)).toBe(true);
    expect(spotProvenanceText(chart)).toMatch(/· 2 gaps shown, not filled\.$/);
    // The candles either side of the hole are two stretches, each captioned once.
    expect(chart.backfill.map((b) => [b.start, b.end])).toEqual([
      [START, START + 10 * HOUR],
      [START + 16 * HOUR, LAST_CANDLE],
    ]);
    // Worker reads 300 s apart are never a gap at a 900 s bucket.
    const reads = spotChartSeries(
      [0, 1, 2, 3].map((i) => ({ t: FIRST_READ + i * 300_000, spot: 0.0115, backfilled: false })),
      900,
    );
    expect(reads.gaps).toEqual([]);
  });

  it('never overprints labels: close gaps are shaded, and only spaced ones are labelled', () => {
    // A worker that restarted every 40 minutes for a few hours: many short real gaps.
    const rows: SpotRow[] = [];
    for (let i = 0; i < 12; i++) rows.push({ t: FIRST_READ + i * 40 * 60_000, spot: 0.0115, backfilled: false });
    const domain: [number, number] = [START, Date.parse('2026-11-17T04:00:00Z')];
    const { gaps } = spotChartSeries(rows, 300, domain);
    expect(gaps.length).toBe(11);
    const labelled = gaps.filter((g) => g.labelled);
    expect(labelled.length).toBe(1);
    for (let i = 1; i < labelled.length; i++) {
      expect(labelled[i].gapStart - labelled[i - 1].gapStart).toBeGreaterThanOrEqual((domain[1] - domain[0]) * GAP_LABEL_MIN_FRACTION);
    }
    // Rendered: one gap caption, one candle caption.
    const chart = buildCostChart(
      series([
        ...points(FIRST_READ, FIRST_READ),
        ...rows.map((r) => ({ at: iso(r.t), price_usd: '0.011500', source: 'uniswap_rpc', is_backfilled: false })),
      ]),
    )!;
    const html = renderToStaticMarkup(<CostChartSvg chart={chart} width={800} height={280} />);
    expect(html.split('no stored price for').length - 1).toBe(chart.gaps.filter((g) => g.labelled).length);
    expect(html.split('hourly third-party candles').length - 1).toBe(1);
  });

  describe('mixed daily and hourly candles (review of #137)', () => {
    const DAY = 24 * HOUR;
    const candle = (t: number): SpotRow => ({ t, spot: 0.0118, backfilled: true });
    const daily = (n: number, from: number) => Array.from({ length: n }, (_, i) => candle(from + i * DAY));

    it('reads each candle against its own period: daily steps are not gaps', () => {
      const days = daily(20, START - 20 * DAY);
      const hours = Array.from({ length: 30 }, (_, i) => candle(START + i * HOUR));
      const { gaps, backfill } = spotChartSeries([...days, ...hours], 900);
      expect(gaps).toEqual([]);
      expect(backfill.map((b) => b.caption)).toEqual(['daily third-party candles', 'hourly third-party candles']);
      // The first hourly candle is a day after the last daily one, so it closes the daily stretch.
      expect(backfill[0]).toMatchObject({ start: START - 20 * DAY, end: START });
      expect(backfill[1]).toMatchObject({ start: START + HOUR, end: START + 29 * HOUR });
    });

    it('still breaks at a two-day hole between hourly candles when daily candles dominate', () => {
      const days = daily(30, START - 30 * DAY);
      const hours = [
        ...Array.from({ length: 5 }, (_, i) => candle(START + i * HOUR)),
        ...Array.from({ length: 5 }, (_, i) => candle(START + 4 * HOUR + 2 * DAY + i * HOUR)),
      ];
      const { gaps } = spotChartSeries([...days, ...hours], 900);
      expect(gaps.map((g) => g.time)).toEqual(['no stored price for 2d 0h']);
    });

    it('breaks between the last candle and a worker read that comes well after it', () => {
      const hours = Array.from({ length: 10 }, (_, i) => candle(START + i * HOUR));
      const lastCandle = START + 9 * HOUR;
      const late = { t: lastCandle + 140 * 60_000, spot: 0.0115, backfilled: false };
      expect(spotChartSeries([...hours, late], 900).gaps.map((g) => g.time)).toEqual(['no stored price for 2h 20m']);
      // A read inside the last candle's own hour plus one bucket rule is not a gap.
      const soon = { t: lastCandle + 80 * 60_000, spot: 0.0115, backfilled: false };
      expect(spotChartSeries([...hours, soon], 900).gaps).toEqual([]);
    });
  });
});
