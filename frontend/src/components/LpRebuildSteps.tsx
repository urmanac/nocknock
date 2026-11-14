/**
 * LP rebuild call to action (v0.5.0, issue #83).
 *
 * Lists the deposits the worker sized for each eligible tracked position, with a link
 * to that position in the Uniswap app, and names the positions it left out and why.
 * The worker never builds or sends the deposit; after the operator adds liquidity the
 * position is read on-chain, so nothing needs recording here.
 */
import type { LpRebuild } from '../types/snapshot';
import { suggestionLegs } from '../accumulation';

export function LpRebuildSteps({ lp }: { lp: LpRebuild }) {
  const suggestions = lp.suggestions ?? [];
  const excluded = lp.excluded ?? [];
  return (
    <div>
      {suggestions.length > 0 ? (
        <div style={{ marginTop: 10, display: 'grid', gap: 8 }}>
          {suggestions.map((sg) => {
            const wanted = parseFloat(sg.target_usd) - parseFloat(sg.current_usd);
            const shrunk = parseFloat(sg.add_usd) + 0.005 < wanted;
            return (
              <div
                key={sg.position_id}
                style={{
                  display: 'flex',
                  gap: 12,
                  flexWrap: 'wrap',
                  alignItems: 'center',
                  justifyContent: 'space-between',
                  background: '#0d1117',
                  border: '1px solid #30363d',
                  borderRadius: 6,
                  padding: '8px 12px',
                }}
              >
                <div style={{ minWidth: 220, flex: 1 }}>
                  <div style={{ color: '#f0f6fc', fontWeight: 600 }}>{sg.label}</div>
                  <div>
                    Add <strong style={{ color: '#f0f6fc' }}>{suggestionLegs(sg)}</strong> (${sg.add_usd}
                    {shrunk ? ` of $${wanted.toFixed(2)} wanted` : ''})
                  </div>
                  {sg.limited_by && (
                    <div style={{ color: '#f59e0b', fontSize: '0.72rem' }}>Limited: {sg.limited_by}.</div>
                  )}
                </div>
                <a
                  href={sg.url}
                  target="_blank"
                  rel="noopener noreferrer"
                  style={{
                    background: '#1f6feb',
                    color: '#f0f6fc',
                    border: '1px solid #30363d',
                    borderRadius: 6,
                    padding: '8px 14px',
                    fontSize: '0.75rem',
                    letterSpacing: 1,
                    textDecoration: 'none',
                    whiteSpace: 'nowrap',
                  }}
                >
                  OPEN IN UNISWAP ↗
                </a>
              </div>
            );
          })}
          <div style={{ fontSize: '0.72rem', color: '#8b949e' }}>
            Amounts are what each range takes at its pool's current price and drift as price moves; the Uniswap app
            shows the exact figures when you add liquidity. Nothing needs recording afterwards: the worker reads the
            position on-chain.
          </div>
        </div>
      ) : (
        <div style={{ marginTop: 8, color: '#f59e0b' }}>No tracked position is eligible for a deposit right now.</div>
      )}
      {excluded.length > 0 && (
        <div style={{ marginTop: 8, fontSize: '0.72rem', color: '#8b949e' }}>
          {excluded.map((ex) => (
            <div key={ex.position_id}>
              ✗ {ex.label}: {ex.reason}
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
