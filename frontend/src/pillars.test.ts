import { describe, expect, it } from 'vitest';
import { NOT_REPORTED, ethUsdPrice, gasLine, hwmSubtitle, pillarLine, pillarsFromGlobal } from './pillars.ts';

// Live 2026-11-14 figures plus ~0.0125 ETH of wallet gas (issue #94).
const G230 = {
  total_portfolio_usd: '2413.57',
  total_native_usd: '1312.88',
  total_stable_usd: '1033.71',
  total_eth_usd: '66.98',
  total_eth_amount: '0.036418',
  wallet_gas_eth: '0.012500',
  wallet_gas_usd: '22.99',
  eth_price_usd: '1839.20',
};

describe('pillars (snapshot 2.3.0)', () => {
  it('NOCK, USDC, ETH and AERO, with ETH marked as gas', () => {
    // G230 predates #93: it has no AERO field, so the fourth pillar is unknown, not 0.
    const p = pillarsFromGlobal(G230);
    expect(p.map(x => x.key)).toEqual(['NOCK', 'USDC', 'ETH', 'AERO']);
    expect(p.map(x => x.usd)).toEqual([1312.88, 1033.71, 66.98, null]);
    expect(p.filter(x => x.gas).map(x => x.key)).toEqual(['ETH']);
    // The pillars partition the total.
    const sum = p.reduce((a, x) => a + (x.usd ?? 0), 0);
    expect(Math.abs(sum - 2413.57)).toBeLessThan(0.03);
  });

  it('the pillar line names each pillar; a missing one reads —, never $0.00', () => {
    expect(pillarLine(G230)).toBe('NOCK $1,312.88 · USDC $1,033.71 · ETH $66.98 · AERO —');
    const old = { total_portfolio_usd: '2390.58', total_native_usd: '1312.88', total_stable_usd: '1033.71' };
    expect(pillarLine(old)).toBe('NOCK $1,312.88 · USDC $1,033.71 · ETH — · AERO —');
    expect(pillarLine(undefined)).toBe('NOCK — · USDC — · ETH — · AERO —');
  });

  it('the four pillars add up to the total once the wallet holds AERO (#93)', () => {
    // The worker's figures for the real wallet at Base block 58077673, with the amount
    // of a real gauge claim (1.359979 AERO) in the wallet.
    const g = { total_portfolio_usd: '2217.50', total_native_usd: '1167.42', total_stable_usd: '922.80', total_eth_usd: '126.71', wallet_aero_usd: '0.57' };
    expect(pillarLine(g)).toBe('NOCK $1,167.42 · USDC $922.80 · ETH $126.71 · AERO $0.57');
    const sum = pillarsFromGlobal(g).reduce((a, x) => a + (x.usd ?? 0), 0);
    expect(Math.abs(sum - 2217.5)).toBeLessThan(0.03);
    // AERO is not gas and a wallet holding none reads $0.00: a read zero, not a gap.
    expect(pillarsFromGlobal(g)[3]).toEqual({ key: 'AERO', usd: 0.57, gas: false });
    expect(pillarLine({ ...g, wallet_aero_usd: '0.00' })).toContain('AERO $0.00');
    expect(pillarLine({ ...g, wallet_aero_usd: null })).toContain('AERO —');
  });

  it('gas is counted in the total, never cash or deployable; absent says so', () => {
    expect(gasLine(G230)).toBe(
      'Gas held (wallet ETH+WETH): 0.012500 ETH ($22.99), counted in the total; not cash, not deployable',
    );
    expect(gasLine({ wallet_gas_eth: '0.010000', wallet_gas_usd: null })).toContain('USD value not reported');
    expect(gasLine({})).toBe(`Gas held (wallet ETH+WETH): ${NOT_REPORTED}`);
    expect(gasLine({})).not.toContain('$0.00');
  });
});

describe('ethUsdPrice', () => {
  const pools = [{ volatile_assets: [{ token_symbol: 'WETH', reserve_amount: '0.023918', reserve_usd: '43.99' }] }];

  it('uses the snapshot eth_price_usd, never a reverse-engineered one', () => {
    expect(ethUsdPrice(G230, pools)).toBe(1839.2);
  });

  it('falls back to the pool ratio only for an older snapshot', () => {
    expect(ethUsdPrice({}, pools)).toBeCloseTo(43.99 / 0.023918, 6);
  });

  it('a dust leg cannot price ETH (0.01 / 0.000004 would read $2,500)', () => {
    const dust = [{ volatile_assets: [{ token_symbol: 'WETH', reserve_amount: '0.000004', reserve_usd: '0.01' }] }];
    expect(ethUsdPrice({}, dust)).toBeNull();
  });

  it('no price anywhere is null, never a default', () => {
    expect(ethUsdPrice({ eth_price_usd: null }, [])).toBeNull();
    expect(ethUsdPrice(undefined, undefined)).toBeNull();
  });
});

describe('hwmSubtitle', () => {
  it('says when the mark was set, not "All-time high"', () => {
    const s = hwmSubtitle('2026-11-14T04:38:29Z');
    expect(s).toContain('set 2026-11-14 04:38 UTC');
    expect(s).not.toContain('All-time');
  });

  it('an unknown date is not invented', () => {
    expect(hwmSubtitle(undefined)).not.toContain('set');
  });
});
