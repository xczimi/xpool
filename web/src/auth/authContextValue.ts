import { createContext } from 'react'

export interface AuthState {
  /** The chosen dev player id, or null for a visitor. */
  playerId: string | null
  login: (playerId: string) => void
  logout: () => void
}

export const AuthContext = createContext<AuthState | undefined>(undefined)
