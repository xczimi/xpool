# Expired-session dead-end — design

**Date:** 2026-07-14
**Status:** approved, not yet implemented

## Problem

A player on prod (`pool.xczimi.com/mytips`, mid-tournament) saw a bare
**"Something went wrong."** on every player page while the auth bar still read
*"Logged in as &lt;their email&gt;"* and the full player nav was rendered. Logging
out and back in fixed it.

The message is a symptom. The real defect: **the SPA renders a signed-in shell
over a session the server has already rejected.**

### The failure chain

1. Auth0's refresh token expires (or is rotated out / revoked), so
   `getAccessTokenSilently()` rejects.
2. `web/src/auth/AuthContext.tsx:67` catches the rejection and *still* sets
   `tokenReady = true`, so `label` resolves to `user.email` — read from the
   Auth0 SDK's `localStorage` cache, which outlives the refresh token. The app
   believes there is a session.
3. `web/src/auth/devAuth.ts` `resolveToken()` catches the same rejection and
   returns `null`, so `fetchWithAuth` sends **no `Authorization` header**.
4. The API's auth seam sees no bearer → `CurrentPlayer::Visitor`
   (`crates/api/src/auth/seam.rs:39`) → the `me` resolver returns `null`
   (`crates/api/src/gql/query.rs:334`).
5. `MyTipsPage.tsx:255` — `if (!tournament || !me) return <ErrorView />` — a
   bare `ErrorView` with no `message` prop, which renders exactly
   `"Something went wrong."` (`StatusViews.tsx`, `errorPrefix` + `'.'`).

Both `catch` sites swallow the one fact that matters: *this session can no
longer authenticate.*

## The invariant

> If the client believes there is a session, the server must agree. When it
> doesn't, say so — never render a signed-in shell over an anonymous session.

Three distinct failures currently degrade into the same silent state. They
collapse into **one** state and **one** view.

## Detection — three funnels, one state

| # | Trigger | Where | Why it is needed |
|---|---------|-------|------------------|
| 1 | `getAccessTokenSilently()` rejects | `auth/AuthContext.tsx`, `auth/devAuth.ts` (`resolveToken`) | The observed production case. Both sites currently `catch {}` and carry on. |
| 2 | A GraphQL response is `401` | `graphql/client.ts` (`fetchWithAuth`) | The seam returns `401 invalid token` for an expired/invalid JWT (`seam.rs:41`). One choke point covering every query **and** mutation. |
| 3 | `me` settles to `null` while `label` is truthy | `components/Layout.tsx` | The direct assertion of the invariant — the net for anything the first two miss. |

Detector 3 fires only on an explicit `data.me === null` (server says Visitor),
never on `data === undefined` (still in flight) — otherwise it would flash
during the first render.

An `UnclaimedViewer` is **not** a null `me`, so detector 3 does not disturb the
existing invite dead-end.

### State

A small `web/src/auth/sessionState.ts` module: a boolean plus a listener set,
consumed via `useSyncExternalStore`. It cannot live in React state alone because
detector 2 fires inside `fetchWithAuth`, outside the component tree.

```ts
markSessionExpired()      // any detector
clearSessionExpired()     // on a successful re-login
isSessionExpired()        // snapshot
subscribeSessionExpired() // useSyncExternalStore
```

`AuthState` gains `sessionExpired: boolean`. When it is true, `label` is `null`
— so every `pause: !label` query stays paused and the auth bar stops claiming
"Logged in as …".

## The view

`web/src/components/SessionExpired.tsx`, a sibling of `NeedsInvite` and built
the same way: a `status` block with a title, one line of explanation, and a
**Log in again** button.

`Layout` swaps it in for `<Outlet />` when
`sessionExpired && accessFor(location.pathname) !== 'public'` — mirroring the
existing `deadEnd` branch, so Rules / Schedule / Privacy stay browsable while
every player-only page explains itself.

The button calls a new `reauthenticate()` on `AuthState`, keeping the view
provider-agnostic:

- **Auth0 mode:** `clearToken()` → `loginWithRedirect({ appState: { returnTo: currentPath } })`
- **Dev-stub mode:** `clearToken()` → drop the label, revealing the dev player picker.

### Deliberate simplification

*Any* silent-refresh failure counts as expired, including a transient network
blip — we do not branch on the Auth0 error code (`login_required`,
`invalid_grant`, …). Without a token no player page can render anyway; the
fallback is the same screen either way; and if the session was in fact alive,
the button silently bounces the user straight back through Auth0. A second
state would add branches with no user-visible payoff.

## Strings

New keys in `web/src/i18n/strings.ts`, English **and** Hungarian
(i18n is first-class — CLAUDE.md):

- `sessionExpiredTitle` — "Your session has expired"
- `sessionExpiredBody` — one line: log in again to see your tips
- `logInAgain` — the button

## Testing

- **E2E (`web/e2e/session-expired.spec.ts`), dev-stub mode:** dev-login, then
  overwrite `localStorage['xpool.jwt']` with a junk token and load `/mytips`.
  The API answers `401`, so this drives **detector 2** end-to-end against the
  real server. Assert the session-expired view renders and the bare
  "Something went wrong." does not.
- **Component test** for detector 3: `label` truthy + `me: null` renders
  `SessionExpired`, while `me: UnclaimedViewer` still renders `NeedsInvite`
  (guards the invite dead-end against regression).

## Scope

Session dead-end **only** (explicit scope decision).

Out of scope: the other bare `<ErrorView />` call sites (`MyTipsPage.tsx:255`,
`ProfilePage.tsx:26`, Schedule / Today / AllTips / H2H / Player / Scoreboard).
`MyTipsPage.tsx:255` stays reachable for an authenticated-but-unclaimed viewer
who *has* a link candidate — Layout only shields unclaimed viewers *without*
one. That path can still produce a contentless "Something went wrong."; it is a
separate fix.

Also out of scope: routing an unclaimed-with-link-candidate viewer into the
link/claim flow.
