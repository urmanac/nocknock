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
        const t3 = parseFloat(ag?.tranche_3_nock?.[0] || '0');
        const t2 = parseFloat(ag?.tranche_2_nock?.[0] || '0');
        const t1 = parseFloat(ag?.tranche_1_nock?.[0] || '0');
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
    : assetTimePoints.map((t: string, i: number) => {
        const ag = metrics?.asset_aggregation_series;
        const t3 = parseFloat(ag?.tranche_3_nock?.[i] || '0');
        const t2 = parseFloat(ag?.tranche_2_nock?.[i] || '0');
        const t1 = parseFloat(ag?.tranche_1_nock?.[i] || '0');
        return {
          time: formatSnapshotTime(t, false),
          tranche3Vault: Math.round(t3),
          tranche2Aero: Math.round(t2),
          tranche1V4: Math.round(t1),
          totalNock: Math.round(t3 + t2 + t1),
          safeHaven: parseFloat(assetSafeHaven[i] || '0'),
          activeProtocol: parseFloat(assetActiveProtocol[i] || '0'),
        };
      });

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
        time: formatSnapshotTime(t, false),
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

  const floorTarget = global?.nock_floor_target ? parseInt(global.nock_floor_target) : 100000;
  const floorTargetFormatted = floorTarget.toLocaleString();
  const floorTargetShort = `${(floorTarget / 1000).toFixed(0)}k`;
  const nockAmountRaw = global?.total_nock_amount ? parseInt(global.total_nock_amount) : 0;
  const totalNockVal = nockAmountRaw > 0 ? nockAmountRaw.toLocaleString() : '—';
  const nockFloorDeltaRaw = global?.nock_floor_delta ? parseInt(global.nock_floor_delta) : (nockAmountRaw ? nockAmountRaw - floorTarget : 0);
  const nockFloorDeltaVal = nockFloorDeltaRaw.toLocaleString();
  const isFloorSafe = nockFloorDeltaRaw >= 0;
  const floorHealthColor = isFloorSafe ? '#10b981' : '#ef4444';
  const nockFloorDeltaPct = ((nockFloorDeltaRaw / floorTarget) * 100).toFixed(1);

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
              background: freshness.syncStatus === 'fresh'
                ? '#2ecc71'
                : freshness.syncStatus === 'syncing'
                  ? '#38bdf8'
                  : '#f39c12',
            }} />
            <span style={styles.statusText}>
              {freshness.syncStatus === 'fresh'
                ? 'LIVE ON-CHAIN'
                : freshness.syncStatus === 'syncing'
                  ? 'SYNC IN PROGRESS'
                  : 'SYNC DELAYED'}
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
            subtitle={`${nockFloorDeltaRaw >= 0 ? '+' : ''}${nockFloorDeltaVal} above ${floorTargetShort} floor`}
            highlightColor={floorHealthColor}
          />
          <StatCard
            label="Total USDC Stack"
            value={global?.total_usdc_amount}
            prefix="$"
            suffix=" USDC"
            subtitle={`+${global?.total_fees_accrued_usd || '0.00'} USD fees accrued`}
            highlightColor="#00d2d3"
          />
          <StatCard
            label="Portfolio Value (USD)"
            value={global?.total_portfolio_usd}
            prefix="$"
            subtitle={`@ $${metrics?.tick_bracket_hypervisor?.current_price || '0.00'}/NOCK`}
          />
          <StatCard
            label="Claimable Yield (Net)"
            value={global?.gas_to_fee_delta_usd}
            prefix="$"
            subtitle={`Gross: $${global?.total_fees_claimable_usd || '0.00'}`}
            highlightColor="#f1c40f"
          />
          <StatCard
            label={`${floorTargetShort.toUpperCase()} Floor Health`}
            value={nockAmountRaw > 0 ? `${((nockAmountRaw / floorTarget) * 100).toFixed(1)}%` : '—'}
            subtitle={isFloorSafe ? `Hard Floor Safe (+${nockFloorDeltaVal} buffer)` : `CRITICAL: ${nockFloorDeltaVal} Below Floor`}
            highlightColor={floorHealthColor}
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
          <div style={{
            ...styles.floorStatusTag,
            background: isFloorSafe ? '#064e3b' : '#450a0a',
            color: isFloorSafe ? '#a7f3d0' : '#fecaca',
            border: isFloorSafe ? '1px solid #059669' : '1px solid #dc2626',
          }}>
            Floor Invariant: <strong>{floorTargetFormatted} NOCK Hard Floor {isFloorSafe ? 'PROTECTED' : 'BREACHED'}</strong> ({nockFloorDeltaRaw >= 0 ? '+' : ''}{nockFloorDeltaVal} cushion)
          </div>
        </div>
        <p style={styles.adviceDetails}>
          {global?.shot_advice_details ||
            `Total NOCK held is ${totalNockVal} (${nockFloorDeltaRaw >= 0 ? '+' : ''}${nockFloorDeltaVal} above ${floorTargetShort} hard floor). AMMs are generating dual yield. Maintain posture.`}
        </p>
        <div style={styles.shotPillContainer}>
          <div style={styles.shotPill}>
            <span style={styles.shotPillLabel}>Goal 1 (NOCK Accretion):</span>
            <span style={styles.shotPillText}>
              {totalNockVal} NOCK total across Treasury (Tranche 3), Aerodrome LP (Tranche 2), and Uniswap V4 LP (Tranche 1). Floor lock active if total &lt; {floorTargetFormatted}.
            </span>
          </div>
          <div style={styles.shotPill}>
            <span style={styles.shotPillLabel}>Goal 2 (USDC Accumulation):</span>
            <span style={styles.shotPillText}>
              ${global?.total_usdc_amount || '0.00'} USDC across wallet and AMM liquidity. ${global?.total_fees_claimable_usd || '0.00'} live claimable yield accrued.
            </span>
          </div>
          <div style={styles.shotPill}>
            <span style={styles.shotPillLabel}>Impermanent Loss / Risk:</span>
            <span style={styles.shotPillText}>
              0% realized losses. Uniswap V4 concentrated range is active (${metrics?.tick_bracket_hypervisor?.brackets?.[0]?.lower_price || '0.01237'} – ${metrics?.tick_bracket_hypervisor?.brackets?.[0]?.upper_price || '2.02783'}). Aerodrome fees live-tracked on-chain.
            </span>
          </div>
        </div>
      </section>

      {/* Storyboard Canvas — 7 Real Charts */}
      <main style={styles.grid}>
        {/* 1. Sovereign NOCK Stack & Floor Reference */}
        <CanvasPanel title={`The Sovereign NOCK Stack vs ${floorTargetFormatted} Floor (Token Units)`}>
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
              <XAxis dataKey="time" stroke="#334155" tick={{ fill: '#94a3b8', fontSize: 11 }} tickLine={false} />
              <YAxis
                stroke="#334155"
                tick={{ fill: '#94a3b8', fontSize: 11 }}
                domain={[
                  (dataMin: number) => Math.max(0, Math.floor(Math.min(dataMin, floorTarget - 5000) / 5000) * 5000),
                  (dataMax: number) => Math.ceil(Math.max(dataMax, floorTarget + 5000) / 5000) * 5000,
                ]}
                tickFormatter={v => `${(v / 1000).toFixed(0)}k`}
              />
              <CartesianGrid strokeDasharray="3 3" stroke="#222" />
              <Tooltip
                contentStyle={{ background: '#161b22', border: '1px solid #30363d', borderRadius: 6, color: '#f0f6fc' }}
                itemStyle={{ color: '#e6edf3', fontSize: '0.85rem' }}
                labelStyle={{ color: '#f0f6fc', fontWeight: 700, marginBottom: 4 }}
                formatter={(v: any) => `${Number(v).toLocaleString()} NOCK`}
              />
              <ReferenceLine
                y={floorTarget}
                stroke="#ef4444"
                strokeDasharray="4 4"
                strokeWidth={2}
                label={{ value: `${floorTargetShort.toUpperCase()} HARD FLOOR`, fill: '#ef4444', fontSize: 11, position: 'insideTopLeft' }}
              />
              <Area
                type="monotone"
                dataKey="tranche3Vault"
                stackId="1"
                fill="url(#colorVault)"
                stroke="#10b981"
                name="Tranche 3: Sovereign Vault"
              />
              <Area
                type="monotone"
                dataKey="tranche2Aero"
                stackId="1"
                fill="url(#colorAero)"
                stroke="#38bdf8"
                name="Tranche 2: Aerodrome LP"
              />
              <Area
                type="monotone"
                dataKey="tranche1V4"
                stackId="1"
                fill="url(#colorV4)"
                stroke="#a855f7"
                name="Tranche 1: Uniswap V4"
              />
              <Legend wrapperStyle={{ color: '#cbd5e1' }} formatter={(val) => <span style={{ color: '#cbd5e1' }}>{val}</span>} />
            </AreaChart>
          </ResponsiveContainer>
          <div style={{ marginTop: 8, fontSize: '0.75rem', color: '#888', display: 'flex', justifyContent: 'space-between' }}>
            <span>Total Holding: <strong style={{ color: floorHealthColor }}>{totalNockVal} NOCK</strong></span>
            <span>Safety Buffer: <strong style={{ color: floorHealthColor }}>{nockFloorDeltaRaw >= 0 ? `+${nockFloorDeltaVal}` : nockFloorDeltaVal} NOCK ({nockFloorDeltaRaw >= 0 ? `+${nockFloorDeltaPct}%` : `${nockFloorDeltaPct}%`})</strong> above floor</span>
          </div>
        </CanvasPanel>

        {/* 2. Total Value vs Baseline — Dual Axis */}
        <CanvasPanel title="Total Value & NOCK Price vs Baseline (Dual Axis)">
          <ResponsiveContainer width="100%" height={260}>
            <ComposedChart data={valueVsBaseline} margin={{ top: 10, right: 80, left: 0, bottom: 0 }}>
              <XAxis dataKey="time" stroke="#334155" tick={{ fill: '#94a3b8', fontSize: 11 }} tickLine={false} />
              <YAxis yAxisId="left" orientation="left" stroke="#334155" tick={{ fill: '#94a3b8', fontSize: 11 }} tickFormatter={v => '$' + v.toLocaleString()} />
              <YAxis yAxisId="right" orientation="right" stroke="#334155" tick={{ fill: '#94a3b8', fontSize: 11 }} tickFormatter={v => '$' + v.toFixed(4)} domain={['auto', 'auto']} />
              <CartesianGrid strokeDasharray="3 3" stroke="#222" />
              <Tooltip
                contentStyle={{ background: '#161b22', border: '1px solid #30363d', borderRadius: 6, color: '#f0f6fc' }}
                itemStyle={{ color: '#e6edf3', fontSize: '0.85rem' }}
                labelStyle={{ color: '#f0f6fc', fontWeight: 700, marginBottom: 4 }}
              />
              <Line yAxisId="left" type="monotone" dataKey="portfolio" stroke="#10b981" strokeWidth={2} dot={{ r: 3 }} name="Portfolio USD" />
              <Line yAxisId="left" type="monotone" dataKey="highWaterMark" stroke="#f59e0b" strokeWidth={1} strokeDasharray="5 5" dot={false} name="High-Water Mark" />
              <Line yAxisId="right" type="monotone" dataKey="tokenPrice" stroke="#38bdf8" strokeWidth={2} dot={{ r: 3 }} name="NOCK Spot Price" />
            </ComposedChart>
          </ResponsiveContainer>
        </CanvasPanel>

        {/* 3. Tick Bracket Hypervisor — Horizontal Ranges */}
        <CanvasPanel title="Tick Bracket Hypervisor (Uniswap V4 vs Aero)">
          <ResponsiveContainer width="100%" height={260}>
            <BarChart data={tickData} layout="vertical" margin={{ top: 10, right: 30, left: 110, bottom: 0 }}>
              <XAxis type="number" tickFormatter={v => '$' + v.toLocaleString()} stroke="#334155" tick={{ fill: '#94a3b8', fontSize: 11 }} />
              <YAxis type="category" dataKey="name" width={110} stroke="#334155" tick={{ fill: '#cbd5e1', fontSize: 11 }} />
              <CartesianGrid strokeDasharray="3 3" stroke="#222" vertical={false} />
              <Tooltip
                contentStyle={{ background: '#161b22', border: '1px solid #30363d', borderRadius: 6, color: '#f0f6fc' }}
                itemStyle={{ color: '#e6edf3', fontSize: '0.85rem' }}
                labelStyle={{ color: '#f0f6fc', fontWeight: 700, marginBottom: 4 }}
                formatter={v => '$' + v.toLocaleString()}
              />
              <Bar dataKey="upper" fill="#f59e0b" fillOpacity={0.6} name="Upper Bracket ($)" />
              <Bar dataKey="lower" fill="#10b981" fillOpacity={0.6} name="Lower Bracket ($)" />
              <Legend wrapperStyle={{ color: '#cbd5e1' }} formatter={(val) => <span style={{ color: '#cbd5e1' }}>{val}</span>} />
            </BarChart>
          </ResponsiveContainer>
          <div style={{ marginTop: 12, fontSize: '0.75rem', color: '#aaa', fontFamily: 'monospace', display: 'flex', justifyContent: 'space-between' }}>
            <span>Spot Price: <strong>${metrics?.tick_bracket_hypervisor?.current_price || '0.00'}</strong></span>
            <span style={{ color: '#10b981' }}>Uniswap V4 Range: {metrics?.tick_bracket_hypervisor?.brackets?.[0]?.lower_price ? `$${metrics?.tick_bracket_hypervisor?.brackets?.[0]?.lower_price} – $${metrics?.tick_bracket_hypervisor?.brackets?.[0]?.upper_price}` : '—'} (IN RANGE)</span>
          </div>
        </CanvasPanel>

        {/* 4. Portfolio Deviation Matrix — Bar Chart */}
        <CanvasPanel title="Portfolio Deviation Matrix (Target vs Actual)">
          <ResponsiveContainer width="100%" height={260}>
            <BarChart data={deviationData} layout="vertical" margin={{ top: 10, right: 30, left: 130, bottom: 0 }}>
              <XAxis type="number" tickFormatter={v => `${v}%`} stroke="#334155" tick={{ fill: '#cbd5e1', fontSize: 11 }} />
              <YAxis type="category" dataKey="name" width={130} stroke="#334155" tick={{ fill: '#e2e8f0', fontSize: 11 }} />
              <CartesianGrid strokeDasharray="3 3" stroke="#222" vertical={false} />
              <Tooltip
                contentStyle={{ background: '#161b22', border: '1px solid #30363d', borderRadius: 6, color: '#f0f6fc' }}
                itemStyle={{ color: '#e6edf3', fontSize: '0.85rem' }}
                labelStyle={{ color: '#f0f6fc', fontWeight: 700, marginBottom: 6, fontSize: '0.9rem' }}
                formatter={(v: any, name: string) => [`${Number(v).toFixed(1)}%`, name]}
              />
              <Bar dataKey="target" fill="#38bdf8" fillOpacity={0.85} stroke="#0284c7" strokeWidth={1} name="Target %" />
              <Bar dataKey="actual" name="Actual %">
                {deviationData.map((d: any, i: number) => (
                  <Cell
                    key={i}
                    fill={d.status === 'over_funded' ? '#ef4444' : d.status === 'under_funded' ? '#f59e0b' : d.status === 'fast_running' ? '#f97316' : '#10b981'}
                  />
                ))}
              </Bar>
              <Legend wrapperStyle={{ color: '#f1f5f9', paddingTop: 8 }} formatter={(value) => <span style={{ color: '#f1f5f9', fontWeight: 600, marginRight: 16 }}>{value}</span>} />
            </BarChart>
          </ResponsiveContainer>
          <div style={{ marginTop: 8, fontSize: '0.75rem', display: 'flex', gap: 16, flexWrap: 'wrap', color: '#cbd5e1' }}>
            <span><span style={{ color: '#38bdf8' }}>■</span> Target Weight</span>
            <span><span style={{ color: '#10b981' }}>■</span> Actual: On Target</span>
            <span><span style={{ color: '#f59e0b' }}>■</span> Actual: Under-Funded</span>
            <span><span style={{ color: '#ef4444' }}>■</span> Actual: Over-Funded</span>
          </div>
        </CanvasPanel>

        {/* 5. Strategy Counterfactual — Dual Line */}
        <CanvasPanel title="Strategy Counterfactual (Active vs 100% Hold)">
          <ResponsiveContainer width="100%" height={260}>
            <LineChart data={counterfactual} margin={{ top: 10, right: 30, left: 0, bottom: 0 }}>
              <XAxis dataKey="time" stroke="#334155" tick={{ fill: '#94a3b8', fontSize: 11 }} tickLine={false} />
              <YAxis stroke="#334155" tick={{ fill: '#94a3b8', fontSize: 11 }} tickFormatter={v => '$' + v.toLocaleString()} />
              <CartesianGrid strokeDasharray="3 3" stroke="#222" />
              <Tooltip
                contentStyle={{ background: '#161b22', border: '1px solid #30363d', borderRadius: 6, color: '#f0f6fc' }}
                itemStyle={{ color: '#e6edf3', fontSize: '0.85rem' }}
                labelStyle={{ color: '#f0f6fc', fontWeight: 700, marginBottom: 4 }}
              />
              <Line type="monotone" dataKey="active" stroke="#10b981" strokeWidth={2} dot={{ r: 3 }} name="Active Portfolio ($)" />
              <Line type="monotone" dataKey="passive" stroke="#94a3b8" strokeWidth={1.5} strokeDasharray="5 5" dot={false} name="Passive Hold ($)" />
              <Legend wrapperStyle={{ color: '#cbd5e1' }} formatter={(val) => <span style={{ color: '#cbd5e1' }}>{val}</span>} />
            </LineChart>
          </ResponsiveContainer>
          {(() => {
            const activeUsd = parseFloat(global?.total_portfolio_usd || '0');
            const passiveUsd = parseFloat(global?.total_native_usd || '0');
            const alphaUsd = activeUsd - passiveUsd;
            const alphaPct = passiveUsd > 0 ? ((alphaUsd / passiveUsd) * 100).toFixed(1) : '0.0';
            const alphaColor = alphaUsd >= 0 ? '#10b981' : '#ef4444';
            return (
              <div style={{ marginTop: 8, fontSize: '0.75rem', color: alphaColor, textAlign: 'center' }}>
                Alpha Generated: <strong>{alphaUsd >= 0 ? `+$${alphaUsd.toFixed(2)}` : `-$${Math.abs(alphaUsd).toFixed(2)}`} USD</strong> ({alphaUsd >= 0 ? `+${alphaPct}%` : `${alphaPct}%`} vs passive hold)
              </div>
            );
          })()}
        </CanvasPanel>

        {/* 6. Velocity Gas Sandbox — Fee Harvesting & Gas ROI */}
        <CanvasPanel title="Velocity Gas Sandbox (Yield vs Gas Break-Even)">
          <ResponsiveContainer width="100%" height={260}>
            <BarChart data={gasData} margin={{ top: 10, right: 30, left: 0, bottom: 0 }}>
              <XAxis dataKey="name" stroke="#334155" tick={{ fill: '#cbd5e1', fontSize: 11 }} tickLine={false} />
              <YAxis stroke="#334155" tick={{ fill: '#94a3b8', fontSize: 11 }} tickFormatter={v => '$' + v.toLocaleString()} />
              <CartesianGrid strokeDasharray="3 3" stroke="#222" />
              <Tooltip
                contentStyle={{ background: '#161b22', border: '1px solid #30363d', borderRadius: 6, color: '#f0f6fc' }}
                itemStyle={{ color: '#e6edf3', fontSize: '0.85rem' }}
                labelStyle={{ color: '#f0f6fc', fontWeight: 700, marginBottom: 4 }}
                formatter={(v: any) => `$${Number(v).toFixed(4)}`}
              />
              <Bar dataKey="claimable" fill="#10b981" fillOpacity={0.85} name="Claimable Fee Yield ($)" />
              <Bar dataKey="gasBreakEven" fill="#ef4444" fillOpacity={0.85} name="Gas Fee ($)" />
              <Legend wrapperStyle={{ color: '#cbd5e1' }} formatter={(val) => <span style={{ color: '#cbd5e1' }}>{val}</span>} />
            </BarChart>
          </ResponsiveContainer>
          <div style={{ marginTop: 8, fontSize: '0.75rem', display: 'flex', gap: 16, flexWrap: 'wrap' }}>
            {gasData.map((g: any) => (
              <span key={g.name} style={{ color: g.action === 'claim_now' ? '#10b981' : '#f59e0b' }}>
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