/**
 * The ladder suggestion (issue #168): `derived_metrics.ladder_suggestion`, a top-of-page
 * event the worker measures every cycle. These helpers are the card's wording and the
 * geometry of its picture; the worker computes every figure.
 *
 * The card needs no input: it states whether a NOCK ladder fits the operator's own
 * rules, what it would do in plain words, and the numbered deposits with their links.
 * It shows only while the snapshot is fresh. NockNock signs and sends nothing, and the
 * links prefill no amount. After a deposit nothing is typed either: the worker finds the
 * new position on-chain (foundPositions.ts).
 *
 * With a configured shape (operator_profile.ladder_rungs, 2.10.0) the suggestion carries
 * `shape.rungs[]`, every rung of the shape in order: the rungs left to deposit are the
 * numbered deposits, and a rung that is deployed, passed or over is one stated line.
 */
import type { SyncStatus } from './hooks/useFreshness';
import { feeEstimateLine, type FeeEstimateForLines } from './feeYield';

export interface LadderToken {
  symbol: string;
  address: string;
  decimals: number;
}

export interface LadderRung {
  index: number;
  of: number;
  rung: 'cap' | 'near' | 'deep' | null;
  /** A rung of a configured shape (2.10.0): always to_deposit here. */
  status?: 'to_deposit';
  /** A rung of a configured shape: its amounts in plain words. */
  summary?: string;
  in_range_now?: boolean | null;
  /** The exact mint of the remainder, for a transaction builder. */
  mint?: { liquidity_raw: string; amount0_raw: string | null; amount1_raw: string | null };
  /** Only for a deposit that holds NOCK (the cap): the limit and the field that keeps it. */
  nock_limit?: { nock: string; is: string; field_to_type: string; nock_to_enter: string; usdc_then_about: string; text: string } | null;
  fee_estimate?: FeeEstimateForLines | null;
  cap?: { bound_by: string; note: string } | null;
  pool: {
    position_id: string;
    label: string;
    protocol: string;
    pool_id: string;
    token0: LadderToken;
    token1: LadderToken;
    fee_tier_bps: number;
    fee_raw: number | null;
    tick_spacing: number | null;
    hooks: string | null;
  };
  tick_lower: number;
  tick_upper: number;
  bounds_usd: { low: string; high: string };
  bounds_nock_per_usdc: { low: string | null; high: string | null };
  usdc: { amount: string; raw: string; decimals: number; to_enter?: string };
  nock: { amount: string; raw: string; decimals: number; to_enter?: string };
  deep_link: { url: string | null; prefilled: string[]; not_prefilled: string[]; reason: string | null; format_note?: string | null; amount_to_enter?: string | null };
  would_sign: string;
}

export interface LadderOutcome {
  kind: 'fall_inside' | 'fall_to_bottom' | 'rise' | string;
  text: string;
}

export interface LadderPictureBar {
  index: number;
  low_usd: string;
  high_usd: string;
  usdc: string;
  label: string;
  /** A configured shape (2.10.0): the rung, its status, and whether it is already deployed. */
  rung?: 'cap' | 'near' | 'deep';
  status?: string;
  deployed?: boolean;
}

export interface LadderPictureData {
  spot_usd: string | null;
  bottom_usd: string;
  bottom_requested_usd: string | null;
  reserve_usdc: string;
  reserve_label: string;
  bars: LadderPictureBar[];
}

/** One rung of the operator's shape as the card states it (2.10.0): shape.rungs[]. */
export interface LadderShapeRow {
  index: number;
  of: number;
  rung: 'cap' | 'near' | 'deep';
  status: 'to_deposit' | 'deployed' | 'over' | 'passed' | 'none' | 'found' | 'below_minimum' | 'held' | 'not_deposited' | 'partly_withdrawn' | 'withdrawn';
  reason: string | null;
  target_usdc: string;
  deployed_usdc: string;
  remaining_usdc: string;
  counterparts: string[];
  holds_now: { usdc: string; nock: string } | null;
  bounds_usd: { low: string; high: string } | null;
  /** The whole row in plain words. */
  text: string;
}

export interface LadderCandidateRow {
  tick: number | null;
  price_usd: string | null;
  pct_of_spot: string | null;
  usdc_ceiling: string | null;
  nock_at_bottom: string | null;
  nock_per_100_usdc: string;
  avg_buy_price_usd: string | null;
}

