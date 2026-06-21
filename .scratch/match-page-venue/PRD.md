# Show venue on the match page

Status: needs-triage
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
