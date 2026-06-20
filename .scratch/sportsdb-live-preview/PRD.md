# Live match preview — provisional points during a game

Status: done (PR #16, 2026-06-20) — match page at `/match/:gameId` (official/live/none
score, all-players tip grid, pool selector); live/provisional scoring covered by
Rust resolver tests. Live scoreboard remains the **#2b** follow-on (see the spec).

## Summary

A **match-detail page** that, for an *in-progress* match, shows the **live score** and computes each player's **provisional points** — "what you'd earn if it ended now." Display-only and ephemeral; it never writes an official result. This is consumer **#2** of the TheSportsDB foundation shipped in PR #15 (the assisted official-result entry / `reportedResults` feature).

## Why

During a live match, players want to see how their prediction is tracking. The scoring engine is a pure function, so feeding it the live score gives provisional points "for free." This is distinct from the shipped feature, which only pre-fills *finished* results for the admin to enter.

## Background / what already exists (reuse, don't rebuild)

Shipped in PR #15 (`master` from `b543e2c`). Read the design first: **`docs/superpowers/specs/2026-06-14-sportsdb-reported-results-design.md` §11** ("How #2 reuses this").

- **`crates/sportsdb`** — V2 client. Already has `lookup_event(id)` (accurate, used by the resolver). Add `livescores()` → `/livescore/4429` for the in-progress feed (premium; one league-wide call). Pure decoders + `Event` model with `str_status` / `str_timestamp` already there.
- **`ReportedResult` GraphQL type** (`crates/api/src/gql/types.rs`) — already provenance-named with `sourceStatus`; a live match just carries a non-finished status (e.g. `2H 67`). Reusable as-is or as the basis for a `LiveScore` type.
- **Pure scoring** — `domain::scoring::score_match_parts(prediction, actual, config)` + `PointsBreakdown::build(...)` (see how `tips`/`perfects` resolvers in `crates/api/src/gql/query.rs` already call it). Feed the live score in place of the official result → provisional points. No new scoring logic.
- **Game ↔ idEvent mapping** — `SingleGame.external_id` is populated for the 72 group games (committed in `fwc26.json`); knockouts fill in later. Same mapping the live feed needs.

## Scope

- A dedicated **match page** (route per game): live score + per-player provisional points/breakdown for that match. (This is the "dedicated page for each match" UX idea — it may also host venue/stats later.)
- The provisional preview is **visible to all players** (not admin-gated like `reportedResults`) — it's read-only public info once a match is live. Confirm the tip-visibility rules still hold (don't reveal a hidden prediction before its match opens; reuse the `tips` resolver's mutual-commitment gating).
- **Out of scope:** writing/auto-entering results (that stays the admin's `submitGroup` flow); the player-detail page (that's a separate #3 — currently only in the design decomposition, not yet a PRD).

## Hard-won learnings from #1 (apply these)

- **The bulk `/schedule/previous/league` feed's `strStatus` lags badly** (stuck on `2H` minutes after FT); **`/lookup/event/{id}` is accurate.** For live preview, `/livescore/4429` is the right source (it's the live feed); cross-check against `lookup_event` if status matters.
- **Scores are consistent across endpoints; only status lags** — so a live preview can trust the scoreline.
- **No finality/status gate, no UI "confirm" badge** for the admin pre-fill (see memory `reported-results-no-status-marker`) — but #2 is different: here surfacing the live status (`2H 67`) is the *whole point*, so DO show it.
- Rate limits are a non-issue if you always pull league-wide (`/livescore/4429`) and never loop per-event.

## Open questions for whoever picks this up

- Polling cadence on the client for the live score (and whether to use the ~45s server cache pattern from `CachingSource`, or a shorter TTL for live).
- Whether provisional points need their own GraphQL query (e.g. `liveMatch(gameId)`) or extend `tips`/the match page query.
- Knockout 90′ vs ET handling for live (the `ninetyMinuteUncertain` flag exists).

## Comments
