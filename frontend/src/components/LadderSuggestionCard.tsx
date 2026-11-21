/**
 * The ladder suggestion card (issue #168): the dashboard's top-of-page ladder surface and
 * its phone cockpit twin. No input of any kind: the worker measures everything, and the
 * card reads it top to bottom as a headline, a picture on a price axis, the outcomes in
 * plain words, and the numbered deposits with their links. Each deposit leaves a slot
 * (`rungAction`) for the follow-up's sign-in-wallet button.
 *
 * Wording lives in ../ladderSuggestion.ts and is unit-tested.
 */
import type { CSSProperties, ReactNode } from 'react';
import type { SyncStatus } from '../hooks/useFreshness';
import {
  LADDER_TITLE,
  type LadderPictureData,
  type LadderRung,
  type LadderSuggestion,
  candidateLines,
  ladderCardView,
  ladderDepositUrl,
  ladderLinkLabel,
  missingSettingSentence,
  pictureGeometry,
  rungRangeLines,
  rungTitle,
} from '../ladderSuggestion';

const MUTED = '#8b949e';
const TEXT = '#c9d1d9';
const GREEN = '#10b981';
const WRAP: CSSProperties = { overflowWrap: 'anywhere' };

/** The ladder on a USD-per-NOCK axis: spot, each rung's bar with its amount, the bottom
 *  line and the protected reserve's zone below it. */
export function LadderPicture({ picture }: { picture: LadderPictureData }) {
  const g = pictureGeometry(picture);
  if (!g) return null;
  const axisY = g.height - 22;
  return (
    <svg
      data-testid="ladder-picture"
      viewBox={`0 0 ${g.width} ${g.height}`}
      width="100%"
      role="img"
      aria-label="The ladder on a price axis"
      style={{ maxWidth: g.width, display: 'block', margin: '8px 0' }}
    >
      {/* The reserve zone: below the ladder bottom. */}
      <rect data-testid="ladder-reserve-zone" x={8} y={14} width={Math.max(0, g.bottomX - 8)} height={axisY - 14} fill="#1e3a8a" fillOpacity={0.35} />
      <text x={12} y={axisY - 6} fontSize={10} fill="#93c5fd">{picture.reserve_label}</text>
      {/* The ladder bottom. */}
      <line data-testid="ladder-bottom-line" x1={g.bottomX} x2={g.bottomX} y1={10} y2={axisY} stroke="#93c5fd" strokeDasharray="4 3" />
      <text x={g.bottomX + 3} y={12} fontSize={10} fill="#93c5fd">bottom ${picture.bottom_usd}</text>
      {/* Each rung. */}
      {g.bars.map((b) => (
        <g key={b.label + b.x} data-testid="ladder-bar">
          <rect x={b.x} y={b.y} width={b.w} height={18} rx={3} fill={GREEN} fillOpacity={0.6} stroke={GREEN} />
          <text x={b.x + b.w / 2} y={b.y + 13} fontSize={11} fill="#ecfdf5" textAnchor="middle">{b.label}</text>
        </g>
      ))}
      {/* Spot. */}
      {g.spotX != null && (
        <g data-testid="ladder-spot-marker">
          <line x1={g.spotX} x2={g.spotX} y1={10} y2={axisY} stroke="#f59e0b" strokeWidth={2} />
          <text x={g.spotX - 3} y={24} fontSize={10} fill="#f59e0b" textAnchor="end">spot ${picture.spot_usd}</text>
        </g>
      )}
      <line x1={8} x2={g.width - 8} y1={axisY} y2={axisY} stroke="#30363d" />
      <text x={8} y={g.height - 6} fontSize={10} fill={MUTED}>${g.min.toFixed(5)}</text>
      <text x={g.width - 8} y={g.height - 6} fontSize={10} fill={MUTED} textAnchor="end">${g.max.toFixed(5)} per NOCK</text>
    </svg>
  );
}

function RungBlock({ r, action, compact }: { r: LadderRung; action?: ReactNode; compact?: boolean }) {
  const url = ladderDepositUrl(r.deep_link?.url);
  return (
    <li data-testid="ladder-rung" style={{ marginBottom: 10, ...WRAP }}>
      <div style={{ fontWeight: 600, color: '#e6edf3' }}>{rungTitle(r)}</div>
      {rungRangeLines(r).map((line) => (
        <div key={line} style={{ color: compact ? MUTED : TEXT }}>{line}</div>
      ))}
      <div style={{ display: 'flex', gap: 10, alignItems: 'center', flexWrap: 'wrap', marginTop: 4 }}>
        {url ? (
          <a href={url} target="_blank" rel="noopener noreferrer" style={{ color: '#38bdf8', fontSize: '0.72rem', fontWeight: 600, letterSpacing: 0.5 }}>
            {ladderLinkLabel(r)}
          </a>
        ) : (
          <span style={{ color: MUTED }}>No link: {r.deep_link?.reason ?? 'not built'}</span>
        )}
        {/* The per-deposit action slot (sign in the paired wallet, a follow-up). */}
        <span data-testid="ladder-rung-action">{action}</span>
      </div>
      {!compact && <div style={{ color: MUTED, fontSize: '0.7rem' }}>{r.would_sign}</div>}
    </li>
  );
}

