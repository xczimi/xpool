# bin/local-dev option to start from a fresh production data snapshot

Status: deferred build — part of the cluster/backend-infra plan (Task 1), docs/superpowers/plans/2026-06-27-cluster-backend-infra.md
Area: bin / tooling

## Idea

Add a variation (or flag/argument) to `bin/local-dev` so it can start the dev
session seeded from a **freshly pulled production data snapshot**, instead of
whatever stale data the branch table currently holds.

## Motivation

Reproducing prod-shaped behaviour locally (live scores, real players, edge
cases) is far easier against current prod data. Today seeding involves separate
steps and the per-branch table wrinkle (see [[per-branch-tables-vs-pull-data]]:
`pull-data` seeds `xpool-master` but a worktree reads `xpool-<branch>`). A single
`bin/local-dev --fresh` (or similar) that pulls a snapshot and loads it into the
**current branch's** table would remove that friction.

## Sketch

- New flag, e.g. `bin/local-dev --fresh` / `--pull-snapshot` (default off so the
  normal fast start is unchanged — it's an opt-in argument, not new default
  behaviour).
- Pull a current prod snapshot (reuse existing `pull-data` / snapshot tooling in
  `bin/` and `snapshots/`).
- Load it into the branch table the dev session actually reads
  (`xpool-<branch>`), not just `xpool-master`.
- Keep it non-destructive / clearly scoped to local; never touch prod.

## Resolved decisions (2026-06-27 grill)

- **Opt-in flag** (e.g. `bin/local-dev --fresh`); default off so the normal fast start
  is unchanged.
- **Source:** load the latest cached snapshot under `snapshots/` by default; pulling a
  fresh one stays a separate explicit step (reuse `bin/pull-data`).
- **Branch-table targeting:** load into the **current branch's** table (`xpool-<branch>`),
  fixing the master-vs-branch mismatch ([[per-branch-tables-vs-pull-data]]).
- **Do NOT auto-blank dev auth/clock** — separate concern; keep it non-destructive and
  local-only (never touch prod).
- Cluster: `cluster/backend-infra` (Wave 1). `bin/` tooling only.
