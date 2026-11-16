/**
 * Fill detection display helpers (v0.6.0, issue #118).
 *
 * The worker looks for the plan's buys on-chain and offers what it finds. Everything
 * shown here comes from the snapshot's `accumulation_plan.fill_detection` or from the
 * fills endpoint; nothing is computed into a number that the worker did not report. A
 * missing value is said to be unavailable, never drawn as zero.
 *
 * Confirming an offer records it through the same endpoint as a pasted hash, so the
 * worker verifies the receipt again. The helpers that act take the API client as an
 * argument so they can be tested without a network.
 */
import { api } from './api/client';
import type { AccumulationEvent, RecordAccumulationResponse } from './api/client';
import type { FillDetection, FillOffer } from './types/snapshot';

export type DetectionTone = 'ok' | 'warn' | 'muted';

export interface DetectionSummary {
  tone: DetectionTone;
  /** Short heading, e.g. "CHECKED" or "COULD NOT CHECK". */
  heading: string;
  text: string;
}

const clock = (iso: string | null | undefined): string | null => {
  if (!iso) return null;
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return null;
  return d.toISOString().replace('T', ' ').slice(0, 16) + ' UTC';
};

/**
 * One line on whether the worker looked for swaps this cycle. "Could not check" and
 * "checked, none found" are different states with different words.
 */
export function detectionSummary(detection: FillDetection | null | undefined): DetectionSummary {
  if (!detection) {
    return {
      tone: 'muted',
      heading: 'SWAP DETECTION NOT REPORTED',
      text: 'This snapshot carries no swap detection (worker older than v0.6.0). Record each buy by its transaction hash.',
    };
  }
  const last = clock(detection.last_checked_at);
  const back = clock(detection.checked_back_to);
  const offers = detection.offers?.length ?? 0;
  const found =
    offers === 0 ? 'no uncounted swap found' : `${offers} swap${offers === 1 ? '' : 's'} found, not counted yet`;
  switch (detection.status) {
    case 'ok':
      return {
        tone: 'ok',
        heading: 'CHECKED',
        text: `Wallet checked${last ? ` at ${last}` : ''}${back ? `, back to ${back}` : ''}: ${found}.`,
      };
    case 'partial':
      return {
        tone: 'warn',
        heading: 'CHECKED, INCOMPLETELY',
        text: `Wallet checked${last ? ` at ${last}` : ''}, but not completely: ${detection.detail ?? 'no reason given'}. So far: ${found}.`,
      };
    case 'unavailable':
      return {
        tone: 'warn',
        heading: 'COULD NOT CHECK',
        text: `Could not check the wallet for swaps this cycle: ${detection.detail ?? 'no reason given'}. Last successful check: ${last ?? 'none on record'}. This does not mean none were made; record a buy by its transaction hash below.`,
      };
    case 'not_run':
      return {
        tone: 'muted',
        heading: 'NOT CHECKED',
        text: `Swap detection did not run: ${detection.detail ?? 'no reason given'}.`,
      };
    default:
      return { tone: 'warn', heading: 'UNKNOWN STATE', text: `Swap detection reported "${String(detection.status)}".` };
  }
}

/** "0x78376f3c…6e31b9" for tight spaces; the full hash is always available alongside. */
export function shortHash(hash: string): string {
  return hash.length > 18 ? `${hash.slice(0, 10)}…${hash.slice(-6)}` : hash;
}

/** Block-explorer page for a transaction (read-only link). */
export function txUrl(hash: string): string {
  return `https://basescan.org/tx/${hash}`;
}

const amount = (s: string, digits: number): string => {
  const v = Number(s);
  if (s.trim() === '' || !Number.isFinite(v)) return s;
  return v.toLocaleString('en-US', { minimumFractionDigits: digits, maximumFractionDigits: digits });
};

type SwapFigures = Pick<FillOffer, 'usdc_spent' | 'nock_received' | 'effective_price_usd'>;

/** "$20.00 USDC → 1,780.36 NOCK at $0.011234 per NOCK" */
export function swapLine(s: SwapFigures): string {
  const price = Number(s.effective_price_usd);
  const priceText = Number.isFinite(price) && s.effective_price_usd.trim() !== '' ? price.toFixed(6) : s.effective_price_usd;
  return `$${amount(s.usdc_spent, 2)} USDC → ${amount(s.nock_received, 2)} NOCK at $${priceText} per NOCK`;
}

