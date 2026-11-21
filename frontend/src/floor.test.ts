import { describe, it, expect } from 'vitest';
import { floorBreachLine, floorState, floorDeltaText, floorSummaryText, regimeLabel, regimeTone, type FloorBreach } from './floor';
import { LIVE_FLOOR_BREACH } from './floorBreach.fixture';

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

describe('floor-crossing price line (#165)', () => {
  const live = LIVE_FLOOR_BREACH;
  const with_ = (over: Partial<FloorBreach>): FloorBreach => ({ ...live, ...over });

  it('crosses: the price, the first range to sell out, what remains and the tolerated level', () => {
    expect(floorBreachLine(live)).toBe(
      'Floor crossed if NOCK ≥ $0.025405 (+96.3%); v4 USDC/NOCK 3.00% #3163450 sells out first (at $0.041055)' +
        ' · if every range sells out, 90,201 NOCK (90,087 wallet + 114 uncollected fees) remain · tolerated 90,000: never crossed',
    );
  });

  it('crosses with only a full-range pool: nothing sells out first', () => {
    const tail = with_({ first_to_sell_out: null, largest_seller: 'Aero USDC/NOCK 0.30% (full range)', positions: [live.positions[1]], tolerated: null });
    expect(floorBreachLine(tail)).toBe(
      'Floor crossed if NOCK ≥ $0.025405 (+96.3%); no range sells out (only full-range pools hold NOCK) · if every range sells out, 90,201 NOCK (90,087 wallet + 114 uncollected fees) remain',
    );
  });

  it('never: the NOCK left if every range sells out', () => {
    const never = with_({
      status: 'never',
      crossing: { level_nock: '100000', status: 'never', multiple: null, price_usd: null, pct_above_spot: null },
      wallet_nock: '100500',
      nock_if_all_sold: '100614',
      largest_seller: null,
    });
    expect(floorBreachLine(never)).toBe(
      'No rise crosses the floor: 100,614 NOCK (100,500 wallet + 114 uncollected fees) remain if every range sells out · tolerated 90,000: never crossed',
    );
  });

  it('already below, with the tolerated level measured on its own', () => {
    const below = with_({
      status: 'already_below',
      crossing: { level_nock: '100000', status: 'already_below', multiple: '1.0000', price_usd: '0.012944', pct_above_spot: '0.0' },
      tolerated: { level_nock: '90000', status: 'crosses', multiple: '1.1830', price_usd: '0.015313', pct_above_spot: '18.3' },
    });
    expect(floorBreachLine(below)).toBe("Below the floor at today's price · tolerated 90,000 crossed at $0.015313 (+18.3%)");
  });

  it('unavailable states the reason, never a number', () => {
    const reason = 'v4 USDC/NOCK 3.00% #3163450: pool price could not be read (sqrtPrice is zero)';
    const line = floorBreachLine({ status: 'unavailable', floor_nock: '100000', positions: [], assumption: '', reason });
    expect(line).toBe(`Floor-crossing price unavailable: ${reason}`);
    expect(line).not.toMatch(/\$\d/);
  });

  it('no floor, or a snapshot from before 2.9.0, shows no line', () => {
    expect(floorBreachLine({ status: 'no_floor', floor_nock: '0', positions: [], assumption: '' })).toBeNull();
    expect(floorBreachLine(null)).toBeNull();
    expect(floorBreachLine(undefined)).toBeNull();
  });

  it('is a measurement, never an instruction', () => {
    for (const fb of [live, with_({ status: 'never' })]) {
      const line = floorBreachLine(fb)!.toLowerCase();
      for (const word of ['should', 'withdraw', 'consider', 'recommend', 'move ', 'rebalance']) expect(line).not.toContain(word);
    }
  });
});
