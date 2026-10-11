/**
 * Contract Tests — Freshness Validation
 *
 * These tests verify the frontend ↔ backend freshness contract by hitting
 * a real (or test-mode) ingestion worker and asserting:
 * 1. Snapshot envelope contains required meta fields
 * 2. state_hash is deterministic (same data = same hash)
 * 3. Freshness computation matches backend's sync window
 * 4. Conditional GET (ETag/If-None-Match) returns 304
 * 5. Stale data indicator logic works correctly
 */

import { describe, it, expect, beforeAll } from 'vitest';

declare const process: { env: Record<string, string | undefined> };

// Test configuration
const API_BASE = process.env.VITE_API_BASE || 'http://localhost:8080/v1';
const SYNC_WINDOW_SECONDS = 300; // Must match backend config

interface SnapshotMeta {
  generated_at: string;
  state_hash: string;
  version: string;
  snapshot_id: string;
}

interface SnapshotEnvelope<T = unknown> {
  meta: SnapshotMeta;
  data: T;
}

async function fetchLatest(): Promise<SnapshotEnvelope | null> {
  const res = await fetch(`${API_BASE}/snapshot/latest`);
  if (res.status === 503) return null;
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

async function fetchMetadata(): Promise<{ snapshots: Array<{ generated_at: string; state_hash: string; age_seconds: number }> }> {
  const res = await fetch(`${API_BASE}/snapshot/metadata?limit=5`);
  if (!res.ok) throw new Error(`HTTP ${res.status}`);
  return res.json();
}

describe('Freshness Contract', () => {
  let latestSnapshot: SnapshotEnvelope | null = null;

  beforeAll(async () => {
    // Wait for backend to be ready (test mode starts worker on :8080)
    for (let i = 0; i < 30; i++) {
      try {
        const health = await fetch(`${API_BASE.replace('/v1', '')}/health`);
        if (health.ok) break;
      } catch {}
      await new Promise(r => setTimeout(r, 1000));
    }

    latestSnapshot = await fetchLatest();
    if (!latestSnapshot) {
      throw new Error('No snapshot available — ensure ingestion worker ran at least once');
    }
  });

  it('returns valid snapshot envelope with required meta fields', () => {
    expect(latestSnapshot).not.toBeNull();
    expect(latestSnapshot!.meta).toBeDefined();
    expect(latestSnapshot!.meta.generated_at).toMatch(/^\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}/);
    expect(latestSnapshot!.meta.state_hash).toMatch(/^[a-f0-9]{64}$/);
    expect(latestSnapshot!.meta.version).toMatch(/^\d+\.\d+\.\d+$/);
    expect(latestSnapshot!.meta.snapshot_id).toMatch(/^([0-9A-Z]{26}|[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12})$/); // ULID
    expect(latestSnapshot!.data).toBeDefined();
  });

  it('state_hash is deterministic for identical data', async () => {
    const snap1 = await fetchLatest();
    const snap2 = await fetchLatest();

    expect(snap1).not.toBeNull();
    expect(snap2).not.toBeNull();

    // If no new ingestion ran between fetches, hashes should match
    // (In test mode with fixed time, they should be identical)
    if (snap1!.meta.snapshot_id === snap2!.meta.snapshot_id) {
      expect(snap1!.meta.state_hash).toBe(snap2!.meta.state_hash);
    }
  });

  it('ETag header matches state_hash', async () => {
    const res = await fetch(`${API_BASE}/snapshot/latest`);
    expect(res.headers.get('ETag')).toBe(latestSnapshot!.meta.state_hash);
  });

  it('conditional GET with matching ETag returns 304', async () => {
    const res = await fetch(`${API_BASE}/snapshot/latest`, {
      headers: { 'If-None-Match': latestSnapshot!.meta.state_hash },
    });
    expect(res.status).toBe(304);
  });

  it('conditional GET with non-matching ETag returns 200 with new data', async () => {
    const res = await fetch(`${API_BASE}/snapshot/latest`, {
      headers: { 'If-None-Match': '0000000000000000000000000000000000000000000000000000000000000000' },
    });
    expect(res.status).toBe(200);
    const data = await res.json();
    expect(data.meta.state_hash).toBe(latestSnapshot!.meta.state_hash);
  });

  it('freshness computed from generated_at matches sync window', () => {
    const generatedAt = new Date(latestSnapshot!.meta.generated_at).getTime();
    const now = Date.now();
    const ageSeconds = Math.floor((now - generatedAt) / 1000);

    // In test mode, ingestion runs just before test, so should be fresh
    expect(ageSeconds).toBeLessThanOrEqual(SYNC_WINDOW_SECONDS + 10); // small buffer
  });

  it('metadata endpoint returns age_seconds for freshness polling', async () => {
    const meta = await fetchMetadata();
    expect(meta.snapshots.length).toBeGreaterThan(0);

    const latest = meta.snapshots[0];
    expect(latest.state_hash).toBe(latestSnapshot!.meta.state_hash);
    expect(latest.age_seconds).toBeGreaterThanOrEqual(0);
    expect(typeof latest.age_seconds).toBe('number');
  });
});

describe('Stale Data Detection Logic', () => {
  // Pure logic tests — no backend needed
  const computeFreshness = (generatedAt: string, now: number, windowSec: number) => {
    const age = Math.floor((now - new Date(generatedAt).getTime()) / 1000);
    return { isFresh: age <= windowSec, ageSeconds: age };
  };

  it('data within window is fresh', () => {
    const now = Date.now();
    const generated = new Date(now - 60_000).toISOString(); // 1 minute ago
    expect(computeFreshness(generated, now, 300).isFresh).toBe(true);
  });

  it('data at window boundary is fresh', () => {
    const now = Date.now();
    const generated = new Date(now - 300_000).toISOString(); // exactly 5 min ago
    expect(computeFreshness(generated, now, 300).isFresh).toBe(true);
  });

  it('data just past window is stale', () => {
    const now = Date.now();
    const generated = new Date(now - 301_000).toISOString(); // 5 min 1 sec ago
    expect(computeFreshness(generated, now, 300).isFresh).toBe(false);
  });

  it('age calculation is correct', () => {
    const now = Date.now();
    const generated = new Date(now - 125_000).toISOString(); // 2 min 5 sec ago
    expect(computeFreshness(generated, now, 300).ageSeconds).toBe(125);
  });

  it('future timestamps are treated as fresh (clock skew tolerant)', () => {
    const now = Date.now();
    const generated = new Date(now + 10_000).toISOString(); // 10 sec in future
    expect(computeFreshness(generated, now, 300).isFresh).toBe(true);
  });
});