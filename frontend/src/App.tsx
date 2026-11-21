import React from 'react';
import { LiteCockpit, WalletConnectBadge } from './components/LiteCockpit';
import { AccumulationPlanPanel } from './components/AccumulationPlanPanel';
import { AverageCostPanel, COST_PANEL_TITLE } from './components/AverageCostPanel';
import { LitePhoneSetup, LiteStatusCard } from './components/LitePhoneCards';
import { NotificationTestPanel } from './components/NotificationTestPanel';
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
  ReferenceArea,
} from 'recharts';
import { api, describeApiError } from './api/client';
import { useFreshness, StaleBanner } from './hooks/useFreshness.tsx';

/** market_price_history provenance tags (main.rs PRICE_SOURCE_*) → display names */
const SOURCE_LABELS: Record<string, string> = {
  geckoterminal: 'GeckoTerminal',
  dexscreener: 'DEXScreener',
  uniswap_rpc: 'Uniswap RPC',
  aerodrome_rpc: 'Aerodrome RPC (historical)',
};
import type { DerivedMetrics } from './types/snapshot';
import { POSITION_COLORS, ethBracketRows, fullRangeEthUsdcCount, hasNockLeg, nockBracketRows, notDrawnRangesText, positionBracket, positionLabel, positionRangeText, positionsSummary, protocolBadge, rangeStatus, statValueText } from './positions.ts';
import { isAeroProtocol, positionFeesText } from './aeroCl.ts';
import { PositionStakingLine, RangeSideList } from './components/PositionRangeNotes.tsx';
import { EthBracketChart } from './components/EthBracketChart.tsx';
import { reserveText } from './reserve.ts';
import { floorState, floorDeltaText, floorSummaryText, regimeLabel, regimeTone } from './floor.ts';
import { buildChartRows, compositionReadout, syncWindowOrDefault, type ChartSeries, type GapRow } from './timeline.ts';
import { ethUsdPrice, gasLine, hwmSubtitle, pillarLine } from './pillars.ts';
import { LEDGER_TITLE, ledgerView } from './ledger.ts';
import { aeroRewardsSentence, aeroStatSubtitle, compositionAeroText, compositionResidualLabel } from './aero.ts';
import { AeroPillarLines, PositionAeroLine } from './components/AeroPillar.tsx';
import { PositionEarningsLines, hasPositionEarnings } from './components/CollectibleEarnings.tsx';
import { ClosedPositionsCard } from './components/ClosedPositions.tsx';
import { priceSourceIsClosedText } from './closedPositions.ts';
import {
  LP_FEE_COLOR,
  LP_FEE_SERIES_NAME,
  REWARD_COLOR,
  REWARD_SERIES_NAME,
  claimableYieldTile,
  velocityHasRewards,
  velocityRewardRowText,
  velocityRowColor,
  velocityRows,
} from './rewards.ts';

