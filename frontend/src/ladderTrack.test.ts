import { describe, expect, it } from 'vitest';
import { existsSync, readFileSync } from 'node:fs';
import {
  isLadder,
  isNockLp,
  ladderCountClause,
  ladderCurveLine,
  ladderHoldsLine,
  ladderLiteLine,
  ladderRangeLine,
  ladderReadingFor,
  ladderRungText,
  ladderSinceDeposit,
  ladderStatus,
  ladderSummaryClause,
  laddersOf,
} from './ladderTrack';
import { hasNockLeg, nockBracketRows, positionsSummary, rangeStatus } from './positions';
import { reserveText } from './reserve';
import { costBasisView, ladderBasisClause, ladderChartNote } from './costBasis';
import { buildPositionSeriesDefs } from './timeline';
import {
  COST_BASIS_BOUGHT,
  DEVIATION_ABOVE,
  LADDERS_ABOVE,
  LADDERS_BOUGHT,
  POSITIONS_ABOVE,
  POSITIONS_BOUGHT,
  RESERVE_ABOVE,
} from './ladderTrack.fixture';
import type { LadderReading } from './types/snapshot';

const near = (l: LadderReading[]) => l.find((r) => r.position_id === 'uniswap-v4:base:3240369')!;
const deep = (l: LadderReading[]) => l.find((r) => r.position_id === 'uniswap-v4:base:3299001')!;

