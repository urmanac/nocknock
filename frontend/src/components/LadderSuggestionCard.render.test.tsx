import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { LadderPicture, LadderSuggestionCard, LadderSuggestionLiteCard } from './LadderSuggestionCard';
import {
  LADDER_BOTTOM_ABOVE_TOP,
  LADDER_NEEDS_SETTING,
  LADDER_NO_TARGET_POOL,
  LADDER_PRICE_BELOW_BOTTOM,
  LADDER_READY,
  LADDER_READY_LIVE,
  LADDER_TRACKED,
} from '../ladderSuggestion.fixture';
import { pictureGeometry, type LadderSuggestion } from '../ladderSuggestion';

/** The operator's two-rung "bottle" as a fixture object: near 15% from the top down to
 *  $0.0110, deep 85% from $0.0110 down to $0.01. The card only lays it out. */
function bottle(): LadderSuggestion {
  const one = LADDER_READY_LIVE.rungs[0];
  const near = { ...one, of: 2, rung: 'near' as const, usdc: { ...one.usdc, amount: '114.88' }, bounds_usd: { low: '0.01100000', high: '0.01274292' } };
  const deep = { ...one, index: 2, of: 2, rung: 'deep' as const, usdc: { ...one.usdc, amount: '650.98' }, bounds_usd: { low: '0.01002405', high: '0.01100000' } };
  return {
    ...LADDER_READY_LIVE,
    headline: 'Ladder ready: up to $765.86 USDC in 2 deposits, 0 NOCK.',
    rungs: [near, deep],
    picture: {
      ...LADDER_READY_LIVE.picture!,
      bars: [
        { index: 1, low_usd: '0.01100000', high_usd: '0.01274292', usdc: '114.88', label: '$114.88' },
        { index: 2, low_usd: '0.01002405', high_usd: '0.01100000', usdc: '650.98', label: '$650.98' },
      ],
    },
  };
}

const NO_INPUT = /<input|<form|<select|<textarea|<button/;
const IMPERATIVE = /\b(should|recommend|you must|you need to|please|make sure)\b/i;

