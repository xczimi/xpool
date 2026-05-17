import { useMemo } from 'react'
import { useQuery } from 'urql'
import { useI18n } from '../i18n/useI18n'
import { TOURNAMENT_QUERY } from '../graphql/queries'
import type { Motd, Tournament } from '../graphql/types'
import { ErrorView, Loading } from '../components/StatusViews'
import { byKickoff, formatKickoff, slotLabel, teamIndex } from '../lib/format'
import { ROUND_LABELS } from '../lib/rounds'

/** Full fixture list, grouped by tournament group (UC-12). Public, read-only. */
export function SchedulePage() {
  const { t, locale } = useI18n()
  const [result, reexecute] = useQuery<{
    tournament: Tournament | null
    motd: Motd | null
  }>({ query: TOURNAMENT_QUERY })

  const tournament = result.data?.tournament ?? null
  const teams = useMemo(
    () => teamIndex(tournament?.teams ?? []),
    [tournament],
  )

  if (result.fetching) return <Loading />
  if (result.error)
    return (
      <ErrorView
        message={result.error.message}
        onRetry={() => reexecute({ requestPolicy: 'network-only' })}
      />
    )
  if (!tournament) return <ErrorView />

  const leafGroups = tournament.groups.filter((g) => g.gameIds.length > 0)

  return (
    <section className="page">
      <h2>{t('scheduleTitle')}</h2>
      {leafGroups.map((group) => {
        const games = tournament.games
          .filter((m) => group.gameIds.includes(m.id))
          .sort(byKickoff)
        return (
          <div key={group.id} className="schedule-group">
            <h3>
              {group.name}{' '}
              <span className="round-tag">{ROUND_LABELS[group.round]}</span>
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
                {games.map((m) => (
                  <tr key={m.id}>
                    <td>{formatKickoff(m.kickoff, locale)}</td>
                    <td>
                      {slotLabel(m.home, teams)} – {slotLabel(m.away, teams)}
                    </td>
                    <td>{m.venue ?? '—'}</td>
                    <td>
                      {m.result
                        ? `${m.result.homeScore}–${m.result.awayScore}`
                        : '—'}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )
      })}
    </section>
  )
}
