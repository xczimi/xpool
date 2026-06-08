import { useI18n } from '../i18n/useI18n'
import type { StandingsScore } from '../graphql/types'

/**
 * A player's standings (group-table) bonus across one or more groups, shown as
 * a total with a hover/tap tooltip breaking it down per group: pairs correct
 * and the multiplier arithmetic. Renders an em dash when there's nothing scored
 * yet (no group carries a scoreable standings bonus for this player).
 */
export function StandingsBadge({
  scores,
  groupLabel,
}: {
  scores: StandingsScore[]
  /** Resolve a group id to a display name (for the per-group tooltip lines). */
  groupLabel?: (groupId: string) => string
}) {
  const { t } = useI18n()
  if (scores.length === 0) return <span className="pts-empty">—</span>

  const total = scores.reduce((sum, s) => sum + s.points, 0)
  const multi = scores.length > 1
  const aria = scores
    .map(
      (s) =>
        `${groupLabel?.(s.groupId) ?? s.groupId}: ${s.pairsCorrect}/${s.pairsTotal} ${t('pairsCorrect')} = ${s.points}`,
    )
    .join('; ')

  return (
    <span className="points-badge" tabIndex={0} aria-label={aria}>
      <span className="pts-value">{total}</span>
      <span className="pts-tip" role="tooltip">
        {scores.map((s) => (
          <span className="pts-tip-row" key={s.groupId}>
            {multi && (
              <span className="pts-tip-label">
                {groupLabel?.(s.groupId) ?? s.groupId}
              </span>
            )}
            <span className="pts-tip-label">
              {s.pairsCorrect}/{s.pairsTotal} {t('pairsCorrect')}
            </span>
            <span className="pts-tip-pts">
              {s.bonus} × {s.multiplier} = {s.points}
            </span>
          </span>
        ))}
        {multi && <span className="pts-tip-row pts-tip-total">{total}</span>}
      </span>
    </span>
  )
}
