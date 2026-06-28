import { describe, expect, it } from 'vitest'
import type { Round, ScoreEntry, Tip } from '../graphql/types'
import { h2hSummary, matchDiffs, roundDeltas } from './headToHead'

const entry = (
  playerId: string,
  total: number,
  stages: Array<[Round, number]>,
): ScoreEntry => ({
  playerId,
  nick: playerId,
  total,
  stages: stages.map(([round, points]) => ({ round, points })),
})

const tip = (
  playerId: string,
  gameId: string,
  pred: { homeScore: number; awayScore: number } | null,
  points: number | null,
): Tip => ({
  playerId,
  nick: playerId,
  gameId,
  prediction: pred ? { gameId, ...pred, locked: true } : null,
  points,
  isPerfect: false,
  breakdown: null,
  maxReachable: null,
})

describe('h2hSummary', () => {
  it('returns null when either player is missing from the board', () => {
    const board = [entry('a', 10, [])]
    expect(h2hSummary(board, 'a', 'b')).toBeNull()
  })

  it('computes ranks and the a-minus-b total delta', () => {
    const board = [
      entry('a', 10, []),
      entry('b', 7, []),
      entry('c', 20, []),
    ]
    const s = h2hSummary(board, 'a', 'b')
    expect(s?.totalDelta).toBe(3)
    expect(s?.rankA).toBe(2)
    expect(s?.rankB).toBe(3)
  })
})

describe('roundDeltas', () => {
  it('subtracts per-round points over the supplied rounds, zero-filling gaps', () => {
    const a = entry('a', 9, [
      ['GROUP_STAGE', 5],
      ['R32', 4],
    ])
    const b = entry('b', 6, [['GROUP_STAGE', 6]])
    const rounds: Round[] = ['GROUP_STAGE', 'R32']
    expect(roundDeltas(a, b, rounds)).toEqual([
      { round: 'GROUP_STAGE', pointsA: 5, pointsB: 6, delta: -1 },
      { round: 'R32', pointsA: 4, pointsB: 0, delta: 4 },
    ])
  })
})

describe('matchDiffs', () => {
  it('omits matches where predictions and points are identical', () => {
    const tips = [
      tip('a', 'g1', { homeScore: 1, awayScore: 0 }, 4),
      tip('b', 'g1', { homeScore: 1, awayScore: 0 }, 4),
    ]
    expect(matchDiffs(tips, 'a', 'b')).toEqual([])
  })

  it('keeps matches where the predictions differ', () => {
    const tips = [
      tip('a', 'g1', { homeScore: 1, awayScore: 0 }, 4),
      tip('b', 'g1', { homeScore: 2, awayScore: 2 }, 0),
    ]
    const rows = matchDiffs(tips, 'a', 'b')
    expect(rows).toHaveLength(1)
    expect(rows[0]).toMatchObject({
      gameId: 'g1',
      predA: { homeScore: 1, awayScore: 0 },
      predB: { homeScore: 2, awayScore: 2 },
      pointsA: 4,
      pointsB: 0,
      hiddenA: false,
      hiddenB: false,
    })
  })

  it('always keeps a row when one side is gated-hidden', () => {
    const tips = [
      tip('a', 'g1', { homeScore: 1, awayScore: 0 }, null),
      tip('b', 'g1', null, null),
    ]
    const rows = matchDiffs(tips, 'a', 'b')
    expect(rows).toHaveLength(1)
    expect(rows[0].hiddenB).toBe(true)
    expect(rows[0].predB).toBeNull()
  })
})
