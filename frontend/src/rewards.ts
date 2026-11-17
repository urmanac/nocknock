/**
 * Pure display helpers for gauge rewards reported as collectible earnings (issue #142).
 *
 * A staked Aero position's earnings are its gauge reward. The worker states it as a
 * structured object (`positions[].collectible_reward`, and `reward` on the fee rows):
 * the token, the amount, its USD value, the gauge it is collected from and how. These
 * helpers restate that object and the three totals beside it. Nothing here computes a
 * number.
 *
 * Rules the wording keeps:
 * - Facts only. "Collectible" states an amount and where it is collected. No helper says
 *   to claim, sell, hold or accumulate AERO, and a reward never sits under one of the
 *   LP-fee harvest words (CLAIM_NOW, CLAIM_RECOMMENDED, ACCUMULATE).
 * - LP fees and rewards are two labelled figures and a labelled total, never one merged
 *   number. `total_fees_claimable_usd` stays LP fees only.
 * - A snapshot without the fields (an older one) reads "not reported". A reward the
 *   worker could not read reads "not read" with its reason, and a total it could not
 *   form reads "not totalled" with its reason. None of them reads as 0.
 * - Nothing here is in the portfolio total.
 */

import { positionFeesText, shortAddress } from './aeroCl.ts';
import { velocityRowText } from './positions.ts';

export const REWARD_NOT_REPORTED = 'not reported';
/** The reward series' colour (AERO orange), apart from the LP-fee green. */
export const REWARD_COLOR = '#fdba74';
export const LP_FEE_COLOR = '#10b981';
/** Legend entries of the velocity chart's two earnings series. */
export const LP_FEE_SERIES_NAME = 'Claimable LP Fee Yield ($)';
export const REWARD_SERIES_NAME = 'Collectible AERO Reward in Gauge ($)';
/** `earnings_kind` of a row whose reported earnings are the gauge reward. */
export const EARNINGS_KIND_GAUGE_REWARD = 'gauge_reward';

/** A snapshot string field, or null when it is absent, null or blank. */
function field(obj: any, key: string): string | null {
  const v = obj?.[key];
  if (v === undefined || v === null) return null;
  const s = String(v).trim();
  return s === '' ? null : s;
}

export interface RewardView {
  /** True when the gauge answered (an amount, possibly 0). */
  read: boolean;
  symbol: string;
  amount: string | null;
  usd: string | null;
  gauge: string | null;
  /** "the Aero app" */
  collectedIn: string;
  /** Why the reward was not read; null when it was. */
  note: string | null;
}

/**
 * The structured reward of a position (`collectible_reward`), a fee row (`reward`) or a
 * claim-payload reward row (the row itself). Null when the snapshot carries none: an
 * older snapshot, or a position that cannot earn a reward.
 */
export function rewardOf(obj: any): RewardView | null {
  const r = obj?.collectible_reward ?? obj?.reward ?? (obj?.collected_in !== undefined ? obj : null);
  if (!r || typeof r !== 'object') return null;
  const amount = field(r, 'amount');
  return {
    read: amount !== null,
    symbol: field(r, 'token_symbol') ?? 'AERO',
    amount,
    usd: amount === null ? null : field(r, 'usd'),
    gauge: field(r, 'gauge'),
    collectedIn: field(r, 'collected_in') ?? 'the Aero app',
    note: field(r, 'note'),
  };
}

/** "17.654015 AERO ($7.75)"; null when the reward was not read. */
export function rewardAmountText(r: RewardView): string | null {
  if (r.amount === null) return null;
  return r.usd === null ? `${r.amount} ${r.symbol} (USD value not reported)` : `${r.amount} ${r.symbol} ($${r.usd})`;
}

/** "collectible from gauge 0x3bf7…60fd in the Aero app". */
export function rewardWhereText(r: RewardView): string {
  return r.gauge === null
    ? `collectible in ${r.collectedIn}`
    : `collectible from gauge ${shortAddress(r.gauge)} in ${r.collectedIn}`;
}

