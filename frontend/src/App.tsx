import React, { useMemo } from 'react';
import { createRoot } from 'react-dom/client';
import {
  AreaChart,
  Area,
  XAxis,
  YAxis,
  CartesianGrid,
  Tooltip,
  ResponsiveContainer,
  LineChart,
  Line,
  BarChart,
  Bar,
  Cell,
  ComposedChart,
  RadarChart,
  PolarGrid,
  Radar,
  Legend,
  ScatterChart,
  Scatter,
} from 'recharts';
import { api } from './api/client';
import { useFreshness, StaleBanner, type SnapshotEnvelope } from './hooks/useFreshness.tsx';
import type { DerivedMetrics } from './types/snapshot';

function Dashboard() {
  const { snapshot, loading, error, freshness, refetch, isStale, ageDisplay } = useFreshness<
    DerivedMetrics
  >(api.latestSnapshot, {
    syncWindowSeconds: 300,
    pollIntervalMs: 10_000,
    debug: import.meta.env.DEV,
    onFreshnessChange: (fresh) => console.log('[Dashboard] Freshness:', fresh ? 'FRESH' : 'STALE'),
  });

  if (loading && !snapshot) {
    return (
      <div style={styles.container}>
        <div style={styles.loading}>Loading portfolio state room...</div>
      </div>
    );
  }

  if (error) {
    return (
      <div style={styles.container}>
        <div style={styles.error}>Error: {error.message}</div>
        <button onClick={refetch} style={styles.button}>
          Retry
        </button>
      </div>
    );
  }

  const metrics = snapshot?.data;
  const global = metrics?.global_status;

  // Transform time-series data for charts
  const assetSeries = useMemo(() => {
    const m = metrics?.asset_aggregation_series;
    if (!m || !m.time_points?.length) return [];
    return m.time_points.map((t, i) => ({
      time: new Date(t).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
      safeHaven: parseFloat(m.safe_haven_series[i] || '0'),
      activeProtocol: parseFloat(m.active_protocol_series[i] || '0'),
    }));
  }, [metrics?.asset_aggregation_series]);

  const valueVsBaseline = useMemo(() => {
    const m = metrics?.total_value_vs_baseline;
    if (!m || !m.time_points?.length) return [];
    return m.time_points.map((t, i) => ({
      time: new Date(t).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
      portfolio: parseFloat(m.portfolio_value_series[i] || '0'),
      tokenPrice: parseFloat(m.token_price_series[i] || '0'),
      highWaterMark: parseFloat(m.high_water_mark_series[i] || '0'),
    }));
  }, [metrics?.total_value_vs_baseline]);

  const counterfactual = useMemo(() => {
    const m = metrics?.strategy_counterfactual;
    if (!m || !m.time_points?.length) return [];
    return m.time_points.map((t, i) => ({
      time: new Date(t).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
      active: parseFloat(m.active_portfolio_series[i] || '0'),
      passive: parseFloat(m.passive_baseline_series[i] || '0'),
    }));
  }, [metrics?.strategy_counterfactual]);

  const deviationData = useMemo(() => {
    const allocs = metrics?.portfolio_deviation_matrix?.allocations || [];
    return allocs.map(a => ({
      name: a.bucket_name,
      target: a.target_weight_bps / 100,
      actual: a.actual_weight_bps / 100,
      deviation: a.deviation_bps / 100,
      status: a.status,
    }));
  }, [metrics?.portfolio_deviation_matrix?.allocations]);

  const tickData = useMemo(() => {
    const brackets = metrics?.tick_bracket_hypervisor?.brackets || [];
    return brackets.map((b, i) => ({
      name: `Range ${i + 1}`,
      lower: parseFloat(b.lower_price),
      upper: parseFloat(b.upper_price),
      liquidity: parseFloat(b.liquidity_usd),
      inRange: b.in_range,
      distance: b.distance_bps / 100,
    }));
  }, [metrics?.tick_bracket_hypervisor?.brackets]);

  const gasData = useMemo(() => {
    const contracts = metrics?.velocity_gas_sandbox?.contracts || [];
    return contracts.map(c => ({
      name: c.contract_name,
      claimable: parseFloat(c.claimable_usd),
      gasBreakEven: parseFloat(c.gas_break_even_usd),
      ratio: c.efficiency_ratio,
      action: c.recommended_action,
    }));
  }, [metrics?.velocity_gas_sandbox?.contracts]);

  return (
    <div style={styles.container}>
      {/* Global Status Bar */}
      <header style={styles.header}>
        <div style={styles.titleRow}>
          <h1 style={styles.title}>NockNock</h1>
          <div style={styles.statusPill}>
            <span style={{
              ...styles.statusDot,
              background: freshness.isFresh ? '#2ecc71' : '#f39c12',
            }} />
            <span style={styles.statusText}>
              {freshness.isFresh ? 'LIVE' : 'STALE'}
            </span>
            <span style={styles.ageText}>{ageDisplay}</span>
          </div>
        </div>

        <div style={styles.globalStats}>
          <StatCard label="Total Portfolio" value={global?.total_portfolio_usd} prefix="$" />
          <StatCard label="Native (NOCK/ETH)" value={global?.total_native_usd} prefix="$" />
          <StatCard label="Stable (USDC)" value={global?.total_stable_usd} prefix="$" />
          <StatCard label="Fees Accrued" value={global?.total_fees_accrued_usd} prefix="$" />
          <StatCard label="Claimable (net)" value={global?.gas_to_fee_delta_usd} prefix="$" />
          <StatCard label="High-Water Mark" value={global?.high_water_mark_usd} prefix="$" />
        </div>
      </header>

      {/* Storyboard Canvas — Real Charts */}
      <main style={styles.grid}>
        {/* 1. Asset Aggregation — Stacked Area */}
        <CanvasPanel title="Asset Aggregation (Stacked Area)">
          <ResponsiveContainer width="100%" height={250}>
            <AreaChart data={assetSeries} margin={{ top: 10, right: 30, left: 0, bottom: 0 }}>
              <defs>
                <linearGradient id="colorSafeHaven" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#2ecc71" stopOpacity={0.4} />
                  <stop offset="95%" stopColor="#2ecc71" stopOpacity={0} />
                </linearGradient>
                <linearGradient id="colorActive" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#3498db" stopOpacity={0.4} />
                  <stop offset="95%" stopColor="#3498db" stopOpacity={0} />
                </linearGradient>
              </defs>
              <XAxis dataKey="time" stroke="#555" fontSize={11} tickLine={false} />
              <YAxis stroke="#555" fontSize={11} tickFormatter={v => '$' + v.toLocaleString()} />
              <CartesianGrid strokeDasharray="3 3" stroke="#222" />
              <Tooltip labelFormatter={v => new Date(v).toLocaleTimeString()} />
              <Area type="monotone" dataKey="safeHaven" fill="url(#colorSafeHaven)" stroke="#2ecc71" name="Safe Haven" />
              <Area type="monotone" dataKey="activeProtocol" fill="url(#colorActive)" stroke="#3498db" name="Active Protocol" />
            </AreaChart>
          </ResponsiveContainer>
        </CanvasPanel>

        {/* 2. Total Value vs Baseline — Dual Axis */}
        <CanvasPanel title="Total Value vs Baseline (Dual Axis)">
          <ResponsiveContainer width="100%" height={250}>
            <ComposedChart data={valueVsBaseline} margin={{ top: 10, right: 80, left: 0, bottom: 0 }}>
              <XAxis dataKey="time" stroke="#555" fontSize={11} tickLine={false} />
              <YAxis yAxisId="left" orientation="left" stroke="#555" fontSize={11} tickFormatter={v => '$' + v.toLocaleString()} />
              <YAxis yAxisId="right" orientation="right" stroke="#555" fontSize={11} tickFormatter={v => '$' + v.toFixed(4)} domain={['auto', 'auto']} />
              <CartesianGrid strokeDasharray="3 3" stroke="#222" />
              <Tooltip labelFormatter={v => new Date(v).toLocaleTimeString()} />
              <Line yAxisId="left" type="monotone" dataKey="portfolio" stroke="#2ecc71" strokeWidth={2} dot={false} name="Portfolio Value" />
              <Line yAxisId="left" type="monotone" dataKey="highWaterMark" stroke="#f39c12" strokeWidth={1} strokeDasharray="5 5" dot={false} name="High-Water Mark" />
              <Line yAxisId="right" type="monotone" dataKey="tokenPrice" stroke="#3498db" strokeWidth={2} dot={false} name="Token Price" />
            </ComposedChart>
          </ResponsiveContainer>
        </CanvasPanel>

        {/* 3. Tick Bracket Hypervisor — Horizontal Ranges */}
        <CanvasPanel title="Tick Bracket Hypervisor">
          <ResponsiveContainer width="100%" height={250}>
            <BarChart data={tickData} layout="vertical" margin={{ top: 10, right: 30, left: 100, bottom: 0 }}>
              <XAxis type="number" tickFormatter={v => '$' + v.toLocaleString()} stroke="#555" fontSize={11} />
              <YAxis type="category" dataKey="name" width={100} stroke="#555" fontSize={11} />
              <CartesianGrid strokeDasharray="3 3" stroke="#222" vertical={false} />
              <Tooltip formatter={v => '$' + v.toLocaleString()} />
              <Bar dataKey="upper" fill="#f39c12" fillOpacity={0.3} name="Upper" />
              <Bar dataKey="lower" fill="#2ecc71" fillOpacity={0.3} name="Lower" />
              <Legend />
            </BarChart>
          </ResponsiveContainer>
          <div style={{ marginTop: 12, fontSize: '0.7rem', color: '#888', fontFamily: 'monospace' }}>
            Current Price: <strong>{metrics?.tick_bracket_hypervisor?.current_price || '—'}</strong>
          </div>
        </CanvasPanel>

        {/* 4. Portfolio Deviation Matrix — Bar Chart */}
        <CanvasPanel title="Portfolio Deviation Matrix">
          <ResponsiveContainer width="100%" height={250}>
            <BarChart data={deviationData} layout="vertical" margin={{ top: 10, right: 30, left: 120, bottom: 0 }}>
              <XAxis type="number" tickFormatter={v => v.toFixed(1) + '%'} stroke="#555" fontSize={11} />
              <YAxis type="category" dataKey="name" width={120} stroke="#555" fontSize={11} />
              <CartesianGrid strokeDasharray="3 3" stroke="#222" vertical={false} />
              <Tooltip formatter={v => v.toFixed(1) + '%'} />
              <Bar dataKey="target" fill="#888" fillOpacity={0.5} name="Target %" />
              <Bar
                dataKey="actual"
                name="Actual %"
              >
                {deviationData.map((d, i) => (
                  <Cell key={i} fill={d.status === 'over_funded' ? '#e74c3c' : d.status === 'under_funded' ? '#f39c12' : d.status === 'fast_running' ? '#e67e22' : '#2ecc71'} />
                ))}
              </Bar>
              <Legend />
            </BarChart>
            <div style={{ marginTop: 8, fontSize: '0.7rem', display: 'flex', gap: 16, flexWrap: 'wrap' }}>
              <span><span style={{ color: '#2ecc71' }}>■</span> On Target</span>
              <span><span style={{ color: '#f39c12' }}>■</span> Under-Funded</span>
              <span><span style={{ color: '#e74c3c' }}>■</span> Over-Funded</span>
              <span><span style={{ color: '#e67e22' }}>■</span> Fast Running</span>
            </div>
          </ResponsiveContainer>
        </CanvasPanel>

        {/* 5. Structural Channels — Line Chart (placeholder) */}
        <CanvasPanel title="Structural Channels (Regression)">
          {metrics?.structural_channels?.channels?.length > 0 ? (
            <ResponsiveContainer width="100%" height={250}>
              <LineChart data={metrics.structural_channels.channels.flatMap(c => c.center_line.map(p => ({ time: new Date(p.time).toLocaleTimeString(), value: parseFloat(p.value), channel: c.start_time }))} margin={{ top: 10, right: 30, left: 0, bottom: 0 }}>
                <XAxis dataKey="time" stroke="#555" fontSize={11} tickLine={false} />
                <YAxis stroke="#555" fontSize={11} tickFormatter={v => '$' + v.toLocaleString()} />
                <CartesianGrid strokeDasharray="3 3" stroke="#222" />
                <Tooltip />
                <Line type="monotone" dataKey="value" stroke="#3498db" strokeWidth={2} dot={false} />
              </LineChart>
            </ResponsiveContainer>
          ) : (
            <div style={styles.emptyState}>No channel data yet — awaiting historical accumulation</div>
          )}
        </CanvasPanel>

        {/* 6. Strategy Counterfactual — Dual Line */}
        <CanvasPanel title="Strategy Counterfactual">
          <ResponsiveContainer width="100%" height={250}>
            <LineChart data={counterfactual} margin={{ top: 10, right: 30, left: 0, bottom: 0 }}>
              <XAxis dataKey="time" stroke="#555" fontSize={11} tickLine={false} />
              <YAxis stroke="#555" fontSize={11} tickFormatter={v => '$' + v.toLocaleString()} />
              <CartesianGrid strokeDasharray="3 3" stroke="#222" />
              <Tooltip labelFormatter={v => new Date(v).toLocaleTimeString()} />
              <Line type="monotone" dataKey="active" stroke="#2ecc71" strokeWidth={2} dot={false} name="Active Portfolio" />
              <Line type="monotone" dataKey="passive" stroke="#e74c3c" strokeWidth={2} strokeDasharray="5 5" dot={false} name="Passive Baseline" />
            </LineChart>
          </ResponsiveContainer>
        </CanvasPanel>

        {/* 7. Velocity Gas Sandbox — Bar Chart */}
        <CanvasPanel title="Velocity Gas Sandbox">
          <ResponsiveContainer width="100%" height={250}>
            <BarChart data={gasData} margin={{ top: 10, right: 30, left: 0, bottom: 0 }}>
              <XAxis dataKey="name" stroke="#555" fontSize={11} tickLine={false} />
              <YAxis stroke="#555" fontSize={11} tickFormatter={v => '$' + v.toLocaleString()} />
              <CartesianGrid strokeDasharray="3 3" stroke="#222" />
              <Tooltip />
              <Bar dataKey="claimable" fill="#2ecc71" fillOpacity={0.7} name="Claimable" />
              <Bar dataKey="gasBreakEven" fill="#e74c3c" fillOpacity={0.7} name="Gas Break-Even" />
              <Legend />
            </BarChart>
            <div style={{ marginTop: 8, fontSize: '0.7rem', display: 'flex', gap: 16, flexWrap: 'wrap' }}>
              {gasData.map(g => (
                <span key={g.name} style={{ color: g.action === 'claim_now' ? '#2ecc71' : g.action === 'wait' ? '#f39c12' : '#e74c3c' }}>
                  {g.name}: {g.action} (ratio: {g.ratio.toFixed(2)})
                </span>
              ))}
            </div>
          </ResponsiveContainer>
        </CanvasPanel>
      </main>

      {/* Stale Banner — non-obtrusive */}
      <StaleBanner freshness={freshness} />
    </div>
  );
}

function StatCard({ label, value, prefix = '' }: { label: string; value?: string; prefix?: string }) {
  const display = value ? `${prefix}${parseFloat(value).toLocaleString(undefined, { maximumFractionDigits: 2 })}` : '—';
  return (
    <div style={styles.statCard}>
      <div style={styles.statLabel}>{label}</div>
      <div style={styles.statValue}>{display}</div>
    </div>
  );
}

function CanvasPanel({ title, children }: { title: string; children: React.ReactNode }) {
  return (
    <div style={styles.panel}>
      <h3 style={styles.panelTitle}>{title}</h3>
      <div style={styles.panelContent}>{children}</div>
    </div>
  );
}

function EmptyState({ children }: { children: React.ReactNode }) {
  return (
    <div style={{
      ...styles.emptyState,
      display: 'flex',
      alignItems: 'center',
      justifyContent: 'center',
      height: '200px',
    }}>
      {children}
    </div>
  );
}

const styles: Record<string, React.CSSProperties> = {
  container: {
    minHeight: '100vh',
    background: '#0d0d0d',
    color: '#e0e0e0',
    fontFamily: 'system-ui, -apple-system, monospace',
    padding: '1rem',
  },
  loading: { textAlign: 'center', padding: '4rem', color: '#666' },
  error: { textAlign: 'center', padding: '2rem', color: '#e74c3c' },
  button: { padding: '0.5rem 1rem', background: '#3498db', border: 'none', borderRadius: 4, color: '#fff', cursor: 'pointer' },
  header: { marginBottom: '1.5rem', borderBottom: '1px solid #222', paddingBottom: '1rem' },
  titleRow: { display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem' },
  title: { margin: 0, fontSize: '1.5rem', fontWeight: 600, letterSpacing: '0.05em', color: '#3498db' },
  statusPill: { display: 'flex', alignItems: 'center', gap: 6, padding: '4px 10px', background: '#1a1a1a', borderRadius: 20, border: '1px solid #333' },
  statusDot: { width: 8, height: 8, borderRadius: '50%' },
  statusText: { fontSize: '0.7rem', textTransform: 'uppercase', letterSpacing: '0.1em', fontWeight: 600 },
  ageText: { fontSize: '0.7rem', color: '#888', fontFamily: 'monospace' },
  globalStats: { display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))', gap: '1rem' },
  statCard: { background: '#151515', border: '1px solid #2a2a2a', borderRadius: 8, padding: '1rem' },
  statLabel: { fontSize: '0.7rem', textTransform: 'uppercase', letterSpacing: '0.1em', color: '#888', marginBottom: '0.25rem' },
  statValue: { fontSize: '1.1rem', fontWeight: 600, fontFamily: 'monospace', color: '#fff' },
  grid: { display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(400px, 1fr))', gap: '1rem' },
  panel: { background: '#151515', border: '1px solid #2a2a2a', borderRadius: 8, padding: '1rem', minHeight: '280px' },
  panelTitle: { margin: '0 0 0.75rem', fontSize: '0.8rem', textTransform: 'uppercase', letterSpacing: '0.1em', color: '#aaa', borderBottom: '1px solid #2a2a2a', paddingBottom: '0.5rem' },
  panelContent: { minHeight: '220px' },
  emptyState: { color: '#555', fontSize: '0.8rem', textAlign: 'center', fontStyle: 'italic' },
};

// Bootstrap
const root = createRoot(document.getElementById('root')!);
root.render(<React.StrictMode><Dashboard /></React.StrictMode>);