function Dashboard() {
  // The worker's real cadence when the historical response reports it; 300 s otherwise.
  const [syncWindowSeconds, setSyncWindowSeconds] = React.useState<number>(syncWindowOrDefault(null));
  const { snapshot, loading, error, freshness, refetch, ageDisplay } = useFreshness<
    DerivedMetrics
  >(api.latestSnapshot, {
    syncWindowSeconds,
    pollIntervalMs: 10_000,
    debug: import.meta.env.DEV,
    onFreshnessChange: (fresh) => console.log('[Dashboard] Freshness:', fresh ? 'FRESH' : 'STALE'),
  });

  const [history, setHistory] = React.useState<any[]>([]);

  // Deep history (~24h at the 5-min sync cadence) powering every chart timeline.
  const loadHistory = React.useCallback(() => {
    api.historicalSnapshots({ limit: 288 })
      .then(res => {
        setSyncWindowSeconds(syncWindowOrDefault(res?.sync_window_seconds));
        if (res?.snapshots?.length) {
          setHistory([...res.snapshots].reverse());
        }
      })
      .catch(() => {}); // retried by the interval below while nothing has loaded
  }, []);

  React.useEffect(() => {
    loadHistory();
  }, [loadHistory, snapshot?.meta?.snapshot_id]);

  // Retry while empty so a single failed history load doesn't leave empty charts forever
  React.useEffect(() => {
    if (history.length > 0) return;
    const retry = setInterval(loadHistory, 60_000);
    return () => clearInterval(retry);
  }, [history.length, loadHistory]);

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
        <div style={styles.error}>{describeApiError(error)}</div>
        <button onClick={refetch} style={styles.button}>
          Retry
        </button>
      </div>
    );
  }

  const rawData = snapshot?.data as any;
  const metrics = rawData?.derived_metrics ?? rawData;
  const global = metrics?.global_status;
  // Market regime telemetry (v0.3.0): daily/micro RSI + third-party backfill provenance
  const provenance = snapshot?.data_provenance;
  const provenanceSources = Object.keys(provenance?.sources ?? {})
    .map(s => SOURCE_LABELS[s] ?? s)
    .join(', ');
  const provenanceCandleOnly = Object.keys(provenance?.sources ?? {}).length > 0 &&
    Object.keys(provenance?.sources ?? {}).every(s => s === 'geckoterminal');
  // Every time-series dataset (#61): real time on the x axis, breaks at gaps, the
  // composition from each row's own totals (N1), LP series keyed by position_id (N15).
  const charts = buildChartRows(history, rawData, syncWindowSeconds, snapshot?.meta?.generated_at);
  const { assetSeries, valueVsBaseline, composition, channel: channelSeries, positionSeriesDefs, spansMultipleDays } = charts;
  const positionsList: any[] = rawData?.positions || [];
  // Goal 1 counts only the ranges that hold NOCK (#151): a gas-side WETH/USDC range
  // holds none.
  const nockLpCount = positionsList.filter(hasNockLeg).length;
  // Built one per position, in the same order: a card's LP-fee reason comes from its row.
  const contractFees: any[] = rawData?.contract_fees || [];
  const compositionHasEth = composition.rows.some((r: any) => r.eth != null);

  const deviationData = (metrics?.portfolio_deviation_matrix?.allocations || []).map((a: any) => ({
    name: a.bucket_name,
    target: a.target_weight_bps / 100,
    actual: a.actual_weight_bps / 100,
    deviation: a.deviation_bps / 100,
    status: a.status,
  }));

  // Live ETH/USD: the snapshot's own eth_price_usd (2.3.0), the price the worker valued
  // gas and LP WETH at. Only a pre-2.3.0 snapshot falls back to a pool-derived ratio.
  // Null when neither exists — zero-mock: never valued at a made-up price.
  // NB: pool_mechanics is a sibling of derived_metrics on the snapshot data object.
  const ethUsd = ethUsdPrice(global, rawData?.pool_mechanics);

  // Bracket prices are token0 per token1. The chart is in USD per NOCK: brackets pair
  // with their positions by position_id (#152; index for older snapshots), a range with
  // no NOCK leg is left out, and ETH-per-NOCK brackets are converted with the live
  // ETH/USD (native units, collapsed and honest, when there is none).
  const tickData = nockBracketRows(
    metrics?.tick_bracket_hypervisor?.brackets,
    positionsList,
    ethUsd,
    parseFloat(metrics?.tick_bracket_hypervisor?.current_price || '0'),
  );
  // The ETH/USDC ranges (#160) get their own axis, USD per ETH: measured, never steered.
  const ethTickData = ethBracketRows(metrics?.tick_bracket_hypervisor?.brackets, positionsList, ethUsd);
  const notDrawnRanges = notDrawnRangesText(positionsList, ethTickData.map((r) => r.positionId));
  const ethFullRangeCount = fullRangeEthUsdcCount(positionsList);

  // Velocity rows (#142): LP fees and gauge rewards are two series, never one figure.
  const gasData = velocityRows(metrics?.velocity_gas_sandbox?.contracts);
  const gasHasRewards = velocityHasRewards(gasData);

  // Zero-mock: never fabricate a floor target. A missing field is '—'/unavailable; the
  // worker's "0" is the operator's own "no floor", shown as such in a neutral style.
  const floor = floorState(global?.nock_floor_target, global?.nock_floor_delta, global?.total_nock_amount);
  const floorKnown = floor.kind === 'set';
  const floorNone = floor.kind === 'none';
  const floorTarget = floor.kind === 'set' ? floor.target : 0;
  const floorTargetFormatted = floorKnown ? floorTarget.toLocaleString() : '—';
  const floorTargetShort = floorKnown
    ? (floorTarget >= 1000 ? `${(floorTarget / 1000).toFixed(0)}k` : floorTarget.toLocaleString())
    : '—';
  const nockAmountParsed = global?.total_nock_amount ? parseInt(global.total_nock_amount) : NaN;
  const nockAmountRaw = Number.isFinite(nockAmountParsed) ? nockAmountParsed : 0;
  // A known zero stack is 0, not unknown; only a missing field is '—'.
  const totalNockVal = Number.isFinite(nockAmountParsed) ? nockAmountParsed.toLocaleString() : '—';
  const nockFloorDeltaKnown = floor.kind === 'set' && floor.delta !== null;
  const nockFloorDeltaRaw = floor.kind === 'set' && floor.delta !== null ? floor.delta : 0;
  const isFloorSafe = nockFloorDeltaKnown ? nockFloorDeltaRaw >= 0 : false;
  const floorHealthColor = !nockFloorDeltaKnown ? '#8b949e' : isFloorSafe ? '#10b981' : '#ef4444';
  const nockFloorDeltaPct = nockFloorDeltaKnown && floorTarget > 0
    ? ((nockFloorDeltaRaw / floorTarget) * 100).toFixed(1)
    : null;
  const regimeToneNow = regimeTone(global?.regime);

  return (
    <div style={styles.container}>
      {/* Global Status Bar */}
      <header style={styles.header}>
        <div style={styles.titleRow}>
          <div style={{ display: 'flex', alignItems: 'center', gap: 12 }}>
            <h1 style={styles.title}>NockNock</h1>
            <span style={styles.engineBadge}>Dual-Growth Engine</span>
          </div>
          <div style={{ display: 'flex', alignItems: 'center', gap: 10 }}>
            <a
              href="/?view=lite"
              style={{
                display: 'inline-flex',
                alignItems: 'center',
                gap: 4,
                fontSize: 11,
                fontWeight: 600,
                letterSpacing: 1,
                color: '#38bdf8',
                textDecoration: 'none',
                border: '1px solid #0284c7',
                borderRadius: 999,
                padding: '4px 10px',
                background: '#0c4a6e22',
              }}
            >
              ⚡ LITE COCKPIT
            </a>
            <WalletConnectBadge />
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
        </div>

        {/* Primary Dual-Goal Metrics Bar */}
        <div style={styles.globalStats}>
          <StatCard
            label="Total NOCK Stack"
            value={global?.total_nock_amount}
            suffix=" NOCK"
            subtitle={floorSummaryText(floor)}
            highlightColor={floorHealthColor}
          />
          <StatCard
            label="Total USDC Stack"
            value={global?.total_usdc_amount}
            prefix="$"
            suffix=" USDC"
            subtitle={[`${statValueText(global?.total_fees_accrued_usd, '$')} of LP fees uncollected`, reserveText(global)].filter(Boolean).join(' · ')}
            highlightColor="#00d2d3"
          />
          <StatCard
            label="Portfolio Value (USD)"
            value={global?.total_portfolio_usd}
            prefix="$"
            subtitle={`${pillarLine(global)} · @ $${metrics?.tick_bracket_hypervisor?.current_price || '—'}/NOCK`}
          />
          {/* Collectible earnings (#142): LP fees and AERO rewards as two labelled figures
              and their total. An older snapshot renders the tile it always did. */}
          <StatCard
            label={claimableYieldTile(global, positionsList).label}
            value={claimableYieldTile(global, positionsList).value}
            prefix="$"
            subtitle={claimableYieldTile(global, positionsList).subtitle}
            highlightColor="#f1c40f"
          />
          <StatCard
            label={floorKnown ? `${floorTargetShort.toUpperCase()} Floor Health` : 'Floor Health'}
            value={floorKnown && Number.isFinite(nockAmountParsed) ? ((nockAmountRaw / floorTarget) * 100).toFixed(1) : undefined}
            suffix="%"
            subtitle={floorNone ? 'No floor configured' : !nockFloorDeltaKnown ? 'floor status unknown' : floorDeltaText(nockFloorDeltaRaw, floorTarget)}
            highlightColor={floorHealthColor}
          />
          <StatCard
            label="High-Water Mark"
            value={global?.high_water_mark_usd}
            prefix="$"
            subtitle={hwmSubtitle(global?.high_water_mark_date)}
          />
          {/* AERO, the fourth pillar (issue #93): an amount, never a "Goal". */}
          <StatCard
            label="AERO (wallet)"
            value={global?.wallet_aero_amount ?? undefined}
            suffix=" AERO"
            subtitle={aeroStatSubtitle(global, positionsList)}
            highlightColor="#fdba74"
          />
        </div>
        {/* Pillars (issue #94): ETH is gas, a pillar line and never a "Goal". */}
        <div style={styles.pillarLine}>
          <span>Pillars: {pillarLine(global)}</span>
          <span style={{ color: '#a78bfa' }}>{gasLine(global)}</span>
        </div>
        <AeroPillarLines global={global} positions={positionsList} />
      </header>

      {/* Tactical Shot Advice Cockpit */}
      <section style={styles.shotAdviceBanner}>
        <div style={styles.adviceHeader}>
          <div style={styles.adviceActionRow}>
            <span style={styles.adviceBadge}>TACTICAL SHOT ADVICE</span>
            <span style={styles.adviceActionText}>
              {global?.shot_advice_action || 'Advice unavailable'}
            </span>
          </div>
          <div style={{
            ...styles.floorStatusTag,
            background: !nockFloorDeltaKnown ? '#161b22' : isFloorSafe ? '#064e3b' : '#450a0a',
            color: !nockFloorDeltaKnown ? '#8b949e' : isFloorSafe ? '#a7f3d0' : '#fecaca',
            border: !nockFloorDeltaKnown ? '1px solid #30363d' : isFloorSafe ? '1px solid #059669' : '1px solid #dc2626',
          }}>
            {floorNone
              ? <>Floor: <strong>No floor configured</strong></>
              : !floorKnown
                ? <>Floor: <strong>— UNKNOWN</strong></>
                : <>Floor: <strong>{floorTargetFormatted} NOCK {!nockFloorDeltaKnown ? 'buffer unavailable' : isFloorSafe ? 'MET' : 'BREACHED'}</strong>{nockFloorDeltaKnown && <> ({floorDeltaText(nockFloorDeltaRaw, floorTarget)})</>}</>}
          </div>
        </div>
        <div style={styles.regimeRow}>
          {global?.regime && (
            <span
              style={{
                ...styles.regimeBadge,
                ...(regimeToneNow === 'warn'
                  ? { background: '#78350f', color: '#fcd34d', border: '1px solid #d97706' }
                  : regimeToneNow === 'ok'
                    ? { background: '#064e3b', color: '#a7f3d0', border: '1px solid #059669' }
                    : { background: '#161b22', color: '#8b949e', border: '1px solid #30363d' }),
              }}
            >
              REGIME: {regimeLabel(global.regime)}
            </span>
          )}
          <span style={styles.rsiChip}>RSI(14D): <strong style={{ color: '#e6edf3' }}>{global?.rsi_14_1d ?? '—'}</strong></span>
          <span style={styles.rsiChip}>RSI(5M): <strong style={{ color: '#e6edf3' }}>{global?.rsi_14 ?? '—'}</strong></span>
          {provenance && provenance.backfilled_points_count > 0 && (
            <span
              style={styles.provenanceTag}
              title={provenance.last_backfill_at ? `Last gap fill: ${provenance.last_backfill_at}` : undefined}
            >
              ✓ {provenance.backfilled_points_count} backfilled {provenanceCandleOnly ? 'candles' : 'price points'}{provenanceSources ? ` (${provenanceSources})` : ''}
            </span>
          )}
        </div>
        <p style={styles.adviceDetails}>
          {global?.shot_advice_details || '—'}
        </p>
        <div style={styles.shotPillContainer}>
          <div style={styles.shotPill}>
            <span style={styles.shotPillLabel}>Goal 1 (NOCK Accretion):</span>
            <span style={styles.shotPillText}>
              {totalNockVal} NOCK total across the Sovereign Treasury wallet and {nockLpCount} tracked NOCK LP position{nockLpCount === 1 ? '' : 's'}. {floorKnown ? `DEFENSIVE LOCK if total < ${floorTargetFormatted}.` : floorNone ? 'No floor configured.' : 'Floor unavailable.'}
            </span>
          </div>
          <div style={styles.shotPill}>
            <span style={styles.shotPillLabel}>Goal 2 (USDC Accumulation):</span>
            <span style={styles.shotPillText}>
              {statValueText(global?.total_usdc_amount, '$', ' USDC')} across wallet and AMM liquidity. {statValueText(global?.total_fees_claimable_usd, '$')} of LP fees uncollected.
            </span>
          </div>
          <div style={styles.shotPill}>
            <span style={styles.shotPillLabel}>LP Positions / Range Risk:</span>
            <span style={styles.shotPillText}>
              {positionsSummary(positionsList)} LP fees are read on-chain. {aeroRewardsSentence(global, positionsList)}
            </span>
          </div>
        </div>
      </section>

      {/* Tracked Positions — one card per live Uniswap position (v2.0.0) */}
      <section style={styles.positionsSection}>
        <div style={styles.positionsHeader}>TRACKED POSITIONS ({positionsList.length})</div>
        <div style={styles.positionsGrid}>
          {positionsList.map((p: any, pi: number) => (
            <div key={p.position_id} style={styles.positionCard}>
              <div style={styles.positionCardTop}>
                <span style={styles.positionPair}>{p.token0.symbol}/{p.token1.symbol}</span>
                <span style={{
                  ...styles.positionBadge,
                  ...(p.protocol === 'uniswap-v4'
                    ? { background: '#0c4a6e22', color: '#38bdf8', border: '1px solid #0284c7' }
                    : isAeroProtocol(p.protocol)
                      ? { background: '#7c2d1222', color: '#fdba74', border: '1px solid #c2410c' }
                      : { background: '#3b076422', color: '#c084fc', border: '1px solid #7e22ce' }),
                }}>
                  {protocolBadge(p.protocol)} · {p.fee_tier_bps / 100}%
                </span>
              </div>
              <div style={styles.positionCardMid}>
                <span style={styles.positionValue}>${p.amount_usd}</span>
                <span style={{
                  ...styles.rangePill,
                  ...(rangeStatus(p).tone === 'good'
                    ? { background: '#064e3b', color: '#a7f3d0', border: '1px solid #059669' }
                    : rangeStatus(p).tone === 'stale'
                      ? { background: '#3f1d1d', color: '#fca5a5', border: '1px solid #b91c1c' }
                      : { background: '#78350f', color: '#fcd34d', border: '1px solid #d97706' }),
                }} title={rangeStatus(p).tone === 'stale' ? 'This pool has no in-range liquidity, so its price is frozen.' : undefined}>
                  {rangeStatus(p).label}
                </span>
              </div>
              <div style={styles.positionCardMeta}>
                Range: {positionRangeText(p)}
              </div>
              {/* A position with a gauge reward (#142) shows it as its collectible
                  earnings, with the LP-fee figure or reason beneath. Any other position,
                  and any older snapshot, keeps the lines it had. */}
              {hasPositionEarnings(p) ? (
                <>
                  <div style={styles.positionCardMeta}>
                    {p.token0.symbol}: {p.amount0} ({p.token1.symbol}: {p.amount1})
                  </div>
                  <PositionEarningsLines position={p} feeRow={contractFees[pi]} style={styles.positionCardMeta} />
                  <PositionStakingLine position={p} style={styles.positionCardMeta} />
                </>
              ) : (
                <>
                  <div style={styles.positionCardMeta}>
                    {p.token0.symbol}: {p.amount0} ({p.token1.symbol}: {p.amount1}) · {positionFeesText(p)}
                  </div>
                  <PositionStakingLine position={p} style={styles.positionCardMeta} />
                  <PositionAeroLine position={p} style={styles.positionCardMeta} />
                </>
              )}
              <div style={styles.positionCardMeta}>
                {p.protocol === 'aerodrome' ? 'LP token' : `#${p.nft_id}`} · {p.network}{p.is_price_source ? ' · price source' : ''}
              </div>
            </div>
          ))}
          {positionsList.length === 0 && (
            <div style={styles.positionCardMeta}>No tracked positions configured.</div>
          )}
        </div>
        {/* Closed positions (#144): configured positions whose NFT no longer exists.
            Stated beside the live cards, never as one, and counted nowhere. */}
        <ClosedPositionsCard data={rawData} />
      </section>

      {/* Storyboard Canvas — Real Charts (structural channel panel appears when band data exists) */}
      <main style={styles.grid}>
        {/* 1. Sovereign NOCK Stack & Floor Reference */}
        <CanvasPanel title={floorNone ? 'The Sovereign NOCK Stack (Token Units, no floor configured)' : `The Sovereign NOCK Stack vs ${floorTargetFormatted} Floor (Token Units)`}>
          <ResponsiveContainer width="100%" height={260}>
            <AreaChart data={assetSeries.rows} margin={{ top: 15, right: 30, left: 15, bottom: 0 }}>
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
              {timeAxis(assetSeries, spansMultipleDays)}
              <YAxis
                stroke="#334155"
                tick={{ fill: '#94a3b8', fontSize: 11 }}
                domain={floorKnown ? [
                  (dataMin: number) => Math.max(0, Math.floor(Math.min(dataMin, floorTarget - 5000) / 5000) * 5000),
                  (dataMax: number) => Math.ceil(Math.max(dataMax, floorTarget + 5000) / 5000) * 5000,
                ] : ['auto', 'auto']}
                tickFormatter={v => `${(v / 1000).toFixed(0)}k`}
              />
              <CartesianGrid strokeDasharray="3 3" stroke="#222" />
              <Tooltip
                contentStyle={{ background: '#161b22', border: '1px solid #30363d', borderRadius: 6, color: '#f0f6fc' }}
                itemStyle={{ color: '#e6edf3', fontSize: '0.85rem' }}
                labelStyle={{ color: '#f0f6fc', fontWeight: 700, marginBottom: 4 }}
                formatter={(v: any) => `${Number(v).toLocaleString()} NOCK`}
                labelFormatter={tooltipTime}
              />
              {gapAreas(assetSeries.gaps)}
              {floorKnown && (
                <ReferenceLine
                  y={floorTarget}
                  stroke="#ef4444"
                  strokeDasharray="4 4"
                  strokeWidth={2}
                  label={{ value: `${floorTargetShort.toUpperCase()} HARD FLOOR`, fill: '#ef4444', fontSize: 11, position: 'insideTopLeft' }}
                />
              )}
              <Area
                type="monotone"
                dataKey="safeHaven"
                stackId="1"
                fill="url(#colorVault)"
                stroke="#10b981"
                name="Sovereign Vault (wallet NOCK)"
                connectNulls={false}
                dot={isolatedDot('#10b981')}
              />
              {positionSeriesDefs.map((def, i) => (
                <Area
                  key={def.key}
                  type="monotone"
                  dataKey={def.key}
                  stackId="1"
                  fill={POSITION_COLORS[i % POSITION_COLORS.length]}
                  fillOpacity={0.35}
                  stroke={POSITION_COLORS[i % POSITION_COLORS.length]}
                  name={def.name}
                  connectNulls={false}
                  dot={isolatedDot(POSITION_COLORS[i % POSITION_COLORS.length])}
                />
              ))}
              <Legend wrapperStyle={{ color: '#cbd5e1' }} formatter={(val) => <span style={{ color: '#cbd5e1' }}>{val}</span>} />
            </AreaChart>
          </ResponsiveContainer>
          <GapCaption gaps={assetSeries.gaps} />
          <div style={{ marginTop: 8, fontSize: '0.75rem', color: '#888', display: 'flex', justifyContent: 'space-between' }}>
            <span>Total Holding: <strong style={{ color: floorHealthColor }}>{totalNockVal} NOCK</strong></span>
            <span>Safety Buffer: <strong style={{ color: floorHealthColor }}>{nockFloorDeltaKnown ? `${floorDeltaText(nockFloorDeltaRaw, floorTarget)} (${nockFloorDeltaRaw >= 0 ? '+' : ''}${nockFloorDeltaPct}%)` : floorNone ? 'no floor configured' : '—'}</strong></span>
          </div>
        </CanvasPanel>

        {/* 2. Total Value vs Baseline — Dual Axis */}
        <CanvasPanel title="Total Value & NOCK Price vs Baseline (Dual Axis)">
          <ResponsiveContainer width="100%" height={260}>
            <ComposedChart data={valueVsBaseline.rows} margin={{ top: 10, right: 80, left: 0, bottom: 0 }}>
              {timeAxis(valueVsBaseline, spansMultipleDays)}
              <YAxis yAxisId="left" orientation="left" stroke="#334155" tick={{ fill: '#94a3b8', fontSize: 11 }} tickFormatter={v => '$' + v.toLocaleString()} />
              <YAxis yAxisId="right" orientation="right" stroke="#334155" tick={{ fill: '#94a3b8', fontSize: 11 }} tickFormatter={v => '$' + v.toFixed(4)} domain={['auto', 'auto']} />
              <CartesianGrid strokeDasharray="3 3" stroke="#222" />
              <Tooltip
                contentStyle={{ background: '#161b22', border: '1px solid #30363d', borderRadius: 6, color: '#f0f6fc' }}
                itemStyle={{ color: '#e6edf3', fontSize: '0.85rem' }}
                labelStyle={{ color: '#f0f6fc', fontWeight: 700, marginBottom: 4 }}
                labelFormatter={tooltipTime}
              />
              {gapAreas(valueVsBaseline.gaps, 'left')}
              <Line yAxisId="left" type="monotone" dataKey="portfolio" stroke="#10b981" strokeWidth={2} dot={{ r: 3 }} connectNulls={false} name="Portfolio USD" />
              <Line yAxisId="left" type="monotone" dataKey="highWaterMark" stroke="#f59e0b" strokeWidth={1} strokeDasharray="5 5" dot={isolatedDot('#f59e0b')} connectNulls={false} name="High-Water Mark" />
              <Line yAxisId="right" type="monotone" dataKey="tokenPrice" stroke="#38bdf8" strokeWidth={2} dot={{ r: 3 }} connectNulls={false} name="NOCK Spot Price" />
            </ComposedChart>
          </ResponsiveContainer>
          <GapCaption gaps={valueVsBaseline.gaps} />
        </CanvasPanel>

        {/* 3. Tick Bracket Hypervisor — Horizontal Ranges (one bar per NOCK range, all
            brackets converted to USD per NOCK so cross-protocol rows share one axis; the
            ETH/USDC ranges below on their own USD-per-ETH axis, #160) */}
        <CanvasPanel title={ethTickData.length > 0 ? 'Tick Bracket Hypervisor (NOCK and ETH/USDC ranges)' : 'Tick Bracket Hypervisor (NOCK ranges)'}>
          <ResponsiveContainer width="100%" height={260}>
            <BarChart data={tickData} layout="vertical" margin={{ top: 10, right: 30, left: 110, bottom: 0 }}>
              <XAxis type="number" tickFormatter={v => '$' + v.toFixed(4)} stroke="#334155" tick={{ fill: '#94a3b8', fontSize: 11 }} domain={[0, 'dataMax']} />
              <YAxis type="category" dataKey="name" width={110} stroke="#334155" tick={{ fill: '#cbd5e1', fontSize: 11 }} />
              <CartesianGrid strokeDasharray="3 3" stroke="#222" vertical={false} />
              <ReferenceLine x={parseFloat(metrics?.tick_bracket_hypervisor?.current_price || '0')} stroke="#e2b714" strokeDasharray="4 4" label={{ value: 'Spot', fill: '#e2b714', fontSize: 11, position: 'top' }} />
              <Tooltip
                contentStyle={{ background: '#161b22', border: '1px solid #30363d', borderRadius: 6, color: '#f0f6fc' }}
                itemStyle={{ color: '#e6edf3', fontSize: '0.85rem' }}
                labelStyle={{ color: '#f0f6fc', fontWeight: 700, marginBottom: 4 }}
                formatter={(v: any, name: any) => ['$' + Number(v).toFixed(5), name]}
              />
              <Bar dataKey="upper" fill="#f59e0b" fillOpacity={0.6} name="Upper Bracket ($)" />
              <Bar dataKey="lower" fill="#10b981" fillOpacity={0.6} name="Lower Bracket ($)" />
              <Legend wrapperStyle={{ color: '#cbd5e1' }} formatter={(val) => <span style={{ color: '#cbd5e1' }}>{val}</span>} />
            </BarChart>
          </ResponsiveContainer>
          <RangeSideList positions={positionsList} poolMechanics={rawData?.pool_mechanics} />
          <div style={{ marginTop: 12, fontSize: '0.75rem', color: '#aaa', fontFamily: 'monospace', display: 'flex', justifyContent: 'space-between', flexWrap: 'wrap', gap: 8 }}>
            <span>Spot Price: <strong>${metrics?.tick_bracket_hypervisor?.current_price || '0.00'}</strong></span>
            {notDrawnRanges && <span style={{ color: '#8b949e' }}>{notDrawnRanges}</span>}
            {tickData.some((t: any) => t.converted) && (
              <span style={{ color: '#8b949e' }}>ETH-denominated brackets converted to USD @ ${ethUsd ? ethUsd.toFixed(2) : '—'}/ETH ({global?.eth_price_usd ? 'snapshot ETH/USD' : 'pool-derived, older snapshot'})</span>
            )}
            {(() => {
              // The price source's bracket, by position_id (#152; by index for an
              // older snapshot). The price source is not necessarily the first one.
              const src = positionsList.find((p: any) => p.is_price_source);
              // #144: the spot can come from the pool of a closed Uniswap v4 entry.
              // There is then no price-source position and no range to state, and
              // another position's bracket must not stand in for it.
              const closedSource = priceSourceIsClosedText(rawData, positionsList);
              if (closedSource) {
                return <span style={{ color: '#8b949e' }}>{closedSource}</span>;
              }
              const brackets = metrics?.tick_bracket_hypervisor?.brackets || [];
              const bracket = src ? positionBracket(src, positionsList, brackets) : brackets[0];
              if (bracket?.full_range) {
                return (
                  <span style={{ color: '#10b981' }}>
                    {src ? positionLabel(src) : 'Price source'}: full range, never out of range
                  </span>
                );
              }
              if (!bracket?.lower_price) {
                return <span style={{ color: '#8b949e' }}>Price-source position range: —</span>;
              }
              return (
                <span style={{ color: bracket.in_range ? '#10b981' : '#f59e0b' }}>
                  {src ? positionLabel(src) : 'Price source'} range: {src ? positionRangeText(src) : '—'} ({bracket.in_range ? 'IN RANGE' : 'OUT OF RANGE'})
                </span>
              );
            })()}
          </div>
          <EthBracketChart
            rows={ethTickData}
            ethUsd={ethUsd}
            ethUsdFromSnapshot={!!global?.eth_price_usd}
            fullRangeCount={ethFullRangeCount}
          />
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
                    fill={d.status === 'over_funded' ? '#ef4444' : d.status === 'under_funded' ? '#f59e0b' : d.status === 'fast_running' ? '#f97316' : d.status === 'on_target' ? '#10b981' : '#8b949e'}
                  />
                ))}
              </Bar>
              {/* The 'actual' series is colored per-status via Cells, so its auto legend swatch
                  (series fill, unset => black) would lie. Legend shows only Target %; the
                  per-status color key below maps the real cell colors. */}
              <Legend
                payload={[{ value: 'Target %', type: 'square', color: '#38bdf8', id: 'target' }]}
                wrapperStyle={{ color: '#f1f5f9', paddingTop: 8 }}
                formatter={(value) => <span style={{ color: '#f1f5f9', fontWeight: 600, marginRight: 16 }}>{value}</span>}
              />
            </BarChart>
          </ResponsiveContainer>
          <div style={{ marginTop: 8, fontSize: '0.75rem', display: 'flex', gap: 16, flexWrap: 'wrap', color: '#cbd5e1' }}>
            <span><span style={{ color: '#38bdf8' }}>■</span> Target Weight</span>
            <span><span style={{ color: '#10b981' }}>■</span> Actual: On Target</span>
            <span><span style={{ color: '#f59e0b' }}>■</span> Actual: Under-Funded</span>
            <span><span style={{ color: '#ef4444' }}>■</span> Actual: Over-Funded</span>
            <span><span style={{ color: '#8b949e' }}>■</span> Actual: No Target Set (incl. Gas, AERO and non-NOCK LP: counted, never rebalanced)</span>
          </div>
        </CanvasPanel>

        {/* 5. Portfolio Composition — each row's own totals (N1): total, NOCK, USDC (and ETH
            once the snapshot reports it). Stable is total_stable_usd, never total − native,
            which also holds LP WETH: ETH is gas, not a stable reserve. */}
        <CanvasPanel title="Portfolio Composition: Total vs NOCK vs USDC">
          <ResponsiveContainer width="100%" height={260}>
            <LineChart data={composition.rows} margin={{ top: 10, right: 30, left: 0, bottom: 0 }}>
              {timeAxis(composition, spansMultipleDays)}
              <YAxis stroke="#334155" tick={{ fill: '#94a3b8', fontSize: 11 }} tickFormatter={v => '$' + v.toLocaleString()} />
              <CartesianGrid strokeDasharray="3 3" stroke="#222" />
              <Tooltip
                contentStyle={{ background: '#161b22', border: '1px solid #30363d', borderRadius: 6, color: '#f0f6fc' }}
                itemStyle={{ color: '#e6edf3', fontSize: '0.85rem' }}
                labelStyle={{ color: '#f0f6fc', fontWeight: 700, marginBottom: 4 }}
                formatter={(v: any) => `$${Number(v).toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`}
                labelFormatter={tooltipTime}
              />
              {gapAreas(composition.gaps)}
              <Line type="monotone" dataKey="active" stroke="#10b981" strokeWidth={2} dot={{ r: 3 }} connectNulls={false} name="Total Portfolio ($)" />
              <Line type="monotone" dataKey="passive" stroke="#94a3b8" strokeWidth={1.5} strokeDasharray="5 5" dot={isolatedDot('#94a3b8')} connectNulls={false} name="NOCK ($)" />
              <Line type="monotone" dataKey="stable" stroke="#00d2d3" strokeWidth={1.5} dot={isolatedDot('#00d2d3')} connectNulls={false} name="USDC ($, wallet + LP)" />
              {compositionHasEth && (
                <Line type="monotone" dataKey="eth" stroke="#a855f7" strokeWidth={1.5} dot={isolatedDot('#a855f7')} connectNulls={false} name="ETH ($, gas + LP WETH)" />
              )}
              <Legend wrapperStyle={{ color: '#cbd5e1' }} formatter={(val) => <span style={{ color: '#cbd5e1' }}>{val}</span>} />
            </LineChart>
          </ResponsiveContainer>
          <GapCaption gaps={composition.gaps} />
          {(() => {
            // Read from the LATEST snapshot, never the last history row (N1).
            const ro = compositionReadout(global);
            const usd = (v: number) => `$${v.toLocaleString(undefined, { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
            return (
              <div style={{ marginTop: 8, fontSize: '0.75rem', color: '#00d2d3', textAlign: 'center' }}>
                Stable Reserve (USDC in wallet + LP, latest snapshot): <strong>{ro.stableUsd === null ? '—' : `${usd(ro.stableUsd)} USD`}</strong>
                {ro.stableUsd !== null && (
                  <> ({ro.pctOfNative === null ? 'no NOCK value to compare' : `${ro.pctOfNative.toFixed(1)}% of NOCK value`})</>
                )}
                {ro.ethUsd !== null && (
                  <span style={{ color: '#a855f7' }}> · ETH {usd(ro.ethUsd)} (wallet gas + LP WETH; not cash)</span>
                )}
                {compositionAeroText(global) !== null && (
                  <span style={{ color: '#fdba74' }}> · {compositionAeroText(global)}</span>
                )}
                {ro.otherUsd !== null && Math.abs(ro.otherUsd) >= 0.01 && (
                  <span style={{ color: '#8b949e' }}> · {usd(ro.otherUsd)} in other legs ({compositionResidualLabel(global, ro.ethUsd !== null)})</span>
                )}
              </div>
            );
          })()}
        </CanvasPanel>

        {/* 5a. Accumulation Plan — DCA pacing + standing LP-rebuild bias (v0.5.0) */}
        <CanvasPanel title="Accumulation Plan (Dollar-Cost-Averaged NOCK Buys)">
          <AccumulationPlanPanel
            plan={metrics?.accumulation_plan}
            lpRebuild={metrics?.lp_rebuild}
            onRecorded={refetch}
          />
        </CanvasPanel>

        {/* 5b. Harvest ledger (virtual tax reserve) + change from the recorded peak (#102) */}
        <CanvasPanel title={LEDGER_TITLE}>
          {(() => {
            const view = ledgerView(metrics?.sovereign_ledger);
            const colors = ['#38bdf8', '#10b981'];
            const hwm = parseFloat(global?.high_water_mark_usd || '0');
            const current = parseFloat(global?.total_portfolio_usd || '0');
            const plPct = hwm > 0 ? ((current / hwm) - 1) * 100 : 0;
            const plColor = plPct >= 0 ? '#10b981' : '#f59e0b';
            const Dial = ({ label, value, pct, sub, color }: { label: string; value: string; pct: number; sub: string; color: string }) => (
              <div style={{ flex: 1, minWidth: 180 }}>
                <div style={{ fontSize: '0.7rem', letterSpacing: 2, color: '#8b949e', marginBottom: 6 }}>{label}</div>
                <div style={{ fontSize: '1.4rem', fontWeight: 700, color, marginBottom: 6 }}>${value}</div>
                <div style={{ background: '#0d1117', borderRadius: 999, height: 10, border: '1px solid #30363d', overflow: 'hidden' }}>
                  <div style={{ width: `${Math.min(100, Math.max(0, pct))}%`, background: color, height: '100%', borderRadius: 999 }} />
                </div>
                <div style={{ fontSize: '0.7rem', color: '#484f58', marginTop: 4 }}>{sub}</div>
              </div>
            );
            return (
              <div>
                <div style={{ display: 'flex', gap: 24, flexWrap: 'wrap', marginBottom: 14 }}>
                  {view.dials.map((d, i) => (
                    <Dial key={d.label} label={d.label} value={d.usd} pct={d.pct} sub={d.sub} color={colors[i % colors.length]} />
                  ))}
                  <div style={{ flex: 1, minWidth: 180 }}>
                    <div style={{ fontSize: '0.7rem', letterSpacing: 2, color: '#8b949e', marginBottom: 6 }}>CHANGE FROM RECORDED PEAK</div>
                    <div style={{ fontSize: '1.4rem', fontWeight: 700, color: hwm > 0 ? plColor : '#8b949e' }}>
                      {hwm > 0 ? `${plPct >= 0 ? '+' : ''}${plPct.toFixed(2)}%` : '—'}
                    </div>
                    <div style={{ fontSize: '0.7rem', color: '#484f58', marginTop: 10 }}>
                      ${current.toFixed(2)} vs HWM ${hwm.toFixed(2)} · {view.summary}
                    </div>
                  </div>
                </div>
                <div style={{ fontSize: '0.7rem', color: '#484f58', lineHeight: 1.6 }}>
                  {view.notes.join(' ')}
                </div>
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
              <Bar dataKey="claimable" fill={LP_FEE_COLOR} fillOpacity={0.85} name={gasHasRewards ? LP_FEE_SERIES_NAME : 'Claimable Fee Yield ($)'} />
              {/* A staked position's earnings (#142): its own series and legend entry. */}
              {gasHasRewards && <Bar dataKey="reward" fill={REWARD_COLOR} fillOpacity={0.9} name={REWARD_SERIES_NAME} />}
              <Bar dataKey="gasBreakEven" fill="#ef4444" fillOpacity={0.85} name="Gas Fee ($)" />
              <Legend wrapperStyle={{ color: '#cbd5e1' }} formatter={(val) => <span style={{ color: '#cbd5e1' }}>{val}</span>} />
            </BarChart>
          </ResponsiveContainer>
          <div style={{ marginTop: 8, fontSize: '0.75rem', display: 'flex', gap: 16, flexWrap: 'wrap' }}>
            {gasData.map((g) => (
              <span key={g.name} style={{ color: velocityRowColor(g) }}>
                {velocityRewardRowText(g)}
              </span>
            ))}
          </div>
        </CanvasPanel>

        {/* 7. Structural Channels — Bollinger Bands on Portfolio Value */}
        {channelSeries.rows.length > 0 && (
          <CanvasPanel title="Structural Channels (20-Period SMA ± 1σ/2σ Bands)">
            <ResponsiveContainer width="100%" height={260}>
              <LineChart data={channelSeries.rows} margin={{ top: 10, right: 30, left: 0, bottom: 0 }}>
                {timeAxis(channelSeries, spansMultipleDays)}
                <YAxis stroke="#334155" tick={{ fill: '#94a3b8', fontSize: 11 }} tickFormatter={v => '$' + v.toLocaleString()} domain={['auto', 'auto']} />
                <CartesianGrid strokeDasharray="3 3" stroke="#222" />
                <Tooltip
                  contentStyle={{ background: '#161b22', border: '1px solid #30363d', borderRadius: 6, color: '#f0f6fc' }}
                  itemStyle={{ color: '#e6edf3', fontSize: '0.85rem' }}
                  labelStyle={{ color: '#f0f6fc', fontWeight: 700, marginBottom: 4 }}
                  formatter={(v: any) => `$${Number(v).toLocaleString()}`}
                  labelFormatter={tooltipTime}
                />
                {gapAreas(channelSeries.gaps)}
                <Line type="monotone" dataKey="upper2" stroke="#7c3aed" strokeWidth={1} strokeDasharray="2 3" dot={isolatedDot('#7c3aed')} connectNulls={false} name="Upper 2σ" />
                <Line type="monotone" dataKey="upper1" stroke="#38bdf8" strokeWidth={1} strokeDasharray="3 3" dot={isolatedDot('#38bdf8')} connectNulls={false} name="Upper 1σ" />
                <Line type="monotone" dataKey="center" stroke="#f0f6fc" strokeWidth={2} dot={isolatedDot('#f0f6fc')} connectNulls={false} name="SMA(20) Center" />
                <Line type="monotone" dataKey="lower1" stroke="#38bdf8" strokeWidth={1} strokeDasharray="3 3" dot={isolatedDot('#38bdf8')} connectNulls={false} name="Lower 1σ" />
                <Line type="monotone" dataKey="lower2" stroke="#7c3aed" strokeWidth={1} strokeDasharray="2 3" dot={isolatedDot('#7c3aed')} connectNulls={false} name="Lower 2σ" />
                <Legend wrapperStyle={{ color: '#cbd5e1' }} formatter={(val) => <span style={{ color: '#cbd5e1' }}>{val}</span>} />
              </LineChart>
            </ResponsiveContainer>
            <GapCaption gaps={channelSeries.gaps} />
            <div style={{ marginTop: 8, fontSize: '0.75rem', color: '#888', textAlign: 'center' }}>
              Regression channel over historical portfolio values — band squeezes precede volatility expansion
            </div>
          </CanvasPanel>
        )}

        {/* 8. Average cost of the verified buys against spot (#98); a full row for the time axis */}
        <div style={{ gridColumn: '1 / -1' }}>
          <CanvasPanel title={COST_PANEL_TITLE}>
            <AverageCostPanel costBasis={metrics?.cost_basis} refreshKey={snapshot?.meta?.snapshot_id} />
          </CanvasPanel>
        </div>

        {/* Notification channels and a labelled test send (#89) */}
        <CanvasPanel title="Notifications (Channels & Test Send)">
          <NotificationTestPanel />
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
  const display = statValueText(value, prefix, suffix);
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
  pillarLine: { display: 'flex', flexWrap: 'wrap', gap: '0.5rem 1.5rem', marginTop: '0.75rem', fontSize: '0.8rem', color: '#cbd5e1' },
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
  // overflowWrap: a subtitle may carry an address or a reason with no space in it.
  statSubtitle: { fontSize: '0.7rem', color: '#7ee787', fontFamily: 'monospace', overflowWrap: 'anywhere' },
  
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
  regimeRow: {
    display: 'flex',
    alignItems: 'center',
    gap: 10,
    flexWrap: 'wrap',
    marginBottom: '0.75rem',
  },
  regimeBadge: {
    fontSize: '0.7rem',
    fontWeight: 700,
    letterSpacing: '0.1em',
    padding: '3px 10px',
    borderRadius: 4,
  },
  rsiChip: {
    fontSize: '0.75rem',
    color: '#94a3b8',
    background: '#1e293b',
    border: '1px solid #334155',
    borderRadius: 6,
    padding: '3px 10px',
  },
  provenanceTag: {
    fontSize: '0.7rem',
    color: '#8b949e',
    fontStyle: 'italic',
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
  positionsSection: {
    margin: '20px 24px 0',
    padding: 16,
    background: '#0d1117',
    border: '1px solid #1c2128',
    borderRadius: 12,
  },
  positionsHeader: {
    fontSize: '0.7rem',
    letterSpacing: 2,
    color: '#8b949e',
    marginBottom: 12,
  },
  positionsGrid: {
    display: 'grid',
    gridTemplateColumns: 'repeat(auto-fit, minmax(280px, 1fr))',
    gap: 12,
  },
  positionCard: {
    background: '#161b22',
    border: '1px solid #30363d',
    borderRadius: 10,
    padding: 14,
  },
  positionCardTop: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    marginBottom: 8,
    gap: 8,
  },
  positionPair: { fontSize: '1rem', fontWeight: 700, color: '#f0f6fc' },
  positionBadge: {
    fontSize: '0.6rem',
    fontWeight: 700,
    letterSpacing: 1,
    borderRadius: 999,
    padding: '3px 8px',
    whiteSpace: 'nowrap' as const,
  },
  positionCardMid: {
    display: 'flex',
    alignItems: 'baseline',
    justifyContent: 'space-between',
    marginBottom: 6,
    gap: 8,
  },
  positionValue: { fontSize: '1.3rem', fontWeight: 700, color: '#10b981' },
  rangePill: {
    fontSize: '0.62rem',
    fontWeight: 700,
    letterSpacing: 1,
    borderRadius: 999,
    padding: '3px 8px',
    whiteSpace: 'nowrap' as const,
  },
  positionCardMeta: { fontSize: '0.72rem', color: '#8b949e', lineHeight: 1.6 },
};

function formatSnapshotTime(when: string | number, spansMultipleDays: boolean = false): string {
  const d = new Date(when);
  if (spansMultipleDays) {
    return d.toLocaleDateString([], { month: 'short', day: 'numeric', hour: '2-digit', minute: '2-digit' });
  }
  return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

/** Tooltip header for a numeric time axis: date and time of the hovered row. */
function tooltipTime(t: any): string {
  const n = typeof t === 'number' ? t : Number(t);
  return Number.isFinite(n) ? formatSnapshotTime(n, true) : '';
}

/**
 * Numeric time x axis (#61): rows are spaced by elapsed time, and ticks come from real
 * rows only, so no label lands on a gap midpoint. Called as a function (not a component)
 * so Recharts still sees an XAxis child.
 */
function timeAxis(series: ChartSeries<{ t: number }>, multiDay: boolean) {
  return (
    <XAxis
      dataKey="t"
      type="number"
      scale="time"
      domain={['dataMin', 'dataMax']}
      ticks={series.ticks}
      tickFormatter={(t: number) => formatSnapshotTime(t, multiDay)}
      stroke="#334155"
      tick={{ fill: '#94a3b8', fontSize: 11 }}
      tickLine={false}
    />
  );
}

/** Shade each stretch with no stored snapshots, labelled literally (never "offline"). */
function gapAreas(gaps: GapRow[], yAxisId?: string) {
  return gaps.map(g => (
    <ReferenceArea
      key={`gap-${g.gapStart}`}
      x1={g.gapStart}
      x2={g.gapEnd}
      {...(yAxisId ? { yAxisId } : {})}
      fill="#64748b"
      fillOpacity={0.15}
      strokeOpacity={0}
      label={{ value: g.time, fill: '#94a3b8', fontSize: 10, position: 'insideTop' }}
    />
  ));
}

/**
 * Dot renderer that draws only rows with no real neighbour. A line needs two
 * consecutive defined points, so without a dot an isolated observation would vanish.
 */
function isolatedDot(color: string) {
  return (props: any) => {
    const { key, cx, cy, payload } = props ?? {};
    if (!payload?.isolated || !Number.isFinite(cx) || !Number.isFinite(cy)) return <g key={key} />;
    return <circle key={key} cx={cx} cy={cy} r={3} fill={color} stroke={color} />;
  };
}

/** Text under a time chart naming each gap, since the tooltip hides inside one. */
function GapCaption({ gaps }: { gaps: GapRow[] }) {
  if (gaps.length === 0) return null;
  const shown = gaps.slice(-3);
  return (
    <div style={{ marginTop: 6, fontSize: '0.7rem', color: '#8b949e', textAlign: 'center' }}>
      Shaded: {shown.map(g => `${g.time} (${formatSnapshotTime(g.gapStart, true)} – ${formatSnapshotTime(g.gapEnd, true)})`).join('; ')}
      {gaps.length > shown.length ? `; and ${gaps.length - shown.length} earlier` : ''}. Gaps are shown, never filled.
    </div>
  );
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
      {new URLSearchParams(window.location.search).get('view') === 'lite' ? (
        <LiteCockpit top={<LiteStatusCard />} bottom={<LitePhoneSetup />} />
      ) : (
        <Dashboard />
      )}
    </ErrorBoundary>
  </React.StrictMode>
);