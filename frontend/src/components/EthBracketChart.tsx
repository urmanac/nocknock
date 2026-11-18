/**
 * The ETH/USDC ranges on the Tick Bracket Hypervisor (#160), in USD per ETH.
 *
 * A gas-side range (WETH/USDC) is not priced in NOCK, so it cannot share the NOCK chart's
 * axis. Each one is a floating bar from its lower to its upper edge, the price span where
 * it is active, with the ETH spot as a line. These ranges are measured, never steered
 * (#151): nothing here feeds a target or a directive.
 */
import { Bar, BarChart, CartesianGrid, Cell, ReferenceLine, ResponsiveContainer, Tooltip, XAxis, YAxis } from 'recharts';
import type { EthBracketRow } from '../positions.ts';

const IN_RANGE = '#10b981';
const OUT_OF_RANGE = '#f59e0b';

const usd = (v: number) => '$' + Math.round(v).toLocaleString('en-US');

/** "$1,480 – $2,951 · IN RANGE · 15.4% to the nearer edge" (or "· at an edge") */
export function ethBracketText(r: EthBracketRow): string {
  const status = r.inRange ? 'IN RANGE' : 'OUT OF RANGE';
  // The status is the position's own pool tick; the distance is from the ETH spot of the
  // price pool, which can sit a few bps across an edge from it. Then say "at an edge".
  const dist = !r.inRange || r.distance === null
    ? ''
    : r.distance < 0.05 ? ' · at an edge' : ` · ${r.distance.toFixed(1)}% to the nearer edge`;
  return `${usd(r.lower)} – ${usd(r.upper)} · ${status}${dist}`;
}

/** The x domain: every edge and the spot, with 5% of the span as margin either side. */
export function ethBracketDomain(rows: EthBracketRow[], spot: number | null): [number, number] {
  const values = rows.flatMap((r) => [r.lower, r.upper]);
  if (spot !== null) values.push(spot);
  const lo = Math.min(...values);
  const hi = Math.max(...values);
  const pad = (hi - lo) * 0.05 || hi * 0.05;
  return [Math.max(0, Math.floor((lo - pad) / 10) * 10), Math.ceil((hi + pad) / 10) * 10];
}

export function EthBracketChart({
  rows,
  ethUsd,
  ethUsdFromSnapshot,
  fullRangeCount,
}: {
  rows: EthBracketRow[];
  ethUsd: number | null;
  ethUsdFromSnapshot: boolean;
  fullRangeCount: number;
}) {
  if (rows.length === 0 && fullRangeCount === 0) return null;
  const spot = ethUsd != null && ethUsd > 0 ? ethUsd : null;
  const data = rows.map((r) => ({ ...r, span: [r.lower, r.upper] as [number, number] }));
  return (
    <div style={{ marginTop: 16 }} data-testid="eth-bracket-chart">
      <div style={{ fontSize: '0.8rem', color: '#cbd5e1', fontWeight: 600, marginBottom: 4 }}>
        ETH/USDC ranges (USD per ETH) · measured, never steered
      </div>
      {rows.length > 0 && (
        <ResponsiveContainer width="100%" height={60 + rows.length * 36}>
          <BarChart data={data} layout="vertical" margin={{ top: 16, right: 30, left: 110, bottom: 0 }}>
            <XAxis
              type="number"
              domain={ethBracketDomain(rows, spot)}
              tickFormatter={(v) => usd(v)}
              stroke="#334155"
              tick={{ fill: '#94a3b8', fontSize: 11 }}
            />
            <YAxis type="category" dataKey="name" width={110} stroke="#334155" tick={{ fill: '#cbd5e1', fontSize: 11 }} />
            <CartesianGrid strokeDasharray="3 3" stroke="#222" vertical={false} />
            {spot !== null && (
              <ReferenceLine x={spot} stroke="#e2b714" strokeDasharray="4 4" label={{ value: 'ETH', fill: '#e2b714', fontSize: 11, position: 'top' }} />
            )}
            <Tooltip
              contentStyle={{ background: '#161b22', border: '1px solid #30363d', borderRadius: 6, color: '#f0f6fc' }}
              itemStyle={{ color: '#e6edf3', fontSize: '0.85rem' }}
              labelStyle={{ color: '#f0f6fc', fontWeight: 700, marginBottom: 4 }}
              formatter={(_v: any, _n: any, item: any) => {
                const r = item.payload as EthBracketRow;
                return [`${usd(r.lower)} – ${usd(r.upper)}`, r.inRange ? 'In range' : 'Out of range'];
              }}
            />
            <Bar dataKey="span" name="Active range ($/ETH)" fillOpacity={0.6}>
              {data.map((r) => (
                <Cell key={r.positionId ?? r.name} fill={r.inRange ? IN_RANGE : OUT_OF_RANGE} />
              ))}
            </Bar>
          </BarChart>
        </ResponsiveContainer>
      )}
      <div style={{ marginTop: 8, fontSize: '0.75rem', fontFamily: 'monospace', display: 'grid', gap: 2 }}>
        {rows.map((r) => (
          <span key={r.positionId ?? r.name} style={{ color: r.inRange ? IN_RANGE : OUT_OF_RANGE }}>
            {r.name}: {ethBracketText(r)}
          </span>
        ))}
        {fullRangeCount > 0 && (
          <span style={{ color: '#8b949e' }}>
            {fullRangeCount} full-range ETH/USDC position{fullRangeCount === 1 ? '' : 's'}: never out of range, no bracket to draw
          </span>
        )}
        <span style={{ color: '#8b949e' }}>
          {spot !== null
            ? `ETH spot ${usd(spot)} (${ethUsdFromSnapshot ? 'snapshot ETH/USD' : 'pool-derived, older snapshot'}); USDC counted as $1`
            : 'No ETH/USD in this snapshot: no spot line. USDC counted as $1'}
        </span>
      </div>
    </div>
  );
}
