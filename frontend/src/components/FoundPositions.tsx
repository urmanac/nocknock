/**
 * Found on-chain (v0.9.0; #145 for Uniswap v4): the block under the ladder card, on the
 * dashboard and in the phone cockpit.
 *
 * The operator deposits in the Uniswap app and comes back. The worker looks for the
 * wallet's new position NFT at the start of every cycle. This block shows what it found:
 *   - a position that matched a suggested rung, tracked by itself;
 *   - a position that did not: FOUND, NOT COUNTED, with what it holds and why, and one
 *     button per choice (track as a ladder rung, track as LP, not now);
 *   - a position set aside, restorable.
 * Every action is one tap. There is no field anywhere: no hash, no NFT id, no amount.
 * When the worker could not check, it says so here, never "nothing found".
 *
 * Wording lives in ../foundPositions.ts and is unit-tested.
 */
import { useState, type CSSProperties } from 'react';
import {
  FOUND_TITLE,
  type FoundPosition,
  type PositionDetection,
  type TapResult,
  type TrackedFromChain,
  checkNow,
  choiceLabel,
  choiceTitle,
  detectionHasRows,
  detectionSummary,
  foundLines,
  foundTitle,
  ignoredLine,
  listHeading,
  mintTxUrl,
  restoreFound,
  shortHash,
  tapFound,
  tapTracked,
  trackedLine,
  trackedState,
  whyNotAutomatic,
} from '../foundPositions';

const MUTED = '#8b949e';
const TEXT = '#c9d1d9';
const TONE = { ok: '#10b981', warn: '#f59e0b', muted: MUTED } as const;
const WRAP: CSSProperties = { overflowWrap: 'anywhere' };

function button(primary: boolean, disabled: boolean, compact?: boolean): CSSProperties {
  return {
    background: disabled ? '#21262d' : primary ? '#1f6feb' : '#0d1117',
    color: '#f0f6fc',
    border: '1px solid #30363d',
    borderRadius: 6,
    // A thumb-sized target on the phone.
    padding: compact ? '10px 12px' : '7px 12px',
    fontSize: compact ? 12 : '0.72rem',
    letterSpacing: 0.5,
    cursor: disabled ? 'default' : 'pointer',
  };
}

function MintLink({ tx }: { tx: string | null }) {
  const url = mintTxUrl(tx);
  if (!url) return null;
  return (
    <a href={url} target="_blank" rel="noopener noreferrer" style={{ color: '#58a6ff' }}>
      mint {shortHash(tx)}
    </a>
  );
}

function FoundRow({
  p,
  dismissed,
  busy,
  compact,
  onTap,
}: {
  p: FoundPosition;
  dismissed?: boolean;
  busy: boolean;
  compact?: boolean;
  onTap: (choice: string) => void;
}) {
  const why = whyNotAutomatic(p);
  return (
    <div
      data-testid={dismissed ? 'found-dismissed' : 'found-offer'}
      style={{
        border: dismissed ? '1px dashed #30363d' : '1px solid #30363d',
        borderLeft: dismissed ? '1px dashed #30363d' : '3px solid #f59e0b',
        borderRadius: 6,
        padding: '8px 12px',
        marginBottom: 8,
        opacity: dismissed ? 0.85 : 1,
        ...WRAP,
      }}
    >
      <div style={{ fontWeight: 600, color: '#e6edf3' }}>{foundTitle(p)}</div>
      {foundLines(p).map((line) => (
        <div key={line} style={{ color: TEXT }}>{line}</div>
      ))}
      {why && <div style={{ color: MUTED }}>{why}</div>}
      <div style={{ color: MUTED }}>
        <MintLink tx={p.mint_tx} />
      </div>
      <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap', marginTop: 6 }}>
        {dismissed ? (
          <button
            onClick={() => onTap('restore')}
            disabled={busy}
            style={button(false, busy, compact)}
            title="Puts it back on offer. It is still not counted until it is tracked."
          >
            RESTORE
          </button>
        ) : (
          p.choices.map((c) => (
            <button key={c} onClick={() => onTap(c)} disabled={busy} style={button(c.startsWith('ladder:'), busy, compact)} title={choiceTitle(c)}>
              {choiceLabel(c)}
            </button>
          ))
        )}
      </div>
    </div>
  );
}

function TrackedRow({ t, busy, compact, onTap }: { t: TrackedFromChain; busy: boolean; compact?: boolean; onTap: (choice: string) => void }) {
  const state = trackedState(t);
  return (
    <div data-testid="found-tracked" style={{ border: '1px solid #30363d', borderLeft: `3px solid ${state.counted ? TONE.ok : TONE.warn}`, borderRadius: 6, padding: '8px 12px', marginBottom: 8, ...WRAP }}>
      <div style={{ fontWeight: 600, color: '#e6edf3' }}>{trackedLine(t)}</div>
      <div style={{ color: state.counted ? TONE.ok : TONE.warn }}>{state.text}</div>
      {t.rung_note && <div style={{ color: MUTED }}>{t.rung_note}.</div>}
      <div style={{ color: MUTED }}>
        Ticks {t.tick_lower} to {t.tick_upper}
        {t.deposit_tx ? ' · ' : ''}
        <MintLink tx={t.deposit_tx} />
      </div>
      <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap', marginTop: 6 }}>
        {t.choices.map((c) => (
          <button key={c} onClick={() => onTap(c)} disabled={busy} style={button(false, busy, compact)} title={choiceTitle(c)}>
            {c.startsWith('ladder:') ? choiceLabel(c).replace('TRACK AS', 'SET AS') : choiceLabel(c)}
          </button>
        ))}
      </div>
    </div>
  );
}

