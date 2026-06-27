import type { MatchPrediction, MatchScore } from '../graphql/types'
import { scoreMatchPoints } from './matchScoring'

/** A hypothetical outcome's new total and its delta vs the current points. */
export interface WhatIfOutcome {
  total: number
  delta: number
}

/** What each next goal would do to one player's score on this match. */
export interface WhatIf {
  current: number
  ifHome: WhatIfOutcome
  ifAway: WhatIfOutcome
}

/**
 * For one player's `prediction` and the live `actual` score, compute the new
 * round-multiplied total (and delta vs current) under the two single-goal
 * hypotheticals: home scores next, or away scores next.
 *
 * Gate-safety: the caller only invokes this for rows with a non-null
 * prediction, so a still-hidden tip is never re-scored or leaked.
 */
export function computeWhatIf(
  prediction: MatchPrediction,
  actual: MatchScore,
  multiplier: number,
): WhatIf {
  const current = scoreMatchPoints(prediction, actual, multiplier)
  const ifHomeTotal = scoreMatchPoints(
    prediction,
    { homeScore: actual.homeScore + 1, awayScore: actual.awayScore },
    multiplier,
  )
  const ifAwayTotal = scoreMatchPoints(
    prediction,
    { homeScore: actual.homeScore, awayScore: actual.awayScore + 1 },
    multiplier,
  )
  return {
    current,
    ifHome: { total: ifHomeTotal, delta: ifHomeTotal - current },
    ifAway: { total: ifAwayTotal, delta: ifAwayTotal - current },
  }
}
