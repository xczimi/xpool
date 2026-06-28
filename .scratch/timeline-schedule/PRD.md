# Timeline schedule — group by date as an alternative to group-by-group

Status: done (shipped via backlog-parallel-build round 1, merged to master; verified in git 2026-06-27)
Area: web

## Idea

Offer a second way to read the schedule: a chronological, **group-by-date**
timeline ("Alternative Timeline") alongside the current **group-by-group**
view. Let the user toggle between the two.

## Motivation

The current `SchedulePage` is organised by tournament group (Group A, B, C…).
That's great for "how does my group stand", but during the tournament people
also think in days — "what's on today / this weekend, in kickoff order". A
date-ordered timeline answers that directly and pairs naturally with the
existing Today view and with [[timezone-clarity]]'s relative-deadline idea.

## Sketch

- A view toggle on the schedule page: **By group** (current) ⇄ **By date**.
- By-date: matches sorted chronologically, sectioned by calendar day, each row
  showing kickoff time, both teams, group label, and deadline state.
- Reuse the same match rows / links to `/match/:gameId`; only the grouping and
  ordering change.
- Honour the server-authoritative clock for "today" boundaries (no `Date.now()`
  branching).

## Open questions

- Persist the chosen view (per-user setting) or default fresh each visit?
- Day boundaries in user-local or stadium-local time? (ties to [[timezone-clarity]])
- Does this subsume the separate Today page, or stay distinct?

## Resolved decisions (grilled 2026-06-21)

- **View toggle: By group (current) ⇄ By date.** By-date sorts matches
  chronologically, sectioned by calendar day, reusing the existing match rows /
  /match/:gameId links.
- **Persist chosen view per-user in localStorage.**
- **Day boundary = viewer's local day** (browser tz, same basis as the existing
  formatKickoff display). No venue-tz data needed; no Date.now() branching —
  day grouping is pure client-side formatting of the ISO kickoff.
- **Today page stays distinct** (keeps its live polling + tip columns).
