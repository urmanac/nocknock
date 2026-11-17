import { describe, expect, it } from 'vitest';
import {
  EARNINGS_KIND_GAUGE_REWARD,
  LP_FEE_COLOR,
  LP_FEE_SERIES_NAME,
  REWARD_COLOR,
  REWARD_NOT_REPORTED,
  REWARD_SERIES_NAME,
  claimableYieldTile,
  collectibleTotals,
  collectibleTotalsLine,
  liteCollectibleTotalsLine,
  liteRewardLines,
  lpFeeReasonOfDetail,
  positionEarningsLines,
  rewardAmountText,
  rewardOf,
  rewardStatement,
  rewardWhereText,
  velocityHasRewards,
  velocityRewardRowText,
  velocityRowColor,
  velocityRows,
} from './rewards.ts';
import { velocityRowText } from './positions.ts';
import {
  BASIC,
  BASIC_GAUGE,
  BASIC_REASON,
  CL,
  CL_GAUGE,
  CONTRACTS,
  CONTRACT_FEES,
  GLOBAL,
  OLD_BASIC,
  OLD_CL,
  OLD_CONTRACTS,
  OLD_GLOBAL,
  OLD_UNISWAP_V4,
  PAYLOAD,
  POSITIONS,
  STAKED_REASON,
  reward,
} from './rewards.fixture.ts';

/** Words that would turn a statement about AERO into advice. */
const ADVICE_WORDS = [
  'should', 'worth claiming', 'claim now', 'claim_now', 'claim recommended', 'claim_recommended', 'recommend',
  'advis', 'accumulate', 'sell', 'buy', 'swap', 'convert', 'harvest', 'hold it', 'keep it', 'claim it',
  'stake more', 'compound', 'consider', 'you can claim', 'time to',
];

function expectNoAdvice(text: string) {
  // The worker's measured reason for the LP-fee part names "swap fees": a fact about
  // where the pool's fees go, not a verb.
  const lower = text.toLowerCase().replace('swap fees on staked liquidity go to the gauge', '');
  for (const banned of ADVICE_WORDS) {
    expect(lower, `found "${banned}" in: ${text}`).not.toContain(banned);
  }
}

const NOT_READ_REASON = `gauge ${CL_GAUGE} earned() reverted`;
const CL_NOT_READ = {
  ...OLD_CL,
  unclaimed_aero: undefined,
  unclaimed_aero_usd: undefined,
  unclaimed_aero_note: NOT_READ_REASON,
  collectible_reward: reward(null, null, CL_GAUGE, NOT_READ_REASON),
};
const GLOBAL_NOT_TOTALLED = {
  ...OLD_GLOBAL,
  unclaimed_aero_amount: null,
  unclaimed_aero_usd: null,
  collectible_rewards_usd: null,
  collectible_rewards_note: `gauge reward not read for aerodrome-cl:base:2914490 (${NOT_READ_REASON})`,
  total_collectible_usd: null,
};

