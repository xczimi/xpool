import { useEffect, useRef } from 'react'
import { useI18n } from '../i18n/useI18n'
import { formatCountdown, remainingMs } from '../lib/countdown'

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
 * A live-ticking countdown to a finalize deadline (My Tips). Display-only: it
 * renders the server-anchored remaining time, and on expiry shows the closed
 * label and signals `onExpire`. It never decides locking itself.
 */
export function Countdown({
  deadline,
  serverNowMs,
  onExpire,
  className,
}: CountdownProps) {
  const { t } = useI18n()
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

  if (remaining === null) return null
  return (
    <span className={className}>
      {expired ? t('finalizeClosed') : formatCountdown(remaining)}
    </span>
  )
}
