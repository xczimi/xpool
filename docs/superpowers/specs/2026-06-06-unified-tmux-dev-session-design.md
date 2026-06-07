# Unified `bin/tmux [worktree]` dev session — design

**Date:** 2026-06-06
**Status:** approved
**Supersedes:** the `bin/switch` addendum in
`docs/superpowers/specs/2026-05-18-tmux-restarter-design.md`

## Goal

Collapse the two dev-session scripts (`bin/tmux` boot + `bin/switch` repoint)
into a single, idempotent `bin/tmux [worktree]` that is simple to use and
"self-healing": one command brings the world to a correct state — infra up,
data seeded, panes present, servers running and pointed where you asked —
whether the problem is a fresh machine, a closed pane, a crashed server, a
wiped database, or a worktree you want to peek at.

The driving constraints (from the design conversation):

- **Simple and straightforward.** One script, one rule, no flags to remember
  for the common case.
- **Mostly single-branch work.** Worktrees are for agentic fan-out and looking
  at variants, not day-to-day. The worktree feature must not clutter the common
  path.
- **Native app, dockerised infra.** `cargo run` (api) and Vite (web) run on the
  host for fast incremental rebuild + HMR; only stateful infra (DynamoDB Local,
  MailHog) runs in `docker compose`. This split is kept — containerising the
  app layer trades away the fast dev loop.

## Background

Two scripts exist today:

- `bin/tmux` — first-boot bootstrap: `docker compose up -d` → wait → `xtask
  import` → `xtask seed`, then create 4 panes (`claude` / `docker` / `api` /
  `web`) tagged with stable `@role` markers, launching api/web pointed at the
  main checkout. Re-running early-exits to `attach` — it does **not** restore
  closed panes.
- `bin/switch [worktree]` — repoints the running session's api + web at a
  worktree by finding the panes by `@role`, stopping the old servers
  (port + `C-c`), and relaunching them with `LOCAL_AUTH_ISSUER=1` against the
  worktree. Docker/DynamoDB left running (shared infra).

Problems this design fixes:

