import { useI18n } from '../i18n/useI18n'
import type { MatchScore, Tip } from '../graphql/types'
import { computePredictionStats } from '../lib/predictionStats'

interface PredictionStatsProps {
  /** The match's visibility-gated tip rows (already pool-scoped by the query). */
  rows: Tip[]
  /** The official/live score, if any — drives the "nailed it" line. */
  actual: MatchScore | null
}

/**
 * Aggregate "what everyone tipped" for one match. Renders nothing until the
 * visibility gate opens (the helper returns `null` when no prediction is
 * visible) — the caller shows a "hidden until kickoff" note in that state.
 */
export function PredictionStats({ rows, actual }: PredictionStatsProps) {
  const { t } = useI18n()
  const stats = computePredictionStats(rows, actual)
  if (!stats) return null

  const scoreLabel =
    stats.mostCommon.length > 1 ? t('mostCommonScorePlural') : t('mostCommonScore')

  return (
    <section className="prediction-stats" aria-label={t('predictionStatsTitle')}>
      <h3 className="prediction-stats-title">{t('predictionStatsTitle')}</h3>

      <dl className="prediction-stats-list">
        <div className="prediction-stats-row">
          <dt>{scoreLabel}</dt>
          <dd>
            {stats.mostCommon.map((s) => (
              <span key={`${s.homeScore}-${s.awayScore}`} className="stats-scoreline">
                {s.homeScore}–{s.awayScore}{' '}
                <small className="stats-count">×{s.count}</small>
              </span>
            ))}
          </dd>
        </div>

        <div className="prediction-stats-row">
          <dt>{t('outcomeSplitLabel')}</dt>
          <dd className="stats-outcome-split">
            <span className="stats-outcome">
              {t('outcomeHome')}: <strong>{stats.outcomeSplit.home}</strong>
            </span>
            <span className="stats-outcome">
              {t('outcomeDraw')}: <strong>{stats.outcomeSplit.draw}</strong>
            </span>
            <span className="stats-outcome">
              {t('outcomeAway')}: <strong>{stats.outcomeSplit.away}</strong>
            </span>
          </dd>
        </div>

        {stats.nailedIt != null && (
          <div className="prediction-stats-row">
            <dt>{t('nailedItLabel')}</dt>
            <dd>
              <strong>{stats.nailedIt}</strong>
            </dd>
          </div>
        )}
      </dl>

      <p className="prediction-stats-total">
        {stats.total} {t('statsTipCount')}
      </p>
    </section>
  )
}
