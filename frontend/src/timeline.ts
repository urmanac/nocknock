/**
 * Chart timeline builders (issue #61, N1, N15).
 *
 * Pure functions, no React and no locale code, so vitest can reach them: `App.tsx`
 * mounts the app on import. Every time-series chart on the dashboard is built here.
 *
 * Rules these builders keep:
 * - Gaps in history are shown, never filled. When two stored snapshots are more than
 *   2.5x the sync window apart, a break row with no series keys goes between them, so
 *   every Recharts Line and stacked Area breaks there and the tooltip hides.
 * - A missing or non-numeric value is null, never 0. A real 0 stays 0.
 * - Each row of the composition chart is built from that row's OWN global_status totals
 *   (N1: the slimmed counterfactual series in a history row is ~8 h old).
 * - LP series are keyed by position_id, never by array position (N15).
 */
import { positionLabel, walletNockUnits } from './positions.ts';

/** The worker's default `sync_window_seconds` (main.rs `default_sync_window`). */
export const DEFAULT_SYNC_WINDOW_SECONDS = 300;

/**
 * A break needs at least two cycles in a row with no stored row. 2x would sit right on
 * the span of one missed cycle (598-602 s observed) and fire about half the time.
 */
export const GAP_CADENCE_MULTIPLIER = 2.5;

/** A finite number, or null. Never turns a missing value into 0. */
export function finiteOrNull(x: unknown): number | null {
  if (typeof x === 'number') return Number.isFinite(x) ? x : null;
  if (typeof x !== 'string') return null;
  const s = x.trim();
  if (s === '') return null;
  const n = Number(s);
  return Number.isFinite(n) ? n : null;
}

/** Epoch milliseconds of an ISO time, or null when it does not parse. */
export function timeMs(x: unknown): number | null {
  if (typeof x !== 'string' || x.trim() === '') return null;
  const t = Date.parse(x);
  return Number.isFinite(t) ? t : null;
}

/** The sync cadence in seconds: the given value when it is a finite number above 0, else 300. */
export function syncWindowOrDefault(syncWindowSeconds?: number | null): number {
  return typeof syncWindowSeconds === 'number' && Number.isFinite(syncWindowSeconds) && syncWindowSeconds > 0
    ? syncWindowSeconds
    : DEFAULT_SYNC_WINDOW_SECONDS;
}

/** Gap threshold in milliseconds: 2.5x the sync window (750000 at the 300 s default). */
export function gapThresholdMs(syncWindowSeconds?: number | null): number {
  return syncWindowOrDefault(syncWindowSeconds) * GAP_CADENCE_MULTIPLIER * 1000;
}

/** Whole minutes as '15m', '1h 0m', '19h 20m' or, from one day up, '1d 1h'. */
export function formatGapDuration(ms: number): string {
  const totalMin = Math.max(0, Math.round(ms / 60_000));
  const days = Math.floor(totalMin / 1440);
  const hours = Math.floor((totalMin % 1440) / 60);
  const mins = totalMin % 60;
  if (days > 0) return `${days}d ${hours}h`;
  if (hours > 0) return `${hours}h ${mins}m`;
  return `${mins}m`;
}

/**
 * A break row. It deliberately has no series keys, so every dataKey reads undefined
 * there. `time` is the caption text: literal, never "offline" (a quiet wallet whose
 * snapshots deduplicate would leave the same hole while the worker is up).
 */
export interface GapRow {
  gap: true;
  /** Midpoint of the gap, so the row has a real x on a numeric time axis. */
  t: number;
  time: string;
  gapStart: number;
  gapEnd: number;
  gapMs: number;
}

export function isGapRow(r: unknown): r is GapRow {
  return !!r && typeof r === 'object' && (r as { gap?: unknown }).gap === true;
}

/**
 * Insert a break row between consecutive rows more than 2.5x the sync window apart.
 * Rows keep their order and identity; no break is ever added at the start or the end,
 * and a NaN, zero or negative interval never produces one.
 */
export function withGapBreaks<T extends { t: number }>(
  rows: T[],
  syncWindowSeconds: number | null | undefined = DEFAULT_SYNC_WINDOW_SECONDS,
): Array<T | GapRow> {
  const threshold = gapThresholdMs(syncWindowSeconds);
  const out: Array<T | GapRow> = [];
  rows.forEach((row, i) => {
    if (i > 0) {
      const start = rows[i - 1].t;
      const dt = row.t - start;
      if (Number.isFinite(dt) && dt > threshold) {
        out.push({
          gap: true,
          t: start + Math.floor(dt / 2),
          time: `no snapshots for ${formatGapDuration(dt)}`,
          gapStart: start,
          gapEnd: row.t,
          gapMs: dt,
        });
      }
    }
    out.push(row);
  });
  return out;
}

