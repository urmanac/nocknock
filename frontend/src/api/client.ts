/**
 * NockNock API Client
 * Generated types would come from openapi-typescript based on specs/core/api.yaml
 * This is a manual implementation until generation is wired up.
 */

const API_BASE = import.meta.env.VITE_API_BASE || '/v1';

export interface SnapshotMeta {
  generated_at: string;
  state_hash: string;
  version: string;
  snapshot_id: string;
}

import type { DataProvenance, RouteQuotes } from '../types/snapshot';

export interface SnapshotEnvelope<T = unknown> {
  meta: SnapshotMeta;
  data: T;
  /** Backfill telemetry (v0.3.0); computed live by the API, not part of state_hash */
  data_provenance?: DataProvenance;
}

export interface HealthResponse {
  status: 'ok' | 'degraded' | 'down';
  last_sync: string;
  ingestion_lag_seconds: number;
  sync_duration_ms?: number;
  sync_window_seconds?: number;
  errors?: string[];
}

export interface MetadataEntry {
  snapshot_id: string;
  generated_at: string;
  state_hash: string;
  age_seconds: number;
}

export interface MetadataResponse {
  snapshots: MetadataEntry[];
}


export interface ClaimFeeAction {
  platform: 'aerodrome' | 'aerodrome-cl' | 'uniswap-v4' | 'uniswap-v3';
  label: string;
  to: string;
  data: string;
  value: string;
  expected_amount_0?: string | null;
  expected_amount_1?: string | null;
  expected_usd?: string | null;
  gas_estimate?: string | null;
  gas_usd?: string | null;
  /** 'not_claimable': the Aero pool's claimFees() reverts, or an Aero CL position is
   *  staked in its gauge (simulation_detail says which); nothing to dispatch. */
  simulation: 'success' | 'failed' | 'unavailable' | 'not_claimable';
  simulation_detail?: string | null;
}

export interface ClaimFeesPayload {
  chain_id: number;
  from: string;
  generated_at: string;
  gross_usd?: string | null;
  estimated_gas_usd?: string | null;
  net_after_gas_usd?: string | null;
  actions: ClaimFeeAction[];
  /** Collectible LP fees: the same figure as gross_usd (#142) */
  collectible_lp_fees_usd?: string | null;
  /** Collectible gauge rewards in USD; null when not totalled (#142) */
  collectible_rewards_usd?: string | null;
  collectible_rewards_note?: string | null;
  /** LP fees plus gauge rewards; null whenever the rewards part is null (#142) */
  total_collectible_usd?: string | null;
  /** One row per position with a gauge reward (#142). Information only: a row has no
   *  `to` and no `data`, so it can never be dispatched to a wallet. Absent from a worker
   *  that predates #142 */
  rewards?: ClaimRewardRow[];
}

/** A position's gauge reward in the claim payload (#142): a fact, not an action. */
export interface ClaimRewardRow {
  position_id: string;
  label: string;
  status: 'read' | 'not_read';
  token_symbol: string;
  amount?: string | null;
  usd?: string | null;
  gauge?: string | null;
  collected_in: string;
  collect_url: string;
  note?: string | null;
  /** Gas units of the verified claim call (hex); null unless verified for this gauge */
  gas_estimate?: string | null;
  gas_usd?: string | null;
  /** The gas statement in words, e.g. why there is no estimate */
  gas_note: string;
}

export interface RecordHarvestResponse {
  recorded: Array<{
    platform: string;
    tx_hash: string;
    token_symbol: string;
    amount_human: string;
    usd_value?: string | null;
    block_number: number;
  }>;
  already_present: number;
  /** Liquidity the transaction returned, left out of the ledger (#156). */
  principal_excluded?: Array<{ token_symbol: string; token_address: string; amount_human: string }>;
}

export interface RecordAccumulationResponse {
  recorded: boolean;
  duplicate: boolean;
  plan_id: string;
  tx_hash: string;
  usdc_spent: string;
  nock_received: string;
  effective_price_usd: string;
  block_number: number;
  confirmed_at: string;
  /** Reference spot at the fill's block time and the cost over it (v0.6.0); null when unknown. */
  reference_spot_usd?: string | null;
  reference_spot_source?: string | null;
  execution_cost_bps?: number | null;
}

