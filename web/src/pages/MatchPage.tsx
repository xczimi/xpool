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
  if (matchResult.error) return <ErrorView message={matchResult.error.message} />
  if (!match) return <ErrorView message="match not found" />

  const { game, actual, rows } = match

  return (
    <section className="match-page">
      <header className="match-head">
        <h1>
          <Matchup home={game.home} away={game.away} teams={teams} />
        </h1>
        <p className="kickoff">{formatKickoff(game.kickoff, locale)}</p>
        {actual ? (
          <p className={`score ${actual.provisional ? 'score-live' : 'score-final'}`}>
            <span className="score-value">
              {actual.homeScore}–{actual.awayScore}
            </span>
            {actual.provisional && (
              <span className="score-status">
                {t('liveLabel')}
                {actual.sourceStatus ? ` · ${actual.sourceStatus}` : ''}
              </span>
            )}
            {actual.provisional && <span className="provisional-note">{t('provisionalLabel')}</span>}
          </p>
        ) : (
          game.resultPending && <p className="awaiting">{t('awaitingResult')}</p>
        )}
        {actual?.ninetyMinuteUncertain && actual.provisional && (
          <p className="ninety-note">{t('ninetyMinuteNote')}</p>
        )}
      </header>

      <table className="tips-grid">
        <tbody>
          {rows.map((row) => (
            <tr key={row.playerId}>
              <td className="nick">{row.nick}</td>
              <td className="pred">
                {row.prediction
                  ? `${row.prediction.homeScore}–${row.prediction.awayScore}`
                  : '—'}
              </td>
              <td className="pts">
                <PointsBadge
                  breakdown={row.breakdown}
                  points={row.points}
                  isPerfect={row.isPerfect}
                />
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </section>
  )
}
