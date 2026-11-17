import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { LiteCollectibleLines, PositionEarningsLines, hasPositionEarnings } from './CollectibleEarnings';
import { REWARD_COLOR } from '../rewards';
import {
  BASIC,
  CL,
  CL_GAUGE,
  CONTRACT_FEES,
  GLOBAL,
  OLD_BASIC,
  OLD_CL,
  OLD_GLOBAL,
  OLD_UNISWAP_V4,
  PAYLOAD,
  POSITIONS,
  STAKED_REASON,
  reward,
} from '../rewards.fixture';

// The figures the deployed worker published on 2026-11-17 at 11:05Z (rewards.fixture.ts).

describe('CollectibleEarnings components (#142)', () => {
  it('a position card shows the reward as collectible earnings, the LP-fee reason beneath', () => {
    const html = renderToStaticMarkup(<PositionEarningsLines position={CL} feeRow={CONTRACT_FEES[2]} />);
    expect(html).toContain('Collectible earnings: 17.654015 AERO ($7.75), collectible from gauge 0x3bf7…60fd in the Aero app; not counted in the total');
    expect(html).toContain(`LP fees: ${STAKED_REASON}`);
    // The reward line comes first and is drawn in the reward colour.
    expect(html.indexOf('Collectible earnings')).toBeLessThan(html.indexOf('LP fees:'));
    expect(html).toContain(`color:${REWARD_COLOR}`);
    // The full gauge address is on the line's title; nothing here is a control.
    expect(html).toContain(`title="${CL_GAUGE}"`);
    expect(html).not.toMatch(/<button|<a /);
    expect(hasPositionEarnings(CL)).toBe(true);
  });

  it('the Aero basic pool gets its LP-fee reason from its fee row', () => {
    const html = renderToStaticMarkup(<PositionEarningsLines position={BASIC} feeRow={CONTRACT_FEES[1]} />);
    expect(html).toContain('Collectible earnings: 0.210015 AERO ($0.09), collectible from gauge 0xc827…3526 in the Aero app');
    expect(html).toContain('LP fees: not LP-claimable (claimFees() reverts)');
    expect(html).not.toContain('$0.000000');
  });

  it('a reward that was not read is a warning line, never a zero', () => {
    const notRead = { ...OLD_CL, unclaimed_aero: undefined, unclaimed_aero_usd: undefined, collectible_reward: reward(null, null, CL_GAUGE, 'gauge read failed') };
    const html = renderToStaticMarkup(<PositionEarningsLines position={notRead} feeRow={CONTRACT_FEES[2]} />);
    expect(html).toContain('Collectible earnings: reward not read (gauge read failed)');
    expect(html).not.toContain(`color:${REWARD_COLOR}`);
    expect(html).not.toMatch(/0\.000000|\$0/);
  });

  it('a position without a structured reward renders nothing (the card keeps its old lines)', () => {
    for (const p of [OLD_CL, OLD_BASIC, OLD_UNISWAP_V4, undefined]) {
      expect(renderToStaticMarkup(<PositionEarningsLines position={p} />)).toBe('');
      expect(hasPositionEarnings(p)).toBe(false);
    }
  });

  it('the Lite cockpit lists the reward lines and the split totals', () => {
    const html = renderToStaticMarkup(<LiteCollectibleLines payload={PAYLOAD} data={{ positions: POSITIONS, derived_metrics: { global_status: GLOBAL } }} />);
    expect(html).toContain('LP fees $1.95 · AERO rewards $7.84 · total collectible $9.79');
    expect(html).toContain('WETH+NOCK (Aero CL #2914490): 17.654015 AERO ($7.75), collectible from gauge 0x3bf7…60fd in the Aero app · gas est. 418528 units (~$0.0067)');
    expect(html).toContain('USDC+NOCK (Aerodrome): 0.210015 AERO ($0.09), collectible from gauge 0xc827…3526 in the Aero app · gas est. 390860 units (~$0.0063)');
    // Facts only: a reward is never a button or a link in the cockpit.
    expect(html).not.toMatch(/<button|<a /);
  });

  it('an older worker and snapshot add nothing to the card', () => {
    // The 2.6.0 snapshot shows its unclaimed AERO on the AERO card; a line saying
    // "AERO rewards not reported" beside it would contradict that figure.
    const html = renderToStaticMarkup(
      <LiteCollectibleLines payload={{ gross_usd: '1.95', actions: [] }} data={{ positions: [OLD_BASIC, OLD_CL], derived_metrics: { global_status: OLD_GLOBAL } }} />,
    );
    expect(html).toBe('');
    // A #142 worker serving that stored snapshot: keys present, null, no reward rows.
    const reserialized = renderToStaticMarkup(
      <LiteCollectibleLines
        payload={{ gross_usd: '1.95', actions: [], collectible_lp_fees_usd: '1.95', collectible_rewards_usd: null, collectible_rewards_note: null, total_collectible_usd: null, rewards: [] }}
        data={{ positions: [OLD_BASIC, OLD_CL], derived_metrics: { global_status: { ...OLD_GLOBAL, collectible_rewards_usd: null, collectible_rewards_note: null, total_collectible_usd: null } } }}
      />,
    );
    expect(reserialized).toBe('');
    // Still loading: nothing, never a zero.
    expect(renderToStaticMarkup(<LiteCollectibleLines payload={null} data={null} />)).toBe('');
  });

  it('a read zero is drawn as nothing to collect', () => {
    const zero = { ...CL, collectible_reward: reward('0.000000', '0.00', CL_GAUGE) };
    const html = renderToStaticMarkup(<PositionEarningsLines position={zero} feeRow={CONTRACT_FEES[2]} />);
    expect(html).toContain('Collectible earnings: 0.000000 AERO ($0.00), nothing to collect in gauge 0x3bf7…60fd');
    expect(html).not.toContain('collectible from gauge');
    expect(html).not.toContain('in the Aero app');
  });

  it('the Lite lines wrap a long reason inside the card', () => {
    // A not-read reason carries a 42-character gauge address with no space in it.
    const reason = `gauge ${CL_GAUGE} earned() read failed: network error`;
    const payload = {
      ...PAYLOAD,
      collectible_rewards_usd: null,
      total_collectible_usd: null,
      collectible_rewards_note: `gauge reward not read for ${CL.position_id} (${reason})`,
      rewards: [PAYLOAD.rewards[0], { position_id: CL.position_id, label: 'WETH+NOCK (Aero CL #2914490)', ...reward(null, null, CL_GAUGE, reason) }],
    };
    const html = renderToStaticMarkup(<LiteCollectibleLines payload={payload} data={null} />);
    expect(html).toContain('overflow-wrap:anywhere');
    expect(html).toContain('LP fees $1.95 · AERO rewards not totalled (1 gauge reward not read) · total collectible not totalled');
    expect(html).toContain(`WETH+NOCK (Aero CL #2914490): reward not read (${reason})`);
  });
});
