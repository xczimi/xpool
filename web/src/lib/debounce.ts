export interface Debounced<A extends unknown[]> {
  call: (...args: A) => void
  cancel: () => void
}

/**
 * Trailing-edge debounce: `call` schedules `fn` after `delayMs`, replacing any
 * pending invocation so only the latest args fire. Pure (no React) so it is
 * unit-testable with fake timers; the `useDebouncedCallback` hook wraps it.
 */
export function debounce<A extends unknown[]>(
  fn: (...args: A) => void,
  delayMs: number,
): Debounced<A> {
  let timer: ReturnType<typeof setTimeout> | null = null
  const cancel = () => {
    if (timer !== null) {
      clearTimeout(timer)
      timer = null
    }
  }
  const call = (...args: A) => {
    cancel()
    timer = setTimeout(() => {
      timer = null
      fn(...args)
    }, delayMs)
  }
  return { call, cancel }
}
