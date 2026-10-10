import { useState, useEffect, useCallback, useRef } from 'react';

// ============================================================
// Types (mirroring snapshot.yaml SnapshotMeta)
// ============================================================

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

export interface FreshnessState {
  isFresh: boolean;
  ageSeconds: number;
  stateHash: string;
  lastChecked: number;
  staleSince: number | null; // timestamp when data became stale
}

export interface UseFreshnessOptions {
  /** Sync window in seconds (default 300 = 5 minutes) */
  syncWindowSeconds?: number;
  /** Polling interval for freshness check (default 10 seconds) */
  pollIntervalMs?: number;
  /** Callback when freshness changes */
  onFreshnessChange?: (isFresh: boolean) => void;
  /** Enable debug logging */
  debug?: boolean;
}

/**
 * Core freshness validation hook.
 *
 * Contract (matching backend):
 * - Backend provides `meta.generated_at` (ISO8601) and `meta.state_hash` (SHA-256)
 * - Frontend computes `age = now - generated_at`
 * - If `age > syncWindowSeconds` → data is stale
 * - Stale data shows non-obtrusive banner; NO push alerts, NO blinking, NO modals
 * - Freshness check runs on mount + poll interval + when new snapshot arrives
 */
export function useFreshness<T = unknown>(
  fetchSnapshot: () => Promise<SnapshotEnvelope<T> | null>,
  options: UseFreshnessOptions = {}
) {
  const {
    syncWindowSeconds = 300,
    pollIntervalMs = 10_000,
    onFreshnessChange,
    debug = false,
  } = options;

  const [snapshot, setSnapshot] = useState<SnapshotEnvelope<T> | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<Error | null>(null);
  const [freshness, setFreshness] = useState<FreshnessState>({
    isFresh: true,
    ageSeconds: 0,
    stateHash: '',
    lastChecked: Date.now(),
    staleSince: null,
  });

  const pollTimerRef = useRef<ReturnType<typeof setInterval> | null>(null);
  const mountedRef = useRef(true);
  const lastKnownFreshRef = useRef(true);

  const log = useCallback((msg: string) => {
    if (debug) console.log(`[useFreshness] ${msg}`);
  }, [debug]);

  // Compute freshness from snapshot meta
  const computeFreshness = useCallback((meta: SnapshotMeta): FreshnessState => {
    const generatedAt = new Date(meta.generated_at).getTime();
    const now = Date.now();
    const ageSeconds = Math.floor((now - generatedAt) / 1000);
    const isFresh = ageSeconds <= syncWindowSeconds;

    return {
      isFresh,
      ageSeconds,
      stateHash: meta.state_hash,
      lastChecked: now,
      staleSince: isFresh ? null : (freshness.staleSince ?? now),
    };
  }, [syncWindowSeconds, freshness.staleSince]);

  // Fetch and update snapshot
  const refresh = useCallback(async () => {
    try {
      log('Fetching snapshot...');
      const data = await fetchSnapshot();
      if (!mountedRef.current) return;

      if (data) {
        setSnapshot(data);
        const newFreshness = computeFreshness(data.meta);
        setFreshness(newFreshness);

        // Notify on freshness transition
        if (newFreshness.isFresh !== lastKnownFreshRef.current) {
          lastKnownFreshRef.current = newFreshness.isFresh;
          onFreshnessChange?.(newFreshness.isFresh);
          log(`Freshness changed: ${newFreshness.isFresh ? 'FRESH' : 'STALE'} (age=${newFreshness.ageSeconds}s)`);
        }
      }
      setError(null);
    } catch (err) {
      if (!mountedRef.current) return;
      setError(err instanceof Error ? err : new Error(String(err)));
      log(`Fetch error: ${err}`);
    } finally {
      if (mountedRef.current) {
        setLoading(false);
      }
    }
  }, [fetchSnapshot, computeFreshness, onFreshnessChange, log]);

  // Initial fetch
  useEffect(() => {
    mountedRef.current = true;
    refresh();

    // Poll for freshness re-evaluation (even without new data)
    pollTimerRef.current = setInterval(() => {
      if (snapshot) {
        const newFreshness = computeFreshness(snapshot.meta);
        setFreshness(newFreshness);
        if (newFreshness.isFresh !== lastKnownFreshRef.current) {
          lastKnownFreshRef.current = newFreshness.isFresh;
          onFreshnessChange?.(newFreshness.isFresh);
        }
      }
    }, pollIntervalMs);

    return () => {
      mountedRef.current = false;
      if (pollTimerRef.current) {
        clearInterval(pollTimerRef.current);
      }
    };
  }, []); // eslint-disable-line react-hooks/exhaustive-deps

  // Re-fetch when snapshot changes externally (e.g., manual refresh button)
  const refetch = useCallback(() => {
    setLoading(true);
    refresh();
  }, [refresh]);

  return {
    snapshot,
    loading,
    error,
    freshness,
    refetch,
    // Convenience getters
    isStale: !freshness.isFresh,
    ageDisplay: freshness.ageSeconds < 60
      ? `${freshness.ageSeconds}s ago`
      : freshness.ageSeconds < 3600
        ? `${Math.floor(freshness.ageSeconds / 60)}m ago`
        : `${Math.floor(freshness.ageSeconds / 3600)}h ago`,
  };
}

