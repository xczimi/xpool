import { useEffect, useMemo, useState, useSyncExternalStore, type ReactNode } from 'react'
import { useAuth0 } from '@auth0/auth0-react'
import { clearToken, devLogin as apiDevLogin, getDevPlayerLabel, setTokenFromAuth0 } from './devAuth'
import { auth0Enabled } from './auth0Provider'
import { AuthContext, type AuthState } from './authContextValue'
import {
  clearSessionExpired,
  isSessionExpired,
  markSessionExpired,
  subscribeSessionExpired,
} from './sessionState'

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
  const [player, setPlayer] = useState<string | null>(getDevPlayerLabel())
  const sessionExpired = useSyncExternalStore(subscribeSessionExpired, isSessionExpired)

  // Dropping the dead session is the same three moves for logout and for
  // re-login; in dev the "login" that follows is the auth-bar player picker,
  // which appears as soon as the label is null.
  const dropSession = () => {
    clearToken()
    clearSessionExpired()
    setPlayer(null)
  }

  const value = useMemo<AuthState>(
    () => ({
      label: sessionExpired ? null : player,
      sessionExpired,
      login: async (id: string) => {
        await apiDevLogin(id)
        clearSessionExpired()
        setPlayer(id)
      },
      logout: dropSession,
      reauthenticate: dropSession,
    }),
    // `dropSession` is re-created every render but closes over only setState,
    // which is stable — no need to re-memo on it.
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [player, sessionExpired],
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
 *
 * When the silent fetch FAILS the refresh token is gone or revoked. The SDK
 * keeps a cached `user` in localStorage that outlives it, so `isAuthenticated`
 * stays true — which is how the app used to render a signed-in shell over a
 * session the server rejects. Marking the session expired is what stops that.
 */
function Auth0AuthProvider({ children }: { children: ReactNode }) {
  const { isAuthenticated, user, getAccessTokenSilently, loginWithRedirect, logout } = useAuth0()
  const [tokenReady, setTokenReady] = useState(false)
  const sessionExpired = useSyncExternalStore(subscribeSessionExpired, isSessionExpired)

  useEffect(() => {
    if (!isAuthenticated) return
    let active = true
    void getAccessTokenSilently()
      .then((token) => {
        setTokenFromAuth0(token)
        clearSessionExpired()
        if (active) setTokenReady(true)
      })
      .catch(() => {
        // No usable token. Drop the stale one and surface the dead session —
        // unblocking `tokenReady` so the app renders the SessionExpired view
        // rather than hanging on a spinner.
        clearToken()
        markSessionExpired()
        if (active) setTokenReady(true)
      })
    // Reset on logout / principal change in cleanup (not synchronously in the
    // body) so a re-login re-gates on a freshly fetched token.
    return () => {
      active = false
      setTokenReady(false)
    }
  }, [isAuthenticated, getAccessTokenSilently])

  const label =
    isAuthenticated && tokenReady && !sessionExpired
      ? (user?.email ?? user?.name ?? 'player')
      : null

  const value = useMemo<AuthState>(
    () => ({
      label,
      sessionExpired,
      // Auth0 login/logout is normally driven from ProdAuthBar via the SDK
      // directly; these keep `useAuth()` consistent for any other caller.
      login: async () => {
        await loginWithRedirect()
      },
      logout: () => {
        clearToken()
        clearSessionExpired()
        logout({ logoutParams: { returnTo: window.location.origin } })
      },
      // Straight back into Auth0, returning to the page they were on. If the
      // tenant session is still alive this is a silent round-trip; if not, they
      // land on the login form.
      reauthenticate: () => {
        clearToken()
        clearSessionExpired()
        void loginWithRedirect({
          appState: { returnTo: window.location.pathname + window.location.search },
        })
      },
    }),
    [label, sessionExpired, loginWithRedirect, logout],
  )

  return <AuthContext.Provider value={value}>{children}</AuthContext.Provider>
}
