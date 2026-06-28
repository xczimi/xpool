import { describe, expect, it } from 'vitest'
import type { PlayerTimeline } from '../graphql/types'
import { buildSeries, pickTickIndices } from './timeline'

const pt = (cumulative: number) => ({
  gameId: `g${cumulative}`,
  kickoff: '2026-06-11T18:00:00Z',
  points: cumulative,
  cumulative,
})

const timeline = (playerId: string, ...cum: number[]): PlayerTimeline => ({
  playerId,
  nick: playerId,
  points: cum.map(pt),
})

describe('pickTickIndices', () => {
  it('returns no ticks for an empty axis', () => {
    expect(pickTickIndices(0)).toEqual([])
  })

  it('returns the single index for a one-game axis', () => {
    expect(pickTickIndices(1)).toEqual([0])
  })

  it('always includes the first and last index', () => {
    const ticks = pickTickIndices(20)
    expect(ticks[0]).toBe(0)
    expect(ticks[ticks.length - 1]).toBe(19)
  })

  it('caps the number of ticks (sparse labels)', () => {
    expect(pickTickIndices(50, 6).length).toBeLessThanOrEqual(6)
  })

  it('keeps every index when there are fewer than the cap', () => {
    expect(pickTickIndices(4, 6)).toEqual([0, 1, 2, 3])
  })
})

describe('buildSeries', () => {
  const timelines = [timeline('a', 1, 2), timeline('b', 0, 5), timeline('c', 3)]

  it('selects and orders by the requested ids, assigning colours by position', () => {
    const series = buildSeries(timelines, ['b', 'a'])
    expect(series.map((s) => s.label)).toEqual(['b', 'a'])
    expect(series[0].color).not.toBe(series[1].color)
    expect(series[0].points.map((p) => p.cumulative)).toEqual([0, 5])
  })

  it('returns all timelines (board order) when ids is null', () => {
    const series = buildSeries(timelines, null)
    expect(series.map((s) => s.label)).toEqual(['a', 'b', 'c'])
  })

  it('skips ids absent from the timelines', () => {
    const series = buildSeries(timelines, ['a', 'missing'])
    expect(series.map((s) => s.label)).toEqual(['a'])
  })
})
