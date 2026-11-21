/**
 * The ladder shape (2.11.0, #169): `derived_metrics.ladder_profile`, every deployed NOCK
 * range as a bar on a logarithmic USD-per-NOCK axis. The worker computes each bar's capital
 * and density (capital / ln(high/low)); this file only lays the bars out, so that on the log
 * axis each bar's AREA is its capital and overlapping ranges stack into the depth profile
 * (operator, 2026-11-21: "so that we can approximate the bottle shape and judge whether it
 * looks bottle-y").
 */

export interface ProfileBar {
  position_id: string;
  label: string;
  name: string;
  role: 'cap' | 'near' | 'deep' | 'ladder' | 'other';
  low_usd: string;
  high_usd: string;
  capital_usd: string | null;
  capital_basis: 'usdc_at_top' | 'value_now';
  density: string | null;
  status: 'measured' | 'capital_unknown';
  clipped_low: boolean;
  clipped_high: boolean;
  clip_text: string | null;
  text: string;
}

export interface ProfileSegment {
  low_usd: string;
  high_usd: string;
  density: string;
  includes_unknown: boolean;
  position_ids: string[];
}

export interface LadderProfile {
  view_low_usd: string | null;
  view_high_usd: string | null;
  spot_usd: string | null;
  ladder_bottom_usd: string | null;
  cash_reserve_usdc: string | null;
  reserve_note: string | null;
  bars: ProfileBar[];
  segments: ProfileSegment[];
  full_range: Array<{ position_id: string; label: string; value_usd: string; text: string }>;
  not_drawn: Array<{ position_id: string; label: string; reason: string }>;
  statement: string;
}

export function ladderProfileOf(data: { derived_metrics?: { ladder_profile?: LadderProfile | null } | null } | null | undefined): LadderProfile | null {
  const p = data?.derived_metrics?.ladder_profile;
  return p && Array.isArray(p.bars) ? p : null;
}

export const ROLE_COLOR: Record<string, string> = {
  cap: '#f59e0b',
  near: '#38bdf8',
  deep: '#818cf8',
  ladder: '#10b981',
  other: '#64748b',
};

/** Ladder rungs at the base (deepest first), other ranges stacked above them. */
const STACK_ORDER = ['deep', 'near', 'cap', 'ladder', 'other'];

/** A price at 4 significant figures: $0.01097, $0.005501. */
export function price4(v: number): string {
  if (!(v > 0) || !Number.isFinite(v)) return '?';
  const digits = Math.max(0, 3 - Math.floor(Math.log10(v)));
  return v.toFixed(digits);
}

export interface ChartRect {
  id: string;
  role: string;
  x: number;
  y: number;
  w: number;
  h: number;
}

export interface ChartBarLabel {
  id: string;
  role: string;
  x: number;
  y: number;
  text: string;
}

export interface ChartGeometry {
  width: number;
  height: number;
  plotTop: number;
  baseY: number;
  rects: ChartRect[];
  outlines: Array<{ id: string; x: number; w: number; text: string }>;
  labels: ChartBarLabel[];
  clipMarks: Array<{ id: string; side: 'left' | 'right'; x: number; text: string }>;
  spotX: number | null;
  bottomX: number | null;
  viewLow: number;
  viewHigh: number;
  /** Density per pixel of height: the scale every bar shares. */
  densityPerPx: number;
}

const n = (s: string | null | undefined) => (s == null ? NaN : Number(s));

/** Lay the profile out in a width × height box. Null when there is no view window. */
export function chartGeometry(p: LadderProfile, width: number, height: number): ChartGeometry | null {
  const lo = n(p.view_low_usd);
  const hi = n(p.view_high_usd);
  if (!(lo > 0) || !(hi > lo)) return null;
  const padX = 8;
  const plotTop = 28;
  const baseY = height - 34;
  const plotH = baseY - plotTop;
  const innerW = width - 2 * padX;
  const x = (v: number) => padX + ((Math.log(Math.min(Math.max(v, lo), hi)) - Math.log(lo)) / (Math.log(hi) - Math.log(lo))) * innerW;
  const known = p.bars.filter((b) => b.density != null);
  // The tallest stack inside the view sets the scale.
  const visible = p.segments.filter((s) => n(s.high_usd) > lo && n(s.low_usd) < hi);
  const maxTotal = Math.max(
    0,
    ...visible.map((s) => s.position_ids.reduce((t, id) => t + (n(known.find((b) => b.position_id === id)?.density) || 0), 0)),
  );
  const densityPerPx = maxTotal > 0 ? maxTotal / plotH : 1;
  const rects: ChartRect[] = [];
  const topOf: Record<string, number> = {};
  for (const s of visible) {
    const x0 = x(n(s.low_usd));
    const x1 = x(n(s.high_usd));
    let y = baseY;
    const here = known
      .filter((b) => s.position_ids.includes(b.position_id))
      .sort((a, b) => STACK_ORDER.indexOf(a.role) - STACK_ORDER.indexOf(b.role));
    for (const b of here) {
      const h = n(b.density) / densityPerPx;
      y -= h;
      rects.push({ id: b.position_id, role: b.role, x: x0, y, w: x1 - x0, h });
      topOf[b.position_id] = Math.min(topOf[b.position_id] ?? baseY, y);
    }
  }
  const outlines = p.bars
    .filter((b) => b.density == null)
    .map((b) => ({ id: b.position_id, x: x(n(b.low_usd)), w: x(n(b.high_usd)) - x(n(b.low_usd)), text: `${b.name}: capital unknown` }));
  const labels: ChartBarLabel[] = p.bars
    .filter((b) => b.density != null)
    .map((b) => {
      const [x0, x1] = [x(n(b.low_usd)), x(n(b.high_usd))];
      const cx = (x0 + x1) / 2;
      // Above the whole stack under the bar's middle, so a range stacked on it never hides it.
      const stackTop = Math.min(topOf[b.position_id] ?? baseY, ...rects.filter((r) => r.x <= cx && cx <= r.x + r.w).map((r) => r.y));
      return { id: b.position_id, role: b.role, x: cx, y: Math.max(12, stackTop - 4), text: b.text };
    });
  const clipMarks: ChartGeometry['clipMarks'] = [];
  for (const b of p.bars) {
    if (b.clipped_low) clipMarks.push({ id: b.position_id, side: 'left', x: padX, text: `← continues to $${price4(n(b.low_usd))}` });
    if (b.clipped_high) clipMarks.push({ id: b.position_id, side: 'right', x: width - padX, text: `continues to $${price4(n(b.high_usd))} →` });
  }
  const spot = n(p.spot_usd);
  const bottom = n(p.ladder_bottom_usd);
  return {
    width,
    height,
    plotTop,
    baseY,
    rects,
    outlines,
    labels,
    clipMarks,
    spotX: spot > 0 && spot >= lo && spot <= hi ? x(spot) : null,
    bottomX: bottom > 0 && bottom >= lo && bottom <= hi ? x(bottom) : null,
    viewLow: lo,
    viewHigh: hi,
    densityPerPx,
  };
}
