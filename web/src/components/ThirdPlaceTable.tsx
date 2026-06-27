import { useI18n } from '../i18n/useI18n'
import type { ThirdPlaceRanking } from '../graphql/types'

/**
 * Read-only best-third-placed-teams table (FWC26_RULES §3). The top-8 rows are
 * highlighted as qualifiers; each qualifier shows its R32 pairing (resolved via
 * Annexe C). Purely presentational — the page owns the query.
 */
export function ThirdPlaceTable({
  title,
  ranking,
}: {
  title: string
  ranking: ThirdPlaceRanking | null
}) {
  const { t } = useI18n()

  if (!ranking || ranking.entries.length === 0) {
    return (
      <div className="third-place">
        <h4>{title}</h4>
        <p className="hint">{t('thirdsPending')}</p>
      </div>
    )
  }

  return (
    <div className="third-place">
      <h4>{title}</h4>
      {!ranking.complete && <p className="hint">{t('thirdsProvisional')}</p>}
      <table className="data-table compact third-place-table">
        <thead>
          <tr>
            <th>{t('thirdsRank')}</th>
            <th>{t('thirdsGroup')}</th>
            <th>{t('thirdsTeam')}</th>
            <th className="num">{t('thirdsPts')}</th>
            <th className="num">{t('thirdsGd')}</th>
            <th className="num">{t('thirdsGf')}</th>
            <th>{t('thirdsFaces')}</th>
          </tr>
        </thead>
        <tbody>
          {ranking.entries.map((e) => (
            <tr
              key={e.group}
              className={e.qualifies ? 'qualifies' : 'eliminated'}
            >
              <td>{e.rank}</td>
              <td>{e.group}</td>
              <td>
                {e.team.flag ? `${e.team.flag} ` : ''}
                {e.team.name}
              </td>
              <td className="num">{e.points}</td>
              <td className="num">{e.goalDiff}</td>
              <td className="num">{e.goalsFor}</td>
              <td>
                {e.qualifies && e.facesWinnerGroup
                  ? `${t('thirdsWinnerPrefix')} ${e.facesWinnerGroup}`
                  : e.qualifies
                    ? t('thirdsQualifies')
                    : '—'}
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
