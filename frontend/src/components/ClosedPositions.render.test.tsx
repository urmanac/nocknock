import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { ClosedPositionsCard, ClosedPositionsLiteCard } from './ClosedPositions';
import { OLD_DATA, V4_SOURCE_DATA, WITHDRAWN_DATA } from '../closedPositions.fixture';

// The worker's published values for the real wallet at Base block 58646285
// (closedPositions.fixture.ts).

describe('ClosedPositions components (#144)', () => {
  it('the dashboard card states the fact beside the tracked positions', () => {
    const html = renderToStaticMarkup(<ClosedPositionsCard data={WITHDRAWN_DATA} />);
    expect(html).toContain('data-testid="closed-positions"');
    expect(html).toContain('CLOSED POSITIONS (1) · NOT COUNTED IN ANY TOTAL');
    expect(html).toContain('Aero CL #2914490 (aero-cl200-weth-nock)');
    expect(html).toContain('closed: NFT #2914490 no longer exists (withdrawn and burned, or a mistyped nft_id)');
    expect(html).toContain('not counted in any total; you can remove its entry (aero-cl200-weth-nock) from config.yaml');
    expect(html).toContain('first recorded as closed 2026-11-17T13:00:00Z');
    // It is a note, not a position card: no value, no range pill, no control, no link.
    expect(html).not.toMatch(/\$\d|IN RANGE|OUT OF RANGE|<button|<a /);
  });

  it('the Lite card states the same fact', () => {
    const html = renderToStaticMarkup(<ClosedPositionsLiteCard data={WITHDRAWN_DATA} />);
    expect(html).toContain('data-testid="closed-positions-lite"');
    expect(html).toContain('CLOSED POSITIONS (1)');
    expect(html).toContain('Aero CL #2914490 (aero-cl200-weth-nock)');
    expect(html).toContain('closed: NFT #2914490 no longer exists (withdrawn and burned, or a mistyped nft_id)');
    expect(html).toContain('not counted in any total; you can remove its entry (aero-cl200-weth-nock) from config.yaml');
    expect(html).not.toMatch(/\$\d|<button|<a /);
  });

  it('a price source that is closed says what to do first, on both cards', () => {
    for (const html of [
      renderToStaticMarkup(<ClosedPositionsCard data={V4_SOURCE_DATA} />),
      renderToStaticMarkup(<ClosedPositionsLiteCard data={V4_SOURCE_DATA} />),
    ]) {
      expect(html).toContain('v4 USDC/NOCK #3229273 (uniswap-v4-nock-usdc-gone)');
      expect(html).toContain('its pool still supplies the NOCK/USD spot: flag another position as is_price_source before removing its entry from config.yaml');
      expect(html).not.toContain('you can remove its entry');
    }
  });

  it('an older snapshot, a loading one and one with nothing closed render nothing at all', () => {
    for (const data of [OLD_DATA, null, undefined, {}, { closed_positions: [] }, { closed_positions: null }]) {
      expect(renderToStaticMarkup(<ClosedPositionsCard data={data} />)).toBe('');
      expect(renderToStaticMarkup(<ClosedPositionsLiteCard data={data as any} />)).toBe('');
    }
  });
});
