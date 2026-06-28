import { useCallback, useEffect, useRef } from 'react'
import { debounce, type Debounced } from '../lib/debounce'

/**
 * A stable debounced callback. The latest `fn` is always invoked (via a ref),
 * so callers can pass a fresh closure each render without resetting the timer.
 * The pending call is cancelled on unmount / delay change.
 *
 * The debounce instance is built inside an effect (ref reads there are
 * deferred, never during render) and the returned function delegates to it, so
 * the public callback identity is stable across renders.
 */
export function useDebouncedCallback<A extends unknown[]>(
  fn: (...args: A) => void,
  delayMs: number,
): (...args: A) => void {
  const fnRef = useRef(fn)
  useEffect(() => {
    fnRef.current = fn
  }, [fn])

  const debouncedRef = useRef<Debounced<A> | null>(null)
  useEffect(() => {
    const d = debounce((...args: A) => fnRef.current(...args), delayMs)
    debouncedRef.current = d
    return () => {
      d.cancel()
      debouncedRef.current = null
    }
  }, [delayMs])

  return useCallback((...args: A) => {
    debouncedRef.current?.call(...args)
  }, [])
}
