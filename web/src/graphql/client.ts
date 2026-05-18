import { Client, cacheExchange, fetchExchange } from 'urql'
import { getDevPlayerId, getDevNow } from '../auth/devAuth'

/**
 * urql client targeting the GraphQL API at /api/graphql (Vite proxies /api
 * to the axum server on :3000).
 *
 * Auth seam: the API resolves the current player from an `X-Dev-Player`
 * header (API.md §8). `fetchOptions` is a function so the header reflects the
 * currently chosen dev player on every request. A visitor sends no header.
 */
export function createGraphqlClient(): Client {
  return new Client({
    url: '/api/graphql',
    // Force POST for every operation. urql defaults to `within-url-limit`,
    // which sends queries as GET — but the API serves the GraphiQL playground
    // on `GET /api/graphql` and executes only on POST.
    preferGetMethod: false,
    exchanges: [cacheExchange, fetchExchange],
    fetchOptions: () => {
      const playerId = getDevPlayerId()
      const headers: Record<string, string> = {
        'content-type': 'application/json',
      }
      if (playerId) {
        headers['X-Dev-Player'] = playerId
      }
      const devNow = getDevNow()
      if (devNow) {
        headers['X-Dev-Now'] = devNow
      }
      return { headers }
    },
  })
}
