import { describe, expect, it } from 'vitest';
import { bracketPosition, ethBracketRows, fullRangeEthUsdcCount, hasNockLeg, isEthUsdcPair, nockBracketRows, nonNockRangeCount, nonNockRangeKind, notDrawnRangesText, positionBracket, positionLabel } from './positions.ts';

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

describe('naming the ranges with no NOCK leg (#151 review)', () => {
  const USDC_AERO = { position_id: 'uniswap-v3:base:9999001', protocol: 'uniswap-v3', fee_tier_bps: 30, token0: tok('USDC'), token1: tok('AERO') };
  const USDC_ETH = { position_id: 'x', token0: tok('USDC'), token1: tok('ETH') };

  it('says ETH/USDC only when every non-NOCK range is an ETH-or-WETH / USDC pair', () => {
    expect(nonNockRangeKind(POSITIONS)).toBe('ETH/USDC');
    expect(nonNockRangeKind([...POSITIONS, USDC_ETH])).toBe('ETH/USDC');
    expect(nonNockRangeKind([...POSITIONS, USDC_AERO])).toBe('non-NOCK');
    expect(nonNockRangeKind([NOCK_USDC, USDC_AERO])).toBe('non-NOCK');
  });

  it('the not-drawn caption names only ranges on neither chart (#160)', () => {
    // ETH/USDC ranges are drawn on their own USD-per-ETH chart since #160.
    const drawn = [GAS_V3.position_id, GAS_CL.position_id];
    expect(notDrawnRangesText(POSITIONS, drawn)).toBeNull();
    expect(notDrawnRangesText([NOCK_USDC, GAS_V3], [GAS_V3.position_id])).toBeNull();
    expect(notDrawnRangesText([...POSITIONS, USDC_AERO], drawn)).toBe('1 non-NOCK range not drawn: not priced in NOCK or ETH');
    expect(notDrawnRangesText([NOCK_USDC, USDC_AERO, { ...USDC_AERO, position_id: 'y' }], drawn)).toBe('2 non-NOCK ranges not drawn: not priced in NOCK or ETH');
    expect(notDrawnRangesText([NOCK_USDC, WETH_NOCK])).toBeNull();
  });

  it('an ETH/USDC range the ETH chart could not draw is named, never dropped silently', () => {
    // Only the v3 range was drawn (the CL bracket was missing or unusable).
    expect(notDrawnRangesText(POSITIONS, [GAS_V3.position_id])).toBe('1 ETH/USDC range not drawn: no usable bracket in this snapshot');
    expect(notDrawnRangesText(POSITIONS)).toBe('2 ETH/USDC ranges not drawn: no usable bracket in this snapshot');
    // A full-range one is named by the ETH chart instead.
    expect(notDrawnRangesText([NOCK_USDC, { ...GAS_V3, full_range: true }])).toBeNull();
    // Mixed: the sentence that covers both.
    expect(notDrawnRangesText([...POSITIONS, USDC_AERO], [GAS_V3.position_id])).toBe('2 non-NOCK ranges not drawn: not priced in NOCK or ETH');
  });

  it('isEthUsdcPair takes ETH or WETH with USDC in either order', () => {
    expect(isEthUsdcPair(GAS_V3)).toBe(true);
    expect(isEthUsdcPair(USDC_ETH)).toBe(true);
    expect(isEthUsdcPair(WETH_NOCK)).toBe(false);
    expect(isEthUsdcPair(USDC_AERO)).toBe(false);
    expect(isEthUsdcPair(null)).toBe(false);
  });
});

