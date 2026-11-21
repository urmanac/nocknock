/**
 * Claim actions in the phone cockpit. The worker builds one action per position, so two
 * Uniswap v4 positions (the tracked range and a ladder rung found on-chain, v0.9.0) are
 * two actions with the same `platform`. Each needs its own row and its own state.
 */

/** The key of one claim action: its label, which names the position ("… (id #nft)"). */
export function claimActionKey(a: { platform: string; label?: string | null }): string {
  const label = (a.label ?? '').trim();
  return label ? `${a.platform}|${label}` : a.platform;
}
