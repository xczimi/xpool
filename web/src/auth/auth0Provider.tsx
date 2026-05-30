import type { ReactNode } from 'react'
import { useEffect } from 'react'
import { Auth0Provider as SdkProvider, useAuth0 } from '@auth0/auth0-react'
import { setTokenFromAuth0 } from './devAuth'

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
      }}
      cacheLocation="memory"
    >
      <TokenBridge>{children}</TokenBridge>
    </SdkProvider>
  )
}

/** Sync the Auth0 access token into the same localStorage slot the urql
 * client reads (`xpool.jwt`). The urql `fetchOptions` runs per-request, so
 * setting it whenever the token changes is sufficient. */
function TokenBridge({ children }: { children: ReactNode }) {
  const { isAuthenticated, getAccessTokenSilently } = useAuth0()
  useEffect(() => {
    if (!isAuthenticated) return
    let cancelled = false
    void getAccessTokenSilently().then((t) => {
      if (!cancelled) setTokenFromAuth0(t)
    })
    return () => { cancelled = true }
  }, [isAuthenticated, getAccessTokenSilently])
  return <>{children}</>
}
