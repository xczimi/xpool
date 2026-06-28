# Better access to your own player-detail page

Status: done (shipped via backlog-parallel-build round 1, merged to master; verified in git 2026-06-27)
Area: web

## Idea

Make it easy and obvious to reach **your own** player-detail page
(`/player/:id`) from anywhere — a primary, always-available entry point rather
than only via incidental name links.

## Motivation

The [[player-detail-page]] shipped, and your name links to it from the AuthBar,
Profile, Perfect, All Tips, and Scoreboard. But "see everything about *my*
tournament" is a frequent, first-class action — it deserves a dedicated,
predictable route in the chrome/nav, not just a link buried in a table cell.

## Sketch

- A clear "My page" / "My tournament" entry in the primary nav (or a prominent
  AuthBar control), pointing at `/player/<my id>`.
- Consistent labelling and placement so it's always in the same spot.
- Visitor / unclaimed states: hide or route to sign-in appropriately.

## Open questions

- Top-nav item, AuthBar avatar/name, or both?
- Wording — "My page", "My tips", "Me"? (i18n EN + HU)
- Does it replace or complement the existing Profile link?

## Resolved decisions (grilled 2026-06-21)

- **Add a top-nav item** ('My player page') in NavBar pointing at
  /player/<my id>, player-gated (hidden for visitor / unclaimed / result-user).
  Keep the existing AuthBar name→/player/:id link too.
- **Reuse the existing `playerPageOwnLink` i18n string** ('My player page' /
  'Az én játékos oldalam').
- **Replace the Profile nav link** with My-player-page. The Profile/settings
  page still exists (route /profile) but is now reached **from** the player
  page (add a Profile/settings link on the own player-detail view), not the nav.
- Current player id comes from ME_QUERY (`__typename === 'Player'`).
