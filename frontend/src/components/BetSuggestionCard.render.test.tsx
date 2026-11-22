import { readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { BetLiteCard, BetQuietLine, BetSuggestionCard } from './BetSuggestionCard';
import { BET_GAS_SHORT, BET_IN_RANGE, BET_NEEDS_SETTING, BET_READY } from '../betSuggestion.fixture';

const NO_INPUT = /<input|<form|<select|<textarea/;
const IMPERATIVE = /\b(should|recommend|you must|you need to|please|make sure|pull now)\b/i;

describe('BetSuggestionCard (#181)', () => {
  it('ready: headline, range, the amounts beside the link, outcomes, gates', () => {
    const html = renderToStaticMarkup(<BetSuggestionCard bet={BET_READY} syncStatus="fresh" />);
    expect(html).toContain('WETH/NOCK BET');
    expect(html).toContain(BET_READY.headline!);
    expect(html).toContain('Amounts to enter in the Aero app: ');
    expect(html).toContain('href="https://app.aero.xyz/deposit/8453-0x957256c98d61A714081DE2EE70ae3A96c9b40874?range=custom&amp;tickLow=70200&amp;tickHigh=71000&amp;native=true"');
    expect(html).toContain('the bet holds only NOCK');
    expect(html).toContain('With this bet, NOCK could rise to $');
    expect(html).not.toMatch(NO_INPUT);
    expect(html).not.toMatch(/<button/);
    expect(html).not.toMatch(IMPERATIVE);
    expect(html).not.toContain('aerodrome.finance');
  });

  it('stale: the ready card shows no link', () => {
    const html = renderToStaticMarkup(<BetSuggestionCard bet={BET_READY} syncStatus="stale" />);
    expect(html).toContain('not shown while the snapshot is stale');
    expect(html).not.toContain('href=');
  });

  it('review of #181: a stale snapshot shows no deposit link anywhere, phone or top of page', async () => {
    expect(renderToStaticMarkup(<BetLiteCard bet={BET_READY} syncStatus="stale" />)).not.toContain('href=');
    const { TopOfPageEvents } = await import('./TopOfPage');
    const e = { id: 'bet_suggestion:WETH/NOCK:ready', kind: 'bet_suggestion', tier: 'top', title: 'WETH/NOCK BET', what_changed: null, text: BET_READY.headline, numbers: {}, since: null, deep_link: '#event-bet-suggestion-weth-nock-ready', links: [], would_sign: null, card: 'bet_suggestion' };
    expect(renderToStaticMarkup(<TopOfPageEvents data={{ derived_metrics: { events: [e] } }} syncStatus="stale" />)).toBe('');
  });

  it('every row wraps and every child may shrink, so nothing leaks out', () => {
    const html = renderToStaticMarkup(<BetSuggestionCard bet={BET_READY} syncStatus="fresh" compact />);
    expect(html).toContain('overflow-wrap:anywhere');
    expect(html).toContain('min-width:0');
    expect(html).toContain('flex-wrap:wrap');
    // The long link text breaks anywhere too.
    expect(html).toMatch(/<a data-testid="bet-link"[^>]*overflow-wrap:anywhere/);
  });

  it('done things and blocked bets are one quiet line, the card behind it', () => {
    const inRange = renderToStaticMarkup(<BetQuietLine bet={BET_IN_RANGE} syncStatus="fresh" />);
    expect(inRange).toContain('<details');
    expect(inRange).toContain('1 in range, 1 cashed out');
    expect(inRange).toContain('out of range, cashed out: it holds only NOCK');
    expect(renderToStaticMarkup(<BetQuietLine bet={BET_GAS_SHORT} />)).toContain('would have to come out of them first');
    expect(renderToStaticMarkup(<BetQuietLine bet={BET_NEEDS_SETTING} />)).toContain('operator_profile.nock_bets.gas_floor_eth');
    // At the top, there is no quiet line.
    expect(renderToStaticMarkup(<BetQuietLine bet={BET_READY} />)).toBe('');
  });

  it('a bet found in the gauge offers one tap each, no typing', () => {
    const found = {
      ...BET_IN_RANGE,
      status: 'found_untracked',
      found_not_counted: [
        { nft_id: 2914490, protocol: 'aerodrome-cl', dismissed: false, tick_lower: 73400, tick_upper: 74800, liquidity: '1', why_not_automatic: ['its deposit was sent by 0x4598…e826, not by your wallet'], choices: ['bet', 'dismiss'], line: 'A position staked by your wallet in the bet gauge (ticks 73400..74800) is not counted: its deposit was sent by 0x4598…e826, not by your wallet.' },
      ],
    };
    const html = renderToStaticMarkup(<BetLiteCard bet={found} syncStatus="fresh" />);
    expect(html).toContain('TRACK AS A BET');
    expect(html).toContain('NOT NOW');
    expect(html).not.toMatch(NO_INPUT);
  });

  it('the dashboard and the phone place the card and its anchor', () => {
    const app = readFileSync(new URL('../App.tsx', import.meta.url), 'utf8');
    expect(app).toContain('<BetSuggestionCard bet={bet}');
    expect(app).toContain('<BetQuietLine bet={bet}');
    const phone = readFileSync(new URL('./LitePhoneCards.tsx', import.meta.url), 'utf8');
    expect(phone).toContain('<BetLiteCard bet={betSuggestionOf(snapshot.data)}');
  });
});
