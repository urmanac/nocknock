/**
 * Ladder Simulator panel (#166): a read-only what-if for a USDC/NOCK range that is mostly
 * USDC and sits below spot. The worker works it out from the stored snapshot; nothing is
 * read on-chain, signed or sent, and no field is prefilled with an amount. A breached
 * reserve or an exceeded budget is shown as a measured fact, never as an instruction.
 */
import { useState } from 'react';
import { api, describeApiError } from '../api/client';
import type { Position } from '../types/snapshot';
import {
  EMPTY_LADDER_FORM,
  buildLadderQuery,
  ladderErrorLines,
  ladderNockMoved,
  ladderPoolOptions,
  ladderSnapshotLine,
  ladderSummaryLines,
  validateLadderForm,
  type LadderForm,
  type LadderOutcome,
} from '../ladder';

const label = { fontSize: '0.7rem', letterSpacing: 2, color: '#8b949e', marginBottom: 6 } as const;
const small = { fontSize: '0.7rem', color: '#8b949e', marginTop: 6, lineHeight: 1.6 } as const;
const field = {
  background: '#0d1117',
  border: '1px solid #30363d',
  borderRadius: 6,
  color: '#f0f6fc',
  padding: '6px 8px',
  fontFamily: 'monospace',
  fontSize: '0.8rem',
  width: 110,
} as const;
const MEASURED_RED = '#f87171';

