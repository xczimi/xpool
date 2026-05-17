import { useMemo } from 'react'
import { useQuery } from 'urql'
import { useI18n } from '../i18n/useI18n'
import { PERFECTS_QUERY, TOURNAMENT_QUERY } from '../graphql/queries'
import type { Perfect, Tournament } from '../graphql/types'
import { ErrorView, Loading } from '../components/StatusViews'
import { slotLabel, teamIndex } from '../lib/format'

/** Players who scored a maximum (4-point) match prediction (UC-10). Public. */
export function PerfectPage() {
  const { t } = useI18n()
  const [result, reexecute] = useQuery<{ perfects: Perfect[] }>({
    query: PERFECTS_QUERY,
  })
  const [tournamentResult] = useQuery<{
    tournament: Tournament | null
    motd: string | null
  }>({ query: TOURNAMENT_QUERY })

  const tournament = tournamentResult.data?.tournament ?? null
  const teams = useMemo(
    () => teamIndex(tournament?.teams ?? []),
    [tournament],
  )
  const gameLabel = useMemo(() => {
    const map = new Map<string, string>()
    for (const g of tournament?.games ?? []) {
      map.set(g.id, `${slotLabel(g.home, teams)} – ${slotLabel(g.away, teams)}`)
    }
    return map
  }, [tournament, teams])

  if (result.fetching) return <Loading />
  if (result.error)
    return (
      <ErrorView
        message={result.error.message}
        onRetry={() => reexecute({ requestPolicy: 'network-only' })}
      />
    )

  const perfects = result.data?.perfects ?? []

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
              <th>{t('match')}</th>
            </tr>
          </thead>
          <tbody>
            {perfects.map((p, i) => (
              <tr key={`${p.playerId}-${p.gameId}-${i}`}>
                <td>{p.nick}</td>
                <td>{gameLabel.get(p.gameId) ?? p.gameId}</td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </section>
  )
}
