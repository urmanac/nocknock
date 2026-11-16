/**
 * AERO, the fourth pillar (issue #93): small read-only pieces that restate the snapshot.
 *
 * - AeroPillarLines: under the pillar line, what the wallet holds, what is unclaimed in
 *   the tracked gauges (one row per position), the price, and the worker's reason for
 *   anything it could not read.
 * - PositionAeroLine: on a position card, that position's unclaimed AERO.
 * - AeroLiteCard: the same facts as one card of the Lite cockpit.
 *
 * The wording lives in ../aero.ts and is unit-tested. It states amounts only: there is
 * no target, no button and no advice about AERO anywhere here.
 */
import type { CSSProperties } from 'react';
import {
  aeroHeldLine,
  aeroNoteLine,
  aeroPositionRows,
  aeroPriceLine,
  positionAeroText,
  unclaimedAeroLine,
  unclaimedAeroText,
} from '../aero';

const AERO_COLOR = '#fdba74';
const MUTED = '#94a3b8';
const WARN = '#fcd34d';

export function AeroPillarLines({ global, positions }: { global: any; positions?: any[] }) {
  const rows = aeroPositionRows(positions);
  const note = aeroNoteLine(global);
  return (
    <div
      data-testid="aero-pillar"
      style={{ display: 'grid', gap: 2, marginTop: '0.5rem', fontSize: '0.8rem', color: AERO_COLOR }}
    >
      <span>{aeroHeldLine(global)}</span>
      <span>{unclaimedAeroLine(global, positions)}</span>
      {rows.map((r) => (
        <span key={r.positionId} style={{ paddingLeft: '1rem', color: r.read ? MUTED : WARN }}>
          {r.label}: {r.text}
        </span>
      ))}
      <span style={{ color: MUTED }}>{aeroPriceLine(global)}</span>
      {note && <span style={{ color: WARN }}>{note}</span>}
    </div>
  );
}

export function PositionAeroLine({ position, style }: { position: any; style?: CSSProperties }) {
  const text = positionAeroText(position ?? {});
  if (!text) return null;
  return <div style={style}>{text}</div>;
}

export function AeroLiteCard({
  data,
  cardStyle,
  labelStyle,
  bigStyle,
  subStyle,
}: {
  /** The latest snapshot's `data`, or null while it loads. */
  data: any | null;
  cardStyle?: CSSProperties;
  labelStyle?: CSSProperties;
  bigStyle?: CSSProperties;
  subStyle?: CSSProperties;
}) {
  const global = data?.derived_metrics?.global_status;
  const positions = data?.positions;
  const note = aeroNoteLine(global);
  return (
    <section style={cardStyle} data-testid="aero-lite">
      <div style={labelStyle}>AERO (FOURTH PILLAR)</div>
      {!data ? (
        <div style={bigStyle}>…</div>
      ) : (
        <>
          <div style={{ ...bigStyle, fontSize: 18 }}>{aeroHeldLine(global).replace('AERO held (wallet): ', '')}</div>
          <div style={subStyle}>unclaimed: {unclaimedAeroText(global, positions)}</div>
          {aeroPositionRows(positions).map((r) => (
            <div key={r.positionId} style={subStyle}>
              {r.label}: {r.text}
            </div>
          ))}
          <div style={subStyle}>{aeroPriceLine(global)}</div>
          {note && <div style={{ ...subStyle, color: WARN }}>{note}</div>}
        </>
      )}
    </section>
  );
}
