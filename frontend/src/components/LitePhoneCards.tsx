/**
 * Cards the Lite cockpit gains for the phone (#89).
 *
 * LiteStatusCard goes at the top: the current directive, whether the page is live, and
 * the accumulation plan with its RECORD BUY field, so a "DCA TRANCHE DUE" notification
 * lands on the tranche it is about. LitePhoneSetup goes at the bottom: how to add the
 * page to the home screen, and the notification test.
 */
import React from 'react';
import { api, describeApiError } from '../api/client';
import { useFreshness } from '../hooks/useFreshness';
import type { DerivedMetrics } from '../types/snapshot';
import { syncWindowOrDefault } from '../timeline';
import { LITE_TONE_COLOR, liteStatusView } from '../liteStatus';
import { detectPlatform, installHint, isStandalone } from '../pwa';
import { AccumulationPlanPanel } from './AccumulationPlanPanel';
import { NotificationTestPanel } from './NotificationTestPanel';

const card: React.CSSProperties = {
  background: '#161b22',
  border: '1px solid #30363d',
  borderRadius: 10,
  padding: 16,
  marginBottom: 12,
};
const label: React.CSSProperties = { fontSize: 11, letterSpacing: 2, color: '#8b949e', marginBottom: 8 };
const sub: React.CSSProperties = { fontSize: 12, color: '#8b949e', lineHeight: 1.6 };

export function LiteStatusCard() {
  // The worker's own cadence when /health reports it; 300 s otherwise.
  const [syncWindowSeconds, setSyncWindowSeconds] = React.useState<number>(syncWindowOrDefault(null));
  React.useEffect(() => {
    let cancelled = false;
    api
      .health()
      .then((h) => {
        if (!cancelled) setSyncWindowSeconds(syncWindowOrDefault(h.sync_window_seconds));
      })
      .catch(() => {});
    return () => {
      cancelled = true;
    };
  }, []);

  const { snapshot, loading, error, freshness, refetch } = useFreshness<{ derived_metrics?: DerivedMetrics }>(
    api.latestSnapshot,
    { syncWindowSeconds, pollIntervalMs: 10_000 },
  );
  const metrics = snapshot?.data?.derived_metrics;
  const view = liteStatusView({
    hasSnapshot: !!snapshot,
    loading,
    error: error ? describeApiError(error) : null,
    syncStatus: freshness.syncStatus,
    ageSeconds: freshness.ageSeconds,
    action: metrics?.global_status?.shot_advice_action,
    details: metrics?.global_status?.shot_advice_details,
  });
  const color = LITE_TONE_COLOR[view.tone];

  return (
    <>
      <section style={{ ...card, borderLeft: `3px solid ${color}` }}>
        <div style={label}>DIRECTIVE</div>
        <div style={{ fontSize: 16, fontWeight: 700, color: '#f0f6fc', lineHeight: 1.4, marginBottom: 6 }}>{view.action}</div>
        {view.details ? <div style={{ ...sub, color: '#c9d1d9' }}>{view.details}</div> : null}
        <div role="status" aria-live="polite" style={{ ...sub, color, marginTop: 8 }}>
          {view.freshness}
        </div>
      </section>
      {snapshot && metrics?.accumulation_plan ? (
        <section style={card}>
          <div style={label}>ACCUMULATION PLAN</div>
          <AccumulationPlanPanel plan={metrics.accumulation_plan} onRecorded={refetch} />
        </section>
      ) : null}
    </>
  );
}

export function LitePhoneSetup() {
  const hint = React.useMemo(() => {
    const standalone = isStandalone(
      typeof window.matchMedia === 'function' && window.matchMedia('(display-mode: standalone)').matches,
      (navigator as Navigator & { standalone?: boolean }).standalone,
    );
    return installHint({
      platform: detectPlatform(navigator.userAgent, navigator.maxTouchPoints),
      standalone,
      secure: window.isSecureContext,
    });
  }, []);

  return (
    <>
      <section style={card}>
        <div style={label}>ON YOUR PHONE</div>
        <div style={{ fontSize: 13, color: '#e6edf3', marginBottom: 4 }}>{hint.title}</div>
        {hint.steps.length > 0 ? (
          <ol style={{ ...sub, margin: 0, paddingLeft: 18 }}>
            {hint.steps.map((s) => (
              <li key={s}>{s}</li>
            ))}
          </ol>
        ) : null}
        {hint.note ? <div style={{ ...sub, marginTop: 6 }}>{hint.note}</div> : null}
      </section>
      <section style={card}>
        <div style={label}>NOTIFICATIONS</div>
        <NotificationTestPanel />
      </section>
    </>
  );
}
