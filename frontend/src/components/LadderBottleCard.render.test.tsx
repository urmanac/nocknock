import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { LadderPicture, LadderSuggestionCard, LadderSuggestionLiteCard } from './LadderSuggestionCard';
import {
  LADDER_BOTTLE,
  LADDER_BOTTLE_ALL_DEPLOYED,
  LADDER_BOTTLE_CAP_NEAR_DEPLOYED,
  LADDER_BOTTLE_DEEP_DEPLOYED,
  LADDER_BOTTLE_NEAR_PASSED,
  LADDER_BOTTLE_OVER,
} from '../ladderBottle.fixture';
import { LADDER_READY_LIVE } from '../ladderSuggestion.fixture';

const NO_INPUT = /<input|<form|<select|<textarea|<button/;
const ADVICE = /\b(should|recommend|you must|you need to|please|make sure|we suggest)\b/i;
const count = (html: string, needle: string) => html.split(needle).length - 1;

describe('LadderSuggestionCard with a configured shape (the bottle)', () => {
  it('three deposits: headline, picture, outcomes, the wallet line and each numbered deposit', () => {
    const html = renderToStaticMarkup(<LadderSuggestionCard suggestion={LADDER_BOTTLE} syncStatus="fresh" rungAction={(r) => <span>slot {r.index}</span>} />);
    expect(html).toContain('Ladder: 3 of 3 deposits left, up to $766.99 USDC and about 87 NOCK.');
    expect(html).toContain('data-testid="ladder-picture"');
    // The outcomes, in plain words.
    expect(html).toContain('A fall to $0.01097 (-15%), the near rung&#x27;s bottom, would buy about 9,811 NOCK for $116.09 of the USDC, an average of $0.01183.');
    expect(html).toContain('A fall to the ladder bottom $0.01002 (-23%) would buy about 71,888 NOCK in total with all $766.99, an average of $0.01067.');
    expect(html).toContain('A rise to $0.01313 (+1.4%) or above sells the cap&#x27;s NOCK, about 87, for about $1.13 of USDC, and sells nothing else: every other rung holds 0 NOCK.');
    // What the deposits leave in the wallet, once.
    expect(count(html, 'data-testid="ladder-wallet-after"')).toBe(1);
    expect(html).toContain('After these deposits your wallet holds about $779 USDC; the $950 reserve counts the $224 of USDC inside your existing NOCK ranges.');
    // The three deposits, in order, each with its range, its link, its amount and a slot.
    const a = html.indexOf('Deposit 1 of 3 (cap): about 86.79 NOCK + $1.23 USDC, in range now');
    const b = html.indexOf('Deposit 2 of 3 (near rung): $114.86 USDC, no NOCK, below the price now');
    const c = html.indexOf('Deposit 3 of 3 (deep rung): $650.89 USDC, no NOCK, below the price now');
    expect(a).toBeGreaterThan(-1);
    expect(b).toBeGreaterThan(a);
    expect(c).toBeGreaterThan(b);
    expect(count(html, 'data-testid="ladder-rung"')).toBe(3);
    expect(count(html, 'OPEN IN UNISWAP ↗ (pool and range prefilled)')).toBe(3);
    expect(count(html, 'href="https://app.uniswap.org/positions/add/new?')).toBe(3);
    expect(html).not.toContain('depositState');
    expect(html).toContain('ticks 273600 to 273900, a new position in the USDC/NOCK 3% v4 pool');
    expect(html).toContain('76.1558 to 78.4750 NOCK per USDC');
    expect(html).toContain('Amount to enter in the app: $114.86 USDC');
    expect(html).toContain('Amount to enter in the app: $650.89 USDC');
    for (const i of [1, 2, 3]) expect(html).toContain(`slot ${i}`);
    // The cap: both amounts, the limit, and the field that keeps it. Only the cap.
    expect(html).toContain('Amount to enter in the app: 86.7890 NOCK, in the NOCK field; the app then works out the USDC side, about $1.23 at this reading.');
    expect(count(html, 'data-testid="ladder-nock-limit"')).toBe(1);
    expect(html).toContain('The NOCK side must not exceed 86.7891 NOCK, your LP NOCK budget. The limit holds when the NOCK field is the one typed, at 86.7890 NOCK or less.');
    // One muted fee line per rung, with its sample size, in-range share and replay bound.
    expect(count(html, 'data-testid="ladder-fee-line"')).toBe(3);
    expect(html).toContain('Fees while in range (an estimate): $8.56/day, measured over 6.33 days (1,836 intervals) on v4 USDC/NOCK 3.00% #3163450. In range 28.63% of the measured time. Replayed over it: at most $6.39 inside and $5.16 more at its edges (no lower bound).');
    expect(count(html, 'data-testid="ladder-link-note"')).toBe(1);
    // No input of any kind, and nothing that reads as advice.
    expect(html).not.toMatch(NO_INPUT);
    expect(html).not.toMatch(ADVICE);
  });

  it('the picture: three bars on the price axis, the cap over the spot marker, the bottom line and the reserve legend', () => {
    const html = renderToStaticMarkup(<LadderPicture picture={LADDER_BOTTLE.picture!} width={340} />);
    expect(count(html, 'data-testid="ladder-bar"')).toBe(3);
    expect(html).not.toContain('data-deployed="true"');
    expect(html).toContain('>$1.23 + 87 NOCK</text>');
    expect(html).toContain('>$114.86</text>');
    expect(html).toContain('>$650.89</text>');
    expect(html).toContain('data-testid="ladder-spot-marker"');
    expect(html).toContain('spot $0.01294');
    expect(html).toContain('data-testid="ladder-bottom-line"');
    expect(html).toContain('bottom $0.01002');
    expect(html).toContain('data-testid="ladder-reserve-zone"');
    expect(html).toContain('$950 kept for below $0.01');
    expect(html).not.toContain('data-testid="ladder-deployed-legend"');
    // The cap's label sits beside its narrow bar; no label is smaller than 12 units.
    expect(html).toMatch(/text-anchor="end">\$1\.23 \+ 87 NOCK<\/text>/);
    expect(html).not.toMatch(/font-size="(?:[0-9]|1[01])"/);
    expect(html).toContain('viewBox="0 0 340 ');
  });

  it('a deployed rung: its bar is hatched, its row says done, and it has no link', () => {
    const html = renderToStaticMarkup(<LadderSuggestionCard suggestion={LADDER_BOTTLE_DEEP_DEPLOYED} syncStatus="fresh" />);
    expect(html).toContain('Ladder: 2 of 3 deposits left, up to $116.09 USDC and about 87 NOCK.');
    expect(count(html, 'data-testid="ladder-bar"')).toBe(3);
    expect(count(html, 'data-deployed="true"')).toBe(1);
    expect(html).toContain('fill="url(#ladder-hatch)"');
    expect(html).toContain('data-testid="ladder-deployed-legend"');
    expect(html).toContain('already deployed');
    expect(html).toContain('>$650.90</text>');
    expect(count(html, 'data-testid="ladder-rung"')).toBe(2);
    expect(count(html, 'data-testid="ladder-rung-done"')).toBe(1);
    expect(html).toContain('data-status="deployed"');
    expect(html).toContain('Deposit 3 of 3 (deep rung): done. It is deployed in v4 USDC/NOCK 3.00% #9000001, which holds $650.90 USDC and 0 NOCK now.');
    expect(count(html, 'href="https://app.uniswap.org/positions/add/new?')).toBe(2);
    // The done row comes after the two deposits, in the shape's order.
    expect(html.indexOf('Deposit 2 of 3 (near rung): $114.86 USDC')).toBeLessThan(html.indexOf('Deposit 3 of 3 (deep rung): done.'));
    expect(html).toContain('A fall to $0.01097 (-15%), the bottom of what is left to deposit, would buy about 9,811 NOCK in total with all $116.09, an average of $0.01183.');
    expect(html).not.toMatch(NO_INPUT);
    expect(html).not.toMatch(ADVICE);
  });

  it('only the deep rung left: the done rows come first and no NOCK limit is stated', () => {
    const html = renderToStaticMarkup(<LadderSuggestionCard suggestion={LADDER_BOTTLE_CAP_NEAR_DEPLOYED} syncStatus="fresh" />);
    expect(html).toContain('Ladder: 1 of 3 deposits left, up to $650.90 USDC.');
    expect(count(html, 'data-testid="ladder-rung-done"')).toBe(2);
    expect(count(html, 'data-testid="ladder-rung"')).toBe(1);
    expect(count(html, 'data-deployed="true"')).toBe(2);
    expect(html.indexOf('Deposit 2 of 3 (near rung): done.')).toBeLessThan(html.indexOf('Deposit 3 of 3 (deep rung): $650.89 USDC'));
    expect(html).not.toContain('data-testid="ladder-nock-limit"');
    expect(html).toContain('A rise sells nothing: these deposits hold 0 NOCK, so their USDC stays USDC.');
  });

  it('all deployed: the card says the ladder is deployed and lists each rung, with no link', () => {
    const html = renderToStaticMarkup(<LadderSuggestionCard suggestion={LADDER_BOTTLE_ALL_DEPLOYED} syncStatus="fresh" />);
    expect(html).toContain('Ladder deployed: all 3 deposits are done.');
    expect(html).toContain('Nothing is left to deposit: of the 3 rungs of your shape, 3 deployed.');
    expect(html).toContain('data-testid="ladder-shape-rows"');
    expect(count(html, ': done. It is deployed in v4 USDC/NOCK 3.00% #')).toBe(3);
    expect(html).not.toMatch(/href=|ladder-picture|ladder-rung"/);
    expect(html).not.toMatch(NO_INPUT);
    expect(html).not.toMatch(ADVICE);
  });

  it('a passed rung and an over rung state their plain reason', () => {
    const passed = renderToStaticMarkup(<LadderSuggestionCard suggestion={LADDER_BOTTLE_NEAR_PASSED} syncStatus="fresh" />);
    expect(passed).toContain('data-status="passed"');
    expect(passed).toContain('Deposit 2 of 3 (near rung): passed. The price is below this rung&#x27;s range; a USDC-only deposit there is not possible.');
    expect(count(passed, 'data-testid="ladder-rung"')).toBe(2);
    expect(count(passed, 'data-testid="ladder-bar"')).toBe(2);
    const over = renderToStaticMarkup(<LadderSuggestionCard suggestion={LADDER_BOTTLE_OVER} syncStatus="fresh" />);
    expect(over).toContain('Ladder: no USDC is free for one at the ladder bottom.');
    expect(over).toContain('A tracked rung holds more than its target');
    expect(over).toContain('Deposit 3 of 3 (deep rung): over its target, so nothing more is listed for it.');
    expect(over).not.toMatch(/href=|ladder-picture/);
    for (const h of [passed, over]) {
      expect(h).not.toMatch(NO_INPUT);
      expect(h).not.toMatch(ADVICE);
    }
  });

  it('stale: nothing of the ladder is shown', () => {
    const html = renderToStaticMarkup(<LadderSuggestionCard suggestion={LADDER_BOTTLE} syncStatus="stale" />);
    expect(html).toContain('Ladder: not shown while the snapshot is stale.');
    expect(html).not.toMatch(/Deposit 1 of|href=|ladder-wallet-after/);
  });

  it('the phone card shows the same headline, outcomes, wallet line and deposits', () => {
    const html = renderToStaticMarkup(<LadderSuggestionLiteCard suggestion={LADDER_BOTTLE_DEEP_DEPLOYED} syncStatus="fresh" />);
    const desk = renderToStaticMarkup(<LadderSuggestionCard suggestion={LADDER_BOTTLE_DEEP_DEPLOYED} syncStatus="fresh" />);
    expect(html).toContain('data-testid="ladder-suggestion-lite"');
    expect(html).toContain('viewBox="0 0 340 ');
    for (const same of [
      'Ladder: 2 of 3 deposits left, up to $116.09 USDC and about 87 NOCK.',
      'A fall to $0.01097 (-15%), the bottom of what is left to deposit, would buy about 9,811 NOCK in total with all $116.09, an average of $0.01183.',
      'After these deposits your wallet holds about $779 USDC;',
      'Deposit 1 of 3 (cap): about 86.79 NOCK + $1.23 USDC, in range now',
      'Deposit 2 of 3 (near rung): $114.86 USDC, no NOCK, below the price now',
      'Deposit 3 of 3 (deep rung): done. It is deployed in v4 USDC/NOCK 3.00% #9000001',
      'The NOCK side must not exceed 86.7891 NOCK, your LP NOCK budget.',
      'Amount to enter in the app: $114.86 USDC',
      'OPEN IN UNISWAP ↗ (pool and range prefilled)',
      'Fees while in range (an estimate): $8.56/day',
      'data-deployed="true"',
    ]) {
      expect(html).toContain(same);
      expect(desk).toContain(same);
    }
    expect(html).not.toMatch(NO_INPUT);
    const full = renderToStaticMarkup(<LadderSuggestionLiteCard suggestion={LADDER_BOTTLE} syncStatus="fresh" />);
    expect(count(full, 'data-testid="ladder-rung"')).toBe(3);
    expect(count(full, 'data-testid="ladder-bar"')).toBe(3);
  });

  it('without a shape the card is the single rung of #168: none of the new rows appears', () => {
    const html = renderToStaticMarkup(<LadderSuggestionCard suggestion={LADDER_READY_LIVE} syncStatus="fresh" />);
    expect(html).toContain('Deposit 1 of 1: $765.86 USDC + 0 NOCK');
    expect(html).not.toMatch(/ladder-wallet-after|ladder-nock-limit|ladder-fee-line|ladder-rung-done|ladder-shape-rows|ladder-deployed-legend|data-deployed/);
    expect(html).toContain('>$765.86</text>');
  });
});
