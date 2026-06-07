# tmux dev-session restarter — design

**Date:** 2026-05-18
**Status:** approved

## Goal

A single script, `bin/tmux`, that brings up the full xpool local dev stack in a
tmux session and re-attaches to it on subsequent runs. Modelled on
`../../Southsiders/membership-new/bin/tmux`, adapted to xpool's ordering
constraints.

## Behavior

1. **Re-attach shortcut.** If a tmux session named `xpool` already exists,
   `tmux -CC attach` to it and exit. No data or containers are touched.
2. **Bootstrap (fresh session only).** Runs *before* any panes are created so
   ordering is guaranteed:
   - `docker compose up -d` — DynamoDB Local (:8000) + MailHog
   - poll `nc -z localhost 8000` until DynamoDB accepts connections
     (the container has no healthcheck)
   - `cargo run -p xtask -- import tournaments/fwc26.json`
   - `cargo run -p xtask -- seed`
3. **Create panes**, then attach with `tmux -CC` (iTerm2 control mode).

DynamoDB Local runs in-memory, so import + seed must re-run after any container
restart — the bootstrap always runs on a fresh session for that reason.

## Layout

```
┌───────────┬─────────────┐
│           │ docker logs │   pane 1: docker compose logs -f
│           ├─────────────┤
│  claude   │ cargo api   │   pane 2: cargo run -p api
│  (pane 0) ├─────────────┤
│           │ web npm dev │   pane 3: cd web && npm run dev
└───────────┴─────────────┘
```

- **Pane 0 (left):** launches `claude` — the primary interactive session,
  focused on attach.
- **Right column:** three stacked panes — docker logs, API, web dev server.

## Details

- `SESSION="xpool"`; project directory hardcoded to `~/Private/SoccerPool/xpool`,
  matching the membership-new convention.
- `DYNAMO_ENDPOINT=http://localhost:8000` is exported for the bootstrap steps.
  tmux panes do not inherit the script's environment, so the API pane receives
  it inline on its `send-keys` command.
- Panes are created at fixed indices for deterministic *initial* layout, and
  each is tagged with a stable `@role` pane option (`claude`/`docker`/`api`/`web`)
  — see the worktree-switching addendum below for why role beats index.
  `select-layout` evens out the three right panes; `pane-border-status` +
  `pane-border-format` surface the role on each pane's border.
- The legacy `archive/` GAE app is untouched — this only drives the new stack.

## Trade-off accepted

The bootstrap blocks the terminal while `cargo run -p xtask` compiles (slow on a
cold `target/`). Bootstrapping inside a pane would lose the strict ordering
guarantee, so blocking is accepted; the output is visible and it is a one-time
cost per fresh session.

## Addendum — `bin/switch` (worktree switching)

> **Superseded (2026-06-06):** `bin/switch` was folded into `bin/tmux
> [worktree]`. See
> `docs/superpowers/specs/2026-06-06-unified-tmux-dev-session-design.md`. The
> rationale below (single-stack, per-worktree `target/`, `@role` markers, port
> teardown) still holds and carries over to `bin/tmux`.

**Date:** 2026-06-06
**Status:** approved

`bin/switch <worktree>` repoints the running session's `api` + `web` servers at a
git worktree (`.claude/worktrees/<name>`, or the main checkout with no arg),
leaving the `docker` pane and DynamoDB untouched.

### Single-stack model

The stack uses fixed ports (`:3000` api, `:5173` web, `:8000` DynamoDB), so only
one worktree's servers can run at a time. "Switching" is therefore *stop current,
start target* — not parallel stacks. The shared in-memory DynamoDB is reused
across switches; `import` + `seed` only need re-running if the target branch
changed schema or seed data.

### Per-worktree `target/` (no shared `CARGO_TARGET_DIR`)

An earlier revision shared the main checkout's `target/` across worktrees to skip
cold rebuilds. That was **reverted**: with a shared target, cargo treats the
`api`/`web` package (same name/version) as already built and serves a binary
compiled from *another* worktree's sources — so `bin/switch <wt>` could silently
run code that isn't in `<wt>`. This actually happened in practice and masked
which branch was live. Each worktree now builds into its own `target/`; the
first build per worktree is a cold compile, accepted in exchange for always
running the code you switched to.

The api is launched with `LOCAL_AUTH_ISSUER=1` so the dev-login route is mounted:
`.env` lives only in the main checkout, and although dotenvy searches ancestor
dirs, relying on that from a worktree is fragile — so the flag is set explicitly.

### Why `@role`, not pane index or title

The original `bin/tmux` targeted panes by index. Indices **renumber** when panes
are added or closed, so a script that hardcodes `0.2`/`0.3` sends commands to the
wrong panes once the layout drifts (observed in practice: a second vite spawned
on `:5174` while the original kept `:5173`). Pane **titles** are no better here —
the fish/tide prompt rewrites `pane_title` to the last command line on every
keystroke. A pane-scoped user option (`@role`) is the one marker nothing else
touches, so `bin/switch` (and `bin/tmux`) target panes by role.

### Robust teardown

`bin/switch` stops the old stack by **port** (`lsof` on `:3000`/`:5173` → `kill`)
*and* sends `C-c` to the role-tagged panes, then waits for the ports to free
before relaunching. Port-based teardown kills a server wherever it landed —
including a stray that fell back to an unexpected port — which index- or
pane-targeted teardown cannot guarantee.
