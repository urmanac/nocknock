/**
 * Static renders of the "Found on-chain" block (v0.9.0), on the dashboard and in the
 * phone cockpit. The frontend tests have no DOM, so these check the markup. The
 * fixtures are the worker's own blocks, built from recorded chain answers.
 */
import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { renderToStaticMarkup } from 'react-dom/server';
import { FoundPositionsCard, FoundPositionsLiteCard } from './FoundPositions';
import { LadderSuggestionCard } from './LadderSuggestionCard';
import { LADDER_READY_LIVE } from '../ladderSuggestion.fixture';
import {
  DETECTION_DISMISSED,
  DETECTION_FOUND,
  DETECTION_NOTHING_FOUND,
  DETECTION_PARTIAL,
  DETECTION_TRACKED_BY_ITSELF,
  DETECTION_TRACKED_BY_TAP,
  DETECTION_UNAVAILABLE,
} from '../foundPositions.fixture';
import type { PositionDetection } from '../foundPositions';

/** Nothing on this block is typed: no field of any kind. */
const NO_TYPED_INPUT = /<input|<form|<select|<textarea|contenteditable/;
const IMPERATIVE = /you should|you must|we recommend|we suggest|act now/i;
const ALL: [string, PositionDetection][] = [
  ['nothing found', DETECTION_NOTHING_FOUND],
  ['found', DETECTION_FOUND],
  ['dismissed', DETECTION_DISMISSED],
  ['tracked by itself', DETECTION_TRACKED_BY_ITSELF],
  ['tracked by tap', DETECTION_TRACKED_BY_TAP],
  ['partial', DETECTION_PARTIAL],
  ['unavailable', DETECTION_UNAVAILABLE],
];
const buttons = (html: string) => [...html.matchAll(/<button[^>]*>([^<]*)<\/button>/g)].map((m) => m[1]);

