import { useI18n } from '../../i18n/useI18n'
import type { Team } from '../../graphql/types'
import { type TeamStats, goalDiff } from '../../lib/standings'
import { TeamLabel } from '../../components/TeamLabel'

/** A read-only standings table. Knockout one-match groups use a simplified
 *  two-team layout (✓ advances · team · goals) instead of P/GD/Pts. */
export function StandingsTable({
  title,
  rows,
  teams,
  isKnockout = false,
}: {
  title: string
  rows: TeamStats[]
  teams: Map<string, Team>
  isKnockout?: boolean
}) {
  return (
    <div className="standings">
      <h4>{title}</h4>
      <table className="data-table compact">
        <thead>
          <tr>
            <th>#</th>
            <th>Team</th>
            {isKnockout ? (
              <th>Goals</th>
            ) : (
              <>
                <th>P</th>
                <th>GD</th>
                <th>Pts</th>
              </>
            )}
          </tr>
        </thead>
        <tbody>
          {rows.map((s, i) => (
            <tr key={s.teamId}>
              <td>{isKnockout ? (i === 0 ? '✓' : '') : i + 1}</td>
              <td>
                <TeamLabel
                  slot={{
                    teamId: s.teamId,
                    description: teams.get(s.teamId)?.name ?? s.teamId,
                  }}
                  teams={teams}
                />
              </td>
              {isKnockout ? (
                <td>{s.goalsFor}</td>
              ) : (
                <>
                  <td>{s.played}</td>
                  <td>{goalDiff(s)}</td>
                  <td>{s.points}</td>
                </>
              )}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}

/**
 * Editable predicted-standings table — lets the player manually order tied
 * teams (the `draw_order`, UC-6 / SCORING.md §4 step 5). Move up/down buttons.
 * For knockout one-match groups it reframes as "who advances on ET/penalties"
 * with a simplified two-team layout.
 */
export function PredictedStandingsEditor({
  rows,
  teams,
  readOnly,
  onReorder,
  isKnockout = false,
}: {
  rows: TeamStats[]
  teams: Map<string, Team>
  readOnly: boolean
  onReorder: (orderedTeamIds: string[]) => void
  isKnockout?: boolean
}) {
  const { t } = useI18n()

  const move = (index: number, delta: number) => {
    const next = [...rows.map((r) => r.teamId)]
    const target = index + delta
    if (target < 0 || target >= next.length) return
    ;[next[index], next[target]] = [next[target], next[index]]
    onReorder(next)
  }

  return (
    <div className="standings">
      <h4>{t(isKnockout ? 'koPredictedTitle' : 'predictedStandings')}</h4>
      {!readOnly && (
        <p className="hint">{t(isKnockout ? 'koAdvanceHint' : 'drawOrderHint')}</p>
      )}
      <table className="data-table compact">
        <thead>
          <tr>
            <th>#</th>
            <th>Team</th>
            {isKnockout ? (
              <th>Goals</th>
            ) : (
              <>
                <th>P</th>
                <th>GD</th>
                <th>Pts</th>
              </>
            )}
            {!readOnly && <th />}
          </tr>
        </thead>
        <tbody>
          {rows.map((s, i) => (
            <tr key={s.teamId}>
              <td>{isKnockout ? (i === 0 ? '✓' : '') : i + 1}</td>
              <td>
                <TeamLabel
                  slot={{
                    teamId: s.teamId,
                    description: teams.get(s.teamId)?.name ?? s.teamId,
                  }}
                  teams={teams}
                />
              </td>
              {isKnockout ? (
                <td>{s.goalsFor}</td>
              ) : (
                <>
                  <td>{s.played}</td>
                  <td>{goalDiff(s)}</td>
                  <td>{s.points}</td>
                </>
              )}
              {!readOnly && (
                <td className="reorder">
                  <button
                    type="button"
                    aria-label={t('moveUp')}
                    disabled={i === 0}
                    onClick={() => move(i, -1)}
                  >
                    ▲
                  </button>
                  <button
                    type="button"
                    aria-label={t('moveDown')}
                    disabled={i === rows.length - 1}
                    onClick={() => move(i, 1)}
                  >
                    ▼
                  </button>
                </td>
              )}
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  )
}
