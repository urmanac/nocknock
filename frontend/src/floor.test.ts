import { describe, it, expect } from 'vitest';
import { floorState, floorDeltaText, floorSummaryText, regimeLabel, regimeTone } from './floor';

describe('floor display (v0.5.1)', () => {
  it('separates a configured "no floor" from a missing field', () => {
    expect(floorState('0', '113863', '113863')).toEqual({ kind: 'none' });
    expect(floorState(undefined, undefined)).toEqual({ kind: 'unavailable' });
    expect(floorState('', '')).toEqual({ kind: 'unavailable' });
    expect(floorState('abc', '1')).toEqual({ kind: 'unavailable' });
    expect(floorSummaryText(floorState('0', '0'))).toBe('No floor configured');
    expect(floorSummaryText(floorState(undefined, undefined))).toBe('floor target unavailable');
  });

  it('says below, never "-X above", for a deficit', () => {
    // R5: the user's own 100,000 floor with 0 NOCK.
    const s = floorState('100000', '-100000', '0');
    expect(s).toEqual({ kind: 'set', target: 100000, delta: -100000 });
    expect(floorSummaryText(s)).toBe('100,000 below the 100,000 floor');
    expect(floorSummaryText(s)).not.toContain('above');
    expect(floorDeltaText(13863, 100000)).toBe('+13,863 above the 100,000 floor');
    expect(floorDeltaText(0, 100000)).toBe('+0 above the 100,000 floor');
  });

  it('derives the delta from the total only when the delta field is missing', () => {
    expect(floorState('100000', undefined, '90000')).toEqual({ kind: 'set', target: 100000, delta: -10000 });
    expect(floorState('100000', undefined, undefined)).toEqual({ kind: 'set', target: 100000, delta: null });
    expect(floorSummaryText(floorState('100000', undefined, undefined))).toBe('100,000 NOCK floor; buffer unavailable');
  });

  it('labels an unknown regime as unknown, in a neutral tone', () => {
    expect(regimeLabel('unknown')).toBe('UNKNOWN (no daily RSI yet)');
    expect(regimeTone('unknown')).toBe('neutral');
    expect(regimeLabel('normal')).toBe('NORMAL');
    expect(regimeTone('normal')).toBe('ok');
    expect(regimeLabel('overextended')).toBe('OVEREXTENDED');
    expect(regimeTone('overextended')).toBe('warn');
    expect(regimeLabel(undefined)).toBe('—');
  });
});
