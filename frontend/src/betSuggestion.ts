/**
 * The WETH/NOCK bet card (issue #181): `derived_metrics.bet_suggestion` as the worker states
 * it. A bet is a small two-sided WETH/NOCK range in one Aero CL pool around the price,
 * staked for AERO. A new one is suggested only while no bet is in range; a bet out of range
 * counts as cashed out. Every number is the worker's; the card computes nothing and asks
 * for nothing. The only buttons are one-tap answers for a bet found in the gauge that is
 * not counted yet (track it as a bet, or set it aside).
 *
 * Wording lives here and is unit-tested (betSuggestion.test.ts).
 */
import { api } from './api/client';
import { refreshWithin } from './fillOffers';
import type { SyncStatus } from './hooks/useFreshness';

export const BET_TITLE = 'WETH/NOCK BET';

export interface BetGate {
  gate: 'settings' | 'gas_floor' | 'nock_floor' | 'no_bet_in_range' | string;
  status: 'pass' | 'fail' | string;
  text: string;
}

export interface BetLine {
  position_id: string;
  source?: string | null;
  status: 'in_range' | 'out_of_range' | string;
  tick_lower: number;
  tick_upper: number;
  low_usd: string | null;
  high_usd: string | null;
  weth: string;
  nock: string;
  value_usd: string;
  unclaimed_aero?: string | null;
  unclaimed_aero_usd?: string | null;
  aero_note?: string | null;
  fees_note?: string | null;
  staked?: boolean;
  line: string;
  [key: string]: unknown;
}

export interface FoundBet {
  nft_id: number;
  protocol: string;
  dismissed: boolean;
  tick_lower: number | null;
  tick_upper: number | null;
  liquidity: string | null;
  why_not_automatic: string[];
  choices: string[];
  line: string;
}

export interface BetLink {
  url: string;
  host: string;
  native: boolean;
  prefilled: string[];
  not_prefilled: string[];
  amounts_to_enter: string;
  format_note: string;
  [key: string]: unknown;
}

export interface BetSuggestionBody {
  tick_lower: number;
  tick_upper: number;
  width: string;
  low_usd: string;
  high_usd: string;
  pool_usd_per_nock: string;
  low_pct: string;
  high_pct: string;
  weth: string;
  nock: string;
  weth_usd: string;
  nock_usd: string;
  value_usd: string;
  bet_usd: string;
  deposit_token: 'WETH' | 'ETH' | string;
  link: BetLink;
  range_text: string;
  outcomes: Array<{ kind: string; text: string }>;
  [key: string]: unknown;
}

export interface BetSuggestion {
  id: string;
  tier: 'top' | 'quiet' | string;
  status: string;
  shown: boolean;
  headline: string | null;
  reason: string | null;
  missing: string[];
  invalid: string[];
  pool: { address: string; label: string; tick: number; usd_per_nock: string | null; fee_pct: string; tick_spacing: number; [key: string]: unknown } | null;
  bets: BetLine[];
  found_not_counted: FoundBet[];
  suggestion: BetSuggestionBody | null;
  gates: BetGate[];
  gas: { text: string; short_by_eth: string | null; [key: string]: unknown } | null;
  nock: { text: string; [key: string]: unknown } | null;
  floor_crossing: { status: string; text: string; [key: string]: unknown } | null;
  after_deposit: string;
  statement: string;
  [key: string]: unknown;
}

/** The snapshot's bet card, or null (operator_profile.nock_bets not set, or a snapshot
 *  stored before 2.12.0). */
export function betSuggestionOf(data: any): BetSuggestion | null {
  const b = data?.derived_metrics?.bet_suggestion;
  return b && typeof b === 'object' && typeof b.status === 'string' ? (b as BetSuggestion) : null;
}

/** Whether the card belongs at the top of the page: a bet suggested now, or a bet found in
 *  the gauge waiting for one tap. Everything else is a quiet line. */
export function betAtTop(b: BetSuggestion | null | undefined): boolean {
  if (!b) return false;
  return (b.status === 'ready' && b.shown) || b.status === 'found_untracked';
}