describe('ethBracketRows (#160)', () => {
  // Live on chudmax, v0.8.2, 2026-11-18: ETH $1,749.16.
  const ETH_SPOT = 1749.16;

  it('draws both gas-side WETH/USDC ranges in USD per ETH, edges swapped', () => {
    const rows = ethBracketRows(BRACKETS_282, POSITIONS, ETH_SPOT);
    expect(rows.map((r) => r.positionId)).toEqual([GAS_V3.position_id, GAS_CL.position_id]);
    expect(rows.map((r) => r.name)).toEqual([positionLabel(GAS_V3), positionLabel(GAS_CL)]);
    expect(rows[0].name).toMatch(/WETH\/USDC/);
    // 1 / 0.000675513 and 1 / 0.000338833: the upper WETH-per-USDC edge is the lower USD edge.
    expect(rows[0].lower).toBeCloseTo(1480.33, 1);
    expect(rows[0].upper).toBeCloseTo(2951.31, 1);
    expect(rows[0].lower).toBeLessThan(rows[0].upper);
    expect(rows[0].liquidity).toBe(257.19);
    expect(rows[0].inRange).toBe(true);
    // Spot is 18.2% above the lower edge, 68.7% below the upper: the nearer edge wins.
    expect(rows[0].distance).toBeCloseTo((ETH_SPOT - 1 / 0.000675513) / ETH_SPOT * 100, 6);
    // No NOCK range is drawn here.
    expect(rows.every((r) => r.lower > 100)).toBe(true);
  });

  it('a USDC/ETH-ordered bracket is already USDC per ETH', () => {
    const USDC_WETH = { position_id: 'uniswap-v3:base:7', protocol: 'uniswap-v3', fee_tier_bps: 5, token0: tok('USDC'), token1: tok('WETH') };
    const b = { position_id: USDC_WETH.position_id, lower_price: '1480', upper_price: '2951', liquidity_usd: '10', in_range: true, full_range: false };
    const [row] = ethBracketRows([b], [USDC_WETH], ETH_SPOT);
    expect(row.lower).toBe(1480);
    expect(row.upper).toBe(2951);
  });

  it('pairs by id when reordered and by index for an older snapshot', () => {
    const reordered = [BRACKETS_282[3], BRACKETS_282[0], BRACKETS_282[2]];
    expect(ethBracketRows(reordered, POSITIONS, ETH_SPOT).map((r) => r.positionId)).toEqual([GAS_CL.position_id, GAS_V3.position_id]);
    const old = [B_NOCK_USDC, B_WETH_NOCK, B_GAS_V3, B_GAS_CL];
    expect(ethBracketRows(old, POSITIONS, ETH_SPOT).map((r) => r.liquidity)).toEqual([257.19, 261.94]);
  });

  it('out of range: distance 0 and the flag kept from the worker', () => {
    const [row] = ethBracketRows([{ ...BRACKETS_282[2], in_range: false }], POSITIONS, 3200);
    expect(row.inRange).toBe(false);
    expect(row.distance).toBe(0);
  });

  it('without an ETH/USD the bars are still drawn, with no distance', () => {
    const rows = ethBracketRows(BRACKETS_282, POSITIONS, null);
    expect(rows).toHaveLength(2);
    expect(rows.every((r) => r.distance === null)).toBe(true);
  });

  it('never draws a made-up edge: full range, zero, missing or non-numeric edges are left out', () => {
    const bad = [
      { ...BRACKETS_282[2], full_range: true },
      { ...BRACKETS_282[3], lower_price: '0' },
    ];
    expect(ethBracketRows(bad, POSITIONS, ETH_SPOT)).toEqual([]);
    expect(ethBracketRows([{ ...BRACKETS_282[2], upper_price: 'n/a' }], POSITIONS, ETH_SPOT)).toEqual([]);
    expect(ethBracketRows([{ ...BRACKETS_282[2], lower_price: undefined }], POSITIONS, ETH_SPOT)).toEqual([]);
    expect(ethBracketRows(undefined, POSITIONS, ETH_SPOT)).toEqual([]);
  });

  it('counts full-range ETH/USDC positions so the chart can name them', () => {
    expect(fullRangeEthUsdcCount(POSITIONS)).toBe(0);
    expect(fullRangeEthUsdcCount([...POSITIONS, { ...GAS_V3, position_id: 'z', full_range: true }])).toBe(1);
    expect(fullRangeEthUsdcCount([{ ...NOCK_USDC, full_range: true }])).toBe(0);
  });
});
