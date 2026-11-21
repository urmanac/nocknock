import { describe, expect, it } from 'vitest';
import { existsSync, readFileSync } from 'node:fs';
import {
  type FoundClient,
  checkNow,
  choiceLabel,
  choiceTitle,
  detectionHasRows,
  detectionSummary,
  foundLines,
  foundTitle,
  ignoredLine,
  listHeading,
  mintTxUrl,
  positionDetectionOf,
  restoreFound,
  shortHash,
  tapFound,
  tapTracked,
  trackedLine,
  trackedState,
  whyNotAutomatic,
} from './foundPositions';
import {
  DETECTION_DISMISSED,
  DETECTION_FOUND,
  DETECTION_NOTHING_FOUND,
  DETECTION_PARTIAL,
  DETECTION_TRACKED_BY_ITSELF,
  DETECTION_TRACKED_BY_TAP,
  DETECTION_UNAVAILABLE,
} from './foundPositions.fixture';
import { claimActionKey } from './claimActions';
import { closedPositionConfigHint } from './closedPositions';
import { ladderRungText } from './ladderTrack';

const MINT = '0x6cdabada1ff062b6d3b4ba87bfe04158dd8b14a91e93cabe11dc3e35b587e89e';

/** A client that records what was called and answers as told. */
function client(overrides: Partial<FoundClient> = {}): FoundClient & { calls: string[] } {
  const calls: string[] = [];
  const base: FoundClient = {
    trackFoundPosition: async (nft, as) => {
      calls.push(`track ${nft} ${as}`);
      return { note: 'Tracked as a ladder: measured, never steered, and counted from the next cycle.', cycle: 'ok' };
    },
    untrackPosition: async (id) => {
      calls.push(`untrack ${id}`);
      return { note: 'NFT #3163450 is still in your wallet and is no longer counted.', cycle: 'ok' };
    },
    dismissFoundPosition: async (nft) => {
      calls.push(`dismiss ${nft}`);
      return { note: 'Set aside: it is not counted and stays listed, and can be restored.' };
    },
    restoreFoundPosition: async (nft) => {
      calls.push(`restore ${nft}`);
      return { note: 'Back on offer: it is still not counted until you track it.' };
    },
    ingestNow: async () => {
      calls.push('ingest');
    },
  };
  return Object.assign({ ...base, ...overrides }, { calls });
}

