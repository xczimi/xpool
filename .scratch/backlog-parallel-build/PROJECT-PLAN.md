# Backlog parallel build — project plan (round 2, 2026-06-27)

Round 1 (`GOAL.md`, 2026-06-21) **shipped and merged**: perfect-page picker+sort,
match-page venue+prediction-stats, live max-achievable, force-refresh-live-score,
timeline-schedule, own-player-page-access — plus best-thirds-table and
knockout-tip-labels landed 2026-06-27, and pr3-review #24–31 are all done. Their
PRD headers are now marked done (verified in git, not assumed).

This round 2 plan covers the **remaining live backlog** — dominated by the ideas
filed today — and orchestrates it as triage → design → parallel implement across a
fan-out agent team.

Deliverable of *this* document: the plan. Execution launches via the prompt in §8.

---

## 1. Remaining live backlog (verified against code + git, not PRD headers)

Design state: **N** = needs triage + design.

| Idea | State | Primary file surface | Notes |
|---|---|---|---|
| match-page-sort-predictions | N | `web/src/pages/MatchPage.tsx` | new |
| match-page-what-if-scores | N | `MatchPage.tsx` (+ scoring) | new |
| live-match-highlight-self | N | `MatchPage.tsx` | new |
| knockout-only-scoreboard | N | `ScoreboardPage.tsx` + `crates/domain` | new |
| head-to-head-players | N | **new** page + `App.tsx` route | new |
| player-points-timeline-chart | N | **new** page/chart + `App.tsx` route | new; needs chart lib decision |
| ses-deadline-reminders | N | `crates` mail + `infrastructure/*.tf` + EventBridge/Lambda | incl. hourly trigger |
| local-dev-fresh-snapshot | N | `bin/` tooling | new |
| knockout-subgroup-anchors | N | `pages/mytips/*` + routing | unblocked (tip-labels shipped); "needs discussion" |
| mobile-prediction-entry | N | prediction entry UX (`mytips/`) | overlaps mytips |
| page-one-liner-intros | N | **every page** | cross-cutting |
| fixed-width-display | N | global CSS | cross-cutting; vague — triage may drop |

**Triage-to-confirm (probably already done):**
- `timezone-clarity` — largely shipped via "humane finalize countdown" (scaled
  granularity + absolute local deadline). Verify remainder; likely mark done.

---

## 2. Triage gate (resolve in P0 before build effort)

1. **timezone-clarity** — read the shipped countdown work; mark done or scope the
   sliver that remains.