export interface LadderSuggestion {
  id: string;
  tier: string;
  status: string;
  shown: boolean;
  headline: string | null;
  reason: string | null;
  sentence: string | null;
  missing: string[];
  rungs: LadderRung[];
  outcomes: LadderOutcome[];
  picture: LadderPictureData | null;
  candidates: { rows: LadderCandidateRow[]; to_pct?: string; step_pct?: string; reason?: string | null } | null;
  /** How a deposited rung becomes tracked: `tracked_automatically` since v0.9.0 (the
   *  worker finds the new position NFT on-chain); `detection_note` says so when this
   *  cycle's check did not run. `manual_config` in a snapshot stored before that. */
  after_deposit: { kind: string; is?: string; text: string; config_snippets?: string[]; detection_status?: string | null; detection_note?: string | null } | null;
  statement: string;
  excluded?: { position_id: string; reason: string }[];
  stated?: { lines?: string[]; note?: string } | null;
  assumptions?: string[];
  /** A configured shape (2.10.0): every rung in order, and how many are left. */
  shape?: { source: string; status: string; of: number; left: number; deployed: number; passed: number; over: number; none: number; total_usdc: string; rungs: LadderShapeRow[] } | null;
  /** What the deposits leave in the wallet, and how the reserve counts the rest. */
  wallet_after?: { text: string; wallet_usdc_after: string; stable_at_gate_usdc: string; wallet_alone_below_reserve: boolean } | null;
  mint_note?: string | null;
  /** How what is left fits the reserve gate; `text` when it was cut (never all or nothing). */
  fit?: { status: string; allowed_usdc?: string | null; needed_usdc?: string | null; shortfall_usdc?: string | null; rule?: string; text?: string | null } | null;
  /** A position found on-chain in the ladder's pool that matches no rung exactly. */
  found_note?: string | null;
  /** In a stored row: what was left out of it. */
  detail_note?: string | null;
}

export const LADDER_TITLE = 'Ladder suggestion (a measurement, not advice)';

/** The snapshot's suggestion, or null from a worker that predates it. */
export function ladderSuggestionOf(data: any): LadderSuggestion | null {
  const l = data?.derived_metrics?.ladder_suggestion;
  return l && typeof l === 'object' && typeof l.status === 'string' ? (l as LadderSuggestion) : null;
}

export type LadderCardMode = 'ready' | 'stale' | 'line';

/** What the card shows: the whole ladder, a stale notice, or one line saying why not. */
export function ladderCardView(s: LadderSuggestion, syncStatus: SyncStatus | undefined): { mode: LadderCardMode; headline: string; line: string | null } {
  if (s.status === 'ready' && s.shown) {
    if (syncStatus === 'stale') {
      return { mode: 'stale', headline: 'Ladder: not shown while the snapshot is stale.', line: 'It shows again once the worker reads the chain.' };
    }
    return { mode: 'ready', headline: s.headline ?? 'Ladder ready.', line: null };
  }
  return { mode: 'line', headline: s.headline ?? 'Ladder: not measured this cycle.', line: s.reason };
}

/** A deposit link is used only on https to the Uniswap web app or the Aero app. */
export function ladderDepositUrl(url: string | null | undefined): string | null {
  if (!url) return null;
  try {
    const u = new URL(url);
    if (u.protocol !== 'https:') return null;
    return u.hostname === 'app.uniswap.org' || u.hostname === 'app.aero.xyz' ? url : null;
  } catch {
    return null;
  }
}

/** The link's label, from what the worker says it prefills. */
export function ladderLinkLabel(r: LadderRung): string {
  const url = ladderDepositUrl(r.deep_link?.url);
  if (!url) return '';
  const host = new URL(url).hostname;
  if (host === 'app.aero.xyz') return 'OPEN POOL DEPOSIT ON AERO.XYZ ↗ (nothing prefilled)';
  const pre = r.deep_link.prefilled ?? [];
  const parts = [
    pre.includes('pair') || pre.includes('fee and tick spacing') ? 'pool' : null,
    pre.includes('range ticks') ? 'range' : null,
    pre.includes('amount') ? 'amount' : null,
  ].filter((x): x is string => !!x);
  if (parts.length === 0) return 'OPEN IN UNISWAP ↗ (nothing prefilled)';
  const words = parts.length === 1 ? parts[0] : `${parts.slice(0, -1).join(', ')} and ${parts[parts.length - 1]}`;
  return `OPEN IN UNISWAP ↗ (${words} prefilled)`;
}

/** The link-format note, once per card: the worker's own words, or null. */
export function linkFormatNote(s: LadderSuggestion): string | null {
  for (const r of s.rungs ?? []) {
    if (r.deep_link?.url && r.deep_link.format_note) return r.deep_link.format_note;
  }
  return null;
}

