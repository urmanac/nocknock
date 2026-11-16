import { describe, expect, it } from 'vitest';
import {
  AERO_NOT_REPORTED,
  aeroHeldLine,
  aeroNoteLine,
  aeroPositionRows,
  aeroPriceLine,
  aeroRewardsSentence,
  aeroStatSubtitle,
  compositionAeroText,
  compositionResidualLabel,
  positionAeroText,
  unclaimedAeroLine,
  unclaimedAeroText,
} from './aero.ts';
import { velocityRowText } from './positions.ts';
import { compositionReadout } from './timeline.ts';

// What the worker published for the real wallet at Base block 58077673
// (ingestion-worker/testdata/golden_snapshot_aero_rewards.json, ONCHAIN_INTEGRATION.md §9):
// both Aero positions staked, 12.043104 AERO earned and not claimed, none in the wallet.
const GLOBAL = {
  total_portfolio_usd: '2216.93',
  total_native_usd: '1167.42',
  total_stable_usd: '922.80',
  total_eth_usd: '126.71',
  aero_price_usd: '0.417608',
  wallet_aero_amount: '0.000000',
  wallet_aero_usd: '0.00',
  unclaimed_aero_amount: '12.043104',
  unclaimed_aero_usd: '5.03',
  aero_note: null,
};
const BASIC = {
  position_id: 'aerodrome:base:0x770fe8fc9248c6382950a86e39a9391b41d59e0a',
  protocol: 'aerodrome',
  nft_id: 0,
  fee_tier_bps: 30,
  token0: { symbol: 'USDC' },
  token1: { symbol: 'NOCK' },
  unclaimed_aero: '0.121026',
  unclaimed_aero_usd: '0.05',
};
const CL = {
  position_id: 'aerodrome-cl:base:2914490',
  protocol: 'aerodrome-cl',
  nft_id: 2914490,
  fee_tier_bps: 97,
  tick_spacing: 200,
  token0: { symbol: 'WETH' },
  token1: { symbol: 'NOCK' },
  fees_note: 'not LP-claimable (staked: swap fees on staked liquidity go to the gauge)',
  staked_in_gauge: '0x3bf70c880adc8370917994ab0df68e763b7a60fd',
  unclaimed_aero: '11.922078',
  unclaimed_aero_usd: '4.98',
};
const UNISWAP = {
  position_id: 'uniswap-v4:base:3163450',
  protocol: 'uniswap-v4',
  nft_id: 3163450,
  fee_tier_bps: 300,
  token0: { symbol: 'USDC' },
  token1: { symbol: 'NOCK' },
};
const POSITIONS = [UNISWAP, BASIC, CL];
// The same cycle with the amount of a real gauge claim in the wallet.
const HELD = { ...GLOBAL, total_portfolio_usd: '2217.50', wallet_aero_amount: '1.359979', wallet_aero_usd: '0.57' };

