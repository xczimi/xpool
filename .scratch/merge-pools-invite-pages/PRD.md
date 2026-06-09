# Merge the Pools and Invite pages

Status: ready-for-agent
Area: web

> **Agreed design below** (grilled 2026-06-09). The original idea framing is
> kept under "## Idea (original)" for history. The grilling reframed the work:
> it is not "combine two feature sets" but **delete the redundant share page +
> repurpose bare `/invite` into a public code-entry front door.**

## Agreed design (grilled 2026-06-09)

### What we found

- `PoolsPage` already **fully contains** `InvitePage`'s functionality: per-pool
  *Share invite* (mint → reveal link → copy → revoke). `InvitePage` is a strict
  subset (a pool dropdown + one share button), so "merge" is really a deletion.
- The **actual invite URL** an inviter shares is `<origin>/invite/<prefix>-<code>`
  (`crates/api/src/gql/mutation.rs` `invite_link`). That maps to **`/invite/:code`
  → `InviteClaimPage`**, the *accept* side — unchanged, out of scope.
- `NeedsInvite` (the unclaimed-viewer dead-end) **already implements** the
  recipient-side "paste a link or code → `extractCode` → navigate to
  `/invite/:code`" widget. It just isn't reachable as a standalone public route.
- No internal links point at bare `/invite` (no `<Link>`, `navigate()`, or e2e).
  The only touchpoints are the NavBar item and the `routeAccess` player entry.

### Decisions

- **D1 — Merge intent = pure deletion.** Delete `InvitePage`'s share role; Pools'
  existing per-pool *Share invite* is the sole share surface. Nothing new is added
  to Pools; no share functionality is lost.
- **D2 — Repurpose bare `/invite`, don't redirect it.** It becomes a **public
  recipient-side "enter your invite code" entry**, not a redirect to `/pools`.
  Real invite links stay `/invite/:code` → `InviteClaimPage` (unchanged).
- **D3 — `/invite` renders the existing `NeedsInvite` component** (verbatim). No
  new page component; reuse the paste → `extractCode` → `/invite/:code` widget.
- **D4 — `/invite` flips player-only → public.** Drop `/invite` from
  `PLAYER_PATHS`; point the route element at `<NeedsInvite/>`; fix the now-stale
  "share page is player-only" comment in `routeAccess.ts`. (The `deadEnd` gate is
  unaffected — it only fires for *unclaimed* viewers on non-public routes; the
  public `/invite` element now renders `NeedsInvite` directly for everyone.)
- **D5 — Remove the `navInvite` nav item.** `/invite` is reached by being directed
  there (or inlined via the unclaimed dead-end). The "Invite" label no longer
  describes the page and doesn't apply to logged-in players.
- **D6 — Logout-button polish deferred.** `NeedsInvite` renders verbatim now; a
  logged-out visitor at `/invite` would see a no-op "Log Out". Filed as a
  follow-up: guard that button on `label` (session present). See follow-ups below.
- **D7 — Keep both code-entry paths.** Pools' player-only "join via code" form
  (in-app convenience for a logged-in player joining another pool) and the public
  `/invite` front door coexist; no change to Pools' forms. Future consolidation
  noted, out of scope.

### Scope (the diff)

- `web/src/pages/InvitePage.tsx` — **delete**.
- `web/src/App.tsx` — drop the `InvitePage` import; change
  `<Route path="invite" element={<InvitePage/>}>` to `element={<NeedsInvite/>}`
  (import `NeedsInvite`). Leave `invite/:code` → `InviteClaimPage` untouched.
- `web/src/auth/routeAccess.ts` — remove `/invite` from `PLAYER_PATHS`; rewrite
  the doc comment (no share page; `/invite` is the public code-entry page,
  `/invite/:code` is the claim page).
- `web/src/components/NavBar.tsx` — remove `{ to: '/invite', label: 'navInvite' }`.
- `web/src/i18n/strings.ts` — delete the orphaned `navInvite` key (en + hu).
  **Keep** `shareInvite` / `inviteShared` / `inviteLinkLabel` (Pools uses them).
- **Unchanged:** `PoolsPage`, `InviteClaimPage`, `NeedsInvite` (verbatim),
  `inviteCode.ts`.

### Follow-ups (separate issues)

- Guard `NeedsInvite`'s Log Out button on session presence so the public
  `/invite` page doesn't show a no-op logout to logged-out visitors (D6).
- Possible future: fold Pools' join-code entry into `/invite` for one code-entry
  path (D7) — additive, out of scope here.

### Testing (frontend work needs E2E)

- e2e: navigate to `/invite` both **logged-out and logged-in**; paste a full
  link and a bare code → each lands on the `/invite/:code` claim page. Assert the
  "Invite" nav item is gone. `InviteClaimPage` flows unchanged.
- Dev-stub auth: e2e needs `web/.env.local` blanking `VITE_AUTH0_*` (Auth0 mode
  hides the auth bar / dev login).

## Idea (original)

Fold the standalone **Invite** page into the **Pools** page so there's a single
place to manage pools and share invites, instead of two pages that overlap.

## Motivation

The two pages already do the same thing. `InvitePage`'s own docstring says it
outright:

> Share your invite into a pool you belong to. ... **The same action lives
> per-pool on the Pools page; this is the standalone entry point.**

So invite creation exists twice:

- `PoolsPage` — per-pool invite/share controls inline with each pool the user
  owns or belongs to (`CREATE_INVITE_MUTATION`, plus revoke, members, etc.).
- `InvitePage` — a pool-selector dropdown + a single "share invite" action, a
  thin subset of what Pools already offers.

Two entry points for one concept (the invite is the front door to identity —
see [[invite-is-front-door-to-identity]]) means duplicated UI, duplicated i18n
strings, and a nav choice that doesn't map to a real distinction. A newcomer has
to guess why "Pools" and "Invite" are separate.

## Sketch

- Make **Pools** the single home for pool membership *and* invite sharing — the
  per-pool invite controls already live there.
- Remove the standalone `InvitePage` route + nav entry (or redirect it to Pools).
- Keep `InviteClaimPage` untouched — that's the *accept* flow (the recipient
  side), a genuinely different concern from creating/sharing.
- Reconcile i18n: drop now-orphaned `shareInvite`-style keys, or repoint them at
  the Pools surface.

## Open questions

- Is there a use case for sharing an invite *without* first seeing the pools
  list (e.g. a deep-link target)? If so, a redirect that pre-selects the pool
  may be better than a hard removal.
- Does anything (docs, the invite explainer, emails) link directly to
  `/invite`? Those references would need updating.

## Related

- [[invite-is-front-door-to-identity]]
- [[invite-auth-soft-funnel]]
- `.scratch/pools-invites-explainer/DESIGN.md` — the maintainer design doc for
  how pools + invites fit together.
- `.scratch/page-one-liner-intros/PRD.md` — lists both `PoolsPage` and
  `InvitePage`; merging removes one of them from that list.
