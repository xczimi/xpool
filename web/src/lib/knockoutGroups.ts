import type { GroupGame, SingleGame } from '../graphql/types'
import { groupGames, isKnockoutMatch } from './standings'

/**
 * The ids of leaf groups that are single-game knockout ties — derived from
 * match arity (one game per group), not the round name, reusing the same
 * `groupGames` + `isKnockoutMatch` helpers the knockout tip labels rely on.
 *
 * Used to decide whether a "go to my tips" link should read "Open this group"
 * (group stage) or "Open this KO match" (knockout). Internal/parent groups
 * hold no games and so are never knockout here.
 */
export function knockoutGroupIds(
  groups: readonly GroupGame[],
  games: SingleGame[],
): Set<string> {
  const ids = new Set<string>()
  for (const group of groups) {
    if (isKnockoutMatch(groupGames(group, games))) ids.add(group.id)
  }
  return ids
}