describe('AERO held and unclaimed (#93)', () => {
  it('states what the wallet holds, counted in the total, with no target', () => {
    expect(aeroHeldLine(HELD)).toBe(
      'AERO held (wallet): 1.359979 AERO ($0.57), counted in the total; not cash, not deployable, no target',
    );
    // Holding none is a read zero.
    expect(aeroHeldLine(GLOBAL)).toContain('0.000000 AERO ($0.00)');
  });

  it('states what is unclaimed in the tracked gauges, outside the total, per position', () => {
    expect(unclaimedAeroLine(GLOBAL, POSITIONS)).toBe(
      'Unclaimed AERO: 12.043104 AERO ($5.03) in tracked gauges, not counted in the total',
    );
    expect(aeroPositionRows(POSITIONS)).toEqual([
      { positionId: BASIC.position_id, label: 'USDC/NOCK Aero · 0.3%', text: '0.121026 AERO ($0.05)', read: true },
      { positionId: CL.position_id, label: 'WETH/NOCK Aero CL200 · 0.97%', text: '11.922078 AERO ($4.98)', read: true },
    ]);
    // A Uniswap position earns no AERO and has no row or card line.
    expect(positionAeroText(UNISWAP)).toBeNull();
    expect(positionAeroText(CL)).toBe('Unclaimed AERO: 11.922078 ($4.98); not counted in the total');
    expect(aeroPriceLine(GLOBAL)).toBe('AERO/USD: $0.417608');
    expect(aeroNoteLine(GLOBAL)).toBeNull();
  });

  it('a gauge with zero earned is a read zero, not "not reported"', () => {
    const zero = { ...BASIC, unclaimed_aero: '0.000000', unclaimed_aero_usd: '0.00' };
    const g = { ...GLOBAL, unclaimed_aero_amount: '0.000000', unclaimed_aero_usd: '0.00' };
    expect(positionAeroText(zero)).toBe('Unclaimed AERO: 0.000000 ($0.00); not counted in the total');
    expect(unclaimedAeroText(g, [zero])).toBe('0.000000 AERO ($0.00) in tracked gauges, not counted in the total');
  });

  it('a wallet with no Aero position says so instead of showing an amount', () => {
    const g = { ...GLOBAL, unclaimed_aero_amount: '0.000000', unclaimed_aero_usd: '0.00' };
    expect(unclaimedAeroText(g, [UNISWAP])).toBe('none: no staked Aero position is tracked');
    expect(unclaimedAeroText(g, [])).toBe('none: no staked Aero position is tracked');
    expect(aeroRewardsSentence(g, [UNISWAP])).toBe(
      'No staked Aero position is tracked, so no AERO is read from a gauge.',
    );
    expect(aeroStatSubtitle(g, [])).toBe(
      '$0.00 in the wallet, counted in the total · unclaimed: none: no staked Aero position is tracked',
    );
  });
});

