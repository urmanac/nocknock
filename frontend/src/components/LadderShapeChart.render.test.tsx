import { describe, expect, it } from 'vitest';
import { readFileSync } from 'node:fs';
import { renderToStaticMarkup } from 'react-dom/server';
import { LadderShapeChart } from './LadderShapeChart';
import { LaddersCard, LadderLiteCard } from './Ladders';
import { FoundQuietLine, LadderDeployedLine, ladderSuggestionAtTop } from './QuietLadderLines';
import { chartGeometry, ladderProfileOf, type LadderProfile } from '../ladderProfile';
import { EVENTS_LIVE_UNSET } from '../events.fixture';
import { LADDERS_BOUGHT, LADDERS_ABOVE, POSITIONS_ABOVE } from '../ladderTrack.fixture';
import { foundNeedsAttention, foundQuietLine, type PositionDetection } from '../foundPositions';
import type { LadderSuggestion } from '../ladderSuggestion';
import { LADDER_BOTTLE_ALL_DEPLOYED } from '../ladderBottle.fixture';
import { DETECTION_TRACKED_BY_ITSELF } from '../foundPositions.fixture';

const live = EVENTS_LIVE_UNSET as any;
const profile = (): LadderProfile => structuredClone(ladderProfileOf(live)!);
const ln = Math.log;

describe('the ladder shape (#169)', () => {
  it('the worker bars of the live bottle', () => {
    const p = profile();
    const by = (id: string) => p.bars.find((b) => b.position_id === id)!;
    expect([by('uniswap-v4:base:3244701').text, by('uniswap-v4:base:3244701').low_usd, by('uniswap-v4:base:3244701').high_usd]).toEqual(['deep $629.52', '0.01002405', '0.01096801']);
    expect([by('uniswap-v4:base:3244700').text, by('uniswap-v4:base:3244700').high_usd]).toEqual(['near $111.09', '0.01313098']);
    expect([by('uniswap-v4:base:3244699').text, by('uniswap-v4:base:3244699').high_usd]).toEqual(['cap $27.63', '0.01353085']);
    expect(by('uniswap-v4:base:3163450').capital_basis).toBe('value_now');
  });

  it('each drawn bar has area proportional to its capital on the log axis', () => {
    const p = profile();
    const g = chartGeometry(p, 640, 220)!;
    const pxPerLn = (640 - 16) / (ln(g.viewHigh) - ln(g.viewLow));
    for (const b of p.bars.filter((x) => x.role !== 'other')) {
      // Area in (ln price × density) units, back from pixels: the bar's capital, to the cent.
      const area = g.rects.filter((r) => r.id === b.position_id).reduce((t, r) => t + (r.w / pxPerLn) * r.h * g.densityPerPx, 0);
      expect(Math.abs(area - Number(b.capital_usd)), b.name).toBeLessThan(0.005);
    }
  });

  it('stacks overlapping ranges: the wide range sits on the deep rung', () => {
    const g = chartGeometry(profile(), 640, 220)!;
    const deep = g.rects.find((r) => r.id === 'uniswap-v4:base:3244701')!;
    const wideOnDeep = g.rects.find((r) => r.id === 'uniswap-v4:base:3163450' && Math.abs(r.x - deep.x) < 1e-9)!;
    expect(wideOnDeep.y + wideOnDeep.h).toBeCloseTo(deep.y, 9);
  });

  it('renders the SVG: bars, labels, the price line, the dashed bottom, clip marks and the caption', () => {
    const html = renderToStaticMarkup(<LadderShapeChart data={live} />);
    expect(html).toContain('data-testid="ladder-shape-svg"');
    expect(html.match(/<rect [^>]*data-bar=/g)!.length).toBeGreaterThanOrEqual(4);
    for (const t of ['deep $629.52', 'near $111.09', 'cap $27.63']) expect(html).toContain(t);
    expect(html).toContain('data-marker="spot"');
    expect(html).toContain('price $0.01356');
    expect(html).toMatch(/data-marker="bottom"><line[^>]*stroke-dasharray="4 3"/);
    expect(html).toContain('bottom $0.01000 · $950.00 reserve kept below');
    expect(html).toContain('continues to $0.04106 →');
    expect(html).toContain('← continues to $0.005501');
    expect(html).toContain('Not drawn: Aero USDC/NOCK 0.30% (full range) $225.36: full range, no finite width, not drawn.');
    expect(html).not.toMatch(/<input|<button|<form/);
  });

  it('a rung of unknown capital is an outline, never a height', () => {
    const p = profile();
    const near = p.bars.find((b) => b.role === 'near')!;
    near.capital_usd = null;
    near.density = null;
    near.status = 'capital_unknown';
    near.text = 'near: capital unknown';
    const html = renderToStaticMarkup(<LadderShapeChart data={{ derived_metrics: { ladder_profile: p } }} />);
    expect(html).toContain('data-unknown="uniswap-v4:base:3244700"');
    expect(html).toContain('near: capital unknown');
    expect(html).not.toContain('data-bar="uniswap-v4:base:3244700"');
  });

  it('renders nothing without the block', () => {
    expect(renderToStaticMarkup(<LadderShapeChart data={{ derived_metrics: {} }} />)).toBe('');
  });
});

