import { useEffect, useMemo, useState } from 'react'
import { useQuery } from 'urql'
import { useAuth } from '../auth/useAuth'
import { useI18n } from '../i18n/useI18n'
import { ME_QUERY, RESULTS_QUERY, TOURNAMENT_QUERY } from '../graphql/queries'
import type {
  MatchPrediction,
  Player,
  Tournament,
} from '../graphql/types'
import { ErrorView, Loading } from '../components/StatusViews'
import { usePolledQuery } from '../lib/usePolledQuery'
import { pollIntervalMs } from '../lib/polling'
import { byKickoff, formatKickoff, slotLabel, teamIndex } from '../lib/format'

const WINDOW_MS = 2 * 24 * 60 * 60 * 1000

/**
 * Today / Fresh — a flat list of matches within ~±2 days of now (UC-11).
 * Polls only while a match is result-pending (API.md §7). Logged-in players
 * also see their prediction per match.
 */
export function TodayPage() {
  const { t, locale } = useI18n()
  const { playerId } = useAuth()
  // Snapshot "now" once per mount so render stays pure (smart polling drives
  // refresh; the ±2-day window does not need second-precision).
  const [now] = useState(() => Date.now())

  // First fetch (no polling) to learn whether anything is result-pending.
  const [probe] = useQuery<{
    tournament: Tournament | null
    motd: string | null
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
    () =>
      pollIntervalMs(
        probe.data?.tournament?.games ?? [],
        new Set(resultsByGame.keys()),
      ),
    [probe.data, resultsByGame],
  )
  const [result, reexecute] = usePolledQuery<{
    tournament: Tournament | null
    motd: string | null
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

  const [meResult] = useQuery<{ me: Player | null }>({
    query: ME_QUERY,
    pause: !playerId,
  })

  const tournament = result.data?.tournament ?? null
  const teams = useMemo(
    () => teamIndex(tournament?.teams ?? []),
    [tournament],
  )
  const tipFor = useMemo(() => {
    const map = new Map<string, { home: number; away: number; locked: boolean }>()
    for (const p of meResult.data?.me?.matchPredictions ?? []) {
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
    .filter((g) => Math.abs(Date.parse(g.kickoff) - now) <= WINDOW_MS)
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
              <th>{t('match')}</th>
              <th>{t('result')}</th>
              {playerId && <th>{t('yourTip')}</th>}
            </tr>
          </thead>
          <tbody>
            {games.map((m) => {
              const tip = tipFor.get(m.id)
              return (
                <tr key={m.id}>
                  <td>{formatKickoff(m.kickoff, locale)}</td>
                  <td>
                    {slotLabel(m.home, teams)} – {slotLabel(m.away, teams)}
                  </td>
                  <td>
                    {(() => {
                      const r = resultsByGame.get(m.id)
                      return r ? `${r.homeScore}–${r.awayScore}` : '—'
                    })()}
                  </td>
                  {playerId && (
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
