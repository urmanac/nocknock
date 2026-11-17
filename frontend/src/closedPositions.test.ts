import { describe, expect, it } from 'vitest';
import { existsSync, readFileSync } from 'node:fs';
import {
  CLOSED_NOT_COUNTED,
  closedFirstSeenText,
  closedPositionConfigHint,
  closedPositionFact,
  closedPositionLine,
  closedPositionName,
  closedPositionsHeader,
  closedPositionsOf,
  priceSourceIsClosedText,
} from './closedPositions';
import { positionsSummary } from './positions';
import {
  OLD_DATA,
  V4_SOURCE_CLOSED,
  V4_SOURCE_DATA,
  V4_SOURCE_POSITIONS,
  WITHDRAWN_CLOSED,
  WITHDRAWN_DATA,
  WITHDRAWN_GLOBAL,
  WITHDRAWN_POSITIONS,
} from './closedPositions.fixture';

// The worker's published values for the real wallet at Base block 58646285
// (closedPositions.fixture.ts): Aero CL NFT #2914490 was withdrawn in block 58637717.

describe('closed positions (#144)', () => {
  it('the fixture is the worker golden file, entry for entry', () => {
    const golden = new URL('../../ingestion-worker/testdata/golden_snapshot_closed.json', import.meta.url);
    // A frontend-only checkout has no worker directory; the copy is then all there is.
    if (!existsSync(golden)) return;
    const doc = JSON.parse(readFileSync(golden, 'utf8'));
    const withdrawn = doc.aero_cl_withdrawn_block_58646285.snapshot;
    expect(WITHDRAWN_CLOSED).toEqual(withdrawn.closed_positions);
    expect(WITHDRAWN_POSITIONS.map((p) => p.position_id)).toEqual(withdrawn.positions.map((p: any) => p.position_id));
    for (const [k, v] of Object.entries(WITHDRAWN_GLOBAL)) {
      expect(withdrawn.derived_metrics.global_status[k], k).toEqual(v);
    }
    expect(V4_SOURCE_CLOSED).toEqual(doc.v4_price_source_closed_block_58646285.snapshot.closed_positions);
  });

  it('states the fact, that it is in no total, and that the entry can be removed', () => {
    const closed = closedPositionsOf(WITHDRAWN_DATA);
    expect(closed).toHaveLength(1);
    const c = closed[0];
    expect(closedPositionsHeader(closed)).toBe('CLOSED POSITIONS (1)');
    expect(closedPositionName(c)).toBe('Aero CL #2914490 (aero-cl200-weth-nock)');
    expect(closedPositionFact(c)).toBe('closed: NFT #2914490 no longer exists (withdrawn and burned, or a mistyped nft_id)');
    expect(CLOSED_NOT_COUNTED).toBe('not counted in any total');
    expect(closedPositionConfigHint(c)).toBe('you can remove its entry (aero-cl200-weth-nock) from config.yaml');
    expect(closedPositionLine(c)).toBe(
      'Aero CL #2914490 (aero-cl200-weth-nock): closed: NFT #2914490 no longer exists (withdrawn and burned, or a mistyped nft_id); not counted in any total; you can remove its entry (aero-cl200-weth-nock) from config.yaml',
    );
    expect(closedFirstSeenText(c)).toBe('first recorded as closed 2026-11-17T13:00:00Z');
    // Nothing printed for a closed position is an amount or a value.
    expect(closedPositionLine(c)).not.toMatch(/\$|USD|NOCK \d|WETH/);
  });

  it('a closed position is not a live one: it is never in the tracked positions', () => {
    // Every consumer of `positions` (cards, totals, charts, the summary line) sees the
    // three live positions only.
    expect(WITHDRAWN_DATA.positions.map((p) => p.position_id)).toEqual([
      'uniswap-v4:base:3163450',
      'uniswap-v3:base:6336662',
      'aerodrome:base:0x770fe8fc9248c6382950a86e39a9391b41d59e0a',
    ]);
    expect(WITHDRAWN_DATA.positions.some((p) => p.nft_id === 2914490)).toBe(false);
    expect(positionsSummary(WITHDRAWN_DATA.positions)).toBe(positionsSummary(OLD_DATA.positions));
    expect(positionsSummary(WITHDRAWN_DATA.positions)).toMatch(/^3 LP positions tracked live/);
    // The worker's count and sentence agree with the list.
    expect(WITHDRAWN_GLOBAL.closed_positions_count).toBe(closedPositionsOf(WITHDRAWN_DATA).length);
    expect(WITHDRAWN_GLOBAL.closed_positions_note).toContain('is not counted in any total');
  });

  it('an older snapshot, and one with nothing closed, has no closed positions', () => {
    expect(closedPositionsOf(OLD_DATA)).toEqual([]);
    expect(closedPositionsOf(null)).toEqual([]);
    expect(closedPositionsOf(undefined)).toEqual([]);
    expect(closedPositionsOf({})).toEqual([]);
    expect(closedPositionsOf({ closed_positions: [] })).toEqual([]);
    expect(closedPositionsOf({ closed_positions: null })).toEqual([]);
    expect(closedPositionsOf({ closed_positions: 'x' })).toEqual([]);
    expect(closedPositionsOf({ closed_positions: [null, 3] })).toEqual([]);
    expect(priceSourceIsClosedText(OLD_DATA, OLD_DATA.positions)).toBeNull();
  });

  it('a price source whose pool still supplies the spot must be replaced before its entry is removed', () => {
    const c = closedPositionsOf(V4_SOURCE_DATA)[0];
    expect(c.supplies_spot_price).toBe(true);
    expect(closedPositionName(c)).toBe('v4 USDC/NOCK #3229273 (uniswap-v4-nock-usdc-gone)');
    expect(closedPositionConfigHint(c)).toBe(
      'its pool still supplies the NOCK/USD spot: flag another position as is_price_source before removing its entry from config.yaml',
    );
    expect(closedPositionConfigHint(c)).not.toContain('you can remove');
    // No live position is the price source: the chart states that, not a range.
    expect(V4_SOURCE_POSITIONS.some((p) => p.is_price_source)).toBe(false);
    expect(priceSourceIsClosedText(V4_SOURCE_DATA, V4_SOURCE_POSITIONS)).toBe(
      'Price source: the pool of v4 USDC/NOCK #3229273 (uniswap-v4-nock-usdc-gone), a closed position; it has no range',
    );
    // With a live price source the line is the chart's usual one.
    expect(priceSourceIsClosedText(WITHDRAWN_DATA, WITHDRAWN_POSITIONS)).toBeNull();
    expect(priceSourceIsClosedText(V4_SOURCE_DATA, WITHDRAWN_POSITIONS)).toBeNull();
  });

  it('degrades without inventing anything when fields are missing', () => {
    expect(closedPositionName({})).toBe('Position');
    expect(closedPositionName({ config_id: 'x' })).toBe('x');
    expect(closedPositionFact({})).toBe('closed: its NFT no longer exists (withdrawn and burned, or a mistyped nft_id)');
    expect(closedPositionConfigHint({})).toBe('you can remove its entry from config.yaml');
    expect(closedFirstSeenText({})).toBeNull();
    expect(closedFirstSeenText({ first_seen_closed_at: null })).toBeNull();
    expect(closedFirstSeenText({ first_seen_closed_at: 'not a time' })).toBeNull();
  });
});
