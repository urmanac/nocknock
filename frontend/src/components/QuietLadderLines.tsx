/**
 * The quiet forms of two top-of-page cards (#169, operator 2026-11-21: the page "is swamped
 * by the 'call to action' that is no longer a call to action"):
 *
 * - A ladder suggestion with nothing left to deposit (ladder_deployed) leaves the top of the
 *   page: one line, "Ladder: all 3 rungs deployed", with the card behind a collapsed
 *   <details>.
 * - Positions found on-chain with nothing to decide (no offer, the check complete) are one
 *   line, "3 positions tracked from the chain · checked at block N", with the rows, the
 *   UNTRACK buttons and CHECK NOW behind a collapsed <details>.
 *
 * An offer waiting for a tap, or a check that did not complete, keeps the full block at the
 * top. `ladderSuggestionAtTop` and `foundNeedsAttention` decide.
 */
import type { CSSProperties } from 'react';
import type { SyncStatus } from '../hooks/useFreshness';
import type { LadderSuggestion } from '../ladderSuggestion';
import { foundNeedsAttention, foundQuietLine, type PositionDetection } from '../foundPositions';
import { LadderSuggestionCard } from './LadderSuggestionCard';
import { FoundPositionsCard } from './FoundPositions';

const MUTED = '#8b949e';
const LINE: CSSProperties = { color: MUTED, fontSize: '0.78rem', margin: '6px 0', overflowWrap: 'anywhere', minWidth: 0 };

/** Whether the suggestion card belongs at the top: anything but a deployed ladder. */
export function ladderSuggestionAtTop(s: LadderSuggestion | null | undefined): boolean {
  return !!s && s.status !== 'ladder_deployed';
}

export function deployedLine(s: LadderSuggestion): string {
  const n = s.shape?.deployed ?? 0;
  return `Ladder: all ${n} rung${n === 1 ? '' : 's'} deployed`;
}

export function LadderDeployedLine({ suggestion, syncStatus, compact }: { suggestion: LadderSuggestion | null | undefined; syncStatus?: SyncStatus; compact?: boolean }) {
  if (!suggestion || ladderSuggestionAtTop(suggestion)) return null;
  return (
    <details data-testid="ladder-deployed-line" style={{ ...LINE, fontSize: compact ? 12 : LINE.fontSize }}>
      <summary style={{ cursor: 'pointer' }}>{deployedLine(suggestion)}</summary>
      <LadderSuggestionCard suggestion={suggestion} syncStatus={syncStatus} />
    </details>
  );
}

export function FoundQuietLine({ detection, onChanged, compact }: { detection: PositionDetection | null | undefined; onChanged?: () => void; compact?: boolean }) {
  if (!detection || foundNeedsAttention(detection)) return null;
  return (
    <details data-testid="found-quiet-line" style={{ ...LINE, fontSize: compact ? 12 : LINE.fontSize }}>
      <summary style={{ cursor: 'pointer' }}>{foundQuietLine(detection)}</summary>
      <FoundPositionsCard detection={detection} onChanged={onChanged} />
    </details>
  );
}
