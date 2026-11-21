import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { renderToStaticMarkup } from 'react-dom/server';
import { TopOfPageEvents, TopOfPageLiteEvents } from './TopOfPage';
import { EventAnchors } from './EventAnchors';
import { EVENT_CARDS, anchorId, eventsOf, stripRenders, type SnapshotEvent } from '../events';
import { LadderLiteCard, LaddersCard } from './Ladders';
import { EVENTS_DEEP_CONVERTED, EVENTS_LIVE_UNSET, EVENTS_NEAR_MOVE, EVENTS_PREDATES } from '../events.fixture';
import { LADDERS_BOUGHT, POSITIONS_ABOVE } from '../ladderTrack.fixture';

describe('the top of the page (#169)', () => {
  it('renders nothing with only quiet entries, or from an older snapshot', () => {
    const quietOnly = { derived_metrics: { events: EVENTS_LIVE_UNSET.derived_metrics.events!.filter((e) => e.tier === 'quiet') } };
    expect(renderToStaticMarkup(<TopOfPageEvents data={quietOnly} syncStatus="fresh" />)).toBe('');
    expect(renderToStaticMarkup(<TopOfPageLiteEvents data={quietOnly} syncStatus="fresh" />)).toBe('');
    expect(renderToStaticMarkup(<TopOfPageEvents data={EVENTS_PREDATES} syncStatus="fresh" />)).toBe('');
  });

  it('a near-rung move: title, text, what would be signed, the venue links and the anchor', () => {
    const html = renderToStaticMarkup(<TopOfPageEvents data={EVENTS_NEAR_MOVE} syncStatus="fresh" />);
    expect(html).toContain('data-testid="top-of-page"');
    expect(html).toContain('id="event-ladder-near-move-uniswap-v4-base-3244700"');
    expect(html).toContain('LADDER: MOVE THE NEAR RUNG');
    expect(html).toContain('since 2026-11-22 17:30 UTC');
    expect(html).toContain('keeping its width (1,800 ticks), would be ticks 273300..275100');
    expect(html).toContain('Would sign:');
    expect(html).toContain('href="https://app.uniswap.org/positions/v4/base/3244700"');
    // No input, no button: the operator signs in the wallet, from the venue's page.
    expect(html).not.toMatch(/<input|<button|<form/);
    expect(html).not.toContain('may be out of date');
    // The quiet entries are not in the strip.
    expect(html).not.toContain('LP FEES COLLECTABLE');
    expect(html).not.toContain('CAP SOLD ITS NOCK');
  });

  it('a stale snapshot says so above the entries', () => {
    const html = renderToStaticMarkup(<TopOfPageEvents data={EVENTS_DEEP_CONVERTED} syncStatus="stale" />);
    expect(html).toContain('LADDER: DEEP RUNG FULLY CONVERTED');
    expect(html).toContain('As of the last snapshot: the sync is delayed, so this may be out of date.');
    expect(html).toContain('NockNock does not suggest a lower ladder');
  });

  it('the Lite cockpit shows the same entries as cards with the same anchors', () => {
    const lite = renderToStaticMarkup(<TopOfPageLiteEvents data={EVENTS_NEAR_MOVE} syncStatus="fresh" />);
    expect(lite).toContain('TOP OF PAGE');
    expect(lite).toContain('id="event-ladder-near-move-uniswap-v4-base-3244700"');
    expect(lite).toContain('LADDER: MOVE THE NEAR RUNG');
  });

  it('is the first thing under the title row on the dashboard and above the directive on the phone', () => {
    const app = readFileSync(new URL('../App.tsx', import.meta.url), 'utf8');
    const strip = app.indexOf('<TopOfPageEvents data={rawData}');
    expect(strip).toBeGreaterThan(app.indexOf('</div>\n        </div>\n'));
    expect(strip).toBeLessThan(app.indexOf('<LadderSuggestionCard suggestion='));
    expect(strip).toBeLessThan(app.indexOf('TACTICAL SHOT ADVICE'));
    const phone = readFileSync(new URL('./LitePhoneCards.tsx', import.meta.url), 'utf8');
    expect(phone.indexOf('<TopOfPageLiteEvents')).toBeLessThan(phone.indexOf('DIRECTIVE</div>'));
  });

  it('the Ladders cards carry each rung\'s quiet maintenance line', () => {
    // The worker's maintenance block, keyed to a rung of the #167 fixture.
    const m = structuredClone(EVENTS_LIVE_UNSET.derived_metrics.ladder_maintenance!);
    m.rungs[1].position_id = LADDERS_BOUGHT[0].position_id;
    const data = { positions: POSITIONS_ABOVE, ladders: LADDERS_BOUGHT, derived_metrics: { ladder_maintenance: m } };
    const html = renderToStaticMarkup(<LaddersCard data={data} />);
    expect(html).toContain('data-testid="ladder-maintenance"');
    expect(html).toContain('Maintenance (rule off): Near rung #3244700 has been out of range above');
    expect(html).not.toMatch(/<button|<input/);
    const lite = renderToStaticMarkup(<LadderLiteCard data={data} />);
    expect(lite).toContain('data-testid="ladder-maintenance-lite"');
    // Without the block (an older worker) the cards look as they did.
    expect(renderToStaticMarkup(<LaddersCard data={{ positions: POSITIONS_ABOVE, ladders: LADDERS_BOUGHT }} />)).not.toContain('ladder-maintenance');
  });

  it('the reserve event shows the split and the ETH/USDC ranges line at the top', () => {
    const html = renderToStaticMarkup(<TopOfPageEvents data={EVENTS_LIVE_UNSET} syncStatus="fresh" />);
    expect(html).toContain('id="event-reserve-below"');
    expect(html).toContain('WALLET USDC BELOW THE STANDING RESERVE');
    expect(html).toContain('(wallet USDC $778.77 + NOCK LP USDC $233.86 + ladder USDC $768.24)');
    expect(html).toContain('withdrawn now, wallet USDC would be $877.85');
    expect(html).not.toMatch(/pull now/i);
  });

  it('every event deep link resolves to an element on the page (#169 review)', () => {
    const app = readFileSync(new URL('../App.tsx', import.meta.url), 'utf8');
    for (const card of EVENT_CARDS) {
      expect(app, card).toContain(`<EventAnchors events={pageEvents} card="${card}" />`);
    }
    // The worker's events, plus the kinds the live fixtures do not carry, as the worker states them.
    const base = eventsOf(EVENTS_NEAR_MOVE)![0];
    const extra: SnapshotEvent[] = [
      { ...base, id: 'ladder_suggestion:NOCK', kind: 'ladder_suggestion', tier: 'top', card: 'ladder_suggestion', deep_link: '#event-ladder-suggestion-nock' },
      { ...base, id: 'found_positions', kind: 'found_positions', tier: 'top', card: 'found_positions', deep_link: '#event-found-positions' },
      { ...base, id: 'fill_offer:0xab', kind: 'fill_offer', tier: 'top', card: 'accumulation_plan', deep_link: '#event-fill-offer-0xab' },
      { ...base, id: 'closed:uniswap-v4:base:1', kind: 'closed_position', tier: 'quiet', card: 'closed_positions', deep_link: '#event-closed-uniswap-v4-base-1' },
      { ...base, id: 'reserve_setting', kind: 'reserve_setting', tier: 'quiet', card: 'reserve', deep_link: '#event-reserve-setting' },
    ];
    const all = [EVENTS_LIVE_UNSET, EVENTS_NEAR_MOVE, EVENTS_DEEP_CONVERTED].flatMap((f) => eventsOf(f)!).concat(extra);
    for (const e of all) {
      expect(EVENT_CARDS as readonly string[], e.id).toContain(e.card);
      const page = [
        renderToStaticMarkup(<TopOfPageEvents data={{ derived_metrics: { events: [e] } }} syncStatus="fresh" />),
        ...EVENT_CARDS.map((c) => renderToStaticMarkup(<EventAnchors events={[e]} card={c} />)),
      ].join('');
      // Exactly one element carries the anchor: the strip, or the event's own place.
      expect(page.split(`id="${anchorId(e)}"`).length - 1, e.id).toBe(1);
      expect(stripRenders(e) || renderToStaticMarkup(<EventAnchors events={[e]} card={e.card!} />).includes(anchorId(e)), e.id).toBe(true);
      // The Lite cockpit holds it too: its strip or its anchors.
      const lite = renderToStaticMarkup(<TopOfPageLiteEvents data={{ derived_metrics: { events: [e] } }} syncStatus="fresh" />) + renderToStaticMarkup(<EventAnchors events={[e]} />);
      expect(lite.split(`id="${anchorId(e)}"`).length - 1, e.id).toBe(1);
    }
    const phone = readFileSync(new URL('./LitePhoneCards.tsx', import.meta.url), 'utf8');
    expect(phone).toContain('<EventAnchors events={eventsOf(snapshot.data)} />');
  });
});
