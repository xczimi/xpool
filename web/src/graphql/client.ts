import { Client, cacheExchange, fetchExchange } from 'urql'
import { getToken, getDevNow } from '../auth/devAuth'

export function createGraphqlClient(): Client {
  return new Client({
    url: '/api/graphql',
    preferGetMethod: false,
    exchanges: [cacheExchange, fetchExchange],
    fetchOptions: () => {
      const token = getToken()
      const headers: Record<string, string> = { 'content-type': 'application/json' }
      if (token) headers['Authorization'] = `Bearer ${token}`
      const devNow = getDevNow()
      if (devNow) headers['X-Dev-Now'] = devNow
      return { headers }
    },
  })
}