/** A receipt-verified fill from GET /accumulation/events. */
export interface AccumulationEvent {
  plan_id: string;
  tx_hash: string;
  /** The plan's venue label when recorded; null when it had none (v0.8.1, #132) */
  venue: string | null;
  usdc_spent: string;
  nock_received: string;
  effective_price_usd: string;
  block_number: number;
  confirmed_at: string;
  /** Null when no stored price was near the fill's block time (v0.6.0; absent before). */
  reference_spot_usd?: string | null;
  reference_spot_source?: string | null;
  execution_cost_bps?: number | null;
  /**
   * The best quote that was current for this fill's tranche, and the fill's cost over
   * it (v0.7.0, issue #115). Null together when no quote was on record; absent before.
   */
  quote_venue?: string | null;
  quote_route?: string | null;
  quote_usdc?: string | null;
  quote_nock_out?: string | null;
  quote_effective_price_usd?: string | null;
  quote_block_number?: number | null;
  quoted_at?: string | null;
  quote_cost_bps?: number | null;
}

/**
 * GET /accumulation/quotes/latest: the quotes the worker holds in memory for the due
 * tranche. It makes no call. Every field is null without a snapshot or a plan.
 */
export interface LatestRouteQuotesResponse {
  plan_id: string | null;
  status: string | null;
  tranche_index: number | null;
  tranche_usdc: string | null;
  route_quotes: RouteQuotes | null;
}

/** POST /accumulation/quotes/refresh: the due tranche quoted again at the latest block. */
export interface RefreshRouteQuotesResponse {
  plan_id: string;
  tranche_index: number;
  route_quotes: RouteQuotes;
}

export interface AccumulationEventsResponse {
  events: AccumulationEvent[];
}

/** Result of marking a detected swap "not part of the plan", or restoring it (v0.6.0). */
export interface FillCandidateActionResponse {
  tx_hash: string;
  state: 'offered' | 'dismissed';
  /** False when the swap was already in that state. */
  changed: boolean;
}

export interface LedgerSummary {
  cumulative_harvested_usd: string;
  tax_reserve_usd: string;
  growth_dry_powder_usd: string;
  harvest_count: number;
  last_harvest_at?: string | null;
  /** Configured ratio the split uses, e.g. "0.40"; "0.00" = none (2.4.0; absent before). */
  tax_reserve_ratio?: string;
  /** Harvest rows with no USD price at their block, left out of the total (2.4.0). */
  unpriced_count?: number;
}

export interface HistoricalResponse {
  snapshots: SnapshotEnvelope[];
  /** The worker's sync cadence, when it reports one (#61); the dashboard falls back to 300. */
  sync_window_seconds?: number;
  pagination: {
    limit: number;
    returned: number;
    has_more: boolean;
    next_cursor?: string;
  };
}

async function request<T>(path: string, init?: RequestInit): Promise<T> {
  const res = await fetch(`${API_BASE}${path}`, {
    ...init,
    headers: {
      'Content-Type': 'application/json',
      ...init?.headers,
    },
  });

  if (!res.ok) {
    const err = await res.json().catch(() => ({ message: res.statusText }));
    // Prefer the human-readable detail the backend provides; surface the machine code
    // ('error') last, and keep the HTTP status so callers can branch (e.g. 409 = already
    // recorded, which is an idempotent success rather than a failure).
    const errorMsg = (err && (err.detail || err.message || err.error)) || `HTTP ${res.status}`;
    throw Object.assign(new Error(errorMsg), {
      status: res.status,
      code: typeof err?.error === 'string' ? err.error : undefined,
      detail: typeof err?.detail === 'string' ? err.detail : undefined,
    });
  }

  if (res.status === 204) {
    return undefined as T;
  }

  return res.json();
}

/**
 * What to show for a failed API call. A thrown error with an HTTP status means the
 * worker answered, so it is not "unreachable". On a first run the worker answers 503
 * NO_SNAPSHOT and carries its last ingestion error as `detail`: show that cause.
 */
