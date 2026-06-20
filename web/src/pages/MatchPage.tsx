import { useEffect, useMemo } from 'react'
import { useParams } from 'react-router-dom'
import { useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import { MATCH_QUERY, TOURNAMENT_QUERY } from '../graphql/queries'
import type { MatchDetail, Tournament } from '../graphql/types'
import { ErrorView, Loading, NeedsLogin } from '../components/StatusViews'
import { Matchup } from '../components/TeamLabel'
import { PointsBadge } from '../components/PointsBadge'
import { teamIndex, formatKickoff } from '../lib/format'

/**
 * Match page (#2). The all-players tip grid is the spine in every state; the
 * live/official score and provisional points are an overlay on top. Polls
 * every 60s only while the match is live (`actual.provisional`).
 */
export function MatchPage() {
  const { gameId = '' } = useParams()
  const { t, locale } = useI18n()
  const { label } = useAuth()

  const [tournamentResult] = useQuery<{ tournament: Tournament | null }>({
    query: TOURNAMENT_QUERY,
  })
  const [matchResult, reexecuteMatch] = useQuery<{ match: MatchDetail | null }>({
    query: MATCH_QUERY,
    variables: { gameId },
    pause: !gameId,
  })

  const tournament = tournamentResult.data?.tournament ?? null
  const teams = useMemo(
    () => teamIndex(tournament?.teams ?? [], locale),
    [tournament?.teams, locale],
  )
  const match = matchResult.data?.match ?? null
  const isLive = match?.actual?.provisional ?? false

  // Poll only while live. 60s matches the server cache floor — polling faster
  // would only re-read the cache, never hit SportsDB more often.
  useEffect(() => {
    if (!isLive) return
    const id = setInterval(
      () => reexecuteMatch({ requestPolicy: 'network-only' }),
      60_000,
    )
    return () => clearInterval(id)
  }, [isLive, reexecuteMatch])

  if (!label) return <NeedsLogin />
  if (matchResult.fetching || tournamentResult.fetching) return <Loading />
  if (matchResult.error)
    return (
      <ErrorView
        message={matchResult.error.message}
        onRetry={() => reexecuteMatch({ requestPolicy: 'network-only' })}
      />
    )
  if (!match) return <ErrorView message="match not found" />

  const { game, actual, rows } = match

  return (
    <section className="page match-page">
      <h2>{t('match')}</h2>

      <div className="match-card">
        <div className="match-card-teams">
          <Matchup home={game.home} away={game.away} teams={teams} />
        </div>
        <div className="match-card-kickoff">
          {formatKickoff(game.kickoff, locale)}
        </div>

        {actual ? (
          <>
            <div
              className={`match-scoreline ${actual.provisional ? 'is-live' : 'is-final'}`}
            >
              <span className="match-scoreline-value">
                {actual.homeScore}–{actual.awayScore}
              </span>
              <span className="match-scoreline-label">
                {actual.provisional
                  ? `${t('liveLabel')}${actual.sourceStatus ? ` · ${actual.sourceStatus}` : ''}`
                  : t('finalLabel')}
              </span>
            </div>
            {actual.provisional && (
              <p className="match-note match-provisional">{t('provisionalLabel')}</p>
            )}
            {actual.provisional && actual.ninetyMinuteUncertain && (
              <p className="match-note match-warn">{t('ninetyMinuteNote')}</p>
            )}
          </>
        ) : (
          game.resultPending && (
            <p className="match-note match-muted">{t('awaitingResult')}</p>
          )
        )}
      </div>

      <table className="data-table compact match-grid">
        <thead>
          <tr>
            <th>{t('player')}</th>
            <th>{t('prediction')}</th>
            <th className="num">{t('points')}</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.playerId}>
              <td className="nick">{row.nick}</td>
              <td className="pred">
                {row.prediction ? (
                  `${row.prediction.homeScore}–${row.prediction.awayScore}`
                ) : (
                  <span className="match-hidden">{t('hiddenTip')}</span>
                )}
              </td>
              <td className="pts num">
                {row.points != null ? (
                  <PointsBadge
                    breakdown={row.breakdown}
                    points={row.points}
                    isPerfect={row.isPerfect}
                  />
                ) : (
                  '—'
                )}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  )
}