describe('Found on-chain block (v0.9.0)', () => {
  it('renders nothing for a worker that predates it', () => {
    expect(renderToStaticMarkup(<FoundPositionsCard detection={null} />)).toBe('');
    expect(renderToStaticMarkup(<FoundPositionsLiteCard detection={undefined} />)).toBe('');
  });

  it('has no typed input in any state, on either surface, and never tells the operator what to do', () => {
    for (const [name, d] of ALL) {
      for (const html of [renderToStaticMarkup(<FoundPositionsCard detection={d} />), renderToStaticMarkup(<FoundPositionsLiteCard detection={d} />)]) {
        expect(html, name).not.toMatch(NO_TYPED_INPUT);
        expect(html, name).not.toMatch(IMPERATIVE);
        expect(html, name).not.toMatch(/transaction hash|paste|config\.yaml entry/i);
        // CHECK NOW is always there: one tap asks the worker to look again.
        expect(buttons(html), name).toContain('CHECK NOW');
      }
    }
  });

  it('nothing found: one status line saying what was read, and what is never listed', () => {
    const html = renderToStaticMarkup(<FoundPositionsCard detection={DETECTION_NOTHING_FOUND} />);
    expect(html).toContain('FOUND ON-CHAIN · NOTHING NEW');
    expect(html).toContain('<strong style="letter-spacing:1px">CHECKED</strong> · Nothing new: no position is found that is not already counted.');
    expect(html).toContain('Not listed: 1 in config.yaml, 37 not on the token whitelist, 5 empty.');
    expect(html).not.toContain('data-testid="found-offer"');
    expect(buttons(html)).toEqual(['CHECK NOW']);
    expect(html).toContain('It never signs or sends.');
  });

  it('found: stated as not counted, with what it holds, why, and one button per choice', () => {
    const html = renderToStaticMarkup(<FoundPositionsCard detection={DETECTION_FOUND} />);
    expect(html).toContain('data-testid="found-heading"');
    expect(html).toContain('1 position, $240.61, found on-chain and NOT counted in any total');
    expect(html).toContain('v4 USDC/NOCK 3.00% #3163450');
    expect(html).toContain('Ticks 262200 to 282300 ($0.00550148 to $0.04105518 per NOCK)');
    expect(html).toContain('Holds 120.02 USDC + 8114.3626 NOCK, worth $240.61');
    expect(html).toContain('Not tracked by itself: no ladder rung is suggested now to match it against.');
    expect(buttons(html)).toEqual(['TRACK AS CAP RUNG', 'TRACK AS NEAR RUNG', 'TRACK AS DEEP RUNG', 'TRACK AS LP', 'NOT NOW', 'CHECK NOW']);
    // The mint the worker found is a link to read, never something to copy in.
    expect(html).toContain('href="https://basescan.org/tx/0x6cdabada1ff062b6d3b4ba87bfe04158dd8b14a91e93cabe11dc3e35b587e89e"');
    expect(html).toContain('mint 0x6cdabada…e89e');
    expect(html).not.toContain('NOTHING NEW');
    expect(html).not.toContain('Nothing new');
  });

  it('dismissed: listed under its own heading with its value, restorable', () => {
    const html = renderToStaticMarkup(<FoundPositionsCard detection={DETECTION_DISMISSED} />);
    expect(html).toContain('SET ASIDE BY YOU · 1 POSITION, $240.61, NOT COUNTED');
    expect(html).toContain('data-testid="found-dismissed"');
    expect(html).not.toContain('data-testid="found-offer"');
    expect(buttons(html)).toEqual(['RESTORE', 'CHECK NOW']);
  });

  it('tracked: what it is tracked as, how, its mint, and whether it is counted', () => {
    const auto = renderToStaticMarkup(<FoundPositionsCard detection={DETECTION_TRACKED_BY_ITSELF} />);
    expect(auto).toContain('TRACKED FROM THE CHAIN');
    expect(auto).toContain('NFT #3163450: ladder, rung not set; tracked by itself: it matched a suggested rung');
    expect(auto).toContain('Counted in the totals.');
    expect(auto).toContain('no rung is set: the suggestion it matched named none, and NockNock assumes neither near nor deep. One tap sets it.');
    expect(auto).toContain('Ticks 262200 to 282300');
    expect(auto).toContain('mint 0x6cdabada…e89e');
    // The rung is set with one tap, and the row can be untracked.
    expect(buttons(auto)).toEqual(['SET AS CAP RUNG', 'SET AS NEAR RUNG', 'SET AS DEEP RUNG', 'UNTRACK', 'CHECK NOW']);
    const tap = renderToStaticMarkup(<FoundPositionsCard detection={DETECTION_TRACKED_BY_TAP} />);
    expect(tap).toContain('NFT #3163450: ladder, deep rung; tracked by your tap');
    expect(buttons(tap)).toEqual(['UNTRACK', 'CHECK NOW']);
  });

  it('a tracked row that is not counted says so in the worker words', () => {
    const base = DETECTION_TRACKED_BY_TAP.tracked[0];
    const rows = [
      { ...base, state: 'empty', note: 'it holds nothing (emptied; the NFT is still in the wallet). It is not a deployed rung.' },
      { ...base, position_id: 'uniswap-v4:base:2', nft_id: 2, state: 'closed', note: 'NFT #2 no longer exists (withdrawn and burned): it is in no total. UNTRACK removes it from this list.' },
      { ...base, position_id: 'uniswap-v4:base:3', nft_id: 3, state: 'not_owned', note: 'NFT #3 is now owned by 0x4598…e826: it is no longer counted. UNTRACK removes it from this list.' },
    ];
    const html = renderToStaticMarkup(<FoundPositionsCard detection={{ ...DETECTION_TRACKED_BY_TAP, tracked: rows }} />);
    expect(html).toContain('Empty: it holds nothing (emptied; the NFT is still in the wallet). It is not a deployed rung.');
    expect(html).toContain('Closed: NFT #2 no longer exists (withdrawn and burned): it is in no total.');
    expect(html).toContain('Not counted: NFT #3 is now owned by 0x4598…e826: it is no longer counted.');
    expect(html).not.toContain('Counted in the totals.');
    expect(buttons(html).filter((b) => b === 'UNTRACK')).toHaveLength(3);
  });

  it('partial and unavailable say what is missing, and keep what was found listed', () => {
    const partial = renderToStaticMarkup(<FoundPositionsCard detection={DETECTION_PARTIAL} />);
    expect(partial).toContain('CHECKED IN PART');
    expect(partial).toContain('transfers before block 52931286 are not read yet');
    expect(partial).toContain('the wallet holds 43 Uniswap v4 position NFTs on-chain and 3 are known: 40 not found yet');
    expect(partial).toContain('data-testid="found-offer"');

    const down = renderToStaticMarkup(<FoundPositionsCard detection={DETECTION_UNAVAILABLE} />);
    expect(down).toContain('COULD NOT CHECK');
    expect(down).toContain('New positions could not be checked this cycle (no Alchemy RPC key is configured)');
    // The position an earlier check found stays listed, with no value it did not read.
    expect(down).toContain('1 position (1 of 1 could not be valued this cycle, so no total is stated), found on-chain and NOT counted in any total');
    expect(down).toContain('It was not read this cycle, so what it holds and its value are not stated.');
    expect(down).not.toMatch(/\$0\.00(?!\d)|Nothing new|NOTHING NEW/);
    // The block draws attention when something is uncounted or unchecked.
    expect(down).toContain('border:1px solid #92400e');
    expect(renderToStaticMarkup(<FoundPositionsCard detection={DETECTION_NOTHING_FOUND} />)).toContain('border:1px solid #30363d');
  });

  it('the phone card shows the same rows and buttons, without the long statement', () => {
    const html = renderToStaticMarkup(<FoundPositionsLiteCard detection={DETECTION_FOUND} />);
    expect(html).toContain('data-testid="found-positions-lite"');
    expect(html).toContain('FOUND ON-CHAIN');
    expect(html).toContain('1 position, $240.61, found on-chain and NOT counted in any total');
    expect(buttons(html)).toEqual(['TRACK AS CAP RUNG', 'TRACK AS NEAR RUNG', 'TRACK AS DEEP RUNG', 'TRACK AS LP', 'NOT NOW', 'CHECK NOW']);
    expect(html).not.toContain('It never signs or sends.');
    // Thumb-sized buttons.
    expect(html).toContain('padding:10px 12px');
  });

  it('is mounted right under the ladder card on the dashboard and in the phone cockpit', () => {
    const app = readFileSync(new URL('../App.tsx', import.meta.url), 'utf8');
    const card = app.indexOf('<LadderSuggestionCard suggestion=');
    // At the top only when it needs a tap or the check did not complete (#169).
    const found = app.indexOf('{foundAtTop && <FoundPositionsCard detection={detection} onChanged={refetch} />}');
    expect(card).toBeGreaterThan(0);
    expect(found).toBeGreaterThan(card);
    expect(found).toBeLessThan(app.indexOf('{/* Primary Dual-Goal Metrics Bar */}'));
    const phone = readFileSync(new URL('./LitePhoneCards.tsx', import.meta.url), 'utf8');
    const lite = phone.indexOf('<LadderSuggestionLiteCard');
    const foundLite = phone.indexOf('<FoundPositionsLiteCard detection={positionDetectionOf(snapshot.data)} onChanged={refetch}');
    expect(foundLite).toBeGreaterThan(lite);
    expect(foundLite).toBeLessThan(phone.indexOf('<AccumulationPlanPanel'));
  });
});

