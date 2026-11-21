/**
 * Calls to action as structured events (2.11.0, issue #169): `derived_metrics.events[]`,
 * and the ladder maintenance block `derived_metrics.ladder_maintenance`.
 *
 * The worker gives every call to action a tier. `top` entries (maintenance the operator
 * signed up for, floor and reserve breaches) are shown at the top of the dashboard and the
 * Lite cockpit; `quiet` entries stay where they are shown today. An entry with a `card`
 * that renders it in full at the top already (the ladder suggestion, positions found
 * on-chain) is left to that card. The worker writes every figure and sentence; these
 * helpers only pick and word the frame. No input anywhere.
 */
import type { SyncStatus } from './hooks/useFreshness';

export interface EventLink {
  label: string;
  url: string;
}

export interface SnapshotEvent {
  id: string;
  kind: string;
  tier: 'top' | 'quiet';
  title: string;
  what_changed: string | null;
  text: string | null;
  numbers: Record<string, string | null>;
  since: string;
  deep_link: string;
  links: EventLink[];
  would_sign: string | null;
  card: string | null;
  source?: string;
}

export interface MaintenanceGate {
  gate: string;
  setting: string;
  measured: string;
  passes: boolean | null;
  text: string;
}

export interface MaintenanceRung {
  position_id: string;
  label: string;
  short_label: string;
  rung: 'cap' | 'near' | 'deep' | null;
  state: string;
  status: string;
  rule: string | null;
  text: string;
  gates: MaintenanceGate[];
  event: Partial<SnapshotEvent> | null;
}

export interface MaintenanceRule {
  status: 'on' | 'off' | 'invalid';
  needs: string[];
  invalid: string[];
  text: string;
}

export interface LadderMaintenance {
  settings_key: string;
  settings: Record<string, string | null>;
  rules: Record<string, MaintenanceRule>;
  history: string;
  watched_since_previous: boolean;
  rungs: MaintenanceRung[];
  statement: string;
}

/** Cards that render their own top-of-page entry in full; the strip leaves them alone. */
export const OWN_CARD_KINDS = ['ladder_suggestion', 'found_positions'];

export function eventsOf(data: { derived_metrics?: { events?: SnapshotEvent[] | null } | null } | null | undefined): SnapshotEvent[] | null {
  const e = data?.derived_metrics?.events;
  return Array.isArray(e) ? e : null;
}

/** Whether the top-of-page strip renders this entry (and so holds its anchor). */
export function stripRenders(e: Pick<SnapshotEvent, 'tier' | 'kind'>): boolean {
  return e.tier === 'top' && !OWN_CARD_KINDS.includes(e.kind);
}

/** The entries for the top-of-page strip, in the worker's order. */
export function topOfPage(events: SnapshotEvent[] | null): SnapshotEvent[] {
  return (events ?? []).filter(stripRenders);
}

/** Every `card` the worker names, and where the dashboard shows it (#169 review). */
export const EVENT_CARDS = [
  'directive',
  'reserve',
  'collectible',
  'closed_positions',
  'floor_health',
  'ladders',
  'ladder_suggestion',
  'found_positions',
  'accumulation_plan',
] as const;

/** The element id for an event's deep link ('#event-…' → 'event-…'). */
export function anchorId(e: Pick<SnapshotEvent, 'deep_link'>): string {
  return e.deep_link.replace(/^#/, '');
}

/** A note for entries read from a snapshot that is not fresh; null when it is. */
export function staleNote(syncStatus: SyncStatus | undefined): string | null {
  if (syncStatus === 'fresh' || syncStatus === 'syncing') return null;
  return 'As of the last snapshot: the sync is delayed, so this may be out of date.';
}

/** "since 2026-11-22 17:30 UTC" from an RFC 3339 time. */
export function sinceText(since: string | null | undefined): string {
  if (!since) return '';
  const d = new Date(since);
  if (Number.isNaN(d.getTime())) return '';
  const iso = d.toISOString();
  return `since ${iso.slice(0, 10)} ${iso.slice(11, 16)} UTC`;
}

/** Red for a breach (the reserve, a floor DEFENSIVE LOCK), amber for maintenance. */
export function tierTone(e: Pick<SnapshotEvent, 'kind' | 'title'>): 'alarm' | 'maintenance' {
  if (e.kind === 'reserve_below') return 'alarm';
  if (e.kind === 'directive' && e.title.startsWith('DEFENSIVE LOCK')) return 'alarm';
  return 'maintenance';
}

export function maintenanceOf(data: { derived_metrics?: { ladder_maintenance?: LadderMaintenance | null } | null } | null | undefined): LadderMaintenance | null {
  const m = data?.derived_metrics?.ladder_maintenance;
  return m && Array.isArray(m.rungs) ? m : null;
}

export function maintenanceFor(m: LadderMaintenance | null, positionId: string | undefined): MaintenanceRung | undefined {
  return m?.rungs.find((r) => r.position_id === positionId);
}

const STATUS_WORDS: Record<string, string> = {
  suggested: 'move or pull stated',
  waiting: 'waiting on your gates',
  off: 'rule off',
  invalid: 'setting invalid',
  unknown: 'not measurable now',
  trading: 'trading',
  holds_nock: 'holds NOCK',
  waiting_for_dip: 'waiting for a dip',
  holding: 'buying',
  fully_converted: 'fully converted',
  sold: 'sold its NOCK',
  in_range: 'in range',
  not_applicable: 'no rule',
};

/** "Maintenance (rule off): …" for a rung's line on the Ladders card. */
export function maintenanceLine(r: MaintenanceRung): string {
  return `Maintenance (${STATUS_WORDS[r.status] ?? r.status}): ${r.text}`;
}
