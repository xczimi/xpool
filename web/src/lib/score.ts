/**
 * Pure helpers for the mobile +/− score stepper. Scores are non-negative
 * integers; `null` means "not yet entered". `SCORE_MAX` is a generous sanity
 * cap (the legacy desktop `<select>` only offered 0–9, but real scores can run
 * higher, so we cap at 20 rather than 9).
 */
export const SCORE_MIN = 0
export const SCORE_MAX = 20

export function clampScore(n: number): number {
  if (Number.isNaN(n)) return SCORE_MIN
  return Math.max(SCORE_MIN, Math.min(SCORE_MAX, Math.trunc(n)))
}

/**
 * Apply a +1 / -1 step. `+` from unset commits the minimum (0); `-` from unset
 * stays unset; `-` below 0 unsets again; `+` saturates at `SCORE_MAX`.
 */
export function stepScore(current: number | null, delta: number): number | null {
  if (current === null) {
    return delta > 0 ? SCORE_MIN : null
  }
  const next = current + delta
  if (next < SCORE_MIN) return null
  return clampScore(next)
}

/** How many matches have BOTH sides entered. */
export function predictedCount(
  matches: ReadonlyArray<{ home: number | null; away: number | null }>,
): number {
  return matches.filter((m) => m.home !== null && m.away !== null).length
}
