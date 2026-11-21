/**
 * The ladder suggestion card (issue #168): the dashboard's top-of-page ladder surface and
 * its phone cockpit twin. No input of any kind: the worker measures everything, and the
 * card reads it top to bottom as a headline, a picture on a price axis, the outcomes in
 * plain words, and the numbered deposits with their links. Each deposit leaves a slot
 * (`rungAction`) for the follow-up's sign-in-wallet button.
 *
 * With a configured shape (the operator's cap | near | deep) the list is every rung of
 * the shape in order: a deposit for each rung left, one stated line for a rung that is
 * deployed, passed or over; the picture draws a bar per rung, a deployed one hatched.
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
  excludedLines,
  ladderCardView,
  ladderDepositUrl,
  ladderLinkLabel,
  ladderList,
  linkFormatNote,
  missingSettingSentence,
  pictureGeometry,
  price4,
  rungFeeLine,
  rungNockLimit,
  rungRangeLines,
  rungTitle,
  shapeRowLines,
  statedLines,
  walletAfterLine,
} from '../ladderSuggestion';

const MUTED = '#8b949e';
const TEXT = '#c9d1d9';
const GREEN = '#10b981';
const AMBER = '#fbbf24';
const WRAP: CSSProperties = { overflowWrap: 'anywhere' };

/** The ladder on a USD-per-NOCK axis: spot, each rung's bar with its amount, the bottom
 *  line and the protected reserve's zone below it, with the reserve's label in its own
 *  legend row so it never runs across the bottom line. `width` is the drawing's unit
 *  width (narrow on the phone, so its labels stay readable). */
export function LadderPicture({ picture, width = 520 }: { picture: LadderPictureData; width?: number }) {
  const g = pictureGeometry(picture, width);
  if (!g) return null;
  const fs = g.fontSize;
  const { labels, axis, axisLabels, legend, deployedLegend } = g.rows;
  // The spot label sits left of its line, the bottom label right of its line.
  return (
    <svg
      data-testid="ladder-picture"
      viewBox={`0 0 ${g.width} ${g.height}`}
      width="100%"
      role="img"
      aria-label="The ladder on a price axis"
      style={{ maxWidth: g.width * 1.2, display: 'block', margin: '8px 0' }}
    >
      <defs>
        <pattern id="ladder-hatch" width={6} height={6} patternUnits="userSpaceOnUse" patternTransform="rotate(45)">
          <line x1={0} y1={0} x2={0} y2={6} stroke={MUTED} strokeWidth={2} />
        </pattern>
      </defs>
      {/* The reserve zone: below the ladder bottom. */}
      <rect data-testid="ladder-reserve-zone" x={8} y={labels + 6} width={Math.max(0, g.bottomX - 8)} height={axis - labels - 6} fill="#1e3a8a" fillOpacity={0.35} />
      {/* The ladder bottom. */}
      <line data-testid="ladder-bottom-line" x1={g.bottomX} x2={g.bottomX} y1={labels + 4} y2={axis} stroke="#93c5fd" strokeDasharray="4 3" />
      <text x={g.bottomX + 3} y={labels} fontSize={fs} fill="#93c5fd">bottom ${price4(Number(picture.bottom_usd))}</text>
      {/* Each rung. */}
      {g.bars.map((b) =>
        b.deployed ? (
          // Already deployed: hatched and muted, so it reads as done, not as a deposit.
          <g key={b.label + b.x} data-testid="ladder-bar" data-deployed="true">
            <rect x={b.x} y={b.y} width={b.w} height={20} rx={3} fill="url(#ladder-hatch)" fillOpacity={0.5} stroke={MUTED} strokeDasharray="3 2" />
            <text x={b.labelX} y={b.y + 14} fontSize={fs} fill={TEXT} textAnchor={b.labelAnchor}>{b.label}</text>
          </g>
        ) : (
          <g key={b.label + b.x} data-testid="ladder-bar">
            <rect x={b.x} y={b.y} width={b.w} height={20} rx={3} fill={GREEN} fillOpacity={0.6} stroke={GREEN} />
            <text x={b.labelX} y={b.y + 14} fontSize={fs} fill={b.labelInside ? '#ecfdf5' : '#6ee7b7'} textAnchor={b.labelAnchor}>{b.label}</text>
          </g>
        ),
      )}
      {/* Spot. */}
      {g.spotX != null && (
        <g data-testid="ladder-spot-marker">
          <line x1={g.spotX} x2={g.spotX} y1={labels + 18} y2={axis} stroke="#f59e0b" strokeWidth={2} />
          <text x={g.spotX} y={labels + 14} fontSize={fs} fill="#f59e0b" textAnchor="end">spot ${price4(Number(picture.spot_usd))}</text>
        </g>
      )}
      <line x1={8} x2={g.width - 8} y1={axis} y2={axis} stroke="#30363d" />
      <text x={8} y={axisLabels} fontSize={fs} fill={MUTED}>${price4(g.min)}</text>
      <text x={g.width - 8} y={axisLabels} fontSize={fs} fill={MUTED} textAnchor="end">${price4(g.max)} per NOCK</text>
      {/* The reserve's legend, in its own row. */}
      <rect x={8} y={legend - fs + 2} width={10} height={10} fill="#1e3a8a" fillOpacity={0.8} />
      <text data-testid="ladder-reserve-label" x={22} y={legend} fontSize={fs} fill="#93c5fd">{picture.reserve_label}</text>
      {deployedLegend != null && (
        <>
          <rect x={8} y={deployedLegend - fs + 2} width={10} height={10} fill="url(#ladder-hatch)" stroke={MUTED} />
          <text data-testid="ladder-deployed-legend" x={22} y={deployedLegend} fontSize={fs} fill={MUTED}>already deployed</text>
        </>
      )}
    </svg>
  );
}

