/**
 * Route quote display helpers (v0.7.0, issue #115).
 *
 * When a tranche is due the worker asks each venue's own quoter contract what that
 * exact USDC amount returns in NOCK, at one block, and reports both. Everything shown
 * here comes from the snapshot's `accumulation_plan.route_quotes` or from the fills
 * endpoint: no figure is computed here that the worker did not report, and a venue that
 * could not be quoted is "unavailable" with its reason, never a number.
 *
 * It compares execution routes for a buy the plan already sized. It is not a
 * recommendation to buy, and the worker never signs, sizes or sends anything.
 */
import type { AccumulationEvent } from './api/client';
import type { RouteAttempt, RouteHop, RouteQuotes, VenueQuote } from './types/snapshot';

/** Said under every set of quotes. Same wording as the worker's MCP text. */
export const ROUTE_QUOTE_CAVEAT =
  "Quotes are contract reads at the stated block: estimates as of that time, not a promise. The wallet app's own router may pick a different route. This compares execution routes for a buy your plan already sized; it is not a recommendation to buy.";

/** The one caveat every cost-over-quote figure needs, shown on the page. */
export const COST_OVER_QUOTE_CAVEAT =
  'Cost over quote compares the swap\'s price with the best quote NockNock read for that tranche in the 60 minutes before the swap\'s block; with no such quote it reads "no quote on record". The market can move between the quote\'s block and the swap, so the figure is not execution cost alone; each one names its quote\'s block.';

/** The only pages a swap link may open. Aero is app.aero.xyz, never aerodrome.finance. */
const SWAP_PAGES: Record<string, string> = {
  Aero: 'https://app.aero.xyz/swap',
  Uniswap: 'https://app.uniswap.org/swap',
};
const SWAP_HOSTS = new Set(['app.aero.xyz', 'app.uniswap.org']);

/**
 * The link for a venue's swap page. The worker's URL is used only when it is https on
 * one of the two known hosts; anything else falls back to the venue's known page, and
 * an unknown venue gets no link at all.
 */
export function swapPageUrl(venue: string, url: string | null | undefined): string | null {
  if (url) {
    try {
      const u = new URL(url);
      if (u.protocol === 'https:' && SWAP_HOSTS.has(u.hostname)) return u.toString();
    } catch {
      // fall through to the known page
    }
  }
  return SWAP_PAGES[venue] ?? null;
}

/** "OPEN AERO SWAP PAGE": a swap page, never called a deposit. */
export function swapLinkLabel(venue: string): string {
  return `OPEN ${venue.toUpperCase()} SWAP PAGE`;
}

const clock = (iso: string | null | undefined): string | null => {
  if (!iso) return null;
  const d = new Date(iso);
  if (Number.isNaN(d.getTime())) return null;
  return d.toISOString().replace('T', ' ').slice(0, 19) + ' UTC';
};

const grouped = (n: number): string => n.toLocaleString('en-US');

/** "block 58,081,910 · 2026-11-16 05:25:57 UTC", or that nothing was read. */
export function quoteBlockLine(q: Pick<RouteQuotes, 'block_number' | 'quoted_at'>): string {
  if (typeof q.block_number !== 'number') return 'no block was read';
  const when = clock(q.quoted_at);
  return `block ${grouped(q.block_number)}${when ? ` · ${when}` : ''}`;
}

/**
 * How old the quotes are, in words: "read 4 min ago". Null when the time is unknown.
 * A quote from the future (clock skew) is "just now", never a negative age.
 */
export function quoteAge(quotedAt: string | null | undefined, nowMs: number): string | null {
  if (!quotedAt) return null;
  const t = Date.parse(quotedAt);
  if (Number.isNaN(t)) return null;
  const seconds = Math.max(0, Math.round((nowMs - t) / 1000));
  if (seconds < 60) return seconds < 5 ? 'read just now' : `read ${seconds} s ago`;
  const minutes = Math.round(seconds / 60);
  if (minutes < 120) return `read ${minutes} min ago`;
  return `read ${Math.round(minutes / 60)} h ago`;
}

/**
 * NOCK amount for reading: "1,773.0768". Cut to four decimals from the exact decimal
 * string, never rounded and never through a float, so it reads the same as the worker's
 * comparison sentence and the MCP text. The exact string stays in the title.
 */
export function nockAmount(s: string | null | undefined): string {
  if (!s || s.trim() === '') return 'n/a';
  const m = /^(\d+)(?:\.(\d*))?$/.exec(s.trim());
  if (!m) return s;
  const whole = m[1].replace(/^0+(?=\d)/, '').replace(/\B(?=(\d{3})+(?!\d))/g, ',');
  return `${whole}.${(m[2] ?? '').slice(0, 4).padEnd(4, '0')}`;
}

