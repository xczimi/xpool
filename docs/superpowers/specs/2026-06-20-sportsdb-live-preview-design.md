# TheSportsDB — Live Match Preview (provisional points during a game)

Consumer **#2** of the TheSportsDB foundation shipped in PR #15. Builds on
`docs/superpowers/specs/2026-06-14-sportsdb-reported-results-design.md` (read its
§11 "How #2 reuses this"). Foundation = `master` from `b543e2c`.

## 1. Motivation

During a live match a player wants to see how their prediction is tracking —
"what you'd earn if it ended now." The scoring engine is a pure function, so
feeding it the live score yields each player's **provisional points** for free.
This is display-only and ephemeral: it never writes an official result (that
stays the admin's `submitGroup` flow, consumer #1).

## 2. Decisions locked during design review (2026-06-20)

1. **Live source = the existing per-event seam.** Reuse `lookup_events(ids)`
   (`crates/api/src/reported.rs`); **no** new `sportsdb` code, **no** bulk
   `/livescore/4429` feed. The seam already returns the current scoreline for an
   in-progress event regardless of status (commit `76d6221`). With **at most two
   matches live at once** (tournament schedule), per-event is ample; the bulk
   feed's O(1)-in-match-count advantage isn't worth a second code path.
2. **One full match-detail page**, `/match/:gameId`, covering every state —
   upcoming, live, finished. "Live preview" is just the *live* branch; the page
   is not a dead-end route that only exists mid-match.
3. **A new `match(gameId): MatchDetail` query** owns the score-merge and returns
   per-player scored rows. The resolver does the lookup + calls pure functions;
   no domain logic leaks into it.
4. **The throttle is the cache floor, not the poll rate.** SportsDB is fetched at
   most once per **60s per match** (the `CachingSource` TTL), independent of how
   fast clients poll or how many viewers there are. The browser polls
   `match(gameId)` every **60s while the match is live**, and not at all
   otherwise. Polling faster than the floor would only re-read the cache — it can
   never leak into SportsDB call frequency.

### Why these (the reasoning we want to keep)

- **Per-event vs bulk live feed.** The foundation's "rate limits are a
  non-constraint" guarantee (foundation §2) assumed league-wide pulls. The
  foundation later switched to per-event lookup for accuracy, so that guarantee
  no longer literally holds. We re-derived the live-path math: per-event worst
  case ≈ `containers × liveMatches × (60 / TTL)`. At ≤2 live matches, a 60s
  floor, and low warm-container count, that is single-digit req/min against the
  100 req/min premium budget — a non-constraint for the right reason now.
- **The accuracy concern that drove per-event for *finished* results
  (`schedule/previous` status lags) does not apply here** — we *want* and *show*
  the live status, and scores are consistent across endpoints.

## 3. Architecture — where the work lands

```
domain ─▶ fwc26 ─┐
   ├────▶ storage ┼─▶ api ─▶ web
   └────▶ sportsdb┤
          xtask ──┘
```

| Crate | Change |
|---|---|
| `domain` | none |
| `storage` | none |
| `sportsdb` | **none** (the dormant `decode_livescore` stays unused) |
| `api` | one new resolver `match(gameId)` + `MatchDetail`/`MatchScore` types; a small shared helper for "score one visible tip" extracted from `tips` |
| `web` | new `MatchPage` + `/match/:gameId` route + `match(gameId)` query; Today/Schedule game rows become links |

## 4. The resolver — `match(gameId)`

Loads the game, its group/round, the two teams, the deadline, and `now` (from the
server-authoritative clock in context). It resolves **one "actual" score** to
score every prediction against, in priority order:

1. **Official** — the result-user's prediction for this game, if entered.
   → `provisional: false`, no SportsDB call.
2. **Live** — else, if the match is in its **live window**
   (`kickoff ≤ now ≤ kickoff + 3h`) and not yet officially entered, call
   `lookup_events([external_id])`. If it returns a score → `provisional: true`,
   carry `sourceStatus` (e.g. `"2H"`). The 3h window covers a knockout's extra
   time and bounds SportsDB calls to genuinely-live matches only.
3. **None** — otherwise (upcoming, mapping absent, source absent/errored, or
   finished-but-not-yet-entered after the live window) → no score; the page shows
   predictions only and, for a just-finished game, "awaiting official result."

It then builds one row per participating player using the **identical visibility
gate as `tips`** (mutual commitment / `time_open`): a prediction is scored and
revealed only when the existing rules already expose it. Since a match in its
live window has kicked off, `time_open` is true and all tips for it are open —
so the live preview reveals nothing that `tips` wouldn't already show. Each
visible prediction is scored against the resolved actual via the pure
`score_match_parts` + `PointsBreakdown`; provisional and official rows share the
breakdown shape and differ only by the `provisional` flag on `MatchScore`.

**Degradation:** source `None`/error → step 2 yields nothing → the page renders
predictions and any official result, exactly as if SportsDB were not configured.
The feature only ever *adds* a live score; it never blocks the page.

**Shared helper.** The per-`(player, game)` "is this visible? if so score it"
logic is extracted from `tips` into one helper so `tips` and `match` cannot
drift. `tips` keeps iterating a group's games; `match` calls it for one game.

## 5. GraphQL surface

```graphql
match(gameId: ID!): MatchDetail

type MatchDetail {
  gameId: ID!
  homeTeam: Team!
  awayTeam: Team!
  kickoff: DateTime!
  deadlinePassed: Boolean!     # server-derived (timeflags), reused
  resultPending: Boolean!      # server-derived (timeflags), reused
  actual: MatchScore           # null until there is a score to show
  rows: [Tip!]!                # reuses the existing Tip + PointsBreakdown
}

type MatchScore {
  homeScore: Int!
  awayScore: Int!
  provisional: Boolean!        # true = live "if it ended now"; false = official
  source: String               # "thesportsdb" when provisional; null when official
  sourceStatus: String         # SportsDB strStatus, e.g. "2H" — shown on the page
  ninetyMinuteUncertain: Boolean!
}
```

`Tip` and `PointsBreakdown` are reused unchanged from the `tips`/`perfects`
resolvers.

## 6. The 90-minute caveat (knockouts)

xpool scores knockouts on the **90-minute** result (`.specs/SCORING.md §5`), but a
live knockout can be in extra time / penalties. Such a match carries
`ninetyMinuteUncertain: true` (the existing flag: `round != GroupStage`); the page
shows the live score but flags the provisional **points** as 90-minute-uncertain.
Group games have no ET, so theirs are exact. Unlike consumer #1 (which hides
source status — see memory `reported-results-no-status-marker`), here surfacing
the live status (`2H`, etc.) is the whole point, so the page **does** show it.

