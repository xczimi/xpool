/**
 * Durable breadcrumb for the invite a recipient is mid-claiming when they leave
 * for Auth0.
 *
 * Unlike `returnTo` (sessionStorage, a within-page bridge from
 * `onRedirectCallback` to `PostLoginRedirect`), the invite path must survive an
 * Auth0 *signup* round-trip — and that flow can break the same-tab chain
 * (e.g. email verification opens a fresh tab with no Auth0 transaction, so
 * `appState.returnTo` is gone). `localStorage` is per-origin and shared across
 * all tabs of this origin, so the code written here before leaving is still
 * present whenever the user lands back on the app — even in a new tab.
 *
 * One-shot: `takePendingInvitePath` reads and clears. `Storage` is injectable so
 * the helper is unit-testable without a browser (vitest runs in the node env).
 *
 * Limitation: still client-side, so it does not cross *devices* (verify on
 * phone, started on laptop). That needs a server-side pending invite.
 */
const KEY = 'xpool.pendingInvite'

export function rememberPendingInvite(
  code: string,
  storage: Storage = globalThis.localStorage,
): void {
  storage.setItem(KEY, code)
}

/** Read-and-clear the remembered code as a claim path (null if none). */
export function takePendingInvitePath(
  storage: Storage = globalThis.localStorage,
): string | null {
  const code = storage.getItem(KEY)
  if (code === null) return null
  storage.removeItem(KEY)
  return `/invite/${code}`
}