describe('AERO that could not be read (#93)', () => {
  it('an older snapshot without the fields reads "not reported", never 0', () => {
    const old = { total_portfolio_usd: '2390.58', total_native_usd: '1312.88', total_stable_usd: '1033.71', total_eth_usd: '43.99' };
    const oldPositions = [UNISWAP, { ...BASIC, unclaimed_aero: undefined, unclaimed_aero_usd: undefined }];
    const lines = [
      aeroHeldLine(old),
      unclaimedAeroLine(old, oldPositions),
      aeroPriceLine(old),
      aeroStatSubtitle(old, oldPositions),
      aeroRewardsSentence(old, oldPositions),
    ];
    expect(lines).toEqual([
      'AERO held (wallet): not reported',
      'Unclaimed AERO: not reported',
      'AERO/USD: not reported',
      'not reported',
      'Unclaimed AERO on staked Aero LP: not reported.',
    ]);
    for (const l of [...lines, aeroHeldLine(undefined), unclaimedAeroLine(undefined, undefined)]) {
      expect(l).not.toMatch(/\d/);
    }
    expect(aeroPositionRows(oldPositions)).toEqual([]);
    expect(positionAeroText(oldPositions[1])).toBeNull();
    expect(AERO_NOT_REPORTED).toBe('not reported');
  });

  it('a failed price read keeps the amounts and says the USD value is not reported', () => {
    const note = 'AERO/USD price unavailable: AERO price pool 0xd0acf293547457fb4fe9ee32ca93e89497626816 slot0() read failed';
    const g = { ...GLOBAL, aero_price_usd: null, unclaimed_aero_usd: null, aero_note: note };
    const positions = [{ ...BASIC, unclaimed_aero_usd: undefined }, { ...CL, unclaimed_aero_usd: undefined }];
    expect(unclaimedAeroLine(g, positions)).toBe(
      'Unclaimed AERO: 12.043104 AERO (USD value not reported) in tracked gauges, not counted in the total',
    );
    expect(aeroPositionRows(positions)[1].text).toBe('11.922078 AERO (USD value not reported)');
    expect(positionAeroText(positions[1])).toBe('Unclaimed AERO: 11.922078 (USD value not reported); not counted in the total');
    expect(aeroPriceLine(g)).toBe('AERO/USD: not reported');
    expect(aeroNoteLine(g)).toBe(`AERO not read: ${note}`);
    // The wallet holds none, which is worth $0 whatever the price.
    expect(aeroHeldLine(g)).toContain('0.000000 AERO ($0.00)');
    expect(unclaimedAeroLine(g, positions)).not.toContain('$0');
  });

  it('a failed gauge read is a reason on its row and no total', () => {
    const reason = 'gauge 0x3bf70c880adc8370917994ab0df68e763b7a60fd earned() read failed: HTTP 429 Too Many Requests';
    const failed = { ...CL, unclaimed_aero: undefined, unclaimed_aero_usd: undefined, unclaimed_aero_note: reason };
    const g = { ...GLOBAL, unclaimed_aero_amount: null, unclaimed_aero_usd: null, aero_note: `unclaimed AERO not read for ${CL.position_id} (${reason})` };
    expect(unclaimedAeroLine(g, [BASIC, failed])).toBe('Unclaimed AERO: not reported');
    expect(aeroPositionRows([BASIC, failed]).map((r) => [r.read, r.text])).toEqual([
      [true, '0.121026 AERO ($0.05)'],
      [false, `not read (${reason})`],
    ]);
    expect(positionAeroText(failed)).toBe(`Unclaimed AERO: not read (${reason})`);
    expect(aeroNoteLine(g)).toContain('HTTP 429');
  });

  it('a basic pool with no gauge configured is "not read", and no zero is shown for it', () => {
    const reason = 'no gauge_address is configured for this position';
    const noGauge = { ...BASIC, unclaimed_aero: undefined, unclaimed_aero_usd: undefined, unclaimed_aero_note: reason };
    const g = { ...GLOBAL, unclaimed_aero_amount: '0.000000', unclaimed_aero_usd: '0.00' };
    expect(unclaimedAeroText(g, [noGauge])).toBe('no gauge was read');
    expect(aeroRewardsSentence(g, [noGauge])).toBe(
      'Unclaimed AERO on staked Aero LP: no gauge was read (see each position).',
    );
    expect(positionAeroText(noGauge)).toBe(`Unclaimed AERO: not read (${reason})`);
  });
});

