/**
 * Lite mobile cockpit (issue #31) + Golden Path execution panel.
 *
 * Three elements only: floor health, uncollected fees + gas ROI, and the
 * EXECUTE BATCH HARVEST button that dispatches the worker-built claim
 * transactions to the paired wallet via WalletConnect (zero custody).
 *
 * Handles the Uniswap-app broadcast quirk: after the wallet approves a
 * transaction, broadcasting may be deferred until the operator returns to the
 * wallet app - the panel reports "approved, awaiting wallet broadcast" and
 * retries the receipt-verified ledger recording until it lands.
 */
import React from 'react';
import { api, ClaimFeeAction, ClaimFeesPayload, LedgerSummary } from '../api/client';
import {
  connectWallet,
  connectedAccount,
  disconnectWallet,
  getProvider,
  sendClaimTransaction,
  walletConnectConfigured,
} from '../walletconnect';

type ActionState =
  | { kind: 'idle' }
  | { kind: 'signing' }
  | { kind: 'awaiting_broadcast'; txHash: string; attempts: number }
  | { kind: 'recorded'; txHash: string }
  | { kind: 'failed'; detail: string };

const AWAIT_POLL_MS = 15_000;
const AWAIT_MAX_ATTEMPTS = 40; // ~10 minutes of pending-broadcast patience

export type RecordErrorKind = 'recorded' | 'awaiting_broadcast' | 'failed';

/**
 * Classify a record-harvest failure. 409 ALREADY_RECORDED is an idempotent success —
 * the harvest IS in the ledger (the first attempt's response was lost). 404
 * RECEIPT_NOT_FOUND covers the Uniswap-app pending-broadcast quirk (retry). Anything
 * else is a real failure.
 */
export function classifyRecordError(e: unknown): RecordErrorKind {
  const status = (e as { status?: number })?.status;
  const msg = e instanceof Error ? e.message : String(e);
  if (status === 409 || msg.includes('ALREADY_RECORDED')) return 'recorded';
  if (status === 404 || msg.includes('RECEIPT_NOT_FOUND') || msg.includes('404')) {
    return 'awaiting_broadcast';
  }
  return 'failed';
}

function shortAddr(a: string | undefined): string {
  if (!a) return '';
  return `${a.slice(0, 6)}…${a.slice(-4)}`;
}

