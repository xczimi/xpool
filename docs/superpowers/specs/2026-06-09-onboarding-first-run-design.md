# Onboarding & first-run UX improvements

**Date:** 2026-06-09
**Status:** Approved design — ready for implementation plan

## Problem

Live testing of the invite flow surfaced four first-run problems, all hitting a
brand-new invited person at the worst moment:

1. **"Log in" is confusing.** A button labelled "Members: log in" implies you
   already have an account. A new invitee doesn't understand that this step
   simply *establishes their identity* (via Auth0).
2. **The invite code is lost across the Auth0 redirect.** A logged-out visitor
   on `/invite/<code>` who signs in is returned to `/` (the SDK `redirect_uri`
   is hardcoded to `window.location.origin`, and no `appState`/`returnTo` is
   captured or restored). They have to go back and find their invite URL again.
3. **The first-time name form is unusable** and inconsistent with the Profile
   form. The claim form uses bare `<input>`s with hardcoded English placeholders
   ("Nick", "Full name", "Join"); the Profile form (the good one) uses the
   shared `.form` pattern with i18n `<label>`s and a `.primary` button.
4. **The settings gear is hard to notice.** A small, dim ⚙ in the top-right —
   a Hungarian invitee can't find the language switch.

## Decisions (settled in brainstorming)

- **New-invitee identity step:** the invite/claim page shows pool context + one
  clear **"Continue to join"** button (not "log in"). Clicking it runs Auth0,
  preserves the code, and returns to the name step.
- **Settings discoverability:** one entry point, made obvious — a bigger,
  brighter, **labelled** gear. Controls stay in the popover (no inline controls).
- **No API change.** All four parts are web-only. Showing the specific *pool
  name* pre-auth would need a new public resolver — out of scope; the welcome
  stays generic.

## Architecture context

- Auth SDK is `@auth0/auth0-react@2.17.0`, which supports
  `loginWithRedirect({ appState })` and `Auth0Provider`'s `onRedirectCallback`.
- **`Auth0Gate` is the outermost provider, *outside* `BrowserRouter`**
  (`web/src/main.tsx`). So `onRedirectCallback` cannot call react-router's
  `navigate`; the return path is handed off via a module-level/`sessionStorage`
  channel and consumed by a small redirect effect *inside* the Router.
- `InviteClaimPage` is the onboarding hub and the rawest page in the app
  (inline GraphQL strings, hardcoded English, `window.location.href`). It is
  rewritten as part of this work.

---

## Part A — Invite link becomes a real "join" flow (problems 1 + 2)

### A1. Preserve the return path across the Auth0 redirect

- The login trigger passes the current path as Auth0 `appState`:
  `loginWithRedirect({ appState: { returnTo: <path> }, authorizationParams: { screen_hint } })`.
- `Auth0Gate` gains `onRedirectCallback(appState)` that writes
  `appState?.returnTo` to a handoff channel (`sessionStorage` key
  `xpool.returnTo`). Using `sessionStorage` (not direct navigation) because the
  callback fires outside the Router.
- A new `<PostLoginRedirect>` component rendered *inside* `BrowserRouter` reads
  the handoff on mount: if `xpool.returnTo` is set and differs from the current
  path, it `navigate(returnTo, { replace: true })` and clears the key.
- Result: a logged-out visitor who starts on `/invite/<code>`, signs in, and is
  bounced through Auth0 lands **back on `/invite/<code>`**, now authenticated.

A pure helper owns the handoff so it is unit-testable without a browser:
`web/src/auth/returnTo.ts` — `stashReturnTo(path)`, `takeReturnTo(): string | null`
(reads-and-clears). The login trigger and `onRedirectCallback` call
`stashReturnTo`; `PostLoginRedirect` calls `takeReturnTo`.

### A2. The logged-out invite page

Rewrite the `!viewer` (logged-out) branch of `InviteClaimPage`:

- Welcome heading + a generic line ("You've been invited to xPool!").
- A one-line, plain-language explainer that addresses the "what is this login"
  confusion: roughly *"We'll set up a quick, secure sign-in (email or Google) so
  only you can enter your picks."* (i18n'd.)
- A single primary **"Continue to join"** button → `loginWithRedirect` with
  `appState.returnTo = /invite/<code>` and `screen_hint: 'signup'` (invitees are
  usually new). No bare `<a href="/">`.

The header "Members:" entry keeps `screen_hint: 'login'` (returning members).

---

## Part B — One clean, shared name form (problem 3)

- Extract `web/src/components/NameForm.tsx` — a presentational form rendering the
  `nick` + `fullName` `<label>`s (i18n) inside `<form className="form">` with a
  `.primary` submit button and a flash slot. Props:
  `{ initialNick, initialFullName, submitLabel, busy, flash, onSubmit(nick, fullName) }`.
- `ProfilePage`'s `ProfileForm` is refactored to render `NameForm` (same markup
  it has today → no visual change, proves the extraction is faithful).
- The claim branch of `InviteClaimPage` renders `NameForm` with
  `submitLabel = t('join')`, replacing the bare inputs and hardcoded strings.
- The whole `InviteClaimPage` is i18n'd and switched from `window.location.href`
  to `useNavigate`. Inline GraphQL strings may stay (out of scope to move them to
  `queries.ts`) but all user-facing copy becomes i18n keys.

New i18n keys (EN + HU): `inviteWelcomeTitle`, `inviteWelcomeBody`,
`inviteContinue` (Continue to join), `join`, plus claim/join/link page titles
currently hardcoded. Reuse existing `nick`, `fullName`, `save`, `errorPrefix`.

---

## Part C — Header wording (problem 1, small)

Change `frontDoorMembers`: "Members: log in" → "Already playing? Log in" (and the
HU equivalent), so the header clearly targets returning members and the invite
link is unambiguously the newcomer's front door. `frontDoorLead` ("Got an
invite? Open the link…") stays.

---

## Part D — Bigger, labelled settings gear (problem 4)

- `SettingsMenu.tsx`: render a `Settings` text label (existing `settings` i18n
  key) alongside the ⚙ glyph inside the trigger button.
- `index.css` `.settings-gear`: larger glyph, brighter default colour (not
  `--text-dim`), and lay out glyph + label inline with a small gap. Controls stay
  in the popover; open/close behaviour unchanged.

---

## Testing

- **Unit (`returnTo.ts`):** `stashReturnTo` then `takeReturnTo` returns the path
  and clears it; a second `takeReturnTo` returns `null`.
- **Unit (existing `extractCode`):** unchanged, still green.
- **E2E (`web/e2e`):** a logged-out visitor opening `/invite/<code>` sees the
  "Continue to join" button and the welcome copy (not the old "Log in to claim").
  The settings trigger shows a visible "Settings" label. The Profile form still
  renders its labelled fields (NameForm extraction regression guard). The full
  Auth0 redirect round-trip can't run in e2e; the `returnTo` handoff is covered
  by the unit test, and the authenticated claim path by the dev-login flow.

## Out of scope (deadline-driven)

- Showing the specific pool name on the pre-auth invite page (needs a new public
  API resolver).
- Surfacing language/theme controls inline in the header (gear stays the single
  entry point).
- Terms of Service; moving `InviteClaimPage`'s inline GraphQL into `queries.ts`.
