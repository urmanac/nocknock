import { describe, expect, it } from 'vitest';
import { bracketPosition, hasNockLeg, nockBracketRows, nonNockRangeCount, positionBracket } from './positions.ts';

// Tracked positions on 2026-11-17: two NOCK ranges and the operator's two gas-side
// WETH/USDC ranges (#151, #152), in config order.
const tok = (symbol: string) => ({ symbol });
const NOCK_USDC = { position_id: 'uniswap-v4:base:3163450', protocol: 'uniswap-v4', fee_tier_bps: 300, token0: tok('USDC'), token1: tok('NOCK'), is_price_source: true };
const WETH_NOCK = { position_id: 'uniswap-v3:base:6336662', protocol: 'uniswap-v3', fee_tier_bps: 100, token0: tok('WETH'), token1: tok('NOCK') };
const GAS_V3 = { position_id: 'uniswap-v3:base:6607225', protocol: 'uniswap-v3', fee_tier_bps: 5, token0: tok('WETH'), token1: tok('USDC') };
const GAS_CL = { position_id: 'aerodrome-cl:base:3449558', protocol: 'aerodrome-cl', fee_tier_bps: 0, tick_spacing: 100, token0: tok('WETH'), token1: tok('USDC') };
const POSITIONS = [NOCK_USDC, WETH_NOCK, GAS_V3, GAS_CL];

// The worker's brackets: token0 per token1. The WETH/USDC ones are WETH per USDC.
const B_NOCK_USDC = { lower_price: '0.00866', upper_price: '0.0646', liquidity_usd: '208.00', in_range: true, distance_bps: 1200, full_range: false };
const B_WETH_NOCK = { lower_price: '0.00000565', upper_price: '0.00000874', liquidity_usd: '12.27', in_range: true, distance_bps: null, full_range: false };
const B_GAS_V3 = { lower_price: '0.000338833', upper_price: '0.000675513', liquidity_usd: '257.19', in_range: true, distance_bps: null, full_range: false };
const B_GAS_CL = { lower_price: '0.000337818', upper_price: '0.000672141', liquidity_usd: '261.94', in_range: true, distance_bps: null, full_range: false };
const withId = (b: object, p: { position_id: string }) => ({ ...b, position_id: p.position_id });
const BRACKETS_282 = [withId(B_NOCK_USDC, NOCK_USDC), withId(B_WETH_NOCK, WETH_NOCK), withId(B_GAS_V3, GAS_V3), withId(B_GAS_CL, GAS_CL)];
const ETH_USD = 1800;
const SPOT = 0.0186;

describe('hasNockLeg (#151)', () => {
  it('is the worker rule: token0 or token1 is NOCK', () => {
    expect(hasNockLeg(NOCK_USDC)).toBe(true);
    expect(hasNockLeg(WETH_NOCK)).toBe(true);
    expect(hasNockLeg({ token0: tok('NOCK'), token1: tok('USDC') })).toBe(true);
    expect(hasNockLeg(GAS_V3)).toBe(false);
    expect(hasNockLeg(GAS_CL)).toBe(false);
    expect(hasNockLeg(null)).toBe(false);
    expect(hasNockLeg({})).toBe(false);
  });

  it('counts the ranges with no NOCK leg', () => {
    expect(nonNockRangeCount(POSITIONS)).toBe(2);
    expect(nonNockRangeCount([NOCK_USDC, WETH_NOCK])).toBe(0);
    expect(nonNockRangeCount(undefined)).toBe(0);
  });
});

describe('nockBracketRows (#152)', () => {
  it('leaves out both WETH/USDC brackets and converts only ETH-per-NOCK', () => {
    const rows = nockBracketRows(BRACKETS_282, POSITIONS, ETH_USD, SPOT);
    expect(rows.map((r) => r.name)).toEqual(['USDC/NOCK v4 · 3%', 'WETH/NOCK v3 · 1%']);
    expect(rows[0].converted).toBe(false);
    expect(rows[0].lower).toBeCloseTo(0.00866, 8);
    expect(rows[0].distance).toBe(12);
    expect(rows[1].converted).toBe(true);
    expect(rows[1].lower).toBeCloseTo(0.00000565 * ETH_USD, 10);
    // No row lands near $0.61..$1.22 "per NOCK".
    expect(rows.every((r) => r.upper < 0.2)).toBe(true);
  });

  it('pairs by position_id when the arrays are not in the same order', () => {
    const reordered = [BRACKETS_282[3], BRACKETS_282[1], BRACKETS_282[2], BRACKETS_282[0]];
    const rows = nockBracketRows(reordered, POSITIONS, ETH_USD, SPOT);
    expect(rows.map((r) => r.name)).toEqual(['WETH/NOCK v3 · 1%', 'USDC/NOCK v4 · 3%']);
    expect(rows[0].converted).toBe(true);
    expect(rows[1].converted).toBe(false);
    expect(rows[1].liquidity).toBe(208);
  });

  it('falls back to the index for a pre-2.8.2 snapshot without position_id', () => {
    const rows = nockBracketRows([B_NOCK_USDC, B_WETH_NOCK, B_GAS_V3, B_GAS_CL], POSITIONS, ETH_USD, SPOT);
    expect(rows.map((r) => r.name)).toEqual(['USDC/NOCK v4 · 3%', 'WETH/NOCK v3 · 1%']);
    expect(rows.map((r) => r.liquidity)).toEqual([208, 12.27]);
  });

  it('keeps native units without an ETH/USD and drops full-range rows', () => {
    const rows = nockBracketRows(BRACKETS_282, POSITIONS, null, SPOT);
    expect(rows[1].converted).toBe(false);
    expect(rows[1].lower).toBeCloseTo(0.00000565, 12);
    const full = nockBracketRows([{ ...BRACKETS_282[0], full_range: true }], POSITIONS, ETH_USD, SPOT);
    expect(full).toEqual([]);
  });
});

describe('bracket lookup by position (#152)', () => {
  it('finds the price-source bracket by id, else by index', () => {
    const reordered = [BRACKETS_282[2], BRACKETS_282[0]];
    expect(positionBracket(NOCK_USDC, POSITIONS, reordered)).toBe(BRACKETS_282[0]);
    // An id-carrying snapshot without this position's bracket has none, never another's.
    expect(positionBracket(WETH_NOCK, POSITIONS, reordered)).toBeUndefined();
    const old = [B_NOCK_USDC, B_WETH_NOCK, B_GAS_V3, B_GAS_CL];
    expect(positionBracket(WETH_NOCK, POSITIONS, old)).toBe(B_WETH_NOCK);
    expect(positionBracket(null, POSITIONS, old)).toBeUndefined();
  });

  it('bracketPosition pairs by id, else by index', () => {
    expect(bracketPosition(BRACKETS_282[3], 0, POSITIONS)).toBe(GAS_CL);
    expect(bracketPosition(B_GAS_V3, 2, POSITIONS)).toBe(GAS_V3);
    expect(bracketPosition({ position_id: 'unknown' }, 0, POSITIONS)).toBeUndefined();
  });
});
