/**
 * Small read-only notes added with Aero concentrated-liquidity positions (v0.6.0, #91).
 *
 * - PositionStakingLine: on a position card, says when the NFT is staked in its gauge.
 * - RangeSideList: under the tick bracket chart, one line per ranged position saying
 *   whether it is in range and, when it is not, which side it left on.
 *
 * Both only restate the snapshot; the wording lives in ../aeroCl.ts and is unit-tested.
 */
import type { CSSProperties } from 'react';
import { positionLabel } from '../positions';
import { rangeSideText, rangeSideTone, stakingText } from '../aeroCl';

const TONE_COLOR: Record<'good' | 'warn' | 'stale' | 'neutral', string> = {
  good: '#10b981',
  warn: '#f59e0b',
  stale: '#fca5a5',
  neutral: '#94a3b8',
};

export function PositionStakingLine({ position, style }: { position: any; style?: CSSProperties }) {
  const text = stakingText(position ?? {});
  if (!text) return null;
  return (
    <div style={style} title={position.staked_in_gauge}>
      {text}
    </div>
  );
}

export function RangeSideList({ positions, poolMechanics }: { positions: any[]; poolMechanics?: any[] }) {
  // pool_mechanics is built one per position, in the same order.
  const rows = (positions ?? [])
    .map((p, i) => ({ p, tick: poolMechanics?.[i]?.tick_current as number | null | undefined }))
    .filter(({ p }) => p && !p.full_range);
  if (rows.length === 0) return null;
  return (
    <div style={{ marginTop: 8, fontSize: '0.75rem', fontFamily: 'monospace', display: 'grid', gap: 2 }}>
      {rows.map(({ p, tick }) => (
        <span key={p.position_id} style={{ color: TONE_COLOR[rangeSideTone(p, tick)] }}>
          {positionLabel(p)}: {rangeSideText(p, tick)}
        </span>
      ))}
    </div>
  );
}
