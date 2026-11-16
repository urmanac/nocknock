import { describe, it, expect } from 'vitest';
import { ageText, liteStatusView, type LiteStatusInput } from './liteStatus';

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
