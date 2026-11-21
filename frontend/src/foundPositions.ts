/**
 * Positions found on-chain (v0.9.0; #145 for Uniswap v4):
 * `derived_metrics.position_detection`. The worker looks for the wallet's new position
 * NFTs at the start of every cycle. These helpers are the block's wording and its
 * one-tap writes; the worker reads and decides everything.
 *
 * - A position that matches a suggested rung exactly is tracked by itself.
 * - Anything else it finds is FOUND, NOT COUNTED, with its value, until one tap says how
 *   to track it (as a ladder rung or as LP) or sets it aside.
 * - Nothing here asks the operator to type: no hash, no NFT id, no amount, no config.
 * - When the worker could not check, this says so; it never shows that as "none found".
 */
import { api } from './api/client';
import { refreshWithin } from './fillOffers';

export interface FoundToken {
  symbol: string;
  amount: string;
}

export interface FoundPosition {
  nft_id: number;
  position_id: string;
  protocol: string;
  pool_id: string | null;
  pool_label: string;
  tick_lower: number | null;
  tick_upper: number | null;
  bounds_usd: { low: string; high: string } | null;
  in_range: boolean | null;
  holds: { token0?: FoundToken; token1?: FoundToken; liquidity?: string } | null;
  value_usd: string | null;
  value_note: string | null;
  mint_tx: string | null;
  mint_block: number | null;
  minted_by: string | null;
  received_from: string | null;
  why_not_automatic: string[];
  /** `ladder:<rung>` | `lp` | `dismiss`, as the worker offers them. */
  choices: string[];
}

export interface FoundList {
  count: number;
  value_usd: string | null;
  value_note: string | null;
  positions: FoundPosition[];
}

export interface TrackedFromChain {
  position_id: string;
  nft_id: number;
  role: string | null;
  rung: string | null;
  rung_note: string | null;
  /** `detected` (matched a suggested rung) | `confirmed` (the operator's tap). */
  source: string;
  deposit_tx: string | null;
  mint_block: number | null;
  tracked_at: string | null;
  tick_lower: number;
  tick_upper: number;
  /** counted | empty | closed | not_owned | shadowed_by_config | quarantined | not_read */
  state: string;
  note: string | null;
  /** `ladder:<rung>` (set the rung) | `untrack`. */
  choices: string[];
}

export interface PositionDetection {
  /** ok | partial | unavailable | not_run */
  status: string;
  reason: string | null;
  last_checked_at: string | null;
  checked_at_block: number | null;
  scan: { coverage?: string; text?: string; logs_from_block?: number | null; logs_behind_blocks?: number | null; sources?: Record<string, string> } | null;
  onchain_count: { held: number | null; known: number | null } | null;
  tracked: TrackedFromChain[];
  found_not_counted: FoundList;
  dismissed: FoundList;
  ignored: Record<string, number>;
  statement: string;
}

export const FOUND_TITLE = 'Found on-chain';

/** The snapshot's block, or null from a worker that predates it. */
export function positionDetectionOf(data: any): PositionDetection | null {
  const d = data?.derived_metrics?.position_detection;
  return d && typeof d === 'object' && typeof d.status === 'string' ? (d as PositionDetection) : null;
}

/** Whether the block has anything to show beyond its one status line. */
export function detectionHasRows(d: PositionDetection): boolean {
  return (d.tracked?.length ?? 0) > 0 || (d.found_not_counted?.count ?? 0) > 0 || (d.dismissed?.count ?? 0) > 0;
}

export type DetectionTone = 'ok' | 'warn' | 'muted';

/** The status line: what was checked, or why it was not. Never "none found" for a check
 *  that did not run. */
export function detectionSummary(d: PositionDetection): { tone: DetectionTone; heading: string; text: string } {
  const coverage = d.scan?.text ?? '';
  switch (d.status) {
    case 'ok':
      return {
        tone: 'ok',
        heading: 'CHECKED',
        text: detectionHasRows(d) ? coverage : `Nothing new: no position is found that is not already counted. ${coverage}`.trim(),
      };
    case 'partial':
      return { tone: 'warn', heading: 'CHECKED IN PART', text: d.reason ?? 'Something is missing from this check.' };
    case 'not_run':
      return { tone: 'muted', heading: 'NOT LOOKED FOR', text: d.reason ?? 'New positions were not looked for.' };
    default:
      return { tone: 'warn', heading: 'COULD NOT CHECK', text: d.reason ?? 'New positions could not be checked this cycle.' };
  }
}

