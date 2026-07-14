/**
 * The one "the server no longer accepts this session" flag.
 *
 * It lives outside React because one of its writers is the urql fetch wrapper
 * (`graphql/client.ts`), which runs outside the component tree. `AuthContext`
 * reads it through `useSyncExternalStore`.
 *
 * Exactly TWO things write this flag, and both are observations of a *dead
 * credential*:
 *   1. an Auth0 silent-refresh rejection (`auth/devAuth.ts`, `resolveToken`)
 *   2. a 401 from the auth seam (`graphql/client.ts`)
 *
 * This flag is the **sticky** signal: we saw the credential fail, so it stays
 * failed until a fresh session replaces it. It is *not* the only way the SPA
 * discovers an expired session. `contentGate`'s `hasSession && viewer ===
 * 'anonymous'` check is the **derived** signal: the server disagrees with us
 * *right now* — `me` resolved to null while the SPA still shows a login — even
 * when no token error was ever observed (e.g. a token that was never sent, or
 * a player deleted server-side). That condition is evaluated per render inside
 * `contentGate`; nothing in `components/Layout.tsx` writes this flag.
 *
 * Both paths must exist; neither subsumes the other. Do not delete one on the
 * assumption that the other covers it.
 *
 * **This flag alone must never gate the dead-end — always pair it with
 * `hasSession`.** It says "a credential was rejected", NOT "this viewer had a
 * session". A visitor who never logged in can still set it: any stray token in
 * localStorage rides along on the first ungated query (`DevAuthBar`'s
 * `PLAYERS_QUERY` fires with no `pause`), gets a 401, and marks the flag.
 * Telling that viewer their session expired is a false positive — they simply
 * need to log in. `contentGate` therefore requires `hasSession` before it
 * dead-ends anyone, and any future reader of this flag must do the same.
 */

type Listener = () => void

let expired = false
const listeners = new Set<Listener>()

function notify(): void {
  // Snapshot: a listener may synchronously mark/clear during delivery, and
  // iterating the live Set would then re-deliver to listeners not yet visited.
  for (const listener of [...listeners]) listener()
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