/** The block's body, shared by the dashboard card and the phone card. */
function Body({
  d,
  compact,
  onChanged,
  actions,
}: {
  d: PositionDetection;
  compact?: boolean;
  /** Called after a tap so the page reads the snapshot again. */
  onChanged?: () => void;
  /** The writes; the real API by default, a stub in tests. */
  actions?: {
    tapFound: typeof tapFound;
    restoreFound: typeof restoreFound;
    tapTracked: typeof tapTracked;
    checkNow: typeof checkNow;
  };
}) {
  const act = actions ?? { tapFound, restoreFound, tapTracked, checkNow };
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<TapResult | null>(null);
  const run = async (action: () => Promise<TapResult>) => {
    setBusy(true);
    setMessage(null);
    const result = await action();
    setMessage(result);
    setBusy(false);
    // A failed tap changed nothing, but the page may be showing something older than
    // what the worker refused on: read again either way.
    onChanged?.();
  };
  const summary = detectionSummary(d);
  const ignored = ignoredLine(d);
  const offers = d.found_not_counted?.positions ?? [];
  const dismissed = d.dismissed?.positions ?? [];
  const small: CSSProperties = { color: MUTED, fontSize: compact ? 11 : '0.72rem' };
  return (
    <>
      {offers.length > 0 && (
        <>
          <div data-testid="found-heading" style={{ fontWeight: 700, color: TONE.warn, marginBottom: 4 }}>
            {listHeading(d.found_not_counted, 'found on-chain and NOT counted in any total')}
          </div>
          {offers.map((p) => (
            <FoundRow key={p.nft_id} p={p} busy={busy} compact={compact} onTap={(c) => run(() => act.tapFound(p, c))} />
          ))}
        </>
      )}
      {d.tracked.length > 0 && (
        <>
          <div style={{ ...small, letterSpacing: 1, marginBottom: 4 }}>TRACKED FROM THE CHAIN</div>
          {d.tracked.map((t) => (
            <TrackedRow key={t.position_id} t={t} busy={busy} compact={compact} onTap={(c) => run(() => act.tapTracked(t, c))} />
          ))}
        </>
      )}
      {dismissed.length > 0 && (
        <>
          <div data-testid="found-dismissed-heading" style={{ ...small, letterSpacing: 1, marginBottom: 4 }}>
            SET ASIDE BY YOU · {listHeading(d.dismissed, 'not counted').toUpperCase()}
          </div>
          {dismissed.map((p) => (
            <FoundRow key={p.nft_id} p={p} dismissed busy={busy} compact={compact} onTap={() => run(() => act.restoreFound(p))} />
          ))}
        </>
      )}
      <div role="status" data-testid="found-status" style={{ ...small, color: TONE[summary.tone] }}>
        <strong style={{ letterSpacing: 1 }}>{summary.heading}</strong> · {summary.text}
      </div>
      {ignored && <div data-testid="found-ignored" style={small}>{ignored}</div>}
      <div style={{ display: 'flex', gap: 10, alignItems: 'center', flexWrap: 'wrap', marginTop: 6 }}>
        <button
          data-testid="found-check-now"
          onClick={() => run(() => act.checkNow())}
          disabled={busy}
          style={button(false, busy, compact)}
          title="Runs one cycle now: the worker looks for new positions, then reads everything again."
        >
          {busy ? 'WORKING…' : 'CHECK NOW'}
        </button>
        {message && (
          <span role="status" data-testid="found-message" style={{ ...small, color: message.ok ? TONE.ok : TONE.warn }}>
            {message.text}
          </span>
        )}
      </div>
      {!compact && <div style={{ ...small, marginTop: 4 }}>{d.statement}</div>}
    </>
  );
}

type Props = {
  detection: PositionDetection | null | undefined;
  onChanged?: () => void;
  actions?: Parameters<typeof Body>[0]['actions'];
};

/** The dashboard block, under the ladder card. Nothing from a worker that predates it. */
export function FoundPositionsCard({ detection, onChanged, actions }: Props) {
  if (!detection) return null;
  const attention = (detection.found_not_counted?.count ?? 0) > 0 || detection.status === 'unavailable';
  return (
    <section
      data-testid="found-positions"
      style={{
        margin: '12px 0',
        padding: '12px 14px',
        border: `1px solid ${attention ? '#92400e' : '#30363d'}`,
        borderRadius: 10,
        background: '#0d1117',
        fontSize: '0.8rem',
        lineHeight: 1.6,
        ...WRAP,
      }}
    >
      <div style={{ fontSize: '0.7rem', letterSpacing: 2, color: MUTED, marginBottom: 4 }}>
        {FOUND_TITLE.toUpperCase()}
        {detectionHasRows(detection) ? '' : ' · NOTHING NEW'}
      </div>
      <Body d={detection} onChanged={onChanged} actions={actions} />
    </section>
  );
}

/** The phone cockpit's card: the same rows and the same one-tap buttons. */
export function FoundPositionsLiteCard({
  detection,
  onChanged,
  actions,
  cardStyle,
  labelStyle,
}: Props & { cardStyle?: CSSProperties; labelStyle?: CSSProperties }) {
  if (!detection) return null;
  return (
    <section style={{ ...cardStyle, fontSize: 12, lineHeight: 1.6, color: TEXT, ...WRAP }} data-testid="found-positions-lite">
      <div style={labelStyle}>{FOUND_TITLE.toUpperCase()}</div>
      <Body d={detection} compact onChanged={onChanged} actions={actions} />
    </section>
  );
}