/** "37 not on the token whitelist, 5 empty": the NFTs the wallet holds that are never
 *  listed, by reason. Empty when there are none. */
export function ignoredLine(d: PositionDetection): string {
  const labels: [string, string][] = [
    ['in_config', 'in config.yaml'],
    ['not_whitelisted', 'not on the token whitelist'],
    ['empty', 'empty'],
    ['not_owned', 'no longer in the wallet'],
    ['hooks', 'in a hooked pool'],
    ['unsupported', 'with a token NockNock cannot read in a position yet'],
    ['gone', 'burned'],
  ];
  const parts = labels.filter(([k]) => (d.ignored?.[k] ?? 0) > 0).map(([k, words]) => `${d.ignored[k]} ${words}`);
  return parts.length ? `Not listed: ${parts.join(', ')}.` : '';
}

/** The heading of one list: "1 position, $240.61, not counted in any total". */
export function listHeading(list: FoundList, what: string): string {
  const noun = list.count === 1 ? 'position' : 'positions';
  const worth = list.value_usd != null ? `, $${list.value_usd}` : list.value_note ? ` (${list.value_note})` : '';
  return `${list.count} ${noun}${worth}, ${what}`;
}

export function foundTitle(p: FoundPosition): string {
  return `${p.pool_label} #${p.nft_id}`;
}

/** What a found position is and holds, as lines of fact. */
export function foundLines(p: FoundPosition): string[] {
  const lines: string[] = [];
  if (p.tick_lower != null && p.tick_upper != null) {
    const usd = p.bounds_usd ? ` ($${p.bounds_usd.low} to $${p.bounds_usd.high} per NOCK)` : '';
    lines.push(`Ticks ${p.tick_lower} to ${p.tick_upper}${usd}`);
  }
  const t0 = p.holds?.token0;
  const t1 = p.holds?.token1;
  if (t0 && t1) {
    const worth = p.value_usd != null ? `, worth $${p.value_usd}` : '';
    lines.push(`Holds ${t0.amount} ${t0.symbol} + ${t1.amount} ${t1.symbol}${worth}`);
  } else if (p.value_usd != null) {
    lines.push(`Worth $${p.value_usd}`);
  }
  if (p.value_usd == null && p.value_note) lines.push(sentence(p.value_note));
  return lines;
}

/** Why it was not tracked by itself, as one sentence. Empty when the worker gave none. */
export function whyNotAutomatic(p: FoundPosition): string {
  return p.why_not_automatic?.length ? `Not tracked by itself: ${p.why_not_automatic.join('; ')}.` : '';
}

function sentence(s: string): string {
  const t = s.trim();
  if (!t) return t;
  const cap = t[0].toUpperCase() + t.slice(1);
  return /[.!?]$/.test(cap) ? cap : `${cap}.`;
}

/** The label of a one-tap choice. */
export function choiceLabel(choice: string): string {
  if (choice === 'lp') return 'TRACK AS LP';
  if (choice === 'dismiss') return 'NOT NOW';
  if (choice === 'untrack') return 'UNTRACK';
  if (choice.startsWith('ladder:')) return `TRACK AS ${choice.slice('ladder:'.length).toUpperCase()} RUNG`;
  return choice.toUpperCase();
}

/** What a choice does, for the button's title. Statements, never instructions. */
export function choiceTitle(choice: string): string {
  if (choice === 'lp') return 'Counts it from the next cycle, classified by its pair. It counts as LP held and is never a rebuild target.';
  if (choice === 'dismiss') return 'Leaves it uncounted. It stays listed below and can be restored.';
  if (choice === 'untrack') return 'Stops counting it. The NFT stays in your wallet and is then listed as set aside.';
  if (choice.startsWith('ladder:')) return 'Counts it from the next cycle as a ladder rung: measured, never steered, with its mint as the baseline.';
  return '';
}

/** One tracked row as a line: what it is tracked as and how it came to be. */
export function trackedLine(t: TrackedFromChain): string {
  const what = t.role === 'ladder' ? (t.rung ? `ladder, ${t.rung} rung` : 'ladder, rung not set') : 'LP, never a rebuild target';
  const how = t.source === 'detected' ? 'tracked by itself: it matched a suggested rung' : 'tracked by your tap';
  return `NFT #${t.nft_id}: ${what}; ${how}`;
}

