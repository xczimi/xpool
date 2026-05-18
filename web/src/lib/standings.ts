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
/** A zeroed `TeamStats` for a team that has not played yet. */
function emptyStats(teamId: string): TeamStats {
  return {
    teamId,
    played: 0,
    won: 0,
    drawn: 0,
    lost: 0,
    goalsFor: 0,
    goalsAgainst: 0,
    points: 0,
  }
}

/** Fold one match's goals (from this team's perspective) into a fresh stats. */
function recordMatch(s: TeamStats, scored: number, conceded: number): TeamStats {
  const win = scored > conceded
  const loss = scored < conceded
  const draw = scored === conceded
  return {
    ...s,
    played: s.played + 1,
    won: s.won + (win ? 1 : 0),
    drawn: s.drawn + (draw ? 1 : 0),
    lost: s.lost + (loss ? 1 : 0),
    goalsFor: s.goalsFor + scored,
    goalsAgainst: s.goalsAgainst + conceded,
    points: s.points + (win ? 3 : draw ? 1 : 0),
  }
}

export function computeStandings(
  games: SingleGame[],
  scoreOf: (gameId: string) => { home: number; away: number } | null,
): TeamStats[] {
  // Fold every game into a fresh map of fresh `TeamStats` objects — no input
  // or accumulator object is ever mutated in place.
  const stats = games.reduce((acc, game) => {
    const home = game.home.teamId
    const away = game.away.teamId
    if (!home || !away) return acc

    const withTeams = new Map(acc)
    if (!withTeams.has(home)) withTeams.set(home, emptyStats(home))
    if (!withTeams.has(away)) withTeams.set(away, emptyStats(away))

    const score = scoreOf(game.id)
    if (!score) return withTeams

    withTeams.set(
      home,
      recordMatch(withTeams.get(home)!, score.home, score.away),
    )
    withTeams.set(
      away,
      recordMatch(withTeams.get(away)!, score.away, score.home),
    )
    return withTeams
  }, new Map<string, TeamStats>())

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