/** The reward as one statement of fact, or that it was not read and why. */
export function rewardStatement(r: RewardView): string {
  const amount = rewardAmountText(r);
  return amount === null ? `not read (${r.note ?? 'no reason recorded'})` : `${amount}, ${rewardWhereText(r)}`;
}

/**
 * A position card's two earnings lines.
 * - `earnings`: the gauge reward as the position's collectible earnings; null when the
 *   snapshot has no structured reward (the card then shows what it showed before).
 * - `lpFees`: the LP-fee figure, or the reason there is none. The reason comes from the
 *   position (`fees_note`, an Aero CL position) or from its fee row (`lp_fees_note`, the
 *   Aero basic pool whose claimFees() reverts).
 */
export function positionEarningsLines(
  p: any,
  feeRow?: any,
): { earnings: string | null; lpFees: string } {
  const reward = rewardOf(p);
  if (reward === null) return { earnings: null, lpFees: positionFeesText(p ?? {}) };
  const reason = field(p, 'fees_note') ?? field(feeRow, 'lp_fees_note');
  const lpFees = reason !== null ? `LP fees: ${reason}` : positionFeesText(p).replace(/^Fees:/, 'LP fees:');
  const earnings = reward.read
    ? `Collectible earnings: ${rewardStatement(reward)}; not counted in the total`
    : `Collectible earnings: reward ${rewardStatement(reward)}`;
  return { earnings, lpFees };
}

/** How many positions carry a structured reward (read or not). */
function rewardPositions(positions: any[] | undefined | null): number {
  return (positions ?? []).filter((p) => rewardOf(p) !== null).length;
}

export interface CollectibleTotals {
  /** "$1.95", or "unavailable". LP fees only. */
  lpFees: string;
  /** "$7.84", "none (no staked Aero position is tracked)", "not totalled (reason)" or "not reported". */
  rewards: string;
  /** "$9.79", "not totalled" or "not reported". */
  total: string;
  /** True when the worker stated a total: all three are then figures. */
  totalled: boolean;
  /** True when the snapshot carries the fields at all (false for an older snapshot). */
  reported: boolean;
}

/**
 * The three collectible figures from `global_status`, in words. `source` may also be a
 * claim payload, which carries the same three under `collectible_lp_fees_usd`,
 * `collectible_rewards_usd` and `total_collectible_usd`.
 */
export function collectibleTotals(source: any, positions?: any[] | null): CollectibleTotals {
  const lp = field(source, 'collectible_lp_fees_usd') ?? field(source, 'total_fees_claimable_usd');
  const rewardsUsd = field(source, 'collectible_rewards_usd');
  const totalUsd = field(source, 'total_collectible_usd');
  const note = field(source, 'collectible_rewards_note');
  const lpFees = lp === null ? 'unavailable' : `$${lp}`;
  if (rewardsUsd !== null) {
    const none = positions !== undefined && positions !== null && rewardPositions(positions) === 0;
    return {
      lpFees,
      rewards: none ? 'none (no staked Aero position is tracked)' : `$${rewardsUsd}`,
      total: totalUsd === null ? REWARD_NOT_REPORTED : `$${totalUsd}`,
      totalled: totalUsd !== null,
      reported: true,
    };
  }
  if (note !== null) {
    return { lpFees, rewards: `not totalled (${note})`, total: 'not totalled', totalled: false, reported: true };
  }
  return { lpFees, rewards: REWARD_NOT_REPORTED, total: REWARD_NOT_REPORTED, totalled: false, reported: false };
}

/** "LP fees $1.95 · AERO rewards $7.84 · total collectible $9.79". */
export function collectibleTotalsLine(source: any, positions?: any[] | null): string {
  const t = collectibleTotals(source, positions);
  return `LP fees ${t.lpFees} · AERO rewards ${t.rewards} · total collectible ${t.total}`;
}

