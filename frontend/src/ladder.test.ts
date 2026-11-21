import { describe, expect, it } from 'vitest';
import { existsSync, readFileSync } from 'node:fs';
import {
  EMPTY_LADDER_FORM,
  buildLadderQuery,
  ladderErrorLines,
  ladderNockMoved,
  ladderPoolOptions,
  ladderSnapshotLine,
  ladderSummaryLines,
  validateLadderForm,
  type LadderForm,
} from './ladder';
import { simulateLadder } from './api/client';
import { CHAT_900_UNSNAPPED, NOCK_TOO_MUCH, SPOT_741_SNAPPED } from './ladder.fixture';
import type { Position } from './types/snapshot';

const form = (over: Partial<LadderForm>): LadderForm => ({ ...EMPTY_LADDER_FORM, ...over });

describe('ladder simulator (#166)', () => {
  it('the fixture is the worker golden file, body for body', () => {
    const golden = new URL('../../ingestion-worker/testdata/ladder/golden_ladder.json', import.meta.url);
    // A frontend-only checkout has no worker directory; the copy is then all there is.
    if (!existsSync(golden)) return;
    const doc = JSON.parse(readFileSync(golden, 'utf8'));
    expect(SPOT_741_SNAPPED).toEqual(doc.spot_741_snapped.body);
    expect(CHAT_900_UNSNAPPED).toEqual(doc.chat_900_unsnapped.body);
    expect(NOCK_TOO_MUCH).toEqual(doc.nock_too_much.body);
  });

  it('builds the query the worker reads', () => {
    expect(buildLadderQuery(form({ usdc: '741', low: '0.01', highMode: 'spot' })).toString()).toBe('usdc=741&low=0.01&high=spot');
    expect(
      buildLadderQuery(form({ usdc: '900', lowMode: 'pct', low: '-30', highMode: 'pct', high: '2', pool: 'uniswap-v4:base:3163450' })).toString(),
    ).toBe('usdc=900&low_pct=-30&high_pct=2&position_id=uniswap-v4%3Abase%3A3163450');
    expect(buildLadderQuery(form({ usdc: '1', nock: '5', low: '0.01', highMode: 'usd', high: '0.02', pool: 'spacing', tickSpacing: '60', gridToPct: '-20' })).toString()).toBe(
      'usdc=1&nock=5&low=0.01&high=0.02&tick_spacing=60&grid_to_pct=-20',
    );
  });

  it('checks the form the way the worker does', () => {
    expect(validateLadderForm(EMPTY_LADDER_FORM)).toBe('USDC must be a number above 0');
    expect(validateLadderForm(form({ usdc: '741' }))).toBe('the bottom must be a number');
    expect(validateLadderForm(form({ usdc: '741', low: '0.013', highMode: 'usd', high: '0.012' }))).toBe('the bottom must be below the top');
    expect(validateLadderForm(form({ usdc: '741', lowMode: 'pct', low: '-100' }))).toBe('the bottom must be above -100%');
    expect(validateLadderForm(form({ usdc: '741', low: '0.01', pool: 'spacing', tickSpacing: '0' }))).toContain('tick spacing');
    expect(validateLadderForm(form({ usdc: '741', low: '0.01', nock: '-1' }))).toContain('NOCK');
    expect(validateLadderForm(form({ usdc: '741', low: '0.01' }))).toBeNull();
  });

  it('never prefills an amount', () => {
    expect(EMPTY_LADDER_FORM.usdc).toBe('');
    expect(EMPTY_LADDER_FORM.nock).toBe('');
    expect(EMPTY_LADDER_FORM.low).toBe('');
    expect(EMPTY_LADDER_FORM.high).toBe('');
  });

  it('words the 741 case as measurements', () => {
    const lines = ladderSummaryLines(SPOT_741_SNAPPED);
    expect(lines).toContain('Sizing: $741.00 USDC and 0 NOCK at spot');
    expect(lines).toContain('Bottom: fully NOCK at $0.01002405 (-22.56%): 65,563 NOCK held, average $0.01130202');
    expect(lines.join('\n')).toContain('holds by $26.45 (this range takes at most $767.45)');
    expect(lines.join('\n')).toContain('$0.01313098 (+1.44%) needs 3,409 NOCK (exceeds the budget)');
    expect(lines.join('\n')).toContain('Below the bottom: at $0.00776640 (-40.00%) stable $930.69 (reserve -$19.31); a measurement, not the gate');
    expect(lines.join('\n')).toContain('Floor budget: needs 0 of the 87 NOCK that may go into LP: fits');
    expect(ladderSnapshotLine(SPOT_741_SNAPPED)).toBe('From the snapshot generated 2026-11-20T12:00:00Z (5 min ago) · worker health: ok');
    const text = lines.join(' ').toLowerCase();
    for (const w of ['deposit', 'should', 'recommend']) expect(text).not.toContain(w);
  });

  it('states a breached reserve and an exceeded budget as facts', () => {
    const lines = ladderSummaryLines(CHAT_900_UNSNAPPED).join('\n');
    expect(lines).toContain('breached: below it by $151.37 (this range takes at most $748.63)');
    expect(lines).toContain('needs 4,194 of the 87 NOCK that may go into LP: exceeds; $18.62 is the most USDC this range holds within it');
    expect(lines).toContain('average $0.01082973');
    expect(lines).toContain('Floor-crossing price: lowers:');
  });

  it('keeps the 422 body: the sub-code, nock_needed as a measured line, the bounds', () => {
    expect(ladderErrorLines(NOCK_TOO_MUCH)).toEqual([
      'NOCK_TOO_MUCH: this range at the pool price takes 0.0000 NOCK with $741.00 USDC; 50 was given',
      'Measured: this range needs 0.0000 NOCK at the pool price for this USDC',
      'Bounds: $0.01000000 – $0.01294400 (-22.74% / 0.00%)',
    ]);
  });

  it('the client returns 422 and 400 bodies whole and throws on anything else', async () => {
    const reply = (status: number, body: unknown) =>
      (async () => new Response(JSON.stringify(body), { status, headers: { 'Content-Type': 'application/json' } })) as unknown as typeof fetch;
    const q = new URLSearchParams('usdc=741&low=0.01&high=spot&nock=50');
    expect(await simulateLadder(q, reply(422, NOCK_TOO_MUCH))).toEqual({ ok: false, status: 422, error: NOCK_TOO_MUCH });
    const bad = { error: 'INVALID_PARAMETER', detail: 'low must be above 0 USD per NOCK: 0' };
    expect(await simulateLadder(q, reply(400, bad))).toEqual({ ok: false, status: 400, error: bad });
    expect(await simulateLadder(q, reply(200, SPOT_741_SNAPPED))).toEqual({ ok: true, sim: SPOT_741_SNAPPED });
    await expect(simulateLadder(q, reply(503, { error: 'NO_SNAPSHOT', message: 'No snapshot available yet' }))).rejects.toMatchObject({
      status: 503,
      code: 'NO_SNAPSHOT',
    });
  });

  it('lists the pools a ladder can snap to, and says why the others cannot', () => {
    const base = { token0: { symbol: 'USDC' }, token1: { symbol: 'NOCK' }, pool_id: '0xabc', nft_id: 1 } as unknown as Position;
    const opts = ladderPoolOptions([
      { ...base, position_id: 'uniswap-v4:base:3163450', protocol: 'uniswap-v4', nft_id: 3163450, tick_spacing: 300 },
      { ...base, position_id: 'aerodrome:base:0x770f', protocol: 'aerodrome', nft_id: 0, full_range: true },
      { ...base, position_id: 'uniswap-v3:base:9', protocol: 'uniswap-v3', nft_id: 9 },
      { ...base, position_id: 'uniswap-v3:base:6607225', protocol: 'uniswap-v3', token0: { symbol: 'WETH' } } as unknown as Position,
    ]);
    expect(opts.map((o) => o.value)).toEqual(['', 'spacing', 'uniswap-v4:base:3163450', 'aerodrome:base:0x770f', 'uniswap-v3:base:9']);
    expect(opts[2]).toMatchObject({ disabled: false, label: 'uniswap-v4 USDC/NOCK #3163450 (tick spacing 300)' });
    expect(opts[3]).toMatchObject({ disabled: true, reason: 'a basic Aero pool has no ranges: its deposits are full range' });
    expect(opts[4]).toMatchObject({ disabled: true, reason: 'tick spacing not read for this pool' });
  });

  it('states the budget note: no floor set, and a negative budget with its reason', () => {
    const noFloor = structuredClone(SPOT_741_SNAPPED);
    noFloor.floor_check.lp_nock_budget = '90086.7891';
    noFloor.floor_check.budget_note = "no treasury_floor_nock is set: the budget is the wallet's NOCK";
    expect(ladderSummaryLines(noFloor)).toContain(
      "Floor budget: needs 0 of the 90,087 NOCK that may go into LP (no treasury_floor_nock is set: the budget is the wallet's NOCK): fits",
    );
    const negative = structuredClone(CHAT_900_UNSNAPPED);
    negative.floor_check.lp_nock_budget = '-40000.0000';
    negative.floor_check.budget_note = 'none: wallet NOCK is 40000 below the 100000 NOCK floor less the 10000 tolerated below it';
    negative.floor_check.max_usdc_within_budget = '0.00';
    expect(ladderSummaryLines(negative)).toContain(
      'Floor budget: needs 4,194 of the -40,000 NOCK that may go into LP (none: wallet NOCK is 40000 below the 100000 NOCK floor less the 10000 tolerated below it): exceeds; $0.00 is the most USDC this range holds within it',
    );
    // With no note the line is the plain one.
    expect(ladderSummaryLines(SPOT_741_SNAPPED)).toContain('Floor budget: needs 0 of the 87 NOCK that may go into LP: fits');
  });

  it('says NOCK sold above spot, never a negative purchase', () => {
    expect(ladderNockMoved('-4193.9773')).toBe('4,194 sold');
    expect(ladderNockMoved('11058.4211')).toBe('11,058 bought');
    expect(ladderNockMoved('0.0000')).toBe('0 bought');
    expect(ladderNockMoved('-0.0000')).toBe('0 bought');
  });
});