describe('only real calls to action at the top (#169)', () => {
  const deployed: LadderSuggestion = LADDER_BOTTLE_ALL_DEPLOYED;
  const one = DETECTION_TRACKED_BY_ITSELF.tracked[0];
  const detection: PositionDetection = { ...DETECTION_TRACKED_BY_ITSELF, status: 'ok', tracked: [one, { ...one, position_id: 'b' }, { ...one, position_id: 'c' }], checked_at_block: 60435277 };

  it('a deployed ladder and found positions with nothing to decide are quiet lines', () => {
    expect(ladderSuggestionAtTop(deployed)).toBe(false);
    expect(ladderSuggestionAtTop({ ...deployed, status: 'ready' } as LadderSuggestion)).toBe(true);
    expect(foundNeedsAttention(detection)).toBe(false);
    expect(foundNeedsAttention({ ...detection, found_not_counted: { ...detection.found_not_counted, count: 1 } })).toBe(true);
    expect(foundNeedsAttention({ ...detection, status: 'partial' })).toBe(true);
    expect(foundQuietLine(detection)).toBe('3 positions tracked from the chain · checked at block 60435277');
    const line = renderToStaticMarkup(<LadderDeployedLine suggestion={deployed} syncStatus="fresh" />);
    expect(line).toMatch(/^<details data-testid="ladder-deployed-line"[^>]*><summary[^>]*>Ladder: all 3 rungs deployed<\/summary>/);
    expect(line).not.toContain('<details open');
    const found = renderToStaticMarkup(<FoundQuietLine detection={detection} />);
    expect(found).toContain('3 positions tracked from the chain · checked at block 60435277</summary>');
    expect(found).toContain('data-testid="found-positions"');
    // A real call to action is not a quiet line.
    expect(renderToStaticMarkup(<FoundQuietLine detection={{ ...detection, status: 'partial' }} />)).toBe('');
  });

  it('the page puts them in the ladder section, and the phone does the same', () => {
    const app = readFileSync(new URL('../App.tsx', import.meta.url), 'utf8');
    expect(app).toContain('{suggestionAtTop && <LadderSuggestionCard suggestion={suggestion} syncStatus={freshness.syncStatus} />}');
    expect(app).toContain('{foundAtTop && <FoundPositionsCard detection={detection} onChanged={refetch} />}');
    const section = app.indexOf('<LadderShapeChart data={rawData} />');
    expect(section).toBeGreaterThan(app.indexOf('TACTICAL SHOT ADVICE'));
    expect(app.indexOf('<LadderDeployedLine')).toBeGreaterThan(section);
    expect(app.indexOf('<FoundQuietLine')).toBeGreaterThan(section);
    expect(app.indexOf('<LaddersCard data={rawData} />')).toBeGreaterThan(section);
    const phone = readFileSync(new URL('./LitePhoneCards.tsx', import.meta.url), 'utf8');
    expect(phone).toContain('ladderSuggestionAtTop(ladderSuggestionOf(snapshot.data)) ? (');
    expect(phone).toContain('foundNeedsAttention(positionDetectionOf(snapshot.data)!) ? (');
    expect(phone).toContain('<LadderShapeChart data={snapshot.data} width={340} compact />');
  });
});