/** A charted dataset: rows with breaks, the gaps to shade and caption, and real-row ticks. */
export interface ChartSeries<T extends { t: number }> {
  rows: Array<T | GapRow>;
  gaps: GapRow[];
  /** x ticks from real rows only, so no label lands on a gap midpoint. */
  ticks: number[];
}

/**
 * Sort real rows by time, drop rows with no usable time (they have no place on a time
 * axis), insert breaks, and mark rows with no real neighbour as `isolated`: a line needs
 * two consecutive defined points, so those rows are drawn as dots or they vanish.
 */
export function toChartSeries<T extends { t: number; isolated?: boolean }>(
  rows: T[],
  syncWindowSeconds?: number | null,
): ChartSeries<T> {
  const sorted = rows.filter((r) => Number.isFinite(r.t)).sort((a, b) => a.t - b.t);
  const withBreaks = withGapBreaks(sorted, syncWindowSeconds);
  withBreaks.forEach((r, i) => {
    if (isGapRow(r)) return;
    const prev = withBreaks[i - 1];
    const next = withBreaks[i + 1];
    const hasRealNeighbour = (prev !== undefined && !isGapRow(prev)) || (next !== undefined && !isGapRow(next));
    (r as T).isolated = !hasRealNeighbour;
  });
  return {
    rows: withBreaks,
    gaps: withBreaks.filter(isGapRow),
    ticks: sorted.map((r) => r.t),
  };
}

// ---------------------------------------------------------------------------
// Dataset builders
// ---------------------------------------------------------------------------

/** One stored snapshot as returned by GET /v1/snapshot/historical. */
export interface HistoryEntry {
  meta?: { generated_at?: string } | null;
  data?: any;
}

export interface AssetRow {
  t: number;
  isolated?: boolean;
  /** Wallet NOCK in token units; null when it cannot be recovered. */
  safeHaven: number | null;
  /** Wallet NOCK + the LP NOCK this row reports; null when wallet NOCK is unknown. */
  totalNock: number | null;
  /** LP NOCK per position, under the key of that position's series def. */
  [lpKey: string]: number | null | boolean | undefined;
}

export interface ValueRow {
  t: number;
  isolated?: boolean;
  portfolio: number | null;
  tokenPrice: number | null;
  highWaterMark: number | null;
}

/**
 * The composition chart's row, from that snapshot's own global_status totals.
 * `stable` is total_stable_usd, not total minus native: that difference also holds LP
 * WETH (and, once gas counts, wallet ETH), and ETH is gas, not a stable reserve.
 */
export interface CompositionRow {
  t: number;
  isolated?: boolean;
  /** total_portfolio_usd */
  active: number | null;
  /** total_native_usd (NOCK only) */
  passive: number | null;
  /** total_stable_usd (USDC in the wallet and in LP) */
  stable: number | null;
  /** total_eth_usd, when the snapshot reports it (spec 2.3.0); otherwise null */
  eth: number | null;
}

export interface ChannelRow {
  t: number;
  isolated?: boolean;
  center: number | null;
  upper1: number | null;
  lower1: number | null;
  upper2: number | null;
  lower2: number | null;
}

/** One stacked LP series. `retired` ids appear in history but not in the latest snapshot. */
export interface PositionSeriesDef {
  key: string;
  positionId: string;
  name: string;
  retired: boolean;
}

export interface ChartRows {
  assetSeries: ChartSeries<AssetRow>;
  valueVsBaseline: ChartSeries<ValueRow>;
  composition: ChartSeries<CompositionRow>;
  channel: ChartSeries<ChannelRow>;
  positionSeriesDefs: PositionSeriesDef[];
  /** True when the real rows span more than one calendar day (local time). */
  spansMultipleDays: boolean;
  /** True when the history path was used (two or more stored rows). */
  fromHistory: boolean;
}

/** derived_metrics of a snapshot's data, for both the envelope and the bare shape. */
export function derivedMetricsOf(data: any): any {
  return data?.derived_metrics ?? data;
}

/** Shorten a position id for a label: 'uniswap-v4:base:3163431' → 'uniswap-v4 #3163431'. */
function shortPositionId(id: string): string {
  const parts = id.split(':');
  if (parts.length >= 3) {
    const tail = parts[parts.length - 1];
    const short = tail.startsWith('0x') && tail.length > 10 ? `${tail.slice(0, 6)}…` : `#${tail}`;
    return `${parts[0]} ${short}`;
  }
  return id;
}

/**
 * The stacked LP series, keyed by position_id: the latest snapshot's order first (so
 * current colours stay put), then any id seen only in history, labelled as no longer
 * tracked. Keys are indexes into this list, so ids never become Recharts paths.
 */
