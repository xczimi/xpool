# Parallel backlog build — `/goal` orchestration prompt

The 8 new `needs-triage` ideas, built as 4 parallel clusters by a team-leader
loop. Paste the block below after `/goal` to start the autonomous loop. The loop
**stops for your final review** before anything merges to master or deploys.

## Clusters (worktree + branch per cluster — built in parallel)

| Cluster | Branch / worktree | Ideas | Main files |
|---|---|---|---|
| `perfect-page` | `cluster/perfect-page` | perfect-page-pool-picker, perfect-page-sort-by-player | `web/src/pages/PerfectPage.tsx` |
| `match-page` | `cluster/match-page` | match-page-venue, match-page-prediction-stats | `web/src/pages/MatchPage.tsx` |
| `live-scoring` | `cluster/live-scoring` | live-max-achievable-points, force-refresh-live-score | scoreboard + sportsdb live + `domain` |
| `schedule-nav` | `cluster/schedule-nav` | timeline-schedule, own-player-page-access | `SchedulePage.tsx` / `AuthBar`+`App.tsx` |

Pairing is by file surface so two agents never edit the same file. The shared
seam is `crates/api/src/gql/query.rs` (clusters may each add a resolver) — handle
it when combining branches.

---

## The `/goal` prompt

```
/goal Act as TEAM LEADER orchestrating the 8 new .scratch/ backlog ideas as 4
parallel sub-agent clusters. Stay coherent with this repo's previous feature
cycles by leaning on the superpowers plugin skills at every phase — do not
freelance a bespoke process.

CLUSTERS (one git worktree + branch each, built in PARALLEL):
  1. cluster/perfect-page  — perfect-page-pool-picker, perfect-page-sort-by-player
  2. cluster/match-page    — match-page-venue, match-page-prediction-stats
  3. cluster/live-scoring  — live-max-achievable-points, force-refresh-live-score
  4. cluster/schedule-nav  — timeline-schedule, own-player-page-access
Read each idea's .scratch/<slug>/PRD.md as the source of intent.

PHASE 0 — DESIGN, QUESTIONS BATCHED UP FRONT (before any code):
  - superpowers:brainstorming over all 8 ideas to surface intent and every
    open design question across the four clusters.
  - Collect ALL of those open questions into ONE list, then run the /grill-me
    skill to resolve them with ME in a single interactive session — grill me
    until every branch of each decision is settled. Do not start code until
    the grilling is done.
  - Write the resolved decisions back into each .scratch/<slug>/PRD.md, then use
    superpowers:writing-plans to produce one implementation plan per cluster.

PHASE 1 — BUILD IN PARALLEL:
  - superpowers:using-git-worktrees: one worktree + branch per cluster (names
    above) so the four build concurrently without collision.
  - Fan the work out with superpowers:dispatching-parallel-agents /
    subagent-driven-development — one implementation agent per cluster executing
    its plan via superpowers:executing-plans.
  - Each agent uses superpowers:test-driven-development (red-green-refactor) and
    superpowers:systematic-debugging on any failure.
  - Per-cluster bar: cargo build/clippy/test green; web build + lint green; an
    e2e proving the feature (frontend work needs an end-to-end test, not just
    build+lint); respect tip-visibility gating; no Date.now() in the SPA
    (server-authoritative clock only). Verify the rendered page visually, not
    just green checks.
  - Close each cluster with superpowers:requesting-code-review +
    verification-before-completion before calling it done.

PHASE 2 — JOINT INTEGRATION BRANCH (the deliverable):
  - Combine the four cluster branches into ONE integration branch
    `backlog-parallel-build` (use superpowers:finishing-a-development-branch).
    Resolve any crates/api/src/gql/query.rs resolver conflicts as you combine.
  - Re-verify the COMBINED branch green end to end (full workspace + web build +
    lint + e2e).
  - Seed my LOCAL stack from the recent prod snapshot for review:
    `bin/pull-data prod snapshots/prod-snapshot.json --reload`, then bring the
    stack up on the integration worktree (`bin/local-dev backlog-parallel-build`)
    so I can click through all 8 features against real-shaped data.

COMPLETION BAR — stop here, hand back to me:
  DONE when the single joint `backlog-parallel-build` branch contains all 4
  clusters, is verified green as a whole, and is seeded locally for my review.
  Then STOP and surface the branch + a per-cluster summary for my FINAL REVIEW.
  Do NOT merge to master. Do NOT deploy. Those are my calls after review.

Throughout: surface a question to me whenever a choice is genuinely mine; the
up-front /grill-me pass should catch most. Otherwise keep working across turns
until the completion bar is met.
```

---

## Notes

- **Local review data:** `snapshots/prod-snapshot.json` (refreshed 2026-06-21).
  `bin/pull-data prod snapshots/prod-snapshot.json --reload` skips AWS and just
  (re)loads that snapshot into local DynamoDB (`:8000`).
- **Why a joint branch, not 4 merges to master:** you want one place to review
  all 8 features together before deciding what deploys — the loop never lands
  code on master on its own.
- **Coherence:** the prompt names the specific superpowers skills per phase so
  the loop mirrors how features were built here before, rather than inventing a
  one-off workflow.
