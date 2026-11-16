/**
 * Per-fill table for the accumulation plan (v0.6.0, issue #118).
 *
 * Lists the receipt-verified buys of the current plan from GET /v1/accumulation/events,
 * with each fill's execution cost over the reference spot at its block time. A fill
 * with no stored price near its block shows "unavailable", never a zero.
 */
import { useEffect, useState } from 'react';
import { api } from '../api/client';
import type { AccumulationEvent } from '../api/client';
import { blockTime, costOverSpot, fillsForPlan, shortHash, txUrl } from '../fillOffers';

const cell = { padding: '4px 10px 4px 0', fontSize: '0.75rem', color: '#c9d1d9', whiteSpace: 'nowrap' } as const;
const head = { ...cell, fontSize: '0.65rem', letterSpacing: 1, color: '#8b949e', textAlign: 'left' } as const;

/** Presentational part: no fetching, so it renders the same on the server and in tests. */
export function PlanFillsTable({
  planId,
  events,
  error,
}: {
  planId: string;
  /** null while the first load is in flight. */
  events: AccumulationEvent[] | null;
  error?: string | null;
}) {
  if (error) {
    return (
      <div role="status" style={{ fontSize: '0.7rem', color: '#f59e0b', marginTop: 10 }}>
        Recorded buys could not be listed: {error}. The plan totals above come from the snapshot and are unaffected.
      </div>
    );
  }
  if (events === null) {
    return <div style={{ fontSize: '0.7rem', color: '#8b949e', marginTop: 10 }}>Loading recorded buys…</div>;
  }
  const fills = fillsForPlan(events, planId);
  if (fills.length === 0) {
    return <div style={{ fontSize: '0.7rem', color: '#8b949e', marginTop: 10 }}>No buys recorded for this plan yet.</div>;
  }
  return (
    <div style={{ marginTop: 12, overflowX: 'auto' }}>
      <div style={{ fontSize: '0.7rem', letterSpacing: 2, color: '#8b949e', marginBottom: 4 }}>
        RECORDED BUYS · VERIFIED ON-CHAIN
      </div>
      <table style={{ borderCollapse: 'collapse' }}>
        <thead>
          <tr>
            <th style={head}>#</th>
            <th style={head}>BLOCK TIME</th>
            <th style={head}>USDC SPENT</th>
            <th style={head}>NOCK RECEIVED</th>
            <th style={head}>PRICE</th>
            <th style={head}>COST OVER SPOT</th>
            <th style={head}>TRANSACTION</th>
          </tr>
        </thead>
        <tbody>
          {fills.map((f, i) => {
            const cost = costOverSpot(f);
            const price = Number(f.effective_price_usd);
            return (
              <tr key={f.tx_hash} data-testid="plan-fill">
                <td style={cell}>{i + 1}</td>
                <td style={cell}>{blockTime(f.confirmed_at)}</td>
                <td style={cell}>${Number(f.usdc_spent).toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}</td>
                <td style={cell}>{Number(f.nock_received).toLocaleString('en-US', { maximumFractionDigits: 2 })}</td>
                <td style={cell}>{Number.isFinite(price) ? `$${price.toFixed(6)}` : f.effective_price_usd}</td>
                <td style={{ ...cell, color: cost.known ? '#c9d1d9' : '#8b949e' }} title={cost.source || undefined}>
                  {cost.known ? cost.text : 'unavailable'}
                </td>
                <td style={{ ...cell, fontFamily: 'monospace' }}>
                  <a href={txUrl(f.tx_hash)} target="_blank" rel="noreferrer" style={{ color: '#58a6ff' }} title={f.tx_hash}>
                    {shortHash(f.tx_hash)}
                  </a>
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
      <div style={{ fontSize: '0.65rem', color: '#484f58', marginTop: 4, lineHeight: 1.6 }}>
        Cost over spot compares each fill's price with the NOCK/USD price NockNock stored nearest its block time (within
        60 minutes; hover a figure for the source). It is a measurement, not a quote.
      </div>
    </div>
  );
}

/** Fetches the fills; `refreshKey` changes when the plan's fill count does. */
export function PlanFills({ planId, refreshKey }: { planId: string; refreshKey: string }) {
  const [events, setEvents] = useState<AccumulationEvent[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let live = true;
    api
      .accumulationEvents()
      .then((res) => {
        if (!live) return;
        setEvents(res.events ?? []);
        setError(null);
      })
      .catch((e) => {
        if (live) setError(e instanceof Error ? e.message : 'request failed');
      });
    return () => {
      live = false;
    };
  }, [planId, refreshKey]);
  return <PlanFillsTable planId={planId} events={events} error={error} />;
}
