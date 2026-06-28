import type { Round, ScoreEntry, Tip } from '../graphql/types'
import { playerEntry, playerRank } from './playerPage'

export interface H2HSummary {
  a: ScoreEntry
  b: ScoreEntry
  rankA: number | null
  rankB: number | null
  /** a.total - b.total (positive ⇒ a is ahead). */
  totalDelta: number
}

/**
 * Resolve both sides from the materialised scoreboard. Returns null if either
 * is absent (e.g. a participant with no scored entry yet). Pure.
 */
export function h2hSummary(
  scoreboard: ScoreEntry[],
  idA: string,
  idB: string,
): H2HSummary | null {
  const a = playerEntry(scoreboard, idA)
  const b = playerEntry(scoreboard, idB)
  if (!a || !b) return null
  return {
    a,
    b,
    rankA: playerRank(scoreboard, idA),
    rankB: playerRank(scoreboard, idB),
    totalDelta: a.total - b.total,
  }
}

export interface RoundDelta {
  round: Round
  pointsA: number
  pointsB: number
  /** pointsA - pointsB. */
  delta: number
}

/** Per-round point comparison over the supplied ordered rounds. Pure. */
export function roundDeltas(
  a: ScoreEntry,
  b: ScoreEntry,
  rounds: Round[],
): RoundDelta[] {
  const ma = new Map(a.stages.map((s) => [s.round, s.points]))
  const mb = new Map(b.stages.map((s) => [s.round, s.points]))
  return rounds.map((round) => {
    const pointsA = ma.get(round) ?? 0
    const pointsB = mb.get(round) ?? 0
    return { round, pointsA, pointsB, delta: pointsA - pointsB }
  })
}

export interface ScoreCell {
  homeScore: number
  awayScore: number
}

export interface MatchDiff {
  gameId: string
  /** null when this player's pick is gated-hidden by the server. */
  predA: ScoreCell | null
  predB: ScoreCell | null
  pointsA: number | null
  pointsB: number | null
  /** true when the server withheld the prediction (Tip.prediction === null). */
  hiddenA: boolean
  hiddenB: boolean
}

/**
 * Per-match comparison for two players from a round's `tips`. The TIPS_QUERY
 * result already applies hidden-until-revealable gating: a withheld prediction
 * arrives as null, so this never branches on a clock. Only rows where the two
 * predictions OR their points differ are returned — the "where they differ"
 * view — except that a row with either side gated-hidden is always kept so the
 * gate stays visible rather than being silently dropped. Pure.
 */
export function matchDiffs(tips: Tip[], idA: string, idB: string): MatchDiff[] {
  const aByGame = new Map<string, Tip>()
  const bByGame = new Map<string, Tip>()
  for (const t of tips) {
    if (t.playerId === idA) aByGame.set(t.gameId, t)
    if (t.playerId === idB) bByGame.set(t.gameId, t)
  }
  const gameIds = [...new Set([...aByGame.keys(), ...bByGame.keys()])]
  const rows: MatchDiff[] = []
  for (const gameId of gameIds) {
    const ta = aByGame.get(gameId) ?? null
    const tb = bByGame.get(gameId) ?? null
    const predA = ta?.prediction
      ? { homeScore: ta.prediction.homeScore, awayScore: ta.prediction.awayScore }
      : null
    const predB = tb?.prediction
      ? { homeScore: tb.prediction.homeScore, awayScore: tb.prediction.awayScore }
      : null
    const hiddenA = ta != null && ta.prediction == null
    const hiddenB = tb != null && tb.prediction == null
    const samePred =
      predA != null &&
      predB != null &&
      predA.homeScore === predB.homeScore &&
      predA.awayScore === predB.awayScore
    const samePoints = (ta?.points ?? null) === (tb?.points ?? null)
    if (samePred && samePoints && !hiddenA && !hiddenB) continue
    rows.push({
      gameId,
      predA,
      predB,
      pointsA: ta?.points ?? null,
      pointsB: tb?.points ?? null,
      hiddenA,
      hiddenB,
    })
  }
  return rows
}
