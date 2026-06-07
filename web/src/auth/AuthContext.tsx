import { useEffect, useMemo, useState, type ReactNode } from 'react'
import { useAuth0 } from '@auth0/auth0-react'
import { clearToken, devLogin as apiDevLogin, getDevPlayerLabel, setTokenFromAuth0 } from './devAuth'
import { auth0Enabled } from './auth0Provider'
import { AuthContext, type AuthState } from './authContextValue'

/**
 * `label` is the app-wide "there is an authenticated session" signal — every
 * page gates its `me` query on `pause: !label` and `GraphqlProvider` rebuilds
 * the urql client (fresh cache) whenever it changes. In dev mode it comes from
 * the dev-login picker; in Auth0 mode it must reflect the Auth0 session.
 */
export function AuthProvider({ children }: { children: ReactNode }) {
  // `auth0Enabled` is a build-time constant, so this branch is stable across
  // renders (no conditional-hook hazard).
  return auth0Enabled ? (
    <Auth0AuthProvider>{children}</Auth0AuthProvider>
  ) : (
    <DevAuthProvider>{children}</DevAuthProvider>
  )
}

/** Dev-stub auth: the label is the seeded player chosen in the dev login bar. */
function DevAuthProvider({ children }: { children: ReactNode }) {
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

/**
 * Auth0 auth. `label` becomes truthy ONLY once the access token has actually
 * been fetched — not merely when `isAuthenticated` flips. Auth0 access tokens
 * omit `email`, so the API resolves it from `/userinfo`; firing `me` before the
 * token is attached would resolve to a Visitor and (via the document cache)
 * stick. Gating on a fetched token guarantees every `pause: !label` query goes
 * out with a bearer.
 */
function Auth0AuthProvider({ children }: { children: ReactNode }) {
  const { isAuthenticated, user, getAccessTokenSilently, loginWithRedirect, logout } = useAuth0()
  const [tokenReady, setTokenReady] = useState(false)

  useEffect(() => {
    if (!isAuthenticated) return
    let active = true
    void getAccessTokenSilently()
      .then((token) => {
        setTokenFromAuth0(token)
        if (active) setTokenReady(true)
      })
      .catch(() => {
        // Even if the silent fetch fails, unblock so the app renders (the
        // request resolves to a Visitor rather than hanging on a spinner).
        if (active) setTokenReady(true)
      })
    // Reset on logout / principal change in cleanup (not synchronously in the
    // body) so a re-login re-gates on a freshly fetched token.
    return () => {
      active = false
      setTokenReady(false)
    }
  }, [isAuthenticated, getAccessTokenSilently])

  const label = isAuthenticated && tokenReady ? (user?.email ?? user?.name ?? 'player') : null

  const value = useMemo<AuthState>(
    () => ({
      label,
      // Auth0 login/logout is normally driven from ProdAuthBar via the SDK
      // directly; these keep `useAuth()` consistent for any other caller.
      login: async () => {
        await loginWithRedirect()
      },
      logout: () => {
        clearToken()
        logout({ logoutParams: { returnTo: window.location.origin } })
      },
    }),
    [label, loginWithRedirect, logout],
  )

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}
