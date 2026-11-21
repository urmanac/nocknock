import { describe, expect, it } from 'vitest';
import { existsSync, readFileSync } from 'node:fs';
import {
  anchorId,
  eventsOf,
  maintenanceFor,
  maintenanceLine,
  maintenanceOf,
  sinceText,
  staleNote,
  tierTone,
  topOfPage,
  type SnapshotEvent,
} from './events';
import { EVENTS_DEEP_CONVERTED, EVENTS_LIVE_UNSET, EVENTS_NEAR_MOVE, EVENTS_PREDATES } from './events.fixture';

const NEAR = 'uniswap-v4:base:3244700';
const DEEP = 'uniswap-v4:base:3244701';
const CAP = 'uniswap-v4:base:3244699';

describe('calls to action as events (#169)', () => {
  it('the fixture is the worker golden, byte for byte', () => {
    const golden = new URL('../../ingestion-worker/testdata/ladder_maintenance/golden_events_text.json', import.meta.url);
    if (!existsSync(golden)) return;
    const doc = JSON.parse(readFileSync(golden, 'utf8'));
    expect(EVENTS_LIVE_UNSET).toEqual(doc.live_unset.data);
    expect(EVENTS_NEAR_MOVE).toEqual(doc.near_move.data);
    expect(EVENTS_DEEP_CONVERTED).toEqual(doc.deep_converted_stale.data);
    expect(EVENTS_PREDATES).toEqual(doc.predates.data);
  });

  it('the live bottle with the settings unset puts nothing at the top', () => {
    const events = eventsOf(EVENTS_LIVE_UNSET);
    expect(events).not.toBeNull();
    expect(topOfPage(events)).toEqual([]);
    // The quiet ones are all there: the directive, the cap, the fees, the rewards, the floor price.
    const quiet = events!.filter((e) => e.tier === 'quiet').map((e) => e.kind);
    expect(quiet).toEqual(['directive', 'ladder_cap_sold', 'harvest', 'aero_rewards', 'floor_crossing']);
  });

  it('a near-rung move is at the top, with what would be signed and its deep link', () => {
    const top = topOfPage(eventsOf(EVENTS_NEAR_MOVE));
    expect(top.map((e) => e.id)).toEqual([`ladder_near_move:${NEAR}`]);
    const e = top[0];
    expect(e.title).toBe('LADDER: MOVE THE NEAR RUNG');
    expect(anchorId(e)).toBe('event-ladder-near-move-uniswap-v4-base-3244700');
    expect(e.numbers.width_ticks).toBe('1800');
    expect(e.would_sign).toMatch(/^Two transactions in your wallet/);
    expect(tierTone(e)).toBe('maintenance');
    expect(sinceText(e.since)).toBe('since 2026-11-22 17:30 UTC');
  });

  it('the strip leaves cards that render themselves at the top', () => {
    const own: SnapshotEvent = { ...topOfPage(eventsOf(EVENTS_NEAR_MOVE))[0], id: 'ladder_suggestion:NOCK', kind: 'ladder_suggestion', card: 'ladder_suggestion' };
    const found: SnapshotEvent = { ...own, id: 'found_positions', kind: 'found_positions', card: 'found_positions' };
    expect(topOfPage([own, found])).toEqual([]);
  });

  it('a snapshot from before 2.11.0 is not reported, never "nothing needs you"', () => {
    expect(eventsOf(EVENTS_PREDATES)).toBeNull();
    expect(topOfPage(eventsOf(EVENTS_PREDATES))).toEqual([]);
    expect(maintenanceOf(EVENTS_PREDATES)).toBeNull();
  });

  it('the stale note is shown only when the sync is delayed', () => {
    expect(staleNote('fresh')).toBeNull();
    expect(staleNote('syncing')).toBeNull();
    expect(staleNote('stale')).toMatch(/may be out of date/);
  });

  it('breaches are red, maintenance amber', () => {
    const base = topOfPage(eventsOf(EVENTS_NEAR_MOVE))[0];
    expect(tierTone({ ...base, kind: 'reserve_below' })).toBe('alarm');
    expect(tierTone({ ...base, kind: 'directive', title: 'DEFENSIVE LOCK (Floor Recovery)' })).toBe('alarm');
    expect(tierTone({ ...base, kind: 'directive', title: 'DCA TRANCHE DUE (2/5)' })).toBe('maintenance');
  });

  it('each rung has its quiet maintenance line', () => {
    const m = maintenanceOf(EVENTS_LIVE_UNSET);
    expect(m!.rules.near_move.status).toBe('off');
    expect(Object.values(m!.settings).every((v) => v === null)).toBe(true);
    expect(maintenanceLine(maintenanceFor(m, NEAR)!)).toMatch(/^Maintenance \(rule off\): Near rung #3244700 has been out of range above/);
    expect(maintenanceLine(maintenanceFor(m, CAP)!)).toMatch(/^Maintenance \(sold its NOCK\): .*does not suggest putting NOCK back/);
    expect(maintenanceLine(maintenanceFor(m, DEEP)!)).toMatch(/^Maintenance \(waiting for a dip\)/);
    const conv = maintenanceOf(EVENTS_DEEP_CONVERTED);
    expect(maintenanceLine(maintenanceFor(conv, DEEP)!)).toContain('NockNock does not suggest a lower ladder');
  });

  it('no event text steers', () => {
    for (const fx of [EVENTS_LIVE_UNSET, EVENTS_NEAR_MOVE, EVENTS_DEEP_CONVERTED]) {
      for (const e of eventsOf(fx)!) {
        if (e.kind === 'directive') continue;
        expect(e.text ?? '').not.toMatch(/\bshould\b|recommend|\bmust\b/i);
      }
    }
  });
});