describe('AERO on the other surfaces (#93)', () => {
  it('the velocity panel prints the worker note: the reason, then the unclaimed AERO', () => {
    const row = (name: string, note: string) =>
      velocityRowText({ name, claimable: 0, gasBreakEven: 0.0041, ratio: 0, action: 'not_claimable', note });
    expect(row('Aero USDC/NOCK 0.30% (full range)', 'not LP-claimable (claimFees() reverts); unclaimed AERO 0.121026 ($0.05)')).toBe(
      'Aero USDC/NOCK 0.30% (full range): not LP-claimable (claimFees() reverts); unclaimed AERO 0.121026 ($0.05)',
    );
    expect(row('Aero CL WETH/NOCK 0.97% #2914490', 'not LP-claimable (staked: swap fees on staked liquidity go to the gauge); unclaimed AERO 11.922078 ($4.98)')).toContain(
      'unclaimed AERO 11.922078 ($4.98)',
    );
    // A claimable row states its fee action only. An AERO amount beside CLAIM_NOW or
    // ACCUMULATE would read as advice about AERO, so even if a note arrives (the worker
    // sends none on such a row) it is not printed there.
    for (const action of ['claim_now', 'claim_recommended', 'accumulate']) {
      const text = velocityRowText({ name: 'Aero USDC/NOCK', claimable: 0.12, gasBreakEven: 0.0041, ratio: 29.3, action, note: 'unclaimed AERO 0.5 ($0.21)' });
      expect(text).toBe(`Aero USDC/NOCK: ${action.toUpperCase()} ($0.12 earned / gas $0.0041 — 29x ROI)`);
      expect(text).not.toContain('AERO');
    }
    // Nothing on any surface still says the rewards are not read (#93).
    expect(row('x', '')).not.toContain('#93');
  });

  it('the composition readout does not call wallet AERO "other legs"', () => {
    expect(compositionReadout(HELD).otherUsd).toBe(0);
    expect(compositionReadout(GLOBAL).otherUsd).toBe(0);
    // An older snapshot has no AERO field and reads exactly as before.
    const old = { total_portfolio_usd: '2413.57', total_native_usd: '1312.88', total_stable_usd: '1033.71', total_eth_usd: '66.98' };
    expect(compositionReadout(old).otherUsd).toBe(0);
  });

  it('the composition readout states wallet AERO, so the Total equals the parts shown', () => {
    // Held: the panel's Total includes $0.57 of AERO, and the readout says so.
    expect(compositionAeroText(HELD)).toBe('AERO $0.57 (wallet; not cash)');
    const ro = compositionReadout(HELD);
    const parts = ro.nativeUsd! + ro.stableUsd! + ro.ethUsd! + 0.57 + ro.otherUsd!;
    expect(Math.abs(ro.totalUsd! - parts)).toBeLessThan(0.005);
    // None held, under a cent, or not reported: nothing to state (never "AERO $0.00"
    // for a snapshot that does not carry the field).
    expect(compositionAeroText(GLOBAL)).toBeNull();
    expect(compositionAeroText({ ...GLOBAL, wallet_aero_usd: '0.004' })).toBeNull();
    expect(compositionAeroText({ total_portfolio_usd: '2413.57' })).toBeNull();
    expect(compositionAeroText({ ...GLOBAL, wallet_aero_usd: null })).toBeNull();
    expect(compositionAeroText(undefined)).toBeNull();
    // The residual's label names AERO whenever AERO was subtracted from it.
    expect(compositionResidualLabel(HELD, true)).toBe('Total − NOCK − USDC − ETH − AERO');
    expect(compositionResidualLabel(GLOBAL, true)).toBe('Total − NOCK − USDC − ETH − AERO');
    // An older snapshot keeps its wording.
    expect(compositionResidualLabel({ total_eth_usd: '66.98' }, true)).toBe('Total − NOCK − USDC − ETH');
    expect(compositionResidualLabel({}, false)).toBe('Total − NOCK − USDC, e.g. LP WETH');
    expect(compositionResidualLabel(undefined, false)).toBe('Total − NOCK − USDC, e.g. LP WETH');
    // It states an amount and gives no advice.
    expect(compositionAeroText(HELD)!.toLowerCase()).not.toMatch(/sell|accumulate|buy|swap|convert|harvest|should|recommend|advis|claim|compound/);
  });

  it('states amounts only: no advice about AERO in any line', () => {
    const reason = 'gauge 0xc82782ee66d82c4d19db0429652091a085083526 earned() reverted';
    const failed = { ...BASIC, unclaimed_aero: undefined, unclaimed_aero_usd: undefined, unclaimed_aero_note: reason };
    const texts = [
      aeroHeldLine(HELD),
      unclaimedAeroLine(HELD, POSITIONS),
      aeroPriceLine(HELD),
      aeroStatSubtitle(HELD, POSITIONS),
      aeroRewardsSentence(HELD, POSITIONS),
      positionAeroText(CL)!,
      positionAeroText(failed)!,
      ...aeroPositionRows(POSITIONS).map((r) => `${r.label}: ${r.text}`),
    ];
    for (const t of texts) {
      expect(t.toLowerCase()).not.toMatch(/sell|accumulate|buy|swap|convert|harvest|should|recommend|advis|claim now|claim it|compound/);
    }
  });
});