/**
 * Cost over the snapshot's reference spot, in words. Negative is real: the reference is
 * one pool's mid price, and a route through deeper pools can beat it.
 */
export function costOverSpotText(pct: string | null | undefined, spot: string | null | undefined): string {
  const v = pct === null || pct === undefined || pct.trim() === '' ? NaN : Number(pct);
  if (!Number.isFinite(v)) return 'cost over spot unavailable (no reference spot)';
  const spotText = spot && Number.isFinite(Number(spot)) ? ` $${Number(spot).toFixed(6)}` : '';
  if (v === 0) return `at the reference spot${spotText}`;
  return `${Math.abs(v).toFixed(2)}% ${v > 0 ? 'over' : 'under'} the reference spot${spotText}`;
}

/** "USDC → WETH · Uniswap v3 · fee 0.05% · 0xd0b53d92…f224" */
export function hopLine(h: RouteHop): string {
  const pool = h.pool.length > 18 ? `${h.pool.slice(0, 10)}…${h.pool.slice(-4)}` : h.pool;
  const fee = h.fee_pct ? `fee ${h.fee_pct}%` : 'fee not read';
  return `${h.token_in} → ${h.token_out} · ${h.pool_kind} · ${fee} · ${pool}`;
}

export interface VenueCard {
  venue: string;
  quoted: boolean;
  /** True only for the venue the worker named as returning more NOCK. */
  returnsMore: boolean;
  /** "1,773.0768 NOCK", or "UNAVAILABLE". */
  headline: string;
  /** Exact NOCK out as the worker reported it; empty when unavailable. */
  exactNock: string;
  /** "$0.011280 per NOCK · 0.06% under the reference spot $0.011287"; the reason when unavailable. */
  detail: string;
  /** The best route's tokens, with its route id when another candidate reads the same. */
  route: string;
  hops: string[];
  /** Other candidate routes of this venue and what each returned. */
  otherRoutes: string[];
  swapUrl: string | null;
  swapLabel: string;
}

/**
 * A candidate route's name. A route is labelled by its tokens, so two routes through
 * different pools can read the same ("USDC → WETH → NOCK" by either Aero USDC/WETH
 * pool). A label shared within the venue gets its route id, so no two lines with
 * different amounts carry one name.
 */
export function routeName(r: Pick<RouteAttempt, 'route' | 'route_id'>, tried: Pick<RouteAttempt, 'route'>[]): string {
  return tried.filter((t) => t.route === r.route).length > 1 ? `${r.route} (${r.route_id})` : r.route;
}

/** Everything one venue's card shows. Nothing is drawn for an unavailable venue but its reason. */
export function venueCard(v: VenueQuote, q: Pick<RouteQuotes, 'more_nock_venue' | 'reference_spot_usd'>): VenueCard {
  const swapUrl = swapPageUrl(v.venue, v.swap_url);
  const swapLabel = swapLinkLabel(v.venue);
  const tried = v.routes_tried ?? [];
  if (v.status !== 'quoted' || !v.nock_out) {
    return {
      venue: v.venue,
      quoted: false,
      returnsMore: false,
      headline: 'UNAVAILABLE',
      exactNock: '',
      detail: v.reason && v.reason.trim() !== '' ? v.reason : 'no reason given',
      route: '',
      hops: [],
      otherRoutes: [],
      swapUrl,
      swapLabel,
    };
  }
  // The candidate the worker reported as this venue's best: same route, same amount.
  const best = tried.find((r) => r.nock_out === v.nock_out && r.route === v.route);
  const price = Number(v.effective_price_usd);
  const priceText =
    v.effective_price_usd && Number.isFinite(price) ? `$${price.toFixed(6)} per NOCK` : 'price unavailable';
  return {
    venue: v.venue,
    quoted: true,
    returnsMore: q.more_nock_venue === v.venue,
    headline: `${nockAmount(v.nock_out)} NOCK`,
    exactNock: v.nock_out,
    detail: `${priceText} · ${costOverSpotText(v.cost_over_spot_pct, q.reference_spot_usd)}`,
    route: best ? routeName(best, tried) : (v.route ?? ''),
    hops: (v.hops ?? []).map(hopLine),
    otherRoutes: tried
      .filter((r) => r !== best)
      .map((r) => {
        const name = routeName(r, tried);
        return r.nock_out ? `${name}: ${nockAmount(r.nock_out)} NOCK` : `${name}: unavailable (${r.reason ?? 'no reason given'})`;
      }),
    swapUrl,
    swapLabel,
  };
}

