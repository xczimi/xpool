import type { SingleGame } from '../graphql/types'

/**
 * Poll only while at least one loaded match is result-pending. Whether a
 * match is result-pending is decided by the server (`Game.resultPending`,
 * `.specs/TESTING.md` §3.3) — the SPA no longer computes time.
 */
export function pollIntervalMs(games: SingleGame[]): number {
  return games.some((g) => g.resultPending) ? 30_000 : 0
}
