/**
 * Pure countdown arithmetic for the My Tips finalize deadline — no clock, no
 * I/O, so it is fully unit-testable. The live clock anchoring lives in
 * `useServerClock`; this module only does math and formatting.
 *
 * Server-authoritative clock (CLAUDE.md): nothing here reads `Date.now()`. The
 * caller supplies an already-estimated server-now (anchored to the GraphQL
 * `now`), so the formatted countdown tracks the server's clock, not the
 * browser's.
 */

const pad2 = (n: number): string => String(n).padStart(2, '0')

/** Signed offset of the server clock from the client clock, in ms. */
export function clockSkewMs(serverNowIso: string, clientNowMs: number): number {
  return Date.parse(serverNowIso) - clientNowMs
}

/** Milliseconds until `deadlineIso`, given an estimated server-now in ms. */
export function remainingMs(
  deadlineIso: string,
  estimatedServerNowMs: number,
): number {
  return Date.parse(deadlineIso) - estimatedServerNowMs
}

/**
 * Format a remaining duration as `HH:MM:SS`, prefixed with `Nd ` once a full
 * day or more remains (e.g. `3d 04:11:22`). Negative input clamps to zero —
 * callers treat `<= 0` as expired and render the closed label instead.
 */
export function formatCountdown(msRemaining: number): string {
  const totalSeconds = Math.max(0, Math.floor(msRemaining / 1000))
  const days = Math.floor(totalSeconds / 86_400)
  const hours = Math.floor((totalSeconds % 86_400) / 3600)
  const minutes = Math.floor((totalSeconds % 3600) / 60)
  const seconds = totalSeconds % 60
  const hms = `${pad2(hours)}:${pad2(minutes)}:${pad2(seconds)}`
  return days > 0 ? `${days}d ${hms}` : hms
}
