import { describe, expect, it } from 'vitest'
import type { GroupGame, MatchPrediction, SingleGame } from '../graphql/types'
import {
  applyDrawOrder,
  computeStandings,
  goalDiff,
  groupGames,
  isKnockoutMatch,
  predictionScoreMap,
  type TeamStats,
} from './standings'

function game(id: string, home: string, away: string): SingleGame {
  return {
    id,
    home: { teamId: home, description: null },
    away: { teamId: away, description: null },
  } as unknown as SingleGame
}

function stats(over: Partial<TeamStats>): TeamStats {
  return {
    teamId: 't',
    played: 0,
    won: 0,
    drawn: 0,
    lost: 0,
    goalsFor: 0,
    goalsAgainst: 0,
    points: 0,
    ...over,
  }
}

describe('isKnockoutMatch', () => {
  it('is true for a one-game leaf group (a two-team knockout tie)', () => {
    expect(isKnockoutMatch([game('g1', 'a', 'b')])).toBe(true)
  })

  it('is false for a multi-game round-robin group', () => {
    expect(
      isKnockoutMatch([
        game('g1', 'a', 'b'),
        game('g2', 'a', 'c'),
        game('g3', 'b', 'c'),
      ]),
    ).toBe(false)
  })

  it('is false for an empty group', () => {
    expect(isKnockoutMatch([])).toBe(false)
  })
})

describe('goalDiff', () => {
  it('is goalsFor minus goalsAgainst', () => {
    expect(goalDiff(stats({ goalsFor: 5, goalsAgainst: 2 }))).toBe(3)
    expect(goalDiff(stats({ goalsFor: 1, goalsAgainst: 4 }))).toBe(-3)
  })
})

describe('computeStandings', () => {
  // A 3-team group: a beats b, a draws c, b beats c.
  const games = [game('g1', 'a', 'b'), game('g2', 'a', 'c'), game('g3', 'b', 'c')]
  const scores: Record<string, { home: number; away: number }> = {
    g1: { home: 2, away: 0 },
    g2: { home: 1, away: 1 },
    g3: { home: 3, away: 1 },
  }
  const scoreOf = (id: string) => scores[id] ?? null

  it('ranks by points, then goal difference, then goals scored', () => {
    const table = computeStandings(games, scoreOf)
    expect(table.map((s) => s.teamId)).toEqual(['a', 'b', 'c'])
  })

  it('accumulates wins, draws, losses and points correctly', () => {
    const table = computeStandings(games, scoreOf)
    const a = table.find((s) => s.teamId === 'a')!
    expect(a).toMatchObject({
      played: 2,
      won: 1,
      drawn: 1,
      lost: 0,
      goalsFor: 3,
      goalsAgainst: 1,
      points: 4,
    })
    const c = table.find((s) => s.teamId === 'c')!
    expect(c).toMatchObject({
      played: 2,
      won: 0,
      drawn: 1,
      lost: 1,
      points: 1,
    })
  })

  it('includes teams with zero played matches', () => {
    const table = computeStandings(games, () => null)
    expect(table).toHaveLength(3)
    expect(table.every((s) => s.played === 0 && s.points === 0)).toBe(true)
  })

  it('skips games whose slots have no team assigned', () => {
    const partial = [
      game('g1', 'a', 'b'),
      { id: 'g2', home: { teamId: null }, away: { teamId: 'c' } } as unknown as SingleGame,
    ]
    const table = computeStandings(partial, () => ({ home: 1, away: 0 }))
    expect(table.map((s) => s.teamId).sort()).toEqual(['a', 'b'])
  })

  it('ranks an away win above the home side', () => {
    const table = computeStandings([game('g1', 'a', 'b')], () => ({
      home: 0,
      away: 2,
    }))
    expect(table.map((s) => s.teamId)).toEqual(['b', 'a'])
  })

  it('does not mutate the input games', () => {
    const input = [game('g1', 'a', 'b')]
    const snapshot = JSON.stringify(input)
    computeStandings(input, () => ({ home: 1, away: 0 }))
    expect(JSON.stringify(input)).toBe(snapshot)
  })

  it('breaks ties on goals scored when points and GD are equal', () => {
    // a and b: both 1 win, GD +1, but a scored more.
    const tieGames = [game('g1', 'a', 'x'), game('g2', 'b', 'y')]
    const tieScores: Record<string, { home: number; away: number }> = {
      g1: { home: 3, away: 2 },
      g2: { home: 1, away: 0 },
    }
    const table = computeStandings(tieGames, (id) => tieScores[id] ?? null)
    expect(table[0].teamId).toBe('a')
  })
})

describe('applyDrawOrder', () => {
  it('returns the input unchanged when drawOrder is empty', () => {
    const ranked = [stats({ teamId: 'a' }), stats({ teamId: 'b' })]
    expect(applyDrawOrder(ranked, [])).toEqual(ranked)
  })

  it('reorders fully-tied teams to follow the manual drawOrder', () => {
    const ranked = [
      stats({ teamId: 'a', points: 3 }),
      stats({ teamId: 'b', points: 3 }),
      stats({ teamId: 'c', points: 3 }),
    ]
    const out = applyDrawOrder(ranked, ['c', 'a', 'b'])
    expect(out.map((s) => s.teamId)).toEqual(['c', 'a', 'b'])
  })

  it('keeps non-tied teams in their computed order', () => {
    const ranked = [
      stats({ teamId: 'a', points: 6 }),
      stats({ teamId: 'b', points: 3 }),
      stats({ teamId: 'c', points: 3 }),
    ]
    const out = applyDrawOrder(ranked, ['c', 'b'])
    expect(out.map((s) => s.teamId)).toEqual(['a', 'c', 'b'])
  })

  it('sorts teams missing from drawOrder after listed ones', () => {
    const ranked = [
      stats({ teamId: 'a', points: 3 }),
      stats({ teamId: 'b', points: 3 }),
      stats({ teamId: 'c', points: 3 }),
    ]
    const out = applyDrawOrder(ranked, ['c'])
    expect(out[0].teamId).toBe('c')
  })

  it('does not mutate the input array', () => {
    const ranked = [
      stats({ teamId: 'a', points: 3 }),
      stats({ teamId: 'b', points: 3 }),
    ]
    const snapshot = ranked.map((s) => s.teamId)
    applyDrawOrder(ranked, ['b', 'a'])
    expect(ranked.map((s) => s.teamId)).toEqual(snapshot)
  })
})

describe('predictionScoreMap', () => {
  it('maps gameId to home/away scores', () => {
    const preds = [
      { gameId: 'g1', homeScore: 2, awayScore: 1 },
      { gameId: 'g2', homeScore: 0, awayScore: 0 },
    ] as MatchPrediction[]
    const map = predictionScoreMap(preds)
    expect(map.get('g1')).toEqual({ home: 2, away: 1 })
    expect(map.get('g2')).toEqual({ home: 0, away: 0 })
  })

  it('is empty for no predictions', () => {
    expect(predictionScoreMap([]).size).toBe(0)
  })
})

describe('groupGames', () => {
  it('returns only the games referenced by the group', () => {
    const group = { childGameIds: ['g1', 'g3'] } as GroupGame
    const all = [game('g1', 'a', 'b'), game('g2', 'c', 'd'), game('g3', 'e', 'f')]
    expect(groupGames(group, all).map((g) => g.id)).toEqual(['g1', 'g3'])
  })

  it('is empty when the group references no loaded games', () => {
    const group = { childGameIds: ['missing'] } as GroupGame
    expect(groupGames(group, [game('g1', 'a', 'b')])).toEqual([])
  })
})
