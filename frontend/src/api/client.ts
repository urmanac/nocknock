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

export interface SnapshotEnvelope<T = unknown> {
  meta: SnapshotMeta;
  data: T;
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
  platform: 'aerodrome' | 'uniswap-v4';
  label: string;
  to: string;
  data: string;
  value: string;
  expected_amount_0?: string | null;
  expected_amount_1?: string | null;
  expected_usd?: string | null;
  gas_estimate?: string | null;
  gas_usd?: string | null;
  simulation: 'success' | 'failed' | 'unavailable';
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
}

export interface LedgerSummary {
  cumulative_harvested_usd: string;
  tax_reserve_usd: string;
  growth_dry_powder_usd: string;
  harvest_count: number;
  last_harvest_at?: string | null;
}

export interface HistoricalResponse {
  snapshots: SnapshotEnvelope[];
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
    throw Object.assign(new Error(errorMsg), { status: res.status });
  }

  if (res.status === 204) {
    return undefined as T;
  }

  return res.json();
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
};