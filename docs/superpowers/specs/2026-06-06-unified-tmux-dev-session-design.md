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

In doing so, factor out the part `bin/tmux` and the e2e bootstrap genuinely
share — "bring the local stack up and seed it" — into one primitive,
`bin/local-stack`, that both wrap.

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
  (port + `C-c`), and relaunching them against the worktree.

A third script, `web/scripts/e2e-stack.sh`, independently boots the same stack
for Playwright (docker up + wait + import + seed + api), using a fresh unique
table per run.

Problems this design fixes:

- **Two entry points for one concern** ("make my dev session exist and point it
  where I say").
- **Triplicated stack logic** — docker-up + wait-for-port + import/seed +
  lsof port helpers are copied across `tmux`, `switch`, and `e2e-stack.sh`.
- **Drifted launch commands** — `bin/tmux` launches api without
  `LOCAL_AUTH_ISSUER`; `bin/switch` with it. Launch knowledge lives in
  `send-keys` strings.
- **No pane restoration** — closing the `api`/`web` pane has no clean recovery.
- **The in-memory-wipe footgun** — DynamoDB Local is in-memory; a docker
  restart silently empties it and you must remember to re-`import`/`seed`.
- **Cross-branch stale data** — a shared `xpool` table means switching to a
  branch with a different schema/seed silently tests new code against old data.

## Non-goals

- **No containerising of api/web.** Compose stays infra-only (DynamoDB Local +
  MailHog). The Rust/Vite dev loop is worse in Docker.
- **No multi-stack / per-worktree ports.** One `xpool` tmux session, fixed
  ports (`:3000` api, `:5173` web, `:8000` DynamoDB). Switching a worktree
  repoints the single running stack. (Data *is* isolated per branch — see
  below — but the running processes/ports are not.)
- **No state file.** Session state lives in tmux pane options; the per-branch
  table name is *derived*, never stored.
- **No sharing of the api *process* launch with e2e.** e2e needs build-then-run
  a detached binary with a PID file for teardown; dev runs `cargo run` in a
  foreground pane. Only the *infra + data* primitive is shared.

## Design

### `bin/` shape

```
bin/lib.sh        # sourced helpers: port_pids / kill_port, wait_for_port,
                  #   aws_env (region + dummy creds), table_for <dir>
bin/local-stack   # infra + data, headless, idempotent           [NEW core]
bin/run-api       # dev api launcher (foreground cargo run)       [NEW]
bin/run-web       # dev web launcher (conditional install + vite) [NEW]
bin/tmux          # interactive: panes + reconcile + attach; wraps local-stack
web/scripts/e2e-stack.sh   # wraps local-stack; keeps detached-api + PID
```

`bin/switch` is **deleted**.

### Per-branch data isolation

The DynamoDB table is derived from the target checkout's branch, so each branch
gets its own seeded data and switching never contaminates:

```sh
# bin/lib.sh — table_for <dir>
#   xpool-<branch>, '/' -> '-'; detached HEAD -> xpool-<short-sha>
table_for() {
  local b
  b="$(git -C "$1" rev-parse --abbrev-ref HEAD)"
  [ "$b" = HEAD ] && b="$(git -C "$1" rev-parse --short HEAD)"
  echo "xpool-$(printf '%s' "$b" | tr '/' '-')"
}
```

- **Uniform rule, no special case:** `master` → `xpool-master`,
  `feat/dev-clock-game-presets` → `xpool-feat-dev-clock-game-presets`.
- The bare `xpool` table is unused by the wrapped flow; `.env`'s
  `XPOOL_TABLE=xpool` remains only as the fallback for running `api`/`xtask`
  bare, outside the wrappers.
- Same pattern as e2e (`xpool-e2e-<ts>`): shared container, isolated table.
- This **subsumes** the cross-branch reseed problem. `--reseed` shrinks to its
  true job: "I changed the schema *within* my current branch."

### `bin/local-stack` — the shared core

Headless, idempotent. Brings the stack up and ensures `$XPOOL_TABLE` is seeded,
then exits. Knows nothing about tmux or about which checkout — the caller sets
`XPOOL_TABLE` (and the import path / clock if it wants).

```
bin/local-stack [--reseed]
  source bin/lib.sh; aws_env
  docker compose up -d                      # idempotent; no-op if up
  wait_for_port 8000
  if --reseed: xtask drop-table
  count = aws dynamodb scan --table-name "$XPOOL_TABLE" --select COUNT \
            --endpoint-url http://localhost:8000   # missing table => reseed
  if missing/empty/--reseed: xtask import tournaments/fwc26.json; xtask seed
  else: echo "data present in $XPOOL_TABLE; pass --reseed to reset"
```

- `aws_env` exports `AWS_REGION` + dummy `AWS_ACCESS_KEY_ID` /
  `AWS_SECRET_ACCESS_KEY` — DynamoDB Local ignores the values but the SDK
  requires them present (matches the old `e2e-stack.sh`).
- Always run (boot *and* reconcile) by `bin/tmux`, so a docker restart that
  wiped the in-memory table is healed on the next invocation.
- e2e's table is unique every run → always missing → always seeded by the same
  self-heal path. e2e needs **no** special "fresh" flag.
- `xtask import`/`seed` both call `repo.ensure_table()` first (idempotent
  create-and-wait), so a brand-new `xpool-<branch>` table is created on first
  seed — no separate create step needed. The exported `XPOOL_TABLE` overrides
  `.env` (dotenvy does not overwrite an already-set var), so seeding and serving
  hit the same table.

### `bin/run-api` / `bin/run-web` — launch wrappers

Single source of truth for how a server starts; usable by `bin/tmux` (sent to a
pane) and by hand in the `shell` pane. Both default their target to their own
checkout (`git rev-parse --show-toplevel`, main-checkout fallback), like
`bin/tmux`.

```sh
# bin/run-api [target]
source "$(dirname "$0")/lib.sh"
t="${1:-$(git rev-parse --show-toplevel)}"
cd "$t" && DYNAMO_ENDPOINT=http://localhost:8000 LOCAL_AUTH_ISSUER=1 \
  XPOOL_TABLE="$(table_for "$t")" cargo run -p api
```

```sh
# bin/run-web [target]
t="${1:-$(git rev-parse --show-toplevel)}"
cd "$t/web" && { [ -d node_modules ] || npm install; } && npm run dev
```

- `XPOOL_TABLE` is the one knob the api/xtask read; `run-api` and `local-stack`
  both derive it via `table_for`, so seeding and serving always agree.
- `web` does not read `XPOOL_TABLE` (it's the SPA, proxying `/api` → `:3000`).
- **Conditional install** — `npm install` only when `node_modules` is absent.
- **`CARGO_TARGET_DIR` left unshared** — each checkout builds into its own
  `target/`. Sharing it lets cargo serve an `api` binary built from a *different*
  worktree (same package id). Correctness beats the saved compile time.

### `bin/tmux [worktree] [--reseed]` — the interactive wrapper

**One rule for the target (cwd-based):**

- **no arg** → the checkout you run it from: `git -C "$PWD" rev-parse
  --show-toplevel`, main checkout (`$PROJECT`) fallback. Each worktree ships its
  own `bin/tmux`, so running the one in front of you targets the code in front
  of you.
- **bare name** → `$PROJECT/.claude/worktrees/<name>`.
- **existing directory** → that directory (must contain `web/`).

**Every run = "make my world correct" (idempotent), in order:**

**1. `bin/local-stack`** with `XPOOL_TABLE=table_for(target)` (passing
`--reseed` through). Infra up + the branch's table seeded.

**2. Reconcile the 4 panes** (found by `@role` — indices renumber, the prompt
overwrites titles):

| Pane     | present                                                          | missing                             |
|----------|------------------------------------------------------------------|-------------------------------------|
| `claude` | leave it (it's you / the agent)                                  | recreate, run `claude`              |
| `api`    | restart **iff** `@target` or `@branch` changed, **or** `:3000` dead; else leave | recreate split, `bin/run-api <target>` |
| `web`    | restart **iff** `@target` or `@branch` changed, **or** `:5173` dead; else leave | recreate split, `bin/run-web <target>` |
| `shell`  | leave it                                                         | recreate, plain shell, cwd = target |

- api/web panes carry `@role` + `@target` (dir) + `@branch`. Restart triggers:
  a different target dir, a different branch (checkout-in-place changes both the
  code and, for api, the table), or a dead port (crash). A bare run on the
  current checkout with healthy servers touches nothing — it just recreates
  whatever pane you closed.
- The 4th pane is a **general-purpose shell** in the target checkout (replaces
  the old low-value `docker compose logs -f` tail). Logs on demand
  (`docker compose logs -f`); MailHog UI at `http://localhost:8025`.
- Recreate = `split-window` (+ set `@role`/`@target`/`@branch`) then
  `select-layout main-vertical` to normalise positions. Pane borders show the
  role (`pane-border-status top`).

**3. Attach** — only if `$TMUX` is unset (run from a plain terminal outside the
session). Run from inside (the `claude` pane) → reconcile and exit, no nested
attach. A fresh boot creates the session first, then attaches.

### `web/scripts/e2e-stack.sh` — refactored onto the core

Keeps its Playwright-specific bits, delegates the shared part:

```
set XPOOL_TABLE=xpool-e2e-<ts>, XPOOL_NOW=<mid-tournament>, INVITE_CODE_SECRET
source bin/lib.sh
bin/local-stack                 # docker + wait + seed the fresh table
kill_port 3000                  # from lib.sh (was duplicated)
cargo build -p api; run the binary detached; write web/.e2e-api.pid
wait for GET /api/health
```

The fresh-table + detached-api-with-PID + clock-pinning stay; the triplicated
docker/wait/seed/lsof logic is gone.

## e2e coexistence (behavioural note)

`e2e-stack.sh` and the dev session share docker but are otherwise isolated:

- **Data: safe.** e2e's `xpool-e2e-<ts>` is unrelated to the dev
  `xpool-<branch>` tables; teardown drops it.
- **Docker: shared.** Same compose, left running.
- **`:3000`: collision.** e2e kills whatever is on `:3000` (your dev api), runs
  its own, stops it on teardown — so after an e2e run the dev api is dead.
- **`:5173`: reused.** Playwright's `webServer` has `reuseExistingServer`.

**Synergy:** after an e2e run, `bin/tmux` self-heals — `:3000` dead → restart
the dev api; the branch's table is untouched, so no reseed.

## Testing

Shell tooling — verified by exercising the real scenarios manually:

1. **Fresh boot** (no session, docker down) → bootstrap, 4 panes, attach.
2. **Close a pane + rerun** → only the closed pane recreated; servers untouched.
3. **Crash api + rerun** → dead `:3000` detected, api restarted; web untouched.
4. **`bin/tmux <worktree>` jump** → api/web repointed/restarted at the worktree
   against `xpool-<that-branch>`; docker untouched; `@target`/`@branch` updated.
5. **Checkout a different branch in place + rerun** → `@branch` mismatch →
   api/web restart, api on the new branch's table.
6. **Docker restart → rerun** → empty table detected, auto-reseed.
7. **`--reseed`** → drop + import + seed the current branch's table.
8. **Post-e2e recovery** → `bin/tmux` restarts the dead dev api, no reseed.
9. **Run from inside the claude pane** → reconciles without a nested attach.
10. **e2e still green** → `npm run e2e` boots via the refactored `e2e-stack.sh`.

## Resolved decisions (from the grill)

| #  | Decision |
|----|----------|
| 1  | api/web stay **native**; compose is infra-only. |
| 2  | **One script** `bin/tmux [worktree]`; `bin/switch` deleted. |
| 3  | Bare run = **cwd-based** target (the checkout you run it from). |
| 4  | Per-pane reconcile table; api/web also **force-restart on dead port**. |
| 5  | **Conditional** `npm install`; `CARGO_TARGET_DIR` unshared. |
| 6  | **Self-healing** bootstrap (item-count check → seed if empty). |
| 7  | 4th pane repurposed from docker-logs to a **shell**. |
| 8  | Launch commands wrapped in **`bin/run-api` / `bin/run-web`**. |
| 9  | Shared **`bin/local-stack`** core + **`bin/lib.sh`**; e2e folded onto it. |
| 10 | **Per-branch tables**, uniform `xpool-<branch>` (subsumes reseed-on-switch). |
