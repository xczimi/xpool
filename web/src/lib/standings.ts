import type { GroupGame, MatchPrediction, SingleGame } from '../graphql/types'

export interface TeamStats {
  teamId: string
  played: number
  won: number
  drawn: number
  lost: number
  goalsFor: number
  goalsAgainst: number
  points: number
}

/** Goal difference. */
export function goalDiff(s: TeamStats): number {
  return s.goalsFor - s.goalsAgainst
}

/**
 * Compute the standings a set of match scores implies, ranked by the
 * score-derivable part of the ladder (SCORING.md §4): points, then all-matches
 * goal difference, then goals scored. Head-to-head and the manual `draw_order`
 * tiebreak are applied separately via `applyDrawOrder`.
 */
export function computeStandings(
  games: SingleGame[],
  scoreOf: (gameId: string) => { home: number; away: number } | null,
): TeamStats[] {
  const stats = new Map<string, TeamStats>()
  const ensure = (teamId: string): TeamStats => {
    let s = stats.get(teamId)
    if (!s) {
      s = {
        teamId,
        played: 0,
        won: 0,
        drawn: 0,
        lost: 0,
        goalsFor: 0,
        goalsAgainst: 0,
        points: 0,
      }
      stats.set(teamId, s)
    }
    return s
  }

  for (const game of games) {
    const home = game.home.teamId
    const away = game.away.teamId
    if (!home || !away) continue
    ensure(home)
    ensure(away)
    const score = scoreOf(game.id)
    if (!score) continue
    const h = stats.get(home)!
    const a = stats.get(away)!
    h.played += 1
    a.played += 1
    h.goalsFor += score.home
    h.goalsAgainst += score.away
    a.goalsFor += score.away
    a.goalsAgainst += score.home
    if (score.home > score.away) {
      h.won += 1
      h.points += 3
      a.lost += 1
    } else if (score.home < score.away) {
      a.won += 1
      a.points += 3
      h.lost += 1
    } else {
      h.drawn += 1
      a.drawn += 1
      h.points += 1
      a.points += 1
    }
  }

  return [...stats.values()].sort((x, y) => {
    if (y.points !== x.points) return y.points - x.points
    if (goalDiff(y) !== goalDiff(x)) return goalDiff(y) - goalDiff(x)
    return y.goalsFor - x.goalsFor
  })
}

/**
 * Re-order teams that are fully tied (same points, GD, goals) so they follow
 * the player's manual `drawOrder` (SCORING.md §4 step 5). Teams not in
 * `drawOrder` keep their computed order.
 */
export function applyDrawOrder(
  ranked: TeamStats[],
  drawOrder: string[],
): TeamStats[] {
  if (drawOrder.length === 0) return ranked
  const tieKey = (s: TeamStats) =>
    `${s.points}|${goalDiff(s)}|${s.goalsFor}`
  const result = [...ranked]
  let i = 0
  while (i < result.length) {
    let j = i
    while (j + 1 < result.length && tieKey(result[j + 1]) === tieKey(result[i])) {
      j += 1
    }
    if (j > i) {
      const group = result.slice(i, j + 1)
      group.sort((a, b) => {
        const ia = drawOrder.indexOf(a.teamId)
        const ib = drawOrder.indexOf(b.teamId)
        if (ia === -1 && ib === -1) return 0
        if (ia === -1) return 1
        if (ib === -1) return -1
        return ia - ib
      })
      result.splice(i, group.length, ...group)
    }
    i = j + 1
  }
  return result
}

/** Score map from a list of match predictions. */
export function predictionScoreMap(
  predictions: MatchPrediction[],
): Map<string, { home: number; away: number }> {
  const map = new Map<string, { home: number; away: number }>()
  for (const p of predictions) {
    map.set(p.gameId, { home: p.homeScore, away: p.awayScore })
  }
  return map
}

/** True when every match in a leaf group has a prediction. */
export function groupGames(
  group: GroupGame,
  allGames: SingleGame[],
): SingleGame[] {
  return allGames.filter((g) => group.childGameIds.includes(g.id))
}
