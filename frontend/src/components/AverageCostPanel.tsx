/**
 * "Average cost vs spot" panel (issue #98).
 *
 * Shows what the plan's verified buys cost against the NOCK price over time, with each
 * buy marked, and what that NOCK is worth at the current spot. When the operator has
 * entered an opening basis in config, the blended whole-stack average is drawn too,
 * labelled as their figure. The panel measures; it never suggests a trade.
 *
 * Summary figures come from the snapshot (`derived_metrics.cost_basis`); the chart and
 * the fill table come from GET /v1/cost-basis/series. All display logic is in
 * ../costBasis.ts, which has the tests.
 */
import { useEffect, useState } from 'react';
import { CartesianGrid, ComposedChart, Legend, Line, ReferenceArea, ResponsiveContainer, XAxis, YAxis } from 'recharts';
import { api } from '../api/client';
import {
  COST_PANEL_TITLE,
  buildCostChart,
  costBasisView,
  fillRows,
  ladderChartNote,
  spotProvenanceText,
  type CostBasis,
  type CostBasisSeries,
  type CostChart,
  type DifferenceTone,
} from '../costBasis';

export { COST_PANEL_TITLE };

const label = { fontSize: '0.7rem', letterSpacing: 2, color: '#8b949e', marginBottom: 6 } as const;
const big = { fontSize: '1.4rem', fontWeight: 700 } as const;
const small = { fontSize: '0.7rem', color: '#8b949e', marginTop: 6, lineHeight: 1.6 } as const;
const cell = { padding: '4px 10px 4px 0', whiteSpace: 'nowrap', textAlign: 'left' } as const;

/** Above/below cost are measurements, so neither is drawn as an alarm or as a win. */
const TONE_COLOR: Record<DifferenceTone, string> = {
  above: '#38bdf8',
  below: '#f59e0b',
  even: '#f0f6fc',
  none: '#f0f6fc',
};

const SPOT_COLOR = '#38bdf8';
const PLAN_COLOR = '#f59e0b';
const WHOLE_COLOR = '#a855f7';
const FILL_COLOR = '#10b981';

