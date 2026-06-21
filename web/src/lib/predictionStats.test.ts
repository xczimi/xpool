import { describe, expect, it } from 'vitest'
import type { MatchScore, Tip } from '../graphql/types'
import { computePredictionStats } from './predictionStats'

/** Build a Tip with a visible prediction (helper for terse fixtures). */
function tip(playerId: string, home: number, away: number): Tip {
  return {
    playerId,
    nick: playerId,
    gameId: 'g1',
    prediction: { gameId: 'g1', homeScore: home, awayScore: away, locked: true },
    points: null,
    isPerfect: false,
    breakdown: null,
    maxReachable: null,
  }
}

/** A Tip whose prediction is still hidden (server gate not yet open). */
function hiddenTip(playerId: string): Tip {
  return {
    playerId,
    nick: playerId,
    gameId: 'g1',
    prediction: null,
    points: null,
    isPerfect: false,
    breakdown: null,
    maxReachable: null,
  }
}

const finalScore: MatchScore = {
  homeScore: 2,
  awayScore: 1,
  provisional: false,
  source: null,
  sourceStatus: null,
  ninetyMinuteUncertain: false,
}

describe('computePredictionStats', () => {
  it('returns null when no predictions are visible (gate closed)', () => {
    const stats = computePredictionStats([hiddenTip('a'), hiddenTip('b')], null)
    expect(stats).toBeNull()
  })

  it('counts the most common scoreline and reports its count', () => {
    const rows = [tip('a', 2, 1), tip('b', 2, 1), tip('c', 1, 0)]
    const stats = computePredictionStats(rows, null)
    expect(stats).not.toBeNull()
    expect(stats!.total).toBe(3)
    expect(stats!.mostCommon).toEqual([{ homeScore: 2, awayScore: 1, count: 2 }])
  })

  it('reports all scorelines tied for most common', () => {
    const rows = [tip('a', 2, 1), tip('b', 1, 0)]
    const stats = computePredictionStats(rows, null)!
    // Both appear once → both are "most common"; order is by descending count
    // then home-major, away-major for determinism.
    expect(stats.mostCommon).toEqual([
      { homeScore: 1, awayScore: 0, count: 1 },
      { homeScore: 2, awayScore: 1, count: 1 },
    ])
  })

  it('splits outcomes into home / draw / away', () => {
    const rows = [
      tip('a', 2, 1), // home
      tip('b', 3, 0), // home
      tip('c', 1, 1), // draw
      tip('d', 0, 2), // away
    ]
    const stats = computePredictionStats(rows, null)!
    expect(stats.outcomeSplit).toEqual({ home: 2, draw: 1, away: 1 })
  })

  it('ignores hidden predictions when aggregating', () => {
    const rows = [tip('a', 2, 1), hiddenTip('b'), tip('c', 2, 1)]
    const stats = computePredictionStats(rows, null)!
    expect(stats.total).toBe(2)
    expect(stats.mostCommon).toEqual([{ homeScore: 2, awayScore: 1, count: 2 }])
  })

  it('counts how many nailed a FINAL result', () => {
    const rows = [tip('a', 2, 1), tip('b', 2, 1), tip('c', 0, 0)]
    const stats = computePredictionStats(rows, finalScore)!
    expect(stats.nailedIt).toBe(2)
  })

  it('does not count nailed-it for a provisional (live) score', () => {
    const rows = [tip('a', 2, 1)]
    const provisional: MatchScore = { ...finalScore, provisional: true }
    const stats = computePredictionStats(rows, provisional)!
    expect(stats.nailedIt).toBeNull()
  })

  it('reports nailedIt = 0 when nobody matched a final result', () => {
    const rows = [tip('a', 0, 0), tip('b', 3, 3)]
    const stats = computePredictionStats(rows, finalScore)!
    expect(stats.nailedIt).toBe(0)
  })
})
