import { useState, type ReactNode } from 'react'
import { useI18n } from '../i18n/useI18n'

/**
 * A button that asks for confirmation inline before running an irreversible
 * action — the in-app replacement for `window.confirm()`. The first click
 * "arms" it: the trigger is swapped for a "{question} [Confirm] [Cancel]" bar.
 * Confirm runs the action; Cancel disarms. Matches the app's inline-form style
 * (no modal/portal).
 */
export function InlineConfirm({
  children,
  question,
  onConfirm,
  className,
  confirmClassName = 'danger',
  disabled,
  title,
}: {
  /** Trigger label. */
  children: ReactNode
  /** Prompt shown while armed — plain text or composed JSX. */
  question: ReactNode
  onConfirm: () => void
  className?: string
  /** Class for the Confirm button (defaults to `danger`). */
  confirmClassName?: string
  disabled?: boolean
  title?: string
}) {
  const { t } = useI18n()
  const [armed, setArmed] = useState(false)

  if (armed) {
    return (
      <span className="inline-confirm" role="group">
        <span className="inline-confirm-q">{question}</span>
        <button
          type="button"
          className={confirmClassName}
          onClick={() => {
            setArmed(false)
            onConfirm()
          }}
        >
          {t('confirmAction')}
        </button>
        <button
          type="button"
          className="link-button"
          onClick={() => setArmed(false)}
        >
          {t('cancel')}
        </button>
      </span>
    )
  }

  return (
    <button
      type="button"
      className={className}
      disabled={disabled}
      title={title}
      onClick={() => setArmed(true)}
    >
      {children}
    </button>
  )
}
