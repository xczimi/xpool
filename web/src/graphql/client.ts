import { Client, cacheExchange, fetchExchange } from 'urql'
import { resolveToken, getDevNow } from '../auth/devAuth'
import { markSessionExpired } from '../auth/sessionState'

/**
 * A custom fetch wrapper that injects a fresh bearer token on every request.
 *
 * urql v5 calls `fetchOptions` synchronously, so async token resolution must
 * happen here — inside the fetch itself — rather than in `fetchOptions`. The
 * Auth0 SDK caches the token internally and only hits the network when the
 * current token is near expiry, so per-request calls are cheap.
 *
 * Exported for unit tests. A `401` is the auth seam telling us the token is not
 * usable (`crates/api/src/auth/seam.rs`) — the single choke point where every
 * query and mutation learns the session is dead.
 */
export async function fetchWithAuth(
  input: RequestInfo | URL,
  init?: RequestInit,
): Promise<Response> {
  const token = await resolveToken()
  const devNow = getDevNow()

  const headers = new Headers(init?.headers)
  headers.set('content-type', 'application/json')
  if (token) headers.set('Authorization', `Bearer ${token}`)
  if (devNow) headers.set('X-Dev-Now', devNow)

  const response = await fetch(input, { ...init, headers })
  if (response.status === 401) markSessionExpired()
  return response
}

export function createGraphqlClient(): Client {
  return new Client({
    url: '/api/graphql',
    preferGetMethod: false,
    exchanges: [cacheExchange, fetchExchange],
    fetch: fetchWithAuth,
  })
}
