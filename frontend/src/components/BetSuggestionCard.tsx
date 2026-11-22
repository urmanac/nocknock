/**
 * The WETH/NOCK bet card (issue #181): the dashboard's card, its quiet line and the phone
 * cockpit's card. No input of any kind: the worker measures everything, and the card reads
 * it top to bottom as a headline, the range, the amounts to type beside the deposit link,
 * the outcomes and the gates. Done things are quiet lines: a bet in range, a bet cashed out.
 * The only buttons answer a bet found staked in the gauge that is not counted yet.
 *
 * Every flex row wraps, every flex child may shrink (min-width: 0) and long text (the link,
 * an address) breaks anywhere, so nothing leaks out of the card on a phone.
 */
import { useState, type CSSProperties } from 'react';
import type { SyncStatus } from '../hooks/useFreshness';
import {
  BET_TITLE,
  betAmountsLine,
  betAtTop,
  betCardView,
  betChoiceLabel,
  betGateLines,
  betLinkLabel,
  betLinkUrl,
  betQuietLine,
  tapBet,
  type BetSuggestion,
  type FoundBet,
} from '../betSuggestion';

const MUTED = '#8b949e';
const TEXT = '#c9d1d9';
const GREEN = '#10b981';
const AMBER = '#fbbf24';
/** Long text breaks anywhere and never sets the card's width. */
const WRAP: CSSProperties = { overflowWrap: 'anywhere', wordBreak: 'break-word', minWidth: 0, maxWidth: '100%' };
/** A flex row that wraps; its children may shrink below their content. */
const ROW: CSSProperties = { display: 'flex', gap: 8, alignItems: 'baseline', flexWrap: 'wrap', minWidth: 0 };

function buttonStyle(primary: boolean, busy: boolean): CSSProperties {
  return {
    background: primary ? '#065f46' : '#1f2937',
    color: primary ? '#ecfdf5' : TEXT,
    border: `1px solid ${primary ? GREEN : '#475569'}`,
    borderRadius: 6,
    padding: '4px 10px',
    fontSize: '0.72rem',
    cursor: busy ? 'wait' : 'pointer',
    opacity: busy ? 0.6 : 1,
    maxWidth: '100%',
    whiteSpace: 'normal',
  };
}

function FoundBetRow({ f, onChanged }: { f: FoundBet; onChanged?: () => void }) {
  const [busy, setBusy] = useState(false);
  const [note, setNote] = useState<string | null>(null);
  const tap = async (choice: string) => {
    setBusy(true);
    const r = await tapBet(f.nft_id, choice);
    setNote(r.text);
    setBusy(false);
    if (r.ok) onChanged?.();
  };
  const choices = f.dismissed ? ['restore'] : f.choices;
  return (
    <li data-testid="bet-found" style={{ marginBottom: 6, ...WRAP }}>
      <div style={WRAP}>{f.dismissed ? `Set aside. ${f.line}` : f.line}</div>
      <div style={{ ...ROW, marginTop: 4 }}>
        {choices.map((c) => (
          <button key={c} onClick={() => tap(c)} disabled={busy} style={buttonStyle(c === 'bet', busy)}>
            {c === 'restore' ? 'RESTORE' : betChoiceLabel(c)}
          </button>
        ))}
      </div>
      {note ? <div style={{ color: MUTED, fontSize: '0.72rem', marginTop: 2, ...WRAP }}>{note}</div> : null}
    </li>
  );
}

