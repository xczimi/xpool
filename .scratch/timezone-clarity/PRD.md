# Clearer timezones & relative deadlines

Status: needs-triage
Area: web (+ api timeflags)

## Idea

Make it unambiguous *which* timezone a kickoff / deadline is shown in, and
surface approaching deadlines as relative time ("in 2h", "closes tomorrow").

## Motivation

A score-prediction pool is timezone-sensitive: a match has a local (stadium)
timezone, the user lives in their own timezone, and deadlines are tied to
kickoff. Today it's easy to misread when a tip actually locks. Users should
never have to guess whose clock a time is in.

## Sketch

- Decide and clearly label the timezone each displayed time is in (user-local
  is the likely default — make the label explicit rather than implied).
- Consider showing the match's local/stadium time as secondary context.
- Add relative-time rendering for upcoming deadlines, derived from the
  server-authoritative clock (the existing `deadlinePassed` / time flags in
  `crates/api/src/timeflags.rs` and the server-derived `now`) — keep the SPA
  from branching on `Date.now()`.
- i18n the relative-time strings (EN + HU).

## Open questions

- Show user-local only, or user-local + stadium-local side by side?
- How early should "deadline approaching" relative time kick in (24h? 6h?)?
- Does relative time need to live-tick in the UI, or refresh per request/load?