describe('tracked ladders (#167)', () => {
  it('the fixture is the worker golden file', () => {
    const golden = new URL('../../ingestion-worker/testdata/golden_snapshot_ladder.json', import.meta.url);
    // A frontend-only checkout has no worker directory; the copy is then all there is.
    if (!existsSync(golden)) return;
    const doc = JSON.parse(readFileSync(golden, 'utf8'));
    expect(LADDERS_ABOVE).toEqual(doc.ladders_above_range.snapshot.ladders);
    expect(LADDERS_BOUGHT).toEqual(doc.near_rung_bought_plan_due.snapshot.ladders);
    expect(POSITIONS_ABOVE).toEqual(doc.ladders_above_range.snapshot.positions);
    expect(COST_BASIS_BOUGHT).toEqual(doc.near_rung_bought_plan_due.snapshot.derived_metrics.cost_basis);
  });

  it('a ladder holds NOCK but is not NOCK LP', () => {
    const roles = POSITIONS_ABOVE.map((p) => [p.nft_id, isLadder(p), isNockLp(p), hasNockLeg(p)]);
    expect(roles).toEqual([
      [3163450, false, true, true],
      [6336662, false, true, true],
      [3240369, true, false, true],
      [3299001, true, false, true],
    ]);
    expect(isNockLp({ token0: { symbol: 'WETH' }, token1: { symbol: 'USDC' } })).toBe(false);
    expect(laddersOf({})).toEqual([]);
    expect(laddersOf({ ladders: LADDERS_ABOVE })).toHaveLength(2);
    expect(ladderReadingFor(POSITIONS_ABOVE[2], LADDERS_ABOVE)?.config_id).toBe('ladder-near');
    expect(ladderReadingFor(POSITIONS_ABOVE[0], LADDERS_ABOVE)).toBeUndefined();
  });

  it('badges a ladder by what it holds, never as OUT OF RANGE', () => {
    expect(ladderStatus(near(LADDERS_ABOVE))).toEqual({ label: '● LADDER ABOVE RANGE (ALL USDC)', tone: 'neutral' });
    expect(ladderStatus(near(LADDERS_BOUGHT))).toEqual({ label: '● LADDER IN RANGE (CONVERTS BOTH WAYS)', tone: 'good' });
    expect(ladderStatus({ state: 'holds_only_nock' })).toEqual({ label: '● LADDER BELOW RANGE (ALL NOCK)', tone: 'neutral' });
    expect(ladderStatus({ state: 'empty' })).toEqual({ label: '● LADDER EMPTY', tone: 'neutral' });
    expect(ladderStatus({ state: 'pool_stale' })).toEqual({ label: '● STALE POOL', tone: 'stale' });
    expect(ladderStatus(undefined).label).toBe('● LADDER (STATE NOT REPORTED)');
    // On a position card the ladder's own badge replaces the in/out-of-range pill.
    const p = POSITIONS_ABOVE[2];
    expect(p.in_range).toBe(false);
    expect(rangeStatus(p, near(LADDERS_ABOVE)).label).toBe('● LADDER ABOVE RANGE (ALL USDC)');
    expect(rangeStatus(p, near(LADDERS_ABOVE)).label).not.toContain('OUT OF RANGE');
    // A position that is not a ladder keeps its pill.
    expect(rangeStatus(POSITIONS_ABOVE[0])).toEqual({ label: '● IN RANGE', tone: 'good' });
  });

  it('states each reading in the worker figures', () => {
    const r = near(LADDERS_BOUGHT);
    expect(ladderRungText(r)).toBe('rung near');
    expect(ladderRungText(deep(LADDERS_BOUGHT))).toBe('rung unset');
    // The bottle's cap (2.10.0) reads like the other rungs.
    expect(ladderRungText({ ...r, rung: 'cap' })).toBe('rung cap');
    expect(ladderRangeLine(deep(LADDERS_ABOVE))).toBe('$0.00678699 – $0.01032932 (−67.16% / −50.01% of spot)');
    // Its bottom is the pool's lowest usable price: said in words, never $0.00000000.
    expect(ladderRangeLine(r)).toBe("no bottom – $0.01720088 (no bottom / +16.18% of spot); the bottom reaches the pool's lowest usable price (tick 887100): the range buys all the way down, so no bottom price and no NOCK at the bottom are stated");
    expect(ladderRangeLine({ ...r, spot_price_usd: null, high_pct_of_spot: null })).toBe("no bottom – $0.01720088 (no spot to compare); the bottom reaches the pool's lowest usable price (tick 887100): the range buys all the way down, so no bottom price and no NOCK at the bottom are stated");
    expect(ladderHoldsLine(r)).toBe('Holds $1137.51 USDC + 5551.3674 NOCK ($1219.69)');
    expect(ladderCurveLine(r)).toBe(
      'Against its all-USDC top ($1226.09 USDC): 5551.3674 NOCK held, $88.59 converted (7.23%), curve average $0.01595807; all NOCK at the bottom: not stated (no bottom)',
    );
    expect(ladderLiteLine(deep(LADDERS_ABOVE))).toBe('v4 USDC/NOCK 3.00% #3299001: above range, all USDC · $77.00 USDC + 0 NOCK');
  });

  it('what it bought comes from the verified receipt only; otherwise the reason', () => {
    expect(ladderSinceDeposit(near(LADDERS_BOUGHT))).toEqual({
      text: 'Since deposit (tx 0xafe42fe2…, block 59660587): net 5303.7885 NOCK bought for net $84.35 USDC spent, average $0.01590283',
      verified: true,
    });
    // Sold back since the deposit: signed figures, no average.
    expect(ladderSinceDeposit(near(LADDERS_ABOVE)).text).toBe(
      'Since deposit (tx 0xafe42fe2…, block 59660587): net -247.5790 NOCK bought for net $-4.24 USDC spent; no average price (it has not net bought NOCK with USDC)',
    );
    expect(ladderSinceDeposit(deep(LADDERS_ABOVE))).toEqual({
      text: 'Since deposit: no baseline: no deposit_tx is set for this ladder in config.yaml: NockNock has no baseline for what it bought and does not invent one',
      verified: false,
    });
    const changed: LadderReading = {
      ...near(LADDERS_ABOVE),
      since_deposit: { ...near(LADDERS_ABOVE).since_deposit, status: 'liquidity_changed', net_nock_bought: null, net_usdc_spent: null, reason: 'liquidity was added or partly withdrawn since the deposit' },
    };
    expect(ladderSinceDeposit(changed)).toEqual({ text: 'Since deposit: liquidity changed: liquidity was added or partly withdrawn since the deposit', verified: false });
  });

  it('the summary leaves ladders out of the range counts and names them, as the worker does', () => {
    // Without a ladder the line is what it was.
    expect(positionsSummary(POSITIONS_ABOVE.slice(0, 2))).toBe('2 LP positions tracked live: 2 ranged (2 in range).');
    expect(positionsSummary(POSITIONS_ABOVE, LADDERS_ABOVE)).toBe(
      '2 LP positions tracked live: 2 ranged (2 in range). +2 ladders, measured only (2 holding only USDC).',
    );
    expect(positionsSummary(POSITIONS_BOUGHT, LADDERS_BOUGHT)).toBe(
      '2 LP positions tracked live: 2 ranged (2 in range). +2 ladders, measured only (1 holding only USDC, 1 in range).',
    );
    expect(positionsSummary(POSITIONS_ABOVE.slice(2), LADDERS_ABOVE)).toBe('No NOCK LP positions tracked. +2 ladders, measured only (2 holding only USDC).');
    expect(ladderSummaryClause(POSITIONS_ABOVE.slice(0, 2), LADDERS_ABOVE)).toBe('');
    // A ladder whose reading is missing says so, never a state.
    expect(ladderSummaryClause(POSITIONS_ABOVE.slice(2, 3), [])).toBe(' +1 ladder, measured only (1 state not reported).');
  });

  it('the Goal 1 line counts NOCK LP and names ladders apart', () => {
    expect(POSITIONS_ABOVE.filter(isNockLp)).toHaveLength(2);
    expect(ladderCountClause(POSITIONS_ABOVE)).toBe(' and 2 ladders');
    expect(ladderCountClause(POSITIONS_ABOVE.slice(0, 3))).toBe(' and 1 ladder');
    expect(ladderCountClause(POSITIONS_ABOVE.slice(0, 2))).toBe('');
  });

  it('the NOCK bracket chart and the NOCK stack draw a ladder, named as one', () => {
    const brackets = POSITIONS_ABOVE.map((p) => ({ position_id: p.position_id, lower_price: p.lower_price, upper_price: p.upper_price, liquidity_usd: p.amount_usd, in_range: p.in_range }));
    const rows = nockBracketRows(brackets, POSITIONS_ABOVE, 1800, 0.02066);
    expect(rows.map((r) => [r.name, r.ladder])).toEqual([
      ['USDC/NOCK v4 · 3%', false],
      ['WETH/NOCK v3 · 1%', false],
      ['USDC/NOCK v4 · 3% (ladder)', true],
      ['USDC/NOCK v4 · 3% (ladder)', true],
    ]);
    const defs = buildPositionSeriesDefs(
      { asset_aggregation_series: { positions_nock: POSITIONS_ABOVE.map((p) => ({ position_id: p.position_id, series: ['0'] })) } },
      POSITIONS_ABOVE,
    );
    expect(defs.map((d) => d.name)).toEqual(['USDC/NOCK v4 · 3%', 'WETH/NOCK v3 · 1%', 'USDC/NOCK v4 · 3% (ladder)', 'USDC/NOCK v4 · 3% (ladder)']);
  });

  it('the reserve line names the ladder USDC it counts, like the MCP line', () => {
    expect(reserveText(RESERVE_ABOVE)).toBe('$950.00 protected reserve (wallet + NOCK LP and ladder USDC, $1,303.10 in ladders): $1,535.13 above it');
    // A ladder holding no USDC is not named; no ladder field is the old line.
    expect(reserveText({ ...RESERVE_ABOVE, reserve_ladder_usdc: '0.00' })).toBe('$950.00 protected reserve (wallet + NOCK LP USDC): $1,535.13 above it');
    expect(reserveText({ ...RESERVE_ABOVE, reserve_ladder_usdc: undefined })).toBe('$950.00 protected reserve (wallet + NOCK LP USDC): $1,535.13 above it');
  });

  it('the cost panel says which ladder buys the whole-stack average counts', () => {
    const notes = costBasisView(COST_BASIS_BOUGHT as any).notes.join('\n');
    expect(notes).toContain(
      'Whole stack counts: The net NOCK bought by 1 of 2 tracked ladders since a receipt-verified deposit after 2026-11-19T00:00:00+00:00: 5303.7885 NOCK for $84.35, measured from each deposit_tx receipt to now.',
    );
    expect(notes).toContain('Tracked ladders left out: uniswap-v4:base:3299001: no verified deposit baseline (no_baseline).');
  });

  it('the whole-stack tile names the ladder buys its total includes, and the chart says it does not draw them', () => {
    const tile = costBasisView(COST_BASIS_BOUGHT as any).tiles.find((t) => t.label === 'WHOLE STACK · AVERAGE COST')!;
    expect(tile.value).toBe('$0.01687');
    expect(tile.sub).toContain('plus 0 verified buys since and the net buys of 1 of 2 tracked ladders since a verified deposit (5303.7885 NOCK for $84.35).');
    // The parts add up: 90,086.79 opening + 5,303.7885 ladder NOCK = the basis.
    expect(COST_BASIS_BOUGHT.whole_stack.basis_nock).toBe('95390.5785');
    expect(ladderChartNote(COST_BASIS_BOUGHT as any)).toBe(
      "The whole-stack line counts the opening basis and verified buys only. The tile's $0.01687 also counts the net buys of 1 tracked ladder since a verified deposit (5303.7885 NOCK for $84.35), which this chart does not draw: a ladder converts continuously, not at recorded buys.",
    );
    // No ladder buy counted, or no ladder: no note, and the tile reads as before.
    expect(ladderChartNote({ ...COST_BASIS_BOUGHT, whole_stack: { ...COST_BASIS_BOUGHT.whole_stack, ladder_buys_since: { ladders_counted: 0, nock: '0.0000', usdc: '0.00', not_counted: ['x', 'y'] } } } as any)).toBeNull();
    const plain = { ...COST_BASIS_BOUGHT.whole_stack } as any;
    delete plain.ladder_buys_since;
    expect(ladderBasisClause(plain)).toBe('');
    expect(ladderChartNote({ ...COST_BASIS_BOUGHT, whole_stack: plain } as any)).toBeNull();
  });

  it('the deviation matrix carries the ladder row last, with no target', () => {
    const last = DEVIATION_ABOVE[DEVIATION_ABOVE.length - 1];
    expect(last).toEqual({ bucket_name: 'NOCK ladders (measured)', target_weight_bps: 0, actual_weight_bps: 2576, deviation_bps: 2576, status: 'no_target' });
  });
});
