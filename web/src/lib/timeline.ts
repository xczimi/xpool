import type { PlayerTimeline, TimelinePoint } from '../graphql/types'
import { seriesColor } from '../components/timelineColors'

/** One overlaid line: a label, a stroke colour, and its cumulative points. */
export interface TimelineSeries {
  label: string
  color: string
  points: TimelinePoint[]
}

/**
 * Indices to label on the game-by-game x-axis. A label per game would be
 * unreadable for a 104-match tournament, so we show at most `max` evenly-spaced
 * ticks, always including the first and last. Pure.
 */
export function pickTickIndices(n: number, max = 6): number[] {
  if (n <= 0) return []
  if (n <= max) return Array.from({ length: n }, (_, i) => i)
  const step = (n - 1) / (max - 1)
  const set = new Set<number>()
  for (let i = 0; i < max; i++) set.add(Math.round(i * step))
  return [...set].sort((a, b) => a - b)
}

/**
 * Build chart series from the resolver's timelines. `ids` picks and orders the
 * players (e.g. `[a, b]` for head-to-head, `[ownerId]` for a single line);
 * `null` keeps every timeline in board order. Colours are assigned by position
 * so overlays stay distinct. Ids absent from the data are skipped. Pure.
 */
export function buildSeries(
  timelines: PlayerTimeline[],
  ids: string[] | null,
): TimelineSeries[] {
  const byId = new Map(timelines.map((t) => [t.playerId, t]))
  const ordered = ids
    ? ids.map((id) => byId.get(id)).filter((t): t is PlayerTimeline => t != null)
    : timelines
  return ordered.map((t, i) => ({
    label: t.nick,
    color: seriesColor(i),
    points: t.points,
  }))
}
