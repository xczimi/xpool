import { useQuery } from 'urql'
import { useI18n } from '../../i18n/useI18n'
import { TOURNAMENT_QUERY } from '../../graphql/queries'
import type { Tournament } from '../../graphql/types'
import { ErrorView, Loading } from '../../components/StatusViews'

/** Admin team view (UC-15) — read-only listing. */
export function AdminTeams() {
  const { t } = useI18n()
  const [result] = useQuery<{
    tournament: Tournament | null
  }>({ query: TOURNAMENT_QUERY })

  if (result.fetching) return <Loading />
  if (result.error) return <ErrorView message={result.error.message} />
  const teams = result.data?.tournament?.teams ?? []

  return (
    <div>
      <h3>{t('adminTeams')}</h3>
      <table className="data-table">
        <thead>
          <tr>
            <th>{t('team')}</th>
            <th>Code</th>
            <th>External id</th>
          </tr>
        </thead>
        <tbody>
          {[...teams]
            .sort((a, b) => a.name.localeCompare(b.name))
            .map((team) => (
              <tr key={team.id}>
                <td>
                  {team.flag ? `${team.flag} ` : ''}
                  {team.name}
                </td>
                <td>{team.shortCode}</td>
                <td>{team.externalId ?? '—'}</td>
              </tr>
            ))}
        </tbody>
      </table>
    </div>
  )
}