describe('LadderSuggestionCard (#168)', () => {
  it('ready: headline, picture, plain outcomes, the numbered deposit, stated lines', () => {
    const html = renderToStaticMarkup(<LadderSuggestionCard suggestion={LADDER_READY_LIVE} syncStatus="fresh" />);
    expect(html).toContain('data-testid="ladder-suggestion"');
    expect(html).toContain('Ladder ready: up to $765.86 USDC in 1 deposit, 0 NOCK.');
    expect(html).toContain('data-testid="ladder-picture"');
    expect(html).toContain('A fall to $0.01100 (-15%) would buy about 40,488 NOCK for $479.41 of the USDC, an average of $0.01184.');
    expect(html).toContain('A fall to the ladder bottom $0.01002 (-23%) would buy about 67,763 NOCK with all $765.86, an average of $0.01130.');
    expect(html).toContain('A rise sells nothing: the ladder holds 0 NOCK, so its USDC stays USDC.');
    expect(html).toContain('Deposit 1 of 1: $765.86 USDC + 0 NOCK');
    expect(html).toContain('78.4750 to 99.7600 NOCK per USDC');
    expect(html).toContain('ticks 273900 to 276300, a new position in the USDC/NOCK 3% v4 pool');
    expect(html).not.toContain('#3163450');
    expect(html).toContain('href="https://app.uniswap.org/positions/add/new?chain=base&amp;protocolVersion=v4');
    expect(html).not.toContain('depositState');
    expect(html).toContain('data-testid="ladder-amount-to-enter"');
    expect(html).toContain('Amount to enter in the app: $765.86 USDC');
    expect(html).toContain('OPEN IN UNISWAP ↗ (pool and range prefilled)');
    expect(html.match(/data-testid="ladder-link-note"/g)).toHaveLength(1);
    expect(html).toContain('check the pool and range in the app');
    expect(html).toContain('data-testid="ladder-rung-action"');
    // The stated lines and the assumptions, muted.
    expect(html).toContain('Plan budget not yet spent: $0.00 (stated, not a gate).');
    expect(html).toContain('Tax reserve: $8.70 (stated, not a gate).');
    expect(html).toMatch(/Cash target: \$[\d.]+ \(5% of the portfolio; stated, not a gate\)\./);
    expect(html).toContain('Assumptions: The amount is the most');
    // After a deposit nothing is typed: the worker finds the position (v0.9.0).
    expect(html).toContain('After you deposit there is nothing to copy or configure.');
    expect(html).not.toContain('deploy.sh');
    expect(html).not.toMatch(NO_INPUT);
    expect(html).not.toMatch(IMPERATIVE);
  });

  it('the picture: spot marker, a bar per rung with its amount, the bottom line and a reserve zone with its own legend', () => {
    const html = renderToStaticMarkup(<LadderPicture picture={LADDER_READY_LIVE.picture!} />);
    expect(html).toContain('data-testid="ladder-spot-marker"');
    expect(html).toContain('spot $0.01294');
    expect(html.match(/data-testid="ladder-bar"/g)).toHaveLength(1);
    expect(html).toContain('>$765.86</text>');
    expect(html).toContain('data-testid="ladder-bottom-line"');
    expect(html).toContain('bottom $0.01002');
    expect(html).toContain('data-testid="ladder-reserve-zone"');
    expect(html).toContain('$950 kept for below $0.01');
    // Readable labels: 12 units at least, and the reserve label sits below the axis, so it
    // never crosses the bottom line (which ends at the axis).
    expect(html).not.toMatch(/font-size="(?:[0-9]|1[01])"/);
    const g = pictureGeometry(LADDER_READY_LIVE.picture!, 340)!;
    expect(g.rows.legend).toBeGreaterThan(g.rows.axis);
    // At 375px the phone's 340-unit drawing is about 309px wide: 12 units ≈ 10.9px.
    expect((g.fontSize * 309) / 340).toBeGreaterThan(10.5);
  });

  it('two rungs render as a numbered list and two bars', () => {
    const html = renderToStaticMarkup(<LadderSuggestionCard suggestion={bottle()} syncStatus="fresh" rungAction={(r) => <span>slot {r.index}</span>} />);
    const a = html.indexOf('Deposit 1 of 2 (near rung): $114.88 USDC + 0 NOCK');
    const b = html.indexOf('Deposit 2 of 2 (deep rung): $650.98 USDC + 0 NOCK');
    expect(a).toBeGreaterThan(-1);
    expect(b).toBeGreaterThan(a);
    expect(html.match(/data-testid="ladder-bar"/g)).toHaveLength(2);
    expect(html).toContain('>$114.88</text>');
    expect(html).toContain('>$650.98</text>');
    expect(html).toContain('slot 1');
    expect(html).toContain('slot 2');
    expect(html).toContain('<ol');
    expect(html.match(/data-testid="ladder-link-note"/g)).toHaveLength(1);
  });

  it('stale: the ladder is not shown', () => {
    const html = renderToStaticMarkup(<LadderSuggestionCard suggestion={LADDER_READY} syncStatus="stale" />);
    expect(html).toContain('Ladder: not shown while the snapshot is stale.');
    expect(html).not.toContain('Deposit 1 of');
    expect(html).not.toContain('href=');
  });

  it('a tracked ladder: the slot says so and shows no ladder', () => {
    const html = renderToStaticMarkup(<LadderSuggestionCard suggestion={LADDER_TRACKED} syncStatus="fresh" />);
    expect(html).toContain('Ladder: tracked. No new one is suggested.');
    expect(html).toContain('A ladder is tracked');
    expect(html).not.toMatch(/Deposit 1 of|ladder-picture|href=/);
  });

  it('no bottom set: one plain sentence and the candidate list, no form', () => {
    const html = renderToStaticMarkup(<LadderSuggestionCard suggestion={LADDER_NEEDS_SETTING} syncStatus="fresh" />);
    expect(html).toContain('Ladder: needs your bottom price (operator_profile.ladder_bottom_usd) before one is measured.');
    expect(html).toContain('NockNock needs your bottom price, operator_profile.ladder_bottom_usd, in config.yaml; it does not choose these numbers.');
    expect(html).toContain('data-testid="ladder-candidates"');
    expect(html).not.toMatch(NO_INPUT);
  });

  it('only the reserve missing: no candidate bottoms', () => {
    const onlyReserve: LadderSuggestion = { ...LADDER_NEEDS_SETTING, missing: ['operator_profile.cash_reserve_usdc'], candidates: null };
    const html = renderToStaticMarkup(<LadderSuggestionCard suggestion={onlyReserve} syncStatus="fresh" />);
    expect(html).toContain('your protected reserve, operator_profile.cash_reserve_usdc');
    expect(html).not.toContain('Candidate bottoms');
  });

  it('each failing condition states its reason, with every excluded pool', () => {
    const below = renderToStaticMarkup(<LadderSuggestionCard suggestion={LADDER_PRICE_BELOW_BOTTOM} syncStatus="fresh" />);
    expect(below).toContain('Ladder: NOCK is at or below your $0.02 ladder bottom, so none is measured above it.');
    expect(below).not.toContain('invalid');
    const narrow = renderToStaticMarkup(<LadderSuggestionCard suggestion={LADDER_BOTTOM_ABOVE_TOP} syncStatus="fresh" />);
    expect(narrow).toContain('both bounds snap to tick 273900');
    const none = renderToStaticMarkup(<LadderSuggestionCard suggestion={LADDER_NO_TARGET_POOL} syncStatus="fresh" />);
    expect(none).toContain('data-testid="ladder-excluded"');
    expect(none).toContain('uniswap-v4:base:3163450: this pool has no in-range liquidity, so its price is frozen');
    expect(none).toContain('a basic Aero pool is full range; it cannot hold a ladder');
    for (const s of [LADDER_PRICE_BELOW_BOTTOM, LADDER_BOTTOM_ABOVE_TOP, LADDER_NO_TARGET_POOL, LADDER_TRACKED, LADDER_NEEDS_SETTING]) {
      const h = renderToStaticMarkup(<LadderSuggestionCard suggestion={s} syncStatus="fresh" />);
      expect(h).not.toMatch(NO_INPUT);
      expect(h).not.toMatch(IMPERATIVE);
    }
  });

  it('renders nothing without a suggestion (an older worker)', () => {
    expect(renderToStaticMarkup(<LadderSuggestionCard suggestion={null} />)).toBe('');
    expect(renderToStaticMarkup(<LadderSuggestionLiteCard suggestion={undefined} />)).toBe('');
  });

  it('the phone card has the same headline, picture, outcomes and deposits', () => {
    const html = renderToStaticMarkup(<LadderSuggestionLiteCard suggestion={bottle()} syncStatus="fresh" />);
    expect(html).toContain('data-testid="ladder-suggestion-lite"');
    expect(html).toContain('Ladder ready: up to $765.86 USDC in 2 deposits, 0 NOCK.');
    expect(html).toContain('viewBox="0 0 340 ');
    expect(html).toContain('A rise sells nothing: the ladder holds 0 NOCK, so its USDC stays USDC.');
    expect(html).toContain('Deposit 1 of 2 (near rung)');
    expect(html).toContain('Deposit 2 of 2 (deep rung)');
    expect(html).toContain('OPEN IN UNISWAP ↗ (pool and range prefilled)');
    expect(html).not.toMatch(NO_INPUT);
    const stale = renderToStaticMarkup(<LadderSuggestionLiteCard suggestion={LADDER_READY} syncStatus="stale" />);
    expect(stale).toContain('Ladder: not shown while the snapshot is stale.');
  });
});

describe('dashboard placement (#168)', () => {
  it('the card sits under the title row, above the stat tiles', async () => {
    const { readFileSync } = await import('node:fs');
    const app = readFileSync(new URL('../App.tsx', import.meta.url), 'utf8');
    const card = app.indexOf('<LadderSuggestionCard');
    const tiles = app.indexOf('{/* Primary Dual-Goal Metrics Bar */}');
    const title = app.indexOf('<div style={styles.titleRow}>');
    expect(card).toBeGreaterThan(title);
    expect(card).toBeLessThan(tiles);
    expect(app).not.toContain('<LadderSimulatorPanel');
  });
});
