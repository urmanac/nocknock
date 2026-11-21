/**
 * Tracked ladders (issue #167): a quiet card beside the tracked positions.
 *
 * - LaddersCard: on the dashboard, under the tracked-position cards.
 * - LadderLiteCard: one muted line per ladder in the Lite cockpit.
 *
 * Both render nothing when the snapshot lists no ladder, so every other snapshot looks
 * exactly as it did. Facts only, no button: a ladder is measured, never steered, and its
 * maintenance (#169) is not here. The wording lives in ../ladderTrack.ts and is unit-tested.
 */
import type { CSSProperties } from 'react';
import {
  LADDERS_COUNTED,
  LADDERS_TITLE,
  ladderCurveLine,
  ladderHoldsLine,
  ladderLiteLine,
  ladderRangeLine,
  ladderRungText,
  ladderSinceDeposit,
  ladderStatus,
  laddersOf,
} from '../ladderTrack';

const MUTED = '#8b949e';
const TEXT = '#c9d1d9';
const WRAP: CSSProperties = { overflowWrap: 'anywhere' };
const TONE: Record<string, CSSProperties> = {
  good: { background: '#064e3b', color: '#a7f3d0', border: '1px solid #059669' },
  neutral: { background: '#1f2937', color: '#cbd5e1', border: '1px solid #475569' },
  stale: { background: '#3f1d1d', color: '#fca5a5', border: '1px solid #b91c1c' },
};

export function LaddersCard({ data }: { data: any | null | undefined }) {
  const ladders = laddersOf(data);
  if (ladders.length === 0) return null;
  return (
    <div
      data-testid="ladders"
      style={{ marginTop: 12, padding: '12px 14px', border: '1px solid #30363d', borderRadius: 10, background: '#0d111766' }}
    >
      <div style={{ fontSize: '0.7rem', letterSpacing: 2, color: MUTED, marginBottom: 4 }}>
        {LADDERS_TITLE.toUpperCase()} ({ladders.length})
      </div>
      <div style={{ fontSize: '0.72rem', color: MUTED, marginBottom: 8 }}>{LADDERS_COUNTED}</div>
      {ladders.map((r) => {
        const status = ladderStatus(r);
        const since = ladderSinceDeposit(r);
        return (
          <div key={r.position_id} style={{ fontSize: '0.78rem', lineHeight: 1.6, color: TEXT, marginBottom: 10, ...WRAP }}>
            <div style={{ display: 'flex', gap: 8, alignItems: 'center', flexWrap: 'wrap' }}>
              <span style={{ fontWeight: 600 }}>{r.label}</span>
              <span style={{ color: MUTED }}>{ladderRungText(r)}</span>
              <span style={{ fontSize: '0.68rem', padding: '1px 6px', borderRadius: 4, ...TONE[status.tone] }} title={r.state_text}>
                {status.label}
              </span>
            </div>
            <div>{ladderRangeLine(r)}</div>
            <div>{ladderHoldsLine(r)}</div>
            <div style={{ color: MUTED }}>{ladderCurveLine(r)}</div>
            <div style={since.verified ? undefined : { color: MUTED }}>{since.text}</div>
            <div style={{ color: MUTED }}>Uncollected fees: ${r.fees_usd}</div>
          </div>
        );
      })}
    </div>
  );
}

export function LadderLiteCard({
  data,
  cardStyle,
  labelStyle,
  subStyle,
}: {
  /** The latest snapshot's `data`, or null while it loads. */
  data: any | null;
  cardStyle?: CSSProperties;
  labelStyle?: CSSProperties;
  subStyle?: CSSProperties;
}) {
  const ladders = laddersOf(data);
  if (ladders.length === 0) return null;
  return (
    <section style={cardStyle} data-testid="ladders-lite">
      <div style={labelStyle}>{LADDERS_TITLE}</div>
      {ladders.map((r) => (
        <div key={r.position_id} style={{ ...subStyle, marginBottom: 4, ...WRAP }}>
          {ladderLiteLine(r)}
        </div>
      ))}
    </section>
  );
}
