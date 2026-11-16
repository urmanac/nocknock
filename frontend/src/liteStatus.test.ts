import { describe, it, expect } from 'vitest';
import { ageText, freshnessAt, liteStatusView, type LiteStatusInput } from './liteStatus';

const base: LiteStatusInput = {
  hasSnapshot: true,
  loading: false,
  error: null,
  syncStatus: 'fresh',
  ageSeconds: 42,
  action: 'DCA TRANCHE DUE (2 of 5)',
  details: 'Buy $20.00 of NOCK on Aero (spot $0.01153).',
};

describe('lite status', () => {
  it('shows the directive and that it is live', () => {
    expect(liteStatusView(base)).toEqual({
      tone: 'live',
      freshness: 'LIVE ON-CHAIN · read 42s ago',
      action: 'DCA TRANCHE DUE (2 of 5)',
      details: 'Buy $20.00 of NOCK on Aero (spot $0.01153).',
    });
  });

  it('labels an old snapshot NOT LIVE, with its age, and still shows it', () => {
    const v = liteStatusView({ ...base, syncStatus: 'stale', ageSeconds: 12 * 60 + 5 });
    expect(v.tone).toBe('stale');
    expect(v.freshness).toBe('NOT LIVE · last read 12m ago. Everything on this page is from that read.');
    expect(v.action).toBe('DCA TRANCHE DUE (2 of 5)');
    expect(v.freshness).not.toContain('LIVE ON-CHAIN');
  });

  it('says a cycle is in progress inside the grace window', () => {
    const v = liteStatusView({ ...base, syncStatus: 'syncing', ageSeconds: 330 });
    expect(v).toMatchObject({ tone: 'syncing', freshness: 'SYNC IN PROGRESS · read 5m ago' });
  });

  it('says NOT LIVE as soon as a poll fails, even while the last read is recent', () => {
    // The phone left the LAN with the app open: the snapshot is 50 s old and every poll
    // since has failed. It must not read LIVE ON-CHAIN.
    const v = liteStatusView({ ...base, ageSeconds: 50, pollError: 'Worker unreachable: Failed to fetch' });
    expect(v).toEqual({
      tone: 'stale',
      freshness:
        'NOT LIVE · Worker unreachable: Failed to fetch · last read 50s ago. Everything on this page is from that read.',
      action: 'DCA TRANCHE DUE (2 of 5)',
      details: 'Buy $20.00 of NOCK on Aero (spot $0.01153).',
    });
    // Nor SYNC IN PROGRESS: with the worker unreachable there is no evidence of a sync.
    const late = liteStatusView({ ...base, syncStatus: 'syncing', ageSeconds: 330, pollError: 'Worker unreachable: Failed to fetch' });
    expect(late.tone).toBe('stale');
    expect(late.freshness).not.toContain('SYNC IN PROGRESS');
    expect(late.freshness).toContain('NOT LIVE · Worker unreachable: Failed to fetch · last read 5m ago');
    // Past the stale threshold the cause is still named.
    const old = liteStatusView({ ...base, syncStatus: 'stale', ageSeconds: 3700, pollError: 'Worker unreachable: Failed to fetch' });
    expect(old.freshness).toContain('NOT LIVE · Worker unreachable: Failed to fetch · last read 1h ago');
    // A poll that succeeds clears it.
    expect(liteStatusView({ ...base, pollError: null }).tone).toBe('live');
    expect(liteStatusView({ ...base, pollError: '  ' }).tone).toBe('live');
  });

  it('computes age and status at render time, by the same thresholds as the hook', () => {
    const at = (s: number) => Date.parse('2026-11-16T12:00:00Z') + s * 1000;
    const gen = '2026-11-16T12:00:00Z';
    expect(freshnessAt(gen, at(42), 300, 90)).toEqual({ syncStatus: 'fresh', ageSeconds: 42 });
    expect(freshnessAt(gen, at(300), 300, 90)).toEqual({ syncStatus: 'fresh', ageSeconds: 300 });
    expect(freshnessAt(gen, at(301), 300, 90)).toEqual({ syncStatus: 'syncing', ageSeconds: 301 });
    expect(freshnessAt(gen, at(390), 300, 90)).toEqual({ syncStatus: 'syncing', ageSeconds: 390 });
    expect(freshnessAt(gen, at(391), 300, 90)).toEqual({ syncStatus: 'stale', ageSeconds: 391 });
    // A phone resumed after an hour asleep: stale at once, not the line from before.
    expect(freshnessAt(gen, at(3600), 300, 90)).toEqual({ syncStatus: 'stale', ageSeconds: 3600 });
    // A device clock behind the worker's never gives a negative age.
    expect(freshnessAt(gen, at(-30), 300, 90)).toEqual({ syncStatus: 'fresh', ageSeconds: 0 });
    // No date to compute from: the caller keeps the hook's value.
    expect(freshnessAt(undefined, at(0), 300, 90)).toBeNull();
    expect(freshnessAt('not a date', at(0), 300, 90)).toBeNull();
  });

  it('never invents a directive', () => {
    expect(liteStatusView({ ...base, action: undefined, details: undefined })).toMatchObject({
      action: 'Directive unavailable',
      details: null,
    });
    expect(liteStatusView({ ...base, action: '  ', details: '' })).toMatchObject({ action: 'Directive unavailable', details: null });
  });

  it('shows the cause when there is no snapshot at all', () => {
    const v = liteStatusView({ ...base, hasSnapshot: false, error: 'Worker unreachable: Failed to fetch', action: null, details: null });
    expect(v).toEqual({
      tone: 'unavailable',
      freshness: 'Worker unreachable: Failed to fetch',
      action: 'Directive unavailable',
      details: null,
    });
    expect(liteStatusView({ ...base, hasSnapshot: false, loading: true })).toMatchObject({ tone: 'loading', freshness: 'Reading the worker…' });
    expect(liteStatusView({ ...base, hasSnapshot: false, loading: false })).toMatchObject({ tone: 'loading', freshness: 'No snapshot yet.' });
  });

  it('formats ages like the full state room', () => {
    expect(ageText(0)).toBe('0s');
    expect(ageText(59)).toBe('59s');
    expect(ageText(60)).toBe('1m');
    expect(ageText(3599)).toBe('59m');
    expect(ageText(7300)).toBe('2h');
    expect(ageText(-5)).toBe('0s');
  });
});
