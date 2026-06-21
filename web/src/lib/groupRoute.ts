import type { GroupGame, Round } from '../graphql/types'
import { roundNodes } from './rounds'

/**
 * Deep-link resolution for the My Tips group route (`/mytips/:groupId`).
 *
 * The URL carries a single path segment naming a group node. Two kinds resolve:
 *
 * - a LEAF group (holds matches, `childGameIds.length > 0`) — e.g. the
 *   group-stage `A`..`L` or a knockout `KO-M73`. Selects that round AND that
 *   group.
 * - a ROUND-NODE group (internal node parenting leaf groups, e.g. `GROUPSTAGE`,
 *   `R32`, `FINAL`). Selects the round only; the page falls back to that round's
 *   first group (group stage) or stacks every one-match group (knockout).
 *
 * Anything else (an unknown id, the ROOT / bare-`Knockout` containers whose
 * children are themselves internal nodes, an empty/undefined segment) returns
 * `null` so the caller keeps its default round/group behaviour.
 */
export function resolveGroupParam(
  groups: GroupGame[],
  nodeId: string | undefined,
): { round: Round; groupId: string | null } | null {
  if (!nodeId) return null

  const match = groups.find((g) => g.id === nodeId)
  if (!match) return null

  // A leaf group directly holds matches → select its round AND group.
  if (match.childGameIds.length > 0) {
    return { round: match.round, groupId: match.id }
  }

  // A round-node group (parents only leaf groups) → select the round only.
  // `roundNodes` excludes the ROOT and the bare `Knockout` container, whose
  // children are themselves internal nodes — those must NOT deep-link.
  const isRoundNode = roundNodes(groups).some((n) => n.id === nodeId)
  if (isRoundNode) {
    return { round: match.round, groupId: null }
  }

  return null
}

/**
 * The id of the round-node group for a round — used to build the URL when a
 * round TAB is clicked (the tab carries a round, the URL carries a group id).
 * Returns `undefined` when the round has no round-node group (not navigable).
 */
export function roundNodeIdFor(
  groups: GroupGame[],
  round: Round,
): string | undefined {
  return roundNodes(groups).find((n) => n.round === round)?.id
}
