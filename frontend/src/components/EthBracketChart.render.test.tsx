/**
 * Static render of the ETH/USDC bracket section (#160). Recharts draws nothing without a
 * measured width, so this checks the captions: one line per range, the spot source, and
 * the honest states (no ETH/USD, full range, nothing to show).
 */
import { it, expect } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { EthBracketChart, ethBracketDomain, ethBracketText } from './EthBracketChart.tsx';
import type { EthBracketRow } from '../positions.ts';

const v3: EthBracketRow = { positionId: 'uniswap-v3:base:6607225', name: 'WETH/USDC v3 · 0.05%', lower: 1480.33, upper: 2951.31, liquidity: 251.92, inRange: true, distance: 15.37 };
const cl: EthBracketRow = { positionId: 'aerodrome-cl:base:3449558', name: 'WETH/USDC Aero CL', lower: 1475.92, upper: 2913.19, liquidity: 256.64, inRange: false, distance: 0 };

it('names each range with its USD-per-ETH span and status', () => {
  expect(ethBracketText(v3)).toBe('$1,480 – $2,951 · IN RANGE · 15.4% to the nearer edge');
  expect(ethBracketText(cl)).toBe('$1,476 – $2,913 · OUT OF RANGE');
  // In range by its own pool's tick, but the price pool's spot is across the edge.
  expect(ethBracketText({ ...v3, distance: 0 })).toBe('$1,480 – $2,951 · IN RANGE · at an edge');
  const html = renderToStaticMarkup(<EthBracketChart rows={[v3, cl]} ethUsd={1749.16} ethUsdFromSnapshot fullRangeCount={0} />);
  expect(html).toContain('ETH/USDC ranges (USD per ETH)');
  expect(html).toContain('measured, never steered');
  expect(html).toContain('WETH/USDC v3 · 0.05%: $1,480 – $2,951 · IN RANGE');
  expect(html).toContain('WETH/USDC Aero CL: $1,476 – $2,913 · OUT OF RANGE');
  expect(html).toContain('ETH spot $1,749 (snapshot ETH/USD)');
});

it('says when there is no ETH/USD, and names full-range positions', () => {
  const html = renderToStaticMarkup(<EthBracketChart rows={[v3]} ethUsd={null} ethUsdFromSnapshot={false} fullRangeCount={1} />);
  expect(html).toContain('No ETH/USD in this snapshot: no spot line');
  expect(html).toContain('1 full-range ETH/USDC position: never out of range, no bracket to draw');
});

it('renders nothing when there is no ETH/USDC range', () => {
  expect(renderToStaticMarkup(<EthBracketChart rows={[]} ethUsd={1749} ethUsdFromSnapshot fullRangeCount={0} />)).toBe('');
});

it('the axis holds every edge and the spot', () => {
  const [lo, hi] = ethBracketDomain([v3, cl], 1749.16);
  expect(lo).toBeLessThanOrEqual(1475.92);
  expect(hi).toBeGreaterThanOrEqual(2951.31);
  const [lo2, hi2] = ethBracketDomain([v3], 3500);
  expect(hi2).toBeGreaterThanOrEqual(3500);
  expect(lo2).toBeGreaterThan(0);
  expect(lo).toBeGreaterThan(0);
});
