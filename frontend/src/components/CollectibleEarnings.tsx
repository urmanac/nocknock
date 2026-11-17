/**
 * Gauge rewards as collectible earnings (issue #142): small read-only pieces that
 * restate the snapshot and the claim payload.
 *
 * - PositionEarningsLines: on a position card, the reward as the position's collectible
 *   earnings (amount, USD, gauge) with the LP-fee figure or reason beneath.
 * - LiteCollectibleLines: in the Lite cockpit's uncollected-fees card, the reward lines
 *   and the split totals (LP fees, AERO rewards, total collectible). Nothing at all for
 *   a worker and snapshot that state no rewards.
 *
 * The wording lives in ../rewards.ts and is unit-tested. It states amounts and where
 * they are collected. There is no button and no link here: NockNock builds no
 * transaction for a reward and gives no advice about AERO.
 */
import type { CSSProperties } from 'react';
import {
  REWARD_COLOR,
  liteCollectibleTotalsLine,
  liteRewardLines,
  positionEarningsLines,
  rewardOf,
} from '../rewards';

const WARN = '#fcd34d';

/**
 * The earnings part of a position card. For a position with a structured reward: the
 * reward line, then the LP-fee line. For any other position (and any older snapshot):
 * nothing, and the card keeps the fee text it always had.
 */
export function PositionEarningsLines({
  position,
  feeRow,
  style,
}: {
  position: any;
  /** The position's row of `contract_fees` (same index), for the LP-fee reason. */
  feeRow?: any;
  style?: CSSProperties;
}) {
  const lines = positionEarningsLines(position ?? {}, feeRow);
  if (lines.earnings === null) return null;
  const read = rewardOf(position)?.read ?? false;
  return (
    <>
      <div
        data-testid="position-earnings"
        style={{ ...style, color: read ? REWARD_COLOR : WARN }}
        title={position?.collectible_reward?.gauge ?? undefined}
      >
        {lines.earnings}
      </div>
      <div style={style}>{lines.lpFees}</div>
    </>
  );
}

/** True when the position's card should draw PositionEarningsLines in place of its fee text. */
export function hasPositionEarnings(position: any): boolean {
  return rewardOf(position) !== null;
}

export function LiteCollectibleLines({
  payload,
  data,
  subStyle,
}: {
  /** The claim payload, or null while it loads. */
  payload: any | null;
  /** The latest snapshot's `data`, or null while it loads. */
  data: any | null;
  subStyle?: CSSProperties;
}) {
  const lines = liteRewardLines(payload, data);
  const totals = liteCollectibleTotalsLine(payload, data);
  // An older worker and snapshot state no rewards: nothing is added to the card.
  if (totals === null && lines.length === 0) return null;
  return (
    // A reason can carry an address or a position id with no space in it: it wraps
    // inside the card and never widens the phone page.
    <div data-testid="lite-collectible" style={{ overflowWrap: 'anywhere' }}>
      {totals !== null && <div style={subStyle}>{totals}</div>}
      {lines.map((l) => (
        <div key={l.key} style={{ ...subStyle, color: l.read ? REWARD_COLOR : WARN }}>
          {l.text}
        </div>
      ))}
    </div>
  );
}
