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
 * `hasSession && viewer === 'anonymous'` below is the **derived** one (the
 * server disagrees with us right now, even if no token error was ever
 * observed). Neither subsumes the other.
 *
 * Public routes always render: a dead session must not lock a viewer out of
 * Rules/Schedule/Privacy, and `/invite/:code` is the way out of the invite
 * dead-end.
 */
export function contentGate(input: {
  access: Access
  sessionExpired: boolean
  /** The client believes a session exists (`label !== null`). */
  hasSession: boolean
  viewer: ViewerState
}): Gate {
  const { access, sessionExpired, hasSession, viewer } = input

  if (access === 'public') return 'page'
  if (sessionExpired) return 'session-expired'
  if (hasSession && viewer === 'anonymous') return 'session-expired'
  if (viewer === 'unclaimed') return 'needs-invite'
  return 'page'
}
