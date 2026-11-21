import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { LadderLiteCard, LaddersCard } from './Ladders';
import { RangeSideList } from './PositionRangeNotes';
import { LADDERS_ABOVE, LADDERS_BOUGHT, POSITIONS_ABOVE } from '../ladderTrack.fixture';

// The worker's golden readings (ladderTrack.fixture.ts).

describe('Ladders components (#167)', () => {
  it('the dashboard card states each ladder, its state and its readings', () => {
    const html = renderToStaticMarkup(<LaddersCard data={{ positions: POSITIONS_ABOVE, ladders: LADDERS_BOUGHT }} />);
    expect(html).toContain('data-testid="ladders"');
    expect(html).toContain('LADDERS (MEASURED, NOT NOCK LP) (2)');
    expect(html).toContain('Counted in the totals, the protected reserve and the floor-crossing price; not in the LP target or the rebuild.');
    expect(html).toContain('v4 USDC/NOCK 3.00% #3240369');
    expect(html).toContain('rung near');
    expect(html).toContain('● LADDER IN RANGE (CONVERTS BOTH WAYS)');
    expect(html).toContain('Holds $1137.51 USDC + 5551.3674 NOCK ($1219.69)');
    expect(html).toContain('Since deposit (tx 0xafe42fe2…, block 59660587): net 5303.7885 NOCK bought for net $84.35 USDC spent, average $0.01590283');
    // The deep rung has no baseline: the reason, muted, and no bought figure.
    expect(html).toContain('rung unset');
    expect(html).toContain('● LADDER ABOVE RANGE (ALL USDC)');
    expect(html).toContain('Since deposit: no baseline: no deposit_tx is set for this ladder in config.yaml');
    // Facts only: no control, no link, no instruction.
    expect(html).not.toMatch(/<button|<a |re-centre|withdraw it|should/i);
    expect(html).not.toContain('OUT OF RANGE');
  });

  it('the Lite card is one muted line per ladder', () => {
    const html = renderToStaticMarkup(<LadderLiteCard data={{ ladders: LADDERS_ABOVE }} />);
    expect(html).toContain('data-testid="ladders-lite"');
    expect(html).toContain('Ladders (measured, not NOCK LP)');
    expect(html).toContain('v4 USDC/NOCK 3.00% #3240369: above range, all USDC · $1,226.09 USDC + 0 NOCK');
    expect(html).toContain('v4 USDC/NOCK 3.00% #3299001: above range, all USDC · $77.00 USDC + 0 NOCK');
    expect(html).not.toMatch(/<button|<a /);
  });

  it('a snapshot with no ladder renders nothing', () => {
    for (const data of [null, {}, { ladders: [] }, { positions: POSITIONS_ABOVE.slice(0, 2) }]) {
      expect(renderToStaticMarkup(<LaddersCard data={data} />)).toBe('');
      expect(renderToStaticMarkup(<LadderLiteCard data={data} />)).toBe('');
    }
  });
});

describe('RangeSideList with tracked ladders (#167 review)', () => {
  it('states a ladder by what it holds, in the neutral tone, never as out of range', () => {
    const html = renderToStaticMarkup(
      <RangeSideList positions={POSITIONS_ABOVE} poolMechanics={POSITIONS_ABOVE.map(() => ({ tick_current: 269065 }))} ladders={LADDERS_ABOVE} />,
    );
    expect(html).toContain(
      'v4 · 3%: ladder, holds only USDC: the price is above the range, so it converts nothing until NOCK falls into it at $0.01032932',
    );
    // Neither ladder is called out of range or said to have sold NOCK, and neither is amber.
    const ladderSpans = html.split('<span').filter((s) => s.includes('ladder,'));
    expect(ladderSpans).toHaveLength(2);
    for (const s of ladderSpans) {
      expect(s).not.toMatch(/out of range|sold all its NOCK/);
      expect(s).toContain('color:#94a3b8');
      expect(s).not.toContain('#f59e0b');
    }
    // Without a reading the ladder is still neutral and says the state is not reported.
    const bare = renderToStaticMarkup(<RangeSideList positions={[POSITIONS_ABOVE[3]]} poolMechanics={[{ tick_current: 269065 }]} />);
    expect(bare).toContain('ladder, state not reported');
    expect(bare).not.toMatch(/out of range|#f59e0b/);
  });
});
