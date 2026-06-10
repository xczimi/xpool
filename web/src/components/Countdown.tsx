import { useEffect, useRef } from 'react'
import { useI18n } from '../i18n/useI18n'
import {
  formatAbsoluteDeadline,
  formatRelative,
  remainingMs,
} from '../lib/countdown'

interface CountdownProps {
  /** ISO deadline; `null` renders nothing (no known deadline). */
  deadline: string | null
  /** Estimated server-now in ms (from `useServerClock`); 0 = clock not ready. */
  serverNowMs: number
  /** Fired once when the countdown crosses zero — wire to a refetch so the
   *  server's `deadlinePassed` takes over as the authority for locking. */
  onExpire?: () => void
  className?: string
}

/**
 * A live finalize-deadline display (My Tips). Shows the absolute deadline in the
 * viewer's local time plus a relative hint whose granularity scales to urgency
 * (see `lib/countdown.formatRelative`) — only the final hour ticks per second.
 * Display-only: on expiry it shows the closed label and signals `onExpire`; it
 * never decides locking itself.
 */
export function Countdown({
  deadline,
  serverNowMs,
  onExpire,
  className,
}: CountdownProps) {
  const { t, locale } = useI18n()
  const firedRef = useRef(false)

  const remaining =
    deadline && serverNowMs ? remainingMs(deadline, serverNowMs) : null
  const expired = remaining !== null && remaining <= 0

  useEffect(() => {
    if (expired && !firedRef.current) {
      firedRef.current = true
      onExpire?.()
    }
    if (!expired) firedRef.current = false
  }, [expired, onExpire])

  if (remaining === null || deadline === null) return null
  if (expired) return <span className={className}>{t('finalizeClosed')}</span>

  const absolute = formatAbsoluteDeadline(deadline, serverNowMs, locale)
  const relative = formatRelative(remaining, locale)
  return (
    <span className={className}>
      {absolute} — {relative}
    </span>
  )
}