/** "$1.94", "-$0.01", or null when the figure is absent. */
function signedUsd(value: string | null): string | null {
  if (value === null) return null;
  return value.startsWith('-') ? `-$${value.slice(1)}` : `$${value}`;
}

export interface ClaimableYieldTile {
  label: string;
  /** The figure the tile shows large; undefined renders as "—". */
  value: string | undefined;
  subtitle: string;
}

/**
 * The CLAIMABLE YIELD stat tile.
 * - The worker stated a total: the tile shows it, and the subtitle gives LP fees and
 *   AERO rewards as two labelled figures, then the LP-fee net of gas (the figure the
 *   tile used to show).
 * - The rewards could not be totalled: the tile keeps the LP-fee net and says why
 *   there is no total. Never a partial sum.
 * - An older snapshot: exactly the tile it was, with "AERO rewards not reported".
 */
export function claimableYieldTile(global: any, positions?: any[] | null): ClaimableYieldTile {
  const t = collectibleTotals(global, positions);
  const net = signedUsd(field(global, 'gas_to_fee_delta_usd'));
  const netText = net === null ? 'unavailable' : net;
  if (t.totalled) {
    return {
      label: 'Claimable Yield (LP fees + AERO rewards)',
      value: field(global, 'total_collectible_usd') ?? undefined,
      subtitle: `LP fees ${t.lpFees} · AERO rewards ${t.rewards}${t.rewards.startsWith('$') ? ' in gauges' : ''} · neither is counted in the portfolio total · LP fees net of two claims' gas ${netText}`,
    };
  }
  if (t.reported) {
    return {
      label: 'Claimable Yield (LP fees, net)',
      value: field(global, 'gas_to_fee_delta_usd') ?? undefined,
      subtitle: `LP fees ${t.lpFees} gross; net of two claims' gas · AERO rewards ${t.rewards} · total collectible ${t.total}`,
    };
  }
  return {
    label: 'Claimable Yield (Net)',
    value: field(global, 'gas_to_fee_delta_usd') ?? undefined,
    subtitle: `Gross: ${t.lpFees === 'unavailable' ? '—' : t.lpFees}; net of two claims' gas · AERO rewards ${REWARD_NOT_REPORTED}`,
  };
}

export interface VelocityRow {
  name: string;
  /** LP fees in USD: the green series. */
  claimable: number;
  /** The gauge reward in USD: its own series. Null when there is none, it was not
   *  read, or it is unpriced (the row's text still states the amount). */
  reward: number | null;
  gasBreakEven: number;
  ratio: number;
  action: string;
  note?: string | null;
  /** 'gauge_reward' when the row's earnings are the position's gauge reward. */
  kind: 'lp_fees' | 'gauge_reward';
  rewardView: RewardView | null;
  /** The reason alone why the LP fees are not LP-claimable. */
  lpFeesNote: string | null;
}

/** The velocity panel's rows from `velocity_gas_sandbox.contracts`. */
export function velocityRows(contracts: any[] | undefined | null): VelocityRow[] {
  return (contracts ?? []).map((c: any) => {
    const isReward = c?.earnings_kind === EARNINGS_KIND_GAUGE_REWARD;
    const rewardView = isReward ? rewardOf(c) : null;
    const usd = rewardView?.usd ?? null;
    const usdNumber = usd === null ? NaN : Number(usd);
    return {
      name: c.contract_name,
      claimable: parseFloat(c.claimable_usd),
      reward: Number.isFinite(usdNumber) ? usdNumber : null,
      gasBreakEven: parseFloat(c.gas_break_even_usd),
      ratio: c.efficiency_ratio,
      action: c.recommended_action,
      note: c.note,
      kind: isReward && rewardView !== null ? 'gauge_reward' : 'lp_fees',
      rewardView,
      lpFeesNote: field(c, 'lp_fees_note'),
    };
  });
}

