import { useI18n } from '../../i18n/useI18n'
import { stepScore } from '../../lib/score'

/**
 * Big thumb-friendly +/− score stepper for the mobile entry flow, replacing the
 * tiny 0–9 `<select>`. `null` renders as `–` (not yet entered). All numeric
 * clamping lives in the pure `stepScore` helper.
 */
export function ScoreStepper({
  value,
  onChange,
}: {
  value: number | null
  onChange: (next: number | null) => void
}) {
  const { t } = useI18n()
  return (
    <span className="score-stepper">
      <button
        type="button"
        className="score-stepper-dec"
        aria-label={t('decScore')}
        onClick={() => onChange(stepScore(value, -1))}
      >
        −
      </button>
      <span className="score-stepper-value">{value === null ? '–' : value}</span>
      <button
        type="button"
        className="score-stepper-inc"
        aria-label={t('incScore')}
        onClick={() => onChange(stepScore(value, +1))}
      >
        +
      </button>
    </span>
  )
}
