/**
 * Pure time arithmetic for the dev clock's game-relative presets.
 *
 * The domain models only a game's `kickoff` (no end/duration), so a match is
 * treated as ~2h long. Offsets are from kickoff K; the result is a plain UTC
 * instant — no local-timezone shift creeps in, so a kickoff near a DST boundary
 * still yields the right UTC time.
 */

export type DevClockPhase = 'before' | 'during' | 'after'

/** Minutes from kickoff for each phase. See the design doc's offset table. */
const PHASE_OFFSET_MIN: Record<DevClockPhase, number> = {
  before: -10, // predictions still open (deadline future)
  during: 60, // kicked off, result pending
  after: 135, // ~2h match over (90' + halftime + stoppage), +15m
}

/** Instant (RFC3339 UTC) for a phase relative to a game's kickoff K. */
export function devClockInstant(kickoffIso: string, phase: DevClockPhase): string {
  const k = Date.parse(kickoffIso)
  const instant = k + PHASE_OFFSET_MIN[phase] * 60_000
  return new Date(instant).toISOString()
}
