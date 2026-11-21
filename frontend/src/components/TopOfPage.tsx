/**
 * The top of the page (issue #169): every `top`-tier call to action the worker lists in
 * `derived_metrics.events`, one card each, in the worker's order. Maintenance the operator
 * signed up for (the ladder's rules, a due tranche, a plan buy to confirm) and floor and
 * reserve breaches. Quiet entries are not here: they stay where they are shown today.
 *
 * - TopOfPageEvents: the dashboard, right under the title row.
 * - TopOfPageLiteEvents: the Lite cockpit, above the directive.
 *
 * Both render nothing when no top entry stands, or when the snapshot predates 2.11.0: no
 * "all clear" banner that could read as a reading. No input and no button that signs:
 * each card says what the operator would sign, and links to the venue's own page.
 */
import type { CSSProperties } from 'react';
import type { SyncStatus } from '../hooks/useFreshness';
import { anchorId, eventsOf, sinceText, staleNote, tierTone, topOfPage, type SnapshotEvent } from '../events';

const MUTED = '#8b949e';
const TEXT = '#c9d1d9';
const TONE: Record<'alarm' | 'maintenance', CSSProperties> = {
  alarm: { border: '1px solid #dc2626', background: '#450a0a33' },
  maintenance: { border: '1px solid #d97706', background: '#78350f22' },
};

function EventBody({ e, compact }: { e: SnapshotEvent; compact?: boolean }) {
  const small: CSSProperties = { color: MUTED, fontSize: compact ? 11 : '0.72rem' };
  return (
    <>
      <div style={{ display: 'flex', gap: 8, alignItems: 'baseline', flexWrap: 'wrap' }}>
        <span data-testid="event-title" style={{ fontWeight: 700, color: '#f0f6fc', fontSize: compact ? 13 : '0.85rem', letterSpacing: 1 }}>
          {e.title}
        </span>
        <span style={small}>{sinceText(e.since)}</span>
      </div>
      {e.text ? <div style={{ color: TEXT, marginTop: 4, overflowWrap: 'anywhere' }}>{e.text}</div> : null}
      {e.would_sign ? (
        <div style={{ ...small, marginTop: 4 }}>
          <strong style={{ color: TEXT }}>Would sign:</strong> {e.would_sign}
        </div>
      ) : null}
      {e.links.length > 0 ? (
        <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap', marginTop: 4 }}>
          {e.links.map((l) => (
            <a key={l.url} href={l.url} target="_blank" rel="noreferrer" style={{ color: '#38bdf8', fontSize: compact ? 12 : '0.75rem' }}>
              {l.label}
            </a>
          ))}
        </div>
      ) : null}
    </>
  );
}

export function TopOfPageEvents({ data, syncStatus }: { data: any | null | undefined; syncStatus?: SyncStatus }) {
  const top = topOfPage(eventsOf(data));
  if (top.length === 0) return null;
  const stale = staleNote(syncStatus);
  return (
    <div data-testid="top-of-page" style={{ display: 'grid', gap: 8, margin: '12px 0', fontSize: '0.8rem', lineHeight: 1.5 }}>
      {stale ? <div style={{ color: '#f59e0b', fontSize: '0.72rem' }}>{stale}</div> : null}
      {top.map((e) => (
        <section key={e.id} id={anchorId(e)} data-tier="top" style={{ borderRadius: 10, padding: '10px 14px', ...TONE[tierTone(e)] }}>
          <EventBody e={e} />
        </section>
      ))}
    </div>
  );
}

export function TopOfPageLiteEvents({
  data,
  syncStatus,
  cardStyle,
  labelStyle,
}: {
  data: any | null | undefined;
  syncStatus?: SyncStatus;
  cardStyle?: CSSProperties;
  labelStyle?: CSSProperties;
}) {
  const top = topOfPage(eventsOf(data));
  if (top.length === 0) return null;
  const stale = staleNote(syncStatus);
  return (
    <>
      {top.map((e) => (
        <section key={e.id} id={anchorId(e)} data-tier="top" style={{ ...cardStyle, borderLeft: `3px solid ${tierTone(e) === 'alarm' ? '#dc2626' : '#d97706'}`, fontSize: 12, lineHeight: 1.6 }}>
          <div style={labelStyle}>TOP OF PAGE</div>
          {stale ? <div style={{ color: '#f59e0b', fontSize: 11, marginBottom: 4 }}>{stale}</div> : null}
          <EventBody e={e} compact />
        </section>
      ))}
    </>
  );
}