function chartTime(t: number): string {
  return new Date(t).toLocaleString(undefined, { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
}

/** Draw only spot points with no stored neighbour: a line needs two points, a lone one would vanish. */
function isolatedSpotDot(props: any) {
  const { key, cx, cy, payload } = props ?? {};
  if (!payload?.isolated || !Number.isFinite(cx) || !Number.isFinite(cy)) return <g key={key} />;
  return <circle key={key} className="cost-spot-isolated" cx={cx} cy={cy} r={2.5} fill={SPOT_COLOR} stroke={SPOT_COLOR} />;
}

function fillDot(props: any) {
  const { key, cx, cy, payload } = props ?? {};
  if (!Number.isFinite(cx) || !Number.isFinite(cy)) return <g key={key} />;
  return (
    <circle key={key} className="cost-fill-marker" cx={cx} cy={cy} r={5} fill={FILL_COLOR} stroke="#0d1117" strokeWidth={1.5}>
      <title>{`Verified buy at $${Number(payload?.price).toFixed(5)} · ${chartTime(payload?.t)}`}</title>
    </circle>
  );
}

/**
 * The chart itself. Each series carries its own rows on one numeric time axis, so the
 * spot line can break at its gaps while the average-cost steps run straight through.
 * `width`/`height` are supplied by ResponsiveContainer (or by a test).
 */
export function CostChartSvg({ chart, width, height }: { chart: CostChart; width?: number; height?: number }) {
  return (
    <ComposedChart width={width} height={height} margin={{ top: 15, right: 30, left: 15, bottom: 0 }}>
      <XAxis
        dataKey="t"
        type="number"
        scale="time"
        domain={chart.domain}
        ticks={chart.ticks}
        interval="preserveStartEnd"
        allowDuplicatedCategory={false}
        stroke="#334155"
        tick={{ fill: '#94a3b8', fontSize: 11 }}
        tickFormatter={chartTime}
      />
      <YAxis
        stroke="#334155"
        tick={{ fill: '#94a3b8', fontSize: 11 }}
        domain={chart.yDomain ?? ['auto', 'auto']}
        tickFormatter={(v: number) => `$${Number(v).toFixed(4)}`}
        width={70}
      />
      <CartesianGrid strokeDasharray="3 3" stroke="#222" />
      {/* Backfilled candles: one lighter region and one caption per run (issue #137). */}
      {chart.backfill.map((b) => (
        <ReferenceArea
          key={`backfill-${b.start}`}
          x1={b.start}
          x2={b.end}
          fill="#a78bfa"
          fillOpacity={0.06}
          strokeOpacity={0}
          label={{ value: b.caption, fill: '#a78bfa', fontSize: 10, position: 'insideBottom' }}
        />
      ))}
      {chart.gaps.map((g) => (
        <ReferenceArea
          key={`gap-${g.gapStart}`}
          x1={g.gapStart}
          x2={g.gapEnd}
          fill="#64748b"
          fillOpacity={0.15}
          strokeOpacity={0}
          label={g.labelled ? { value: g.time, fill: '#94a3b8', fontSize: 10, position: 'insideTop' } : undefined}
        />
      ))}
      <Line
        data={chart.spot}
        dataKey="spot"
        name="NOCK spot (stored observations)"
        type="linear"
        stroke={SPOT_COLOR}
        strokeWidth={1.5}
        dot={isolatedSpotDot}
        activeDot={false}
        connectNulls={false}
        isAnimationActive={false}
      />
      <Line
        data={chart.planAvg}
        dataKey="v"
        name="Plan buys: average cost"
        type="stepAfter"
        stroke={PLAN_COLOR}
        strokeWidth={2}
        dot={false}
        activeDot={false}
        isAnimationActive={false}
      />
      {chart.wholeAvg.length > 0 && (
        <Line
          data={chart.wholeAvg}
          dataKey="v"
          name="Whole stack: average cost (your opening basis + verified buys)"
          type="stepAfter"
          stroke={WHOLE_COLOR}
          strokeWidth={2}
          strokeDasharray="6 4"
          dot={false}
          activeDot={false}
          isAnimationActive={false}
        />
      )}
      <Line
        data={chart.fills}
        dataKey="price"
        name="Verified buy (price paid)"
        stroke={FILL_COLOR}
        strokeWidth={0}
        legendType="circle"
        dot={fillDot}
        activeDot={false}
        isAnimationActive={false}
      />
      <Legend wrapperStyle={{ color: '#cbd5e1' }} formatter={(val) => <span style={{ color: '#cbd5e1' }}>{val}</span>} />
    </ComposedChart>
  );
}

function seriesErrorText(e: unknown): string {
  const status = (e as { status?: unknown } | null)?.status;
  if (status === 404) return 'This worker does not serve /v1/cost-basis/series yet (it predates the chart).';
  const msg = e instanceof Error ? e.message : String(e);
  return typeof status === 'number' ? msg : `Worker unreachable: ${msg}`;
}

export function AverageCostPanel({
  costBasis,
  refreshKey,
}: {
  costBasis?: CostBasis | null;
  /** Changes when a new snapshot arrives; the series is fetched again then. */
  refreshKey?: string;
}) {
  const [series, setSeries] = useState<CostBasisSeries | null>(null);
  const [seriesError, setSeriesError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    api
      .costBasisSeries()
      .then((res) => {
        if (cancelled) return;
        setSeries(res);
        setSeriesError(null);
      })
      .catch((e) => {
        if (cancelled) return;
        // Keep nothing stale on screen under an error: the chart is hidden with the reason.
        setSeries(null);
        setSeriesError(seriesErrorText(e));
      });
    return () => {
      cancelled = true;
    };
  }, [refreshKey]);

  const view = costBasisView(costBasis);
  const chart = buildCostChart(series);
  const rows = fillRows(series);
  const showWhole = series?.whole_stack?.status === 'ok';

  return (
    <div>
      {view.configError && (
        <div
          role="alert"
          style={{
            border: '1px solid #30363d',
            borderLeft: '3px solid #f59e0b',
            borderRadius: 6,
            padding: '10px 14px',
            marginBottom: 14,
            fontSize: '0.8rem',
            color: '#c9d1d9',
            lineHeight: 1.6,
          }}
        >
          <strong style={{ color: '#f59e0b', letterSpacing: 1 }}>CONFIGURATION ERROR</strong> · {view.configError}
        </div>
      )}

      <div style={{ display: 'flex', gap: 24, flexWrap: 'wrap', marginBottom: 14 }}>
        {view.tiles.map((tile) => (
          <div key={tile.label} style={{ flex: 1, minWidth: 200 }}>
            <div style={label}>{tile.label}</div>
            <div style={{ ...big, color: TONE_COLOR[tile.tone] }}>{tile.value}</div>
            <div style={small}>{tile.sub}</div>
          </div>
        ))}
      </div>

      {seriesError && <div style={{ ...small, color: '#f59e0b' }}>Chart unavailable: {seriesError}</div>}
      {!seriesError && !series && <div style={small}>Loading the average-cost series…</div>}

      {chart && (
        <>
          <ResponsiveContainer width="100%" height={280}>
            <CostChartSvg chart={chart} />
          </ResponsiveContainer>
          <div style={{ ...small, textAlign: 'center' }}>
            {spotProvenanceText(chart)} The average-cost lines are steps: an average changes only when a buy is recorded.
          </div>
          {ladderChartNote(costBasis) && (
            <div style={{ ...small, textAlign: 'center' }} data-testid="cost-ladder-note">{ladderChartNote(costBasis)}</div>
          )}
        </>
      )}

      {series && rows.length === 0 && (
        <div style={small}>No verified buys recorded yet, so there is no average cost to draw.</div>
      )}

      {rows.length > 0 && (
        <div style={{ overflowX: 'auto', marginTop: 12 }}>
          <table style={{ borderCollapse: 'collapse', fontSize: '0.75rem', color: '#c9d1d9', width: '100%' }}>
            <thead>
              <tr style={{ color: '#8b949e', letterSpacing: 1, fontSize: '0.65rem' }}>
                <th style={cell}>VERIFIED BUY</th>
                <th style={cell}>PLAN</th>
                <th style={cell}>SPENT</th>
                <th style={cell}>NOCK RECEIVED</th>
                <th style={cell}>PRICE PAID</th>
                <th style={cell}>STORED SPOT THEN</th>
                <th style={cell}>PLAN-BUYS AVERAGE</th>
                {showWhole && <th style={cell}>WHOLE-STACK AVERAGE AFTER</th>}
                <th style={cell}>TX</th>
              </tr>
            </thead>
            <tbody>
              {rows.map((r) => (
                <tr key={r.txHash} style={{ borderTop: '1px solid #21262d' }}>
                  <td style={cell}>{new Date(r.confirmedAt).toLocaleString()}</td>
                  <td style={cell}>{r.planId}</td>
                  <td style={cell}>{r.spent}</td>
                  <td style={cell}>{r.nock}</td>
                  <td style={{ ...cell, color: FILL_COLOR }}>{r.price}</td>
                  <td style={cell}>{r.spotAtFill}</td>
                  <td style={{ ...cell, color: PLAN_COLOR }}>
                    {r.avgBefore} → {r.avgAfter}
                  </td>
                  {showWhole && <td style={{ ...cell, color: WHOLE_COLOR }}>{r.wholeAfter}</td>}
                  <td style={cell}>
                    <a
                      href={`https://basescan.org/tx/${r.txHash}`}
                      target="_blank"
                      rel="noreferrer"
                      style={{ color: '#58a6ff', fontFamily: 'monospace' }}
                    >
                      {r.txHash.slice(0, 10)}…
                    </a>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      )}

      <div style={{ ...small, color: '#484f58' }}>
        {view.notes.map((n) => (
          <div key={n}>{n}</div>
        ))}
        <div>These are measurements of buys already made. NockNock sizes no buy from them.</div>
      </div>
    </div>
  );
}
