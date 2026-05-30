import { useContext } from 'react'
import { AuthContext, type AuthState } from './authContextValue'

export function useAuth(): AuthState {
  const ctx = useContext(AuthContext)
  if (ctx === null) {
    throw new Error('useAuth must be used within AuthProvider')
  }
  return ctx
}
