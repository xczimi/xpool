# Better access to your own player-detail page

Status: needs-triage
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
