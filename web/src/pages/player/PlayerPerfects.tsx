import { useMemo } from 'react'
import { useI18n } from '../../i18n/useI18n'
import type { MatchPrediction, Perfect, Tournament } from '../../graphql/types'
import type { Locale } from '../../i18n/strings'
import { teamIndex } from '../../lib/format'
import { Matchup } from '../../components/TeamLabel'
import { PointsBadge } from '../../components/PointsBadge'

/**
 * A player's perfect predictions shown with full match context — which game,
 * the official result, the points — because a bare points badge says nothing.
 * Renders nothing when the player has no perfects.
 */
export function PlayerPerfects({
  perfects,
  tournament,
  resultByGame,
  locale,
}: {
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

  if (perfects.length === 0) return null

  return (
    <section className="player-perfects">
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
                  <td>{r ? `${r.homeScore}–${r.awayScore}` : '—'}</td>
                  <td>
                    <PointsBadge breakdown={p.breakdown} isPerfect />
                  </td>
                </tr>
              )
            })}
          </tbody>
        </table>
      </div>
    </section>
  )
}
