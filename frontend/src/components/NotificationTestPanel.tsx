/**
 * Notification check (#89): which channels the worker has configured, and a button
 * that sends one message labelled TEST through them. The worker does the sending and
 * reports each channel; this panel shows what it said. It never sees a token.
 */
import React from 'react';
import { notificationsApi } from '../api/notifications';
import {
  TONE_COLOR,
  channelLine,
  channelTone,
  describeNotificationError,
  statusSummary,
  testOutcome,
  type NotificationStatus,
  type NotificationTestResult,
} from '../notifications';

const text: React.CSSProperties = { fontSize: 12, color: '#8b949e', lineHeight: 1.6 };

export function NotificationTestPanel() {
  const [status, setStatus] = React.useState<NotificationStatus | null>(null);
  const [statusError, setStatusError] = React.useState<string | null>(null);
  const [busy, setBusy] = React.useState(false);
  const [result, setResult] = React.useState<NotificationTestResult | null>(null);
  const [sendError, setSendError] = React.useState<string | null>(null);

  React.useEffect(() => {
    let cancelled = false;
    notificationsApi
      .status()
      .then((s) => {
        if (!cancelled) setStatus(s);
      })
      .catch((e) => {
        if (!cancelled) setStatusError(describeNotificationError(e, false));
      });
    return () => {
      cancelled = true;
    };
  }, []);

  const send = async () => {
    setBusy(true);
    setSendError(null);
    setResult(null);
    try {
      setResult(await notificationsApi.sendTest());
    } catch (e) {
      setSendError(describeNotificationError(e, true));
    } finally {
      setBusy(false);
    }
  };

  if (statusError) {
    return <div style={{ ...text, color: TONE_COLOR.warn }}>Notification status unavailable. {statusError}</div>;
  }
  if (!status) {
    return <div style={text}>Reading notification status…</div>;
  }

  const summary = statusSummary(status);
  const outcome = result ? testOutcome(result) : null;
  const canSend = status.any_configured && !busy;
  // After a send, show what each channel did; before one, what is configured.
  const lines = result ? result.channels : status.channels;

  return (
    <div>
      <div style={{ ...text, color: TONE_COLOR[summary.tone] }}>{summary.text}</div>
      <div style={{ ...text, marginTop: 4 }}>
        {lines.map((c) => (
          <div key={c.channel} style={{ color: TONE_COLOR[channelTone(c.status)] }}>
            {channelLine(c)}
          </div>
        ))}
      </div>
      <button
        onClick={send}
        disabled={!canSend}
        style={{
          width: '100%',
          border: 'none',
          borderRadius: 8,
          padding: '12px 0',
          fontSize: 13,
          fontWeight: 700,
          letterSpacing: 2,
          marginTop: 10,
          cursor: canSend ? 'pointer' : 'default',
          background: canSend ? '#38bdf8' : '#30363d',
          color: canSend ? '#0b1020' : '#8b949e',
        }}
      >
        {busy ? 'SENDING…' : 'SEND TEST NOTIFICATION'}
      </button>
      {outcome && (
        <div role="status" style={{ ...text, color: TONE_COLOR[outcome.tone], marginTop: 8 }}>
          {outcome.headline}
        </div>
      )}
      {sendError && (
        <div role="status" style={{ ...text, color: TONE_COLOR.warn, marginTop: 8 }}>
          {sendError}
        </div>
      )}
      {status.cockpit_url ? (
        <div style={{ ...text, marginTop: 8 }}>
          Each message links to <span style={{ color: '#c9d1d9' }}>{status.cockpit_url}</span>
          {status.cockpit_url_note ? <span style={{ color: TONE_COLOR.warn }}> {status.cockpit_url_note}</span> : null}
        </div>
      ) : null}
      <div style={{ ...text, color: '#484f58', marginTop: 8 }}>
        {status.fires_on ? `The worker notifies on: ${status.fires_on} ` : ''}
        The test is labelled TEST and changes nothing. Channels are set in the worker's config.yaml; no token is
        shown here.
      </div>
    </div>
  );
}
