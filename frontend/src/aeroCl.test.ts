import { describe, it, expect } from 'vitest';
import {
  isAeroProtocol,
  positionFeesText,
  rangeSide,
  rangeSideText,
  rangeSideTone,
  shortAddress,
  stakingText,
} from './aeroCl';
import { positionLabel, positionRangeText, protocolBadge, rangeStatus, velocityRowText } from './positions';

// The reason alone since #93; the position's unclaimed AERO is its own field (aero.test.ts).
const STAKED_NOTE = 'not LP-claimable (staked: swap fees on staked liquidity go to the gauge)';

/** The operator's WETH/NOCK CL200 position as the worker emits it (Base block 58061668). */
const cl = {
  position_id: 'aerodrome-cl:base:2914490',
  protocol: 'aerodrome-cl',
  nft_id: 2914490,
  fee_tier_bps: 97,
  tick_spacing: 200,
  token0: { symbol: 'WETH' },
  token1: { symbol: 'NOCK' },
  amount0: '0.055908',
  amount1: '6945.829656',
  amount_usd: '180.20',
  fees_usd: '0.000000',
  fees_note: STAKED_NOTE,
  staked_in_gauge: '0x3bf70c880adc8370917994ab0df68e763b7a60fd',
  lower_tick: 73400,
  upper_tick: 74800,
  // As the worker publishes them since v0.8.1 (#131): six significant digits.
  lower_price: '0.00000564468',
  upper_price: '0.00000649289',
  in_range: true,
  full_range: false,
  pool_active: true,
};

describe('Aero CL position display (#91)', () => {
  it('is labelled Aero CL, never Uniswap', () => {
    expect(positionLabel(cl)).toBe('WETH/NOCK Aero CL200 · 0.97%');
    expect(positionLabel({ ...cl, tick_spacing: undefined })).toBe('WETH/NOCK Aero CL · 0.97%');
    expect(protocolBadge('aerodrome-cl')).toBe('AERO CL');
    expect(isAeroProtocol('aerodrome-cl')).toBe(true);
    expect(isAeroProtocol('aerodrome')).toBe(true);
    expect(isAeroProtocol('uniswap-v3')).toBe(false);
    expect(isAeroProtocol(undefined)).toBe(false);
    // A ranged position: it prints a real range and can be in or out of it.
    // The ticks give 154,015 – 177,158, which is what the Aero app shows (#131).
    expect(positionRangeText(cl)).toBe('154,015 – 177,158 NOCK per WETH');
    // Three significant digits printed 154,083 – 177,305.
    expect(positionRangeText({ ...cl, lower_price: '0.00000564', upper_price: '0.00000649' })).toBe('154,083 – 177,305 NOCK per WETH');
    expect(rangeStatus(cl)).toEqual({ label: '● IN RANGE', tone: 'good' });
    expect(rangeStatus({ ...cl, in_range: false })).toEqual({ label: '● OUT OF RANGE', tone: 'warn' });
  });

  it('shows why a staked position has no fee figure, never "$0"', () => {
    expect(positionFeesText(cl)).toBe(`Fees: ${STAKED_NOTE}`);
    expect(positionFeesText(cl)).not.toContain('$');
    // Positions whose fees were read keep the figure exactly as before.
    expect(positionFeesText({ fees_usd: '0.012345' })).toBe('Fees: $0.012345');
    expect(positionFeesText({ fees_usd: '0.000000', fees_note: null })).toBe('Fees: $0.000000');
    // A missing figure is unavailable, not a blank dollar sign.
    expect(positionFeesText({})).toBe('Fees: unavailable');
  });

  it('says when the NFT is staked in its gauge', () => {
    expect(stakingText(cl)).toBe('Staked in gauge 0x3bf7…60fd for this wallet');
    expect(stakingText({ ...cl, staked_in_gauge: undefined })).toBeNull();
    expect(stakingText({ staked_in_gauge: '' })).toBeNull();
    expect(shortAddress('0x1234')).toBe('0x1234');
  });

  it('prints the worker reason in the velocity row', () => {
    const row = { name: 'Aero CL WETH/NOCK 0.97% #2914490', claimable: 0, gasBreakEven: 0.0041, ratio: 0, action: 'not_claimable' };
    const text = velocityRowText({ ...row, note: STAKED_NOTE });
    expect(text).toBe(`Aero CL WETH/NOCK 0.97% #2914490: ${STAKED_NOTE}`);
    expect(text).not.toContain('earned');
    expect(text).not.toContain('claimFees()');
    // Without a note the Aero basic pool's wording is unchanged.
    expect(velocityRowText(row)).toContain('not LP-claimable (claimFees() reverts)');
  });
});

