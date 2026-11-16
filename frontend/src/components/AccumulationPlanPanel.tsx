/**
 * Accumulation Plan panel (v0.5.0, issue #79).
 *
 * Shows the dollar-cost-averaging plan the worker is pacing, the standing LP-rebuild
 * bias, the swaps the worker found on-chain and offers for confirmation (v0.6.0,
 * issue #118), and a field to record a buy by transaction hash, which does not depend
 * on swap detection (it still needs a plan, an RPC key and a readable receipt).
 * The worker verifies the receipt on-chain before anything counts; this panel never
 * marks a tranche done on its own.
 */
import { useState } from 'react';
import { api } from '../api/client';
import type { AccumulationPlanStatus, LpRebuild } from '../types/snapshot';
import { LpRebuildSteps } from './LpRebuildSteps';
import { FillOffers } from './FillOffers';
import { PlanFills } from './PlanFills';
import { RouteQuotes } from './RouteQuotes';
import { trancheDueHint } from '../fillOffers';
import {
  PLAN_STATUS_COLOR,
  PLAN_STATUS_LABEL,
  dcaVsLumpSumNock,
  isTxHash,
  lpRebuildActive,
  planProgressPct,
  timeUntil,
} from '../accumulation';

const label = { fontSize: '0.7rem', letterSpacing: 2, color: '#8b949e', marginBottom: 6 } as const;
const big = { fontSize: '1.4rem', fontWeight: 700 } as const;
const small = { fontSize: '0.7rem', color: '#8b949e', marginTop: 6, lineHeight: 1.6 } as const;

