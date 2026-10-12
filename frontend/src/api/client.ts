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
    throw new Error(err.message || `HTTP ${res.status}`);
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
};