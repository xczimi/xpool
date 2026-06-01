import { useMemo, useState, type ReactNode } from 'react'
import { clearToken, devLogin as apiDevLogin, getDevPlayerLabel } from './devAuth'
import { AuthContext, type AuthState } from './authContextValue'

export function AuthProvider({ children }: { children: ReactNode }) {
  const [label, setLabel] = useState<string | null>(getDevPlayerLabel())

  const value = useMemo<AuthState>(
    () => ({
      label,
      login: async (id: string) => {
        await apiDevLogin(id)
        setLabel(id)
      },
      logout: () => {
        clearToken()
        setLabel(null)
      },
    }),
    [label],
  )

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}