describe('range side (#91)', () => {
  it('reads in range from the worker and names both tokens', () => {
    expect(rangeSide(cl, 74002)).toBe('in_range');
    expect(rangeSideText(cl, 74002)).toBe('in range, holding WETH and NOCK');
    expect(rangeSideTone(cl, 74002)).toBe('good');
  });

  it('names the side an out-of-range position left on, from the pool tick', () => {
    // Pool tick under the lower tick: NOCK rose past the range, which sold all its NOCK.
    const above = { ...cl, in_range: false, amount0: '0.107', amount1: '0.000000' };
    expect(rangeSide(above, 73000)).toBe('all_token0');
    expect(rangeSideText(above, 73000)).toBe(
      'out of range above: NOCK is priced over this range, which has sold all its NOCK and holds only WETH',
    );
    // Pool tick at or over the upper tick: NOCK fell past the range, which is all NOCK.
    const below = { ...cl, in_range: false, amount0: '0.000000', amount1: '16800.5' };
    expect(rangeSide(below, 74800)).toBe('all_token1');
    expect(rangeSide(below, 75200)).toBe('all_token1');
    expect(rangeSideText(below, 75200)).toBe(
      'out of range below: NOCK is priced under this range, which now holds only NOCK',
    );
    expect(rangeSideTone(below, 75200)).toBe('warn');
  });

  it('falls back to the amounts when the pool tick is unknown, and never guesses', () => {
    const allNock = { ...cl, in_range: false, amount0: '0.000000', amount1: '16800.5' };
    expect(rangeSide(allNock)).toBe('all_token1');
    expect(rangeSide(allNock, null)).toBe('all_token1');
    const allWeth = { ...cl, in_range: false, amount0: '0.107', amount1: '0.000000' };
    expect(rangeSide(allWeth, undefined)).toBe('all_token0');
    // Missing or unparsable amounts: the side is not known, and nothing is called empty.
    expect(rangeSide({ in_range: false })).toBe('out_unknown');
    expect(rangeSideText({ in_range: false })).toBe('out of range');
    expect(rangeSide({ ...cl, in_range: false, amount0: 'n/a', amount1: '' })).toBe('out_unknown');
  });

  it('says a position that holds nothing is empty, never in range or out of range', () => {
    // The worker's in_range compares ticks only, so an emptied position can still carry
    // in_range: true. It holds neither token: no holdings are claimed for it.
    const emptied = { ...cl, amount0: '0.000000', amount1: '0.000000' };
    expect(rangeSide(emptied, 74002)).toBe('empty');
    expect(rangeSideText(emptied, 74002)).toBe(
      'empty: holds no WETH and no NOCK, so it is neither in nor out of range',
    );
    expect(rangeSideTone(emptied, 74002)).toBe('neutral');
    // Out of range by tick, on either side: still empty, not "holds only NOCK".
    for (const tick of [73000, 75200, undefined]) {
      const out = { ...emptied, in_range: false };
      expect(rangeSide(out, tick)).toBe('empty');
      expect(rangeSideText(out, tick)).not.toContain('holds only');
      expect(rangeSideTone(out, tick)).toBe('neutral');
    }
    // The worker's own rule is liquidity 0; it decides when it is given.
    expect(rangeSide({ ...cl, liquidity_amount: '0' }, 74002)).toBe('empty');
    expect(rangeSide({ ...cl, liquidity_amount: '57859113596792372851' }, 74002)).toBe('in_range');
    // Dust that prints as 0.000000 on both legs but still has liquidity is not empty.
    expect(rangeSide({ ...emptied, liquidity_amount: '12' }, 74002)).toBe('in_range');
    // Every ranged protocol, not only Aero CL; a full-range position has no such state.
    const v3 = { ...emptied, protocol: 'uniswap-v3', token0: { symbol: 'USDC' } };
    expect(rangeSideText(v3, 74002)).toBe('empty: holds no USDC and no NOCK, so it is neither in nor out of range');
    expect(rangeSide({ in_range: true, full_range: true, amount0: '0', amount1: '0' })).toBe('full_range');
    // A stale pool is said first: its tick is frozen whatever the position holds.
    expect(rangeSideText({ ...emptied, pool_active: false }, 74002)).toContain('stale pool');
  });

  it('handles NOCK as token0 and pairs without NOCK', () => {
    const nockFirst = { in_range: false, lower_tick: 100, upper_tick: 200, token0: { symbol: 'NOCK' }, token1: { symbol: 'USDC' } };
    expect(rangeSideText(nockFirst, 50)).toBe(
      'out of range below: NOCK is priced under this range, which now holds only NOCK',
    );
    expect(rangeSideText(nockFirst, 250)).toBe(
      'out of range above: NOCK is priced over this range, which has sold all its NOCK and holds only USDC',
    );
    const other = { in_range: false, lower_tick: 100, upper_tick: 200, token0: { symbol: 'WETH' }, token1: { symbol: 'USDC' } };
    expect(rangeSideText(other, 50)).toBe('out of range: holds only WETH');
  });

  it('claims no side for a full-range position or a stale pool', () => {
    expect(rangeSide({ in_range: true, full_range: true })).toBe('full_range');
    expect(rangeSideText({ in_range: true, full_range: true })).toBe('full range (cannot go out of range)');
    const stale = { ...cl, pool_active: false };
    expect(rangeSideText(stale, 74002)).toContain('stale pool');
    expect(rangeSideTone(stale, 74002)).toBe('stale');
  });
});
