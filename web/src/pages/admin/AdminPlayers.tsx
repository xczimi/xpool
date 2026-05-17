import { useQuery } from 'urql'
import { useI18n } from '../../i18n/useI18n'
import { SCOREBOARD_QUERY } from '../../graphql/queries'
import type { Scoreboard } from '../../graphql/types'
import { ErrorView, Loading } from '../../components/StatusViews'

/**
 * Admin player listing (UC-16). Sourced from the scoreboard entries — the
 * dedicated `players` admin query lands with P5.
 */
export function AdminPlayers() {
  const { t } = useI18n()
  const [result] = useQuery<{ scoreboard: Scoreboard | null }>({
    query: SCOREBOARD_QUERY,
    variables: { pool: null },
  })

  if (result.fetching) return <Loading />
  if (result.error) return <ErrorView message={result.error.message} />
  const entries = result.data?.scoreboard?.entries ?? []

  return (
    <div>
      <h3>{t('adminPlayers')}</h3>
      <table className="data-table">
        <thead>
          <tr>
            <th>Id</th>
            <th>{t('nick')}</th>
            <th>{t('total')}</th>
          </tr>
        </thead>
        <tbody>
          {entries.map((entry) => (
            <tr key={entry.playerId}>
              <td>{entry.playerId}</td>
              <td>{entry.nick}</td>
              <td>{entry.total}</td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