- **Two entry points for one concern** ("make my dev session exist and point it
  where I say").
- **Drifted launch commands** — `bin/tmux` launches api without
  `LOCAL_AUTH_ISSUER`; `bin/switch` with it. The launch knowledge lives in
  `send-keys` strings in two places (and a third copy in
  `web/scripts/e2e-stack.sh`).
- **No pane restoration** — closing the `api`/`web` pane has no clean recovery.
- **The in-memory-wipe footgun** — DynamoDB Local is in-memory; a docker
  restart silently empties it and you must remember to re-`import`/`seed`.

## Non-goals

- **No containerising of api/web.** Compose stays infra-only (DynamoDB Local +
  MailHog). Decided explicitly; the Rust/Vite dev loop is worse in Docker.
- **No multi-stack / per-worktree ports.** One `xpool` session, fixed ports
  (`:3000` api, `:5173` web, `:8000` DynamoDB). Switching a worktree repoints
  the single running stack.
- **No change to e2e.** `web/scripts/e2e-stack.sh` keeps its own launcher
  (it needs build-then-run-binary + background + PID file + a unique table).
  Sharing `bin/run-api` with e2e is a possible future dedup, not in scope.
- **No state file.** Session state lives in tmux pane options only.

## Design

### Invocation

```sh
bin/tmux              # target = the checkout you run it from
bin/tmux <worktree>   # jump the single stack to that worktree
bin/tmux [...] --reseed   # force drop-table && import && seed
```

**One rule for the target (cwd-based, option (ii) from the grill):**

- **no arg** → the checkout you run it from: `git -C "$PWD" rev-parse
  --show-toplevel`, with the main checkout (`$PROJECT`) as fallback if cwd is
  not an xpool checkout. Each worktree ships its own `bin/tmux`, so running the
  one in front of you targets the code in front of you.
- **bare name** → `$PROJECT/.claude/worktrees/<name>`.
- **an existing directory** → that directory (must contain `web/`).

`--reseed` forces a full data reset for the case self-healing can't detect:
a branch whose schema/seed changed, where the table is present but *stale*
(not empty).

### Every run = "make my world correct" (idempotent)

A single invocation performs three reconciliations in order. Each is a no-op
when already satisfied, so the script is safe to run repeatedly.

**1. Infra + data (self-healing).**

- `docker compose up -d` — idempotent; no-op if already up. Always detached, so
  infra survives pane churn and worktree jumps.
- Wait for DynamoDB Local on `:8000`.
- `aws dynamodb scan --table-name "$XPOOL_TABLE" --select COUNT
  --endpoint-url http://localhost:8000` — if the table is **missing or empty**
  (or `--reseed` was passed) → `xtask import` + `xtask seed`; otherwise skip and
  print a one-line reminder (`data present; pass --reseed to reset`). The script
  exports `AWS_REGION` + dummy `AWS_ACCESS_KEY_ID`/`AWS_SECRET_ACCESS_KEY` first
  — DynamoDB Local ignores the values but the AWS SDK requires them present
  (same as `e2e-stack.sh`). `XPOOL_TABLE` defaults to `xpool` for dev.

This runs on **every** invocation (boot and reconcile), which is what kills the
in-memory-wipe footgun: after a docker restart, the next `bin/tmux` notices the
empty table and reseeds.

**2. Reconcile the 4 panes** (found by `@role` marker — indices renumber and the
prompt overwrites titles, so neither is load-bearing):

| Pane     | present                                                        | missing                              |
|----------|---------------------------------------------------------------|--------------------------------------|
| `claude` | leave it (it's you / the agent)                               | recreate, run `claude`               |
| `api`    | restart **iff** `@target` changed **or** `:3000` not listening; else leave | recreate split, launch at target     |
| `web`    | restart **iff** `@target` changed **or** `:5173` not listening; else leave | recreate split, launch at target     |
| `shell`  | leave it                                                       | recreate, plain shell, cwd = target  |

- "Restart only if `@target` changed or the port is dead" means a bare run on
  the current checkout with healthy servers touches nothing — it just recreates
  whatever pane you closed. The dead-port check also force-restarts a crashed
  server even when the pane is still open.
- The 4th pane is a **general-purpose shell** in the target checkout (replaces
  the old low-value `docker compose logs -f` tail). Logs remain available on
  demand (`docker compose logs -f`); MailHog UI is `http://localhost:8025`.
- Recreate = `split-window` (+ re-set `@role`/`@target`) then `select-layout
  main-vertical` to normalise positions so a restored pane doesn't land oddly.

**3. Attach** — only if `$TMUX` is unset (run from a plain terminal outside the
session). Run from inside (the `claude` pane) → reconcile and exit, no nested
attach. A fresh boot creates the session first, then attaches.

### Launch wrappers — single source of truth

The api/web launch commands move out of `send-keys` strings into two small
`bin/` scripts (matching the existing `bin/` convention — `deploy-*`, `tmux`).
`bin/tmux` sends `bin/run-api <target>` / `bin/run-web <target>` to the panes;
you can also run them by hand in the `shell` pane with the same environment.
Both default their target to their own checkout (`git rev-parse --show-toplevel`,
main-checkout fallback), exactly like `bin/tmux`.

```sh
# bin/run-api [target]
cd "<target>" && DYNAMO_ENDPOINT=http://localhost:8000 LOCAL_AUTH_ISSUER=1 \
  cargo run -p api
```

```sh
# bin/run-web [target]
cd "<target>/web" && { [ -d node_modules ] || npm install; } && npm run dev
```

- `DYNAMO_ENDPOINT` + `LOCAL_AUTH_ISSUER` are set inline. Harmless on the main
  checkout (its `.env` sets the same values; dotenvy also walks up the tree),
  and robust for a worktree regardless of nesting.
- **Conditional install** — `npm install` only when `node_modules` is absent
  (fast on the main checkout, auto-bootstraps a fresh worktree). The rare
  "deps changed but dir exists" case surfaces as an obvious failure; re-run.
- **`CARGO_TARGET_DIR` left unshared** — each checkout builds into its own
  `target/`. Sharing it lets cargo serve an `api` binary built from a *different*
  worktree (same package id), so you'd run code that isn't in the worktree you
  switched to. Correctness beats the saved compile time (carried over from
  `bin/switch`).

### State mechanism

Pure tmux pane options, no state file:

- `@role` — `claude` / `api` / `web` / `shell` (existing mechanism, extended).
- `@target` — **new**; the checkout path an api/web pane was launched against,
  so reconcile can compare desired-vs-current and avoid needless restarts.

Pane borders show the role (`pane-border-status top` +
`pane-border-format`), so the layout stays self-describing.

### Removal + doc updates

- **Delete `bin/switch`** outright (no shim).
- Repoint references to `bin/tmux <worktree>` in: `README.md`, `CLAUDE.md`, and
  the `bin/switch` addendum of
  `docs/superpowers/specs/2026-05-18-tmux-restarter-design.md`.

## e2e coexistence (documented; no code change)

`web/scripts/e2e-stack.sh` and the dev session share docker but are otherwise
mostly isolated:

- **Data: safe.** e2e uses a unique table `xpool-e2e-<timestamp>` and drops it
  on teardown; the dev `xpool` table is never touched.
- **Docker: shared.** Same compose, left running; e2e reuses DynamoDB Local +
  MailHog.
- **`:3000`: collision.** e2e kills whatever is on `:3000` (your dev api),
  runs its own, and stops it on teardown — so after an e2e run the dev api is
  dead.
- **`:5173`: reused.** Playwright's `webServer` has `reuseExistingServer`.

**Synergy:** after an e2e run, `bin/tmux` self-heals — it sees `:3000` dead and
restarts the dev api; the dev table is intact (separate table) so no reseed.
The thing that's annoying today becomes one command.

## Testing

Shell tooling — verified by exercising the real scenarios manually:

1. **Fresh boot** (no session, docker down) → full bootstrap, 4 panes, attach.
2. **Close a pane + rerun** → only the closed pane is recreated; running servers
   untouched.
3. **Crash api + rerun** → dead `:3000` detected, api restarted; web untouched.
4. **`bin/tmux <worktree>` jump** → api/web repointed and restarted at the
   worktree; docker/data untouched; `@target` updated.
5. **Docker restart → rerun** → empty table detected, auto-reseed.
6. **Post-e2e recovery** → after an e2e run, `bin/tmux` restarts the dead dev
   api with no reseed.
7. **Run from inside the claude pane** → reconciles without a nested attach.

## Resolved decisions (from the grill)

| # | Decision |
|---|----------|
| 1 | api/web stay **native**; compose is infra-only. |
| 2 | **One script** `bin/tmux [worktree]`; `bin/switch` deleted. |
| 3 | Bare run = **cwd-based** target (the checkout you run it from). |
| 4 | Per-pane reconcile table; api/web also **force-restart on dead port**. |
| 5 | **Conditional** `npm install`; `CARGO_TARGET_DIR` unshared. |
| 6 | **Self-healing** bootstrap (item-count check → seed if empty). |
| 7 | 4th pane repurposed from docker-logs to a **shell**. |
| 8 | Launch commands wrapped in **`bin/run-api` / `bin/run-web`**. |
