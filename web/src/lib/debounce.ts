export interface Debounced<A extends unknown[]> {
  call: (...args: A) => void
  cancel: () => void
  flush: () => void
}

/**
 * Trailing-edge debounce: `call` schedules `fn` after `delayMs`, replacing any
 * pending invocation so only the latest args fire. `cancel` drops a pending
 * call; `flush` fires it immediately. Pure (no React) so it is unit-testable
 * with fake timers; the `useDebouncedCallback` hook wraps it.
 */
export function debounce<A extends unknown[]>(
  fn: (...args: A) => void,
  delayMs: number,
): Debounced<A> {
  let timer: ReturnType<typeof setTimeout> | null = null
  let pending: A | null = null

  const cancel = () => {
    if (timer !== null) {
      clearTimeout(timer)
      timer = null
    }
    pending = null
  }

  const flush = () => {
    if (timer === null) return
    clearTimeout(timer)
    timer = null
    const args = pending
    pending = null
    if (args) fn(...args)
  }

  const call = (...args: A) => {
    if (timer !== null) clearTimeout(timer)
    pending = args
    timer = setTimeout(() => {
      timer = null
      const next = pending
      pending = null
      if (next) fn(...next)
    }, delayMs)
  }

  return { call, cancel, flush }
}
