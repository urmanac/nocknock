import { describe, expect, it } from 'vitest';
import { renderToStaticMarkup } from 'react-dom/server';
import { FloorBreachNote } from './FloorBreachNote';
import { LIVE_FLOOR_BREACH } from '../floorBreach.fixture';

describe('FloorBreachNote (#165)', () => {
  it('renders the measured line under the Floor Health tile, with the assumption as its title', () => {
    const html = renderToStaticMarkup(<FloorBreachNote floorBreach={LIVE_FLOOR_BREACH} />);
    expect(html).toContain('data-testid="floor-breach"');
    expect(html).toContain('Floor crossed if NOCK ≥ $0.025405 (+96.3%); v4 USDC/NOCK 3.00% #3163450 sells out first (at $0.041055)');
    expect(html).toContain('ETH/USD constant');
    expect(html).not.toMatch(/<button|<a /);
  });

  it('renders nothing without a floor or from an older worker', () => {
    expect(renderToStaticMarkup(<FloorBreachNote floorBreach={undefined} />)).toBe('');
    expect(renderToStaticMarkup(<FloorBreachNote floorBreach={{ status: 'no_floor', floor_nock: '0', positions: [], assumption: '' }} />)).toBe('');
  });
});
