import { useI18n } from '../../i18n/useI18n'
import type { ScoreEntry } from '../../graphql/types'

/**
 * Dense, always-visible summary of one player: total + rank as stat cards. The
 * perfects (`PlayerPerfects`) and per-round breakdown (`PlayerRounds`) render
 * as their own sections below.
 */
export function PlayerHeader({
  entry,
  rank,
}: {
  entry: ScoreEntry
  rank: number | null
}) {
  const { t } = useI18n()

  return (
    <div className="player-header">
      <div className="player-stats">
        <div className="player-stat">
          <span className="player-stat-label">{t('total')}</span>
          <span className="player-stat-value">{entry.total}</span>
        </div>
        {rank !== null && (
          <div className="player-stat">
            <span className="player-stat-label">{t('rank')}</span>
            <span className="player-stat-value">#{rank}</span>
          </div>
        )}
      </div>
    </div>
  )
}
