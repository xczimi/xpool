/**
 * Dev auth stub. The API resolves the current player from an `X-Dev-Player`
 * header (API.md §8). This module persists the chosen player id in
 * localStorage; the urql client reads it on every request.
 */

const STORAGE_KEY = 'xpool.devPlayerId'

export function getDevPlayerId(): string | null {
  try {
    return localStorage.getItem(STORAGE_KEY)
  } catch {
    return null
  }
}

export function setDevPlayerId(playerId: string): void {
  try {
    localStorage.setItem(STORAGE_KEY, playerId)
  } catch {
    /* ignore storage failures */
  }
}

export function clearDevPlayerId(): void {
  try {
    localStorage.removeItem(STORAGE_KEY)
  } catch {
    /* ignore */
  }
}
