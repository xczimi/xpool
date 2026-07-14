/**
 * The one "the server no longer accepts this session" flag.
 *
 * It lives outside React because one of its writers is the urql fetch wrapper
 * (`graphql/client.ts`), which runs outside the component tree. `AuthContext`
 * reads it through `useSyncExternalStore`.
 *
 * Set by: an Auth0 silent-refresh rejection (`devAuth.resolveToken`), a 401
 * from the auth seam (`graphql/client.ts`), and `me` resolving to null while
 * the SPA still believes it is logged in (`components/Layout.tsx`).
 */

type Listener = () => void

let expired = false
const listeners = new Set<Listener>()

function notify(): void {
  for (const listener of listeners) listener()
}

/** The session cannot authenticate any more. Idempotent. */
export function markSessionExpired(): void {
  if (expired) return
  expired = true
  notify()
}

/** A fresh, working session exists again. Idempotent. */
export function clearSessionExpired(): void {
  if (!expired) return
  expired = false
  notify()
}

export function isSessionExpired(): boolean {
  return expired
}

/** `useSyncExternalStore` subscribe: returns the unsubscribe function. */
export function subscribeSessionExpired(listener: Listener): () => void {
  listeners.add(listener)
  return () => {
    listeners.delete(listener)
  }
}