function RungBlock({ r, action, compact }: { r: LadderRung; action?: ReactNode; compact?: boolean }) {
  const url = ladderDepositUrl(r.deep_link?.url);
  const limit = rungNockLimit(r);
  const fee = rungFeeLine(r);
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
      {/* The app drops an amount passed in the link (observed 2026-11-21): it is stated here. */}
      {url && r.deep_link?.amount_to_enter && (
        <div data-testid="ladder-amount-to-enter" style={{ fontWeight: 600, color: '#e6edf3' }}>{r.deep_link.amount_to_enter}</div>
      )}
      {/* A deposit that holds NOCK (the cap): the app works out the other side from the
          field typed, so the limit and the field that keeps it are stated. */}
      {limit && <div data-testid="ladder-nock-limit" style={{ color: AMBER }}>{limit}</div>}
      {fee && <div data-testid="ladder-fee-line" style={{ color: MUTED, fontSize: compact ? 11 : '0.7rem' }}>{fee}</div>}
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
  const muted: CSSProperties = { color: MUTED, fontSize: compact ? 11 : '0.72rem' };
  if (view.mode !== 'ready') {
    const missing = missingSettingSentence(s);
    const candidates = view.mode === 'line' ? candidateLines(s) : [];
    const excluded = view.mode === 'line' && s.status === 'no_target_pool' ? excludedLines(s) : [];
    const rows = view.mode === 'line' ? shapeRowLines(s) : [];
    return (
      <>
        <div data-testid="ladder-headline" style={{ fontSize: compact ? 14 : '0.95rem', fontWeight: 700, color: '#e6edf3' }}>{view.headline}</div>
        {missing ? <div style={{ color: TEXT }}>{missing}</div> : view.line ? <div style={{ color: MUTED }}>{view.line}</div> : null}
        {excluded.length > 0 && (
          <ul data-testid="ladder-excluded" style={{ margin: '2px 0 0', paddingLeft: 18, ...muted }}>
            {excluded.map((e) => (
              <li key={e}>{e}</li>
            ))}
          </ul>
        )}
        {rows.length > 0 && (
          <ol data-testid="ladder-shape-rows" style={{ margin: '4px 0 0', paddingLeft: 18, listStyle: 'none', ...muted }}>
            {rows.map((r) => (
              <li key={r}>{r}</li>
            ))}
          </ol>
        )}
        {candidates.length > 0 && (
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
  const note = linkFormatNote(s);
  const stated = statedLines(s);
  const wallet = walletAfterLine(s);
  return (
    <>
      <div data-testid="ladder-headline" style={{ fontSize: compact ? 15 : '1rem', fontWeight: 700, color: '#e6edf3' }}>{view.headline}</div>
      {s.picture && <LadderPicture picture={s.picture} width={compact ? 340 : 520} />}
      <ul data-testid="ladder-outcomes" style={{ margin: '4px 0 8px', paddingLeft: 18, color: TEXT }}>
        {s.outcomes.map((o) => (
          <li key={o.kind}>{o.text}</li>
        ))}
      </ul>
      {wallet && (
        <div data-testid="ladder-wallet-after" style={{ color: TEXT, margin: '0 0 8px' }}>{wallet}</div>
      )}
      <ol data-testid="ladder-rungs" style={{ margin: 0, paddingLeft: 18 }}>
        {ladderList(s).map((item) =>
          item.kind === 'deposit' ? (
            <RungBlock key={item.rung.index} r={item.rung} compact={compact} action={rungAction?.(item.rung)} />
          ) : (
            <li key={item.row.index} data-testid="ladder-rung-done" data-status={item.row.status} style={{ marginBottom: 10, color: MUTED, ...WRAP }}>
              {item.row.text}
            </li>
          ),
        )}
      </ol>
      {note && <div data-testid="ladder-link-note" style={muted}>{note}</div>}
      {stated.length > 0 && (
        <div data-testid="ladder-stated" style={{ marginTop: 6 }}>
          {stated.map((line) => (
            <div key={line} style={muted}>{line}</div>
          ))}
        </div>
      )}
      {s.after_deposit && (
        <div data-testid="ladder-after-deposit" style={{ ...muted, marginTop: 4 }}>
          {s.after_deposit.text}
        </div>
      )}
      {!compact && (s.assumptions?.length ?? 0) > 0 && (
        <div data-testid="ladder-assumptions" style={{ ...muted, marginTop: 4 }}>Assumptions: {s.assumptions!.join(' ')}</div>
      )}
      <div style={{ ...muted, marginTop: 4 }}>{s.statement}</div>
    </>
  );
}

/** The dashboard card, at the top of the page. Nothing without a suggestion. */
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

/** The phone cockpit's card: the same headline, picture, outcomes and deposits. */
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
