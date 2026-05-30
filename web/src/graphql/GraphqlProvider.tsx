import { useMemo, type ReactNode } from 'react'
import { Provider as UrqlProvider } from 'urql'
import { useAuth } from '../auth/useAuth'
import { createGraphqlClient } from './client'

/**
 * Provides the urql client, recreating it whenever the active player changes.
 *
 * The auth seam is an `Authorization: Bearer` header, not a GraphQL variable —
 * so urql's document cache cannot tell two players apart and would serve one
 * player's `me`/`pools`/`tips` to another after a dev-login switch. Rebuilding
 * the client on every `label` change gives each identity a fresh cache.
 */
export function GraphqlProvider({ children }: { children: ReactNode }) {
  const { label } = useAuth()
  // `label` in the dep array → a new client (and empty cache) per identity.
  // It is deliberately a dependency even though the factory does not read it:
  // changing identity must drop the cache. eslint cannot see that intent.
  // eslint-disable-next-line react-hooks/exhaustive-deps
  const client = useMemo(() => createGraphqlClient(), [label])
  return <UrqlProvider value={client}>{children}</UrqlProvider>
}
