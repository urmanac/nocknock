/**
 * Detected swaps for the accumulation plan (v0.6.0, issue #118).
 *
 * The worker reads the wallet's incoming NOCK transfers every cycle and checks each new
 * transaction against its receipt with the rules of the record endpoint. What passes is
 * OFFERED here, never counted on its own:
 *   CONFIRM               records it through the same endpoint as a pasted hash, which
 *                         verifies the receipt again.
 *   NOT PART OF THE PLAN  leaves it uncounted, listed below, restorable.
 * When the worker could not check, this says so; it never shows that as "none found".
 */
import { useState } from 'react';
import type { FillDetection, FillOffer } from '../types/snapshot';
import {
  COST_OVER_SPOT_CAVEAT,
  blockTime,
  confirmOffer,
  costOverSpot,
  detectionSummary,
  dismissOffer,
  restoreOffer,
  swapLine,
  timingLabel,
  txUrl,
  type OfferActionResult,
} from '../fillOffers';

const TONE_COLOR = { ok: '#10b981', warn: '#f59e0b', muted: '#8b949e' } as const;

const small = { fontSize: '0.7rem', color: '#8b949e', lineHeight: 1.6 } as const;
const mono = { fontFamily: 'monospace', fontSize: '0.72rem', color: '#c9d1d9', wordBreak: 'break-all' } as const;

function button(primary: boolean, disabled: boolean) {
  return {
    background: disabled ? '#21262d' : primary ? '#1f6feb' : '#0d1117',
    color: '#f0f6fc',
    border: '1px solid #30363d',
    borderRadius: 6,
    padding: '8px 14px',
    fontSize: '0.75rem',
    letterSpacing: 1,
    cursor: disabled ? 'default' : 'pointer',
  } as const;
}

function SwapFacts({ offer }: { offer: FillOffer }) {
  const cost = costOverSpot(offer);
  const timing = timingLabel(offer.timing);
  return (
    <div style={{ minWidth: 260, flex: 1 }}>
      <div style={{ color: '#f0f6fc', fontWeight: 600, fontSize: '0.85rem' }}>{swapLine(offer)}</div>
      <div style={small}>
        {blockTime(offer.confirmed_at)} · block {offer.block_number}
        {timing && (
          <>
            {' '}
            · <span style={{ color: offer.timing === 'early' ? '#f59e0b' : '#8b949e' }}>{timing}</span>
          </>
        )}
      </div>
      <div style={{ ...small, color: cost.known ? '#c9d1d9' : '#8b949e' }} title={cost.source || undefined}>
        Execution cost: {cost.text}
        {cost.reference && <span style={{ color: '#8b949e' }}> · {cost.reference}</span>}
      </div>
      <div style={mono}>
        <a href={txUrl(offer.tx_hash)} target="_blank" rel="noreferrer" style={{ color: '#58a6ff' }}>
          {offer.tx_hash}
        </a>
      </div>
    </div>
  );
}

export function FillOffers({
  detection,
  onChanged,
}: {
  detection?: FillDetection | null;
  /** Called after an action so the dashboard refetches the snapshot. */
  onChanged?: () => void;
}) {
  const [busy, setBusy] = useState<string | null>(null);
  const [message, setMessage] = useState<OfferActionResult | null>(null);
  const summary = detectionSummary(detection);
  const offers = detection?.offers ?? [];
  const dismissed = detection?.dismissed ?? [];
  const anyCost = [...offers, ...dismissed].some((o) => costOverSpot(o).known);

  const run = async (offer: FillOffer, action: (o: FillOffer) => Promise<OfferActionResult>) => {
    setBusy(offer.tx_hash);
    setMessage(null);
    const result = await action(offer);
    setMessage(result);
    setBusy(null);
    if (result.ok) onChanged?.();
  };

  return (
    <div style={{ marginBottom: 14 }}>
      {offers.map((offer) => (
        <div
          key={offer.tx_hash}
          data-testid="fill-offer"
          style={{
            border: '1px solid #30363d',
            borderLeft: '3px solid #38bdf8',
            borderRadius: 6,
            padding: '10px 14px',
            marginBottom: 8,
            display: 'flex',
            gap: 12,
            flexWrap: 'wrap',
            alignItems: 'center',
            justifyContent: 'space-between',
          }}
        >
          <div style={{ flexBasis: '100%', fontSize: '0.7rem', letterSpacing: 2, color: '#38bdf8' }}>
            DETECTED SWAP · NOT YET COUNTED
          </div>
          <SwapFacts offer={offer} />
          <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap' }}>
            <button
              onClick={() => run(offer, (o) => confirmOffer(o))}
              disabled={busy !== null}
              style={button(true, busy !== null)}
              title="Record this swap in the plan. The worker verifies its receipt on-chain again first."
            >
              {busy === offer.tx_hash ? 'WORKING…' : 'CONFIRM'}
            </button>
            <button
              onClick={() => run(offer, (o) => dismissOffer(o))}
              disabled={busy !== null}
              style={button(false, busy !== null)}
              title="Leave this swap out of the plan. It stays listed below and can be restored."
            >
              NOT PART OF THE PLAN
            </button>
          </div>
        </div>
      ))}

      {anyCost && (
        <div data-testid="cost-caveat" style={{ ...small, fontSize: '0.65rem', color: '#484f58' }}>
          {COST_OVER_SPOT_CAVEAT}
        </div>
      )}
      <div role="status" style={{ ...small, color: TONE_COLOR[summary.tone] }}>
        <strong style={{ letterSpacing: 1 }}>{summary.heading}</strong> · {summary.text}
      </div>
      {message && (
        <div role="status" style={{ ...small, color: message.ok ? '#10b981' : '#f59e0b' }}>
          {message.text}
        </div>
      )}

      {dismissed.length > 0 && (
        <div style={{ marginTop: 8 }}>
          <div style={{ fontSize: '0.7rem', letterSpacing: 2, color: '#8b949e', marginBottom: 4 }}>
            NOT COUNTED · MARKED “NOT PART OF THE PLAN”
          </div>
          {dismissed.map((offer) => (
            <div
              key={offer.tx_hash}
              data-testid="fill-dismissed"
              style={{
                border: '1px dashed #30363d',
                borderRadius: 6,
                padding: '8px 12px',
                marginBottom: 6,
                display: 'flex',
                gap: 12,
                flexWrap: 'wrap',
                alignItems: 'center',
                justifyContent: 'space-between',
                opacity: 0.85,
              }}
            >
              <SwapFacts offer={offer} />
              <button
                onClick={() => run(offer, (o) => restoreOffer(o))}
                disabled={busy !== null}
                style={button(false, busy !== null)}
                title="Put this swap back on offer. It is still not counted until you confirm it."
              >
                {busy === offer.tx_hash ? 'WORKING…' : 'RESTORE'}
              </button>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
