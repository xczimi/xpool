import { useEffect, useRef, useState } from 'react'
import { clockSkewMs } from './countdown'

/**
 * The estimated server-now in milliseconds, ticking once a second.
 *
 * Anchored to the GraphQL `now` (`serverNowIso`) so the value tracks the
 * SERVER clock, not the browser's: we capture the skew between server and
 * client once, then add it to `Date.now()` on each tick. This is the only
 * `Date.now()` in the app, and it is purely cosmetic — a tick source for the
 * countdown display, never a gate on locking (that stays server-authoritative
 * via `deadlinePassed`). Returns 0 until a server time is available.
 */
export function useServerClock(serverNowIso: string | null | undefined): number {
  const skewRef = useRef(0)
  const [nowMs, setNowMs] = useState(0)

  useEffect(() => {
    if (!serverNowIso) return
    skewRef.current = clockSkewMs(serverNowIso, Date.now())
    setNowMs(Date.now() + skewRef.current)
    const id = setInterval(() => setNowMs(Date.now() + skewRef.current), 1000)
    return () => clearInterval(id)
  }, [serverNowIso])

  return nowMs
}
