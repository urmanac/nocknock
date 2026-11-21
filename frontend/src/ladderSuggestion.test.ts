import { describe, expect, it } from 'vitest';
import { existsSync, readFileSync } from 'node:fs';
import {
  candidateLines,
  ladderCardView,
  ladderDepositUrl,
  ladderLinkLabel,
  ladderSuggestionOf,
  missingSettingSentence,
  pictureGeometry,
  rungRangeLines,
  rungTitle,
} from './ladderSuggestion';
import { LADDER_BOTTOM_ABOVE_TOP, LADDER_NEEDS_SETTING, LADDER_READY, LADDER_TRACKED } from './ladderSuggestion.fixture';

describe('ladder suggestion (#168)', () => {
  it('the fixture is the worker golden file', () => {
    const golden = new URL('../../ingestion-worker/testdata/ladder_suggest/golden_ladder_suggest.json', import.meta.url);
    if (!existsSync(golden)) return;
    const doc = JSON.parse(readFileSync(golden, 'utf8'));
    expect(LADDER_READY).toEqual(doc.ready.ladder_suggestion);
    expect(LADDER_NEEDS_SETTING).toEqual(doc.needs_setting.ladder_suggestion);
    expect(LADDER_BOTTOM_ABOVE_TOP).toEqual(doc.bottom_at_or_above_top.ladder_suggestion);
    expect(LADDER_TRACKED).toEqual(doc.tracked.ladder_suggestion);
  });

  it('reads the snapshot field and tolerates an older worker', () => {
    expect(ladderSuggestionOf({ derived_metrics: { ladder_suggestion: LADDER_READY } })).toBe(LADDER_READY);
    expect(ladderSuggestionOf({ derived_metrics: {} })).toBeNull();
    expect(ladderSuggestionOf(null)).toBeNull();
  });

  it('shows the ladder only when ready and fresh, else one line saying why', () => {
    expect(ladderCardView(LADDER_READY, 'fresh')).toEqual({ mode: 'ready', headline: 'Ladder ready: $767.45 of idle USDC in 1 deposit, 0 NOCK.', line: null });
    expect(ladderCardView(LADDER_READY, 'syncing').mode).toBe('ready');
    expect(ladderCardView(LADDER_READY, 'stale').mode).toBe('stale');
    const tracked = ladderCardView(LADDER_TRACKED, 'fresh');
    expect(tracked.mode).toBe('line');
    expect(tracked.headline).toBe('Ladder: tracked. No new one is suggested.');
    expect(tracked.line).toContain('A ladder is tracked');
    expect(ladderCardView(LADDER_BOTTOM_ABOVE_TOP, 'fresh').headline).toBe(
      'Ladder: the price is at or near your bottom, so there is nothing to buy below it.',
    );
  });

  it('a missing setting is one plain sentence, and the candidates a short list', () => {
    expect(missingSettingSentence(LADDER_NEEDS_SETTING)).toBe(
      'NockNock needs your bottom price, operator_profile.ladder_bottom_usd, in config.yaml; it does not choose these numbers.',
    );
    expect(missingSettingSentence(LADDER_READY)).toBeNull();
    const lines = candidateLines(LADDER_NEEDS_SETTING);
    expect(lines.length).toBeGreaterThanOrEqual(6);
    expect(lines.length).toBeLessThanOrEqual(8);
    expect(lines[0]).toMatch(/^\$0\.0\d+ \(-\d+\.\d+%\): up to \$\d+\.\d\d → about \d+ NOCK, average \$0\.\d+$/);
  });

  it('each deposit states its amount and its range in both forms', () => {
    const r = LADDER_READY.rungs[0];
    expect(rungTitle(r)).toBe('Deposit 1 of 1: $767.45 USDC + 0 NOCK');
    expect(rungRangeLines(r)).toEqual([
      '$0.01002405 to $0.01274292 per NOCK',
      "78.4749 to 99.7601 NOCK per USDC (the app's form)",
      'ticks 273900 to 276300 in v4 USDC/NOCK 3.00% #3163450',
    ]);
    expect(rungTitle({ ...r, index: 2, of: 2, rung: 'deep' })).toBe('Deposit 2 of 2 (deep rung): $767.45 USDC + 0 NOCK');
  });

  it('a deposit link is used only on the two app hosts, over https', () => {
    const r = LADDER_READY.rungs[0];
    expect(ladderDepositUrl(r.deep_link.url)).toBe(r.deep_link.url);
    expect(ladderLinkLabel(r)).toBe('OPEN NEW POSITION IN UNISWAP ↗ (range prefilled, amount not)');
    for (const bad of ['http://app.uniswap.org/x', 'https://aerodrome.finance/deposit', 'https://evil.example/app.uniswap.org', 'javascript:alert(1)', '', null]) {
      expect(ladderDepositUrl(bad as any)).toBeNull();
    }
    expect(ladderLinkLabel({ ...r, deep_link: { ...r.deep_link, url: 'https://app.aero.xyz/deposit/8453-0xabc', prefilled: [] } })).toBe(
      'OPEN POOL DEPOSIT ON AERO.XYZ ↗ (nothing prefilled)',
    );
    expect(r.deep_link.url).not.toMatch(/depositState|exactAmounts/);
  });

  it('the picture puts the bottom left of the bars and spot right of them', () => {
    const g = pictureGeometry(LADDER_READY.picture!)!;
    expect(g).not.toBeNull();
    expect(g.bars).toHaveLength(1);
    expect(g.bottomX).toBeLessThanOrEqual(g.bars[0].x + 0.001);
    expect(g.spotX!).toBeGreaterThan(g.bars[0].x + g.bars[0].w);
    expect(g.min).toBeLessThan(Number(LADDER_READY.picture!.bottom_usd));
  });
});
