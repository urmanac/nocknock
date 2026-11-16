/**
 * Lite cockpit status line (#89). Pure. The phone install opens the Lite view, and a
 * notification's link lands there, so that view has to say two things the full state
 * room already says: what the directive is, and whether the numbers are live.
 *
 * A snapshot that has stopped refreshing is labelled NOT LIVE with its age. It is never
 * hidden and never shown as current. A snapshot is also NOT LIVE as soon as the latest
 * poll for a newer one failed: a phone that left the LAN with the app open must not go
 * on reading LIVE, or SYNC IN PROGRESS, about a worker it cannot reach.
 */
import type { SyncStatus } from './hooks/useFreshness';

export type LiteTone = 'live' | 'syncing' | 'stale' | 'unavailable' | 'loading';

export const LITE_TONE_COLOR: Record<LiteTone, string> = {
  live: '#2ecc71',
  syncing: '#38bdf8',
  stale: '#f39c12',
  unavailable: '#ef4444',
  loading: '#8b949e',
};

export interface LiteStatusInput {
  hasSnapshot: boolean;
  loading: boolean;
  /** Set only when there is no snapshot to show at all (cold start). */
  error: string | null;
  /**
   * Why the most recent poll failed, in words; null or absent once a poll succeeds.
   * Unlike `error` it is set while an earlier snapshot is still on the page.
   */
  pollError?: string | null;
  syncStatus: SyncStatus;
  ageSeconds: number;
  action?: string | null;
  details?: string | null;
}

export interface LiteStatusView {
  tone: LiteTone;
  /** "LIVE · read 42s ago" or "NOT LIVE · last read 12m ago …". */
  freshness: string;
  /** The directive, or a plain statement that there is none to show. */
  action: string;
  details: string | null;
}

/** "42s", "5m", "2h": the same steps the full state room uses. */
export function ageText(ageSeconds: number): string {
  const s = Math.max(0, Math.floor(ageSeconds));
  if (s < 60) return `${s}s`;
  if (s < 3600) return `${Math.floor(s / 60)}m`;
  return `${Math.floor(s / 3600)}h`;
}

export function liteStatusView(input: LiteStatusInput): LiteStatusView {
  if (!input.hasSnapshot) {
    if (input.error) {
      return { tone: 'unavailable', freshness: input.error, action: 'Directive unavailable', details: null };
    }
    return {
      tone: 'loading',
      freshness: input.loading ? 'Reading the worker…' : 'No snapshot yet.',
      action: '…',
      details: null,
    };
  }
  const age = ageText(input.ageSeconds);
  const action = input.action && input.action.trim() !== '' ? input.action : 'Directive unavailable';
  const details = input.details && input.details.trim() !== '' ? input.details : null;
  if (input.pollError && input.pollError.trim() !== '') {
    return {
      tone: 'stale',
      freshness: `NOT LIVE · ${input.pollError.trim()} · last read ${age} ago. Everything on this page is from that read.`,
      action,
      details,
    };
  }
  if (input.syncStatus === 'stale') {
    return {
      tone: 'stale',
      freshness: `NOT LIVE · last read ${age} ago. Everything on this page is from that read.`,
      action,
      details,
    };
  }
  if (input.syncStatus === 'syncing') {
    return { tone: 'syncing', freshness: `SYNC IN PROGRESS · read ${age} ago`, action, details };
  }
  return { tone: 'live', freshness: `LIVE ON-CHAIN · read ${age} ago`, action, details };
}

/**
 * Age and sync status of a snapshot at `nowMs`, by the rule useFreshness applies on its
 * 10 s tick (fresh up to the sync window, syncing through the grace allowance, stale
 * after). The card calls this on every render, so a phone that resumes a suspended app
 * shows the real age on the first paint instead of the line from before it slept.
 * Null when `generatedAt` is not a date: the caller keeps what the hook last computed.
 */
export function freshnessAt(
  generatedAt: string | undefined,
  nowMs: number,
  syncWindowSeconds: number,
  syncGraceSeconds: number,
): { syncStatus: SyncStatus; ageSeconds: number } | null {
  const t = generatedAt ? Date.parse(generatedAt) : NaN;
  if (!Number.isFinite(t)) return null;
  const ageSeconds = Math.max(0, Math.floor((nowMs - t) / 1000));
  const syncStatus: SyncStatus =
    ageSeconds > syncWindowSeconds + syncGraceSeconds ? 'stale' : ageSeconds > syncWindowSeconds ? 'syncing' : 'fresh';
  return { syncStatus, ageSeconds };
}