/** The full card. */
export function BetSuggestionCard({ bet, syncStatus, onChanged, compact }: { bet: BetSuggestion; syncStatus?: SyncStatus; onChanged?: () => void; compact?: boolean }) {
  const view = betCardView(bet, syncStatus);
  const s = bet.suggestion;
  const url = betLinkUrl(s?.link?.url);
  const small: CSSProperties = { color: MUTED, fontSize: compact ? 11 : '0.72rem', ...WRAP };
  const ready = view.mode === 'ready' && !!s;
  return (
    <section
      data-testid="bet-card"
      data-status={bet.status}
      style={{ border: `1px solid ${ready ? GREEN : '#30363d'}`, borderRadius: 10, padding: compact ? '8px 10px' : '10px 14px', margin: '8px 0', fontSize: compact ? 12 : '0.8rem', lineHeight: 1.5, color: TEXT, ...WRAP }}
    >
      <div style={ROW}>
        <span style={{ fontWeight: 700, letterSpacing: 1, color: '#f0f6fc' }}>{BET_TITLE}</span>
        {bet.pool ? <span style={small}>{bet.pool.label}</span> : null}
      </div>
      <div data-testid="bet-headline" style={{ marginTop: 4, fontWeight: 600, ...WRAP }}>{view.headline}</div>
      {view.line ? <div style={{ ...small, marginTop: 2 }}>{view.line}</div> : null}
      {ready && s ? (
        <>
          <div data-testid="bet-range" style={{ marginTop: 6, ...WRAP }}>{s.range_text}</div>
          <div data-testid="bet-amounts" style={{ marginTop: 6, padding: '6px 8px', background: '#0d1117', border: '1px solid #30363d', borderRadius: 6, ...WRAP }}>
            <strong>{betAmountsLine(s)}</strong>
            <div style={small}>
              ${s.value_usd} in all: WETH ${s.weth_usd} + NOCK ${s.nock_usd}. The app has no amount in its link: type these.
            </div>
          </div>
          {url ? (
            <div style={{ marginTop: 6, ...WRAP }}>
              <a data-testid="bet-link" href={url} target="_blank" rel="noreferrer" style={{ color: '#38bdf8', ...WRAP }}>
                {betLinkLabel(s)}
              </a>
            </div>
          ) : null}
          <ul style={{ margin: '6px 0 0', paddingLeft: 18, ...WRAP }}>
            {s.outcomes.map((o) => (
              <li key={o.kind} style={WRAP}>{o.text}</li>
            ))}
          </ul>
        </>
      ) : null}
      {bet.found_not_counted.length > 0 ? (
        <ul style={{ margin: '6px 0 0', paddingLeft: 18, ...WRAP }}>
          {bet.found_not_counted.map((f) => (
            <FoundBetRow key={f.nft_id} f={f} onChanged={onChanged} />
          ))}
        </ul>
      ) : null}
      {betGateLines(bet).length > 0 ? (
        <ul data-testid="bet-gates" style={{ margin: '6px 0 0', paddingLeft: 0, listStyle: 'none', ...WRAP }}>
          {betGateLines(bet).map((g, i) => (
            <li key={i} style={{ ...small, color: g.ok ? MUTED : AMBER }}>
              {g.ok ? '✓' : '·'} {g.text}
            </li>
          ))}
        </ul>
      ) : null}
      {bet.floor_crossing?.text ? <div style={{ ...small, marginTop: 4 }}>{bet.floor_crossing.text}</div> : null}
      {bet.bets.length > 0 ? (
        <ul data-testid="bet-lines" style={{ margin: '6px 0 0', paddingLeft: 0, listStyle: 'none', ...WRAP }}>
          {bet.bets.map((l) => (
            <li key={l.position_id} style={small}>{l.line}</li>
          ))}
        </ul>
      ) : null}
      {ready ? <div style={{ ...small, marginTop: 6 }}>{bet.after_deposit}</div> : null}
      {ready && s ? <div style={{ ...small, marginTop: 2 }}>{s.link.format_note}</div> : null}
      <div style={{ ...small, marginTop: 4 }}>{bet.statement}</div>
    </section>
  );
}

/** The quiet form: one line, the card behind a collapsed <details>. Nothing when the card
 *  is at the top or there is no card. */
export function BetQuietLine({ bet, syncStatus, onChanged, compact }: { bet: BetSuggestion | null | undefined; syncStatus?: SyncStatus; onChanged?: () => void; compact?: boolean }) {
  if (!bet || betAtTop(bet)) return null;
  return (
    <details data-testid="bet-quiet-line" style={{ color: MUTED, fontSize: compact ? 12 : '0.78rem', margin: '6px 0', ...WRAP }}>
      <summary style={{ cursor: 'pointer', ...WRAP }}>{betQuietLine(bet)}</summary>
      <BetSuggestionCard bet={bet} syncStatus={syncStatus} onChanged={onChanged} compact={compact} />
    </details>
  );
}

/** The phone cockpit: the full card while it is at the top, otherwise its quiet line. */
export function BetLiteCard({ bet, syncStatus, onChanged }: { bet: BetSuggestion | null | undefined; syncStatus?: SyncStatus; onChanged?: () => void }) {
  if (!bet) return null;
  return betAtTop(bet) ? <BetSuggestionCard bet={bet} syncStatus={syncStatus} onChanged={onChanged} compact /> : <BetQuietLine bet={bet} syncStatus={syncStatus} onChanged={onChanged} compact />;
}
