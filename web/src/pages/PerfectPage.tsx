import { useMemo, useState, type ReactNode } from 'react'
import { Link } from 'react-router-dom'
import { useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import {
  PERFECTS_QUERY,
  POOLS_QUERY,
  RESULTS_QUERY,
  TOURNAMENT_QUERY,
} from '../graphql/queries'
import type { MatchPrediction, Perfect, Pool, Tournament } from '../graphql/types'
import { ErrorView, Loading } from '../components/StatusViews'
import { teamIndex } from '../lib/format'
import { Matchup } from '../components/TeamLabel'
import { PointsBadge } from '../components/PointsBadge'
import { PoolSelector } from '../pools/PoolSelector'
import { useSelectedPool } from '../pools/useSelectedPool'
import { effectiveSelectedPool } from '../lib/selectedPool'
import {
  orderPerfects,
  readPerfectView,
  writePerfectView,
  type PerfectView,
} from '../lib/perfectOrder'

/** Players who scored a maximum (4-point) match prediction (UC-10). Public. */
export function PerfectPage() {
  const { t, locale } = useI18n()
  const { label } = useAuth()
  const { selected } = useSelectedPool()
  const [view, setView] = useState<PerfectView>(readPerfectView)

  // Pools require auth; PerfectPage is public, so pause for visitors — they
  // see the global list (effectivePool resolves to null with no pools).
  const [poolsResult] = useQuery<{ pools: Pool[] }>({
    query: POOLS_QUERY,
    pause: !label,
  })
  const pools = poolsResult.data?.pools ?? []
  const effectivePool = effectiveSelectedPool(
    selected,
    pools.map((p) => p.id),
  )

  const [result, reexecute] = useQuery<{ perfects: Perfect[] }>({
    query: PERFECTS_QUERY,
    variables: { pool: effectivePool },
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
  // gameId -> kickoff epoch, for the ordering helper (server-provided times;
  // Date.parse is formatting a server timestamp, not a behavioural clock read).
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

  const perfects = useMemo(
    () => orderPerfects(result.data?.perfects ?? [], view, kickoffOf),
    [result.data, view, kickoffOf],
  )

  const chooseView = (next: PerfectView) => {
    writePerfectView(next)
    setView(next)
  }

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

      <PoolSelector pools={pools} />

      <div className="seg-toggle" role="group" aria-label={t('perfectTitle')}>
        <button
          type="button"
          className={`seg-option${view === 'match' ? ' is-active' : ''}`}
          aria-pressed={view === 'match'}
          onClick={() => chooseView('match')}
        >
          {t('perfectByMatch')}
        </button>
        <button
          type="button"
          className={`seg-option${view === 'player' ? ' is-active' : ''}`}
          aria-pressed={view === 'player'}
          onClick={() => chooseView('player')}
        >
          {t('perfectByPlayer')}
        </button>
      </div>

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
