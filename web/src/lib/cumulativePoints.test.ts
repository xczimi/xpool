import { describe, expect, it } from 'vitest'
import type { Round, ScoreEntry } from '../graphql/types'
import { cumulativeSeries } from './cumulativePoints'

const entry = (stages: Array<[Round, number]>): ScoreEntry => ({
  playerId: 'p',
  nick: 'p',
  total: stages.reduce((n, [, v]) => n + v, 0),
  stages: stages.map(([round, points]) => ({ round, points })),
})

describe('cumulativeSeries', () => {
  it('returns an empty series for no rounds', () => {
    expect(cumulativeSeries(entry([['GROUP_STAGE', 5]]), [])).toEqual([])
  })

  it('accumulates points across the supplied round order', () => {
    const e = entry([
      ['GROUP_STAGE', 5],
      ['R32', 4],
      ['R16', 6],
    ])
    const rounds: Round[] = ['GROUP_STAGE', 'R32', 'R16']
    expect(cumulativeSeries(e, rounds)).toEqual([
      { round: 'GROUP_STAGE', points: 5, cumulative: 5 },
      { round: 'R32', points: 4, cumulative: 9 },
      { round: 'R16', points: 6, cumulative: 15 },
    ])
  })

  it('treats rounds absent from the entry as zero points', () => {
    const e = entry([['GROUP_STAGE', 3]])
    const rounds: Round[] = ['GROUP_STAGE', 'R32', 'R16']
    expect(cumulativeSeries(e, rounds)).toEqual([
      { round: 'GROUP_STAGE', points: 3, cumulative: 3 },
      { round: 'R32', points: 0, cumulative: 3 },
      { round: 'R16', points: 0, cumulative: 3 },
    ])
  })

  it('honours the caller-supplied round order, not the stage order', () => {
    const e = entry([
      ['R16', 6],
      ['GROUP_STAGE', 5],
    ])
    const rounds: Round[] = ['GROUP_STAGE', 'R16']
    expect(cumulativeSeries(e, rounds).map((p) => p.cumulative)).toEqual([5, 11])
  })
})
