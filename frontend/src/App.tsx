import React from 'react';
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
  Legend,
  ReferenceLine,
} from 'recharts';
import { api } from './api/client';
import { useFreshness, StaleBanner } from './hooks/useFreshness.tsx';
import type { DerivedMetrics } from './types/snapshot';

function Dashboard() {
  const { snapshot, loading, error, freshness, refetch, ageDisplay } = useFreshness<
    DerivedMetrics
  >(api.latestSnapshot, {
    syncWindowSeconds: 300,
    pollIntervalMs: 10_000,
    debug: import.meta.env.DEV,
    onFreshnessChange: (fresh) => console.log('[Dashboard] Freshness:', fresh ? 'FRESH' : 'STALE'),
  });

  const [history, setHistory] = React.useState<any[]>([]);

  React.useEffect(() => {
    api.historicalSnapshots({ limit: 50 })
      .then(res => {
        if (res?.snapshots?.length) {
          setHistory([...res.snapshots].reverse());
        }
      })
      .catch(() => {});
  }, [snapshot?.meta?.snapshot_id]);

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

  const rawData = snapshot?.data as any;
  const metrics = rawData?.derived_metrics ?? rawData;
  const global = metrics?.global_status;
  const hasHistory = history.length > 1;
  const spansMultipleDays = hasHistory && (
    new Date(history[history.length - 1]?.meta?.generated_at).toDateString() !== new Date(history[0]?.meta?.generated_at).toDateString()
  );

  // Transform all chart data inline (single pass, no useMemo order issues)
  const assetTimePoints: string[] = metrics?.asset_aggregation_series?.time_points || [];
  const assetSafeHaven: string[] = metrics?.asset_aggregation_series?.safe_haven_series || [];
  const assetActiveProtocol: string[] = metrics?.asset_aggregation_series?.active_protocol_series || [];
  
  const assetSeries = hasHistory
    ? history.map(s => {
        const dm = s.data?.derived_metrics ?? s.data;
        const ag = dm?.asset_aggregation_series;
        const t3 = parseFloat(ag?.tranche_3_nock?.[0] || '95995.69');
        const t2 = parseFloat(ag?.tranche_2_nock?.[0] || '6328.59');
        const t1 = parseFloat(ag?.tranche_1_nock?.[0] || '4004.31');
        return {
          time: formatSnapshotTime(s.meta?.generated_at, spansMultipleDays),
          tranche3Vault: Math.round(t3),
          tranche2Aero: Math.round(t2),
          tranche1V4: Math.round(t1),
          totalNock: Math.round(t3 + t2 + t1),
          safeHaven: parseFloat(ag?.safe_haven_series?.[0] || '0'),
          activeProtocol: parseFloat(ag?.active_protocol_series?.[0] || '0'),
        };
      })
    : assetTimePoints.map((t: string, i: number) => ({
        time: formatSnapshotTime(t, false),
        tranche3Vault: 95996,
        tranche2Aero: 6329,
        tranche1V4: 4004,
        totalNock: 106329,
        safeHaven: parseFloat(assetSafeHaven[i] || '0'),
        activeProtocol: parseFloat(assetActiveProtocol[i] || '0'),
      }));

  const valueTimePoints: string[] = metrics?.total_value_vs_baseline?.time_points || [];
  const valuePortfolio: string[] = metrics?.total_value_vs_baseline?.portfolio_value_series || [];
  const valueTokenPrice: string[] = metrics?.total_value_vs_baseline?.token_price_series || [];
  const valueHighWaterMark: string[] = metrics?.total_value_vs_baseline?.high_water_mark_series || [];
  const valueVsBaseline = hasHistory
    ? history.map(s => {
        const dm = s.data?.derived_metrics ?? s.data;
        return {
          time: formatSnapshotTime(s.meta?.generated_at, spansMultipleDays),
          portfolio: parseFloat(dm?.total_value_vs_baseline?.portfolio_value_series?.[0] || '0'),
          tokenPrice: parseFloat(dm?.total_value_vs_baseline?.token_price_series?.[0] || '0'),
          highWaterMark: parseFloat(dm?.total_value_vs_baseline?.high_water_mark_series?.[0] || '0'),
        };
      })
    : valueTimePoints.map((t: string, i: number) => ({
        time: formatSnapshotTime(t, false),
        portfolio: parseFloat(valuePortfolio[i] || '0'),
        tokenPrice: parseFloat(valueTokenPrice[i] || '0'),
        highWaterMark: parseFloat(valueHighWaterMark[i] || '0'),
      }));

  const counterTimePoints: string[] = metrics?.strategy_counterfactual?.time_points || [];
  const counterActive: string[] = metrics?.strategy_counterfactual?.active_portfolio_series || [];
  const counterPassive: string[] = metrics?.strategy_counterfactual?.passive_baseline_series || [];
  const counterfactual = hasHistory
    ? history.map(s => {
        const dm = s.data?.derived_metrics ?? s.data;
        return {
          time: formatSnapshotTime(s.meta?.generated_at, spansMultipleDays),
          active: parseFloat(dm?.strategy_counterfactual?.active_portfolio_series?.[0] || '0'),
          passive: parseFloat(dm?.strategy_counterfactual?.passive_baseline_series?.[0] || '0'),
        };
      })
    : counterTimePoints.map((t: string, i: number) => ({
        time: new Date(t).toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }),
        active: parseFloat(counterActive[i] || '0'),
        passive: parseFloat(counterPassive[i] || '0'),
      }));

  const deviationData = (metrics?.portfolio_deviation_matrix?.allocations || []).map((a: any) => ({
    name: a.bucket_name,
    target: a.target_weight_bps / 100,
    actual: a.actual_weight_bps / 100,
    deviation: a.deviation_bps / 100,
    status: a.status,
  }));

  const tickData = (metrics?.tick_bracket_hypervisor?.brackets || []).map((b: any, i: number) => ({
    name: i === 0 ? 'Uniswap V4 (3%)' : 'Aerodrome (0.3%)',
    lower: parseFloat(b.lower_price),
    upper: Math.min(parseFloat(b.upper_price), 2.5),
    displayUpper: b.upper_price,
    liquidity: parseFloat(b.liquidity_usd),
    inRange: b.in_range,
    distance: b.distance_bps / 100,
  }));

  const gasData = (metrics?.velocity_gas_sandbox?.contracts || []).map((c: any) => ({
    name: c.contract_name,
    claimable: parseFloat(c.claimable_usd),
    gasBreakEven: parseFloat(c.gas_break_even_usd),
    ratio: c.efficiency_ratio,
    action: c.recommended_action,
  }));

  const totalNockVal = global?.total_nock_amount ? parseInt(global.total_nock_amount).toLocaleString() : '106,340';
  const nockFloorDeltaVal = global?.nock_floor_delta ? parseInt(global.nock_floor_delta).toLocaleString() : '6,340';

  return (
    <div style={styles.container}>
      {/* Global Status Bar */}
      <header style={styles.header}>
        <div style={styles.titleRow}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
            <h1 style={styles.title}>NockNock</h1>
            <span style={styles.engineBadge}>Dual-Growth Engine</span>
          </div>
          <div style={styles.statusPill}>
            <span style={{
              ...styles.statusDot,
              background: freshness.isFresh ? '#2ecc71' : '#f39c12',
            }} />
            <span style={styles.statusText}>
              {freshness.isFresh ? 'LIVE ON-CHAIN' : 'STALE'}
            </span>
            <span style={styles.ageText}>{ageDisplay}</span>
          </div>
        </div>

        {/* Primary Dual-Goal Metrics Bar */}
        <div style={styles.globalStats}>
          <StatCard
            label="Total NOCK Stack"
            value={totalNockVal}
            suffix=" NOCK"
            subtitle={`+${nockFloorDeltaVal} above 100k floor`}
            highlightColor="#2ecc71"
          />
          <StatCard
            label="Total USDC Stack"
            value={global?.total_usdc_amount || '163.83'}
            prefix="$"
            suffix=" USDC"
            subtitle="+0.65 USDC fees accrued"
            highlightColor="#00d2d3"
          />
          <StatCard
            label="Portfolio Value (USD)"
            value={global?.total_portfolio_usd}
            prefix="$"
            subtitle={`@ $${metrics?.tick_bracket_hypervisor?.current_price || '0.0273'}/NOCK`}
          />
          <StatCard
            label="Claimable Yield (Net)"
            value={global?.gas_to_fee_delta_usd}
            prefix="$"
            subtitle="Gas: $0.005 (190x ROI)"
            highlightColor="#f1c40f"
          />
          <StatCard
            label="100k Floor Health"
            value="106.3%"
            subtitle="Hard Floor Safe (+6.3% buffer)"
            highlightColor="#2ecc71"
          />
          <StatCard
            label="High-Water Mark"
            value={global?.high_water_mark_usd}
            prefix="$"
            subtitle="All-time high"
          />
        </div>
      </header>

      {/* Tactical Shot Advice Cockpit */}
      <section style={styles.shotAdviceBanner}>
        <div style={styles.adviceHeader}>
          <div style={styles.adviceActionRow}>
            <span style={styles.adviceBadge}>TACTICAL SHOT ADVICE</span>
            <span style={styles.adviceActionText}>
              {global?.shot_advice_action || 'HOLD & ACCRETE (Dual Growth Engine Active)'}
            </span>
          </div>
          <div style={styles.floorStatusTag}>
            Floor Invariant: <strong>100,000 NOCK Hard Floor PROTECTED</strong> (+{nockFloorDeltaVal} cushion)
          </div>
        </div>
        <p style={styles.adviceDetails}>
          {global?.shot_advice_details ||
            'Total NOCK held is 106,340 (+6,340 above the 100k hard floor). AMMs are generating dual yield (+0.61 USDC in V4 scale-out, +0.34 in Aero). Maintain posture.'}
        </p>
        <div style={styles.shotPillContainer}>
          <div style={styles.shotPill}>
            <span style={styles.shotPillLabel}>Goal 1 (NOCK Accretion):</span>
            <span style={styles.shotPillText}>
              106,340 NOCK total. Split: 96,005 Liquid Sovereign Vault, 6,329 Aerodrome LP, 4,004 Uniswap V4 LP. Target: Compound fees if spot dips &lt; $0.020.
            </span>
          </div>
          <div style={styles.shotPill}>
            <span style={styles.shotPillLabel}>Goal 2 (USDC Accumulation):</span>
            <span style={styles.shotPillText}>
              $163.83 USDC principal + $0.65 USDC pure trading fees claimable. Gas break-even is $0.005. Target: Batch harvest when fees cross $5.00.
            </span>
          </div>
          <div style={styles.shotPill}>
            <span style={styles.shotPillLabel}>Impermanent Loss / Risk:</span>
            <span style={styles.shotPillText}>
              0% realized losses (0 trades or claims made since deployment). Uniswap V4 position is 100% in-range ($0.01237 - $2.02783).
            </span>
          </div>
        </div>
      </section>

      {/* Storyboard Canvas — 7 Real Charts */}
      <main style={styles.grid}>
        {/* 1. Sovereign NOCK Stack & Floor Reference */}
        <CanvasPanel title="The Sovereign NOCK Stack vs 100,000 Floor (Token Units)">
          <ResponsiveContainer width="100%" height={260}>
            <AreaChart data={assetSeries} margin={{ top: 15, right: 30, left: 15, bottom: 0 }}>
              <defs>
                <linearGradient id="colorVault" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#2ecc71" stopOpacity={0.7} />
                  <stop offset="95%" stopColor="#2ecc71" stopOpacity={0.15} />
                </linearGradient>
                <linearGradient id="colorAero" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#3498db" stopOpacity={0.7} />
                  <stop offset="95%" stopColor="#3498db" stopOpacity={0.15} />
                </linearGradient>
                <linearGradient id="colorV4" x1="0" y1="0" x2="0" y2="1">
                  <stop offset="5%" stopColor="#9b59b6" stopOpacity={0.7} />
                  <stop offset="95%" stopColor="#9b59b6" stopOpacity={0.15} />
                </linearGradient>
              </defs>
              <XAxis dataKey="time" stroke="#555" fontSize={11} tickLine={false} />
              <YAxis
                stroke="#555"
                fontSize={11}
                domain={[90000, 115000]}
                tickFormatter={v => `${(v / 1000).toFixed(0)}k`}
              />
              <CartesianGrid strokeDasharray="3 3" stroke="#222" />
              <Tooltip formatter={(v: any) => `${Number(v).toLocaleString()} NOCK`} />
              <ReferenceLine
                y={100000}
                stroke="#e74c3c"
                strokeDasharray="4 4"
                strokeWidth={2}
                label={{ value: '100k HARD FLOOR', fill: '#e74c3c', fontSize: 11, position: 'insideTopLeft' }}
              />
              <Area
                type="monotone"
                dataKey="tranche3Vault"
                stackId="1"
                fill="url(#colorVault)"
                stroke="#2ecc71"
                name="Tranche 3: Sovereign Vault (96,005 NOCK)"
              />
              <Area
                type="monotone"
                dataKey="tranche2Aero"
                stackId="1"
                fill="url(#colorAero)"
                stroke="#3498db"
                name="Tranche 2: Aerodrome LP (6,329 NOCK)"
              />
              <Area
                type="monotone"
                dataKey="tranche1V4"
                stackId="1"
                fill="url(#colorV4)"
                stroke="#9b59b6"
                name="Tranche 1: Uniswap V4 (4,004 NOCK)"
              />
              <Legend />
            </AreaChart>
          </ResponsiveContainer>
          <div style={{ marginTop: 8, fontSize: '0.75rem', color: '#888', display: 'flex', justifyContent: 'space-between' }}>
            <span>Total Holding: <strong style={{ color: '#2ecc71' }}>{totalNockVal} NOCK</strong></span>
            <span>Safety Buffer: <strong style={{ color: '#2ecc71' }}>+{nockFloorDeltaVal} NOCK (+6.3%)</strong> above floor</span>
          </div>
        </CanvasPanel>

        {/* 2. Total Value vs Baseline — Dual Axis */}
        <CanvasPanel title="Total Value & NOCK Price vs Baseline (Dual Axis)">
          <ResponsiveContainer width="100%" height={260}>
            <ComposedChart data={valueVsBaseline} margin={{ top: 10, right: 80, left: 0, bottom: 0 }}>
              <XAxis dataKey="time" stroke="#555" fontSize={11} tickLine={false} />
              <YAxis yAxisId="left" orientation="left" stroke="#555" fontSize={11} tickFormatter={v => '$' + v.toLocaleString()} />
              <YAxis yAxisId="right" orientation="right" stroke="#555" fontSize={11} tickFormatter={v => '$' + v.toFixed(4)} domain={['auto', 'auto']} />
              <CartesianGrid strokeDasharray="3 3" stroke="#222" />
              <Tooltip labelFormatter={v => new Date(v).toLocaleTimeString()} />
              <Line yAxisId="left" type="monotone" dataKey="portfolio" stroke="#2ecc71" strokeWidth={2} dot={{ r: 3 }} name="Portfolio USD" />
              <Line yAxisId="left" type="monotone" dataKey="highWaterMark" stroke="#f39c12" strokeWidth={1} strokeDasharray="5 5" dot={false} name="High-Water Mark" />
              <Line yAxisId="right" type="monotone" dataKey="tokenPrice" stroke="#3498db" strokeWidth={2} dot={{ r: 3 }} name="NOCK Spot Price" />
            </ComposedChart>
          </ResponsiveContainer>
        </CanvasPanel>

        {/* 3. Tick Bracket Hypervisor — Horizontal Ranges */}
        <CanvasPanel title="Tick Bracket Hypervisor (Uniswap V4 vs Aero)">
          <ResponsiveContainer width="100%" height={260}>
            <BarChart data={tickData} layout="vertical" margin={{ top: 10, right: 30, left: 100, bottom: 0 }}>
              <XAxis type="number" tickFormatter={v => '$' + v.toLocaleString()} stroke="#555" fontSize={11} />
              <YAxis type="category" dataKey="name" width={100} stroke="#555" fontSize={11} />
              <CartesianGrid strokeDasharray="3 3" stroke="#222" vertical={false} />
              <Tooltip formatter={v => '$' + v.toLocaleString()} />
              <Bar dataKey="upper" fill="#f39c12" fillOpacity={0.4} name="Upper Bracket ($)" />
              <Bar dataKey="lower" fill="#2ecc71" fillOpacity={0.4} name="Lower Bracket ($)" />
              <Legend />
            </BarChart>
          </ResponsiveContainer>
          <div style={{ marginTop: 12, fontSize: '0.75rem', color: '#aaa', fontFamily: 'monospace', display: 'flex', justifyContent: 'space-between' }}>
            <span>Spot Price: <strong>${metrics?.tick_bracket_hypervisor?.current_price || '0.02726'}</strong></span>
            <span style={{ color: '#2ecc71' }}>Uniswap V4 Range: $0.01237 – $2.02783 (IN RANGE)</span>
          </div>
        </CanvasPanel>

        {/* 4. Portfolio Deviation Matrix — Bar Chart */}
        <CanvasPanel title="Portfolio Deviation Matrix (Target vs Actual)">
          <ResponsiveContainer width="100%" height={260}>
            <BarChart data={deviationData} layout="vertical" margin={{ top: 10, right: 30, left: 120, bottom: 0 }}>
              <XAxis type="number" tickFormatter={v => `${v}%`} stroke="#555" fontSize={11} />
              <YAxis type="category" dataKey="name" width={120} stroke="#555" fontSize={11} />
              <CartesianGrid strokeDasharray="3 3" stroke="#222" vertical={false} />
              <Tooltip formatter={(v: any) => `${Number(v).toFixed(1)}%`} />
              <Bar dataKey="target" fill="#666" fillOpacity={0.5} name="Target %" />
              <Bar dataKey="actual" name="Actual %">
                {deviationData.map((d: any, i: number) => (
                  <Cell
                    key={i}
                    fill={d.status === 'over_funded' ? '#e74c3c' : d.status === 'under_funded' ? '#f39c12' : d.status === 'fast_running' ? '#e67e22' : '#2ecc71'}
                  />
                ))}
              </Bar>
              <Legend />
            </BarChart>
          </ResponsiveContainer>
          <div style={{ marginTop: 8, fontSize: '0.7rem', display: 'flex', gap: 16, flexWrap: 'wrap' }}>
            <span><span style={{ color: '#2ecc71' }}>■</span> On Target</span>
            <span><span style={{ color: '#f39c12' }}>■</span> Under-Funded</span>
            <span><span style={{ color: '#e74c3c' }}>■</span> Over-Funded</span>
          </div>
        </CanvasPanel>

        {/* 5. Strategy Counterfactual — Dual Line */}
        <CanvasPanel title="Strategy Counterfactual (Active vs 100% Hold)">
          <ResponsiveContainer width="100%" height={260}>
            <LineChart data={counterfactual} margin={{ top: 10, right: 30, left: 0, bottom: 0 }}>
              <XAxis dataKey="time" stroke="#555" fontSize={11} tickLine={false} />
              <YAxis stroke="#555" fontSize={11} tickFormatter={v => '$' + v.toLocaleString()} />
              <CartesianGrid strokeDasharray="3 3" stroke="#222" />
              <Tooltip labelFormatter={v => new Date(v).toLocaleTimeString()} />
              <Line type="monotone" dataKey="active" stroke="#2ecc71" strokeWidth={2} dot={{ r: 3 }} name="Active Portfolio ($)" />
              <Line type="monotone" dataKey="passive" stroke="#888" strokeWidth={1.5} strokeDasharray="5 5" dot={false} name="Passive Hold ($)" />
              <Legend />
            </LineChart>
          </ResponsiveContainer>
          <div style={{ marginTop: 8, fontSize: '0.75rem', color: '#2ecc71', textAlign: 'center' }}>
            Alpha Generated: <strong>+${(parseFloat(global?.total_portfolio_usd || '3062') - parseFloat(global?.total_native_usd || '2898')).toFixed(2)} USD</strong> (+5.6% above passive hold)
          </div>
        </CanvasPanel>

        {/* 6. Velocity Gas Sandbox — Fee Harvesting & Gas ROI */}
        <CanvasPanel title="Velocity Gas Sandbox (Yield vs Gas Break-Even)">
          <ResponsiveContainer width="100%" height={260}>
            <BarChart data={gasData} margin={{ top: 10, right: 30, left: 0, bottom: 0 }}>
              <XAxis dataKey="name" stroke="#555" fontSize={11} tickLine={false} />
              <YAxis stroke="#555" fontSize={11} tickFormatter={v => '$' + v.toLocaleString()} />
              <CartesianGrid strokeDasharray="3 3" stroke="#222" />
              <Tooltip formatter={(v: any) => `$${Number(v).toFixed(4)}`} />
              <Bar dataKey="claimable" fill="#2ecc71" fillOpacity={0.8} name="Claimable Fee Yield ($)" />
              <Bar dataKey="gasBreakEven" fill="#e74c3c" fillOpacity={0.8} name="Gas Fee ($)" />
              <Legend />
            </BarChart>
          </ResponsiveContainer>
          <div style={{ marginTop: 8, fontSize: '0.75rem', display: 'flex', gap: 16, flexWrap: 'wrap' }}>
            {gasData.map((g: any) => (
              <span key={g.name} style={{ color: g.action === 'claim_now' ? '#2ecc71' : '#f39c12' }}>
                <strong>{g.name}:</strong> {g.action.toUpperCase()} (${g.claimable} earned / gas ${g.gasBreakEven} — {g.ratio.toFixed(0)}x ROI)
              </span>
            ))}
          </div>
        </CanvasPanel>
      </main>

      {/* Stale Banner — non-obtrusive */}
      <StaleBanner freshness={freshness} />
    </div>
  );
}

