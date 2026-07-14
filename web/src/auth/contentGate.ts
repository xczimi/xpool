import type { Access } from './routeAccess'

/** What the `me` query says about the viewer, once resolved. */
export type ViewerState =
  /** The `me` query has not settled yet (or is paused). */
  | 'loading'
  /** A real Player. */
  | 'player'
  /** Authenticated, not a Player, no link candidate — the invite dead-end. */
  | 'unclaimed'
  /** Authenticated, not a Player, but mid link/claim flow. */
  | 'unclaimed-linkable'
  /** The server sees a Visitor: `me` resolved to null. */
  | 'anonymous'

export type Gate = 'page' | 'session-expired' | 'needs-invite'

/**
 * What `Layout` renders in the content area.
 *
 * The invariant: if the client believes there is a session, the server must
 * agree. When it does not — a rejected token, or `me` resolving to null while
 * the SPA still shows a login — we say so, instead of rendering a signed-in
 * shell over an anonymous session (which bottomed out in a contentless
 * "Something went wrong." on every player page).
 *
 * Two independent signals reach that verdict, and both must exist:
 * `sessionExpired` is the **sticky** one (`auth/sessionState.ts` — a credential
 * we watched fail: an Auth0 refresh rejection or a 401), while
 * `viewer === 'anonymous'` is the **derived** one (the server disagrees with us
 * right now, even if no token error was ever observed). Neither subsumes the
 * other.
 *
 * **Both are preconditioned on `hasSession`** — the client must actually have
 * had a session for "it expired" to mean anything. That is why `hasSession` is
 * its own `AuthState` field and NOT `Boolean(label)`: `label` is forced to null
 * precisely when the session expires, so gating on it would make this view
 * unreachable.
 *
 * Public routes always render: a dead session must not lock a viewer out of
 * Rules/Schedule/Privacy, and `/invite/:code` is the way out of the invite
 * dead-end.
 */
export function contentGate(input: {
  access: Access
  sessionExpired: boolean
  /**
   * The client obtained a session in this browser — the raw belief, NOT
   * `Boolean(label)` (which is nulled on expiry). See `AuthState.hasSession`.
   */
  hasSession: boolean
  viewer: ViewerState
}): Gate {
  const { access, sessionExpired, hasSession, viewer } = input

  if (access === 'public') return 'page'
  // Both detectors require that the client believed it had a session: the
  // sticky flag (a credential we watched the server reject) and the derived
  // check (the server says Visitor right now). A visitor who never logged in —
  // even one carrying a stray token — gets the page's own login prompt, not a
  // "your session expired" dead-end they can make no sense of.
  if (hasSession && (sessionExpired || viewer === 'anonymous')) {
    return 'session-expired'
  }
  if (viewer === 'unclaimed') return 'needs-invite'
  return 'page'
}