export function describeApiError(e: unknown): string {
  const msg = e instanceof Error ? e.message : String(e);
  const { status, code, detail } = (e ?? {}) as { status?: unknown; code?: unknown; detail?: unknown };
  if (typeof status !== 'number') return `Worker unreachable: ${msg}`;
  if (code === 'NO_SNAPSHOT') {
    return typeof detail === 'string' && detail.trim() !== ''
      ? `No snapshot yet. Last ingestion error: ${detail}`
      : 'No snapshot yet: the first ingestion cycle has not completed.';
  }
  return msg;
}

export const api = {
  health: () => request<HealthResponse>('/health'),

  latestSnapshot: <T = unknown>() => request<SnapshotEnvelope<T>>('/snapshot/latest', { cache: 'no-cache' }),

  historicalSnapshots: (params?: {
    limit?: number;
    before?: string;
    after?: string;
  }) => {
    const search = new URLSearchParams();
    if (params?.limit) search.set('limit', String(params.limit));
    if (params?.before) search.set('before', params.before);
    if (params?.after) search.set('after', params.after);
    return request<HistoricalResponse>(`/snapshot/historical?${search}`);
  },

  metadata: (limit = 20) => request<MetadataResponse>(`/snapshot/metadata?limit=${limit}`),

  snapshotById: <T = unknown>(id: string) => request<SnapshotEnvelope<T>>(`/snapshot/${id}`),

  // Golden Path execution (v0.2.0): unsigned claim payloads + receipt-verified ledger
  claimFeesPayload: () => request<ClaimFeesPayload>('/execute/claim-fees/payload'),

  recordHarvest: (platform: string, txHash: string) =>
    request<RecordHarvestResponse>('/execute/record-harvest', {
      method: 'POST',
      body: JSON.stringify({ platform, tx_hash: txHash }),
    }),

  // Accumulation plan (v0.5.0): attach a confirmed USDC→NOCK buy to the plan. The
  // worker accepts it only after verifying the on-chain receipt.
  recordAccumulation: (txHash: string) =>
    request<RecordAccumulationResponse>('/execute/record-accumulation', {
      method: 'POST',
      body: JSON.stringify({ tx_hash: txHash }),
    }),

  // Ask the worker to ingest now (used after recording a buy so the plan advances).
  ingestNow: () => request<unknown>('/ingest', { method: 'POST' }),

  // Receipt-verified fills, oldest first, with each fill's cost over spot (v0.6.0).
  accumulationEvents: () => request<AccumulationEventsResponse>('/accumulation/events'),

  // Fill detection (v0.6.0, issue #118). Neither call touches the plan ledger: they move
  // a detected swap between "offered" and "not part of the plan". Confirming an offer is
  // recordAccumulation with the offered hash; there is no second write path.
  dismissFillOffer: (txHash: string) =>
    request<FillCandidateActionResponse>('/accumulation/candidates/dismiss', {
      method: 'POST',
      body: JSON.stringify({ tx_hash: txHash }),
    }),

  restoreFillOffer: (txHash: string) =>
    request<FillCandidateActionResponse>('/accumulation/candidates/restore', {
      method: 'POST',
      body: JSON.stringify({ tx_hash: txHash }),
    }),

  // Route quotes (v0.7.0, issue #115): quote the due tranche again at the latest block.
  // It takes no amount: the worker quotes only the tranche its own plan says is due.
  refreshRouteQuotes: () =>
    request<RefreshRouteQuotesResponse>('/accumulation/quotes/refresh', { method: 'POST' }),

  // The quotes the worker holds now. The snapshot's ETag is its state hash and the
  // quotes are not part of it, so a revalidated snapshot can carry older ones.
  latestRouteQuotes: () =>
    request<LatestRouteQuotesResponse>('/accumulation/quotes/latest', { cache: 'no-store' }),
  // Average cost vs spot (v0.6.0, #98): verified fills with running averages, and stored spot.
  costBasisSeries: (days?: number) =>
    request<import('../costBasis').CostBasisSeries>(`/cost-basis/series${days ? `?days=${days}` : ''}`),

  // The ladder simulator (#166): a read-only what-if over the stored snapshot.
  simulateLadder: (query: URLSearchParams) => simulateLadder(query),

  // Positions found on-chain (v0.9.0, #145). One tap each: no hash, no amount and no
  // config is ever typed. `track` and `untrack` change what is counted, so the worker
  // runs a cycle after them and reports it as `cycle`. Nothing here builds a transaction.
  foundPositions: () => request<FoundPositionsResponse>('/positions/found', { cache: 'no-store' }),

  trackFoundPosition: (nftId: number, as: string) =>
    request<PositionTapResponse>('/positions/track', {
      method: 'POST',
      body: JSON.stringify({ nft_id: nftId, as }),
    }),

  untrackPosition: (positionId: string) =>
    request<PositionTapResponse>('/positions/untrack', {
      method: 'POST',
      body: JSON.stringify({ position_id: positionId }),
    }),

  dismissFoundPosition: (nftId: number) =>
    request<PositionTapResponse>('/positions/dismiss', {
      method: 'POST',
      body: JSON.stringify({ nft_id: nftId }),
    }),

  restoreFoundPosition: (nftId: number) =>
    request<PositionTapResponse>('/positions/restore', {
      method: 'POST',
      body: JSON.stringify({ nft_id: nftId }),
    }),

  // A WETH/NOCK bet found staked in the bet gauge (#181): the same writes, named by its
  // protocol (an Aero CL NFT id is not a Uniswap v4 id). Tracking it is `as: "bet"`.
  dismissFoundBet: (nftId: number) =>
    request<PositionTapResponse>('/positions/dismiss', {
      method: 'POST',
      body: JSON.stringify({ nft_id: nftId, protocol: 'aerodrome-cl' }),
    }),

  restoreFoundBet: (nftId: number) =>
    request<PositionTapResponse>('/positions/restore', {
      method: 'POST',
      body: JSON.stringify({ nft_id: nftId, protocol: 'aerodrome-cl' }),
    }),
};