/**
 * Lightweight freshness check for metadata-only polling.
 * Use when you only need to know if data is stale without fetching full payload.
 */
export function useFreshnessPoll(
  fetchMetadata: () => Promise<{ generated_at: string; state_hash: string } | null>,
  options: UseFreshnessOptions = {}
) {
  const { syncWindowSeconds = 300, pollIntervalMs = 10_000, onFreshnessChange, debug = false } = options;

  const [freshness, setFreshness] = useState<FreshnessState>({
    isFresh: true,
    ageSeconds: 0,
    stateHash: '',
    lastChecked: Date.now(),
    staleSince: null,
  });

  const log = useCallback((msg: string) => {
    if (debug) console.log(`[useFreshnessPoll] ${msg}`);
  }, [debug]);

  useEffect(() => {
    let mounted = true;
    let timer: ReturnType<typeof setInterval>;

    const check = async () => {
      const meta = await fetchMetadata();
      if (!mounted || !meta) return;

      const generatedAt = new Date(meta.generated_at).getTime();
      const now = Date.now();
      const ageSeconds = Math.floor((now - generatedAt) / 1000);
      const isFresh = ageSeconds <= syncWindowSeconds;

      const newFreshness: FreshnessState = {
        isFresh,
        ageSeconds,
        stateHash: meta.state_hash,
        lastChecked: now,
        staleSince: isFresh ? null : (freshness.staleSince ?? now),
      };

      setFreshness(newFreshness);
      if (newFreshness.isFresh !== freshness.isFresh) {
        onFreshnessChange?.(newFreshness.isFresh);
      }
    };

    check();
    timer = setInterval(check, pollIntervalMs);

    return () => {
      mounted = false;
      clearInterval(timer);
    };
  }, [fetchMetadata, syncWindowSeconds, pollIntervalMs, onFreshnessChange, freshness.isFresh, freshness.staleSince, log]);

  return freshness;
}

/**
 * Stale Data Banner Component — Non-obtrusive, calm indicator.
 *
 * Design rules (from spec):
 * - No blinking, no countdowns, no push alerts
 * - Fixed position, low visual priority
 * - Dismissible but auto-reappears on next stale check
 * - Shows age in human-readable format
 */
export interface StaleBannerProps {
  freshness: FreshnessState;
  onDismiss?: () => void;
  dismissible?: boolean;
}

export function StaleBanner({ freshness, onDismiss, dismissible = true }: StaleBannerProps) {
  if (freshness.isFresh) return null;

  const age = freshness.ageSeconds;
  const ageStr = age < 60
    ? `${age}s`
    : age < 3600
      ? `${Math.floor(age / 60)}m`
      : `${Math.floor(age / 3600)}h`;

  return (
    <div
      style={{
        position: 'fixed',
        bottom: 16,
        right: 16,
        zIndex: 1000,
        background: 'rgba(30, 30, 30, 0.95)',
        border: '1px solid #444',
        borderRadius: 8,
        padding: '8px 12px',
        fontSize: '0.8rem',
        color: '#aaa',
        fontFamily: 'system-ui, monospace',
        display: 'flex',
        alignItems: 'center',
        gap: 8,
        boxShadow: '0 4px 20px rgba(0,0,0,0.3)',
        backdropFilter: 'blur(8px)',
      }}
      role="status"
      aria-live="polite"
    >
      <span style={{ color: '#f39c12', fontWeight: 500 }}>●</span>
      <span>Data is {ageStr} old (sync window: {syncWindowSeconds}s)</span>
      {dismissible && (
        <button
          onClick={onDismiss}
          style={{
            background: 'none',
            border: 'none',
            color: '#666',
            cursor: 'pointer',
            fontSize: '1rem',
            lineHeight: 1,
            padding: 0,
          }}
          aria-label="Dismiss stale indicator"
        >
          ×
        </button>
      )}
    </div>
  );
}

// Need to import syncWindowSeconds for the component
const syncWindowSeconds = 300;