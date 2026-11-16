/**
 * Static render of the #91 notes: what a position card and the tick bracket panel say
 * for a staked Aero CL position next to the positions that existed before it.
 */
import { it, expect } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { PositionStakingLine, RangeSideList } from './PositionRangeNotes.tsx';

const cl = {
  position_id: 'aerodrome-cl:base:2914490',
  protocol: 'aerodrome-cl',
  fee_tier_bps: 97,
  tick_spacing: 200,
  token0: { symbol: 'WETH' },
  token1: { symbol: 'NOCK' },
  amount0: '0.055908',
  amount1: '6945.829656',
  staked_in_gauge: '0x3bf70c880adc8370917994ab0df68e763b7a60fd',
  lower_tick: 73400,
  upper_tick: 74800,
  in_range: true,
  full_range: false,
  pool_active: true,
};
const v4 = {
  position_id: 'uniswap-v4:base:3163450',
  protocol: 'uniswap-v4',
  fee_tier_bps: 300,
  token0: { symbol: 'USDC' },
  token1: { symbol: 'NOCK' },
  amount0: '0.000000',
  amount1: '9000.000000',
  lower_tick: 262200,
  upper_tick: 282300,
  in_range: false,
  full_range: false,
  pool_active: true,
};
const aeroBasic = {
  position_id: 'aerodrome:base:0x770fe8fc9248c6382950a86e39a9391b41d59e0a',
  protocol: 'aerodrome',
  fee_tier_bps: 30,
  token0: { symbol: 'USDC' },
  token1: { symbol: 'NOCK' },
  in_range: true,
  full_range: true,
  pool_active: true,
};

it('a staked position card names its gauge; an unstaked one adds nothing', () => {
  const html = renderToStaticMarkup(<PositionStakingLine position={cl} />);
  expect(html).toContain('Staked in gauge 0x3bf7…60fd for this wallet');
  // The full address is available on hover.
  expect(html).toContain('title="0x3bf70c880adc8370917994ab0df68e763b7a60fd"');
  expect(renderToStaticMarkup(<PositionStakingLine position={v4} />)).toBe('');
  expect(renderToStaticMarkup(<PositionStakingLine position={aeroBasic} />)).toBe('');
});

it('the tick bracket panel lists every ranged position with its side', () => {
  const html = renderToStaticMarkup(
    <RangeSideList
      positions={[v4, cl, aeroBasic]}
      poolMechanics={[{ tick_current: 283000 }, { tick_current: 74002 }, { tick_current: 0 }]}
    />,
  );
  expect(html).toContain('WETH/NOCK Aero CL200 · 0.97%: in range, holding WETH and NOCK');
  expect(html).toContain(
    'USDC/NOCK v4 · 3%: out of range below: NOCK is priced under this range, which now holds only NOCK',
  );
  // A full-range pool has no side and is not listed; nothing is called Uniswap by mistake.
  expect(html).not.toContain('Aero · 0.3%');
  expect(html).not.toContain('Uniswap');
  // No ranged positions: nothing is rendered.
  expect(renderToStaticMarkup(<RangeSideList positions={[aeroBasic]} />)).toBe('');
  expect(renderToStaticMarkup(<RangeSideList positions={[]} />)).toBe('');
});