describe('a reward as collectible earnings (#142)', () => {
  it('reads the structured reward from a position, a fee row and a payload row', () => {
    const fromPosition = rewardOf(CL)!;
    expect(fromPosition).toEqual({
      read: true,
      symbol: 'AERO',
      amount: '17.654015',
      usd: '7.75',
      gauge: CL_GAUGE,
      collectedIn: 'the Aero app',
      note: null,
    });
    expect(rewardOf(CONTRACTS[2])).toEqual(fromPosition);
    expect(rewardOf(CONTRACT_FEES[2])).toEqual(fromPosition);
    expect(rewardOf(PAYLOAD.rewards[1])).toEqual(fromPosition);
    // A position that cannot earn one, an older snapshot's position, and nothing at all.
    expect(rewardOf(OLD_UNISWAP_V4)).toBeNull();
    expect(rewardOf(OLD_CL)).toBeNull();
    expect(rewardOf(undefined)).toBeNull();
    expect(rewardOf({ collectible_reward: null })).toBeNull();
  });

  it('states the amount, its USD value and where it is collected', () => {
    const r = rewardOf(CL)!;
    expect(rewardAmountText(r)).toBe('17.654015 AERO ($7.75)');
    expect(rewardWhereText(r)).toBe('collectible from gauge 0x3bf7…60fd in the Aero app');
    expect(rewardStatement(r)).toBe('17.654015 AERO ($7.75), collectible from gauge 0x3bf7…60fd in the Aero app');
    expect(rewardStatement(rewardOf(BASIC)!)).toBe('0.210015 AERO ($0.09), collectible from gauge 0xc827…3526 in the Aero app');
  });

  it('an unpriced reward has its amount and no dollar figure', () => {
    const r = rewardOf({ collectible_reward: reward('17.654015', null, CL_GAUGE) })!;
    expect(r.read).toBe(true);
    expect(r.usd).toBeNull();
    expect(rewardAmountText(r)).toBe('17.654015 AERO (USD value not reported)');
    expect(rewardStatement(r)).not.toContain('$');
  });

  it('a reward that was not read says so with its reason, never 0', () => {
    const r = rewardOf(CL_NOT_READ)!;
    expect(r.read).toBe(false);
    expect(r.amount).toBeNull();
    expect(r.usd).toBeNull();
    expect(rewardAmountText(r)).toBeNull();
    expect(rewardStatement(r)).toBe(`not read (${NOT_READ_REASON})`);
    // A stray usd on an unread reward is not shown either.
    expect(rewardOf({ collectible_reward: { ...reward(null, null, CL_GAUGE, 'x'), usd: '0.00' } })!.usd).toBeNull();
    // No gauge configured: no gauge is named.
    const noGauge = rewardOf({ collectible_reward: reward(null, null, null, 'no gauge_address is configured for this position') })!;
    expect(rewardStatement(noGauge)).toBe('not read (no gauge_address is configured for this position)');
    expect(rewardWhereText(rewardOf({ collectible_reward: reward('1.000000', '0.44', null) })!)).toBe('collectible in the Aero app');
  });

  it('a read zero is a zero, with no figure invented around it', () => {
    const r = rewardOf({ collectible_reward: reward('0.000000', '0.00', CL_GAUGE) })!;
    expect(r.read).toBe(true);
    expect(rewardAmountText(r)).toBe('0.000000 AERO ($0.00)');
  });
});

describe('position card earnings (#142)', () => {
  it('shows the reward as the collectible earnings with the LP-fee reason beneath', () => {
    expect(positionEarningsLines(CL, CONTRACT_FEES[2])).toEqual({
      earnings: 'Collectible earnings: 17.654015 AERO ($7.75), collectible from gauge 0x3bf7…60fd in the Aero app; not counted in the total',
      lpFees: `LP fees: ${STAKED_REASON}`,
    });
    // The Aero basic pool has no fees_note of its own: the reason comes from its fee row.
    expect(positionEarningsLines(BASIC, CONTRACT_FEES[1])).toEqual({
      earnings: 'Collectible earnings: 0.210015 AERO ($0.09), collectible from gauge 0xc827…3526 in the Aero app; not counted in the total',
      lpFees: `LP fees: ${BASIC_REASON}`,
    });
    // A position whose LP fees ARE claimable keeps its fee figure beside the reward.
    const claimable = { ...BASIC, fees_usd: '8.000000' };
    expect(positionEarningsLines(claimable, { fee_token_symbol: 'x' }).lpFees).toBe('LP fees: $8.000000');
  });

  it('a reward that was not read is stated as not read', () => {
    const lines = positionEarningsLines(CL_NOT_READ, CONTRACT_FEES[2]);
    expect(lines.earnings).toBe(`Collectible earnings: reward not read (${NOT_READ_REASON})`);
    expect(lines.earnings).not.toMatch(/\$|0\.0/);
    expect(lines.lpFees).toBe(`LP fees: ${STAKED_REASON}`);
  });

  it('an older snapshot and a Uniswap position render as before', () => {
    expect(positionEarningsLines(OLD_CL)).toEqual({ earnings: null, lpFees: `Fees: ${STAKED_REASON}` });
    expect(positionEarningsLines(OLD_BASIC)).toEqual({ earnings: null, lpFees: 'Fees: $0.000000' });
    expect(positionEarningsLines(OLD_UNISWAP_V4)).toEqual({ earnings: null, lpFees: 'Fees: $0.542769' });
  });
});

