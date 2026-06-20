import { useMemo } from 'react'
import { useI18n } from '../../i18n/useI18n'
import type {
  MatchPrediction,
  Perfect,
  ScoreEntry,
  Tournament,
} from '../../graphql/types'
import type { Locale } from '../../i18n/strings'
import { teamIndex } from '../../lib/format'
import { Matchup } from '../../components/TeamLabel'
import { PointsBadge } from '../../components/PointsBadge'

/**
 * Dense, always-visible summary of one player: total + rank as stat cards, and
 * their perfect predictions shown with full match context (which game, the
 * official result, the points) — a bare points badge alone says nothing. The
 * per-round breakdown is the collapsed `PlayerRounds` list below, not repeated
 * here.
 */
export function PlayerHeader({
  entry,
  rank,
  perfects,
  tournament,
  resultByGame,
  locale,
}: {
  entry: ScoreEntry
  rank: number | null
  perfects: Perfect[]
  tournament: Tournament
  resultByGame: Map<string, MatchPrediction>
  locale: Locale
}) {
  const { t } = useI18n()
  const teams = useMemo(
    () => teamIndex(tournament.teams, locale),
    [tournament.teams, locale],
  )
  const gameById = useMemo(
    () => new Map(tournament.games.map((g) => [g.id, g])),
    [tournament.games],
  )

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

      {perfects.length > 0 && (
        <div className="player-perfects">
          <h3>
            {t('playerPerfectsHeading')} ({perfects.length})
          </h3>
          <div className="grid-scroll">
            <table className="data-table compact">
              <thead>
                <tr>
                  <th className="col-match">{t('match')}</th>
                  <th>{t('result')}</th>
                  <th>{t('points')}</th>
                </tr>
              </thead>
              <tbody>
                {perfects.map((p) => {
                  const g = gameById.get(p.gameId)
                  const r = resultByGame.get(p.gameId)
                  return (
                    <tr key={p.gameId}>
                      <td>
                        {g ? (
                          <Matchup
                            home={g.home}
                            away={g.away}
                            teams={teams}
                            compact
                          />
                        ) : (
                          p.gameId
                        )}
                      </td>
                      <td>
                        {r ? `${r.homeScore}–${r.awayScore}` : '—'}
                      </td>
                      <td>
                        <PointsBadge breakdown={p.breakdown} isPerfect />
                      </td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
          </div>
        </div>
      )}
    </div>
  )
}
