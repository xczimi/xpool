import type { Round } from '../graphql/types'
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
