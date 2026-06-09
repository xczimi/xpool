import { useEffect } from 'react'
import { useNavigate } from 'react-router-dom'
import { useAuth0 } from '@auth0/auth0-react'
import { takeReturnTo } from '../auth/returnTo'

/**
 * After an Auth0 sign-in redirect lands the app back on `/`, restore the page
 * the user started from (e.g. their `/invite/<code>`), which `onRedirectCallback`
 * stashed via `stashReturnTo`. Gated on `isAuthenticated` so it runs only once
 * the SDK has processed the redirect (which is after the stash), avoiding a race
 * with this component's mount. No-op in dev (no Auth0 provider → `isAuthenticated`
 * stays false, and nothing is ever stashed).
 */
export function PostLoginRedirect() {
  const navigate = useNavigate()
  const { isAuthenticated } = useAuth0()
  useEffect(() => {
    if (!isAuthenticated) return
    const target = takeReturnTo()
    if (target && target !== window.location.pathname) {
      navigate(target, { replace: true })
    }
  }, [isAuthenticated, navigate])
  return null
}