/** The state of a tracked row in words, and whether it is counted. */
export function trackedState(t: TrackedFromChain): { counted: boolean; text: string } {
  if (t.state === 'counted') return { counted: true, text: 'Counted in the totals.' };
  const label: Record<string, string> = {
    empty: 'Empty',
    closed: 'Closed',
    not_owned: 'Not counted',
    shadowed_by_config: 'In config.yaml',
    quarantined: 'Not counted',
    not_read: 'Not read',
  };
  return { counted: false, text: `${label[t.state] ?? t.state}: ${t.note ?? 'no reason reported.'}` };
}

/** "0x6cdabada…e89e" */
export function shortHash(h: string | null | undefined): string {
  if (!h) return '';
  return h.length > 14 ? `${h.slice(0, 10)}…${h.slice(-4)}` : h;
}

/** A transaction on BaseScan; null unless it is a 32-byte hash. */
export function mintTxUrl(tx: string | null | undefined): string | null {
  return tx && /^0x[0-9a-fA-F]{64}$/.test(tx) ? `https://basescan.org/tx/${tx}` : null;
}

// ---- one-tap writes ----

export interface TapResult {
  ok: boolean;
  text: string;
}

export interface FoundClient {
  trackFoundPosition: (nftId: number, as: string) => Promise<{ note?: string; cycle?: string }>;
  untrackPosition: (positionId: string) => Promise<{ note?: string; cycle?: string }>;
  dismissFoundPosition: (nftId: number) => Promise<{ note?: string }>;
  restoreFoundPosition: (nftId: number) => Promise<{ note?: string }>;
  ingestNow: () => Promise<unknown>;
}

const failure = (e: unknown, fallback: string): TapResult => ({
  ok: false,
  text: e instanceof Error && e.message ? e.message : fallback,
});

/** A write whose cycle failed is still a stored write: say both. */
function withCycle(note: string | undefined, cycle: string | undefined, fallback: string): string {
  const base = note ?? fallback;
  return cycle && cycle !== 'ok' ? `${base} (${cycle})` : base;
}

/**
 * Apply one choice to a found position: `ladder:<rung>` or `lp` tracks it (the worker
 * reads the NFT on-chain again first and runs a cycle after), `dismiss` sets it aside.
 * Nothing is typed and nothing is signed. Never throws.
 */
export async function tapFound(p: Pick<FoundPosition, 'nft_id'>, choice: string, client: FoundClient = api): Promise<TapResult> {
  try {
    if (choice === 'dismiss') {
      const r = await client.dismissFoundPosition(p.nft_id);
      await refreshWithin(() => client.ingestNow());
      return { ok: true, text: r.note ?? 'Set aside: it is not counted and can be restored.' };
    }
    const r = await client.trackFoundPosition(p.nft_id, choice);
    return { ok: true, text: withCycle(r.note, r.cycle, 'Tracked: it is counted from the next cycle.') };
  } catch (e) {
    return failure(e, 'That did not go through; nothing changed.');
  }
}

/** RESTORE: put a set-aside position back on offer. Never throws. */
export async function restoreFound(p: Pick<FoundPosition, 'nft_id'>, client: FoundClient = api): Promise<TapResult> {
  try {
    const r = await client.restoreFoundPosition(p.nft_id);
    await refreshWithin(() => client.ingestNow());
    return { ok: true, text: r.note ?? 'Back on offer: it is still not counted until you track it.' };
  } catch (e) {
    return failure(e, 'Could not restore it; nothing changed.');
  }
}

/** A choice on a row tracked from the chain: `untrack`, or `ladder:<rung>` to set its
 *  rung. Never throws. */
export async function tapTracked(t: Pick<TrackedFromChain, 'nft_id' | 'position_id'>, choice: string, client: FoundClient = api): Promise<TapResult> {
  try {
    if (choice === 'untrack') {
      const r = await client.untrackPosition(t.position_id);
      return { ok: true, text: withCycle(r.note, r.cycle, 'Untracked: it is no longer counted.') };
    }
    const r = await client.trackFoundPosition(t.nft_id, choice);
    return { ok: true, text: withCycle(r.note, r.cycle, 'Updated.') };
  } catch (e) {
    return failure(e, 'That did not go through; nothing changed.');
  }
}

/** CHECK NOW: one ingestion cycle, which looks for new positions before it reads. */
export async function checkNow(client: Pick<FoundClient, 'ingestNow'> = api): Promise<TapResult> {
  try {
    await client.ingestNow();
    return { ok: true, text: 'Checked: the chain was read again just now.' };
  } catch (e) {
    return failure(e, 'The check did not finish; the next scheduled cycle checks again.');
  }
}