describe('collectible totals (#142)', () => {
  it('states LP fees and AERO rewards as two labelled figures and their total', () => {
    expect(collectibleTotals(GLOBAL, POSITIONS)).toEqual({
      lpFees: '$1.95',
      rewards: '$7.84',
      total: '$9.79',
      totalled: true,
      reported: true,
    });
    expect(collectibleTotalsLine(GLOBAL, POSITIONS)).toBe('LP fees $1.95 · AERO rewards $7.84 · total collectible $9.79');
  });

  it('a failed gauge read leaves the total unstated, never a partial sum', () => {
    const t = collectibleTotals(GLOBAL_NOT_TOTALLED, [OLD_UNISWAP_V4, BASIC, CL_NOT_READ]);
    expect(t.lpFees).toBe('$1.95');
    expect(t.rewards).toBe(`not totalled (gauge reward not read for aerodrome-cl:base:2914490 (${NOT_READ_REASON}))`);
    expect(t.total).toBe('not totalled');
    expect(t.totalled).toBe(false);
    expect(t.reported).toBe(true);
    // Not the $0.09 of the gauge that was read, and not a zero.
    const line = collectibleTotalsLine(GLOBAL_NOT_TOTALLED, [BASIC, CL_NOT_READ]);
    expect(line).not.toContain('0.09');
    expect(line).not.toMatch(/AERO rewards \$/);
    expect(line).not.toMatch(/total collectible \$/);
  });

  it('an older snapshot reads "not reported", never 0', () => {
    const t = collectibleTotals(OLD_GLOBAL, [OLD_UNISWAP_V4, OLD_BASIC, OLD_CL]);
    expect(t).toEqual({ lpFees: '$1.95', rewards: REWARD_NOT_REPORTED, total: REWARD_NOT_REPORTED, totalled: false, reported: false });
    expect(collectibleTotalsLine(OLD_GLOBAL)).toBe('LP fees $1.95 · AERO rewards not reported · total collectible not reported');
    // Keys present but null with no reason (a stored snapshot the worker re-serialized).
    expect(collectibleTotals({ ...OLD_GLOBAL, collectible_rewards_usd: null, total_collectible_usd: null, collectible_rewards_note: null }).rewards).toBe(REWARD_NOT_REPORTED);
    expect(collectibleTotals(undefined)).toEqual({ lpFees: 'unavailable', rewards: REWARD_NOT_REPORTED, total: REWARD_NOT_REPORTED, totalled: false, reported: false });
  });

  it('a wallet with no staked Aero position says "none", and its total is its LP fees', () => {
    const global = { ...OLD_GLOBAL, total_fees_claimable_usd: '0.54', collectible_rewards_usd: '0.00', collectible_rewards_note: null, total_collectible_usd: '0.54' };
    const t = collectibleTotals(global, [OLD_UNISWAP_V4]);
    expect(t.rewards).toBe('none (no staked Aero position is tracked)');
    expect(t.total).toBe('$0.54');
    // Every gauge answered zero: a real $0.00, with the positions that earned it.
    expect(collectibleTotals(global, [{ collectible_reward: reward('0.000000', '0.00', CL_GAUGE) }]).rewards).toBe('$0.00');
  });
});

