/**
 * Closed positions (snapshot 2.8.0, issue #144).
 *
 * A closed position is a configured position whose NFT no longer exists: the operator
 * withdrew it and the wallet app burned the NFT (a mistyped nft_id reads the same). The
 * worker lists it in `data.closed_positions`, never in `data.positions`, and counts
 * nothing of it in any total.
 *
 * These helpers only restate the snapshot. They print no amount and no value for a
 * closed position: none was read, and its last known value is not a current one. A
 * snapshot without the field (every one from before 2.8.0, and every one without a
 * closed position) yields an empty list, so it renders exactly as before.
 */

export interface ClosedPositionLike {
  config_id?: string;
  position_id?: string;
  protocol?: string;
  nft_id?: number;
  label?: string;
  status?: string;
  reason?: string;
  position_manager?: string;
  first_seen_closed_at?: string | null;
  supplies_spot_price?: boolean;
}

/** The snapshot's closed positions; empty when the snapshot lists none. */
export function closedPositionsOf(data: unknown): ClosedPositionLike[] {
  const list = data && typeof data === 'object' ? (data as { closed_positions?: unknown }).closed_positions : undefined;
  return Array.isArray(list) ? (list.filter((c) => c && typeof c === 'object') as ClosedPositionLike[]) : [];
}

/** What the position is called: the worker's label, or its config id. */
export function closedPositionName(c: ClosedPositionLike): string {
  return (c.label ?? '').trim() || (c.config_id ?? '').trim() || 'Position';
}

/** The fact, in the same words on every surface. */
export function closedPositionFact(c: ClosedPositionLike): string {
  return Number.isFinite(c.nft_id)
    ? `closed: NFT #${c.nft_id} no longer exists (withdrawn and burned, or a mistyped nft_id)`
    : 'closed: its NFT no longer exists (withdrawn and burned, or a mistyped nft_id)';
}

/** That nothing of it is counted. */
export const CLOSED_NOT_COUNTED = 'not counted in any total';

/**
 * What the operator can do with the config entry. A Uniswap v4 price source whose pool
 * still supplies the NOCK/USD spot must be replaced first: removing the entry would
 * leave the worker without a price.
 */
export function closedPositionConfigHint(c: ClosedPositionLike): string {
  if (c.supplies_spot_price === true) {
    return 'its pool still supplies the NOCK/USD spot: flag another position as is_price_source before removing its entry from config.yaml';
  }
  const id = (c.config_id ?? '').trim();
  return id ? `you can remove its entry (${id}) from config.yaml` : 'you can remove its entry from config.yaml';
}

/** One closed position as one line. */
export function closedPositionLine(c: ClosedPositionLike): string {
  return `${closedPositionName(c)}: ${closedPositionFact(c)}; ${CLOSED_NOT_COUNTED}; ${closedPositionConfigHint(c)}`;
}

/** "CLOSED POSITIONS (1)". */
export function closedPositionsHeader(list: ClosedPositionLike[]): string {
  return `CLOSED POSITIONS (${list.length})`;
}

/** When the worker first recorded it as closed (not the time of the burn), or null. */
export function closedFirstSeenText(c: ClosedPositionLike): string | null {
  const at = (c.first_seen_closed_at ?? '').trim();
  if (!at) return null;
  const t = Date.parse(at);
  if (!Number.isFinite(t)) return null;
  return `first recorded as closed ${new Date(t).toISOString().replace('.000Z', 'Z')}`;
}

/**
 * The line under the tick-bracket chart when the NOCK/USD spot comes from the pool of a
 * closed position (a Uniswap v4 entry whose NFT is gone): there is no price-source
 * position, so there is no range to state. Null whenever a live position is the price
 * source, and for every snapshot without such a closed entry (the chart then says what
 * it always said).
 */
export function priceSourceIsClosedText(
  data: unknown,
  positions: Array<{ is_price_source?: boolean }> | null | undefined,
): string | null {
  if ((positions ?? []).some((p) => p?.is_price_source)) return null;
  const source = closedPositionsOf(data).find((c) => c.supplies_spot_price === true);
  if (!source) return null;
  return `Price source: the pool of ${closedPositionName(source)}, a closed position; it has no range`;
}
