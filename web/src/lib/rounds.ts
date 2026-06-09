import type { GroupGame, Round, SingleGame } from '../graphql/types'
import type { StringKey } from '../i18n/strings'

/**
 * Map each round to its i18n catalogue key. Round names are first-class
 * translated strings (CLAUDE.md i18n) — render them via `roundLabel`, never
 * a hardcoded English constant.
 */
const ROUND_LABEL_KEYS: Record<Round, StringKey> = {
  GROUP_STAGE: 'roundGroupStage',
  R32: 'roundR32',
  R16: 'roundR16',
  QF: 'roundQF',
  SF: 'roundSF',
  THIRD_PLACE: 'roundThirdPlace',
  FINAL: 'roundFinal',
}

/** The i18n string key for a round's display label. */
export function roundLabelKey(round: Round): StringKey {
  return ROUND_LABEL_KEYS[round]
}

/** Localised display label for a round, given a translator `t`. */
export function roundLabel(round: Round, t: (key: StringKey) => string): string {
  return t(roundLabelKey(round))
}

/**
 * Leaf groups (those holding matches) ordered chronologically by their
 * `deadline` — the earliest kickoff in the group's subtree. Groups without a
 * scheduled deadline sort last. Returns a new array; the input is not mutated.
 * Shared by the My Tips / All Tips pages and the `GroupSubNav` pills so the
 * default-selected group and the rendered order always agree.
 */
export function chronologicalLeafGroups(groups: GroupGame[]): GroupGame[] {
  const deadlineMs = (g: GroupGame): number =>
    g.deadline ? Date.parse(g.deadline) : Number.POSITIVE_INFINITY
  return groups
    .filter((g) => g.childGameIds.length > 0)
    .slice()
    .sort((a, b) => deadlineMs(a) - deadlineMs(b))
}

/** A group node is a leaf when it directly holds matches. */
function isLeafGroup(g: GroupGame): boolean {
  return g.childGameIds.length > 0
}

/**
 * The round-level group nodes — the internal nodes whose children are all
 * leaf groups (Group Stage parents the 12 groups; each knockout round parents
 * its one-match groups). Excludes the root and the bare `Knockout Stage`
 * container, whose children are themselves internal nodes. Ordered by
 * `ROUND_ORDER`. Drives the round-tab navigation.
 */
export function roundNodes(groups: GroupGame[]): GroupGame[] {
  const byId = new Map(groups.map((g) => [g.id, g]))
  return groups
    .filter(
      (g) =>
        g.childGroupIds.length > 0 &&
        g.childGroupIds.every((id) => {
          const child = byId.get(id)
          return child !== undefined && isLeafGroup(child)
        }),
    )
    .slice()
    .sort((a, b) => ROUND_ORDER.indexOf(a.round) - ROUND_ORDER.indexOf(b.round))
}

/**
 * The rounds whose participants are known well enough to predict: a round is
 * "ready" once at least one of its games has BOTH teams determined (a real
 * `teamId`, not a knockout placeholder). Group Stage games carry real teams
 * from import, so it is always ready. Readiness reflects the official results
 * the API has already resolved onto the games — never a player's own picks.
 */
export function readyRounds(
  groups: GroupGame[],
  games: SingleGame[],
): Set<Round> {
  const roundByGroupId = new Map(groups.map((g) => [g.id, g.round]))
  const ready = new Set<Round>()
  for (const game of games) {
    if (game.home.teamId && game.away.teamId) {
      const round = roundByGroupId.get(game.groupId)
      if (round) ready.add(round)
    }
  }
  return ready
}

/**
 * `roundNodes` filtered to the rounds ready for predictions (see `readyRounds`).
 * Drives the round-tab nav and the default round selection so neither ever
 * surfaces a round whose teams are still unknown. The ready-set only grows as
 * the tournament progresses, so a visible round never disappears underneath the
 * user.
 */
export function visibleRoundNodes(
  groups: GroupGame[],
  games: SingleGame[],
): GroupGame[] {
  const ready = readyRounds(groups, games)
  return roundNodes(groups).filter((node) => ready.has(node.round))
}

/**
 * The leaf groups directly under a round node, ordered chronologically.
 */
export function leafGroupsOfRound(
  roundNode: GroupGame,
  groups: GroupGame[],
): GroupGame[] {
  const byId = new Map(groups.map((g) => [g.id, g]))
  const children = roundNode.childGroupIds
    .map((id) => byId.get(id))
    .filter((g): g is GroupGame => g !== undefined)
  return chronologicalLeafGroups(children)
}

/**
 * The round the player should land on: the first round still open to predict
 * (its deadline has not passed), falling back to the last round once the
 * tournament is over. Server-authoritative — reads `deadlinePassed`, never the
 * wall clock.
 */
export function currentRoundNode(rounds: GroupGame[]): GroupGame | null {
  return rounds.find((r) => !r.deadlinePassed) ?? rounds[rounds.length - 1] ?? null
}

/** Display order for per-stage scoreboard breakdowns. */
export const ROUND_ORDER: Round[] = [
  'GROUP_STAGE',
  'R32',
  'R16',
  'QF',
  'SF',
  'THIRD_PLACE',
  'FINAL',
]

/**
 * Stage scoring multipliers (SCORING.md §2). The scoreboard `stages` are
 * already weighted server-side; these are a static frontend constant kept
 * only for display (e.g. the rules screen).
 */
export const STAGE_MULTIPLIERS: Record<Round, number> = {
  GROUP_STAGE: 1,
  R32: 2,
  R16: 3,
  QF: 4,
  SF: 5,
  THIRD_PLACE: 5,
  FINAL: 6,
}