/** The CSS rules that keep text inside the card, at a phone and a desktop width. jsdom lays
 *  nothing out, so the test checks the rules and estimates the widest unbreakable run. */
describe('compact ladder rows never overflow (#169)', () => {
  const m = structuredClone(live.derived_metrics.ladder_maintenance);
  m.rungs[1].position_id = LADDERS_BOUGHT[0].position_id;
  const data = { positions: POSITIONS_ABOVE, ladders: LADDERS_BOUGHT, derived_metrics: { ladder_maintenance: m } };

  for (const width of [375, 1280]) {
    it(`at ${width} px`, () => {
      const card = renderToStaticMarkup(<LaddersCard data={data} />);
      const html = renderToStaticMarkup(<div style={{ width }}><LaddersCard data={data} /></div>);
      // The card breaks long text anywhere and is never wider than its parent.
      expect(html).toMatch(/data-testid="ladders" style="[^"]*overflow-wrap:anywhere[^"]*max-width:100%/);
      // Every flex row wraps, and every child of one may shrink.
      const flexRows = html.match(/<div style="display:flex[^"]*"/g) ?? [];
      expect(flexRows.length).toBeGreaterThan(0);
      for (const row of flexRows) {
        expect(row).toContain('flex-wrap:wrap');
        expect(row).toContain('min-width:0');
      }
      const rowChildren = html.match(/<div style="display:flex[^>]*>(.*?)<\/div><details/g) ?? [];
      for (const r of rowChildren) {
        for (const span of r.match(/<span [^>]*>/g) ?? []) expect(span).toContain('min-width:0');
      }
      expect(card).not.toMatch(/white-space:nowrap|[^-]width:\d+px/);
      // Each summary line: one row per rung, the long figures behind <details>.
      expect(html.match(/data-testid="ladder-row"/g)!.length).toBe(LADDERS_BOUGHT.length);
      expect(html.match(/<details/g)!.length).toBe(LADDERS_BOUGHT.length);
      const summaries = [...html.matchAll(/data-testid="ladder-summary"[^>]*>([^<]*)</g)].map((x) => x[1]);
      // The widest unbreakable run of a summary fits the card's inner width at 0.78rem
      // (about 7.5 px a character), even before overflow-wrap breaks it.
      const inner = width - 2 * 14 - 2 * 16;
      for (const s of summaries) {
        const longest = Math.max(...s.split(/\s+/).map((w) => w.length));
        expect(longest * 7.5, s).toBeLessThan(inner);
        expect(s).toMatch(/^holds \$[\d,]+\.\d{2} USDC \+ [\d,]+ NOCK · (\$0\.\d{4,6}|no bottom)–(\$0\.\d{4,6}|no top)$/);
      }
    });
  }

  it('the phone card wraps too, and its maintenance line is behind <details>', () => {
    const html = renderToStaticMarkup(<LadderLiteCard data={{ ...data, ladders: LADDERS_ABOVE }} />);
    expect(html).toMatch(/<section style="[^"]*overflow-wrap:anywhere[^"]*" data-testid="ladders-lite"/);
  });
});

describe('the ladder shape on a wide screen (v0.9.1)', () => {
  it('stops growing at 1.25x its drawn width, so its text keeps a readable size', () => {
    const html = renderToStaticMarkup(<LadderShapeChart data={live} />);
    expect(html).toMatch(/data-testid="ladder-shape-svg"[^>]*max-width:800px/);
  });
});
