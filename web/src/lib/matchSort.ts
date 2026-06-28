import type { Tip } from '../graphql/types'

/** Which column the match-page prediction grid is sorted by. */
export type MatchSortColumn = 'standing' | 'player' | 'prediction' | 'points'

export type SortDirection = 'asc' | 'desc'

export interface MatchSort {
  column: MatchSortColumn
  direction: SortDirection
}

/** Default = the server/scoreboard order the rows arrive in. */
export const DEFAULT_MATCH_SORT: MatchSort = { column: 'standing', direction: 'asc' }

export const MATCH_SORT_KEY = 'xpool.matchSort'

/** The direction a column adopts when first selected. */
const DEFAULT_DIRECTION: Record<MatchSortColumn, SortDirection> = {
  standing: 'asc',
  player: 'asc',
  prediction: 'asc',
  points: 'desc',
}

/**
 * Sort a copy of the gated tip rows. Hidden tips (`prediction === null`) and
 * unscored rows (`points === null`) always sink to the bottom regardless of
 * direction, so the visible data is never interleaved with placeholders. Stable:
 * ties fall back to the original (server) order via the captured index.
 */
export function sortRows(rows: readonly Tip[], sort: MatchSort): Tip[] {
  const indexed = rows.map((row, index) => ({ row, index }))
  const factor = sort.direction === 'asc' ? 1 : -1

  indexed.sort((a, b) => {
    switch (sort.column) {
      case 'player':
        return a.row.nick.localeCompare(b.row.nick) * factor || a.index - b.index
      case 'prediction': {
        const pa = a.row.prediction
        const pb = b.row.prediction
        if (!pa && !pb) return a.index - b.index
        if (!pa) return 1
        if (!pb) return -1
        return (
          ((pa.homeScore - pb.homeScore) || (pa.awayScore - pb.awayScore)) * factor ||
          a.index - b.index
        )
      }
      case 'points': {
        const va = a.row.points
        const vb = b.row.points
        if (va == null && vb == null) return a.index - b.index
        if (va == null) return 1
        if (vb == null) return -1
        return (va - vb) * factor || a.index - b.index
      }
      case 'standing':
      default:
        return (a.index - b.index) * factor || a.row.nick.localeCompare(b.row.nick)
    }
  })

  return indexed.map((entry) => entry.row)
}

/** Clicking a header: toggle direction if same column, else adopt its default. */
export function nextSort(current: MatchSort, column: MatchSortColumn): MatchSort {
  if (current.column === column) {
    return { column, direction: current.direction === 'asc' ? 'desc' : 'asc' }
  }
  return { column, direction: DEFAULT_DIRECTION[column] }
}

function isMatchSort(value: unknown): value is MatchSort {
  if (typeof value !== 'object' || value === null) return false
  const v = value as Record<string, unknown>
  return (
    (v.column === 'standing' ||
      v.column === 'player' ||
      v.column === 'prediction' ||
      v.column === 'points') &&
    (v.direction === 'asc' || v.direction === 'desc')
  )
}

/** Read the persisted sort, falling back to the default on any error. */
export function readMatchSort(): MatchSort {
  try {
    const raw = localStorage.getItem(MATCH_SORT_KEY)
    if (!raw) return DEFAULT_MATCH_SORT
    const parsed: unknown = JSON.parse(raw)
    return isMatchSort(parsed) ? parsed : DEFAULT_MATCH_SORT
  } catch {
    return DEFAULT_MATCH_SORT
  }
}

/** Persist the chosen sort (best-effort — a convenience, not load-bearing). */
export function writeMatchSort(sort: MatchSort): void {
  try {
    localStorage.setItem(MATCH_SORT_KEY, JSON.stringify(sort))
  } catch {
    /* ignore */
  }
}