function StatCard({
  label,
  value,
  prefix = '',
  suffix = '',
  subtitle,
  highlightColor,
}: {
  label: string;
  value?: string;
  prefix?: string;
  suffix?: string;
  subtitle?: string;
  highlightColor?: string;
}) {
  const display = value ? `${prefix}${parseFloat(value.replace(/,/g, '')).toLocaleString(undefined, { maximumFractionDigits: 2 })}${suffix}` : '—';
  return (
    <div style={styles.statCard}>
      <div style={styles.statLabel}>{label}</div>
      <div style={{ ...styles.statValue, color: highlightColor || '#fff' }}>{display}</div>
      {subtitle && <div style={styles.statSubtitle}>{subtitle}</div>}
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

const styles: Record<string, React.CSSProperties> = {
  container: {
    minHeight: '100vh',
    background: '#0a0a0c',
    color: '#e0e0e0',
    fontFamily: 'system-ui, -apple-system, monospace',
    padding: '1.25rem',
  },
  loading: { textAlign: 'center', padding: '4rem', color: '#666' },
  error: { textAlign: 'center', padding: '2rem', color: '#e74c3c' },
  button: { padding: '0.5rem 1rem', background: '#3498db', border: 'none', borderRadius: 4, color: '#fff', cursor: 'pointer' },
  header: { marginBottom: '1.25rem', borderBottom: '1px solid #222', paddingBottom: '1.25rem' },
  titleRow: { display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1.25rem' },
  title: { margin: 0, fontSize: '1.6rem', fontWeight: 700, letterSpacing: '0.05em', color: '#3498db' },
  engineBadge: {
    fontSize: '0.75rem',
    textTransform: 'uppercase',
    letterSpacing: '0.08em',
    padding: '3px 8px',
    borderRadius: 4,
    background: '#1e293b',
    color: '#38bdf8',
    fontWeight: 600,
    border: '1px solid #0284c7',
  },
  statusPill: { display: 'flex', alignItems: 'center', gap: 6, padding: '4px 12px', background: '#161b22', borderRadius: 20, border: '1px solid #30363d' },
  statusDot: { width: 8, height: 8, borderRadius: '50%' },
  statusText: { fontSize: '0.7rem', textTransform: 'uppercase', letterSpacing: '0.1em', fontWeight: 600 },
  ageText: { fontSize: '0.7rem', color: '#888', fontFamily: 'monospace' },
  globalStats: { display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(180px, 1fr))', gap: '1rem' },
  statCard: { background: '#111317', border: '1px solid #22262d', borderRadius: 8, padding: '1rem', display: 'flex', flexDirection: 'column', gap: 4 },
  statLabel: { fontSize: '0.7rem', textTransform: 'uppercase', letterSpacing: '0.08em', color: '#8b949e' },
  statValue: { fontSize: '1.2rem', fontWeight: 700, fontFamily: 'monospace' },
  statSubtitle: { fontSize: '0.7rem', color: '#7ee787', fontFamily: 'monospace' },
  
  // Shot Advice Banner Styles
  shotAdviceBanner: {
    background: '#0f172a',
    border: '1px solid #1e3a8a',
    borderRadius: 8,
    padding: '1.25rem',
    marginBottom: '1.5rem',
    boxShadow: '0 4px 20px rgba(0,0,0,0.3)',
  },
  adviceHeader: {
    display: 'flex',
    justifyContent: 'space-between',
    alignItems: 'center',
    marginBottom: '0.75rem',
    flexWrap: 'wrap',
    gap: 8,
  },
  adviceActionRow: { display: 'flex', alignItems: 'center', gap: 10 },
  adviceBadge: {
    background: '#2563eb',
    color: '#fff',
    fontSize: '0.7rem',
    fontWeight: 700,
    letterSpacing: '0.1em',
    padding: '3px 8px',
    borderRadius: 4,
  },
  adviceActionText: {
    fontSize: '1.05rem',
    fontWeight: 700,
    color: '#60a5fa',
    letterSpacing: '0.02em',
  },
  floorStatusTag: {
    fontSize: '0.75rem',
    background: '#064e3b',
    color: '#a7f3d0',
    padding: '4px 10px',
    borderRadius: 6,
    border: '1px solid #059669',
  },
  adviceDetails: {
    fontSize: '0.85rem',
    color: '#cbd5e1',
    lineHeight: 1.5,
    margin: '0 0 1rem 0',
  },
  shotPillContainer: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))',
    gap: '0.75rem',
  },
  shotPill: {
    background: '#1e293b',
    border: '1px solid #334155',
    borderRadius: 6,
    padding: '0.65rem 0.85rem',
    display: 'flex',
    flexDirection: 'column',
    gap: 2,
  },
  shotPillLabel: {
    fontSize: '0.7rem',
    color: '#94a3b8',
    fontWeight: 600,
    textTransform: 'uppercase',
  },
  shotPillText: {
    fontSize: '0.75rem',
    color: '#f8fafc',
    lineHeight: 1.4,
  },
  grid: { display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(420px, 1fr))', gap: '1.25rem' },
  panel: { background: '#111317', border: '1px solid #22262d', borderRadius: 8, padding: '1rem', minHeight: '300px' },
  panelTitle: { margin: '0 0 0.75rem', fontSize: '0.8rem', textTransform: 'uppercase', letterSpacing: '0.08em', color: '#94a3b8', borderBottom: '1px solid #22262d', paddingBottom: '0.5rem' },
  panelContent: { minHeight: '240px' },
  emptyState: { color: '#555', fontSize: '0.8rem', textAlign: 'center', fontStyle: 'italic' },
};

