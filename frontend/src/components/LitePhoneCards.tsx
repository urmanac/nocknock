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
import { LITE_TONE_COLOR, freshnessAt, liteStatusView } from '../liteStatus';
import { detectPlatform, installHint, isStandalone } from '../pwa';
import { AccumulationPlanPanel } from './AccumulationPlanPanel';
import { NotificationTestPanel } from './NotificationTestPanel';
import { LadderSuggestionLiteCard } from './LadderSuggestionCard';
import { ladderSuggestionOf } from '../ladderSuggestion';
import { FoundPositionsLiteCard } from './FoundPositions';
import { positionDetectionOf } from '../foundPositions';
import { ladderProfileOf } from '../ladderProfile';
import { TopOfPageLiteEvents } from './TopOfPage';
import { LadderShapeChart } from './LadderShapeChart';
import { FoundQuietLine, LadderDeployedLine, ladderSuggestionAtTop } from './QuietLadderLines';
import { foundNeedsAttention } from '../foundPositions';
import { EventAnchors } from './EventAnchors';
import { eventsOf } from '../events';
import { BetLiteCard } from './BetSuggestionCard';
import { betSuggestionOf } from '../betSuggestion';

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

  // useFreshness keeps the last snapshot through a failed poll and reports an error only
  // on a cold start. This card must say so the moment the worker stops answering (a
  // phone that left the LAN with the app open), so the failure is recorded here. The
  // callback is stable: the hook restarts its polling when the fetcher's identity changes.
  const [pollError, setPollError] = React.useState<string | null>(null);
  const fetchLatest = React.useCallback(async () => {
    try {
      const latest = await api.latestSnapshot<{ derived_metrics?: DerivedMetrics }>();
      setPollError(null);
      return latest;
    } catch (e) {
      setPollError(describeApiError(e));
      throw e;
    }
  }, []);

  const { snapshot, loading, error, freshness, refetch } = useFreshness<{ derived_metrics?: DerivedMetrics }>(
    fetchLatest,
    { syncWindowSeconds, pollIntervalMs: 10_000 },
  );

  // A home-screen app is suspended in the background and its timers stop. Coming back
  // to the foreground reads the worker at once instead of waiting for the next tick.
  React.useEffect(() => {
    const onVisible = () => {
      if (document.visibilityState === 'visible') refetch();
    };
    document.addEventListener('visibilitychange', onVisible);
    return () => document.removeEventListener('visibilitychange', onVisible);
  }, [refetch]);

  const metrics = snapshot?.data?.derived_metrics;
  // Age and status as of this render, not as of the hook's last tick.
  const now = freshnessAt(snapshot?.meta?.generated_at, Date.now(), syncWindowSeconds, freshness.syncGraceSeconds);
  const view = liteStatusView({
    hasSnapshot: !!snapshot,
    loading,
    error: error ? describeApiError(error) : null,
    pollError,
    syncStatus: now?.syncStatus ?? freshness.syncStatus,
    ageSeconds: now?.ageSeconds ?? freshness.ageSeconds,
    action: metrics?.global_status?.shot_advice_action,
    details: metrics?.global_status?.shot_advice_details,
  });
  const color = LITE_TONE_COLOR[view.tone];

  return (
    <>
      {/* The top of the page (#169): every top-tier call to action, above the directive,
          so a push about one lands on its card. */}
      {snapshot ? (
        <TopOfPageLiteEvents data={snapshot.data} syncStatus={now?.syncStatus ?? freshness.syncStatus} cardStyle={card} labelStyle={label} />
      ) : null}
      <section style={{ ...card, borderLeft: `3px solid ${color}` }}>
        {/* The anchors of every event the strip above does not render (#169). */}
        {snapshot ? <EventAnchors events={eventsOf(snapshot.data)} /> : null}
        <div style={label}>DIRECTIVE</div>
        <div style={{ fontSize: 16, fontWeight: 700, color: '#f0f6fc', lineHeight: 1.4, marginBottom: 6 }}>{view.action}</div>
        {view.details ? <div style={{ ...sub, color: '#c9d1d9' }}>{view.details}</div> : null}
        <div role="status" aria-live="polite" style={{ ...sub, color, marginTop: 8 }}>
          {view.freshness}
        </div>
      </section>
      {/* The ladder suggestion (#168): right under the directive, the same headline,
          outcomes and deposits as the dashboard. */}
      {snapshot && ladderSuggestionAtTop(ladderSuggestionOf(snapshot.data)) ? (
        <LadderSuggestionLiteCard
          suggestion={ladderSuggestionOf(snapshot.data)}
          syncStatus={now?.syncStatus ?? freshness.syncStatus}
          cardStyle={card}
          labelStyle={label}
        />
      ) : null}
      {/* Found on-chain (v0.9.0): the same block as the dashboard's, with the same
          one-tap buttons, right under the ladder card. */}
      {snapshot && positionDetectionOf(snapshot.data) && foundNeedsAttention(positionDetectionOf(snapshot.data)!) ? (
        <FoundPositionsLiteCard detection={positionDetectionOf(snapshot.data)} onChanged={refetch} cardStyle={card} labelStyle={label} />
      ) : null}
      {/* The WETH/NOCK bet (#181): the full card while a bet is suggested or a found bet
          waits for a tap; otherwise one quiet line. */}
      {snapshot && betSuggestionOf(snapshot.data) ? (
        <section style={{ ...card, minWidth: 0, overflowWrap: 'anywhere' }} data-testid="bet-lite-section">
          <BetLiteCard bet={betSuggestionOf(snapshot.data)} syncStatus={now?.syncStatus ?? freshness.syncStatus} onChanged={refetch} />
        </section>
      ) : null}
      {/* The ladder's shape and its quiet lines (#169): nothing to decide, so not at the top. */}
      {snapshot && (ladderProfileOf(snapshot.data) || !ladderSuggestionAtTop(ladderSuggestionOf(snapshot.data))) ? (
        <section style={{ ...card, minWidth: 0, overflowWrap: 'anywhere' }} data-testid="ladder-lite-section">
          <LadderShapeChart data={snapshot.data} width={340} compact />
          <LadderDeployedLine suggestion={ladderSuggestionOf(snapshot.data)} syncStatus={now?.syncStatus ?? freshness.syncStatus} compact />
          <FoundQuietLine detection={positionDetectionOf(snapshot.data)} onChanged={refetch} compact />
        </section>
      ) : null}
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
