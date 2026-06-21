import { describe, expect, it } from 'vitest'
import type { Perfect } from '../graphql/types'
import { orderPerfects, type PerfectView } from './perfectOrder'

const bd = {
  exactHome: true,
  exactAway: true,
  outcome: true,
  base: 4,
  multiplier: 1,
  points: 4,
}
const p = (playerId: string, nick: string, gameId: string): Perfect => ({
  playerId,
  nick,
  gameId,
  points: 4,
  breakdown: bd,
})

// ada: 2 perfects (g1, g3); bob: 1 perfect (g2). Kickoffs g1<g2<g3.
const list: Perfect[] = [p('bob', 'Bob', 'g2'), p('ada', 'Ada', 'g3'), p('ada', 'Ada', 'g1')]
const kickoff = new Map<string, number>([
  ['g1', 100],
  ['g2', 200],
  ['g3', 300],
])

describe('orderPerfects by-match (default)', () => {
  it('orders by kickoff asc, tie-broken by nick', () => {
    const out = orderPerfects(list, 'match', kickoff)
    expect(out.map((x) => x.gameId)).toEqual(['g1', 'g2', 'g3'])
  })
  it('does not mutate the input', () => {
    const copy = [...list]
    orderPerfects(list, 'match', kickoff)
    expect(list).toEqual(copy)
  })
})

describe('orderPerfects by-player', () => {
  it('groups each player contiguously, players by perfect-count desc', () => {
    const out = orderPerfects(list, 'player', kickoff)
    // ada (2 perfects) before bob (1); ada's own perfects kickoff-ordered.
    expect(out.map((x) => `${x.playerId}:${x.gameId}`)).toEqual([
      'ada:g1',
      'ada:g3',
      'bob:g2',
    ])
  })
  it('breaks count ties by first appearance (stable)', () => {
    // cy and dan each have 1 perfect; cy appears first in the input.
    const tied: Perfect[] = [p('cy', 'Cy', 'g2'), p('dan', 'Dan', 'g1')]
    const out = orderPerfects(tied, 'player', kickoff)
    expect(out.map((x) => x.playerId)).toEqual(['cy', 'dan'])
  })
})

// `PerfectView` is exercised through `orderPerfects`; reference it so the type
// import is not flagged unused by the linter.
const _view: PerfectView = 'match'
void _view
