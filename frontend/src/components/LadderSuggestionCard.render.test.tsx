import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { LadderPicture, LadderSuggestionCard, LadderSuggestionLiteCard } from './LadderSuggestionCard';
import { LADDER_BOTTOM_ABOVE_TOP, LADDER_NEEDS_SETTING, LADDER_READY, LADDER_TRACKED } from '../ladderSuggestion.fixture';
import type { LadderSuggestion } from '../ladderSuggestion';

/** The operator's two-rung "bottle" as a fixture object: near 15% from the top down to
 *  $0.0110, deep 85% from $0.0110 down to $0.01. The card only lays it out. */
function bottle(): LadderSuggestion {
  const one = LADDER_READY.rungs[0];
  const near = { ...one, of: 2, rung: 'near' as const, usdc: { ...one.usdc, amount: '115.12' }, bounds_usd: { low: '0.01100000', high: '0.01274292' } };
  const deep = { ...one, index: 2, of: 2, rung: 'deep' as const, usdc: { ...one.usdc, amount: '652.33' }, bounds_usd: { low: '0.01002405', high: '0.01100000' } };
  return {
    ...LADDER_READY,
    headline: 'Ladder ready: $767.45 of idle USDC in 2 deposits, 0 NOCK.',
    rungs: [near, deep],
    picture: {
      ...LADDER_READY.picture!,
      bars: [
        { index: 1, low_usd: '0.01100000', high_usd: '0.01274292', usdc: '115.12', label: '$115.12' },
        { index: 2, low_usd: '0.01002405', high_usd: '0.01100000', usdc: '652.33', label: '$652.33' },
      ],
    },
  };
}

describe('LadderSuggestionCard (#168)', () => {
  it('ready: headline, picture, plain outcomes and the numbered deposit with its link', () => {
    const html = renderToStaticMarkup(<LadderSuggestionCard suggestion={LADDER_READY} syncStatus="fresh" />);
    expect(html).toContain('data-testid="ladder-suggestion"');
    expect(html).toContain('Ladder ready: $767.45 of idle USDC in 1 deposit, 0 NOCK.');
    expect(html).toContain('data-testid="ladder-picture"');
    // The plain-language outcomes, in the worker's words.
    expect(html).toContain('A fall to the ladder bottom $0.01002405 (-22.56%) would buy about 67,904 NOCK with all $767.45, an average of $0.01130202.');
    expect(html).toContain('A rise sells nothing: the ladder holds 0 NOCK, so its USDC stays USDC.');
    expect(html).toMatch(/A fall to \$0\.0\d+ \(-\d+\.\d+%\) would buy about [\d,]+ NOCK for \$[\d.]+ of the USDC/);
    // The deposit, in both forms, with the link and an action slot.
    expect(html).toContain('Deposit 1 of 1: $767.45 USDC + 0 NOCK');
    expect(html).toContain('78.4749 to 99.7601 NOCK per USDC');
    expect(html).toContain('ticks 273900 to 276300');
    expect(html).toContain('href="https://app.uniswap.org/positions/add/new?chain=base&amp;protocolVersion=v4');
    expect(html).toContain('OPEN NEW POSITION IN UNISWAP ↗ (range prefilled, amount not)');
    expect(html).toContain('data-testid="ladder-rung-action"');
    expect(html).toContain('After you deposit (current fallback):');
    // No input of any kind, and no instruction.
    expect(html).not.toMatch(/<input|<form|<select|<textarea|<button/);
    expect(html.toLowerCase()).not.toMatch(/should|recommend/);
  });

  it('the picture: spot marker, a bar per rung with its amount, the bottom line and the reserve zone', () => {
    const html = renderToStaticMarkup(<LadderPicture picture={LADDER_READY.picture!} />);
    expect(html).toContain('data-testid="ladder-spot-marker"');
    expect(html).toContain('spot $0.01294400');
    expect(html.match(/data-testid="ladder-bar"/g)).toHaveLength(1);
    expect(html).toContain('>$767.45</text>');
    expect(html).toContain('data-testid="ladder-bottom-line"');
    expect(html).toContain('bottom $0.01002405');
    expect(html).toContain('data-testid="ladder-reserve-zone"');
    expect(html).toContain('$950 kept for below $0.01');
  });

  it('two rungs render as a numbered list and two bars', () => {
    const html = renderToStaticMarkup(<LadderSuggestionCard suggestion={bottle()} syncStatus="fresh" rungAction={(r) => <span>slot {r.index}</span>} />);
    const a = html.indexOf('Deposit 1 of 2 (near rung): $115.12 USDC + 0 NOCK');
    const b = html.indexOf('Deposit 2 of 2 (deep rung): $652.33 USDC + 0 NOCK');
    expect(a).toBeGreaterThan(-1);
    expect(b).toBeGreaterThan(a);
    expect(html.match(/data-testid="ladder-bar"/g)).toHaveLength(2);
    expect(html).toContain('>$115.12</text>');
    expect(html).toContain('>$652.33</text>');
    expect(html).toContain('slot 1');
    expect(html).toContain('slot 2');
    expect(html).toContain('<ol');
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
    expect(html).toContain('Ladder: set your bottom price (operator_profile.ladder_bottom_usd) to see one.');
    expect(html).toContain('NockNock needs your bottom price, operator_profile.ladder_bottom_usd, in config.yaml; it does not choose these numbers.');
    expect(html).toContain('data-testid="ladder-candidates"');
    expect(html).toContain('Candidate bottoms (a measurement; none is picked):');
    expect(html).not.toMatch(/<input|<form|<select|<textarea|<button/);
  });

  it('a failing gate states its reason', () => {
    const html = renderToStaticMarkup(<LadderSuggestionCard suggestion={LADDER_BOTTOM_ABOVE_TOP} syncStatus="fresh" />);
    expect(html).toContain('Ladder: the price is at or near your bottom, so there is nothing to buy below it.');
    expect(html).toContain('both bounds snap to tick 273900');
  });

  it('renders nothing without a suggestion (an older worker)', () => {
    expect(renderToStaticMarkup(<LadderSuggestionCard suggestion={null} />)).toBe('');
    expect(renderToStaticMarkup(<LadderSuggestionLiteCard suggestion={undefined} />)).toBe('');
  });

  it('the phone card has the same headline, outcomes and deposits', () => {
    const html = renderToStaticMarkup(<LadderSuggestionLiteCard suggestion={bottle()} syncStatus="fresh" />);
    expect(html).toContain('data-testid="ladder-suggestion-lite"');
    expect(html).toContain('Ladder ready: $767.45 of idle USDC in 2 deposits, 0 NOCK.');
    expect(html).toContain('A rise sells nothing: the ladder holds 0 NOCK, so its USDC stays USDC.');
    expect(html).toContain('Deposit 1 of 2 (near rung)');
    expect(html).toContain('Deposit 2 of 2 (deep rung)');
    expect(html).toContain('OPEN NEW POSITION IN UNISWAP');
    expect(html).not.toMatch(/<input|<form|<button/);
    const stale = renderToStaticMarkup(<LadderSuggestionLiteCard suggestion={LADDER_READY} syncStatus="stale" />);
    expect(stale).toContain('Ladder: not shown while the snapshot is stale.');
  });
});