describe('positions found on-chain (v0.9.0)', () => {
  it('the fixture is the worker golden file, built from recorded chain answers', () => {
    const golden = new URL('../../ingestion-worker/testdata/position_detect/golden_position_detect.json', import.meta.url);
    if (!existsSync(golden)) return;
    const doc = JSON.parse(readFileSync(golden, 'utf8'));
    expect(DETECTION_NOTHING_FOUND).toEqual(doc.nothing_found.position_detection);
    expect(DETECTION_FOUND).toEqual(doc.found_not_counted.position_detection);
    expect(DETECTION_DISMISSED).toEqual(doc.dismissed.position_detection);
    expect(DETECTION_TRACKED_BY_ITSELF).toEqual(doc.tracked_by_itself.position_detection);
    expect(DETECTION_TRACKED_BY_TAP).toEqual(doc.tracked_by_tap.position_detection);
    expect(DETECTION_PARTIAL).toEqual(doc.partial.position_detection);
    expect(DETECTION_UNAVAILABLE).toEqual(doc.unavailable.position_detection);
  });

  it('reads the snapshot field and tolerates an older worker', () => {
    expect(positionDetectionOf({ derived_metrics: { position_detection: DETECTION_FOUND } })).toBe(DETECTION_FOUND);
    expect(positionDetectionOf({ derived_metrics: {} })).toBeNull();
    expect(positionDetectionOf({ derived_metrics: { position_detection: null } })).toBeNull();
    expect(positionDetectionOf(null)).toBeNull();
  });

  it('says what was checked, and never "nothing new" for a check that did not run', () => {
    const nothing = detectionSummary(DETECTION_NOTHING_FOUND);
    expect(nothing.tone).toBe('ok');
    expect(nothing.heading).toBe('CHECKED');
    expect(nothing.text).toBe(
      "Nothing new: no position is found that is not already counted. Every Uniswap v4 position NFT sent to the wallet was read from the transfer logs, from the first block to this cycle's block.",
    );
    // With rows to show, the line is the coverage alone.
    expect(detectionSummary(DETECTION_FOUND).text).toBe(DETECTION_FOUND.scan!.text);
    expect(detectionSummary(DETECTION_FOUND).text).not.toContain('Nothing new');

    const partial = detectionSummary(DETECTION_PARTIAL);
    expect(partial.tone).toBe('warn');
    expect(partial.heading).toBe('CHECKED IN PART');
    expect(partial.text).toContain('transfers before block 52931286 are not read yet');
    expect(partial.text).toContain('the wallet holds 43 Uniswap v4 position NFTs on-chain and 3 are known: 40 not found yet');

    const unavailable = detectionSummary(DETECTION_UNAVAILABLE);
    expect(unavailable.tone).toBe('warn');
    expect(unavailable.heading).toBe('COULD NOT CHECK');
    expect(unavailable.text).toMatch(/^New positions could not be checked this cycle \(.+\): a deposit made since the last check would not be counted yet\./);
    for (const d of [DETECTION_PARTIAL, DETECTION_UNAVAILABLE]) {
      expect(detectionSummary(d).text).not.toMatch(/nothing new|none found/i);
    }
    expect(detectionSummary({ ...DETECTION_NOTHING_FOUND, status: 'not_run', reason: 'no tracked wallet is configured' })).toEqual({
      tone: 'muted',
      heading: 'NOT LOOKED FOR',
      text: 'no tracked wallet is configured',
    });
    // A status this page does not know is not read as a good one.
    expect(detectionSummary({ ...DETECTION_NOTHING_FOUND, status: 'something_else', reason: null }).tone).toBe('warn');
  });

  it('counts what the wallet holds that is never listed, by reason', () => {
    expect(ignoredLine(DETECTION_NOTHING_FOUND)).toBe('Not listed: 1 in config.yaml, 37 not on the token whitelist, 5 empty.');
    expect(ignoredLine(DETECTION_FOUND)).toBe('Not listed: 37 not on the token whitelist, 5 empty.');
    expect(ignoredLine({ ...DETECTION_FOUND, ignored: {} })).toBe('');
    expect(ignoredLine({ ...DETECTION_FOUND, ignored: { hooks: 2, gone: 1, not_owned: 1, unsupported: 1 } })).toBe(
      'Not listed: 1 no longer in the wallet, 2 in a hooked pool, 1 with a token NockNock cannot read in a position yet, 1 burned.',
    );
  });

  it('states a found position as facts: what it is, what it holds, why it waits', () => {
    const p = DETECTION_FOUND.found_not_counted.positions[0];
    expect(foundTitle(p)).toBe('v4 USDC/NOCK 3.00% #3163450');
    expect(foundLines(p)).toEqual([
      'Ticks 262200 to 282300 ($0.00550148 to $0.04105518 per NOCK)',
      'Holds 120.02 USDC + 8114.3626 NOCK, worth $240.61',
    ]);
    expect(whyNotAutomatic(p)).toBe('Not tracked by itself: no ladder rung is suggested now to match it against.');
    expect(listHeading(DETECTION_FOUND.found_not_counted, 'found on-chain and NOT counted in any total')).toBe(
      '1 position, $240.61, found on-chain and NOT counted in any total',
    );
    expect(listHeading(DETECTION_DISMISSED.dismissed, 'not counted')).toBe('1 position, $240.61, not counted');
    // A position that was not read this cycle states no value, and says why.
    const stale = DETECTION_UNAVAILABLE.found_not_counted.positions[0];
    expect(stale.value_usd).toBeNull();
    expect(foundLines(stale)).toEqual([
      'Ticks 262200 to 282300 ($0.00550148 to $0.04105518 per NOCK)',
      'It was not read this cycle, so what it holds and its value are not stated.',
    ]);
    expect(listHeading(DETECTION_UNAVAILABLE.found_not_counted, 'not counted')).toBe(
      '1 position (1 of 1 could not be valued this cycle, so no total is stated), not counted',
    );
    expect(foundLines(stale).join(' ')).not.toMatch(/\$0\.00(?!\d)|worth/);
    expect(whyNotAutomatic({ ...p, why_not_automatic: [] })).toBe('');
  });

  it('labels the one-tap choices and says what each does', () => {
    expect(DETECTION_FOUND.found_not_counted.positions[0].choices.map(choiceLabel)).toEqual([
      'TRACK AS CAP RUNG',
      'TRACK AS NEAR RUNG',
      'TRACK AS DEEP RUNG',
      'TRACK AS LP',
      'NOT NOW',
    ]);
    expect(choiceLabel('ladder:cap')).toBe('TRACK AS CAP RUNG');
    expect(choiceLabel('untrack')).toBe('UNTRACK');
    expect(choiceTitle('lp')).toContain('never a rebuild target');
    expect(choiceTitle('ladder:deep')).toContain('with its mint as the baseline');
    expect(choiceTitle('dismiss')).toContain('can be restored');
    expect(choiceTitle('untrack')).toContain('The NFT stays in your wallet');
    for (const c of ['lp', 'dismiss', 'untrack', 'ladder:near']) {
      expect(choiceTitle(c)).not.toMatch(/you should|we recommend|must/i);
    }
  });

  it('states a row tracked from the chain: what as, how, and whether it is counted', () => {
    const auto = DETECTION_TRACKED_BY_ITSELF.tracked[0];
    expect(trackedLine(auto)).toBe('NFT #3163450: ladder, rung not set; tracked by itself: it matched a suggested rung');
    expect(trackedState(auto)).toEqual({ counted: true, text: 'Counted in the totals.' });
    expect(auto.deposit_tx).toBe(MINT);
    expect(auto.choices).toEqual(['ladder:cap', 'ladder:near', 'ladder:deep', 'untrack']);
    const tap = DETECTION_TRACKED_BY_TAP.tracked[0];
    expect(trackedLine(tap)).toBe('NFT #3163450: ladder, deep rung; tracked by your tap');
    expect(tap.choices).toEqual(['untrack']);
    expect(trackedLine({ ...tap, role: null, rung: null })).toBe('NFT #3163450: LP, never a rebuild target; tracked by your tap');
    // Every state that is not "counted" says so, with the worker's reason.
    expect(trackedState({ ...tap, state: 'empty', note: 'it holds nothing (emptied; the NFT is still in the wallet). It is not a deployed rung.' })).toEqual({
      counted: false,
      text: 'Empty: it holds nothing (emptied; the NFT is still in the wallet). It is not a deployed rung.',
    });
    expect(trackedState({ ...tap, state: 'not_owned', note: 'NFT #3163450 is now owned by 0x4598…e826: it is no longer counted.' }).text).toMatch(/^Not counted: NFT #3163450 is now owned by/);
    expect(trackedState({ ...tap, state: 'closed', note: null })).toEqual({ counted: false, text: 'Closed: no reason reported.' });
    expect(trackedState({ ...tap, state: 'shadowed_by_config', note: 'x' }).text).toBe('In config.yaml: x');
    expect(trackedState({ ...tap, state: 'later_state', note: 'y' })).toEqual({ counted: false, text: 'later_state: y' });
  });

  it('links a mint only when it is a transaction hash', () => {
    expect(mintTxUrl(MINT)).toBe(`https://basescan.org/tx/${MINT}`);
    expect(mintTxUrl('0x12')).toBeNull();
    expect(mintTxUrl('javascript:alert(1)')).toBeNull();
    expect(mintTxUrl(null)).toBeNull();
    expect(shortHash(MINT)).toBe('0x6cdabada…e89e');
    expect(shortHash(null)).toBe('');
  });

  it('knows when the block has rows to show', () => {
    expect(detectionHasRows(DETECTION_NOTHING_FOUND)).toBe(false);
    expect(detectionHasRows(DETECTION_FOUND)).toBe(true);
    expect(detectionHasRows(DETECTION_DISMISSED)).toBe(true);
    expect(detectionHasRows(DETECTION_TRACKED_BY_ITSELF)).toBe(true);
  });
});

describe('one-tap writes on found positions', () => {
  const p = DETECTION_FOUND.found_not_counted.positions[0];

  it('track sends the NFT and the choice, nothing typed, and the worker runs the cycle', async () => {
    const c = client();
    const r = await tapFound(p, 'ladder:near', c);
    expect(r).toEqual({ ok: true, text: 'Tracked as a ladder: measured, never steered, and counted from the next cycle.' });
    // The worker runs a cycle after a track itself: the page asks for no second one.
    expect(c.calls).toEqual(['track 3163450 ladder:near']);
    const lp = client();
    await tapFound(p, 'lp', lp);
    expect(lp.calls).toEqual(['track 3163450 lp']);
  });

  it('a write whose cycle failed is still reported as stored', async () => {
    const c = client({ trackFoundPosition: async () => ({ note: 'Tracked as LP.', cycle: 'the write is stored; the cycle after it failed: RPC down' }) });
    expect(await tapFound(p, 'lp', c)).toEqual({ ok: true, text: 'Tracked as LP. (the write is stored; the cycle after it failed: RPC down)' });
  });

  it('not now and restore move it between the two lists and then refresh', async () => {
    const c = client();
    expect(await tapFound(p, 'dismiss', c)).toEqual({ ok: true, text: 'Set aside: it is not counted and stays listed, and can be restored.' });
    expect(await restoreFound(p, c)).toEqual({ ok: true, text: 'Back on offer: it is still not counted until you track it.' });
    expect(c.calls).toEqual(['dismiss 3163450', 'ingest', 'restore 3163450', 'ingest']);
  });

  it('untrack names the position, and setting a rung is a track', async () => {
    const t = DETECTION_TRACKED_BY_ITSELF.tracked[0];
    const c = client();
    expect((await tapTracked(t, 'untrack', c)).text).toBe('NFT #3163450 is still in your wallet and is no longer counted.');
    await tapTracked(t, 'ladder:deep', c);
    expect(c.calls).toEqual(['untrack uniswap-v4:base:3163450', 'track 3163450 ladder:deep']);
  });

  it('a refusal is shown in the worker words and nothing throws', async () => {
    const refuse = async () => {
      throw Object.assign(new Error('NFT #3163450 is owned by 0x4598…e826, not by the tracked wallet. Nothing changed.'), { status: 422, code: 'NOT_OWNED' });
    };
    const c = client({ trackFoundPosition: refuse, untrackPosition: refuse, dismissFoundPosition: refuse, restoreFoundPosition: refuse });
    for (const r of [await tapFound(p, 'lp', c), await tapFound(p, 'dismiss', c), await restoreFound(p, c), await tapTracked(DETECTION_TRACKED_BY_TAP.tracked[0], 'untrack', c)]) {
      expect(r).toEqual({ ok: false, text: 'NFT #3163450 is owned by 0x4598…e826, not by the tracked wallet. Nothing changed.' });
    }
    const silent = client({ trackFoundPosition: async () => { throw 'boom'; } });
    expect(await tapFound(p, 'lp', silent)).toEqual({ ok: false, text: 'That did not go through; nothing changed.' });
  });

  it('check now is one ingestion cycle', async () => {
    const c = client();
    expect(await checkNow(c)).toEqual({ ok: true, text: 'Checked: the chain was read again just now.' });
    expect(c.calls).toEqual(['ingest']);
    const down = client({ ingestNow: async () => { throw new Error('Ingestion failed: Failed to query live gas price from RPC'); } });
    expect(await checkNow(down)).toEqual({ ok: false, text: 'Ingestion failed: Failed to query live gas price from RPC' });
  });
});

describe('surfaces a tracked position changes', () => {
  it('two Uniswap v4 claim actions are two rows with two states', () => {
    const range = { platform: 'uniswap-v4', label: 'Collect Uniswap V4 LP fees (uniswap-v4-nock-usdc-wide #3163450)' };
    const rung = { platform: 'uniswap-v4', label: 'Collect Uniswap V4 LP fees (tracked-v4-3243807 #3243807)' };
    expect(claimActionKey(range)).not.toBe(claimActionKey(rung));
    expect(claimActionKey(range)).toBe('uniswap-v4|Collect Uniswap V4 LP fees (uniswap-v4-nock-usdc-wide #3163450)');
    // Without a label the platform is all there is.
    expect(claimActionKey({ platform: 'aerodrome' })).toBe('aerodrome');
    expect(claimActionKey({ platform: 'aerodrome', label: '  ' })).toBe('aerodrome');
  });

  it('a closed position found on-chain names UNTRACK, not a config entry', () => {
    const closed = { config_id: 'tracked-v4-3229273', nft_id: 3229273, source: 'tracked' };
    expect(closedPositionConfigHint(closed)).toBe('it was tracked from the chain, not from config.yaml: UNTRACK (under Found on-chain) removes it from this list');
    expect(closedPositionConfigHint({ config_id: 'aero-cl200-weth-nock', nft_id: 2914490 })).toBe('you can remove its entry (aero-cl200-weth-nock) from config.yaml');
    expect(closedPositionConfigHint({ ...closed, source: null })).toBe('you can remove its entry (tracked-v4-3229273) from config.yaml');
  });

  it('a ladder found on-chain says so beside its rung', () => {
    const base = { rung: 'near' } as any;
    expect(ladderRungText(base)).toBe('rung near');
    expect(ladderRungText({ ...base, source: 'tracked' })).toBe('rung near · found on-chain');
    expect(ladderRungText({ rung: null, source: 'tracked' } as any)).toBe('rung unset · found on-chain');
  });
});
