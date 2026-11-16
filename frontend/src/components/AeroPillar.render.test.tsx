import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { AeroLiteCard, AeroPillarLines, PositionAeroLine } from './AeroPillar';

// The worker's figures for the real wallet at Base block 58077673 (see aero.test.ts).
const global = {
  aero_price_usd: '0.417608',
  wallet_aero_amount: '1.359979',
  wallet_aero_usd: '0.57',
  unclaimed_aero_amount: '12.043104',
  unclaimed_aero_usd: '5.03',
  aero_note: null,
};
const cl = {
  position_id: 'aerodrome-cl:base:2914490',
  protocol: 'aerodrome-cl',
  nft_id: 2914490,
  fee_tier_bps: 97,
  tick_spacing: 200,
  token0: { symbol: 'WETH' },
  token1: { symbol: 'NOCK' },
  unclaimed_aero: '11.922078',
  unclaimed_aero_usd: '4.98',
};
const uniswap = { position_id: 'uniswap-v4:base:1', protocol: 'uniswap-v4', nft_id: 1, fee_tier_bps: 300, token0: { symbol: 'USDC' }, token1: { symbol: 'NOCK' } };

describe('AeroPillar components (#93)', () => {
  it('the pillar lines render held, unclaimed, per-position and price', () => {
    const html = renderToStaticMarkup(<AeroPillarLines global={global} positions={[uniswap, cl]} />);
    expect(html).toContain('AERO held (wallet): 1.359979 AERO ($0.57), counted in the total; not cash, not deployable, no target');
    expect(html).toContain('Unclaimed AERO: 12.043104 AERO ($5.03) in tracked gauges, not counted in the total');
    expect(html).toContain('WETH/NOCK Aero CL200 · 0.97%: 11.922078 AERO ($4.98)');
    expect(html).toContain('AERO/USD: $0.417608');
    expect(html).not.toContain('AERO not read');
    // No control of any kind: AERO is stated, never acted on.
    expect(html).not.toMatch(/<button|<a /);
  });

  it('an older snapshot renders "not reported", never a zero', () => {
    const html = renderToStaticMarkup(<AeroPillarLines global={{ total_portfolio_usd: '2390.58' }} positions={[uniswap]} />);
    expect(html).toContain('AERO held (wallet): not reported');
    expect(html).toContain('Unclaimed AERO: not reported');
    expect(html).toContain('AERO/USD: not reported');
    expect(html).not.toMatch(/\$0|0\.0/);
    expect(renderToStaticMarkup(<AeroPillarLines global={undefined} />)).toContain('not reported');
  });

  it('the worker reason for an unread value is shown', () => {
    const note = 'AERO/USD price unavailable: Stale AERO/USD price: price pool 0xb27460d0 has zero in-range liquidity';
    const html = renderToStaticMarkup(
      <AeroPillarLines global={{ ...global, aero_price_usd: null, unclaimed_aero_usd: null, aero_note: note }} positions={[{ ...cl, unclaimed_aero_usd: undefined }]} />,
    );
    expect(html).toContain(`AERO not read: ${note}`);
    expect(html).toContain('11.922078 AERO (USD value not reported)');
  });

  it('a position card line appears only for a position with something to say', () => {
    expect(renderToStaticMarkup(<PositionAeroLine position={cl} />)).toContain('Unclaimed AERO: 11.922078 ($4.98); not counted in the total');
    expect(renderToStaticMarkup(<PositionAeroLine position={uniswap} />)).toBe('');
    expect(renderToStaticMarkup(<PositionAeroLine position={undefined} />)).toBe('');
  });

  it('the Lite card states the same facts and waits for the snapshot', () => {
    expect(renderToStaticMarkup(<AeroLiteCard data={null} />)).toContain('…');
    const html = renderToStaticMarkup(<AeroLiteCard data={{ positions: [cl], derived_metrics: { global_status: global } }} />);
    expect(html).toContain('AERO (FOURTH PILLAR)');
    expect(html).toContain('1.359979 AERO ($0.57), counted in the total');
    expect(html).toContain('unclaimed: 12.043104 AERO ($5.03) in tracked gauges, not counted in the total');
    expect(html).toContain('AERO/USD: $0.417608');
    expect(html).not.toMatch(/<button|<a /);
    // A snapshot with no AERO fields.
    const old = renderToStaticMarkup(<AeroLiteCard data={{ positions: [], derived_metrics: { global_status: {} } }} />);
    expect(old).toContain('not reported');
    expect(old).not.toMatch(/\$0|0\.0/);
  });
});
