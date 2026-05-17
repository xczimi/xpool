import type { Round } from '../graphql/types'

/** Human-readable round labels. */
export const ROUND_LABELS: Record<Round, string> = {
  GROUP_STAGE: 'Group Stage',
  R32: 'Round of 32',
  R16: 'Round of 16',
  QF: 'Quarter-final',
  SF: 'Semi-final',
  THIRD_PLACE: 'Third place',
  FINAL: 'Final',
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
