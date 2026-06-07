import { useMemo } from 'react'
import { useQuery } from 'urql'
import { useI18n } from '../i18n/useI18n'
import { RESULTS_QUERY, TOURNAMENT_QUERY } from '../graphql/queries'
import type { MatchPrediction, Tournament } from '../graphql/types'
import { ErrorView, Loading } from '../components/StatusViews'
import { byKickoff, formatKickoff, teamIndex } from '../lib/format'
import { Matchup } from '../components/TeamLabel'
import { roundLabel } from '../lib/rounds'

/** Full fixture list, grouped by tournament group (UC-12). Public, read-only. */
export function SchedulePage() {
  const { t, locale } = useI18n()
  const [result, reexecute] = useQuery<{
    tournament: Tournament | null
  }>({ query: TOURNAMENT_QUERY })
  const [resultsResult] = useQuery<{ results: MatchPrediction[] }>({
    query: RESULTS_QUERY,
  })

  const tournament = result.data?.tournament ?? null
  const teams = useMemo(
    () => teamIndex(tournament?.teams ?? []),
    [tournament],
  )
  const resultsByGame = useMemo(() => {
    const map = new Map<string, MatchPrediction>()
    for (const r of resultsResult.data?.results ?? []) {
      map.set(r.gameId, r)
    }
    return map
  }, [resultsResult.data])

  if (result.fetching) return <Loading />
  if (result.error)
    return (
      <ErrorView
        message={result.error.message}
        onRetry={() => reexecute({ requestPolicy: 'network-only' })}
      />
    )
  if (!tournament) return <ErrorView />

  // Leaf groups (those holding matches), ordered chronologically by their
  // deadline — the earliest kickoff in the group — so the schedule reads in
  // time order: group stage A–L, then each knockout round.
  const leafGroups = tournament.groups
    .filter((g) => g.childGameIds.length > 0)
    .sort((a, b) => {
      const da = a.deadline ? Date.parse(a.deadline) : Number.POSITIVE_INFINITY
      const db = b.deadline ? Date.parse(b.deadline) : Number.POSITIVE_INFINITY
      return da - db
    })

  return (
    <section className="page">
      <h2>{t('scheduleTitle')}</h2>
      {leafGroups.map((group) => {
        const games = tournament.games
          .filter((m) => group.childGameIds.includes(m.id))
          .sort(byKickoff)
        return (
          <div key={group.id} className="schedule-group">
            <h3>
              {group.name}{' '}
              <span className="round-tag">{roundLabel(group.round, t)}</span>
            </h3>
            <table className="data-table">
              <thead>
                <tr>
                  <th>{t('kickoff')}</th>
                  <th>{t('match')}</th>
                  <th>{t('venue')}</th>
                  <th>{t('result')}</th>
                </tr>
              </thead>
              <tbody>
                {games.map((m) => {
                  const r = resultsByGame.get(m.id)
                  return (
                    <tr key={m.id}>
                      <td>{formatKickoff(m.kickoff, locale)}</td>
                      <td>
                        <Matchup home={m.home} away={m.away} teams={teams} />
                      </td>
                      <td>{m.venue ?? '—'}</td>
                      <td>{r ? `${r.homeScore}–${r.awayScore}` : '—'}</td>
                    </tr>
                  )
                })}
              </tbody>
            </table>
          </div>
        )
      })}
    </section>
  )
}
