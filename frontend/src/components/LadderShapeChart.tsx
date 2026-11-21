/**
 * The ladder shape (#169, operator 2026-11-21): every deployed NOCK range as a bar on a
 * logarithmic USD-per-NOCK axis, its height capital / ln(high/low), so each bar's area is its
 * capital and the stacked outline is the depth profile. A vertical line marks the price, a
 * dashed one the ladder bottom; the reserve kept below it is noted. A rung of unknown
 * capital is an outline, never a guessed height. Full-range pools are stated in the caption,
 * not drawn. The worker computes every figure (derived_metrics.ladder_profile); no input.
 */
import type { CSSProperties } from 'react';
import { ROLE_COLOR, chartGeometry, ladderProfileOf, price4, type LadderProfile } from '../ladderProfile';

const MUTED = '#8b949e';
const TEXT = '#c9d1d9';

export function LadderShapeChart({ data, width = 640, compact }: { data: any | null | undefined; width?: number; compact?: boolean }) {
  const p: LadderProfile | null = ladderProfileOf(data);
  if (!p) return null;
  const height = compact ? 170 : 220;
  const g = chartGeometry(p, width, height);
  const small: CSSProperties = { color: MUTED, fontSize: compact ? 11 : '0.72rem', overflowWrap: 'anywhere' };
  const others = p.bars.filter((b) => b.role === 'other');
  return (
    <section data-testid="ladder-shape" style={{ margin: '12px 0', padding: compact ? 0 : '12px 14px', border: compact ? undefined : '1px solid #30363d', borderRadius: 10, background: compact ? undefined : '#0d111766', minWidth: 0 }}>
      <div style={{ fontSize: compact ? 11 : '0.7rem', letterSpacing: 2, color: MUTED, marginBottom: 4 }}>LADDER SHAPE (DEPLOYED NOCK RANGES)</div>
      {g ? (
        <svg
          data-testid="ladder-shape-svg"
          viewBox={`0 0 ${g.width} ${g.height}`}
          width="100%"
          role="img"
          aria-label="Deployed NOCK ranges on a log price axis; each bar's area is its capital"
          style={{ display: 'block', maxWidth: '100%' }}
        >
          <line x1={0} x2={g.width} y1={g.baseY} y2={g.baseY} stroke="#30363d" />
          {g.rects.map((r, i) => (
            <rect key={`${r.id}-${i}`} data-bar={r.id} data-role={r.role} x={r.x} y={r.y} width={Math.max(0, r.w)} height={Math.max(0, r.h)} fill={ROLE_COLOR[r.role] ?? ROLE_COLOR.other} fillOpacity={r.role === 'other' ? 0.45 : 0.85} />
          ))}
          {g.outlines.map((o) => (
            <g key={o.id} data-unknown={o.id}>
              <rect x={o.x} y={g.plotTop} width={Math.max(0, o.w)} height={g.baseY - g.plotTop} fill="none" stroke={MUTED} strokeDasharray="3 3" />
              <text x={o.x + o.w / 2} y={g.plotTop + 12} fill={MUTED} fontSize={10} textAnchor="middle">
                {o.text}
              </text>
            </g>
          ))}
          {g.labels
            .filter((l) => l.role !== 'other')
            .map((l) => (
              <text key={l.id} data-label={l.id} x={l.x} y={l.y} fill={TEXT} fontSize={compact ? 9 : 10} textAnchor="middle">
                {l.text}
              </text>
            ))}
          {g.clipMarks.map((c) => (
            <text key={`${c.id}-${c.side}`} data-clip={c.side} x={c.x} y={g.baseY + 26} fill={MUTED} fontSize={9} textAnchor={c.side === 'left' ? 'start' : 'end'}>
              {c.text}
            </text>
          ))}
          {g.bottomX != null && (
            <g data-marker="bottom">
              <line x1={g.bottomX} x2={g.bottomX} y1={g.plotTop - 6} y2={g.baseY} stroke="#ef4444" strokeDasharray="4 3" />
              <text x={g.bottomX + 3} y={g.baseY + 13} fill="#fca5a5" fontSize={9}>
                bottom ${price4(Number(p.ladder_bottom_usd))}
                {p.cash_reserve_usdc ? ` · $${p.cash_reserve_usdc} reserve kept below` : ''}
              </text>
            </g>
          )}
          {g.spotX != null && (
            <g data-marker="spot">
              <line x1={g.spotX} x2={g.spotX} y1={g.plotTop - 14} y2={g.baseY} stroke="#f0f6fc" strokeWidth={1.5} />
              <text x={g.spotX} y={g.plotTop - 18} fill="#f0f6fc" fontSize={10} textAnchor="end">
                price ${price4(Number(p.spot_usd))}
              </text>
            </g>
          )}
        </svg>
      ) : (
        <div style={small}>No view window: no ladder rung has a price range to draw.</div>
      )}
      {others.length > 0 && (
        <div data-testid="ladder-shape-others" style={small}>
          <span style={{ color: ROLE_COLOR.other }}>■</span> other NOCK ranges (value now, stacked above the rungs):{' '}
          {others.map((b) => `${b.label} $${b.capital_usd ?? '?'} over $${price4(Number(b.low_usd))}–$${price4(Number(b.high_usd))}`).join('; ')}
        </div>
      )}
      <div style={small}>
        <span style={{ color: ROLE_COLOR.cap }}>■</span> cap <span style={{ color: ROLE_COLOR.near }}>■</span> near{' '}
        <span style={{ color: ROLE_COLOR.deep }}>■</span> deep: capital = value as all USDC at the rung's top, which does not change with the price.
      </div>
      {p.full_range.map((f) => (
        <div key={f.position_id} data-testid="ladder-shape-full-range" style={small}>
          Not drawn: {f.text}.
        </div>
      ))}
      {!compact && <div style={small}>{p.statement}</div>}
    </section>
  );
}
