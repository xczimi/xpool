import { useMemo, type ReactNode } from 'react'
import { Link } from 'react-router-dom'
import { useQuery } from 'urql'
import { useI18n } from '../i18n/useI18n'
import { PERFECTS_QUERY, RESULTS_QUERY, TOURNAMENT_QUERY } from '../graphql/queries'
import type { MatchPrediction, Perfect, Tournament } from '../graphql/types'
import { ErrorView, Loading } from '../components/StatusViews'
import { teamIndex } from '../lib/format'
import { Matchup } from '../components/TeamLabel'
import { PointsBadge } from '../components/PointsBadge'

/** Players who scored a maximum (4-point) match prediction (UC-10). Public. */
export function PerfectPage() {
  const { t, locale } = useI18n()
  const [result, reexecute] = useQuery<{ perfects: Perfect[] }>({
    query: PERFECTS_QUERY,
  })
  const [tournamentResult] = useQuery<{
    tournament: Tournament | null
  }>({ query: TOURNAMENT_QUERY })
  const [resultsResult] = useQuery<{ results: MatchPrediction[] }>({
    query: RESULTS_QUERY,
  })

  const tournament = tournamentResult.data?.tournament ?? null
  const teams = useMemo(
    () => teamIndex(tournament?.teams ?? [], locale),
    [tournament, locale],
  )
  const gameLabel = useMemo(() => {
    const map = new Map<string, ReactNode>()
    for (const g of tournament?.games ?? []) {
      map.set(
        g.id,
        <Link to={`/match/${g.id}`}>
          <Matchup home={g.home} away={g.away} teams={teams} />
        </Link>,
      )
    }
    return map
  }, [tournament, teams])
  // gameId -> kickoff epoch, for sorting perfects by match order (not by name).
  const kickoffOf = useMemo(() => {
    const map = new Map<string, number>()
    for (const g of tournament?.games ?? []) {
      map.set(g.id, Date.parse(g.kickoff))
    }
    return map
  }, [tournament])
  const resultByGame = useMemo(() => {
    const map = new Map<string, MatchPrediction>()
    for (const r of resultsResult.data?.results ?? []) {
      map.set(r.gameId, r)
    }
    return map
  }, [resultsResult.data])

  // Order by match kickoff (earliest first), tie-broken by player nick so the
  // several perfects for one match stay grouped and stable.
  const perfects = useMemo(() => {
    const list = result.data?.perfects ?? []
    return [...list].sort((a, b) => {
      const ka = kickoffOf.get(a.gameId) ?? Infinity
      const kb = kickoffOf.get(b.gameId) ?? Infinity
      return ka - kb || a.nick.localeCompare(b.nick)
    })
  }, [result.data, kickoffOf])

  if (result.fetching) return <Loading />
  if (result.error)
    return (
      <ErrorView
        message={result.error.message}
        onRetry={() => reexecute({ requestPolicy: 'network-only' })}
      />
    )

  return (
    <section className="page">
      <h2>{t('perfectTitle')}</h2>
      <p>{t('perfectIntro')}</p>
      {perfects.length === 0 ? (
        <p>{t('perfectEmpty')}</p>
      ) : (
        <table className="data-table">
          <thead>
            <tr>
              <th>{t('player')}</th>
              <th className="col-match">{t('match')}</th>
              <th>{t('result')}</th>
              <th>{t('points')}</th>
            </tr>
          </thead>
          <tbody>
            {perfects.map((p, i) => {
              const r = resultByGame.get(p.gameId)
              return (
                <tr key={`${p.playerId}-${p.gameId}-${i}`}>
                  <td>
                    <Link to={`/player/${p.playerId}`}>{p.nick}</Link>
                  </td>
                  <td>{gameLabel.get(p.gameId) ?? p.gameId}</td>
                  <td>{r ? `${r.homeScore}–${r.awayScore}` : '—'}</td>
                  <td>
                    <PointsBadge breakdown={p.breakdown} isPerfect />
                  </td>
                </tr>
              )
            })}
          </tbody>
        </table>
      )}
    </section>
  )
}
