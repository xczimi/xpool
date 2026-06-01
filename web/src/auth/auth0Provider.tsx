import type { ReactNode } from 'react'
import { useEffect } from 'react'
import { Auth0Provider as SdkProvider, useAuth0 } from '@auth0/auth0-react'
import { clearToken, setAuth0Getter, setTokenFromAuth0 } from './devAuth'

const DOMAIN = import.meta.env.VITE_AUTH0_DOMAIN as string | undefined
const CLIENT = import.meta.env.VITE_AUTH0_CLIENT_ID as string | undefined
const AUDIENCE = import.meta.env.VITE_AUTH0_AUDIENCE as string | undefined

export const auth0Enabled = !!(DOMAIN && CLIENT)

export function Auth0Gate({ children }: { children: ReactNode }) {
  if (!auth0Enabled) return <>{children}</>
  return (
    <SdkProvider
      domain={DOMAIN!}
      clientId={CLIENT!}
      authorizationParams={{
        redirect_uri: window.location.origin,
        audience: AUDIENCE,
        // `offline_access` requests a refresh token alongside the access
        // token — required for `useRefreshTokens` below.
        scope: 'openid profile email offline_access',
      }}
      // localStorage + refresh tokens survives page reload without relying on
      // Auth0's third-party tenant cookie (blocked by default in Safari/Brave
      // and being phased out by Chrome). Refresh-token rotation is on by
      // default for SPA app types in Auth0; the SDK handles rotation
      // transparently. Trade-off: refresh token is reachable to XSS — accepted
      // for a hobby app.
      cacheLocation="localstorage"
      useRefreshTokens
    >
      <TokenBridge>{children}</TokenBridge>
    </SdkProvider>
  )
}

/**
 * Register a per-request Auth0 token getter with the urql client.
 *
 * Instead of writing the access token to localStorage once on login, this
 * registers `getAccessTokenSilently` as a getter that the urql `fetchWithAuth`
 * wrapper calls before every GraphQL request. The Auth0 SDK caches the token
 * and silently refreshes it when it expires (~24 h), so per-request calls are
 * cheap when the token is still valid.
 *
 * We also seed localStorage on the first authentication so a hard reload before
 * any GraphQL request still has a token to send (the getter is registered
 * asynchronously via React's effect loop).
 */
function TokenBridge({ children }: { children: ReactNode }) {
  const { isAuthenticated, getAccessTokenSilently } = useAuth0()
  useEffect(() => {
    if (!isAuthenticated) {
      setAuth0Getter(null)
      // Auth0's logout() clears the SDK + tenant session but leaves the
      // localStorage-seeded JWT in place. Without this, a stale Bearer token
      // outlives logout and the API keeps returning AuthenticatedUnclaimed.
      clearToken()
      return
    }
    // Register the getter so every subsequent urql request fetches a fresh token.
    setAuth0Getter(() => getAccessTokenSilently())
    // Seed localStorage for hard-reload resilience (the getter covers all
    // normal in-session requests; localStorage covers the gap before the first
    // effect fires after a page reload).
    void getAccessTokenSilently().then((t) => setTokenFromAuth0(t)).catch(() => {})
    return () => { setAuth0Getter(null) }
  }, [isAuthenticated, getAccessTokenSilently])
  return <>{children}</>
}
