import { useEffect, useMemo, useRef } from 'react'
import { debounce, type Debounced } from '../lib/debounce'

/**
 * A stable debounced callback exposing `call` / `cancel` / `flush`. The latest
 * `fn` is always invoked (via a ref), so callers can pass a fresh closure each
 * render without resetting the timer.
 *
 * The debounce instance is built inside an effect (ref reads there are
 * deferred, never during render). A pending call is FLUSHED — not dropped — on
 * unmount / delay change, so an in-flight edit is never silently lost (e.g.
 * swiping to the next group right after a stepper tap). Callers that must drop
 * a pending call (e.g. just before finalize/lock) call `cancel` explicitly.
 */
export function useDebouncedCallback<A extends unknown[]>(
  fn: (...args: A) => void,
  delayMs: number,
): Debounced<A> {
  const fnRef = useRef(fn)
  useEffect(() => {
    fnRef.current = fn
  }, [fn])

  const debouncedRef = useRef<Debounced<A> | null>(null)
  useEffect(() => {
    const d = debounce((...args: A) => fnRef.current(...args), delayMs)
    debouncedRef.current = d
    return () => {
      d.flush()
      debouncedRef.current = null
    }
  }, [delayMs])

  // A stable handle that delegates to the current debounce instance. Reads of
  // `debouncedRef.current` happen only when these methods are invoked (events /
  // effects), never during render.
  return useMemo<Debounced<A>>(
    () => ({
      call: (...args: A) => debouncedRef.current?.call(...args),
      cancel: () => debouncedRef.current?.cancel(),
      flush: () => debouncedRef.current?.flush(),
    }),
    [],
  )
}
