/**
 * The standing protected cash reserve on the Total USDC Stack tile (#162).
 *
 * `operator_profile.cash_reserve_usdc` is measured on wallet USDC plus USDC inside
 * NOCK-pair LP; the tile's own total also counts the gas-side ETH/USDC ranges, so the
 * line names what it is measured on. Null when no reserve is set: never a default.
 */
const money = (v: number) => `$${v.toLocaleString('en-US', { minimumFractionDigits: 2, maximumFractionDigits: 2 })}`;

function num(v: string | null | undefined): number | null {
  if (v == null || v === '') return null;
  const n = Number(v);
  return Number.isFinite(n) ? n : null;
}

/** "$950.00 protected reserve (wallet + NOCK LP USDC): $818.18 above it" or "… $150.00 below it". */
export function reserveText(global: {
  cash_reserve_usdc?: string | null;
  reserve_headroom_usdc?: string | null;
  reserve_note?: string | null;
} | null | undefined): string | null {
  const reserve = num(global?.cash_reserve_usdc);
  if (reserve === null) {
    // A configured reserve that is not applied is said, never shown as no reserve.
    return global?.reserve_note ? `protected reserve not applied: ${global.reserve_note}` : null;
  }
  const raw = num(global?.reserve_headroom_usdc);
  if (raw === null) return `${money(reserve)} protected reserve: not measured this cycle`;
  // "-0.00" parses to -0: it is zero, never "$-0.00 above it".
  const headroom = raw === 0 ? 0 : raw;
  const side = headroom < 0 ? `${money(-headroom)} below it` : `${money(headroom)} above it`;
  return `${money(reserve)} protected reserve (wallet + NOCK LP USDC): ${side}`;
}