describe('the ladder card after a deposit (v0.9.0)', () => {
  it('says the position is found by itself, with no fallback and nothing to copy', () => {
    const html = renderToStaticMarkup(<LadderSuggestionCard suggestion={LADDER_READY_LIVE} syncStatus="fresh" />);
    expect(html).toContain('data-testid="ladder-after-deposit"');
    expect(html).toContain('After you deposit there is nothing to copy or configure.');
    expect(html).toContain('at the start of every cycle (CHECK NOW runs one at once), so a deposit is normally found in the first cycle after it lands.');
    expect(html).toContain('is not counted until one tap says how to track it.');
    expect(html).not.toMatch(/fallback|not built yet|config\.yaml|deposit_tx|nft_id/);
    expect(html).not.toContain('data-testid="ladder-after-deposit-note"');
  });

  it('says so beside it when this cycle could not check for new positions', () => {
    const note = "This cycle's check did not run: New positions could not be checked this cycle (the request timed out). A deposit made now is found once the check runs again.";
    const s = { ...LADDER_READY_LIVE, after_deposit: { ...LADDER_READY_LIVE.after_deposit!, detection_status: 'unavailable', detection_note: note } };
    const html = renderToStaticMarkup(<LadderSuggestionCard suggestion={s} syncStatus="fresh" />);
    expect(html).toContain('data-testid="ladder-after-deposit-note"');
    expect(html).toContain('This cycle&#x27;s check did not run: New positions could not be checked this cycle (the request timed out).');
  });

  it('a snapshot stored before detection still prints its own after-deposit text', () => {
    const old = { ...LADDER_READY_LIVE, after_deposit: { kind: 'manual_config', text: 'A minted rung is tracked only from a config.yaml entry (role: ladder).' } };
    expect(renderToStaticMarkup(<LadderSuggestionCard suggestion={old} syncStatus="fresh" />)).toContain('A minted rung is tracked only from a config.yaml entry');
  });
});
