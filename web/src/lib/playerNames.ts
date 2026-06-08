import type { PlayerSummary } from '../graphql/types'

/** Build an id→nick lookup from the roster (the public `players` list). */
export function nickIndex(players: readonly PlayerSummary[]): Map<string, string> {
  return new Map(players.map((p) => [p.id, p.nick]))
}

/**
 * Resolve a player id to its display nick. An unknown id returns `fallback`
 * (a neutral placeholder) so a data gap surfaces without leaking a raw id.
 */
export function displayNick(
  index: Map<string, string>,
  id: string,
  fallback: string,
): string {
  return index.get(id) ?? fallback
}
