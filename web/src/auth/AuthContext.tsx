import { useMemo, useState, type ReactNode } from 'react'
import {
  clearDevPlayerId,
  getDevPlayerId,
  setDevPlayerId,
} from './devAuth'
import { AuthContext, type AuthState } from './authContextValue'

/**
 * Dev auth provider. Holds the chosen dev player id; the urql client reads it
 * from localStorage on every request (`X-Dev-Player`, API.md §8).
 */
export function AuthProvider({ children }: { children: ReactNode }) {
  const [playerId, setPlayerId] = useState<string | null>(getDevPlayerId())

  const value = useMemo<AuthState>(
    () => ({
      playerId,
      login: (id: string) => {
        setDevPlayerId(id)
        setPlayerId(id)
      },
      logout: () => {
        clearDevPlayerId()
        setPlayerId(null)
      },
    }),
    [playerId],
  )

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}
