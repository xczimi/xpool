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
  /**
   * The client obtained a session in this browser — independent of whether the
   * server still accepts it. Distinct from `label`, which is forced to null on
   * expiry: a viewer with a dead session still HAD one, and that is what makes
   * "your session expired" the right message rather than "log in".
   *
   * False for a visitor who never logged in (even one carrying a stray token
   * in localStorage) — they get the normal login prompt, not a dead-end.
   */
  hasSession: boolean
  login: (playerId: string) => Promise<void>
  logout: () => void
  /** Recover from a dead session: drop it, then start a fresh login. */
  reauthenticate: () => void
}

export const AuthContext = createContext<AuthState | null>(null)