describe('the CLAIMABLE YIELD tile (#142)', () => {
  it('shows the total with LP fees and AERO rewards as two labelled figures', () => {
    expect(claimableYieldTile(GLOBAL, POSITIONS)).toEqual({
      label: 'Claimable Yield (LP fees + AERO rewards)',
      value: '9.79',
      subtitle: "LP fees $1.95 · AERO rewards $7.84 in gauges · neither is counted in the portfolio total · LP fees net of two claims' gas $1.94",
    });
    // A negative LP net keeps its sign.
    expect(claimableYieldTile({ ...GLOBAL, gas_to_fee_delta_usd: '-0.01' }, POSITIONS).subtitle).toContain("LP fees net of two claims' gas -$0.01");
  });

  it('keeps the LP-fee net and says why there is no total when a gauge was not read', () => {
    const tile = claimableYieldTile(GLOBAL_NOT_TOTALLED, [BASIC, CL_NOT_READ]);
    expect(tile.label).toBe('Claimable Yield (LP fees, net)');
    expect(tile.value).toBe('1.94');
    expect(tile.subtitle).toBe(
      `LP fees $1.95 gross; net of two claims' gas · AERO rewards not totalled (gauge reward not read for aerodrome-cl:base:2914490 (${NOT_READ_REASON})) · total collectible not totalled`,
    );
    expect(tile.subtitle).not.toContain('0.09');
  });

  it('an older snapshot renders the tile it always did, with "not reported"', () => {
    expect(claimableYieldTile(OLD_GLOBAL, [OLD_BASIC, OLD_CL])).toEqual({
      label: 'Claimable Yield (Net)',
      value: '1.94',
      subtitle: "Gross: $1.95; net of two claims' gas · AERO rewards not reported",
    });
    // No snapshot at all: a dash, never a zero.
    const empty = claimableYieldTile(undefined);
    expect(empty.value).toBeUndefined();
    expect(empty.subtitle).toBe("Gross: —; net of two claims' gas · AERO rewards not reported");
  });
});

describe('velocity rows and chart series (#142)', () => {
  it('a staked position reports its reward as its own series, apart from LP fees', () => {
    const rows = velocityRows(CONTRACTS);
    expect(rows.map((r) => r.kind)).toEqual(['lp_fees', 'gauge_reward', 'gauge_reward']);
    // LP fees stay what they were; the reward is a separate number.
    expect(rows.map((r) => r.claimable)).toEqual([0.54, 0, 0]);
    expect(rows.map((r) => r.reward)).toEqual([null, 0.09, 7.75]);
    expect(velocityHasRewards(rows)).toBe(true);
    expect(LP_FEE_SERIES_NAME).not.toBe(REWARD_SERIES_NAME);
    expect(LP_FEE_COLOR).not.toBe(REWARD_COLOR);
    expect(rows.map(velocityRowColor)).toEqual([LP_FEE_COLOR, REWARD_COLOR, REWARD_COLOR]);
    // The reward row never carries a harvest word, and its kind is not one of them.
    for (const r of rows.filter((x) => x.kind === 'gauge_reward')) {
      expect(r.action).toBe('not_claimable');
      expect(['claim_now', 'claim_recommended', 'accumulate']).not.toContain(EARNINGS_KIND_GAUGE_REWARD);
    }
  });

  it('the text row states the reward, then the LP-fee reason', () => {
    const rows = velocityRows(CONTRACTS);
    expect(velocityRewardRowText(rows[2])).toBe(
      `Aero CL WETH/NOCK 0.97% #2914490: reward collectible in gauge 0x3bf7…60fd: 17.654015 AERO ($7.75); LP fees ${STAKED_REASON}`,
    );
    expect(velocityRewardRowText(rows[1])).toBe(
      `Aero USDC/NOCK 0.30% (full range): reward collectible in gauge 0xc827…3526: 0.210015 AERO ($0.09); LP fees ${BASIC_REASON}`,
    );
    // An LP-fee row reads exactly as it did.
    expect(velocityRewardRowText(rows[0])).toBe(velocityRowText(rows[0]));
    expect(velocityRewardRowText(rows[0])).toBe('Uniswap V4 (USDC/NOCK 3.00%, #3163450): CLAIM_NOW ($0.54 earned / gas $0.0036 — 150x ROI)');
  });

  it('an unpriced or unread reward draws no bar and still states what is known', () => {
    const unpriced = velocityRows([{ ...CONTRACTS[2], reward: reward('17.654015', null, CL_GAUGE) }])[0];
    expect(unpriced.reward).toBeNull();
    expect(velocityRewardRowText(unpriced)).toBe(
      `Aero CL WETH/NOCK 0.97% #2914490: reward collectible in gauge 0x3bf7…60fd: 17.654015 AERO (USD value not reported); LP fees ${STAKED_REASON}`,
    );
    const unread = velocityRows([{ ...CONTRACTS[2], reward: reward(null, null, CL_GAUGE, NOT_READ_REASON) }])[0];
    expect(unread.kind).toBe('gauge_reward');
    expect(unread.reward).toBeNull();
    expect(velocityRewardRowText(unread)).toBe(`Aero CL WETH/NOCK 0.97% #2914490: reward not read (${NOT_READ_REASON}); LP fees ${STAKED_REASON}`);
  });

  it('a row with a fee action never shows a reward, even if one were attached', () => {
    // The worker sends no reward on such a row; a reward without the kind is ignored.
    const row = velocityRows([{ ...OLD_CONTRACTS[0], reward: reward('1.000000', '0.44', BASIC_GAUGE) }])[0];
    expect(row.kind).toBe('lp_fees');
    expect(row.reward).toBeNull();
    expect(velocityRewardRowText(row)).not.toContain('AERO');
  });

  it('an older snapshot draws and reads as before', () => {
    const rows = velocityRows(OLD_CONTRACTS);
    expect(rows.every((r) => r.kind === 'lp_fees' && r.reward === null)).toBe(true);
    expect(velocityHasRewards(rows)).toBe(false);
    expect(velocityRewardRowText(rows[2])).toBe(`Aero CL WETH/NOCK 0.97% #2914490: ${STAKED_REASON}; unclaimed AERO 17.654015 ($7.75)`);
    expect(rows.map(velocityRowColor)).toEqual([LP_FEE_COLOR, '#f59e0b', '#f59e0b']);
    expect(velocityRows(undefined)).toEqual([]);
  });
});