/** A one-tap write on a found or tracked position (POST /v1/positions/*). */
export interface PositionTapResponse {
  status: string;
  nft_id?: number;
  position_id?: string;
  role?: string | null;
  rung?: string | null;
  deposit_tx?: string | null;
  changed?: boolean;
  note?: string;
  /** 'ok', or why the cycle after the write failed (the write is stored). */
  cycle?: string;
}

/** GET /v1/positions/found: the snapshot's detection block and the database rows. */
export interface FoundPositionsResponse {
  detection: import('../foundPositions').PositionDetection | null;
  tracked_rows: {
    position_id: string;
    nft_id: number;
    role: string | null;
    rung: string | null;
    source: string;
    deposit_tx: string | null;
    tracked_at: string | null;
    config_entry: string;
  }[];
  rollback_note: string;
}

/**
 * GET /v1/simulate/ladder (#166). Its own fetch, because a 400 or 422 body is a result
 * to show, not only an error message: the sub-code, `nock_needed` and the bounds are
 * kept. Throws only when the worker does not answer with one of the endpoint's bodies.
 */
export async function simulateLadder(
  query: URLSearchParams,
  fetchImpl: typeof fetch = fetch,
): Promise<import('../ladder').LadderOutcome> {
  const res = await fetchImpl(`${API_BASE}/simulate/ladder?${query.toString()}`, { cache: 'no-store' });
  const body = await res.json().catch(() => null);
  if (res.ok && body) return { ok: true, sim: body };
  if ((res.status === 400 || res.status === 422) && body && typeof body.error === 'string') {
    return { ok: false, status: res.status, error: body };
  }
  const msg = (body && (body.detail || body.message || body.error)) || res.statusText || `HTTP ${res.status}`;
  throw Object.assign(new Error(msg), {
    status: res.status,
    code: typeof body?.error === 'string' ? body.error : undefined,
    detail: typeof body?.detail === 'string' ? body.detail : undefined,
  });
}