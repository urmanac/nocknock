/**
 * Pins the Recharts 2.15 behaviour the #61 fix relies on, by static rendering: a break row
 * splits a line, a ReferenceArea labels the gap (also on a two-axis chart, where it needs
 * a yAxisId), and an isolated row is drawn as a dot instead of vanishing.
 */
import { it, expect } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { AreaChart, Area, ComposedChart, Line, XAxis, YAxis, ReferenceArea } from 'recharts';
import { toChartSeries } from './timeline.ts';

type Row = { t: number; a: number; b: number; isolated?: boolean };

const series = toChartSeries<Row>(
  [0, 300000, 5000000, 9000000, 9300000].map((t, i) => ({ t, a: 10 + i, b: 5 })),
  300,
);

const dot = (props: any) =>
  props.payload?.isolated
    ? <circle key={props.key} className="isolated-dot" cx={props.cx} cy={props.cy} r={3} />
    : <g key={props.key} />;

const axis = (
  <XAxis dataKey="t" type="number" scale="time" domain={['dataMin', 'dataMax']} ticks={series.ticks} />
);

it('stacked areas: gaps are shaded and labelled, isolated rows get dots', () => {
  const html = renderToStaticMarkup(
    <AreaChart width={600} height={300} data={series.rows}>
      {axis}
      <YAxis />
      {series.gaps.map((g) => (
        <ReferenceArea key={g.gapStart} x1={g.gapStart} x2={g.gapEnd} label={{ value: g.time }} />
      ))}
      <Area dataKey="a" stackId="1" dot={dot} isAnimationActive={false} />
      <Area dataKey="b" stackId="1" dot={dot} isAnimationActive={false} />
    </AreaChart>,
  );
  expect(html).toContain('no snapshots for 1h 18m');
  expect(html).toContain('no snapshots for 1h 7m');
  expect((html.match(/recharts-reference-area-rect/g) || []).length).toBe(2);
  // One isolated row (t = 5000000), one dot per stacked series.
  expect((html.match(/class="isolated-dot"/g) || []).length).toBe(2);
});

it('two-axis line chart: the line breaks into three runs at the two gaps', () => {
  const html = renderToStaticMarkup(
    <ComposedChart width={600} height={300} data={series.rows}>
      {axis}
      <YAxis yAxisId="left" />
      <YAxis yAxisId="right" orientation="right" />
      {series.gaps.map((g) => (
        <ReferenceArea key={g.gapStart} yAxisId="left" x1={g.gapStart} x2={g.gapEnd} label={{ value: g.time }} />
      ))}
      <Line yAxisId="left" dataKey="a" dot={dot} isAnimationActive={false} />
    </ComposedChart>,
  );
  expect(html).toContain('no snapshots for 1h 18m');
  const path = html.match(/class="recharts-curve recharts-line-curve"[^>]*d="([^"]*)"|d="([^"]*)"[^>]*class="recharts-curve recharts-line-curve"/);
  const d = path?.[1] ?? path?.[2] ?? '';
  expect(d.match(/M/g)?.length).toBe(3);
  expect((html.match(/class="isolated-dot"/g) || []).length).toBe(1);
});