describe('the Lite cockpit lines (#142)', () => {
  it('lists each reward with where it is collected and the gas statement', () => {
    expect(liteRewardLines(PAYLOAD, null)).toEqual([
      {
        key: BASIC.position_id,
        text: 'USDC+NOCK (Aerodrome): 0.210015 AERO ($0.09), collectible from gauge 0xc827…3526 in the Aero app · gas est. 390860 units (~$0.0063)',
        read: true,
      },
      {
        key: CL.position_id,
        text: 'WETH+NOCK (Aero CL #2914490): 17.654015 AERO ($7.75), collectible from gauge 0x3bf7…60fd in the Aero app · gas est. 418528 units (~$0.0067)',
        read: true,
      },
    ]);
    expect(liteCollectibleTotalsLine(PAYLOAD, null)).toBe('LP fees $1.95 · AERO rewards $7.84 · total collectible $9.79');
  });

  it('a gauge with no verified claim call says so in place of a gas figure', () => {
    const payload = {
      ...PAYLOAD,
      rewards: [{ ...PAYLOAD.rewards[1], gas_estimate: null, gas_usd: null, gas_note: 'gas not estimated (claim call not verified for this gauge)' }],
    };
    expect(liteRewardLines(payload, null)[0].text).toBe(
      'WETH+NOCK (Aero CL #2914490): 17.654015 AERO ($7.75), collectible from gauge 0x3bf7…60fd in the Aero app · gas not estimated (claim call not verified for this gauge)',
    );
  });

  it('falls back to the snapshot when the payload carries no rewards', () => {
    const oldPayload = { ...PAYLOAD, rewards: undefined, collectible_rewards_usd: undefined, total_collectible_usd: undefined, collectible_lp_fees_usd: undefined };
    const data = { positions: POSITIONS, contract_fees: CONTRACT_FEES, derived_metrics: { global_status: GLOBAL } };
    expect(liteRewardLines(oldPayload, data).map((l) => l.text)).toEqual([
      'USDC+NOCK (Aerodrome): 0.210015 AERO ($0.09), collectible from gauge 0xc827…3526 in the Aero app',
      'WETH+NOCK (Aero CL #2914490): 17.654015 AERO ($7.75), collectible from gauge 0x3bf7…60fd in the Aero app',
    ]);
    expect(liteCollectibleTotalsLine(oldPayload, data)).toBe('LP fees $1.95 · AERO rewards $7.84 · total collectible $9.79');
  });

  it('an older worker and an older snapshot give no lines and "not reported"', () => {
    const oldPayload = { gross_usd: '1.95', actions: [] };
    const oldData = { positions: [OLD_UNISWAP_V4, OLD_BASIC, OLD_CL], derived_metrics: { global_status: OLD_GLOBAL } };
    expect(liteRewardLines(oldPayload, oldData)).toEqual([]);
    expect(liteCollectibleTotalsLine(oldPayload, oldData)).toBe('LP fees $1.95 · AERO rewards not reported · total collectible not reported');
    expect(liteRewardLines(null, null)).toEqual([]);
    expect(liteCollectibleTotalsLine(null, null)).toBe('LP fees unavailable · AERO rewards not reported · total collectible not reported');
  });

  it('a reward that was not read is a line of its own, never a zero', () => {
    const payload = {
      ...PAYLOAD,
      collectible_rewards_usd: null,
      total_collectible_usd: null,
      collectible_rewards_note: GLOBAL_NOT_TOTALLED.collectible_rewards_note,
      rewards: [
        PAYLOAD.rewards[0],
        { position_id: CL.position_id, label: 'WETH+NOCK (Aero CL #2914490)', ...reward(null, null, CL_GAUGE, NOT_READ_REASON), gas_estimate: null, gas_usd: null, gas_note: 'gas not estimated (the reward was not read)' },
      ],
    };
    const lines = liteRewardLines(payload, null);
    expect(lines[1]).toEqual({ key: CL.position_id, text: `WETH+NOCK (Aero CL #2914490): reward not read (${NOT_READ_REASON})`, read: false });
    expect(liteCollectibleTotalsLine(payload, null)).toBe(
      `LP fees $1.95 · AERO rewards not totalled (gauge reward not read for aerodrome-cl:base:2914490 (${NOT_READ_REASON})) · total collectible not totalled`,
    );
  });

  it('repeats only the LP-fee reason of a not_claimable action', () => {
    expect(lpFeeReasonOfDetail(PAYLOAD.actions[1].simulation_detail)).toBe(STAKED_REASON);
    expect(lpFeeReasonOfDetail(PAYLOAD.actions[0].simulation_detail)).toBe(BASIC_REASON);
    expect(lpFeeReasonOfDetail(STAKED_REASON)).toBe(STAKED_REASON);
    expect(lpFeeReasonOfDetail(null)).toBe('not LP-claimable');
  });
});

