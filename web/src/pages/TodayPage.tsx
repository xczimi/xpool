import { useEffect, useMemo } from 'react'
import { Link } from 'react-router-dom'
import { useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import { ME_QUERY, RESULTS_QUERY, TOURNAMENT_QUERY } from '../graphql/queries'
import type {
  MatchPrediction,
  Me,
  Tournament,
} from '../graphql/types'
import { ErrorView, Loading } from '../components/StatusViews'
import { usePolledQuery } from '../lib/usePolledQuery'
import { pollIntervalMs } from '../lib/polling'
import { byKickoff, formatKickoff, teamIndex } from '../lib/format'
import { Matchup } from '../components/TeamLabel'

/**
 * Today / Fresh — a flat list of matches within ~±2 days of now (UC-11).
 * Polls only while a match is result-pending (API.md §7). Logged-in players
 * also see their prediction per match.
 */
export function TodayPage() {
  const { t, locale } = useI18n()
  const { label } = useAuth()

  // First fetch (no polling) to learn whether anything is result-pending.
  const [probe] = useQuery<{
    tournament: Tournament | null
  }>({ query: TOURNAMENT_QUERY })
  const [resultsResult, refetchResults] = useQuery<{
    results: MatchPrediction[]
  }>({ query: RESULTS_QUERY })
  const resultsByGame = useMemo(() => {
    const map = new Map<string, MatchPrediction>()
    for (const r of resultsResult.data?.results ?? []) {
      map.set(r.gameId, r)
    }
    return map
  }, [resultsResult.data])
  const interval = useMemo(
    () => pollIntervalMs(probe.data?.tournament?.games ?? []),
    [probe.data],
  )
  const [result, reexecute] = usePolledQuery<{
    tournament: Tournament | null
  }>({ query: TOURNAMENT_QUERY }, interval)

  // Keep the official results in step with the tournament poll.
  useEffect(() => {
    if (interval <= 0) return
    const id = setInterval(
      () => refetchResults({ requestPolicy: 'network-only' }),
      interval,
    )
    return () => clearInterval(id)
  }, [interval, refetchResults])

  const [meResult] = useQuery<{ me: Me }>({
    query: ME_QUERY,
    pause: !label,
  })

  const tournament = result.data?.tournament ?? null
  const teams = useMemo(
    () => teamIndex(tournament?.teams ?? [], locale),
    [tournament, locale],
  )
  const tipFor = useMemo(() => {
    const map = new Map<string, { home: number; away: number; locked: boolean }>()
    const mePlayer = meResult.data?.me?.__typename === 'Player' ? meResult.data.me : null
    for (const p of mePlayer?.matchPredictions ?? []) {
      map.set(p.gameId, {
        home: p.homeScore,
        away: p.awayScore,
        locked: p.locked,
      })
    }
    return map
  }, [meResult.data])

  if (result.fetching && !tournament) return <Loading />
  if (result.error)
    return (
      <ErrorView
        message={result.error.message}
        onRetry={() => reexecute({ requestPolicy: 'network-only' })}
      />
    )
  if (!tournament) return <ErrorView />

  const games = tournament.games
    .filter((g) => g.withinTodayWindow)
    .sort(byKickoff)

  return (
    <section className="page">
      <h2>{t('todayTitle')}</h2>
      {interval > 0 && <p className="poll-note">● live</p>}
      {games.length === 0 ? (
        <p>{t('todayEmpty')}</p>
      ) : (
        <table className="data-table">
          <thead>
            <tr>
              <th>{t('kickoff')}</th>
              <th className="col-match">{t('match')}</th>
              <th>{t('result')}</th>
              {label && <th>{t('yourTip')}</th>}
            </tr>
          </thead>
          <tbody>
            {games.map((m) => {
              const tip = tipFor.get(m.id)
              return (
                <tr key={m.id} className={m.isToday ? 'is-today' : undefined}>
                  <td>{formatKickoff(m.kickoff, locale)}</td>
                  <td>
                    <Link to={`/match/${m.id}`}>
                      <Matchup home={m.home} away={m.away} teams={teams} />
                    </Link>
                    {' '}
                    <Link
                      to={`/mytips/${m.groupId}`}
                      className="open-group-link"
                    >
                      {t('openGroup')}
                    </Link>
                  </td>
                  <td>
                    {(() => {
                      const r = resultsByGame.get(m.id)
                      return r ? `${r.homeScore}–${r.awayScore}` : '—'
                    })()}
                  </td>
                  {label && (
                    <td>
                      {tip
                        ? `${tip.home}–${tip.away} ${
                            tip.locked ? `(${t('locked')})` : `(${t('draft')})`
                          }`
                        : '—'}
                    </td>
                  )}
                </tr>
              )
            })}
          </tbody>
        </table>
      )}
    </section>
  )
}
