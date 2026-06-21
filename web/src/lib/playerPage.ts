import type { Perfect, Pool, Round, ScoreEntry } from '../graphql/types'

/** Scoreboard entries ranked by total points, descending. Returns a new array. */
export function rankedScoreboard(scoreboard: ScoreEntry[]): ScoreEntry[] {
  return [...scoreboard].sort((a, b) => b.total - a.total)
}

/**
 * The first pool the viewer shares with `playerId`. The `pools` query returns
 * only the viewer's own pools, so a hit means both belong to that pool. Checks
 * EVERY pool the viewer is in, not just the first — the gate for viewing a
 * player's detail page is "you share at least one pool with them". `undefined`
 * when they share none.
 */
export function sharedPoolWith(
  pools: Pool[],
  playerId: string,
): Pool | undefined {
  return pools.find((p) => p.members.includes(playerId))
}

/** A player's scoreboard entry, or null if absent (e.g. not a pool-mate). */
export function playerEntry(
  scoreboard: ScoreEntry[],
  playerId: string,
): ScoreEntry | null {
  return scoreboard.find((e) => e.playerId === playerId) ?? null
}

/** 1-based rank of a player within the total-desc scoreboard, or null. */
export function playerRank(
  scoreboard: ScoreEntry[],
  playerId: string,
): number | null {
  const idx = rankedScoreboard(scoreboard).findIndex(
    (e) => e.playerId === playerId,
  )
  return idx === -1 ? null : idx + 1
}

/** Per-round points for an entry, as a lookup map. */
export function roundPointsOf(entry: ScoreEntry): Map<Round, number> {
  return new Map(entry.stages.map((s) => [s.round, s.points]))
}

/** A single player's perfect predictions. */
export function perfectsOf(perfects: Perfect[], playerId: string): Perfect[] {
  return perfects.filter((p) => p.playerId === playerId)
}
