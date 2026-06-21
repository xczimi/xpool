import type { MatchScore, Tip } from '../graphql/types'

/** One scoreline and how many players predicted it. */
export interface ScorelineCount {
  homeScore: number
  awayScore: number
  count: number
}

/** Home/draw/away split of the visible predictions. */
export interface OutcomeSplit {
  home: number
  draw: number
  away: number
}

/** Aggregate prediction stats for one match, over the visible (gated) rows. */
export interface PredictionStats {
  /** Number of visible (non-hidden) predictions aggregated. */
  total: number
  /** Scoreline(s) tied for most predicted, descending count then deterministic. */
  mostCommon: ScorelineCount[]
  outcomeSplit: OutcomeSplit
  /**
   * How many predicted the exact FINAL result. `null` when there is no final
   * result yet (no `actual`, or `actual.provisional` — a live score must never
   * be reported as "nailed it").
   */
  nailedIt: number | null
}

/**
 * Aggregate the visibility-gated tip rows for a match. Returns `null` when no
 * prediction is visible yet (the gate is closed) — the caller renders nothing.
 *
 * Gate-safety: a still-hidden tip has `prediction === null` (the server gate in
 * `scored_tip`, `crates/api/src/gql/query.rs`). We aggregate only non-null
 * predictions, so before the gate opens there is nothing to leak.
 *
 * Pool scope: scoping is the caller's job — it passes the rows already filtered
 * to the selected pool (the `MATCH_QUERY` `pool` arg).
 */
export function computePredictionStats(
  rows: Tip[],
  actual: MatchScore | null,
): PredictionStats | null {
  const visible = rows.flatMap((r) => (r.prediction ? [r.prediction] : []))
  if (visible.length === 0) return null

  // Count each distinct scoreline.
  const counts = new Map<string, ScorelineCount>()
  for (const p of visible) {
    const key = `${p.homeScore}-${p.awayScore}`
    const existing = counts.get(key)
    counts.set(
      key,
      existing
        ? { ...existing, count: existing.count + 1 }
        : { homeScore: p.homeScore, awayScore: p.awayScore, count: 1 },
    )
  }

  // Most common = every scoreline tied for the top count. Sort descending by
  // count, then by home then away score for a stable, deterministic order.
  const ranked = [...counts.values()].sort(
    (a, b) =>
      b.count - a.count ||
      a.homeScore - b.homeScore ||
      a.awayScore - b.awayScore,
  )
  const topCount = ranked[0].count
  const mostCommon = ranked.filter((s) => s.count === topCount)

  const outcomeSplit = visible.reduce<OutcomeSplit>(
    (acc, p) => {
      if (p.homeScore > p.awayScore) return { ...acc, home: acc.home + 1 }
      if (p.homeScore < p.awayScore) return { ...acc, away: acc.away + 1 }
      return { ...acc, draw: acc.draw + 1 }
    },
    { home: 0, draw: 0, away: 0 },
  )

  const nailedIt =
    actual && !actual.provisional
      ? visible.filter(
          (p) =>
            p.homeScore === actual.homeScore &&
            p.awayScore === actual.awayScore,
        ).length
      : null

  return { total: visible.length, mostCommon, outcomeSplit, nailedIt }
}
