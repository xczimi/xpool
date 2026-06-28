# Show venue on the match page

Status: done (shipped via backlog-parallel-build round 1, merged to master; verified in git 2026-06-27)
Area: web

## Idea

Display the match **venue** (stadium) on the match page at `/match/:gameId`.

## Motivation

A match has a place, not just a time. Showing the venue adds context (and a
sense of occasion) and sets up later location/timezone work — the stadium's
local time is the natural "secondary" time in [[timezone-clarity]].

## Sketch

- **Frontend-only.** `venue: Option<String>` already exists on the domain model
  and is exposed in the GraphQL type (`crates/api/src/gql/types.rs:94`), so no
  backend change — just select and render it on `web/src/pages/MatchPage.tsx`.
- Render near the kickoff time/header; hide gracefully when `venue` is null
  (knockout fixtures resolved late may not have one yet).
- i18n the label (EN + HU).

## Open questions

- Just the stadium name, or stadium + city/country?
- Any link-out (map) or purely informational?

## Resolved decisions (grilled 2026-06-21)

- **Informational text only** — render the `venue` string near the kickoff
  time; no map link-out. Hide gracefully when null.
- `venue` is already selected in MATCH_QUERY and exposed on the GraphQL Game
  type — **frontend-only render change** on MatchPage. i18n the label (en+hu).
