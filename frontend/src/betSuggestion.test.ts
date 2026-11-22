import { existsSync, readFileSync } from 'node:fs';
import { describe, expect, it } from 'vitest';
import { BET_GAS_SHORT, BET_IN_RANGE, BET_NEEDS_SETTING, BET_READY } from './betSuggestion.fixture';
import {
  betAmountsLine,
  betAtTop,
  betCardView,
  betChoiceLabel,
  betGateLines,
  betLinkLabel,
  betLinkUrl,
  betQuietLine,
  betSuggestionOf,
  tapBet,
  type BetClient,
} from './betSuggestion';
import { isNockLp } from './ladderTrack';
import { rangeStatus } from './positions';
import { OWN_CARD_KINDS, stripRenders } from './events';

describe('betSuggestion (#181)', () => {
  it('the fixture is the worker golden file', () => {
    const golden = new URL('../../ingestion-worker/testdata/bets/golden_bet_suggestion.json', import.meta.url);
    if (!existsSync(golden)) return;
    const doc = JSON.parse(readFileSync(golden, 'utf8'));
    expect(BET_READY).toEqual(doc.ready.bet_suggestion);
    expect(BET_GAS_SHORT).toEqual(doc.gas_short.bet_suggestion);
    expect(BET_IN_RANGE).toEqual(doc.bet_in_range.bet_suggestion);
    expect(BET_NEEDS_SETTING).toEqual(doc.needs_setting.bet_suggestion);
  });

  it('reads the card from a snapshot, and nothing from one without it', () => {
    expect(betSuggestionOf({ derived_metrics: { bet_suggestion: BET_READY } })).toBe(BET_READY);
    expect(betSuggestionOf({ derived_metrics: {} })).toBeNull();
    expect(betSuggestionOf(null)).toBeNull();
  });

  it('only a suggested bet (or a found one waiting) is at the top', () => {
    expect(betAtTop(BET_READY)).toBe(true);
    expect(betAtTop(BET_GAS_SHORT)).toBe(false);
    expect(betAtTop(BET_IN_RANGE)).toBe(false);
    expect(betAtTop(BET_NEEDS_SETTING)).toBe(false);
    expect(betAtTop({ ...BET_IN_RANGE, status: 'found_untracked' })).toBe(true);
    // The card renders its own top entry; the strip leaves it alone.
    expect(OWN_CARD_KINDS).toContain('bet_suggestion');
    expect(stripRenders({ tier: 'top', kind: 'bet_suggestion' })).toBe(false);
  });

  it('a ready card is not a call to action on a stale snapshot', () => {
    expect(betCardView(BET_READY, 'fresh').mode).toBe('ready');
    expect(betCardView(BET_READY, 'stale').mode).toBe('stale');
    expect(betCardView(BET_GAS_SHORT, 'fresh')).toEqual({ mode: 'line', headline: BET_GAS_SHORT.headline, line: BET_GAS_SHORT.reason });
  });

  it('the link goes only to the Aero app, with the range, and the amounts are printed beside it', () => {
    const s = BET_READY.suggestion!;
    expect(betLinkUrl(s.link.url)).toBe('https://app.aero.xyz/deposit/8453-0x957256c98d61A714081DE2EE70ae3A96c9b40874?range=custom&tickLow=70200&tickHigh=71000&native=true');
    expect(betLinkUrl('https://aerodrome.finance/deposit')).toBeNull();
    expect(betLinkUrl('http://app.aero.xyz/deposit')).toBeNull();
    expect(betLinkLabel(s)).toContain('native ETH');
    expect(betAmountsLine(s)).toMatch(/^Amounts to enter in the Aero app: [\d.]+ ETH and [\d.]+ NOCK$/);
  });

  it('gates read in the issue order, and a short wallet is a measurement', () => {
    expect(betGateLines(BET_READY).map((g) => g.ok)).toEqual([true, true, true, true]);
    const gas = betGateLines(BET_GAS_SHORT)[1];
    expect(gas.ok).toBe(false);
    expect(gas.text).toContain('A measurement, not an instruction.');
    expect(gas.text.toLowerCase()).not.toContain('pull now');
  });

  it('done things are one quiet line', () => {
    expect(betQuietLine(BET_IN_RANGE)).toBe('Bets: a bet is in range. No new one is suggested. · 1 in range, 1 cashed out');
    expect(betQuietLine(BET_NEEDS_SETTING)).toBe('Bets: a setting is needed before one is measured.');
  });

  it('a bet is not NOCK LP, and out of range it is cashed out, not a warning', () => {
    const bet = { role: 'bet', token0: { symbol: 'WETH' }, token1: { symbol: 'NOCK' } };
    expect(isNockLp(bet)).toBe(false);
    expect(isNockLp({ ...bet, role: null })).toBe(true);
    expect(rangeStatus({ in_range: false, role: 'bet' })).toEqual({ label: '● BET CASHED OUT', tone: 'neutral' });
    expect(rangeStatus({ in_range: true, role: 'bet' }).label).toBe('● BET IN RANGE');
  });

  it('a tap tracks or sets aside a found bet and never throws', async () => {
    const calls: string[] = [];
    const client: BetClient = {
      trackFoundPosition: async (id, as) => (calls.push(`track ${id} ${as}`), { note: 'Tracked as a bet.', cycle: 'ok' }),
      dismissFoundBet: async (id) => (calls.push(`dismiss ${id}`), {}),
      restoreFoundBet: async (id) => (calls.push(`restore ${id}`), {}),
      ingestNow: async () => undefined,
    };
    expect(await tapBet(7, 'bet', client)).toEqual({ ok: true, text: 'Tracked as a bet.' });
    expect((await tapBet(7, 'dismiss', client)).ok).toBe(true);
    expect(calls).toEqual(['track 7 bet', 'dismiss 7']);
    const broken: BetClient = { ...client, trackFoundPosition: async () => { throw new Error('NOT_OWNED'); } };
    expect(await tapBet(7, 'bet', broken)).toEqual({ ok: false, text: 'NOT_OWNED' });
    expect(betChoiceLabel('bet')).toBe('TRACK AS A BET');
    expect(betChoiceLabel('dismiss')).toBe('NOT NOW');
  });
});
