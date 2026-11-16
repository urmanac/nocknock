/**
 * Notification routes (#89). Kept apart from client.ts so the two calls can be tested
 * with a stand-in fetch. Both are network-only: `cache: 'no-store'`.
 */
import type { NotificationStatus, NotificationTestResult } from '../notifications';

const API_BASE = import.meta.env.VITE_API_BASE || '/v1';

type Fetch = (input: string, init?: RequestInit) => Promise<Response>;

async function call<T>(doFetch: Fetch, path: string, method: 'GET' | 'POST'): Promise<T> {
  const res = await doFetch(`${API_BASE}${path}`, { method, cache: 'no-store' });
  if (!res.ok) {
    const body = await res.json().catch(() => null);
    const message = (body && (body.message || body.error)) || res.statusText || `HTTP ${res.status}`;
    throw Object.assign(new Error(message), { status: res.status });
  }
  return res.json();
}

export const notificationsApi = {
  status: (doFetch: Fetch = fetch) => call<NotificationStatus>(doFetch, '/notifications/status', 'GET'),
  sendTest: (doFetch: Fetch = fetch) => call<NotificationTestResult>(doFetch, '/notifications/test', 'POST'),
};
