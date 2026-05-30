/**
 * Dev auth. Mints a local-issuer JWT against the API's `POST /api/dev/login`
 * endpoint and stashes it in localStorage. The urql client reads the JWT on
 * every request and sends `Authorization: Bearer <jwt>`.
 *
 * Production builds use Auth0 SPA SDK instead — see auth0Provider.tsx (Phase 5).
 */

const TOKEN_KEY = 'xpool.jwt'
const PLAYER_KEY = 'xpool.devPlayer'

export function getToken(): string | null {
  try { return localStorage.getItem(TOKEN_KEY) } catch { return null }
}

export function getDevPlayerLabel(): string | null {
  try { return localStorage.getItem(PLAYER_KEY) } catch { return null }
}

export async function devLogin(playerId: string): Promise<string> {
  const res = await fetch('/api/dev/login', {
    method: 'POST',
    headers: { 'content-type': 'application/json' },
    body: JSON.stringify({ player: playerId }),
  })
  if (!res.ok) throw new Error(`dev-login failed: ${res.status}`)
  const { token } = (await res.json()) as { token: string }
  try {
    localStorage.setItem(TOKEN_KEY, token)
    localStorage.setItem(PLAYER_KEY, playerId)
  } catch { /* ignore */ }
  return token
}

export function clearToken(): void {
  try {
    localStorage.removeItem(TOKEN_KEY)
    localStorage.removeItem(PLAYER_KEY)
  } catch { /* ignore */ }
}

export function setTokenFromAuth0(token: string): void {
  try { localStorage.setItem(TOKEN_KEY, token) } catch { /* ignore */ }
}

// Dev-clock storage stays as-is.
const NOW_KEY = 'xpool.devNow'
export function getDevNow(): string | null {
  try { return localStorage.getItem(NOW_KEY) } catch { return null }
}
export function setDevNow(iso: string): void {
  try { localStorage.setItem(NOW_KEY, iso) } catch { /* ignore */ }
}
export function clearDevNow(): void {
  try { localStorage.removeItem(NOW_KEY) } catch { /* ignore */ }
}
