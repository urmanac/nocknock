/**
 * Closed positions (issue #144): a separate note beside the tracked positions.
 *
 * - ClosedPositionsCard: on the dashboard, under the tracked-position cards.
 * - ClosedPositionsLiteCard: the same facts as one card of the Lite cockpit.
 *
 * Both render nothing when the snapshot lists no closed position, so an older snapshot
 * looks exactly as it did. A closed position is never drawn as a position card: it has
 * no value, no range and no fees to show. The wording lives in ../closedPositions.ts
 * and is unit-tested. There is no button: removing a config entry is the operator's.
 */
import type { CSSProperties } from 'react';
import {
  CLOSED_NOT_COUNTED,
  closedFirstSeenText,
  closedPositionConfigHint,
  closedPositionFact,
  closedPositionName,
  closedPositionsHeader,
  closedPositionsOf,
} from '../closedPositions';

const MUTED = '#8b949e';
const TEXT = '#c9d1d9';
const NOTE = '#fcd34d';
// Every line of an entry may carry the operator's config id, which can be long and
// have no space or hyphen in it: let it break anywhere instead of leaving the card.
const WRAP: CSSProperties = { overflowWrap: 'anywhere' };

export function ClosedPositionsCard({ data }: { data: any | null | undefined }) {
  const closed = closedPositionsOf(data);
  if (closed.length === 0) return null;
  return (
    <div
      data-testid="closed-positions"
      style={{
        marginTop: 12,
        padding: '12px 14px',
        border: '1px dashed #30363d',
        borderRadius: 10,
        background: '#0d111766',
      }}
    >
      <div style={{ fontSize: '0.7rem', letterSpacing: 2, color: MUTED, marginBottom: 8 }}>
        {closedPositionsHeader(closed)} · {CLOSED_NOT_COUNTED.toUpperCase()}
      </div>
      {closed.map((c, i) => {
        const seen = closedFirstSeenText(c);
        return (
          <div key={c.position_id ?? c.config_id ?? i} style={{ fontSize: '0.78rem', lineHeight: 1.6, color: TEXT, marginBottom: 6, ...WRAP }}>
            <div style={{ fontWeight: 600 }}>{closedPositionName(c)}</div>
            <div style={{ color: NOTE }}>{closedPositionFact(c)}</div>
            <div style={{ color: MUTED }}>
              {CLOSED_NOT_COUNTED}; {closedPositionConfigHint(c)}
            </div>
            {seen && <div style={{ color: MUTED }}>{seen}</div>}
          </div>
        );
      })}
    </div>
  );
}

export function ClosedPositionsLiteCard({
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
  const closed = closedPositionsOf(data);
  if (closed.length === 0) return null;
  return (
    <section style={cardStyle} data-testid="closed-positions-lite">
      <div style={labelStyle}>{closedPositionsHeader(closed)}</div>
      {closed.map((c, i) => (
        <div key={c.position_id ?? c.config_id ?? i} style={{ marginBottom: 6, ...WRAP }}>
          <div style={{ fontSize: 13, color: '#e6edf3' }}>{closedPositionName(c)}</div>
          <div style={{ ...subStyle, color: NOTE }}>{closedPositionFact(c)}</div>
          <div style={subStyle}>
            {CLOSED_NOT_COUNTED}; {closedPositionConfigHint(c)}
          </div>
        </div>
      ))}
    </section>
  );
}
