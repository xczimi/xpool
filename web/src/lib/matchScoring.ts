/**
 * Client-side mirror of the domain per-match scoring rule
 * (`crates/domain/src/scoring.rs`, SCORING.md §2–3). Used for live "what-if"
 * re-scoring in the browser — NOT a replacement for server scoring, which
 * remains authoritative. Constants mirror `ScoringConfig::default()`.
 */

/** A scoreline — both predictions and (hypothetical) results share this shape. */
export interface ScoreInput {
  homeScore: number
  awayScore: number
}

const EXACT_SCORE_POINT = 1
const OUTCOME_POINT = 2
const HIGH_SCORING_THRESHOLD = 4

/** sign of (home - away): home win > 0, draw = 0, away win < 0. */
const outcomeSign = (s: ScoreInput): number => Math.sign(s.homeScore - s.awayScore)

/**
 * Base (pre-multiplier) points for prediction `pred` vs result `actual`,
 * applying the per-side symmetric 4-goal rule: a side counts as exact when the
 * two values match OR both are at/above the high-scoring threshold.
 */
export function scoreMatchBase(pred: ScoreInput, actual: ScoreInput): number {
  const exactHome =
    pred.homeScore === actual.homeScore ||
    (pred.homeScore >= HIGH_SCORING_THRESHOLD && actual.homeScore >= HIGH_SCORING_THRESHOLD)
  const exactAway =
    pred.awayScore === actual.awayScore ||
    (pred.awayScore >= HIGH_SCORING_THRESHOLD && actual.awayScore >= HIGH_SCORING_THRESHOLD)
  const outcome = outcomeSign(pred) === outcomeSign(actual)

  return (
    (exactHome ? EXACT_SCORE_POINT : 0) +
    (exactAway ? EXACT_SCORE_POINT : 0) +
    (outcome ? OUTCOME_POINT : 0)
  )
}

/** Round-multiplied points for prediction `pred` vs result `actual`. */
export function scoreMatchPoints(pred: ScoreInput, actual: ScoreInput, multiplier: number): number {
  return scoreMatchBase(pred, actual) * multiplier
}