/** The lines that are stated, not gates (plan budget, tax reserve, cash target). */
export function statedLines(s: LadderSuggestion): string[] {
  return Array.isArray(s.stated?.lines) ? s.stated!.lines! : [];
}

/** Each excluded pool and why, for "no pool can hold one". */
export function excludedLines(s: LadderSuggestion): string[] {
  return (s.excluded ?? []).map((e) => `${e.position_id}: ${e.reason}`);
}

/** A price with 4 significant digits for the plain lines: $0.01274, $0.01002. */
export function price4(v: number): string {
  if (!(Number.isFinite(v) && v > 0)) return v.toFixed(2);
  const dp = Math.max(0, 3 - Math.floor(Math.log10(v)));
  return v.toFixed(dp);
}

/** "Deposit 1 of 2 (near rung): $115.12 USDC + 0 NOCK"; for a rung of a configured shape,
 *  "Deposit 1 of 3 (cap): about 86.79 NOCK + $1.23 USDC, in range now". */
export function rungTitle(r: LadderRung): string {
  if (r.summary) {
    const which = r.rung === 'cap' ? ' (cap)' : r.rung ? ` (${r.rung} rung)` : '';
    return `Deposit ${r.index} of ${r.of}${which}: ${r.summary}`;
  }
  const name = r.rung ? ` (${r.rung} rung)` : '';
  const nock = Number(r.nock?.amount);
  return `Deposit ${r.index} of ${r.of}${name}: $${r.usdc.amount} USDC + ${Number.isFinite(nock) && nock > 0 ? r.nock.amount : '0'} NOCK`;
}

/** The range in both forms: USD per NOCK and NOCK per USDC (the Uniswap app's form). */
export function rungRangeLines(r: LadderRung): string[] {
  const lines = [`$${r.bounds_usd.low} to $${r.bounds_usd.high} per NOCK`];
  if (r.bounds_nock_per_usdc?.low && r.bounds_nock_per_usdc?.high) {
    lines.push(`${r.bounds_nock_per_usdc.low} to ${r.bounds_nock_per_usdc.high} NOCK per USDC (the app's form)`);
  }
  lines.push(`ticks ${r.tick_lower} to ${r.tick_upper}, a new position in ${r.pool.label}`);
  return lines;
}

export type LadderListItem = { kind: 'deposit'; rung: LadderRung } | { kind: 'row'; row: LadderShapeRow };

/** The card's numbered list. With a configured shape it is every rung of the shape in
 *  order: a deposit for a rung left to deposit, the stated row for one that is deployed,
 *  passed or over. Without one it is the deposits as they come. */
export function ladderList(s: LadderSuggestion): LadderListItem[] {
  const rows = s.shape?.rungs;
  if (!rows?.length) return (s.rungs ?? []).map((rung) => ({ kind: 'deposit', rung }));
  return rows.map((row) => {
    const rung = (s.rungs ?? []).find((r) => r.index === row.index);
    return rung ? { kind: 'deposit', rung } : { kind: 'row', row };
  });
}

/** Every rung's stated line, for a card that lists no deposit (all deployed, all passed). */
export function shapeRowLines(s: LadderSuggestion): string[] {
  return (s.shape?.rungs ?? []).map((r) => r.text).filter((t) => typeof t === 'string' && t.length > 0);
}

/** The reserve gate's cut of what is left, in the worker's words, or null when it fits. */
export function fitLine(s: LadderSuggestion): string | null {
  return s.fit?.text ?? null;
}

/** A position found on-chain in the ladder's pool, not counted until one tap, or null. */
export function foundNote(s: LadderSuggestion): string | null {
  return s.found_note ?? null;
}

/** "After these deposits your wallet holds about $779 USDC; …", or null. */
export function walletAfterLine(s: LadderSuggestion): string | null {
  return s.wallet_after?.text ?? null;
}

/** The NOCK limit of a deposit that holds NOCK (the cap), in the worker's words, or null. */
export function rungNockLimit(r: LadderRung): string | null {
  return r.nock_limit?.text ?? null;
}

/** One muted fee line for a rung of a configured shape (never without its sample size,
 *  in-range share and replay bound; the reason when it is not measured). Null for the
 *  single rung of #168, which states no estimate. */
export function rungFeeLine(r: LadderRung): string | null {
  return r.summary ? feeEstimateLine(r.fee_estimate) : null;
}

