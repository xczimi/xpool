import type { SingleGame } from '../graphql/types'

/**
 * Buffer after kickoff before a match's result is expected (API.md §7):
 * ~1h45 for a 90-minute match. Knockout matches may run to ET/penalties so
 * the model uses a longer window.
 */
const GROUP_BUFFER_MS = 105 * 60 * 1000
const KNOCKOUT_BUFFER_MS = 150 * 60 * 1000

function bufferFor(game: SingleGame): number {
  return game.groupId.startsWith('G-') || /group/i.test(game.groupId)
    ? GROUP_BUFFER_MS
    : KNOCKOUT_BUFFER_MS
}

/**
 * A match is *result-pending* when its estimated end has passed and no locked
 * official result has been entered yet (API.md §7). Official results come
 * from the `results` query, keyed by game id.
 */
export function isResultPending(
  game: SingleGame,
  resultGameIds: ReadonlySet<string>,
  now: number,
): boolean {
  if (resultGameIds.has(game.id)) {
    return false
  }
  const kickoff = Date.parse(game.kickoff)
  if (Number.isNaN(kickoff)) {
    return false
  }
  return now > kickoff + bufferFor(game)
}

/**
 * Returns a poll interval: poll only when at least one loaded match is
 * result-pending; otherwise no polling (static data). `resultGameIds` is the
 * set of game ids with a locked official result.
 */
export function pollIntervalMs(
  games: SingleGame[],
  resultGameIds: ReadonlySet<string> = new Set(),
  now: number = Date.now(),
): number {
  const pending = games.some((g) => isResultPending(g, resultGameIds, now))
  return pending ? 30_000 : 0
}