/** True when some row reports a gauge reward: the chart then draws the reward series. */
export function velocityHasRewards(rows: VelocityRow[]): boolean {
  return rows.some((r) => r.kind === 'gauge_reward');
}

/**
 * A velocity row in words. A gauge-reward row states the reward first, then the reason
 * its LP fees are not LP-claimable; it carries no action word. Every other row reads
 * exactly as it did (positions.ts velocityRowText).
 */
export function velocityRewardRowText(g: VelocityRow): string {
  if (g.kind !== 'gauge_reward' || g.rewardView === null) return velocityRowText(g);
  const r = g.rewardView;
  const amount = rewardAmountText(r);
  const reward =
    amount === null
      ? `reward not read (${r.note ?? 'no reason recorded'})`
      : `reward collectible in gauge${r.gauge === null ? '' : ` ${shortAddress(r.gauge)}`}: ${amount}`;
  const reason = g.lpFeesNote ?? 'not LP-claimable';
  return `${g.name}: ${reward}; LP fees ${reason}`;
}

/** The colour of a velocity text row: the reward colour for a gauge-reward row. */
export function velocityRowColor(g: VelocityRow): string {
  if (g.kind === 'gauge_reward') return REWARD_COLOR;
  return g.action === 'claim_now' ? LP_FEE_COLOR : '#f59e0b';
}

export interface LiteRewardLine {
  key: string;
  text: string;
  /** False for a reward that was not read (drawn as a warning, never as 0). */
  read: boolean;
}

/**
 * The Lite cockpit's reward lines. From the claim payload's `rewards` when the worker
 * sends them (they carry the gas statement); otherwise from the snapshot's positions.
 * An older worker and an older snapshot give no lines at all.
 */
export function liteRewardLines(payload: any, snapshotData: any): LiteRewardLine[] {
  const rows: any[] | undefined = Array.isArray(payload?.rewards) ? payload.rewards : undefined;
  if (rows !== undefined) {
    return rows.flatMap((row: any, i: number) => {
      const r = rewardOf(row);
      if (r === null) return [];
      const gas = field(row, 'gas_note');
      const label = field(row, 'label') ?? 'Position';
      const text = r.read
        ? `${label}: ${rewardStatement(r)}${gas === null ? '' : ` · ${gas}`}`
        : `${label}: reward ${rewardStatement(r)}`;
      return [{ key: String(row.position_id ?? i), text, read: r.read }];
    });
  }
  return (snapshotData?.positions ?? []).flatMap((p: any, i: number) => {
    const r = rewardOf(p);
    if (r === null) return [];
    const fee = snapshotData?.contract_fees?.[i];
    const label = field(fee, 'fee_token_symbol') ?? `${p?.token0?.symbol ?? '?'}/${p?.token1?.symbol ?? '?'}`;
    const text = r.read ? `${label}: ${rewardStatement(r)}` : `${label}: reward ${rewardStatement(r)}`;
    return [{ key: String(p.position_id ?? i), text, read: r.read }];
  });
}

/**
 * The split totals for the Lite cockpit: the claim payload's own when it carries them,
 * otherwise the snapshot's.
 */
export function liteCollectibleTotalsLine(payload: any, snapshotData: any): string {
  const fromPayload = payload && (payload.collectible_rewards_usd !== undefined || payload.total_collectible_usd !== undefined);
  const source = fromPayload ? payload : snapshotData?.derived_metrics?.global_status;
  return collectibleTotalsLine(source, fromPayload ? (payload.rewards ?? null) : (snapshotData?.positions ?? null));
}

/**
 * The LP-fee reason of a `not_claimable` claim action. Since #142 the worker appends
 * the reward to the detail ("…; reward 17.654015 AERO ($7.75), collectible from …");
 * the Lite cockpit lists rewards on their own lines, so only the reason is repeated.
 */
export function lpFeeReasonOfDetail(detail: string | null | undefined): string {
  const d = (detail ?? '').trim();
  if (d === '') return 'not LP-claimable';
  return d.split('; reward ')[0];
}
