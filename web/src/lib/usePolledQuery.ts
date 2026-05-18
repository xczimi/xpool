import { useEffect } from 'react'
import { useQuery, type AnyVariables, type UseQueryArgs } from 'urql'

/**
 * A urql query that re-executes on an interval, but only while `intervalMs`
 * is non-zero (smart polling, API.md §7). When `intervalMs` is 0 the data is
 * treated as static — one fetch on load.
 */
export function usePolledQuery<
  Data = unknown,
  Variables extends AnyVariables = AnyVariables,
>(args: UseQueryArgs<Variables, Data>, intervalMs: number) {
  const [result, reexecute] = useQuery<Data, Variables>(args)

  useEffect(() => {
    if (intervalMs <= 0) {
      return
    }
    const id = setInterval(() => {
      reexecute({ requestPolicy: 'network-only' })
    }, intervalMs)
    return () => clearInterval(id)
  }, [intervalMs, reexecute])

  return [result, reexecute] as const
}
