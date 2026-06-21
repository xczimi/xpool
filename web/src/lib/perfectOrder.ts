import type { Perfect } from '../graphql/types'

/** The two ways the Perfect page can order its flat list. */
export type PerfectView = 'match' | 'player'

/** Kickoff epoch for a perfect's game; missing games sort last. */
function kickoffOf(p: Perfect, kickoff: Map<string, number>): number {
  return kickoff.get(p.gameId) ?? Infinity
}

/** By-match: kickoff asc, tie-broken by nick so a match's perfects stay grouped. */
function byMatch(list: Perfect[], kickoff: Map<string, number>): Perfect[] {
  return [...list].sort(
    (a, b) =>
      kickoffOf(a, kickoff) - kickoffOf(b, kickoff) || a.nick.localeCompare(b.nick),
  )
}

/**
 * By-player: each player's perfects contiguous and kickoff-ordered; players
 * ordered by perfect-count desc, ties broken by first appearance (stable).
 */
function byPlayer(list: Perfect[], kickoff: Map<string, number>): Perfect[] {
  // Preserve first-appearance order while grouping, in one pass.
  const order: string[] = []
  const groups = new Map<string, Perfect[]>()
  for (const perfect of list) {
    const bucket = groups.get(perfect.playerId)
    if (bucket) {
      bucket.push(perfect)
    } else {
      order.push(perfect.playerId)
      groups.set(perfect.playerId, [perfect])
    }
  }

  return order
    .map((playerId, index) => ({ playerId, index }))
    // Most perfects first; equal counts keep first-appearance order (stable).
    .sort(
      (a, b) =>
        (groups.get(b.playerId)?.length ?? 0) -
          (groups.get(a.playerId)?.length ?? 0) || a.index - b.index,
    )
    .flatMap(({ playerId }) =>
      [...(groups.get(playerId) ?? [])].sort(
        (a, b) => kickoffOf(a, kickoff) - kickoffOf(b, kickoff),
      ),
    )
}

/** Reorder the flat perfects list for the chosen view. Never mutates input. */
export function orderPerfects(
  list: Perfect[],
  view: PerfectView,
  kickoff: Map<string, number>,
): Perfect[] {
  return view === 'player' ? byPlayer(list, kickoff) : byMatch(list, kickoff)
}

/** localStorage key for the persisted Perfect-page view mode. */
export const PERFECT_VIEW_KEY = 'xpool.perfectView'

/** Read the persisted view, defaulting to by-match. Total + failure-safe. */
export function readPerfectView(): PerfectView {
  try {
    return localStorage.getItem(PERFECT_VIEW_KEY) === 'player' ? 'player' : 'match'
  } catch {
    return 'match'
  }
}

/** Persist the chosen view. Convenience state — failures are swallowed. */
export function writePerfectView(view: PerfectView): void {
  try {
    localStorage.setItem(PERFECT_VIEW_KEY, view)
  } catch {
    /* ignore */
  }
}
