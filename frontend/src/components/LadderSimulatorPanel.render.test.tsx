import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { LadderResult, LadderSimulatorPanel } from './LadderSimulatorPanel';
import { CHAT_900_UNSNAPPED, NOCK_TOO_MUCH, SPOT_741_SNAPPED } from '../ladder.fixture';
import type { Position } from '../types/snapshot';

describe('LadderSimulatorPanel (#166)', () => {
  it('renders an empty form: no amount is prefilled, and it says it is a what-if', () => {
    const positions = [
      {
        position_id: 'aerodrome:base:0x770f',
        protocol: 'aerodrome',
        nft_id: 0,
        pool_id: '0x770fe8fc92',
        full_range: true,
        token0: { symbol: 'USDC' },
        token1: { symbol: 'NOCK' },
      },
    ] as unknown as Position[];
    const html = renderToStaticMarkup(<LadderSimulatorPanel positions={positions} snapshotId="s1" />);
    expect(html).toContain('WHAT-IF ONLY · FROM THE STORED SNAPSHOT · NOTHING IS READ, SIGNED OR SENT');
    for (const name of ['Ladder USDC', 'Ladder NOCK', 'Ladder bottom']) {
      expect(html).toMatch(new RegExp(`aria-label="${name}"[^>]*value=""`));
    }
    // The basic Aero pool is listed, disabled, with its reason.
    expect(html).toMatch(/<option value="aerodrome:base:0x770f" disabled=""[^>]*>[^<]*a basic Aero pool has no ranges/);
    expect(html).not.toContain('data-testid="ladder-result"');
  });

  it('renders a result as measured lines with the snapshot age, and the grid', () => {
    const html = renderToStaticMarkup(<LadderResult outcome={{ ok: true, sim: SPOT_741_SNAPPED }} />);
    expect(html).toContain('From the snapshot generated 2026-11-20T12:00:00Z (5 min ago) · worker health: ok');
    expect(html).toContain('holds by $26.45');
    expect(html).toContain('-22.56% (bottom)');
    expect(html).toContain('$930.69');
    expect(html).not.toMatch(/<button|<a /);
  });

  it('shows a breached reserve as a measured line in red, with no imperative', () => {
    const html = renderToStaticMarkup(<LadderResult outcome={{ ok: true, sim: CHAT_900_UNSNAPPED }} />);
    expect(html).toMatch(/color:#f87171">Reserve: stable \$798\.63 at the bottom against the \$950\.00 reserve: breached: below it by \$151\.37/);
    expect(html.toLowerCase()).not.toMatch(/deposit|should|recommend|withdraw/);
  });

  it('shows a NOCK_TOO_MUCH answer with nock_needed as a measurement', () => {
    const html = renderToStaticMarkup(<LadderResult outcome={{ ok: false, status: 422, error: NOCK_TOO_MUCH }} />);
    expect(html).toContain('data-testid="ladder-error"');
    expect(html).toContain('NOCK_TOO_MUCH: this range at the pool price takes 0.0000 NOCK');
    expect(html).toContain('Measured: this range needs 0.0000 NOCK at the pool price for this USDC');
  });
});
