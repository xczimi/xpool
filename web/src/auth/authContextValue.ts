import { createContext } from 'react'

export type AuthState = {
  /** Display label for the currently-active player (dev mode); null = visitor. */
  label: string | null
  login: (playerId: string) => Promise<void>
  logout: () => void
}

export const AuthContext = createContext<AuthState | null>(null)
