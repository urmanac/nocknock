/**
 * Route quotes for a due tranche (v0.7.0, issue #115).
 *
 * Two venues side by side, each with what the plan's tranche returns in NOCK along its
 * best verified route, read from the venue's own quoter contract at one block. The
 * worker names the venue that returns more NOCK as a fact; this panel marks it and
 * links each venue's own swap page. Nothing is prefilled and nothing is sent: the
 * operator enters the amount there and signs in their own wallet.
 *
 * REFRESH asks the worker to quote the same tranche again at the latest block. It takes
 * no amount: the worker quotes only the tranche its own plan says is due.
 */
import { useState } from 'react';
import { api } from '../api/client';
import type { RouteQuotes as RouteQuotesData } from '../types/snapshot';
import { ROUTE_QUOTE_CAVEAT, newerQuotes, quoteAge, quoteBlockLine, venueCard, type VenueCard } from '../routeQuotes';

const label = { fontSize: '0.7rem', letterSpacing: 2, color: '#8b949e' } as const;
const small = { fontSize: '0.7rem', color: '#8b949e', lineHeight: 1.6 } as const;

function Card({ card, amountUsdc }: { card: VenueCard; amountUsdc: string }) {
  const border = card.returnsMore ? '#10b981' : '#30363d';
  return (
    <div
      data-testid="venue-quote"
      style={{
        flex: 1,
        minWidth: 260,
        border: `1px solid ${border}`,
        borderRadius: 6,
        padding: '10px 14px',
        background: '#0d1117',
      }}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', gap: 8, alignItems: 'baseline' }}>
        <div style={{ ...label, color: '#c9d1d9' }}>{card.venue.toUpperCase()}</div>
        {card.returnsMore && (
          <div data-testid="returns-more" style={{ fontSize: '0.65rem', letterSpacing: 1, color: '#10b981' }}>
            RETURNS MORE NOCK
          </div>
        )}
      </div>
      <div
        title={card.exactNock || undefined}
        style={{ fontSize: '1.2rem', fontWeight: 700, color: card.quoted ? '#f0f6fc' : '#f59e0b', marginTop: 4 }}
      >
        {card.headline}
      </div>
      <div style={{ ...small, color: card.quoted ? '#c9d1d9' : '#f59e0b' }}>{card.detail}</div>
      {card.quoted && (
        <div style={{ ...small, marginTop: 6 }}>
          <div style={{ color: '#c9d1d9' }}>{card.route}</div>
          {card.hops.map((h) => (
            <div key={h} style={{ fontFamily: 'monospace', fontSize: '0.65rem' }}>
              {h}
            </div>
          ))}
          {card.otherRoutes.length > 0 && (
            <div style={{ marginTop: 4, color: '#6e7681' }}>
              Other verified routes: {card.otherRoutes.join(' · ')}
            </div>
          )}
        </div>
      )}
      {card.swapUrl && (
        <div style={{ marginTop: 8 }}>
          <a
            href={card.swapUrl}
            target="_blank"
            rel="noreferrer"
            style={{ color: '#58a6ff', fontSize: '0.72rem', letterSpacing: 1 }}
          >
            {card.swapLabel} ↗
          </a>
          <div style={small}>
            Nothing is prefilled: enter ${amountUsdc} USDC → NOCK there yourself.
          </div>
        </div>
      )}
    </div>
  );
}

/** Presentational part: no fetching, so it renders the same on the server and in tests. */
export function RouteQuotesView({
  quotes,
  nowMs,
  busy = false,
  error = null,
  onRefresh,
}: {
  quotes: RouteQuotesData;
  nowMs: number;
  busy?: boolean;
  error?: string | null;
  onRefresh?: () => void;
}) {
  const age = quoteAge(quotes.quoted_at, nowMs);
  return (
    <div
      data-testid="route-quotes"
      style={{ border: '1px solid #30363d', borderRadius: 6, padding: '10px 14px', marginBottom: 14 }}
    >
      <div style={{ display: 'flex', justifyContent: 'space-between', gap: 12, flexWrap: 'wrap', alignItems: 'center' }}>
        <div>
          <div style={label}>ROUTE QUOTES · ${quotes.amount_usdc} USDC → NOCK · READ-ONLY</div>
          <div style={small}>
            {quoteBlockLine(quotes)}
            {age ? ` · ${age}` : ''}
          </div>
        </div>
        {onRefresh && (
          <button
            onClick={onRefresh}
            disabled={busy}
            style={{
              background: busy ? '#21262d' : '#0d1117',
              color: '#f0f6fc',
              border: '1px solid #30363d',
              borderRadius: 6,
              padding: '6px 12px',
              fontSize: '0.72rem',
              letterSpacing: 1,
              cursor: busy ? 'default' : 'pointer',
            }}
          >
            {busy ? 'QUOTING…' : 'REFRESH QUOTES'}
          </button>
        )}
      </div>
      <div style={{ display: 'flex', gap: 12, flexWrap: 'wrap', marginTop: 10 }}>
        {quotes.venues.map((v) => (
          <Card key={v.venue} card={venueCard(v, quotes)} amountUsdc={quotes.amount_usdc} />
        ))}
      </div>
      <div data-testid="quote-comparison" style={{ fontSize: '0.8rem', color: '#f0f6fc', marginTop: 10 }}>
        {quotes.comparison}
      </div>
      {error && (
        <div role="status" style={{ ...small, color: '#f59e0b' }}>
          Could not refresh the quotes: {error}. The quotes above are unchanged and keep their own block.
        </div>
      )}
      <div data-testid="quote-caveat" style={{ ...small, color: '#6e7681', marginTop: 6 }}>
        {ROUTE_QUOTE_CAVEAT}
      </div>
    </div>
  );
}

/** The quotes of the snapshot, with a refresh that asks the worker for newer ones. */
export function RouteQuotes({ quotes, onChanged }: { quotes?: RouteQuotesData | null; onChanged?: () => void }) {
  const [refreshed, setRefreshed] = useState<RouteQuotesData | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const shown = newerQuotes(quotes, refreshed);
  if (!shown) return null;

  const refresh = async () => {
    setBusy(true);
    setError(null);
    try {
      const res = await api.refreshRouteQuotes();
      setRefreshed(res.route_quotes);
      onChanged?.();
    } catch (e) {
      setError(e instanceof Error ? e.message : 'request failed');
    } finally {
      setBusy(false);
    }
  };
  return <RouteQuotesView quotes={shown} nowMs={Date.now()} busy={busy} error={error} onRefresh={refresh} />;
}
