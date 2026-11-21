import { floorBreachLine, type FloorBreach } from '../floor';

/**
 * The floor-crossing price (#165) as a muted line under the Floor Health tile: a
 * measurement of the deployed ranges, never an instruction. Renders nothing when no
 * floor is configured or the snapshot predates 2.9.0.
 */
export function FloorBreachNote({ floorBreach }: { floorBreach: FloorBreach | null | undefined }) {
  const text = floorBreachLine(floorBreach);
  if (!text) return null;
  return (
    <div
      data-testid="floor-breach"
      title={floorBreach?.assumption}
      style={{ fontSize: '0.65rem', color: '#9ca3af', fontFamily: 'monospace', marginTop: 4, overflowWrap: 'anywhere' }}
    >
      {text}
    </div>
  );
}
