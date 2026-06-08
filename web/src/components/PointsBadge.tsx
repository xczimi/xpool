import { useI18n } from '../i18n/useI18n'
import type { PointsBreakdown } from '../graphql/types'

/** One scored/not-scored component mark (exact home / away / outcome). */
function Mark({ on, label }: { on: boolean; label: string }) {
  return (
    <span className={`pts-mark${on ? ' on' : ''}`} title={label}>
      {on ? '✓' : '·'}
    </span>
  )
}

/**
 * The points a single prediction earned, made transparent: always-visible
 * component marks (exact home / exact away / outcome) and the points value,
 * with the full labelled arithmetic in a hover/tap tooltip. A "perfect" gets a
 * star and a brighter treatment.
 *
 * Renders nothing until the game has a result (`points == null`), so callers
 * can drop it straight into a cell without guarding.
 */
export function PointsBadge({
  breakdown,
  points,
  isPerfect,
}: {
  breakdown?: PointsBreakdown | null
  /** Fallback when no breakdown is supplied. */
  points?: number | null
  isPerfect?: boolean
}) {
  const { t } = useI18n()
  const value = breakdown?.points ?? points
  if (value == null) return null

  const rows: Array<[boolean, string, number]> = breakdown
    ? [
        [breakdown.exactHome, t('exactHome'), 1],
        [breakdown.exactAway, t('exactAway'), 1],
        [breakdown.outcome, t('outcomeResult'), 2],
      ]
    : []

  // A concise screen-reader / native-tooltip summary mirroring the visual tip.
  const ariaParts = rows.map(
    ([on, label, pts]) => `${label}: ${on ? `+${pts}` : '0'}`,
  )
  if (breakdown) {
    ariaParts.push(
      `${t('base')} ${breakdown.base} × ${breakdown.multiplier} = ${breakdown.points}`,
    )
  }

  return (
    <span
      className={`points-badge${isPerfect ? ' is-perfect' : ''}`}
      tabIndex={0}
      aria-label={ariaParts.join('; ') || String(value)}
    >
      <span className="pts-value">
        {isPerfect && <span aria-hidden="true">★</span>}
        {value}
      </span>
      {rows.length > 0 && (
        <span className="pts-marks" aria-hidden="true">
          {rows.map(([on, label], i) => (
            <Mark key={i} on={on} label={label} />
          ))}
        </span>
      )}
      {breakdown && (
        <span className="pts-tip" role="tooltip">
          {rows.map(([on, label, pts], i) => (
            <span className="pts-tip-row" key={i}>
              <span className={`pts-mark${on ? ' on' : ''}`}>
                {on ? '✓' : '·'}
              </span>
              <span className="pts-tip-label">{label}</span>
              <span className="pts-tip-pts">{on ? `+${pts}` : '0'}</span>
            </span>
          ))}
          <span className="pts-tip-row pts-tip-total">
            {t('base')} {breakdown.base} × {breakdown.multiplier} ={' '}
            {breakdown.points}
          </span>
        </span>
      )}
    </span>
  )
}