/** The result of one simulation, or the worker's stated reason for none. Pure. */
export function LadderResult({ outcome }: { outcome: LadderOutcome }) {
  if (!outcome.ok) {
    return (
      <div data-testid="ladder-error" style={{ ...small, color: '#f59e0b' }}>
        {ladderErrorLines(outcome.error).map((l) => (
          <div key={l}>{l}</div>
        ))}
      </div>
    );
  }
  const sim = outcome.sim;
  const lines = ladderSummaryLines(sim);
  const breached = sim.reserve_check.status === 'breached';
  const exceeds = sim.floor_check.status === 'exceeds';
  return (
    <div data-testid="ladder-result" style={{ marginTop: 12 }}>
      <div style={{ ...small, marginTop: 0 }}>{ladderSnapshotLine(sim)}</div>
      <div style={{ fontFamily: 'monospace', fontSize: '0.75rem', color: '#c9d1d9', lineHeight: 1.7, marginTop: 6 }}>
        {lines.map((l) => (
          <div
            key={l}
            style={{
              color: (breached && l.startsWith('Reserve:')) || (exceeds && l.startsWith('Floor budget:')) ? MEASURED_RED : undefined,
            }}
          >
            {l}
          </div>
        ))}
      </div>
      <table style={{ width: '100%', borderCollapse: 'collapse', fontSize: '0.72rem', fontFamily: 'monospace', marginTop: 10 }}>
        <thead>
          <tr style={{ color: '#8b949e', textAlign: 'right' }}>
            <th style={{ textAlign: 'left' }}>% of spot</th>
            <th>Price</th>
            <th>NOCK bought / sold</th>
            <th>USDC left</th>
            <th>Stable</th>
            <th>Reserve headroom</th>
          </tr>
        </thead>
        <tbody>
          {sim.grid.map((r) => (
            <tr key={r.price_usd} style={{ textAlign: 'right', color: r.markers.length ? '#f0f6fc' : '#c9d1d9' }}>
              <td style={{ textAlign: 'left' }}>
                {r.pct_of_spot}%{r.markers.length ? ` (${r.markers.join(', ')})` : ''}
              </td>
              <td>${r.price_usd}</td>
              <td>{ladderNockMoved(r.ladder_nock_acquired)}</td>
              <td>${r.ladder_usdc}</td>
              <td>{r.stable_usdc == null ? 'unknown' : `$${r.stable_usdc}`}</td>
              <td style={{ color: r.reserve_headroom_usdc?.startsWith('-') ? MEASURED_RED : undefined }}>
                {r.reserve_headroom_usdc ?? '—'}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
      <div style={small}>
        {sim.assumptions.join(' ')} {sim.statement}
      </div>
    </div>
  );
}

export function LadderSimulatorPanel({
  positions,
  snapshotId,
}: {
  positions?: Position[] | null;
  /** Changes when a new snapshot arrives; a result from the previous one is cleared. */
  snapshotId?: string;
}) {
  const [form, setForm] = useState<LadderForm>(EMPTY_LADDER_FORM);
  const [result, setResult] = useState<{ outcome: LadderOutcome; snapshotId?: string } | null>(null);
  const [message, setMessage] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  // A result describes the snapshot it was worked from: a new one invalidates it.
  const stale = result !== null && result.snapshotId !== snapshotId;
  const outcome = stale ? null : result?.outcome ?? null;

  const set = <K extends keyof LadderForm>(k: K, v: LadderForm[K]) => setForm((f) => ({ ...f, [k]: v }));
  const options = ladderPoolOptions(positions);

  const simulate = async () => {
    const problem = validateLadderForm(form);
    if (problem) {
      setMessage(problem);
      return;
    }
    setBusy(true);
    setMessage(null);
    try {
      setResult({ outcome: await api.simulateLadder(buildLadderQuery(form)), snapshotId });
    } catch (e) {
      setResult(null);
      setMessage(describeApiError(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div data-testid="ladder-simulator">
      <div style={label}>WHAT-IF ONLY · FROM THE STORED SNAPSHOT · NOTHING IS READ, SIGNED OR SENT</div>
      <div style={{ display: 'flex', gap: 10, flexWrap: 'wrap', alignItems: 'flex-end', fontSize: '0.75rem', color: '#c9d1d9' }}>
        <label>
          USDC
          <br />
          <input aria-label="Ladder USDC" style={field} value={form.usdc} onChange={(e) => set('usdc', e.target.value)} inputMode="decimal" />
        </label>
        <label>
          NOCK (empty: worked out)
          <br />
          <input aria-label="Ladder NOCK" style={field} value={form.nock} onChange={(e) => set('nock', e.target.value)} inputMode="decimal" />
        </label>
        <label>
          Bottom
          <br />
          <input aria-label="Ladder bottom" style={field} value={form.low} onChange={(e) => set('low', e.target.value)} inputMode="decimal" />
          <select aria-label="Bottom unit" style={{ ...field, width: 'auto' }} value={form.lowMode} onChange={(e) => set('lowMode', e.target.value as LadderForm['lowMode'])}>
            <option value="usd">USD per NOCK</option>
            <option value="pct">% of spot</option>
          </select>
        </label>
        <label>
          Top
          <br />
          <input
            aria-label="Ladder top"
            style={field}
            value={form.highMode === 'spot' ? '' : form.high}
            disabled={form.highMode === 'spot'}
            onChange={(e) => set('high', e.target.value)}
            inputMode="decimal"
          />
          <select aria-label="Top unit" style={{ ...field, width: 'auto' }} value={form.highMode} onChange={(e) => set('highMode', e.target.value as LadderForm['highMode'])}>
            <option value="spot">= spot</option>
            <option value="usd">USD per NOCK</option>
            <option value="pct">% of spot</option>
          </select>
        </label>
        <label>
          Pool
          <br />
          <select aria-label="Ladder pool" style={{ ...field, width: 'auto' }} value={form.pool} onChange={(e) => set('pool', e.target.value)}>
            {options.map((o) => (
              <option key={o.value} value={o.value} disabled={o.disabled} title={o.reason ?? undefined}>
                {o.reason ? `${o.label}: ${o.reason}` : o.label}
              </option>
            ))}
          </select>
        </label>
        {form.pool === 'spacing' && (
          <label>
            Tick spacing
            <br />
            <input aria-label="Tick spacing" style={field} value={form.tickSpacing} onChange={(e) => set('tickSpacing', e.target.value)} inputMode="numeric" />
          </label>
        )}
        <label>
          Grid to (% of spot)
          <br />
          <input aria-label="Grid depth" style={field} value={form.gridToPct} onChange={(e) => set('gridToPct', e.target.value)} placeholder="-40" inputMode="decimal" />
        </label>
        <button
          onClick={simulate}
          disabled={busy}
          style={{
            background: busy ? '#21262d' : '#1f6feb',
            color: '#f0f6fc',
            border: '1px solid #30363d',
            borderRadius: 6,
            padding: '7px 14px',
            fontSize: '0.8rem',
            letterSpacing: 1,
            cursor: busy ? 'default' : 'pointer',
          }}
        >
          {busy ? 'SIMULATING…' : 'SIMULATE'}
        </button>
      </div>
      {(message || stale) && (
        <div role="status" style={{ ...small, color: '#f59e0b' }}>
          {message ?? 'A new snapshot arrived: simulate again to see it.'}
        </div>
      )}
      {outcome && <LadderResult outcome={outcome} />}
    </div>
  );
}
