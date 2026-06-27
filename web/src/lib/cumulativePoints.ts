import type { Round, ScoreEntry } from '../graphql/types'

/** One x-axis point: a round, its points, and the running total through it. */
export interface CumulativePoint {
  round: Round
  points: number
  cumulative: number
}

/**
 * Running cumulative points for a scoreboard entry over an ordered list of
 * rounds. `rounds` is supplied by the caller (ROUND_ORDER filtered by
 * readyRounds), so the series never reaches past the server-derived horizon —
 * there is no Date.now() here. Rounds absent from the entry's stages
 * contribute 0. Pure and immutable.
 */
export function cumulativeSeries(
  entry: ScoreEntry,
  rounds: Round[],
): CumulativePoint[] {
  const byRound = new Map(entry.stages.map((s) => [s.round, s.points]))
  let running = 0
  return rounds.map((round) => {
    const points = byRound.get(round) ?? 0
    running += points
    return { round, points, cumulative: running }
  })
}