2. **fixed-width-display** — vague "revisit"; triage to a concrete scope or drop.
3. **knockout-subgroup-anchors** — flagged "needs more discussion". In or out this
   round? (Recommend: in, Wave 2 — tip-labels has shipped so it's unblocked.)
4. **player-points-timeline-chart** — pick the charting approach (hand-rolled SVG
   vs a lib) before design, since it drives the plan and bundle size.
5. **Scope confirmation** — Wave 1 = 4 clusters below; Wave 2 = the collide-y rest.
   Confirm the cut.

---

## 3. Clusters — parallelism by file-surface ownership

Hard rule (carried from round 1, which worked): **one file, one owner per wave** so
no two agents edit the same file. Each cluster = one git worktree + branch + impl
agent; multi-feature clusters commit sequentially within.

### Wave 1 — 4 disjoint clusters, fully parallel

| Cluster | Branch | Ideas | Owns |
|---|---|---|---|
| `match-page` | `cluster/match-page` | sort-predictions, what-if-scores, highlight-self | `MatchPage.tsx` (+ its components) |
| `standings` | `cluster/standings` | knockout-only-scoreboard | `ScoreboardPage.tsx` + `domain` scoring |
| `player-analytics` | `cluster/player-analytics` | head-to-head, points-timeline-chart | **new** page files + `App.tsx` routing |
| `backend-infra` | `cluster/backend-infra` | ses-deadline-reminders, local-dev-fresh-snapshot | `crates` mail, `infrastructure/*.tf`, `bin/` |

`backend-infra` shares no frontend surface → runs fully independent.

### Wave 2 — collide with Wave 1 or each other (run after it lands)

| Cluster | Branch | Ideas | Why deferred |
|---|---|---|---|
| `mytips-nav` | `cluster/mytips-nav` | knockout-subgroup-anchors, mobile-prediction-entry | both edit `pages/mytips/*`; serialize |
| `cross-cutting-ux` | `cluster/cross-cutting-ux` | page-one-liner-intros, (fixed-width-display if kept) | touch every page; wait until page features settle |

---

## 4. Shared seams (resolve at integration, not during build)

- **`crates/api/src/gql/query.rs`** — `standings`, `player-analytics`,
  `backend-infra` may each add a resolver. Each adds its own; reconcile on combine.
- **`web/src/App.tsx` routing** — `player-analytics` owns it; any other cluster
  needing a route flags it for the integrator.
- **`web/src/lib/standings.ts`** — `standings` + `player-analytics` both read it;
  changes append-only where possible.
- **`web/src/i18n/strings.ts`** — every UI cluster appends keys; trivially merged.

---

## 5. Phases

**P0 — Triage (short fan-out).** One agent per §2 question (confirm timezone-clarity
done, scope/drop fixed-width, decide anchors in/out, pick chart approach, confirm
the cut). Output: final cluster set.

**P1 — Design / grill.** Every remaining idea is `N` (none pre-grilled this round).
`superpowers:brainstorming` across them → batch **all** open questions into one
list → `/grill-me` in a single session to settle them → write resolved decisions
back into each `.scratch/<slug>/PRD.md` → `superpowers:writing-plans`, one plan per
cluster.

**P2 — Parallel build (Wave 1).** `superpowers:using-git-worktrees`: one worktree +
branch per cluster. Fan out with `superpowers:dispatching-parallel-agents` /
`subagent-driven-development` — one impl agent per cluster via
`superpowers:executing-plans`, each using `test-driven-development` and
`systematic-debugging`. Meet the §6 bar. Close each with `requesting-code-review` +
`verification-before-completion`.

**P2b — Wave 2.** After Wave-1 page clusters land, run `mytips-nav` then
`cross-cutting-ux`.

**P3 — Integration branch.** Combine all cluster branches into one
`backlog-parallel-build` branch (`superpowers:finishing-a-development-branch`),
resolving §4 seams. Re-verify the **combined** branch green end-to-end:
`cargo build/clippy/test` + `web` build/lint + e2e.

**P4 — Local review handoff.** `bin/pull-data prod snapshots/prod-snapshot.json
--reload`, then `bin/local-dev backlog-parallel-build` so every feature is clickable
against real-shaped data. **Stop. Hand back. Do not merge or deploy.**

---

## 6. Per-cluster quality bar (non-negotiable)

- `cargo build` / `cargo clippy -- -D warnings` / `cargo test` green.
- `web`: `npm run build` + `npm run lint` green.
- **An e2e proving the feature** — frontend work needs an end-to-end test, not just
  build+lint. ([[frontend-work-needs-e2e]])
- **Verify the rendered page visually**; new class names need CSS.
  ([[verify-frontend-visually]])
- Respect **tip-visibility gating**; never leak still-locked tips.
- **No `Date.now()` in the SPA** — server-authoritative clock only.
- E2E needs dev-stub auth (`web/.env.local` blanking `VITE_AUTH0_*`).
  ([[e2e-needs-dev-stub-auth]])
- A worktree reads `xpool-<branch>`, not `xpool-master`.
  ([[per-branch-tables-vs-pull-data]])

---

## 7. Completion bar

DONE when the single `backlog-parallel-build` branch contains all Wave-1 (and
chosen Wave-2) clusters, is verified green as a whole, and is seeded locally for
review. Then STOP and surface the branch + per-cluster summary for FINAL REVIEW.
**No merge to master. No deploy** — Peter's calls after review.

---

## 8. Paste-ready `/goal` launch prompt

```
/goal Act as TEAM LEADER executing .scratch/backlog-parallel-build/PROJECT-PLAN.md
(round 2). Lean on the superpowers plugin skills at every phase; do not freelance.

P0 TRIAGE: fan out one agent per open question in the plan's Triage Gate (confirm
timezone-clarity is done, scope-or-drop fixed-width-display, decide
knockout-subgroup-anchors in/out, pick the timeline chart approach, confirm the
Wave-1 cut). Report the final cluster set, then continue.

P1 DESIGN: superpowers:brainstorming over all remaining ideas; batch ALL open
questions into ONE list; run /grill-me in a single session to settle them; write
resolved decisions back into each .scratch/<slug>/PRD.md; then
superpowers:writing-plans — one plan per cluster.

P2 BUILD (Wave 1, PARALLEL): superpowers:using-git-worktrees — one worktree+branch
per Wave-1 cluster (match-page, standings, player-analytics, backend-infra). Fan
out with superpowers:dispatching-parallel-agents / subagent-driven-development, one
impl agent per cluster via superpowers:executing-plans, each using
test-driven-development + systematic-debugging. Meet the plan's per-cluster bar
(cargo + web green; an e2e per feature; visual check; tip-gating; no Date.now();
dev-stub auth; branch-table awareness). Close each with requesting-code-review +
verification-before-completion.

P2b WAVE 2: after Wave-1 page clusters land, run cluster/mytips-nav then
cluster/cross-cutting-ux.

P3 INTEGRATE: combine all cluster branches into ONE backlog-parallel-build branch
(superpowers:finishing-a-development-branch), resolving the query.rs / App.tsx /
standings.ts / strings.ts seams. Re-verify the COMBINED branch green end to end.

P4 REVIEW: bin/pull-data prod snapshots/prod-snapshot.json --reload, then
bin/local-dev backlog-parallel-build so I can click through everything.

COMPLETION BAR: stop when the single backlog-parallel-build branch holds all chosen
clusters, is verified green as a whole, and is seeded locally. Surface branch +
per-cluster summary for my FINAL REVIEW. Do NOT merge to master. Do NOT deploy.

Surface a question whenever a choice is genuinely mine; the P1 grill should catch
most. Otherwise keep working across turns until the completion bar is met.
```
