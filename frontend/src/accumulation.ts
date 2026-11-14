/**
 * Accumulation plan display helpers (snapshot 2.1.0, issue #79).
 *
 * Pure functions only: everything shown comes from the worker's snapshot, which in
 * turn derives plan progress from receipt-verified buys. Nothing here invents a
 * number — a missing value renders as an em dash.
 */
import type { AccumulationPlanStatus, LpRebuild, LpSuggestion } from './types/snapshot';

/** "78.20 USDC + 8,730 NOCK" — legs with a zero amount are left out (single-sided ranges). */
export function suggestionLegs(
  sg: Pick<LpSuggestion, 'amount0' | 'token0_symbol' | 'amount1' | 'token1_symbol'>,
): string {
  const fmt = (amount: string) => {
    const decimals = amount.includes('.') ? amount.split('.')[1].length : 0;
    return parseFloat(amount).toLocaleString('en-US', {
      minimumFractionDigits: decimals,
      maximumFractionDigits: decimals,
    });
  };
  return [
    [sg.amount0, sg.token0_symbol],
    [sg.amount1, sg.token1_symbol],
  ]
    .filter(([amount]) => parseFloat(amount) > 0)
    .map(([amount, symbol]) => `${fmt(amount)} ${symbol}`)
    .join(' + ');
}

/**
 * Button text for a suggestion's link, taken from where the link really goes. The
 * button used to read "Open in Uniswap" for every suggestion, including the Aerodrome
 * one whose link is the pool on a block explorer. "Uniswap" alone was also ambiguous
 * here: the position link opens the Uniswap web app, not the Uniswap wallet app.
 */
export function suggestionLinkLabel(url: string): string {
  let host = '';
  try {
    host = new URL(url).hostname.toLowerCase();
  } catch {
    return 'OPEN LINK ↗';
  }
  if (host === 'app.uniswap.org') return 'OPEN POSITION IN UNISWAP WEB APP ↗';
  if (host === 'app.aero.xyz') return 'OPEN DEPOSIT ON AERO.XYZ ↗';
  if (host === 'basescan.org') return 'VIEW POOL ON BASESCAN ↗';
  return `OPEN ${host.toUpperCase()} ↗`;
}

export const PLAN_STATUS_LABEL: Record<AccumulationPlanStatus['status'], string> = {
  pending: 'PENDING',
  tranche_due: 'TRANCHE DUE',
  waiting: 'ON SCHEDULE',
  paused: 'PAUSED',
  blocked: 'BLOCKED',
  complete: 'COMPLETE',
  invalid: 'INVALID CONFIG',
};

export const PLAN_STATUS_COLOR: Record<AccumulationPlanStatus['status'], string> = {
  pending: '#8b949e',
  tranche_due: '#38bdf8',
  waiting: '#10b981',
  paused: '#f59e0b',
  blocked: '#f59e0b',
  complete: '#10b981',
  invalid: '#ef4444',
};

/** Share of the plan budget already spent, 0–100. */
export function planProgressPct(plan: Pick<AccumulationPlanStatus, 'spent_usdc' | 'budget_usdc'>): number {
  const spent = parseFloat(plan.spent_usdc);
  const budget = parseFloat(plan.budget_usdc);
  if (!(budget > 0) || !Number.isFinite(spent)) return 0;
  return Math.min(100, Math.max(0, (spent / budget) * 100));
}

/** "in 14h 05m" / "in 3d 2h" / "now" — null when there is no due time. */
export function timeUntil(iso: string | null | undefined, now: Date = new Date()): string | null {
  if (!iso) return null;
  const due = new Date(iso).getTime();
  if (Number.isNaN(due)) return null;
  const mins = Math.round((due - now.getTime()) / 60000);
  if (mins <= 0) return 'now';
  const d = Math.floor(mins / 1440);
  const h = Math.floor((mins % 1440) / 60);
  const m = mins % 60;
  if (d > 0) return `in ${d}d ${h}h`;
  if (h > 0) return `in ${h}h ${String(m).padStart(2, '0')}m`;
  return `in ${m}m`;
}

/**
 * DCA vs "bought it all at the first fill": NOCK the plan is on pace to acquire for the
 * whole budget at its average cost, minus the lump-sum counterfactual. Positive means
 * averaging is ahead. Null until there is at least one fill.
 */
export function dcaVsLumpSumNock(
  plan: Pick<AccumulationPlanStatus, 'avg_price_usd' | 'lump_sum_nock' | 'budget_usdc'>,
): number | null {
  if (!plan.avg_price_usd || !plan.lump_sum_nock) return null;
  const avg = parseFloat(plan.avg_price_usd);
  const lump = parseFloat(plan.lump_sum_nock);
  const budget = parseFloat(plan.budget_usdc);
  if (!(avg > 0) || !Number.isFinite(lump) || !(budget > 0)) return null;
  return budget / avg - lump;
}

/** True when the standing LP-rebuild bias should be shown. */
export function lpRebuildActive(lp: LpRebuild | null | undefined): boolean {
  return lp?.status === 'under_target';
}

/** 32-byte hex transaction hash, with or without 0x. */
export function isTxHash(s: string): boolean {
  return /^(0x)?[0-9a-fA-F]{64}$/.test(s.trim());
}
