import { createContext } from 'react'

export type AuthState = {
  /**
   * Display label for the currently-active player; null = visitor.
   *
   * Forced to null while `sessionExpired` — the server has rejected this
   * session, so the app must not keep claiming one (every `pause: !label`
   * query would otherwise keep firing against an anonymous session).
   */
  label: string | null
  /** The server no longer accepts this session — see `auth/sessionState.ts`. */
  sessionExpired: boolean
  login: (playerId: string) => Promise<void>
  logout: () => void
  /** Recover from a dead session: drop it, then start a fresh login. */
  reauthenticate: () => void
}

export const AuthContext = createContext<AuthState | null>(null)
