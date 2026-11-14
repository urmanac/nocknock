/**
 * Pure display helpers for the portfolio pillars (snapshot 2.3.0, issue #94).
 *
 * The total has three pillars: NOCK, USDC and ETH. ETH is gas: wallet ETH + WETH is
 * counted in the total but is never cash, never deployable and has no target, so it is
 * shown as a pillar line, never as a "Goal". Fields a pre-2.3.0 snapshot lacks render
 * as "—" or "not reported", never as $0.00.
 */

import { finiteOrNull } from './timeline.ts';

export const NOT_REPORTED = 'not reported (snapshot predates 2.3.0)';

export interface Pillar {
  key: 'NOCK' | 'USDC' | 'ETH';
  usd: number | null;
  /** ETH: gas. Counted in the total; never cash, never deployable, no target. */
  gas: boolean;
}

function money(v: number): string {
  return `$${v.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;
}

/** The three pillars, from the snapshot's own global_status. */
export function pillarsFromGlobal(global: any): Pillar[] {
  return [
    { key: 'NOCK', usd: finiteOrNull(global?.total_native_usd), gas: false },
    { key: 'USDC', usd: finiteOrNull(global?.total_stable_usd), gas: false },
    { key: 'ETH', usd: finiteOrNull(global?.total_eth_usd), gas: true },
  ];
}

/** "NOCK $1,312.88 · USDC $1,033.71 · ETH $66.98"; a missing pillar reads "—". */
export function pillarLine(global: any): string {
  return pillarsFromGlobal(global)
    .map(p => `${p.key} ${p.usd === null ? '—' : money(p.usd)}`)
    .join(' · ');
}

/** The gas fact: wallet ETH + WETH, counted in the total, never cash or deployable. */
export function gasLine(global: any): string {
  const eth = finiteOrNull(global?.wallet_gas_eth);
  const usd = finiteOrNull(global?.wallet_gas_usd);
  if (eth === null) return `Gas held (wallet ETH+WETH): ${NOT_REPORTED}`;
  const usdText = usd === null ? 'USD value not reported' : money(usd);
  return `Gas held (wallet ETH+WETH): ${eth.toFixed(6)} ETH (${usdText}), counted in the total; not cash, not deployable`;
}

/**
 * ETH/USD for converting ETH-denominated brackets. The snapshot's own eth_price_usd
 * (2.3.0) wins. Only a pre-2.3.0 snapshot falls back to the pool-derived ratio of an
 * ETH/WETH reserve, and only from a leg that is not dust (rounded strings make a dust
 * leg's ratio meaningless). Null when neither exists: never a made-up price.
 */
export function ethUsdPrice(global: any, poolMechanics: any[] | undefined | null): number | null {
  const reported = finiteOrNull(global?.eth_price_usd);
  if (reported !== null && reported > 0) return reported;
  for (const pm of poolMechanics || []) {
    const eth = (pm?.volatile_assets || []).find((a: any) => ['ETH', 'WETH'].includes(a?.token_symbol));
    if (!eth) continue;
    const amt = finiteOrNull(eth.reserve_amount);
    const usd = finiteOrNull(eth.reserve_usd);
    // A leg under $1 is rounded too coarsely ({:.2} USD over {:.6} ETH) to price ETH.
    if (amt !== null && usd !== null && amt > 0 && usd >= 1) return usd / amt;
  }
  return null;
}

/**
 * The high-water mark card's subtitle. The mark is the highest stored total since the
 * database began, not an all-time high, so the subtitle says when it was set.
 */
export function hwmSubtitle(date: string | undefined | null): string {
  const ms = date ? Date.parse(date) : NaN;
  if (!Number.isFinite(ms)) return 'Highest stored total (incl. gas)';
  const iso = new Date(ms).toISOString();
  return `Highest stored total (incl. gas) · set ${iso.slice(0, 10)} ${iso.slice(11, 16)} UTC`;
}
