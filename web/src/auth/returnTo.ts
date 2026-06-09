/**
 * One-shot handoff for the path to return to after an Auth0 sign-in redirect.
 *
 * The Auth0 redirect lands the app back on `/` (the SDK `redirect_uri` is the
 * origin), so the page the user started from — e.g. their `/invite/<code>` — is
 * otherwise lost. `onRedirectCallback` stashes the path here; `PostLoginRedirect`
 * (inside the Router) takes it and navigates. Backed by `sessionStorage` so it
 * survives the redirect round-trip in the same tab. `Storage` is injectable so
 * the helper is unit-testable without a browser (vitest runs in the node env).
 */
const KEY = 'xpool.returnTo'

export function stashReturnTo(
  path: string,
  storage: Storage = globalThis.sessionStorage,
): void {
  storage.setItem(KEY, path)
}

/** Read-and-clear the stashed path (returns null if none). */
export function takeReturnTo(
  storage: Storage = globalThis.sessionStorage,
): string | null {
  const value = storage.getItem(KEY)
  if (value !== null) storage.removeItem(KEY)
  return value
}
