import { describe, expect, it } from 'vitest';
import { existsSync, readFileSync } from 'node:fs';
import {
  candidateLines,
  excludedLines,
  ladderCardView,
  ladderDepositUrl,
  ladderLinkLabel,
  ladderSuggestionOf,
  linkFormatNote,
  missingSettingSentence,
  pictureGeometry,
  price4,
  rungRangeLines,
  rungTitle,
  statedLines,
} from './ladderSuggestion';
import {
  LADDER_BOTTOM_ABOVE_TOP,
  LADDER_NEEDS_SETTING,
  LADDER_NO_TARGET_POOL,
  LADDER_PRICE_BELOW_BOTTOM,
  LADDER_READY,
  LADDER_READY_LIVE,
  LADDER_TRACKED,
} from './ladderSuggestion.fixture';

describe('ladder suggestion (#168)', () => {
  it('the fixture is the worker golden file', () => {
    const golden = new URL('../../ingestion-worker/testdata/ladder_suggest/golden_ladder_suggest.json', import.meta.url);
    if (!existsSync(golden)) return;
    const doc = JSON.parse(readFileSync(golden, 'utf8'));
    expect(LADDER_READY).toEqual(doc.ready.ladder_suggestion);
    expect(LADDER_READY_LIVE).toEqual(doc.ready_live.ladder_suggestion);
    expect(LADDER_NEEDS_SETTING).toEqual(doc.needs_setting.ladder_suggestion);
    expect(LADDER_BOTTOM_ABOVE_TOP).toEqual(doc.bottom_at_or_above_top.ladder_suggestion);
    expect(LADDER_PRICE_BELOW_BOTTOM).toEqual(doc.price_at_or_below_bottom.ladder_suggestion);
    expect(LADDER_NO_TARGET_POOL).toEqual(doc.no_target_pool.ladder_suggestion);
    expect(LADDER_TRACKED).toEqual(doc.tracked.ladder_suggestion);
  });

  it('reads the snapshot field and tolerates an older worker', () => {
    expect(ladderSuggestionOf({ derived_metrics: { ladder_suggestion: LADDER_READY } })).toBe(LADDER_READY);
    expect(ladderSuggestionOf({ derived_metrics: {} })).toBeNull();
    expect(ladderSuggestionOf(null)).toBeNull();
  });

  it('shows the ladder only when ready and fresh, else one line saying why', () => {
    expect(ladderCardView(LADDER_READY_LIVE, 'fresh')).toEqual({ mode: 'ready', headline: 'Ladder ready: up to $765.86 USDC in 1 deposit, 0 NOCK.', line: null });
    expect(ladderCardView(LADDER_READY, 'syncing').mode).toBe('ready');
    expect(ladderCardView(LADDER_READY, 'stale').mode).toBe('stale');
    const tracked = ladderCardView(LADDER_TRACKED, 'fresh');
    expect(tracked.mode).toBe('line');
    expect(tracked.headline).toBe('Ladder: tracked. No new one is suggested.');
    expect(ladderCardView(LADDER_PRICE_BELOW_BOTTOM, 'fresh').headline).toBe(
      'Ladder: NOCK is at or below your $0.02 ladder bottom, so none is measured above it.',
    );
    expect(ladderCardView(LADDER_BOTTOM_ABOVE_TOP, 'fresh').headline).toBe(
      'Ladder: NOCK is within one tick spacing of your bottom, so the range would be empty.',
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

  it('each deposit states its amount and its range in both forms, as a new position', () => {
    const r = LADDER_READY_LIVE.rungs[0];
    expect(rungTitle(r)).toBe('Deposit 1 of 1: $765.86 USDC + 0 NOCK');
    expect(rungRangeLines(r)).toEqual([
      '$0.01002405 to $0.01274292 per NOCK',
      "78.4750 to 99.7600 NOCK per USDC (the app's form)",
      'ticks 273900 to 276300, a new position in the USDC/NOCK 3% v4 pool',
    ]);
    expect(rungRangeLines(r).join(' ')).not.toContain('#3163450');
    expect(rungTitle({ ...r, index: 2, of: 2, rung: 'deep' })).toBe('Deposit 2 of 2 (deep rung): $765.86 USDC + 0 NOCK');
  });

  it('the link label says what the link prefills; only the two app hosts over https', () => {
    const r = LADDER_READY_LIVE.rungs[0];
    expect(ladderDepositUrl(r.deep_link.url)).toBe(r.deep_link.url);
    expect(ladderLinkLabel(r)).toBe('OPEN IN UNISWAP ↗ (pool and range prefilled)');
    expect(r.deep_link.prefilled).toEqual(['pair', 'fee and tick spacing', 'range ticks']);
    expect(r.deep_link.not_prefilled).toEqual(['amount']);
    // The app drops an amount passed in the link (observed 2026-11-21): none is passed,
    // and the amount is stated beside it.
    expect(r.deep_link.url).not.toContain('depositState');
    expect(r.deep_link.amount_to_enter).toBe('Amount to enter in the app: $765.86 USDC');
    for (const bad of ['http://app.uniswap.org/x', 'https://aerodrome.finance/deposit', 'https://evil.example/app.uniswap.org', 'javascript:alert(1)', '', null]) {
      expect(ladderDepositUrl(bad as any)).toBeNull();
    }
    expect(ladderLinkLabel({ ...r, deep_link: { ...r.deep_link, url: 'https://app.aero.xyz/deposit/8453-0xabc', prefilled: [] } })).toBe(
      'OPEN POOL DEPOSIT ON AERO.XYZ ↗ (nothing prefilled)',
    );
    expect(linkFormatNote(LADDER_READY_LIVE)).toContain('check the pool and range in the app');
  });

  it('states the lines that are not gates, and each excluded pool', () => {
    const stated = statedLines(LADDER_READY_LIVE);
    expect(stated.some((l) => l.startsWith('Plan budget not yet spent: $'))).toBe(true);
    expect(stated.some((l) => l.startsWith('Tax reserve: $8.70'))).toBe(true);
    expect(stated.some((l) => l.startsWith('Cash target: $'))).toBe(true);
    expect(excludedLines(LADDER_NO_TARGET_POOL)).toContain('uniswap-v4:base:3163450: this pool has no in-range liquidity, so its price is frozen');
  });

  it('plain prices have 4 significant digits', () => {
    expect(price4(0.0110)).toBe('0.01100');
    expect(price4(0.01002405)).toBe('0.01002');
    expect(price4(0.012944)).toBe('0.01294');
  });

  it('the picture puts the bottom left of the bars, spot right of them, and never a negative axis', () => {
    const g = pictureGeometry(LADDER_READY.picture!)!;
    expect(g.bars).toHaveLength(1);
    expect(g.bottomX).toBeLessThanOrEqual(g.bars[0].x + 0.001);
    expect(g.spotX!).toBeGreaterThan(g.bars[0].x + g.bars[0].w);
    expect(g.min).toBeLessThan(Number(LADDER_READY.picture!.bottom_usd));
    // Spot more than five times the bottom: the axis starts at $0, never below.
    const far = pictureGeometry({ ...LADDER_READY.picture!, spot_usd: '0.10000000', bars: [{ ...LADDER_READY.picture!.bars[0], high_usd: '0.09900000' }] })!;
    expect(far.min).toBe(0);
    expect(far.x(0)).toBeGreaterThanOrEqual(8);
    expect(g.fontSize).toBeGreaterThanOrEqual(12);
  });
});
