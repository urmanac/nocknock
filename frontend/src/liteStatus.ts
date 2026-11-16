/**
 * Lite cockpit status line (#89). Pure. The phone install opens the Lite view, and a
 * notification's link lands there, so that view has to say two things the full state
 * room already says: what the directive is, and whether the numbers are live.
 *
 * A snapshot that has stopped refreshing is labelled NOT LIVE with its age. It is never
 * hidden and never shown as current.
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
