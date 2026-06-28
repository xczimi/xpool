import { useEffect, useMemo, useRef } from 'react'
import { debounce } from '../lib/debounce'

/**
 * A stable debounced callback. The latest `fn` is always invoked (via a ref),
 * so callers can pass a fresh closure each render without resetting the timer.
 * The pending call is cancelled on unmount / delay change.
 */
export function useDebouncedCallback<A extends unknown[]>(
  fn: (...args: A) => void,
  delayMs: number,
): (...args: A) => void {
  const fnRef = useRef(fn)
  useEffect(() => {
    fnRef.current = fn
  }, [fn])

  const debounced = useMemo(
    () => debounce((...args: A) => fnRef.current(...args), delayMs),
    [delayMs],
  )

  useEffect(() => debounced.cancel, [debounced])

  return debounced.call
}
