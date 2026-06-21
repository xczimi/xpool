import { describe, expect, it } from 'vitest'
import type { Perfect, ScoreEntry } from '../graphql/types'
import {
  perfectsOf,
  playerEntry,
  playerRank,
  rankedScoreboard,
  roundPointsOf,
} from './playerPage'

const board: ScoreEntry[] = [
  { playerId: 'a', nick: 'Ada', total: 10, maxAchievable: null, stages: [{ round: 'GROUP_STAGE', points: 6 }, { round: 'R16', points: 4 }] },
  { playerId: 'b', nick: 'Bob', total: 25, maxAchievable: null, stages: [{ round: 'GROUP_STAGE', points: 25 }] },
  { playerId: 'c', nick: 'Cy', total: 25, maxAchievable: null, stages: [] },
]

describe('rankedScoreboard', () => {
  it('sorts by total descending without mutating the input', () => {
    const copy = [...board]
    const ranked = rankedScoreboard(board)
    expect(ranked.map((e) => e.playerId)).toEqual(['b', 'c', 'a'])
    expect(board).toEqual(copy)
  })
})

describe('playerEntry', () => {
  it('finds the entry by id', () => {
    expect(playerEntry(board, 'a')?.nick).toBe('Ada')
  })
  it('returns null when the player is absent (not a pool-mate)', () => {
    expect(playerEntry(board, 'zzz')).toBeNull()
  })
})

describe('playerRank', () => {
  it('is 1-based over the total-desc order', () => {
    expect(playerRank(board, 'b')).toBe(1)
    expect(playerRank(board, 'a')).toBe(3)
  })
  it('returns null for an absent player', () => {
    expect(playerRank(board, 'zzz')).toBeNull()
  })
})

describe('roundPointsOf', () => {
  it('maps each round to its points', () => {
    const m = roundPointsOf(board[0])
    expect(m.get('GROUP_STAGE')).toBe(6)
    expect(m.get('R16')).toBe(4)
    expect(m.get('FINAL')).toBeUndefined()
  })
})

describe('perfectsOf', () => {
  it('keeps only the given player', () => {
    const perfects: Perfect[] = [
      { playerId: 'a', nick: 'Ada', gameId: 'g1', points: 4, breakdown: { exactHome: true, exactAway: true, outcome: true, base: 4, multiplier: 1, points: 4 } },
      { playerId: 'b', nick: 'Bob', gameId: 'g1', points: 4, breakdown: { exactHome: true, exactAway: true, outcome: true, base: 4, multiplier: 1, points: 4 } },
    ]
    expect(perfectsOf(perfects, 'a').map((p) => p.playerId)).toEqual(['a'])
  })
})