describe('no advice about AERO on any of these surfaces (#142)', () => {
  it('every reward text states facts only', () => {
    const texts: string[] = [];
    const push = (...t: (string | null)[]) => texts.push(...(t.filter(Boolean) as string[]));
    for (const p of [BASIC, CL, CL_NOT_READ, { collectible_reward: reward('17.654015', null, CL_GAUGE) }, { collectible_reward: reward('0.000000', '0.00', CL_GAUGE) }]) {
      const lines = positionEarningsLines(p, CONTRACT_FEES[2]);
      push(lines.earnings, lines.lpFees, rewardStatement(rewardOf(p)!));
    }
    for (const g of [GLOBAL, GLOBAL_NOT_TOTALLED, OLD_GLOBAL]) {
      const tile = claimableYieldTile(g, POSITIONS);
      push(tile.label, tile.subtitle, collectibleTotalsLine(g, POSITIONS));
    }
    for (const row of velocityRows([...CONTRACTS, { ...CONTRACTS[2], reward: reward(null, null, CL_GAUGE, NOT_READ_REASON) }])) {
      if (row.kind === 'gauge_reward') push(velocityRewardRowText(row));
    }
    push(...liteRewardLines(PAYLOAD, null).map((l) => l.text), liteCollectibleTotalsLine(PAYLOAD, null), LP_FEE_SERIES_NAME, REWARD_SERIES_NAME);
    expect(texts.length).toBeGreaterThan(20);
    for (const text of texts) expectNoAdvice(text);
    // And no link to the retired app.
    for (const text of texts) expect(text).not.toContain('aerodrome.finance');
  });

  it('the fixture rewards point at the Aero app only', () => {
    for (const p of [BASIC, CL]) {
      expect(p.collectible_reward.collect_url).toBe('https://app.aero.xyz');
      expect(p.collectible_reward.gauge).toMatch(/^0x[0-9a-f]{40}$/);
    }
    expect(BASIC.collectible_reward.gauge).toBe(BASIC_GAUGE);
  });
});