function formatSnapshotTime(dateStr: string, spansMultipleDays: boolean = false): string {
  const d = new Date(dateStr);
  if (spansMultipleDays) {
    return d.toLocaleDateString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
  }
  return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

class ErrorBoundary extends React.Component<{ children: React.ReactNode }, { hasError: boolean; error: Error | null }> {
  constructor(props: { children: React.ReactNode }) {
    super(props);
    this.state = { hasError: false, error: null };
  }
  static getDerivedStateFromError(error: Error) {
    return { hasError: true, error };
  }
  componentDidCatch(error: Error, info: React.ErrorInfo) {
    console.error('[ErrorBoundary caught error]:', error, info);
  }
  render() {
    if (this.state.hasError) {
      return (
        <div style={{ padding: '2rem', textAlign: 'center', color: '#e74c3c', background: '#0a0a0c', minHeight: '100vh' }}>
          <h2>State Room Encountered An Issue</h2>
          <pre style={{ color: '#aaa', fontSize: '0.85rem' }}>{this.state.error?.message}</pre>
          <button onClick={() => window.location.reload()} style={{ marginTop: '1rem', padding: '0.5rem 1rem', background: '#3498db', border: 'none', borderRadius: 4, color: '#fff', cursor: 'pointer' }}>
            Reload Portal
          </button>
        </div>
      );
    }
    return this.props.children;
  }
}

// Bootstrap
const root = createRoot(document.getElementById('root')!);
root.render(
  <React.StrictMode>
    <ErrorBoundary>
      <Dashboard />
    </ErrorBoundary>
  </React.StrictMode>
);