export function AccumulationPlanPanel({
  plan,
  lpRebuild,
  onRecorded,
}: {
  plan?: AccumulationPlanStatus | null;
  lpRebuild?: LpRebuild;
  onRecorded?: () => void;
}) {
  const [txHash, setTxHash] = useState('');
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState<{ ok: boolean; text: string } | null>(null);

  const record = async () => {
    const hash = txHash.trim();
    if (!isTxHash(hash)) {
      setMessage({ ok: false, text: 'That is not a 32-byte transaction hash.' });
      return;
    }
    setBusy(true);
    setMessage(null);
    try {
      const res = await api.recordAccumulation(hash);
      setMessage({
        ok: true,
        text: res.duplicate
          ? 'Already recorded: this transaction is in the ledger.'
          : `Verified on-chain: $${res.usdc_spent} USDC bought ${Number(res.nock_received).toLocaleString(undefined, { maximumFractionDigits: 2 })} NOCK at $${Number(res.effective_price_usd).toFixed(6)}.`,
      });
      setTxHash('');
      // Refresh the worker so the plan advances, then the dashboard. A failed refresh
      // is not a failed recording: the next scheduled cycle picks it up.
      await api.ingestNow().catch(() => undefined);
      onRecorded?.();
    } catch (e) {
      setMessage({ ok: false, text: e instanceof Error ? e.message : 'Recording failed.' });
    } finally {
      setBusy(false);
    }
  };

  const lpNote = lpRebuildActive(lpRebuild) && lpRebuild && (
    <div
      style={{
        border: '1px solid #30363d',
        borderLeft: '3px solid #38bdf8',
        borderRadius: 6,
        padding: '10px 14px',
        marginBottom: 14,
        fontSize: '0.8rem',
        color: '#c9d1d9',
        lineHeight: 1.6,
      }}
    >
      <strong style={{ color: '#38bdf8', letterSpacing: 1 }}>REBUILD LP TRANCHES</strong> · LP holds ${lpRebuild.actual_usd}{' '}
      against a ${lpRebuild.target_usd} target ({(lpRebuild.target_weight_bps / 100).toFixed(0)}% of portfolio), $
      {lpRebuild.gap_usd} short. Re-establishing ranges from NOCK and USDC already held is a reposition, not a purchase:
      the stable total is unchanged, only its location moves.
      <LpRebuildSteps lp={lpRebuild} />
    </div>
  );

  if (!plan) {
    return (
      <div>
        {lpNote}
        <div style={{ fontSize: '0.8rem', color: '#8b949e', lineHeight: 1.6 }}>
          No accumulation plan is configured. Add an <code>accumulation_plan</code> section to the worker's config.yaml
          (budget, tranches, interval, with your own numbers) to have NOCK buys paced; NockNock does not size buys without one.
        </div>
      </div>
    );
  }

  const color = PLAN_STATUS_COLOR[plan.status] ?? '#8b949e';
  const pct = planProgressPct(plan);
  const due = timeUntil(plan.next_due_at);
  const vsLump = dcaVsLumpSumNock(plan);
  const acquired = parseFloat(plan.acquired_nock);
  const canRecord = plan.status !== 'invalid';
  // What to say around "Buy $X" follows the detection state (v0.6.0, issue #118).
  const dueHint = trancheDueHint(plan.fill_detection);

  return (
    <div>
      {lpNote}
      <div style={{ display: 'flex', gap: 24, flexWrap: 'wrap', marginBottom: 14 }}>
        <div style={{ flex: 1, minWidth: 200 }}>
          <div style={label}>
            PLAN “{plan.plan_id}” · {plan.venue.toUpperCase()}
          </div>
          <div style={{ ...big, color }}>{PLAN_STATUS_LABEL[plan.status] ?? plan.status}</div>
          <div style={small}>
            {plan.status === 'tranche_due' && (
              <>
                {dueHint.offerFirst && (
                  <>
                    <strong data-testid="offer-first" style={{ color: '#38bdf8' }}>
                      {dueHint.offerFirst}
                    </strong>{' '}
                  </>
                )}
                Buy <strong style={{ color: '#f0f6fc' }}>${plan.tranche_usdc}</strong> of NOCK (tranche {plan.tranche_index} of{' '}
                {plan.tranches_total}). {dueHint.afterBuying}
              </>
            )}
            {plan.status === 'waiting' && (
              <>
                Tranche {plan.tranche_index} of {plan.tranches_total} (${plan.nominal_tranche_usdc}) is due {due ?? '—'}.
              </>
            )}
            {plan.status === 'pending' && <>Starts {due ?? '—'}.</>}
            {plan.status === 'complete' && <>All {plan.fills} buys verified; the budget is deployed.</>}
            {(plan.status === 'paused' || plan.status === 'blocked' || plan.status === 'invalid') &&
              (plan.binding_constraint ?? '—')}
          </div>
          {plan.status === 'tranche_due' && plan.binding_constraint && (
            <div style={{ ...small, color: '#f59e0b' }}>
              Sized below the nominal ${plan.nominal_tranche_usdc}: {plan.binding_constraint}.
            </div>
          )}
        </div>

        <div style={{ flex: 1, minWidth: 200 }}>
          <div style={label}>BUDGET DEPLOYED</div>
          <div style={{ ...big, color: '#f0f6fc' }}>
            ${plan.spent_usdc} <span style={{ fontSize: '0.85rem', color: '#8b949e' }}>of ${plan.budget_usdc}</span>
          </div>
          <div
            style={{
              background: '#0d1117',
              borderRadius: 999,
              height: 10,
              border: '1px solid #30363d',
              overflow: 'hidden',
              marginTop: 6,
            }}
          >
            <div style={{ width: `${pct}%`, background: color, height: '100%', borderRadius: 999 }} />
          </div>
          <div style={small}>
            {plan.fills} verified buy{plan.fills === 1 ? '' : 's'} · ${plan.remaining_usdc} remaining
          </div>
          {plan.cash_reserve_usdc && (
            <div style={{ ...small, color: '#2ecc71', letterSpacing: 1 }}>
              ✓ ${plan.cash_reserve_usdc} CASH RESERVE PROTECTED — the plan never draws below it
            </div>
          )}
        </div>

        <div style={{ flex: 1, minWidth: 200 }}>
          <div style={label}>AVERAGE COST</div>
          <div style={{ ...big, color: '#f0f6fc' }}>
            {plan.avg_price_usd ? `$${parseFloat(plan.avg_price_usd).toFixed(5)}` : '—'}
          </div>
          <div style={small}>
            {acquired > 0
              ? `${acquired.toLocaleString(undefined, { maximumFractionDigits: 0 })} NOCK acquired`
              : 'No verified buys yet'}
            {vsLump !== null && (
              <>
                <br />
                On pace for{' '}
                <strong style={{ color: vsLump >= 0 ? '#10b981' : '#f59e0b' }}>
                  {vsLump >= 0 ? '+' : '−'}
                  {Math.abs(vsLump).toLocaleString(undefined, { maximumFractionDigits: 0 })} NOCK
                </strong>{' '}
                vs buying the whole budget at the first fill's price.
              </>
            )}
          </div>
        </div>
      </div>

      {/* Read-only quotes for the due tranche on Aero and on Uniswap (v0.7.0, issue #115). */}
      {plan.status === 'tranche_due' && <RouteQuotes quotes={plan.route_quotes} onChanged={onRecorded} />}

      {/* Detected swaps: offered, never counted until confirmed (v0.6.0, issue #118). */}
      {canRecord && <FillOffers detection={plan.fill_detection} onChanged={onRecorded} />}

      {canRecord && (
        <div style={{ ...label, marginBottom: 4 }}>RECORD A BUY BY TRANSACTION HASH · WORKS WITHOUT SWAP DETECTION</div>
      )}
      {canRecord && (
        <div style={{ display: 'flex', gap: 8, flexWrap: 'wrap', alignItems: 'center' }}>
          <input
            aria-label="Transaction hash of a NOCK purchase"
            value={txHash}
            onChange={(e) => setTxHash(e.target.value)}
            placeholder="0x… transaction hash of your USDC→NOCK swap"
            spellCheck={false}
            style={{
              flex: 1,
              minWidth: 260,
              background: '#0d1117',
              border: '1px solid #30363d',
              borderRadius: 6,
              color: '#f0f6fc',
              padding: '8px 10px',
              fontFamily: 'monospace',
              fontSize: '0.8rem',
            }}
          />
          <button
            onClick={record}
            disabled={busy || txHash.trim() === ''}
            style={{
              background: busy || txHash.trim() === '' ? '#21262d' : '#1f6feb',
              color: '#f0f6fc',
              border: '1px solid #30363d',
              borderRadius: 6,
              padding: '8px 14px',
              fontSize: '0.8rem',
              letterSpacing: 1,
              cursor: busy || txHash.trim() === '' ? 'default' : 'pointer',
            }}
          >
            {busy ? 'VERIFYING…' : 'RECORD BUY'}
          </button>
        </div>
      )}
      {message && (
        <div role="status" style={{ ...small, color: message.ok ? '#10b981' : '#f59e0b' }}>
          {message.text}
        </div>
      )}
      {canRecord && <PlanFills planId={plan.plan_id} refreshKey={`${plan.fills}:${plan.last_fill_at ?? ''}`} />}
      <div style={{ ...small, color: '#484f58' }}>
        The plan paces buys; it never executes them. Progress counts only swaps the worker has verified on-chain as
        USDC leaving and NOCK entering the sovereign wallet. A swap NockNock finds on its own is offered, not counted:
        it counts once you confirm it, and confirming verifies its receipt again. Missed windows do not stack.
      </div>
    </div>
  );
}
