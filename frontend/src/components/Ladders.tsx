/**
 * Tracked ladders (issue #167): a quiet card beside the tracked positions.
 *
 * - LaddersCard: on the dashboard, under the tracked-position cards.
 * - LadderLiteCard: one muted line per ladder in the Lite cockpit.
 *
 * Both render nothing when the snapshot lists no ladder, so every other snapshot looks
 * exactly as it did. Facts only, no button: a ladder is measured, never steered.
 *
 * Each rung is one compact summary row (#169, operator 2026-11-21: the rows "have too much
 * text for their width, they leak out"): its label, rung, a short badge, what it holds in
 * plain numbers and its range at 4 significant figures. The rest (against its top, since its
 * deposit, fees, the maintenance line) is behind a per-row <details>. Every flex row wraps,
 * every flex child may shrink (min-width: 0) and long text breaks anywhere, so nothing can
 * leak out of the card. The wording lives in ../ladderTrack.ts and ../events.ts.
 */
import type { CSSProperties } from 'react';
import {
  LADDERS_COUNTED,
  LADDERS_TITLE,
  ladderBadge,
  ladderCurveLine,
  ladderHoldsLine,
  ladderLiteLine,
  ladderRangeLine,
  ladderRungText,
  ladderSinceDeposit,
  ladderStatus,
  ladderSummary,
  laddersOf,
} from '../ladderTrack';
import { maintenanceFor, maintenanceLine, maintenanceOf } from '../events';

const MUTED = '#8b949e';
const TEXT = '#c9d1d9';
/** Long text breaks anywhere and never sets the card's width. */
export const WRAP: CSSProperties = { overflowWrap: 'anywhere', wordBreak: 'break-word', minWidth: 0, maxWidth: '100%' };
/** A flex row that wraps; its children may shrink below their content. */
export const ROW: CSSProperties = { display: 'flex', gap: 8, alignItems: 'center', flexWrap: 'wrap', minWidth: 0 };
const CHILD: CSSProperties = { minWidth: 0, ...WRAP };
const TONE: Record<string, CSSProperties> = {
  good: { background: '#064e3b', color: '#a7f3d0', border: '1px solid #059669' },
  neutral: { background: '#1f2937', color: '#cbd5e1', border: '1px solid #475569' },
  stale: { background: '#3f1d1d', color: '#fca5a5', border: '1px solid #b91c1c' },
};

export function LaddersCard({ data }: { data: any | null | undefined }) {
  const ladders = laddersOf(data);
  const maintenance = maintenanceOf(data);
  if (ladders.length === 0) return null;
  return (
    <div
      data-testid="ladders"
      style={{ marginTop: 12, padding: '12px 14px', border: '1px solid #30363d', borderRadius: 10, background: '#0d111766', boxSizing: 'border-box', ...WRAP }}
    >
      <div style={{ fontSize: '0.7rem', letterSpacing: 2, color: MUTED, marginBottom: 4 }}>
        {LADDERS_TITLE.toUpperCase()} ({ladders.length})
      </div>
      <div style={{ fontSize: '0.72rem', color: MUTED, marginBottom: 8, ...WRAP }}>{LADDERS_COUNTED}</div>
      {ladders.map((r) => {
        const status = ladderStatus(r);
        const since = ladderSinceDeposit(r);
        const m = maintenanceFor(maintenance, r.position_id);
        return (
          <div key={r.position_id} data-testid="ladder-row" style={{ fontSize: '0.78rem', lineHeight: 1.6, color: TEXT, marginBottom: 8, ...WRAP }}>
            <div style={ROW}>
              <span style={{ fontWeight: 600, ...CHILD }}>{r.label}</span>
              <span style={{ color: MUTED, ...CHILD }}>{ladderRungText(r)}</span>
              <span style={{ fontSize: '0.68rem', padding: '1px 6px', borderRadius: 4, ...TONE[status.tone], ...CHILD }} title={`${status.label}: ${r.state_text}`}>
                {ladderBadge(r.state)}
              </span>
              <span data-testid="ladder-summary" style={CHILD}>{ladderSummary(r)}</span>
            </div>
            <details style={WRAP}>
              <summary style={{ color: MUTED, cursor: 'pointer', fontSize: '0.72rem' }}>details</summary>
              <div style={WRAP}>{ladderRangeLine(r)}</div>
              <div style={WRAP}>{ladderHoldsLine(r)}</div>
              <div style={{ color: MUTED, ...WRAP }}>{ladderCurveLine(r)}</div>
              <div style={{ ...(since.verified ? {} : { color: MUTED }), ...WRAP }}>{since.text}</div>
              <div style={{ color: MUTED }}>Uncollected fees: ${r.fees_usd}</div>
              {m ? (
                <div data-testid="ladder-maintenance" style={{ color: m.status === 'suggested' ? '#fbbf24' : MUTED, ...WRAP }}>
                  {maintenanceLine(m)}
                </div>
              ) : null}
            </details>
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
  const maintenance = maintenanceOf(data);
  if (ladders.length === 0) return null;
  return (
    <section style={{ ...cardStyle, ...WRAP }} data-testid="ladders-lite">
      <div style={labelStyle}>{LADDERS_TITLE}</div>
      {ladders.map((r) => {
        const m = maintenanceFor(maintenance, r.position_id);
        return (
          <div key={r.position_id} style={{ ...subStyle, marginBottom: 4, ...WRAP }}>
            {ladderLiteLine(r)}
            {m ? (
              <details style={WRAP}>
                <summary style={{ cursor: 'pointer' }}>maintenance</summary>
                <div data-testid="ladder-maintenance-lite" style={WRAP}>{maintenanceLine(m)}</div>
              </details>
            ) : null}
          </div>
        );
      })}
    </section>
  );
}