export function WalletConnectBadge() {
  const [account, setAccount] = React.useState<string | undefined>(undefined);
  const [busy, setBusy] = React.useState(false);
  const [hint, setHint] = React.useState<string | null>(null);

  React.useEffect(() => {
    if (!walletConnectConfigured()) return;
    let cancelled = false;
    getProvider()
      .then((p) => {
        if (!cancelled) setAccount(connectedAccount(p));
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, []);

  const connect = async () => {
    setBusy(true);
    setHint(null);
    try {
      setAccount(await connectWallet());
    } catch (e) {
      setHint(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  const disconnect = async () => {
    setBusy(true);
    try {
      await disconnectWallet();
      setAccount(undefined);
    } catch (e) {
      setHint(e instanceof Error ? e.message : String(e));
    } finally {
      setBusy(false);
    }
  };

  if (!walletConnectConfigured()) {
    return (
      <span style={{ ...badgeStyle, color: '#8b949e' }} title="Set VITE_WALLETCONNECT_PROJECT_ID and rebuild to enable wallet pairing">
        WALLET: NOT CONFIGURED
      </span>
    );
  }
  return (
    <span style={{ ...badgeStyle, color: account ? '#2ecc71' : '#8b949e' }}>
      <span style={{ ...dotStyle, background: account ? '#2ecc71' : '#8b949e' }} />
      {account ? `PAIRED: ${shortAddr(account)}` : 'WALLET: NOT PAIRED'}
      <button
        onClick={account ? disconnect : connect}
        disabled={busy}
        style={{ ...badgeButton, color: '#38bdf8' }}
      >
        {busy ? '…' : account ? 'UNPAIR' : 'PAIR'}
      </button>
      {hint ? <span style={{ marginLeft: 6, color: '#f39c12' }}>{hint}</span> : null}
    </span>
  );
}

export function LiteCockpit() {
  const [payload, setPayload] = React.useState<ClaimFeesPayload | null>(null);
  const [payloadError, setPayloadError] = React.useState<string | null>(null);
  const [ledger, setLedger] = React.useState<LedgerSummary | null>(null);
  const [floorDelta, setFloorDelta] = React.useState<number | null>(null);
  const [floorTarget, setFloorTarget] = React.useState<number | null>(null);
  const [actionStates, setActionStates] = React.useState<Record<string, ActionState>>({});

  const loadPayload = React.useCallback(() => {
    api
      .claimFeesPayload()
      .then((p) => {
        setPayload(p);
        setPayloadError(null);
      })
      .catch((e) => setPayloadError(e instanceof Error ? e.message : String(e)));
  }, []);

  const loadLedger = React.useCallback(() => {
    api
      .latestSnapshot<any>()
      .then((s) => {
        const sl = s.data?.derived_metrics?.sovereign_ledger;
        if (sl) setLedger(sl as LedgerSummary);
        const gs = s.data?.derived_metrics?.global_status;
        if (gs?.nock_floor_delta != null) setFloorDelta(parseInt(gs.nock_floor_delta, 10));
        if (gs?.nock_floor_target != null) setFloorTarget(parseInt(gs.nock_floor_target, 10));
      })
      .catch(() => {});
  }, []);

  React.useEffect(() => {
    loadPayload();
    loadLedger();
    const t = setInterval(() => {
      loadPayload();
      loadLedger();
    }, 30_000);
    return () => clearInterval(t);
  }, [loadPayload, loadLedger]);

  const setState = (platform: string, s: ActionState) =>
    setActionStates((prev) => ({ ...prev, [platform]: s }));

  const recordWithPendingHandling = async (platform: string, txHash: string) => {
    for (let attempt = 1; attempt <= AWAIT_MAX_ATTEMPTS; attempt++) {
      try {
        await api.recordHarvest(platform, txHash);
        setState(platform, { kind: 'recorded', txHash });
        loadLedger();
        return;
      } catch (e) {
        const kind = classifyRecordError(e);
        if (kind === 'recorded') {
          setState(platform, { kind: 'recorded', txHash });
          loadLedger();
          return;
        }
        if (kind === 'awaiting_broadcast') {
          setState(platform, { kind: 'awaiting_broadcast', txHash, attempts: attempt });
          await new Promise((r) => setTimeout(r, AWAIT_POLL_MS));
          continue;
        }
        setState(platform, { kind: 'failed', detail: e instanceof Error ? e.message : String(e) });
        return;
      }
    }
    setState(platform, {
      kind: 'failed',
      detail: 'Transaction stayed unbroadcast for ~10 minutes; open the Uniswap app to broadcast, then retry from the ledger panel.',
    });
  };

  const executeAction = async (action: ClaimFeeAction) => {
    if (!payload) return;
    setState(action.platform, { kind: 'signing' });
    try {
      const txHash = await sendClaimTransaction(
        { to: action.to, data: action.data, value: action.value },
        payload.from,
      );
      await recordWithPendingHandling(action.platform, txHash);
    } catch (e) {
      setState(action.platform, {
        kind: 'failed',
        detail: e instanceof Error ? e.message : String(e),
      });
    }
  };

  const executable = (payload?.actions ?? []).filter((a) => a.simulation === 'success');
  const stateOf = (a: ClaimFeeAction): ActionState => actionStates[a.platform] ?? { kind: 'idle' };

  const floorColor = floorDelta == null ? '#8b949e' : floorDelta >= 0 ? '#2ecc71' : '#ef4444';

  return (
    <div style={lite.container}>
      <header style={lite.header}>
        <h1 style={lite.title}>NockNock Lite</h1>
        <div style={{ display: 'flex', alignItems: 'center', gap: 8 }}>
          <a
            href="/"
            style={{
              display: 'inline-flex',
              alignItems: 'center',
              gap: 4,
              fontSize: 11,
              fontWeight: 600,
              letterSpacing: 1,
              color: '#94a3b8',
              textDecoration: 'none',
              border: '1px solid #30363d',
              borderRadius: 999,
              padding: '4px 10px',
              background: '#161b22',
            }}
          >
            📊 FULL STATE ROOM
          </a>
          <WalletConnectBadge />
        </div>
      </header>

      {/* 1. Floor health */}
      <section style={lite.card}>
        <div style={lite.label}>SOVEREIGN FLOOR</div>
        <div style={{ ...lite.big, color: floorColor }}>
          {floorDelta == null
            ? '…'
            : `${floorDelta >= 0 ? '+' : ''}${floorDelta.toLocaleString()} NOCK`}
        </div>
        <div style={lite.sub}>
          {floorTarget != null ? `target ${floorTarget.toLocaleString()} NOCK` : 'target loading…'}
        </div>
      </section>

      {/* 2. Uncollected fees + gas ROI */}
      <section style={lite.card}>
        <div style={lite.label}>UNCOLLECTED FEES (BOTH LPs)</div>
        {payloadError ? (
          <div style={lite.errorText}>Worker unreachable: {payloadError}</div>
        ) : !payload ? (
          <div style={lite.big}>…</div>
        ) : (
          <>
            <div style={lite.big}>${payload.gross_usd ?? '—'}</div>
            <div style={lite.sub}>
              est. gas ${payload.estimated_gas_usd ?? '—'} · net after gas $
              {payload.net_after_gas_usd ?? '—'}
            </div>
            <div style={lite.sub}>
              {ledger
                ? `ledger: ${ledger.harvest_count} verified harvests · 40% tax reserve $${ledger.tax_reserve_usd} · 60% dry powder $${ledger.growth_dry_powder_usd}`
                : 'ledger: loading…'}
            </div>
          </>
        )}
      </section>

      {/* 3. EXECUTE BATCH HARVEST */}
      <section style={lite.card}>
        <div style={lite.label}>GOLDEN PATH</div>
        {!walletConnectConfigured() ? (
          <div style={lite.errorText}>
            Wallet pairing needs VITE_WALLETCONNECT_PROJECT_ID (free Reown project id) set at build time.
          </div>
        ) : !walletConnectConfigured() ? null : executable.length === 0 && payload ? (
          <div style={lite.sub}>No simulated-ready claim actions right now.</div>
        ) : null}
        {walletConnectConfigured() && executable.map((a) => {
          const st = stateOf(a);
          return (
            <div key={a.platform} style={lite.actionRow}>
              <div style={lite.actionLabel}>{a.label}</div>
              <div style={lite.sub}>
                {a.expected_usd ? `$${a.expected_usd}` : '—'} · gas ${a.gas_usd ?? '—'} · sim:{' '}
                {a.simulation}
              </div>
              <button
                style={{
                  ...lite.button,
                  background: st.kind === 'idle' ? '#38bdf8' : '#30363d',
                  color: st.kind === 'idle' ? '#0b1020' : '#8b949e',
                }}
                disabled={st.kind !== 'idle'}
                onClick={() => executeAction(a)}
              >
                {st.kind === 'idle' && 'EXECUTE'}
                {st.kind === 'signing' && 'SIGN IN WALLET…'}
                {st.kind === 'awaiting_broadcast' && `APPROVED - AWAITING BROADCAST (${st.attempts})`}
                {st.kind === 'recorded' && 'RECORDED ✓'}
                {st.kind === 'failed' && 'RETRY'}
              </button>
              {st.kind === 'awaiting_broadcast' && (
                <div style={lite.pendingText}>
                  Tx {shortAddr(st.txHash)} approved in the wallet but not yet broadcast - the
                  Uniswap app broadcasts when you return to it. Recording starts automatically.
                </div>
              )}
              {st.kind === 'recorded' && (
                <div style={lite.recordedText}>Verified receipt recorded in the 60/40 ledger.</div>
              )}
              {st.kind === 'failed' && <div style={lite.errorText}>{st.detail}</div>}
            </div>
          );
        })}
      </section>

      <footer style={lite.footer}>
        Zero custody - the sovereign wallet signs in your wallet app; NockNock only proposes
        worker-built transactions.
      </footer>
    </div>
  );
}

const badgeStyle: React.CSSProperties = {
  display: 'inline-flex',
  alignItems: 'center',
  gap: 6,
  fontSize: 11,
  letterSpacing: 1,
  border: '1px solid #30363d',
  borderRadius: 999,
  padding: '4px 10px',
  background: '#161b22',
};
const dotStyle: React.CSSProperties = {
  width: 7,
  height: 7,
  borderRadius: 999,
  display: 'inline-block',
};
const badgeButton: React.CSSProperties = {
  background: 'none',
  border: 'none',
  cursor: 'pointer',
  fontSize: 11,
  letterSpacing: 1,
  padding: 0,
  marginLeft: 4,
};

const lite: Record<string, React.CSSProperties> = {
  container: {
    minHeight: '100vh',
    background: '#0d1117',
    color: '#e6edf3',
    fontFamily: 'ui-monospace, SFMono-Regular, Menlo, monospace',
    padding: 16,
    maxWidth: 480,
    margin: '0 auto',
  },
  header: {
    display: 'flex',
    alignItems: 'center',
    justifyContent: 'space-between',
    marginBottom: 16,
  },
  title: { fontSize: 20, letterSpacing: 2, color: '#f0f6fc', margin: 0 },
  card: {
    background: '#161b22',
    border: '1px solid #30363d',
    borderRadius: 10,
    padding: 16,
    marginBottom: 12,
  },
  label: { fontSize: 11, letterSpacing: 2, color: '#8b949e', marginBottom: 8 },
  big: { fontSize: 28, fontWeight: 700, marginBottom: 4 },
  sub: { fontSize: 12, color: '#8b949e', lineHeight: 1.6 },
  button: {
    width: '100%',
    border: 'none',
    borderRadius: 8,
    padding: '12px 0',
    fontSize: 13,
    fontWeight: 700,
    letterSpacing: 2,
    cursor: 'pointer',
    marginTop: 10,
  },
  actionRow: { marginTop: 10 },
  actionLabel: { fontSize: 13, color: '#e6edf3', marginBottom: 2 },
  pendingText: { fontSize: 11, color: '#38bdf8', marginTop: 6, lineHeight: 1.5 },
  recordedText: { fontSize: 11, color: '#2ecc71', marginTop: 6 },
  errorText: { fontSize: 12, color: '#f39c12', marginTop: 6, lineHeight: 1.5 },
  footer: { fontSize: 10, color: '#484f58', textAlign: 'center', marginTop: 16, lineHeight: 1.6 },
};