export function buildPositionSeriesDefs(
  latestMetrics: any,
  positions: any[],
  history: HistoryEntry[] = [],
): PositionSeriesDef[] {
  const ids: string[] = [];
  const seen = new Set<string>();
  const add = (id: unknown) => {
    if (typeof id !== 'string' || id === '' || seen.has(id)) return;
    seen.add(id);
    ids.push(id);
  };
  const latestIds = new Set<string>();
  for (const s of latestMetrics?.asset_aggregation_series?.positions_nock ?? []) {
    add(s?.position_id);
    if (typeof s?.position_id === 'string') latestIds.add(s.position_id);
  }
  for (const p of positions ?? []) {
    if (typeof p?.position_id === 'string') latestIds.add(p.position_id);
  }
  for (const h of history) {
    for (const s of derivedMetricsOf(h?.data)?.asset_aggregation_series?.positions_nock ?? []) add(s?.position_id);
  }
  return ids.map((id, i) => {
    const pos = (positions ?? []).find((p: any) => p?.position_id === id);
    const retired = !latestIds.has(id);
    return {
      key: `lp${i}`,
      positionId: id,
      name: pos ? positionLabel(pos) : `${shortPositionId(id)} (no longer tracked)`,
      retired,
    };
  });
}

/**
 * One stacked-chart row from an asset_aggregation_series at index `i`. `dm` is the
 * derived_metrics of the same snapshot, so a pre-0.5.0 wallet NOCK can be recovered
 * from its own USD value and spot (issue #85).
 */
export function assetRow(ag: any, i: number, t: number, dm: any, defs: PositionSeriesDef[]): AssetRow {
  const spot = finiteOrNull(
    dm?.total_value_vs_baseline?.token_price_series?.[i] ?? dm?.tick_bracket_hypervisor?.current_price,
  );
  const walletNock = walletNockUnits(ag, i, spot);
  const keyOf = new Map(defs.map((d) => [d.positionId, d.key]));
  const row: AssetRow = {
    t,
    // Token units: this chart is NOCK against the NOCK floor, never dollars.
    safeHaven: walletNock === null ? null : Math.round(walletNock),
    totalNock: null,
  };
  let lpTotal = 0;
  for (const s of ag?.positions_nock ?? []) {
    const key = keyOf.get(s?.position_id);
    const v = finiteOrNull(s?.series?.[i]);
    if (v === null) continue;
    lpTotal += v;
    if (key) row[key] = Math.round(v);
  }
  row.totalNock = walletNock === null ? null : Math.round(lpTotal + walletNock);
  return row;
}

/** A composition row from one snapshot's own global_status. */
export function compositionRow(dm: any, t: number): CompositionRow {
  const gs = dm?.global_status;
  return {
    t,
    active: finiteOrNull(gs?.total_portfolio_usd),
    passive: finiteOrNull(gs?.total_native_usd),
    stable: finiteOrNull(gs?.total_stable_usd),
    eth: finiteOrNull(gs?.total_eth_usd),
  };
}

/**
 * The Stable Reserve readout, from the LATEST snapshot's own totals (never the last
 * history row). The percentage of native value is null when native value is 0 or
 * unknown: undefined, not 0.0%. `ethUsd` is total_eth_usd (2.3.0: wallet gas + LP WETH),
 * null on an older snapshot. `other` is total − native − stable − ETH − wallet AERO
 * (#93; AERO is its own pillar, shown beside the pillar line): about 0 on a 2.3.0
 * snapshot, and the LP WETH on an older one (which did not report ETH).
 */
export function compositionReadout(globalStatus: any): {
  stableUsd: number | null;
  nativeUsd: number | null;
  totalUsd: number | null;
  pctOfNative: number | null;
  ethUsd: number | null;
  otherUsd: number | null;
} {
  const totalUsd = finiteOrNull(globalStatus?.total_portfolio_usd);
  const nativeUsd = finiteOrNull(globalStatus?.total_native_usd);
  const stableUsd = finiteOrNull(globalStatus?.total_stable_usd);
  const ethUsd = finiteOrNull(globalStatus?.total_eth_usd);
  const pctOfNative = stableUsd !== null && nativeUsd !== null && nativeUsd > 0 ? (stableUsd / nativeUsd) * 100 : null;
  const otherUsd =
    totalUsd !== null && nativeUsd !== null && stableUsd !== null
      ? // "+ 0" turns a rounded -0 into 0.
        Math.round((totalUsd - nativeUsd - stableUsd - (ethUsd ?? 0) - (finiteOrNull(globalStatus?.wallet_aero_usd) ?? 0)) * 100) / 100 + 0
      : null;
  return { stableUsd, nativeUsd, totalUsd, pctOfNative, ethUsd, otherUsd };
}

