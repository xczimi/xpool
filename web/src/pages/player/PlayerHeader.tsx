import { useI18n } from '../../i18n/useI18n'
import type { Perfect, ScoreEntry } from '../../graphql/types'
import { roundPointsOf } from '../../lib/playerPage'
import { ROUND_ORDER, roundLabel } from '../../lib/rounds'
import { PointsBadge } from '../../components/PointsBadge'

/**
 * Dense, always-visible summary of one player: total + rank, a per-round point
 * strip (only rounds they have a score in), and their perfect predictions.
 * Pure presentation — all figures are derived upstream.
 */
export function PlayerHeader({
  entry,
  rank,
  perfects,
}: {
  entry: ScoreEntry
  rank: number | null
  perfects: Perfect[]
}) {
  const { t } = useI18n()
  const byRound = roundPointsOf(entry)
  // Show a strip cell for every round the player actually scored, in order.
  const strip = ROUND_ORDER.filter((r) => byRound.has(r))

  return (
    <div className="player-header">
      <div className="player-totals">
        <span className="player-total">
          {t('total')}: <strong>{entry.total}</strong>
        </span>
        {rank !== null && (
          <span className="player-rank">
            {t('rank')}: <strong>{rank}</strong>
          </span>
        )}
      </div>

      {strip.length > 0 && (
        <ul className="player-round-strip">
          {strip.map((r) => (
            <li key={r}>
              <span className="strip-round">{roundLabel(r, t)}</span>
              <span className="strip-points">{byRound.get(r) ?? 0}</span>
            </li>
          ))}
        </ul>
      )}

      {perfects.length > 0 && (
        <div className="player-perfects">
          <h3>
            {t('playerPerfectsHeading')} ({perfects.length})
          </h3>
          <ul className="player-perfect-list">
            {perfects.map((p) => (
              <li key={p.gameId}>
                <PointsBadge breakdown={p.breakdown} isPerfect />
              </li>
            ))}
          </ul>
        </div>
      )}
    </div>
  )
}
