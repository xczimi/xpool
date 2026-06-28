import { describe, it, expect, beforeEach } from 'vitest'
import {
  sortRows,
  nextSort,
  readMatchSort,
  writeMatchSort,
  DEFAULT_MATCH_SORT,
  MATCH_SORT_KEY,
  type MatchSort,
} from './matchSort'
import type { Tip } from '../graphql/types'

function tip(over: Partial<Tip> & Pick<Tip, 'playerId' | 'nick'>): Tip {
  return {
    gameId: 'M1',
    prediction: null,
    points: null,
    isPerfect: false,
    breakdown: null,
    maxReachable: null,
    ...over,
  }
}

const ada = tip({ playerId: 'ada', nick: 'Ada', prediction: { gameId: 'M1', homeScore: 2, awayScore: 1, locked: true }, points: 1 })
const bob = tip({ playerId: 'bob', nick: 'Bob', prediction: { gameId: 'M1', homeScore: 1, awayScore: 0, locked: true }, points: 4 })
const cyd = tip({ playerId: 'cyd', nick: 'Cyd', prediction: null, points: null }) // hidden tip
const rows: Tip[] = [bob, ada, cyd] // server order

describe('sortRows', () => {
  it('standing (default) preserves server order', () => {
    expect(sortRows(rows, { column: 'standing', direction: 'asc' }).map((r) => r.playerId)).toEqual([
      'bob',
      'ada',
      'cyd',
    ])
  })

  it('sorts by player name ascending', () => {
    expect(sortRows(rows, { column: 'player', direction: 'asc' }).map((r) => r.playerId)).toEqual([
      'ada',
      'bob',
      'cyd',
    ])
  })

  it('sorts by player name descending', () => {
    expect(sortRows(rows, { column: 'player', direction: 'desc' }).map((r) => r.playerId)).toEqual([
      'cyd',
      'bob',
      'ada',
    ])
  })

  it('sorts by prediction with hidden tips always last', () => {
    // asc by home then away: bob 1-0 before ada 2-1; cyd (null) last
    expect(sortRows(rows, { column: 'prediction', direction: 'asc' }).map((r) => r.playerId)).toEqual([
      'bob',
      'ada',
      'cyd',
    ])
    // desc keeps hidden last too
    expect(sortRows(rows, { column: 'prediction', direction: 'desc' }).map((r) => r.playerId)).toEqual([
      'ada',
      'bob',
      'cyd',
    ])
  })

  it('sorts by points with nulls always last', () => {
    expect(sortRows(rows, { column: 'points', direction: 'desc' }).map((r) => r.playerId)).toEqual([
      'bob',
      'ada',
      'cyd',
    ])
    expect(sortRows(rows, { column: 'points', direction: 'asc' }).map((r) => r.playerId)).toEqual([
      'ada',
      'bob',
      'cyd',
    ])
  })

  it('sorts by max-reachable with nulls always last', () => {
    // ada ceiling 4, bob ceiling 7, cyd none (null) → sinks regardless.
    const aMax = tip({ playerId: 'ada', nick: 'Ada', maxReachable: 4 })
    const bMax = tip({ playerId: 'bob', nick: 'Bob', maxReachable: 7 })
    const cMax = tip({ playerId: 'cyd', nick: 'Cyd', maxReachable: null })
    const maxRows: Tip[] = [aMax, bMax, cMax]
    expect(sortRows(maxRows, { column: 'max', direction: 'desc' }).map((r) => r.playerId)).toEqual([
      'bob',
      'ada',
      'cyd',
    ])
    expect(sortRows(maxRows, { column: 'max', direction: 'asc' }).map((r) => r.playerId)).toEqual([
      'ada',
      'bob',
      'cyd',
    ])
  })

  it('does not mutate the input array', () => {
    const input = [...rows]
    sortRows(input, { column: 'player', direction: 'asc' })
    expect(input.map((r) => r.playerId)).toEqual(['bob', 'ada', 'cyd'])
  })
})

describe('nextSort', () => {
  it('toggles direction when the column is unchanged', () => {
    expect(nextSort({ column: 'player', direction: 'asc' }, 'player')).toEqual({
      column: 'player',
      direction: 'desc',
    })
  })

  it('uses the column default direction when switching columns', () => {
    expect(nextSort({ column: 'player', direction: 'asc' }, 'points')).toEqual({
      column: 'points',
      direction: 'desc',
    })
    expect(nextSort({ column: 'points', direction: 'desc' }, 'player')).toEqual({
      column: 'player',
      direction: 'asc',
    })
    expect(nextSort({ column: 'player', direction: 'asc' }, 'max')).toEqual({
      column: 'max',
      direction: 'desc',
    })
  })
})

describe('read/writeMatchSort', () => {
  beforeEach(() => localStorage.clear())

  it('returns the default when nothing is stored', () => {
    expect(readMatchSort()).toEqual(DEFAULT_MATCH_SORT)
  })

  it('round-trips a stored sort', () => {
    const sort: MatchSort = { column: 'points', direction: 'asc' }
    writeMatchSort(sort)
    expect(localStorage.getItem(MATCH_SORT_KEY)).toBeTruthy()
    expect(readMatchSort()).toEqual(sort)
  })

  it('falls back to the default on malformed storage', () => {
    localStorage.setItem(MATCH_SORT_KEY, 'not json')
    expect(readMatchSort()).toEqual(DEFAULT_MATCH_SORT)
  })
})