export interface CostLabel {
  /** False when there is no reference price, so no figure. */
  known: boolean;
  text: string;
  /** Where the reference came from, for a tooltip; empty when unknown. */
  source: string;
}

type CostFigures = Pick<FillOffer, 'reference_spot_usd' | 'reference_spot_source' | 'execution_cost_bps'>;

/**
 * Execution cost over the reference spot. Positive means the swap paid more than the
 * reference. Without a reference there is no figure: "unavailable", never 0.
 */
export function costOverSpot(f: CostFigures): CostLabel {
  const bps = f.execution_cost_bps;
  const spot = f.reference_spot_usd;
  if (typeof bps !== 'number' || !Number.isFinite(bps) || !spot) {
    return { known: false, text: 'cost over spot unavailable (no stored price near this block)', source: '' };
  }
  const pct = (Math.abs(bps) / 100).toFixed(2);
  const spotText = Number.isFinite(Number(spot)) ? Number(spot).toFixed(6) : spot;
  const text =
    bps === 0
      ? `at spot ($${spotText})`
      : `${bps > 0 ? '+' : '−'}${Math.abs(bps)} bps (${pct}%) ${bps > 0 ? 'over' : 'under'} spot $${spotText}`;
  return { known: true, text, source: f.reference_spot_source ?? '' };
}

export function timingLabel(timing: FillOffer['timing'] | string | undefined): string {
  if (timing === 'early') return 'early: before the open tranche came due';
  if (timing === 'on_schedule') return 'on schedule';
  return '';
}

/** The API calls the offer actions need; `api` satisfies it. */
export interface FillOfferClient {
  recordAccumulation: (txHash: string) => Promise<RecordAccumulationResponse>;
  dismissFillOffer: (txHash: string) => Promise<unknown>;
  restoreFillOffer: (txHash: string) => Promise<unknown>;
  ingestNow: () => Promise<unknown>;
}

export interface OfferActionResult {
  ok: boolean;
  text: string;
}

const failure = (e: unknown, fallback: string): OfferActionResult => ({
  ok: false,
  text: e instanceof Error && e.message ? e.message : fallback,
});

/**
 * CONFIRM: record the offered swap. This is the manual path with the hash filled in:
 * the worker fetches and verifies the receipt again, and what it records comes from
 * that verification, not from the offer. A failed refresh afterwards is not a failed
 * recording; the next cycle picks it up.
 */
export async function confirmOffer(
  offer: Pick<FillOffer, 'tx_hash'>,
  client: FillOfferClient = api,
): Promise<OfferActionResult> {
  try {
    const res = await client.recordAccumulation(offer.tx_hash);
    await client.ingestNow().catch(() => undefined);
    return {
      ok: true,
      text: res.duplicate
        ? 'Already recorded: this transaction is in the ledger.'
        : `Confirmed and verified on-chain: ${swapLine(res)}.`,
    };
  } catch (e) {
    return failure(e, 'Confirming failed; nothing was recorded.');
  }
}

/** NOT PART OF THE PLAN: the swap stays listed as not counted, and can be restored. */
export async function dismissOffer(
  offer: Pick<FillOffer, 'tx_hash'>,
  client: FillOfferClient = api,
): Promise<OfferActionResult> {
  try {
    await client.dismissFillOffer(offer.tx_hash);
    await client.ingestNow().catch(() => undefined);
    return { ok: true, text: 'Marked as not part of the plan. It is not counted, and can be restored below.' };
  } catch (e) {
    return failure(e, 'Could not mark the swap; nothing changed.');
  }
}

export async function restoreOffer(
  offer: Pick<FillOffer, 'tx_hash'>,
  client: FillOfferClient = api,
): Promise<OfferActionResult> {
  try {
    await client.restoreFillOffer(offer.tx_hash);
    await client.ingestNow().catch(() => undefined);
    return { ok: true, text: 'Restored: the swap is on offer again. It is still not counted until you confirm it.' };
  } catch (e) {
    return failure(e, 'Could not restore the swap; nothing changed.');
  }
}

/** The fills of one plan, oldest first, for the per-fill table. */
export function fillsForPlan(events: AccumulationEvent[] | null | undefined, planId: string): AccumulationEvent[] {
  return (events ?? [])
    .filter((e) => e.plan_id === planId)
    .slice()
    .sort((a, b) => a.confirmed_at.localeCompare(b.confirmed_at));
}

/** "2026-11-16 03:08 UTC" from a block time; the raw string when it does not parse. */
export function blockTime(iso: string): string {
  return clock(iso) ?? iso;
}