/**
 * An answer read from the worker's memory (the latest-quotes endpoint, or a refresh),
 * with the snapshot's quotes object that was on screen when it was asked for.
 */
export interface WorkerQuotes {
  /** Null when the worker holds no quotes (nothing due in its latest cycle). */
  quotes: RouteQuotes | null;
  /** The snapshot's `route_quotes` when the request was issued; null when it had none. */
  basis: RouteQuotes | null;
}

/** How often the panel asks the worker for the quotes it holds. */
export const ROUTE_QUOTES_POLL_MS = 30_000;

/** Changes whenever the snapshot's quotes do; "none" without quotes. */
export function quotesKey(q: RouteQuotes | null | undefined): string {
  return q ? `${q.block_number ?? 'no-block'}|${q.quoted_at ?? ''}|${q.status}|${q.amount_usdc}` : 'none';
}

/**
 * The quotes to draw. The snapshot's quotes are left out of its state hash, and the
 * dashboard only takes a new snapshot when that hash changes, so the snapshot's copy
 * can be older than what the worker holds. An answer read from the worker's memory is
 * therefore shown instead, while the snapshot object it was asked against is still the
 * one on screen: it was read after that snapshot arrived, so it is the same or newer,
 * including when it says "unavailable". Once a new snapshot arrives the answer is
 * dropped until the worker is asked again. An answer for another amount than the
 * tranche the plan panel shows is never drawn.
 */
export function quotesToShow(
  snapshot: RouteQuotes | null | undefined,
  worker: WorkerQuotes | null | undefined,
  trancheUsdc?: string | null,
): RouteQuotes | null {
  const snap = snapshot ?? null;
  if (!worker || worker.basis !== snap || !worker.quotes) return snap;
  const amount = trancheUsdc ?? snap?.amount_usdc ?? null;
  return amount !== null && worker.quotes.amount_usdc === amount ? worker.quotes : snap;
}

/**
 * What to say when a refresh came back with nothing quoted: the worker answered, and
 * its answer is "unavailable". Null when at least one venue was quoted.
 */
export function refreshFailure(quotes: RouteQuotes): string | null {
  if (quotes.status !== 'unavailable') return null;
  return `neither venue could be quoted (${quotes.detail && quotes.detail.trim() !== '' ? quotes.detail : 'no reason given'})`;
}

export interface QuoteCostLabel {
  /** False when no quote was on record for the fill's tranche. */
  known: boolean;
  text: string;
  /** "best quote: Aero, $20.00 for 1,773.0768 NOCK at block 58,081,910 (2026-11-16 05:25:57 UTC)" */
  reference: string;
}

type QuoteCostFigures = Pick<
  AccumulationEvent,
  | 'quote_venue'
  | 'quote_usdc'
  | 'quote_nock_out'
  | 'quote_effective_price_usd'
  | 'quote_block_number'
  | 'quoted_at'
  | 'quote_cost_bps'
>;

/**
 * A fill's cost over the best quote that was current for its tranche. Without a stored
 * quote there is no figure: "no quote on record", never 0 and never a later quote. The
 * worker pairs them (same tranche, read within 60 minutes before the fill).
 */
export function costOverQuote(f: QuoteCostFigures): QuoteCostLabel {
  const bps = f.quote_cost_bps;
  if (typeof bps !== 'number' || !Number.isFinite(bps) || !f.quote_effective_price_usd || !f.quote_venue) {
    return { known: false, text: 'no quote on record', reference: '' };
  }
  const pct = (Math.abs(bps) / 100).toFixed(2);
  const text =
    bps === 0 ? 'at the quote' : `${bps > 0 ? '+' : '−'}${Math.abs(bps)} bps (${pct}%) ${bps > 0 ? 'over' : 'under'} the quote`;
  const amount = f.quote_usdc ? `$${f.quote_usdc} for ` : '';
  const nock = f.quote_nock_out ? `${nockAmount(f.quote_nock_out)} NOCK ` : '';
  const block = typeof f.quote_block_number === 'number' ? `at block ${grouped(f.quote_block_number)}` : 'block not recorded';
  const when = clock(f.quoted_at);
  return {
    known: true,
    text,
    reference: `best quote: ${f.quote_venue}, ${amount}${nock}${block}${when ? ` (${when})` : ''}`,
  };
}
