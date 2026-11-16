/**
 * Notification display helpers (#89). Pure functions over what the worker reports from
 * GET /v1/notifications/status and POST /v1/notifications/test. Nothing here guesses:
 * a channel is shown as delivered only when the worker says its provider accepted the
 * message, and an unreachable worker is shown as unreachable, never as "not configured".
 */

export type ChannelName = 'telegram' | 'pushover' | 'webhook';
export type ChannelStatus = 'configured' | 'incomplete' | 'not_configured' | 'delivered' | 'failed';

export interface ChannelReport {
  channel: ChannelName | string;
  status: ChannelStatus | string;
  detail: string;
}

export interface NotificationStatus {
  any_configured: boolean;
  channels: ChannelReport[];
  cockpit_url?: string | null;
  cockpit_url_note?: string | null;
  fires_on?: string;
  test_cooldown_seconds?: number;
}

export interface NotificationTestResult {
  sent: boolean;
  any_configured: boolean;
  message: string;
  sent_at?: string;
  text?: string;
  channels: ChannelReport[];
}

export type Tone = 'ok' | 'warn' | 'bad' | 'muted';

export const TONE_COLOR: Record<Tone, string> = {
  ok: '#2ecc71',
  warn: '#f59e0b',
  bad: '#ef4444',
  muted: '#8b949e',
};

const CHANNEL_LABEL: Record<string, string> = {
  telegram: 'Telegram',
  pushover: 'Pushover',
  webhook: 'Webhook',
};

export function channelLabel(channel: string): string {
  return CHANNEL_LABEL[channel] ?? channel;
}

const STATUS_WORD: Record<string, string> = {
  configured: 'configured',
  incomplete: 'incomplete',
  not_configured: 'not configured',
  delivered: 'accepted',
  failed: 'FAILED',
};

export function channelTone(status: string): Tone {
  if (status === 'delivered' || status === 'configured') return 'ok';
  if (status === 'failed') return 'bad';
  if (status === 'incomplete') return 'warn';
  return 'muted';
}

/** "Telegram: accepted" / "Pushover: FAILED (HTTP 400: application token is invalid)". */
export function channelLine(r: ChannelReport): string {
  const word = STATUS_WORD[r.status] ?? r.status;
  // A plain "not configured" needs no detail; everything else earns its reason.
  const showDetail = r.status === 'failed' || r.status === 'incomplete';
  return `${channelLabel(r.channel)}: ${word}${showDetail && r.detail ? ` (${r.detail})` : ''}`;
}

/** One sentence on what is set up, from the status response. */
export function statusSummary(status: NotificationStatus): { text: string; tone: Tone } {
  const ready = status.channels.filter((c) => c.status === 'configured').map((c) => channelLabel(c.channel));
  const incomplete = status.channels.filter((c) => c.status === 'incomplete');
  if (ready.length === 0) {
    return {
      text:
        incomplete.length > 0
          ? 'No notification channel is complete, so nothing is pushed.'
          : 'No notification channel is configured, so nothing is pushed. Add a notifications block to config.yaml and redeploy.',
      tone: incomplete.length > 0 ? 'warn' : 'muted',
    };
  }
  const list = ready.length === 1 ? ready[0] : `${ready.slice(0, -1).join(', ')} and ${ready[ready.length - 1]}`;
  return {
    text: `${list} ${ready.length === 1 ? 'is' : 'are'} configured.`,
    tone: incomplete.length > 0 ? 'warn' : 'ok',
  };
}

/** Headline for a test send. "Accepted" is the provider's word, not the phone's. */
export function testOutcome(result: NotificationTestResult): { headline: string; tone: Tone } {
  if (!result.sent) return { headline: result.message, tone: 'muted' };
  const tried = result.channels.filter((c) => c.status === 'delivered' || c.status === 'failed');
  const failed = tried.filter((c) => c.status === 'failed');
  if (tried.length === 0) return { headline: result.message, tone: 'warn' };
  if (failed.length === 0) {
    return {
      headline: `Test accepted by ${tried.map((c) => channelLabel(c.channel)).join(' and ')}. Check that it reached your phone.`,
      tone: 'ok',
    };
  }
  if (failed.length === tried.length) {
    return { headline: 'Test failed on every configured channel. Nothing reached your phone.', tone: 'bad' };
  }
  return {
    headline: `Test accepted by ${tried.length - failed.length} of ${tried.length} channels; ${failed
      .map((c) => channelLabel(c.channel))
      .join(' and ')} failed.`,
    tone: 'warn',
  };
}

/**
 * What to show when the status or test request itself failed. `sending` is true for the
 * test send, where it matters to say that nothing went out.
 */
export function describeNotificationError(e: unknown, sending: boolean): string {
  const msg = e instanceof Error ? e.message : String(e);
  const status = (e as { status?: unknown } | null)?.status;
  const tail = sending ? ' Nothing was sent.' : '';
  if (typeof status !== 'number') return `Worker unreachable: ${msg}.${tail}`;
  if (status === 429) return msg;
  if (status === 404) {
    return `This worker has no notification routes (it is older than the dashboard).${tail}`;
  }
  return `The worker answered HTTP ${status}: ${msg}.${tail}`;
}