function Body({
  s,
  syncStatus,
  compact,
  rungAction,
}: {
  s: LadderSuggestion;
  syncStatus?: SyncStatus;
  compact?: boolean;
  rungAction?: (r: LadderRung) => ReactNode;
}) {
  const view = ladderCardView(s, syncStatus);
  if (view.mode !== 'ready') {
    const missing = missingSettingSentence(s);
    const candidates = candidateLines(s);
    return (
      <>
        <div data-testid="ladder-headline" style={{ fontSize: compact ? 14 : '0.95rem', fontWeight: 700, color: '#e6edf3' }}>{view.headline}</div>
        {missing ? <div style={{ color: TEXT }}>{missing}</div> : view.line ? <div style={{ color: MUTED }}>{view.line}</div> : null}
        {view.mode === 'line' && candidates.length > 0 && (
          <>
            <div style={{ color: MUTED, marginTop: 6 }}>Candidate bottoms (a measurement; none is picked):</div>
            <ul data-testid="ladder-candidates" style={{ margin: '2px 0 0', paddingLeft: 18, color: TEXT }}>
              {candidates.map((c) => (
                <li key={c}>{c}</li>
              ))}
            </ul>
          </>
        )}
      </>
    );
  }
  return (
    <>
      <div data-testid="ladder-headline" style={{ fontSize: compact ? 15 : '1rem', fontWeight: 700, color: '#e6edf3' }}>{view.headline}</div>
      {s.picture && <LadderPicture picture={s.picture} />}
      <ul data-testid="ladder-outcomes" style={{ margin: '4px 0 8px', paddingLeft: 18, color: TEXT }}>
        {s.outcomes.map((o) => (
          <li key={o.kind}>{o.text}</li>
        ))}
      </ul>
      <ol data-testid="ladder-rungs" style={{ margin: 0, paddingLeft: 18 }}>
        {s.rungs.map((r) => (
          <RungBlock key={r.index} r={r} compact={compact} action={rungAction?.(r)} />
        ))}
      </ol>
      {s.after_deposit && (
        <div data-testid="ladder-after-deposit" style={{ color: MUTED, fontSize: '0.72rem', marginTop: 4 }}>
          After you deposit{s.after_deposit.kind === 'manual_config' ? ' (current fallback)' : ''}: {s.after_deposit.text}
        </div>
      )}
      <div style={{ color: MUTED, fontSize: '0.7rem', marginTop: 4 }}>{s.statement}</div>
    </>
  );
}

/** The dashboard card, directly under the directive banner. Nothing without a suggestion. */
export function LadderSuggestionCard({
  suggestion,
  syncStatus,
  rungAction,
}: {
  suggestion: LadderSuggestion | null | undefined;
  syncStatus?: SyncStatus;
  rungAction?: (r: LadderRung) => ReactNode;
}) {
  if (!suggestion) return null;
  return (
    <section
      data-testid="ladder-suggestion"
      style={{ margin: '12px 0', padding: '12px 14px', border: '1px solid #065f46', borderRadius: 10, background: '#0d1117', fontSize: '0.8rem', lineHeight: 1.6, ...WRAP }}
    >
      <div style={{ fontSize: '0.7rem', letterSpacing: 2, color: MUTED, marginBottom: 4 }}>{LADDER_TITLE.toUpperCase()}</div>
      <Body s={suggestion} syncStatus={syncStatus} rungAction={rungAction} />
    </section>
  );
}

/** The phone cockpit's card: the same headline, outcomes and deposits; the picture too. */
export function LadderSuggestionLiteCard({
  suggestion,
  syncStatus,
  cardStyle,
  labelStyle,
  rungAction,
}: {
  suggestion: LadderSuggestion | null | undefined;
  syncStatus?: SyncStatus;
  cardStyle?: CSSProperties;
  labelStyle?: CSSProperties;
  rungAction?: (r: LadderRung) => ReactNode;
}) {
  if (!suggestion) return null;
  return (
    <section style={{ ...cardStyle, fontSize: 12, lineHeight: 1.6, color: TEXT, ...WRAP }} data-testid="ladder-suggestion-lite">
      <div style={labelStyle}>LADDER</div>
      <Body s={suggestion} syncStatus={syncStatus} compact rungAction={rungAction} />
    </section>
  );
}