/** The card's view: a ready card on a stale snapshot is not shown as a call to action. */
export function betCardView(b: BetSuggestion, syncStatus: SyncStatus | undefined): { mode: 'ready' | 'stale' | 'line'; headline: string; line: string | null } {
  if (b.status === 'ready' && b.shown) {
    if (syncStatus === 'stale') {
      return { mode: 'stale', headline: 'Bet: not shown while the snapshot is stale.', line: 'It shows again once the worker reads the chain.' };
    }
    return { mode: 'ready', headline: b.headline ?? 'Bet: one fits your settings now.', line: null };
  }
  return { mode: 'line', headline: b.headline ?? 'Bets: not measured this cycle.', line: b.reason };
}

/** The one quiet line of a card that is not at the top. */
export function betQuietLine(b: BetSuggestion): string {
  const open = b.bets.filter((l) => l.status === 'in_range').length;
  const out = b.bets.length - open;
  const parts = [b.headline ?? 'Bets: not measured this cycle.'];
  if (b.bets.length > 0) parts.push(`${open} in range, ${out} cashed out`);
  return parts.join(' · ');
}

/** The deposit link is used only on https to the Aero app. */
export function betLinkUrl(url: string | null | undefined): string | null {
  if (!url) return null;
  try {
    const u = new URL(url);
    return u.protocol === 'https:' && u.hostname === 'app.aero.xyz' ? url : null;
  } catch {
    return null;
  }
}

/** The link's label: what it fills in, and what it cannot. */
export function betLinkLabel(b: BetSuggestionBody): string {
  return b.link.native ? 'OPEN THE DEPOSIT ON AERO.XYZ ↗ (pool and range filled in, native ETH)' : 'OPEN THE DEPOSIT ON AERO.XYZ ↗ (pool and range filled in)';
}

/** The amounts to type, as the worker prints them, without the trailing period. */
export function betAmountsLine(b: BetSuggestionBody): string {
  return b.link.amounts_to_enter.replace(/\.$/, '');
}

/** The gate rows, each with its mark. */
export function betGateLines(b: BetSuggestion): Array<{ ok: boolean; text: string }> {
  return (b.gates ?? []).map((g) => ({ ok: g.status === 'pass', text: g.text }));
}

/** A found bet's one-tap choice labels. */
export function betChoiceLabel(choice: string): string {
  if (choice === 'bet') return 'TRACK AS A BET';
  if (choice === 'dismiss') return 'NOT NOW';
  return choice.toUpperCase();
}

export interface BetTapResult {
  ok: boolean;
  text: string;
}

export interface BetClient {
  trackFoundPosition: (nftId: number, as: string) => Promise<{ note?: string; cycle?: string }>;
  dismissFoundBet: (nftId: number) => Promise<{ note?: string }>;
  restoreFoundBet: (nftId: number) => Promise<{ note?: string }>;
  ingestNow: () => Promise<unknown>;
}

/** One tap on a found bet: `bet` tracks it (the worker reads its stake again first and
 *  runs a cycle after), `dismiss` sets it aside, `restore` puts it back. Never throws. */
export async function tapBet(nftId: number, choice: string, client: BetClient = api): Promise<BetTapResult> {
  try {
    if (choice === 'dismiss' || choice === 'restore') {
      const r = choice === 'dismiss' ? await client.dismissFoundBet(nftId) : await client.restoreFoundBet(nftId);
      await refreshWithin(() => client.ingestNow());
      return { ok: true, text: r.note ?? (choice === 'dismiss' ? 'Set aside: it is not counted and can be restored.' : 'Back on offer.') };
    }
    const r = await client.trackFoundPosition(nftId, choice);
    const base = r.note ?? 'Tracked as a bet: counted from the next cycle.';
    return { ok: true, text: r.cycle && r.cycle !== 'ok' ? `${base} (${r.cycle})` : base };
  } catch (e) {
    return { ok: false, text: e instanceof Error && e.message ? e.message : 'That did not go through; nothing changed.' };
  }
}