## 7. Frontend

- **Route:** `/match/:gameId` → `MatchPage`, rendering the three states from
  `match(gameId)`:
  - **Upcoming** — teams, kickoff, deadline countdown; predictions appear under
    the same visibility rules (own always; others once open).
  - **Live** — the live score + `sourceStatus`, and each player's provisional
    points/breakdown, visibly marked provisional ("if it ended now").
  - **Finished** — the official score + final points (same grid, `provisional:
    false`).
- **Polling:** the page refetches `match(gameId)` every **60s only while
  `actual.provisional == true`**; no polling in other states. (Server cache floor
  means this never raises the SportsDB call rate above one hit / match / 60s /
  container.)
- **Entry points:** each game row in **Today** and **Schedule** links to
  `/match/:gameId`. No new nav item.

## 8. Testing (per `.specs/TESTING.md`; no live network in any test)

- **`api` `match` resolver** — `InMemoryRepository` + stub `ReportedResultSource`:
  - official-takes-priority (entered result → `provisional: false`, no source call);
  - live path (in-window, not entered, stub returns a score → `provisional: true`,
    `sourceStatus` carried);
  - visibility gating (a hidden prediction stays hidden pre-kickoff; all open
    once `time_open`);
  - knockout → `ninetyMinuteUncertain: true`; group → exact;
  - graceful `actual: null` when the source is absent/errors or the match is
    outside its live window.
- **`web`** — unit-render `MatchPage` for upcoming / live / finished fixtures.
- **E2E (Playwright)** — stubbed SportsDB source, an in-progress game: open
  `/match/:id`, assert the live score + provisional points render and are marked
  provisional. The source is stubbed/off in the suite (consistent with how it is
  shut down during testing).

## 9. Out of scope (explicit)

- Writing or auto-entering results — stays the admin's `submitGroup` flow (#1).
- Adding the bulk `/livescore/4429` feed or any new `sportsdb` method.
- Venue / stats / lineups on the match page (a possible later use of the route).
- The player-detail page (#3) — separate PRD.

## 10. Deployment delta

None beyond #1. Same `THESPORTSDB_API_KEY`; if unset, `NullSource` makes every
match page degrade to predictions-only with no live score. No new infra, no new
secret, no background job.
