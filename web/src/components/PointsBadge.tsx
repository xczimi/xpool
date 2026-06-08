/**
 * The points a single prediction earned against the official result —
 * round-multiplied server-side so per-game points sum to the scoreboard stage
 * total. A "perfect" (max base score) gets a star and a brighter treatment.
 *
 * Renders nothing until the game has a result (`points == null`), so callers
 * can drop it straight into a cell without guarding.
 */
export function PointsBadge({
  points,
  isPerfect,
}: {
  points: number | null | undefined
  isPerfect?: boolean
}) {
  if (points == null) return null
  return (
    <span
      className={`points-badge${isPerfect ? ' is-perfect' : ''}`}
      title={isPerfect ? `${points} (perfect)` : `${points}`}
    >
      {isPerfect && <span aria-hidden="true">★ </span>}
      {points}
    </span>
  )
}
