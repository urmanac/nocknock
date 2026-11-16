import { describe, it, expect } from 'vitest';
import {
  channelLine,
  channelTone,
  describeNotificationError,
  statusSummary,
  testOutcome,
  type ChannelReport,
  type NotificationStatus,
  type NotificationTestResult,
} from './notifications';
import { notificationsApi } from './api/notifications';

const ch = (channel: string, status: string, detail = ''): ChannelReport => ({ channel, status, detail });

const status = (channels: ChannelReport[]): NotificationStatus => ({
  any_configured: channels.some((c) => c.status === 'configured'),
  channels,
});

// The worker's own response shapes (ingestion-worker/src/notify.rs).
const NONE = status([
  ch('telegram', 'not_configured', 'notifications.telegram_bot_token and notifications.telegram_chat_id are not set'),
  ch('pushover', 'not_configured', 'notifications.pushover_token and notifications.pushover_user are not set'),
  ch('webhook', 'not_configured', 'notifications.webhook_url is not set'),
]);

describe('status summary', () => {
  it('says plainly when nothing is configured', () => {
    const s = statusSummary(NONE);
    expect(s.text).toContain('No notification channel is configured, so nothing is pushed.');
    expect(s.tone).toBe('muted');
  });

  it('names the configured channels', () => {
    expect(statusSummary(status([ch('telegram', 'configured'), ch('pushover', 'not_configured'), ch('webhook', 'not_configured')])).text).toBe(
      'Telegram is configured.',
    );
    const three = statusSummary(status([ch('telegram', 'configured'), ch('pushover', 'configured'), ch('webhook', 'configured')]));
    expect(three.text).toBe('Telegram, Pushover and Webhook are configured.');
    expect(three.tone).toBe('ok');
  });

  it('warns about half a channel instead of hiding it', () => {
    const half = status([
      ch('telegram', 'incomplete', 'notifications.telegram_bot_token is set but notifications.telegram_chat_id is not, so nothing is sent'),
      ch('pushover', 'not_configured'),
      ch('webhook', 'not_configured'),
    ]);
    expect(statusSummary(half)).toEqual({ text: 'No notification channel is complete, so nothing is pushed.', tone: 'warn' });
    expect(channelLine(half.channels[0])).toBe(
      'Telegram: incomplete (notifications.telegram_bot_token is set but notifications.telegram_chat_id is not, so nothing is sent)',
    );
    const mixed = status([ch('telegram', 'configured'), ch('pushover', 'incomplete', 'x'), ch('webhook', 'not_configured')]);
    expect(statusSummary(mixed).tone).toBe('warn');
  });
});

describe('channel lines', () => {
  it('shows the reason for a failure and keeps quiet otherwise', () => {
    expect(channelLine(ch('telegram', 'delivered', 'accepted by Telegram (HTTP 200)'))).toBe('Telegram: accepted');
    expect(channelLine(ch('pushover', 'failed', 'HTTP 400: application token is invalid'))).toBe(
      'Pushover: FAILED (HTTP 400: application token is invalid)',
    );
    expect(channelLine(ch('webhook', 'not_configured', 'notifications.webhook_url is not set'))).toBe('Webhook: not configured');
    expect(channelLine(ch('carrier-pigeon', 'mystery'))).toBe('carrier-pigeon: mystery');
  });

  it('colours by outcome', () => {
    expect(channelTone('delivered')).toBe('ok');
    expect(channelTone('configured')).toBe('ok');
    expect(channelTone('failed')).toBe('bad');
    expect(channelTone('incomplete')).toBe('warn');
    expect(channelTone('not_configured')).toBe('muted');
  });
});

describe('test outcome', () => {
  const result = (channels: ChannelReport[], sent = true): NotificationTestResult => ({
    sent,
    any_configured: sent,
    message: sent ? 'n of m' : 'No notification channel is configured, so nothing was sent.',
    channels,
  });

  it('never calls an accepted message "received"', () => {
    const o = testOutcome(result([ch('telegram', 'delivered'), ch('pushover', 'not_configured'), ch('webhook', 'not_configured')]));
    expect(o).toEqual({ headline: 'Test accepted by Telegram. Check that it reached your phone.', tone: 'ok' });
  });

  it('reports a partial failure as a warning and a total failure as a failure', () => {
    const partial = testOutcome(result([ch('telegram', 'delivered'), ch('pushover', 'failed', 'HTTP 400'), ch('webhook', 'not_configured')]));
    expect(partial).toEqual({ headline: 'Test accepted by 1 of 2 channels; Pushover failed.', tone: 'warn' });
    const total = testOutcome(result([ch('telegram', 'failed', 'HTTP 401: Unauthorized'), ch('pushover', 'not_configured'), ch('webhook', 'not_configured')]));
    expect(total).toEqual({ headline: 'Test failed on every configured channel. Nothing reached your phone.', tone: 'bad' });
  });

  it('passes on the worker\'s words when nothing was sent', () => {
    const o = testOutcome(result(NONE.channels, false));
    expect(o.headline).toBe('No notification channel is configured, so nothing was sent.');
    expect(o.tone).toBe('muted');
  });
});

describe('request errors', () => {
  it('tells an unreachable worker from an answer', () => {
    expect(describeNotificationError(new TypeError('Failed to fetch'), true)).toBe('Worker unreachable: Failed to fetch. Nothing was sent.');
    const limited = Object.assign(new Error('A test notification was sent moments ago. Try again in 18s.'), { status: 429 });
    expect(describeNotificationError(limited, true)).toBe('A test notification was sent moments ago. Try again in 18s.');
    expect(describeNotificationError(Object.assign(new Error('Not Found'), { status: 404 }), true)).toBe(
      'This worker has no notification routes (it is older than the dashboard). Nothing was sent.',
    );
    expect(describeNotificationError(Object.assign(new Error('boom'), { status: 500 }), true)).toBe(
      'The worker answered HTTP 500: boom. Nothing was sent.',
    );
  });

  it('does not talk about sending when only the status was asked for', () => {
    expect(describeNotificationError(new TypeError('Failed to fetch'), false)).toBe('Worker unreachable: Failed to fetch.');
    expect(describeNotificationError(Object.assign(new Error('Not Found'), { status: 404 }), false)).toBe(
      'This worker has no notification routes (it is older than the dashboard).',
    );
  });
});

describe('api calls', () => {
  const respond = (status: number, body: unknown) =>
    ({ ok: status >= 200 && status < 300, status, statusText: 'x', json: async () => body }) as Response;

  it('GETs status and POSTs the test, never from a cache', async () => {
    const calls: Array<[string, RequestInit | undefined]> = [];
    const fake = async (input: string, init?: RequestInit) => {
      calls.push([input, init]);
      return respond(200, input.endsWith('/status') ? NONE : { sent: false, any_configured: false, message: 'm', channels: [] });
    };
    expect((await notificationsApi.status(fake)).any_configured).toBe(false);
    expect((await notificationsApi.sendTest(fake)).sent).toBe(false);
    expect(calls.map(([url, init]) => [url, init?.method, init?.cache])).toEqual([
      ['/v1/notifications/status', 'GET', 'no-store'],
      ['/v1/notifications/test', 'POST', 'no-store'],
    ]);
  });

  it('carries the HTTP status and the worker\'s message on a refusal', async () => {
    const fake = async () => respond(429, { error: 'RATE_LIMITED', message: 'Try again in 18s.', retry_after_seconds: 18 });
    await expect(notificationsApi.sendTest(fake)).rejects.toMatchObject({ status: 429, message: 'Try again in 18s.' });
  });
});
