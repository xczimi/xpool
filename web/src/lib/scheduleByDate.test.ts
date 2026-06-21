import { describe, expect, it } from 'vitest'
import type { SingleGame } from '../graphql/types'
import { dayKey, groupByDay } from './scheduleByDate'

const game = (id: string, kickoff: string): SingleGame =>
  ({ id, kickoff }) as SingleGame

describe('dayKey', () => {
  it('returns the same key for two kickoffs on the same local day', () => {
    // Same local calendar day in UTC (used by the CI/runner tz-independently:
    // both are 2026-06-20 in UTC).
    const a = dayKey('2026-06-20T09:00:00Z', 'en')
    const b = dayKey('2026-06-20T21:00:00Z', 'en')
    expect(a).toBe(b)
  })

  it('returns different keys for kickoffs on different local days', () => {
    const a = dayKey('2026-06-20T12:00:00Z', 'en')
    const b = dayKey('2026-06-21T12:00:00Z', 'en')
    expect(a).not.toBe(b)
  })

  it('returns a stable, non-empty key for a valid date', () => {
    const k = dayKey('2026-06-20T12:00:00Z', 'en')
    expect(k.length).toBeGreaterThan(0)
  })

  it('falls back to the raw string for an unparseable date', () => {
    expect(dayKey('not-a-date', 'en')).toBe('not-a-date')
  })
})

describe('groupByDay', () => {
  it('buckets games into one section per local calendar day', () => {
    const games = [
      game('a', '2026-06-20T12:00:00Z'),
      game('b', '2026-06-20T18:00:00Z'),
      game('c', '2026-06-21T12:00:00Z'),
    ]
    const sections = groupByDay(games, 'en')
    expect(sections).toHaveLength(2)
    expect(sections[0].games.map((g) => g.id)).toEqual(['a', 'b'])
    expect(sections[1].games.map((g) => g.id)).toEqual(['c'])
  })

  it('orders sections chronologically and games within a section by kickoff', () => {
    const games = [
      game('c', '2026-06-21T12:00:00Z'),
      game('b', '2026-06-20T18:00:00Z'),
      game('a', '2026-06-20T12:00:00Z'),
    ]
    const sections = groupByDay(games, 'en')
    expect(sections.map((s) => s.games.map((g) => g.id))).toEqual([
      ['a', 'b'],
      ['c'],
    ])
  })

  it('gives each section a stable key and a human label', () => {
    const sections = groupByDay([game('a', '2026-06-20T12:00:00Z')], 'en')
    expect(sections[0].key.length).toBeGreaterThan(0)
    expect(sections[0].label.length).toBeGreaterThan(0)
  })

  it('does not mutate the input array', () => {
    const games = [
      game('b', '2026-06-20T18:00:00Z'),
      game('a', '2026-06-20T12:00:00Z'),
    ]
    const before = games.map((g) => g.id)
    groupByDay(games, 'en')
    expect(games.map((g) => g.id)).toEqual(before)
  })

  it('returns an empty list for no games', () => {
    expect(groupByDay([], 'en')).toEqual([])
  })
})
