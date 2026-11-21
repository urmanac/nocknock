/**
 * The deep-link anchors of the events the top-of-page strip does not render (#169 review):
 * quiet entries, and top entries whose own card renders them (the ladder suggestion, the
 * positions found on-chain). Each is an empty element with `id="event-…"` placed where its
 * `card` is shown, so every event's `deep_link` lands on the place that shows it.
 *
 * With `card` unset (the Lite cockpit) every such event gets its anchor here.
 */
import { anchorId, stripRenders, type SnapshotEvent } from '../events';

export function EventAnchors({ events, card }: { events: SnapshotEvent[] | null; card?: string }) {
  const here = (events ?? []).filter((e) => !stripRenders(e) && (card === undefined || e.card === card));
  if (here.length === 0) return null;
  return (
    <>
      {here.map((e) => (
        <span key={e.id} id={anchorId(e)} data-event-anchor={e.kind} />
      ))}
    </>
  );
}