/** The setting the card is missing, in one plain sentence. */
export function missingSettingSentence(s: LadderSuggestion): string | null {
  if (!s.missing?.length) return null;
  const words = s.missing.map((k) =>
    k === 'operator_profile.ladder_bottom_usd'
      ? 'your bottom price, operator_profile.ladder_bottom_usd'
      : k === 'operator_profile.cash_reserve_usdc'
        ? 'your protected reserve, operator_profile.cash_reserve_usdc'
        : k,
  );
  return `NockNock needs ${words.join(' and ')}, in config.yaml; it does not choose these numbers.`;
}

/** One short line per candidate bottom (no bottom set): a measurement, none picked. */
export function candidateLines(s: LadderSuggestion): string[] {
  return (s.candidates?.rows ?? []).slice(0, 8).map((r) => {
    const amount = r.usdc_ceiling ? `up to $${r.usdc_ceiling} → about ${r.nock_at_bottom} NOCK` : `${r.nock_per_100_usdc} NOCK per $100`;
    return `$${r.price_usd} (${r.pct_of_spot}%): ${amount}, average $${r.avg_buy_price_usd}`;
  });
}

export interface PictureGeometry {
  width: number;
  height: number;
  x: (usd: number) => number;
  min: number;
  max: number;
  spotX: number | null;
  bottomX: number;
  bars: {
    x: number;
    w: number;
    y: number;
    label: string;
    low: number;
    high: number;
    /** Where the label is drawn: inside a bar wide enough for it, otherwise beside it. */
    labelX: number;
    labelAnchor: 'middle' | 'end' | 'start';
    labelInside: boolean;
    /** Already deployed (a configured shape): drawn hatched and muted. */
    deployed: boolean;
  }[];
  /** Rows (y) of the drawing: top labels, the axis line, the axis labels, the reserve
   *  legend, and the deployed legend when a deployed bar is drawn. */
  rows: { labels: number; axis: number; axisLabels: number; legend: number; deployedLegend: number | null };
  fontSize: number;
}

/** The width a label takes at `fontSize`, in drawing units (a little generous). */
export function labelWidth(label: string, fontSize: number): number {
  return label.length * fontSize * 0.6;
}

/**
 * A linear USD-per-NOCK axis from below the bottom to above spot, never below $0. Pure.
 * `width` is the drawing's own unit width: the phone uses a narrow one so its 12-unit
 * labels stay about 11 CSS px at 375px.
 */
export function pictureGeometry(p: LadderPictureData, width = 520): PictureGeometry | null {
  const bottom = Number(p.bottom_usd);
  const spot = p.spot_usd != null ? Number(p.spot_usd) : NaN;
  const highs = p.bars.map((b) => Number(b.high_usd)).filter(Number.isFinite);
  const top = Math.max(...highs, Number.isFinite(spot) ? spot : -Infinity);
  if (!Number.isFinite(bottom) || !Number.isFinite(top) || !(top > bottom)) return null;
  const span = top - bottom;
  const min = Math.max(0, bottom - span * 0.25);
  const max = top + span * 0.1;
  const fontSize = 12;
  const left = 8;
  const right = width - 8;
  const x = (usd: number) => left + ((usd - min) / (max - min)) * (right - left);
  const labels = 16;
  const barTop = 44;
  const bars = p.bars.map((b, i) => {
    const low = Number(b.low_usd);
    const high = Number(b.high_usd);
    const bx = x(low);
    const w = Math.max(2, x(high) - x(low));
    // A label goes inside a bar wide enough for it; a narrow bar (the cap) carries it
    // beside itself, on the side with room, so it stays readable on a phone.
    const lw = labelWidth(b.label, fontSize);
    let labelX = bx + w / 2;
    let labelAnchor: 'middle' | 'end' | 'start' = 'middle';
    let labelInside = true;
    if (lw + 6 > w) {
      if (bx - 4 - lw >= left) {
        labelX = bx - 4;
        labelAnchor = 'end';
        labelInside = false;
      } else if (bx + w + 4 + lw <= right) {
        labelX = bx + w + 4;
        labelAnchor = 'start';
        labelInside = false;
      }
    }
    return { x: bx, w, y: barTop + i * 28, label: b.label, low, high, labelX, labelAnchor, labelInside, deployed: b.deployed === true };
  });
  const axis = barTop + p.bars.length * 28 + 6;
  const anyDeployed = bars.some((b) => b.deployed);
  const rows = { labels, axis, axisLabels: axis + 16, legend: axis + 34, deployedLegend: anyDeployed ? axis + 50 : null };
  return {
    width,
    height: (rows.deployedLegend ?? rows.legend) + 8,
    x,
    min,
    max,
    spotX: Number.isFinite(spot) ? x(spot) : null,
    bottomX: x(bottom),
    bars,
    rows,
    fontSize,
  };
}
