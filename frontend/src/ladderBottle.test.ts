import { describe, expect, it } from 'vitest';
import { existsSync, readFileSync } from 'node:fs';
import {
  labelWidth,
  ladderCardView,
  ladderDepositUrl,
  ladderLinkLabel,
  ladderList,
  linkFormatNote,
  pictureGeometry,
  rungFeeLine,
  rungNockLimit,
  rungRangeLines,
  rungTitle,
  shapeRowLines,
  walletAfterLine,
} from './ladderSuggestion';
import { feeEstimateLine } from './feeYield';
import { LADDER_READY_LIVE } from './ladderSuggestion.fixture';
import {
  LADDER_BOTTLE,
  LADDER_BOTTLE_ALL_DEPLOYED,
  LADDER_BOTTLE_CAP_NEAR_DEPLOYED,
  LADDER_BOTTLE_DEEP_DEPLOYED,
  LADDER_BOTTLE_NEAR_PASSED,
  LADDER_BOTTLE_OVER,
  LADDER_BOTTLE_TEXT,
} from './ladderBottle.fixture';

describe('the ladder suggestion for a configured shape (the bottle)', () => {
  it('the fixture is the worker golden file', () => {
    const golden = new URL('../../ingestion-worker/testdata/ladder_suggest/golden_ladder_bottle.json', import.meta.url);
    if (!existsSync(golden)) return;
    const doc = JSON.parse(readFileSync(golden, 'utf8'));
    expect(LADDER_BOTTLE).toEqual(doc.bottle.ladder_suggestion);
    expect(LADDER_BOTTLE_DEEP_DEPLOYED).toEqual(doc.deep_deployed.ladder_suggestion);
    expect(LADDER_BOTTLE_CAP_NEAR_DEPLOYED).toEqual(doc.cap_and_near_deployed.ladder_suggestion);
    expect(LADDER_BOTTLE_ALL_DEPLOYED).toEqual(doc.all_deployed.ladder_suggestion);
    expect(LADDER_BOTTLE_NEAR_PASSED).toEqual(doc.near_passed.ladder_suggestion);
    expect(LADDER_BOTTLE_OVER).toEqual(doc.deep_over.ladder_suggestion);
    expect(LADDER_BOTTLE_TEXT).toBe(doc.bottle.text);
  });

  it('three numbered deposits, the cap first, each with its amounts in plain words', () => {
    expect(ladderCardView(LADDER_BOTTLE, 'fresh')).toEqual({
      mode: 'ready',
      headline: 'Ladder: 3 of 3 deposits left, up to $766.99 USDC and about 87 NOCK.',
      line: null,
    });
    const list = ladderList(LADDER_BOTTLE);
    expect(list.map((i) => i.kind)).toEqual(['deposit', 'deposit', 'deposit']);
    const [cap, near, deep] = LADDER_BOTTLE.rungs;
    expect(rungTitle(cap)).toBe('Deposit 1 of 3 (cap): about 86.79 NOCK + $1.23 USDC, in range now');
    expect(rungTitle(near)).toBe('Deposit 2 of 3 (near rung): $114.86 USDC, no NOCK, below the price now');
    expect(rungTitle(deep)).toBe('Deposit 3 of 3 (deep rung): $650.89 USDC, no NOCK, below the price now');
    expect(rungRangeLines(cap)).toEqual([
      '$0.01274292 to $0.01313098 per NOCK',
      "76.1558 to 78.4750 NOCK per USDC (the app's form)",
      'ticks 273600 to 273900, a new position in the USDC/NOCK 3% v4 pool',
    ]);
    expect(rungRangeLines(deep)[2]).toBe('ticks 275400 to 276300, a new position in the USDC/NOCK 3% v4 pool');
    // The exact mint each deposit carries, for a transaction builder.
    expect([cap, near, deep].map((r) => r.mint?.liquidity_raw)).toEqual(['138192109253769', '1408301615002237', '14125167068081104']);
    expect([cap, near, deep].map((r) => r.usdc.raw)).toEqual(['1225999', '114864600', '650899400']);
    expect(cap.nock.raw).toBe('867890958577132038');
  });

  it('each deposit has its own link with the pool and range and never an amount', () => {
    const ticks = LADDER_BOTTLE.rungs.map((r) => {
      const url = ladderDepositUrl(r.deep_link.url)!;
      expect(url).not.toContain('depositState');
      expect(ladderLinkLabel(r)).toBe('OPEN IN UNISWAP ↗ (pool and range prefilled)');
      const state = JSON.parse(new URL(url).searchParams.get('priceRangeState')!);
      return [state.minTick, state.maxTick];
    });
    expect(ticks).toEqual([
      [273600, 273900],
      [273900, 275400],
      [275400, 276300],
    ]);
    expect(LADDER_BOTTLE.rungs.map((r) => r.deep_link.amount_to_enter)).toEqual([
      'Amount to enter in the app: 86.7890 NOCK, in the NOCK field; the app then works out the USDC side, about $1.23 at this reading.',
      'Amount to enter in the app: $114.86 USDC',
      'Amount to enter in the app: $650.89 USDC',
    ]);
    expect(linkFormatNote(LADDER_BOTTLE)).toContain('check the pool and range in the app');
  });

  it('the cap states the NOCK limit and the field that keeps it; the other rungs have none', () => {
    const [cap, near, deep] = LADDER_BOTTLE.rungs;
    expect(rungNockLimit(cap)).toBe(
      'The NOCK side must not exceed 86.7891 NOCK, your LP NOCK budget. The limit holds when the NOCK field is the one typed, at 86.7890 NOCK or less. When the USDC field is typed instead, the app works out the NOCK side from the price at that moment, and it can come out above the limit; neither the link nor the app applies a NOCK limit of its own.',
    );
    expect(cap.nock_limit?.field_to_type).toBe('NOCK');
    expect(Number(cap.nock_limit!.nock_to_enter)).toBeLessThanOrEqual(Number(cap.nock.amount));
    expect(Number(cap.nock.amount)).toBeLessThanOrEqual(Number(cap.nock_limit!.nock));
    expect(rungNockLimit(near)).toBeNull();
    expect(rungNockLimit(deep)).toBeNull();
  });

  it('states what the deposits leave in the wallet', () => {
    expect(walletAfterLine(LADDER_BOTTLE)).toBe(
      "After these deposits your wallet holds about $779 USDC; the $950 reserve counts the $224 of USDC inside your existing NOCK ranges. At $0.01 with every rung converted, the stable balance reads $950.00: the wallet's $778.85 plus $171.15 still inside those ranges.",
    );
    expect(walletAfterLine(LADDER_READY_LIVE)).toBeNull();
    expect(walletAfterLine(LADDER_BOTTLE_ALL_DEPLOYED)).toBeNull();
  });

  it('one fee line per rung, never without its sample size, in-range share and replay bound, in the worker\'s own words', () => {
    const lines = LADDER_BOTTLE.rungs.map((r) => rungFeeLine(r)!);
    expect(lines).toEqual([
      'Fees while in range (an estimate): $0.84/day, measured over 6.33 days (1,836 intervals) on v4 USDC/NOCK 3.00% #3163450. In range 1.95% of the measured time. Replayed over it: at most $0.02 inside and $0.15 more at its edges (no lower bound).',
      'Fees while in range (an estimate): $8.56/day, measured over 6.33 days (1,836 intervals) on v4 USDC/NOCK 3.00% #3163450. In range 28.63% of the measured time. Replayed over it: at most $6.39 inside and $5.16 more at its edges (no lower bound).',
      'Fees while in range (an estimate): $85.90/day, measured over 6.33 days (1,836 intervals) on v4 USDC/NOCK 3.00% #3163450. In range 5.82% of the measured time. Replayed over it: at most $32.55 inside and $50.00 more at its edges (no lower bound).',
    ]);
    // The same bytes both MCP transports print.
    for (const line of lines) expect(LADDER_BOTTLE_TEXT).toContain(`    ${line}\n`);
    // Not measured: the reason, never a zero. A part that is missing prints its reason.
    expect(feeEstimateLine({ status: 'unavailable', reason: 'no tracked range in this pool measures fees' })).toBe(
      'Fees while in range: not estimated: no tracked range in this pool measures fees.',
    );
    expect(feeEstimateLine({ status: 'measured', per_day_in_range_usd: '1.00', in_range_share_reason: 'no stored history', replay_reason: 'no stored history' })).toBe(
      'Fees while in range (an estimate): $1.00/day, measured sample size not stated. In range over stored history: not stated (no stored history). Replay: not stated (no stored history).',
    );
    expect(feeEstimateLine(null)).toBe('Fees while in range: not stated in this body.');
    // The single rung of #168 states no estimate.
    expect(rungFeeLine(LADDER_READY_LIVE.rungs[0])).toBeNull();
  });

  it('a deployed rung is one stated line and is not offered again', () => {
    expect(ladderCardView(LADDER_BOTTLE_DEEP_DEPLOYED, 'fresh').headline).toBe('Ladder: 2 of 3 deposits left, up to $116.09 USDC and about 87 NOCK.');
    const list = ladderList(LADDER_BOTTLE_DEEP_DEPLOYED);
    expect(list.map((i) => i.kind)).toEqual(['deposit', 'deposit', 'row']);
    const last = list[2];
    expect(last.kind === 'row' && last.row.text).toBe(
      'Deposit 3 of 3 (deep rung): done. It is deployed in v4 USDC/NOCK 3.00% #9000001, which holds $650.90 USDC and 0 NOCK now.',
    );
    // The two rungs left keep their numbers and their exact mints.
    for (const i of [0, 1]) {
      expect(LADDER_BOTTLE_DEEP_DEPLOYED.rungs[i].index).toBe(i + 1);
      expect(LADDER_BOTTLE_DEEP_DEPLOYED.rungs[i].mint).toEqual(LADDER_BOTTLE.rungs[i].mint);
    }
    expect(ladderCardView(LADDER_BOTTLE_CAP_NEAR_DEPLOYED, 'fresh').headline).toBe('Ladder: 1 of 3 deposits left, up to $650.90 USDC.');
    expect(ladderList(LADDER_BOTTLE_CAP_NEAR_DEPLOYED).map((i) => i.kind)).toEqual(['row', 'row', 'deposit']);
    expect(rungTitle(LADDER_BOTTLE_CAP_NEAR_DEPLOYED.rungs[0])).toBe('Deposit 3 of 3 (deep rung): $650.89 USDC, no NOCK, below the price now');
  });

  it('all deployed, a passed rung and an over rung each say so in one line', () => {
    const all = ladderCardView(LADDER_BOTTLE_ALL_DEPLOYED, 'fresh');
    expect(all).toEqual({
      mode: 'line',
      headline: 'Ladder deployed: all 3 deposits are done.',
      line: 'Nothing is left to deposit: of the 3 rungs of your shape, 3 deployed. Their readings are in the Ladders card.',
    });
    expect(shapeRowLines(LADDER_BOTTLE_ALL_DEPLOYED)).toHaveLength(3);
    expect(shapeRowLines(LADDER_BOTTLE_ALL_DEPLOYED)[0]).toBe('Deposit 1 of 3 (cap): done. It is deployed in v4 USDC/NOCK 3.00% #9000002, which holds $1.23 USDC and 87 NOCK now.');
    const passed = ladderList(LADDER_BOTTLE_NEAR_PASSED);
    expect(passed.map((i) => i.kind)).toEqual(['deposit', 'row', 'deposit']);
    expect(passed[1].kind === 'row' && passed[1].row.text).toBe(
      "Deposit 2 of 3 (near rung): passed. The price is below this rung's range; a USDC-only deposit there is not possible.",
    );
    expect(ladderCardView(LADDER_BOTTLE_OVER, 'fresh').mode).toBe('line');
    expect(shapeRowLines(LADDER_BOTTLE_OVER)[2]).toMatch(/^Deposit 3 of 3 \(deep rung\): over its target, so nothing more is listed for it\. The tracked rung holds \$[\d.]+ USDC more than its target\.$/);
    expect(shapeRowLines(LADDER_READY_LIVE)).toEqual([]);
  });

  it('the picture: a bar per rung, the cap straddling spot, labels that fit a phone', () => {
    for (const width of [520, 340]) {
      const g = pictureGeometry(LADDER_BOTTLE.picture!, width)!;
      expect(g.bars).toHaveLength(3);
      const [cap, near, deep] = g.bars;
      // The cap straddles the spot marker; the others sit left of it, deep lowest.
      expect(cap.x).toBeLessThan(g.spotX!);
      expect(cap.x + cap.w).toBeGreaterThan(g.spotX!);
      expect(near.x + near.w).toBeLessThanOrEqual(cap.x + 0.001);
      expect(deep.x + deep.w).toBeLessThanOrEqual(near.x + 0.001);
      expect(g.bottomX).toBeLessThanOrEqual(deep.x + 0.001);
      expect(g.bars.map((b) => b.label)).toEqual(['$1.23 + 87 NOCK', '$114.86', '$650.89']);
      // The cap's bar is too narrow for its label, which sits beside it, clear of the
      // spot line and of the bottom line; the wide bars carry theirs inside.
      expect(cap.labelInside).toBe(false);
      expect(cap.labelAnchor).toBe('end');
      const capLabelLeft = cap.labelX - labelWidth(cap.label, g.fontSize);
      expect(cap.labelX).toBeLessThan(g.spotX!);
      expect(capLabelLeft).toBeGreaterThan(g.bottomX);
      expect(near.labelInside && deep.labelInside).toBe(true);
      // Every label lies inside the drawing, at a readable size.
      for (const b of g.bars) {
        const lw = labelWidth(b.label, g.fontSize);
        const leftEdge = b.labelAnchor === 'end' ? b.labelX - lw : b.labelAnchor === 'start' ? b.labelX : b.labelX - lw / 2;
        expect(leftEdge).toBeGreaterThanOrEqual(0);
        expect(leftEdge + lw).toBeLessThanOrEqual(g.width);
      }
      expect(g.fontSize).toBeGreaterThanOrEqual(12);
      expect(g.rows.deployedLegend).toBeNull();
    }
    // At 375px the phone's 340-unit drawing is about 309px wide: 12 units ≈ 10.9px.
    expect((pictureGeometry(LADDER_BOTTLE.picture!, 340)!.fontSize * 309) / 340).toBeGreaterThan(10.5);
    // A deployed bar is marked, and the drawing grows a legend row for it.
    const done = pictureGeometry(LADDER_BOTTLE_DEEP_DEPLOYED.picture!, 340)!;
    expect(done.bars.map((b) => b.deployed)).toEqual([false, false, true]);
    expect(done.bars[2].label).toBe('$650.90');
    expect(done.rows.deployedLegend).toBeGreaterThan(done.rows.legend);
    expect(done.height).toBeGreaterThan(pictureGeometry(LADDER_BOTTLE.picture!, 340)!.height);
  });
});
