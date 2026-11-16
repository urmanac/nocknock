/**
 * Pins what the "Average cost vs spot" chart draws (issue #98), by static rendering:
 * the spot line breaks at each gap instead of bridging it, a lone stored point is a dot,
 * the average cost is a step line, and every verified buy has its own marker.
 */
import { it, expect } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { CostChartSvg } from './components/AverageCostPanel';
import { buildCostChart, type CostBasisSeries } from './costBasis';

const point = (at: string, price: string, backfilled = false) => ({
  at,
  price_usd: price,
  source: backfilled ? 'geckoterminal' : 'uniswap_rpc',
  is_backfilled: backfilled,
});

const fill = (tx: string, at: string, price: string, avg: string, whole: string | null) => ({
  plan_id: '2026-11-reentry',
  tx_hash: tx,
  block_number: 1,
  confirmed_at: at,
  usdc_spent: '20',
  nock_received: '1700',
  effective_price_usd: price,
  plan_avg_cost_usd: avg,
  avg_cost_usd: avg,
  whole_stack_avg_cost_usd: whole,
  spot_at_fill: null,
});

const series = (whole: boolean): CostBasisSeries => ({
  generated_at: '2026-11-16T04:00:00+00:00',
  from: '2026-11-14T00:00:00+00:00',
  to: '2026-11-16T04:00:00+00:00',
  bucket_seconds: 900,
  spot_match_window_minutes: 60,
  spot: [
    point('2026-11-14T06:00:00+00:00', '0.0124'),
    point('2026-11-14T06:15:00+00:00', '0.0125'),
    point('2026-11-14T06:30:00+00:00', '0.01245'),
    point('2026-11-15T03:00:00+00:00', '0.0117', true),
    point('2026-11-16T03:00:00+00:00', '0.0112'),
    point('2026-11-16T03:15:00+00:00', '0.01125'),
  ],
  spot_error: null,
  fills: [
    fill('0xa', '2026-11-14T06:02:25+00:00', '0.01251616', '0.01251616', null),
    fill('0xb', '2026-11-15T02:58:46+00:00', '0.01162643', '0.01205490', whole ? '0.01778234' : null),
    fill('0xc', '2026-11-16T03:08:37+00:00', '0.01123369', '0.01176814', whole ? '0.01765764' : null),
  ],
  whole_stack: whole
    ? {
        status: 'ok',
        reason: null,
        opening: { as_of: '2026-11-14T12:00:00+00:00', nock: '90000.0000', cost_usd: '1611.00', avg_cost_usd: '0.01790000', source: 'test figures' },
      }
    : { status: 'not_configured', reason: null, opening: null },
});

const render = (whole: boolean) =>
  renderToStaticMarkup(<CostChartSvg chart={buildCostChart(series(whole))!} width={800} height={280} />);

/** The `d` attribute of each line curve, in document order. */
const curves = (html: string) =>
  [...html.matchAll(/<path[^>]*class="recharts-curve recharts-line-curve"[^>]*>/g)].map(
    (m) => m[0].match(/ d="([^"]*)"/)?.[1] ?? '',
  );

it('plan buys only: spot breaks at both gaps, average is a step line, three buys are marked', () => {
  const html = render(false);
  const [spot, plan] = curves(html);
  // Three separate runs (three points, the lone candle, two points): the path lifts the
  // pen at each gap instead of bridging it.
  expect((spot.match(/M/g) || []).length).toBe(3);
  expect((spot.match(/L/g) || []).length).toBe(3);
  // The lone backfilled candle between the gaps is a dot.
  expect((html.match(/class="cost-spot-isolated"/g) || []).length).toBe(1);
  // Both gaps are shaded and labelled.
  expect(html).toContain('no stored price for 20h 30m');
  expect(html).toContain('no stored price for 1d 0h');
  expect((html.match(/recharts-reference-area-rect/g) || []).length).toBe(2);
  // A step line: one unbroken path whose every segment is horizontal or vertical.
  expect((plan.match(/M/g) || []).length).toBe(1);
  expect(plan).not.toMatch(/C/);
  const pts = [...plan.matchAll(/[ML]([\d.]+),([\d.]+)/g)].map((m) => [Number(m[1]), Number(m[2])]);
  expect(pts.length).toBeGreaterThanOrEqual(6);
  for (let i = 1; i < pts.length; i++) {
    expect(pts[i][0] === pts[i - 1][0] || pts[i][1] === pts[i - 1][1]).toBe(true);
  }
  // The average falls at each of these three buys (y grows downward), and never rises.
  const ys = pts.map((q) => q[1]);
  expect(new Set(ys).size).toBe(3);
  expect([...ys].sort((a, b) => a - b)).toEqual(ys);
  expect((html.match(/class="cost-fill-marker"/g) || []).length).toBe(3);
  expect(html).toContain('Verified buy at $0.01252');
  expect(html).toContain('Plan buys: average cost');
  expect(html).not.toContain('Whole stack: average cost');
});

it('with an opening basis the whole-stack step line is drawn and named as the operator\'s', () => {
  const html = render(true);
  expect(html).toContain('Whole stack: average cost (your opening basis + verified buys)');
  // spot, plan average, whole-stack average, and the zero-width path under the buy markers.
  expect(curves(html).length).toBeGreaterThanOrEqual(3);
  expect((html.match(/class="cost-fill-marker"/g) || []).length).toBe(3);
});
