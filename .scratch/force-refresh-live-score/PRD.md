# Force-refresh live score

Status: done (shipped via backlog-parallel-build round 1, merged to master; verified in git 2026-06-27)
Area: web (+ api/sportsdb)

## Idea

Give the user (or admin) a way to **force a refresh** of the live score for a
match, rather than waiting for the next automatic poll/cache cycle.

## Motivation

Live data from TheSportsDB ([[sportsdb-live-preview]]) is fetched/cached on some
interval. During a tense finish, a stale score is frustrating — a manual "refresh
now" gives immediate certainty and is reassuring when something looks wrong.

## Sketch

- A refresh control on the match page (and/or the live scoreboard) that re-fetches
  the live score on demand.
- Decide where the cache lives: if the API caches SportsDB responses, the button
  needs a cache-bypass path (rate-limited to respect SportsDB quotas); if the SPA
  just re-queries, it may be a client-only refetch.
- Show last-updated timestamp + a spinner/feedback on refresh.

## Open questions

- Everyone, or admin-only? (rate-limit / SportsDB quota considerations)
- Bypass the server cache or just re-issue the GraphQL query?
- Pair with an auto-refresh interval while a match is live, with the button as
  an override?

## Resolved decisions (grilled 2026-06-21)

- **Everyone can refresh** (no admin gate).
- **Button just re-issues the GraphQL query** — no `forceRefresh` cache-bypass
  arg. The existing 45s server-side CachingSource stays as the SportsDB quota
  guard, so 'everyone' is safe (no bypass to abuse).
- **Add auto-poll on the match page while a match is live** (server-driven
  interval, like TodayPage), with the button as a manual 'refresh now' override.
- Net: **frontend-only** — match-page refresh button + polling; no API change.
- Show a last-updated indicator + spinner feedback on refresh.