function channelRows(latestMetrics: any): ChannelRow[] {
  const channel = latestMetrics?.structural_channels?.channels?.[0];
  if (!channel || !Array.isArray(channel.center_line)) return [];
  const rows: ChannelRow[] = [];
  channel.center_line.forEach((p: any, i: number) => {
    const t = timeMs(p?.time);
    if (t === null) return;
    rows.push({
      t,
      center: finiteOrNull(p?.value),
      upper1: finiteOrNull(channel.upper_1sd?.[i]?.value),
      lower1: finiteOrNull(channel.lower_1sd?.[i]?.value),
      upper2: finiteOrNull(channel.upper_2sd?.[i]?.value),
      lower2: finiteOrNull(channel.lower_2sd?.[i]?.value),
    });
  });
  return rows;
}

/** True when the given times fall on more than one local calendar day. */
export function spansMultipleDays(ts: number[]): boolean {
  const finite = ts.filter(Number.isFinite);
  if (finite.length < 2) return false;
  const min = Math.min(...finite);
  const max = Math.max(...finite);
  return new Date(min).toDateString() !== new Date(max).toDateString();
}

/**
 * Every time-series dataset on the dashboard.
 *
 * With two or more stored rows, each stored row becomes one chart row (history path).
 * Otherwise the latest snapshot's own series are used (a fresh database); its composition
 * point sits at `latestGeneratedAt` (the envelope's meta.generated_at). The structural
 * channel always comes from the latest snapshot's channel points.
 */
export function buildChartRows(
  history: HistoryEntry[],
  latestData: any,
  syncWindowSeconds?: number | null,
  latestGeneratedAt?: string | null,
): ChartRows {
  const latestMetrics = derivedMetricsOf(latestData);
  const positions: any[] = Array.isArray(latestData?.positions) ? latestData.positions : [];
  const fromHistory = history.length > 1;
  const defs = buildPositionSeriesDefs(latestMetrics, positions, fromHistory ? history : []);

  let asset: AssetRow[];
  let value: ValueRow[];
  let composition: CompositionRow[];

  if (fromHistory) {
    asset = [];
    value = [];
    composition = [];
    for (const s of history) {
      const t = timeMs(s?.meta?.generated_at);
      if (t === null) continue;
      const dm = derivedMetricsOf(s?.data);
      asset.push(assetRow(dm?.asset_aggregation_series, 0, t, dm, defs));
      const tv = dm?.total_value_vs_baseline;
      value.push({
        t,
        portfolio: finiteOrNull(tv?.portfolio_value_series?.[0]),
        tokenPrice: finiteOrNull(tv?.token_price_series?.[0]),
        highWaterMark: finiteOrNull(tv?.high_water_mark_series?.[0]),
      });
      composition.push(compositionRow(dm, t));
    }
  } else {
    const ag = latestMetrics?.asset_aggregation_series;
    asset = [];
    (ag?.time_points ?? []).forEach((tp: string, i: number) => {
      const t = timeMs(tp);
      if (t !== null) asset.push(assetRow(ag, i, t, latestMetrics, defs));
    });
    const tv = latestMetrics?.total_value_vs_baseline;
    value = [];
    (tv?.time_points ?? []).forEach((tp: string, i: number) => {
      const t = timeMs(tp);
      if (t === null) return;
      value.push({
        t,
        portfolio: finiteOrNull(tv?.portfolio_value_series?.[i]),
        tokenPrice: finiteOrNull(tv?.token_price_series?.[i]),
        highWaterMark: finiteOrNull(tv?.high_water_mark_series?.[i]),
      });
    });
    // The composition is the snapshot's own totals at its own time: one point.
    const latestT =
      timeMs(latestGeneratedAt) ??
      timeMs(tv?.time_points?.[tv.time_points.length - 1]) ??
      timeMs(ag?.time_points?.[ag.time_points.length - 1]);
    composition = latestT !== null && latestMetrics?.global_status ? [compositionRow(latestMetrics, latestT)] : [];
  }

  const assetSeries = toChartSeries(asset, syncWindowSeconds);
  const valueVsBaseline = toChartSeries(value, syncWindowSeconds);
  const compositionSeries = toChartSeries(composition, syncWindowSeconds);
  const channel = toChartSeries(channelRows(latestMetrics), syncWindowSeconds);

  return {
    assetSeries,
    valueVsBaseline,
    composition: compositionSeries,
    channel,
    positionSeriesDefs: defs,
    spansMultipleDays: spansMultipleDays([...assetSeries.ticks, ...valueVsBaseline.ticks, ...channel.ticks]),
    fromHistory,
  };
}
