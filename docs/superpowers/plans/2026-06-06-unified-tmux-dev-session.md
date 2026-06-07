# Unified `bin/tmux` Dev Session Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Collapse `bin/tmux` + `bin/switch` into one idempotent, self-healing `bin/tmux [worktree]`, backed by a shared `bin/local-stack` primitive that the e2e bootstrap also reuses, with per-branch DynamoDB tables.

**Architecture:** A tiny sourced helper library (`bin/lib.sh`) holds pure functions (`table_for`, port helpers). `bin/local-stack` is a headless "infra up + `$XPOOL_TABLE` seeded" primitive. `bin/run-api`/`bin/run-web` are dev launchers (the single source of truth for how a server starts). `bin/tmux` orchestrates a 4-pane tmux session (`claude`/`api`/`web`/`shell`), calling those pieces and reconciling panes by their `@role`/`@target`/`@branch` markers. `web/scripts/e2e-stack.sh` is refactored to wrap `bin/local-stack`.

**Tech Stack:** Bash, tmux (pane user-options `@role`/`@target`/`@branch`), Docker Compose (DynamoDB Local + MailHog), Rust (`cargo run -p api`/`xtask`), Vite/npm, the `aws` CLI (DynamoDB Local item-count check).

---

## Context the engineer needs

- **The spec:** `docs/superpowers/specs/2026-06-06-unified-tmux-dev-session-design.md`. Read it. The "Resolved decisions" table at the bottom is the quick reference.
- **The clock/env model:** the API reads config from `.env` via dotenvy itself. **These scripts must not source `.env`.** The only env override they apply is `XPOOL_TABLE` (per branch). The `aws` CLI is the one consumer that can't read `.env`, so it gets dummy creds inline.
- **Single stack:** fixed ports `:3000` (api), `:5173` (web), `:8000` (DynamoDB). Only one checkout's servers run at a time.
- **DynamoDB Local is in-memory** — wiped on container restart; the self-healing seed handles that.
- **Existing scripts to learn from / replace:** `bin/tmux` (current bootstrap), `bin/switch` (current repoint — being deleted), `web/scripts/e2e-stack.sh` (current e2e bootstrap — being refactored). Read all three before starting.
- **`xtask` subcommands:** `import <path>`, `seed`, `drop-table` (clap kebab-case). Both `import` and `seed` call `repo.ensure_table()` first, so a brand-new table is created on first seed.
- **`set -euo pipefail`** is used by every script. Mind that an unguarded failing command aborts the script — wrap "allowed to fail" commands with `|| true`.
- **Verification reality:** these are orchestration scripts driving docker/tmux/cargo. Per the spec, the orchestration is verified by **manual scenario runs**, not an automated harness. The one genuinely unit-testable piece is `table_for` (a pure function) — that gets a real test in Task 1.

## File Structure

| File | Responsibility |
|------|----------------|
| `bin/lib.sh` (new) | Pure sourced helpers: `table_for`, `port_pids`, `kill_port`, `wait_for_port`. No side effects on source. |
| `bin/lib.test.sh` (new) | Standalone assertions for `table_for`. |
| `bin/local-stack` (new) | Headless: docker up + wait + seed `$XPOOL_TABLE` if empty (or `--reseed`). |
| `bin/run-api` (new) | Dev API launcher; overrides only `XPOOL_TABLE`. |
| `bin/run-web` (new) | Dev web launcher; conditional `npm install` + Vite. |
| `bin/tmux` (rewrite) | Interactive orchestrator: resolve target → local-stack → reconcile 4 panes → attach. |
| `bin/switch` (delete) | Folded into `bin/tmux <worktree>`. |
| `web/scripts/e2e-stack.sh` (modify) | Refactor onto `bin/lib.sh` + `bin/local-stack`. |
| `README.md`, `CLAUDE.md`, `docs/superpowers/specs/2026-05-18-tmux-restarter-design.md` (modify) | Repoint `bin/switch` references to `bin/tmux <worktree>`. |

---

## Task 1: `bin/lib.sh` — pure helpers (+ test for `table_for`)

**Files:**
- Create: `bin/lib.sh`
- Test: `bin/lib.test.sh`

- [ ] **Step 1: Write the failing test**

Create `bin/lib.test.sh`:

```bash
#!/usr/bin/env bash
# Tests for bin/lib.sh pure helpers. Run: bash bin/lib.test.sh
set -uo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$HERE/lib.sh"

fails=0
check() {  # <description> <expected> <actual>
  if [ "$2" = "$3" ]; then
    echo "ok   - $1"
  else
    echo "FAIL - $1: expected '$2', got '$3'"; fails=$((fails + 1))
  fi
}

# table_for sanitises the branch into xpool-<branch>, '/' -> '-'.
# Use a throwaway git repo so the test is independent of the current branch.
tmp="$(mktemp -d)"
git -C "$tmp" init -q
git -C "$tmp" config user.email t@t.t
git -C "$tmp" config user.name t
git -C "$tmp" commit -q --allow-empty -m init
git -C "$tmp" branch -m master
check "master -> xpool-master" "xpool-master" "$(table_for "$tmp")"
git -C "$tmp" checkout -q -b feat/dev-clock
check "feat/dev-clock -> xpool-feat-dev-clock" "xpool-feat-dev-clock" "$(table_for "$tmp")"
rm -rf "$tmp"

[ "$fails" -eq 0 ] && { echo "all passed"; exit 0; } || { echo "$fails failed"; exit 1; }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `bash bin/lib.test.sh`
Expected: FAIL — `lib.sh: No such file or directory` (the source line fails because `bin/lib.sh` doesn't exist yet).

- [ ] **Step 3: Write `bin/lib.sh`**

Create `bin/lib.sh`:

```bash
#!/usr/bin/env bash
# Shared PURE helpers for the xpool dev-session scripts. Sourcing this file has
# NO side effects (no env mutation, no docker/tmux/network calls) — it only
# defines functions. Sourced by bin/local-stack, bin/run-api, bin/run-web,
# bin/tmux, and web/scripts/e2e-stack.sh.

# Canonical DynamoDB table for a checkout: xpool-<branch>, with '/' -> '-'.
# A detached HEAD (no branch name) falls back to xpool-<short-sha>.
table_for() {
  local dir="$1" branch
  branch="$(git -C "$dir" rev-parse --abbrev-ref HEAD 2>/dev/null)"
  if [ -z "$branch" ] || [ "$branch" = HEAD ]; then
    branch="$(git -C "$dir" rev-parse --short HEAD 2>/dev/null)"
  fi
  printf 'xpool-%s\n' "$(printf '%s' "$branch" | tr '/' '-')"
}

# PIDs listening on a TCP port (empty if none).
port_pids() {
  lsof -ti tcp:"$1" -sTCP:LISTEN 2>/dev/null || true
}

# Kill whatever is listening on a TCP port (no-op if nothing).
kill_port() {
  local pids
  pids="$(port_pids "$1")"
  if [ -n "$pids" ]; then
    # shellcheck disable=SC2086
    kill $pids 2>/dev/null || true
  fi
}

# Block until a TCP port accepts connections (~30s budget), else fail.
wait_for_port() {
  local port="$1" i
  for i in $(seq 1 60); do
    nc -z localhost "$port" 2>/dev/null && return 0
    sleep 0.5
  done
  echo "lib.sh: timed out waiting for :$port" >&2
  return 1
}
```

- [ ] **Step 4: Run test to verify it passes**

Run: `bash bin/lib.test.sh`
Expected: PASS — three `ok` lines then `all passed`, exit 0.

- [ ] **Step 5: Make scripts executable + commit**

```bash
chmod +x bin/lib.sh bin/lib.test.sh
git add bin/lib.sh bin/lib.test.sh
git commit -m "feat(bin): add lib.sh pure helpers (table_for, port helpers)"
```

---

## Task 2: `bin/local-stack` — the shared infra+data primitive

**Files:**
- Create: `bin/local-stack`

- [ ] **Step 1: Write `bin/local-stack`**

Create `bin/local-stack`:

```bash
#!/usr/bin/env bash
# xpool local-stack primitive: ensure docker infra is up and $XPOOL_TABLE is
# seeded, then exit. Headless and idempotent. Knows nothing about tmux or which
# checkout — the CALLER exports XPOOL_TABLE (bin/tmux derives it per branch;
# e2e-stack.sh sets a fresh unique table) and runs this from a checkout root
# (so `docker compose` finds docker-compose.yml and cargo finds the workspace).
#
#   bin/local-stack [--reseed]   (--reseed forces drop + import + seed)
#
# See docs/superpowers/specs/2026-06-06-unified-tmux-dev-session-design.md
set -euo pipefail

HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$HERE/lib.sh"

RESEED=0
[ "${1:-}" = "--reseed" ] && RESEED=1

: "${XPOOL_TABLE:?bin/local-stack: XPOOL_TABLE must be set by the caller}"
export XPOOL_TABLE
DYNAMO_ENDPOINT="${DYNAMO_ENDPOINT:-http://localhost:8000}"
export DYNAMO_ENDPOINT

echo "==> docker compose up -d"
docker compose up -d

echo "==> waiting for DynamoDB Local on :8000"
wait_for_port 8000

# The aws CLI is the only consumer that can't read .env, so give it dummy creds
# inline — DynamoDB Local ignores the values; the SDK only needs them present.
# A missing table makes `scan` exit non-zero -> empty count -> treated as "seed".
item_count() {
  AWS_REGION=us-east-1 AWS_ACCESS_KEY_ID=local AWS_SECRET_ACCESS_KEY=local \
    aws dynamodb scan --table-name "$XPOOL_TABLE" --select COUNT \
      --endpoint-url "$DYNAMO_ENDPOINT" --output text --query Count 2>/dev/null
}

if [ "$RESEED" = 1 ]; then
  echo "==> --reseed: dropping $XPOOL_TABLE"
  cargo run -q -p xtask -- drop-table || true
fi

count="$(item_count || true)"
if [ "$RESEED" = 1 ] || [ -z "$count" ] || [ "$count" = 0 ]; then
  echo "==> seeding $XPOOL_TABLE (import + seed)"
  cargo run -q -p xtask -- import tournaments/fwc26.json
  cargo run -q -p xtask -- seed
else
  echo "==> data present in $XPOOL_TABLE ($count items); pass --reseed to reset"
fi
```

- [ ] **Step 2: Verify it boots + seeds a fresh table**

```bash
chmod +x bin/local-stack
XPOOL_TABLE=xpool-localstack-smoke bin/local-stack
```

Expected: prints `docker compose up -d`, waits for `:8000`, then `seeding xpool-localstack-smoke (import + seed)`, and the two xtask lines (`imported tournament: …`, `seeded demo data: …`). Exit 0.

- [ ] **Step 3: Verify idempotency (second run skips seeding)**

Run: `XPOOL_TABLE=xpool-localstack-smoke bin/local-stack`
Expected: ends with `data present in xpool-localstack-smoke (N items); pass --reseed to reset` (no re-seed). Exit 0.

- [ ] **Step 4: Verify `--reseed` re-seeds**

Run: `XPOOL_TABLE=xpool-localstack-smoke bin/local-stack --reseed`
Expected: `--reseed: dropping xpool-localstack-smoke` then `seeding …` again. Exit 0.

- [ ] **Step 5: Clean up the smoke table + commit**

```bash
XPOOL_TABLE=xpool-localstack-smoke cargo run -q -p xtask -- drop-table || true
git add bin/local-stack
git commit -m "feat(bin): add local-stack infra+data primitive"
```

---

## Task 3: `bin/run-api` and `bin/run-web` — dev launchers

**Files:**
- Create: `bin/run-api`
- Create: `bin/run-web`

- [ ] **Step 1: Write `bin/run-api`**

Create `bin/run-api`:

```bash
#!/usr/bin/env bash
# Launch the dev API for a checkout (foreground). The ONLY env override is
# XPOOL_TABLE (per-branch). DYNAMO_ENDPOINT, LOCAL_AUTH_ISSUER, AWS_*, and the
# secrets all come from .env, self-loaded by the api via dotenvy. Default target
# is the current checkout. Usable by bin/tmux and by hand in the shell pane.
set -euo pipefail
HERE="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
source "$HERE/lib.sh"

target="${1:-$(git rev-parse --show-toplevel)}"
cd "$target"
exec env XPOOL_TABLE="$(table_for "$target")" cargo run -p api
```

- [ ] **Step 2: Write `bin/run-web`**

Create `bin/run-web`:

```bash
#!/usr/bin/env bash
# Launch the dev web server (Vite) for a checkout. Conditional install only.
# Vite reads web/.env.* itself (the Auth0 toggle) — not touched here.
# Default target is the current checkout.
set -euo pipefail
target="${1:-$(git rev-parse --show-toplevel)}"
cd "$target/web"
[ -d node_modules ] || npm install
exec npm run dev
```

- [ ] **Step 3: Verify `run-api` starts and serves health**

Make sure infra is up first (`XPOOL_TABLE=xpool-$(git rev-parse --abbrev-ref HEAD | tr / -) bin/local-stack`), then:

```bash
chmod +x bin/run-api bin/run-web
bin/run-api . &
RUN_API_PID=$!
# wait for it, then check health
for i in $(seq 1 60); do curl -fsS localhost:3000/api/health >/dev/null 2>&1 && break; sleep 1; done
curl -fsS localhost:3000/api/health && echo " <- api healthy"
kill "$RUN_API_PID" 2>/dev/null || true
```

Expected: the health endpoint responds (non-error), printing `… <- api healthy`.

- [ ] **Step 4: Verify `run-web` starts Vite on :5173**

```bash
bin/run-web . &
RUN_WEB_PID=$!
for i in $(seq 1 60); do curl -fsS localhost:5173 >/dev/null 2>&1 && break; sleep 1; done
curl -fsS -o /dev/null -w "web HTTP %{http_code}\n" localhost:5173
kill "$RUN_WEB_PID" 2>/dev/null || true
```

Expected: `web HTTP 200`.

- [ ] **Step 5: Commit**

```bash
git add bin/run-api bin/run-web
git commit -m "feat(bin): add run-api / run-web dev launchers"
```

---

## Task 4: rewrite `bin/tmux` — interactive orchestrator

**Files:**
- Modify (full rewrite): `bin/tmux`

- [ ] **Step 1: Replace `bin/tmux` entirely**

Overwrite `bin/tmux` with:

```bash
#!/usr/bin/env bash
# xpool dev session — one idempotent "make my world correct" command.
# Boots (or heals) a tmux session 'xpool' with 4 panes (claude / api / web /
# shell), brings infra + the per-branch table up via bin/local-stack, and points
# api/web at a target checkout. Re-running recreates only what's missing and
# restarts api/web only when the target/branch changed or the port is dead.
#
#   bin/tmux               target = the checkout you run it from
#   bin/tmux <worktree>    target = .claude/worktrees/<worktree> (or a dir)
#   bin/tmux [...] --reseed  force drop + import + seed of the target's table
#
# See docs/superpowers/specs/2026-06-06-unified-tmux-dev-session-design.md
set -euo pipefail

SESSION="xpool"
BIN="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PROJECT="$(git -C "$BIN" rev-parse --show-toplevel)"
source "$BIN/lib.sh"

# --- parse args: an optional worktree name/dir + an optional --reseed --------
RESEED=""
ARG=""
for a in "$@"; do
  case "$a" in
    --reseed) RESEED="--reseed" ;;
    *)        ARG="$a" ;;
  esac
done

# --- resolve target ---------------------------------------------------------
#   (no arg)     -> the checkout you run it from (cwd toplevel; PROJECT fallback)
#   existing dir -> that directory
#   bare name    -> $PROJECT/.claude/worktrees/<name>
if [ -z "$ARG" ]; then
  TARGET="$(git rev-parse --show-toplevel 2>/dev/null || echo "$PROJECT")"
elif [ -d "$ARG" ]; then
  TARGET="$(cd "$ARG" && pwd)"
else
  TARGET="$PROJECT/.claude/worktrees/$ARG"
fi
[ -d "$TARGET/web" ] || {
  echo "bin/tmux: not an xpool checkout (no web/): $TARGET" >&2
  git -C "$PROJECT" worktree list >&2
  exit 1
}
TABLE="$(table_for "$TARGET")"
BRANCH="$(git -C "$TARGET" rev-parse --abbrev-ref HEAD)"

# --- 1. infra + data (self-healing) -----------------------------------------
( cd "$TARGET" && XPOOL_TABLE="$TABLE" "$BIN/local-stack" $RESEED )

# --- tmux helpers -----------------------------------------------------------
pane_by_role() {
  tmux list-panes -s -t "$SESSION" -F '#{@role}'$'\t''#{pane_id}' 2>/dev/null \
    | awk -F'\t' -v r="$1" '$1==r {print $2; exit}'
}
pane_opt() { tmux display-message -p -t "$1" "#{$2}" 2>/dev/null; }

normalise_layout() {
  tmux set-option -w -t "$SESSION:0" pane-border-status top
  tmux set-option -w -t "$SESSION:0" pane-border-format ' #{?@role,#{@role},#{pane_title}} '
  tmux select-layout -t "$SESSION" main-vertical
}

# Tag + launch a server pane via its wrapper, recording target/branch.
launch_server() {  # <pane_id> <api|web>
  local pane="$1" role="$2"
  tmux set-option -p -t "$pane" @role "$role"
  tmux set-option -p -t "$pane" @target "$TARGET"
  tmux set-option -p -t "$pane" @branch "$BRANCH"
  tmux send-keys -t "$pane" "$BIN/run-$role '$TARGET'" C-m
}

# Stop a server pane's process by port + C-c, then wait for the port to free.
stop_server() {  # <pane_id> <port>
  kill_port "$2"
  tmux send-keys -t "$1" C-c
  local _
  for _ in $(seq 1 20); do
    [ -z "$(port_pids "$2")" ] && break
    sleep 0.25
  done
}

ensure_server() {  # <api|web> <port>
  local role="$1" port="$2" pane tgt br
  pane="$(pane_by_role "$role")"
  if [ -z "$pane" ]; then
    pane="$(tmux split-window -v -P -F '#{pane_id}' -t "$SESSION" -c "$TARGET")"
    launch_server "$pane" "$role"
    normalise_layout
    return
  fi
  tgt="$(pane_opt "$pane" @target)"
  br="$(pane_opt "$pane" @branch)"
  if [ "$tgt" != "$TARGET" ] || [ "$br" != "$BRANCH" ] || [ -z "$(port_pids "$port")" ]; then
    stop_server "$pane" "$port"
    launch_server "$pane" "$role"
  fi
}

ensure_simple() {  # <role> <launch-cmd-or-empty>
  local role="$1" cmd="$2" pane
  pane="$(pane_by_role "$role")"
  [ -n "$pane" ] && return
  pane="$(tmux split-window -v -P -F '#{pane_id}' -t "$SESSION" -c "$TARGET")"
  tmux set-option -p -t "$pane" @role "$role"
  [ -n "$cmd" ] && tmux send-keys -t "$pane" "$cmd" C-m
  normalise_layout
}

# --- 2. build a fresh session, or reconcile an existing one -----------------
if ! tmux has-session -t "$SESSION" 2>/dev/null; then
  tmux new-session -d -s "$SESSION" -c "$TARGET"
  tmux set-option -p -t "$SESSION:0.0" @role claude
  tmux send-keys -t "$SESSION:0.0" 'claude' C-m
  api_pane="$(tmux split-window -h -P -F '#{pane_id}' -t "$SESSION:0.0" -c "$TARGET")"
  launch_server "$api_pane" api
  web_pane="$(tmux split-window -v -P -F '#{pane_id}' -t "$api_pane" -c "$TARGET")"
  launch_server "$web_pane" web
  shell_pane="$(tmux split-window -v -P -F '#{pane_id}' -t "$web_pane" -c "$TARGET")"
  tmux set-option -p -t "$shell_pane" @role shell
  normalise_layout
else
  ensure_simple claude 'claude'
  ensure_server api 3000
  ensure_server web 5173
  ensure_simple shell ''
fi

# --- 3. attach (only from outside tmux) -------------------------------------
if [ -z "${TMUX:-}" ]; then
  exec tmux -CC attach -t "$SESSION"
else
  echo "bin/tmux: reconciled '$SESSION' (already inside tmux; not attaching)"
fi
```

- [ ] **Step 2: Verify a fresh boot (scenario 1)**

Make sure no session exists: `tmux kill-session -t xpool 2>/dev/null || true`. Then from the main checkout run `bin/tmux`. (If running inside an existing tmux/claude pane, it will reconcile and print the "not attaching" line instead of attaching — that is correct.)

Expected: `local-stack` output (infra + seed of `xpool-<branch>`), a session `xpool` with 4 panes whose borders read `claude` / `api` / `web` / `shell`, api on `:3000`, web on `:5173`. Verify panes:

```bash
tmux list-panes -s -t xpool -F '#{@role} #{@target} #{@branch}'
```

Expected: four lines; `api` and `web` rows show the main checkout path and current branch.

- [ ] **Step 3: Verify pane restoration (scenario 2) — closing a pane is healed, servers untouched**

Close the shell pane, then re-run:

```bash
tmux kill-pane -t "$(tmux list-panes -s -t xpool -F '#{@role} #{pane_id}' | awk '$1=="shell"{print $2}')"
bin/tmux
tmux list-panes -s -t xpool -F '#{@role}' | sort
```

Expected: `shell` reappears; api/web were **not** restarted (no new `local-stack`-driven rebuild of running servers — confirm `:3000`/`:5173` PIDs are unchanged via `lsof -ti tcp:3000 -sTCP:LISTEN` before/after).

- [ ] **Step 4: Verify crash recovery (scenario 3) — dead api port is restarted**

```bash
kill $(lsof -ti tcp:3000 -sTCP:LISTEN) 2>/dev/null || true
bin/tmux
for i in $(seq 1 60); do curl -fsS localhost:3000/api/health >/dev/null 2>&1 && break; sleep 1; done
curl -fsS localhost:3000/api/health && echo " <- api restarted"
```

Expected: api comes back; web pane untouched.

- [ ] **Step 5: Commit**

```bash
git add bin/tmux
git commit -m "feat(bin): rewrite tmux as idempotent self-healing dev session"
```

---

## Task 5: delete `bin/switch` + update docs

**Files:**
- Delete: `bin/switch`
- Modify: `README.md` (the "One-command session" paragraph)
- Modify: `CLAUDE.md` (the `bin/tmux` / `bin/switch` section under "tmux dev session & worktree switching")
- Modify: `docs/superpowers/specs/2026-05-18-tmux-restarter-design.md` (the `bin/switch` addendum)

- [ ] **Step 1: Delete `bin/switch`**

```bash
git rm bin/switch
```

- [ ] **Step 2: Update `README.md`**

Replace the paragraph that currently reads:

```
**One-command session:** `bin/tmux` does the whole bootstrap above and lays it
out in a tmux session (docker / api / web panes plus a `claude` pane), then
re-attaches on later runs. To point the api + web servers at a git worktree
without restarting docker, use `bin/switch <worktree>` (or `bin/switch` for the
main checkout) — only one worktree's stack runs at a time, since the ports are
fixed. See
[`docs/superpowers/specs/2026-05-18-tmux-restarter-design.md`](./docs/superpowers/specs/2026-05-18-tmux-restarter-design.md).
```

with:

```
**One-command session:** `bin/tmux` is an idempotent, self-healing dev session.
It brings infra + the per-branch DynamoDB table up, lays out a tmux session
(`claude` / `api` / `web` / `shell` panes), and re-running it recreates any pane
you closed and restarts a crashed server — without touching healthy ones. To
point the api + web servers at a git worktree, run `bin/tmux <worktree>` (or
`bin/tmux` for the checkout you're in) — only one checkout's stack runs at a
time, since the ports are fixed. See
[`docs/superpowers/specs/2026-06-06-unified-tmux-dev-session-design.md`](./docs/superpowers/specs/2026-06-06-unified-tmux-dev-session-design.md).
```

- [ ] **Step 3: Update `CLAUDE.md`**

In the section "### tmux dev session & worktree switching", replace the description of `bin/switch <worktree>` and its single-stack/`CARGO_TARGET_DIR`/`@role` paragraphs so the entry point is `bin/tmux <worktree>`. Concretely, change the opening of that subsection from:

```
`bin/switch <worktree>` repoints the running session's `api` + `web` servers at a
git worktree without touching docker/DynamoDB:

```sh
bin/switch scoreboard-design   # → .claude/worktrees/scoreboard-design
bin/switch                     # back to the main checkout (master)
```
```

to:

```
`bin/tmux <worktree>` repoints the running session's `api` + `web` servers at a
git worktree without touching docker/DynamoDB (it's the same idempotent command
that boots the session):

```sh
bin/tmux scoreboard-design   # → .claude/worktrees/scoreboard-design
bin/tmux                     # the checkout you're in (main checkout by default)
```
```

Then, in the remaining prose of that subsection, replace every other occurrence of `bin/switch` with `bin/tmux` (the `CARGO_TARGET_DIR`, `@role`, and `LOCAL_AUTH_ISSUER` notes still apply, just under the unified command). Also update the data note to mention the per-branch table: each branch uses its own `xpool-<branch>` table (seeded on first use), so switching branches no longer risks stale data; `--reseed` forces a rebuild after an in-place schema change.

- [ ] **Step 4: Update the 2026-05-18 spec addendum**

At the top of the "## Addendum — `bin/switch` (worktree switching)" section in `docs/superpowers/specs/2026-05-18-tmux-restarter-design.md`, add a superseded banner directly under the heading:

```
> **Superseded (2026-06-06):** `bin/switch` was folded into `bin/tmux
> [worktree]`. See
> `docs/superpowers/specs/2026-06-06-unified-tmux-dev-session-design.md`. The
> rationale below (single-stack, per-worktree `target/`, `@role` markers, port
> teardown) still holds and carries over to `bin/tmux`.
```

Leave the rest of that addendum intact as historical rationale.

- [ ] **Step 5: Verify no stale references remain**

Run: `grep -rn "bin/switch" --include="*.md" . | grep -v node_modules | grep -v "Superseded" | grep -v ".claude/worktrees"`
Expected: no output (every live reference now points to `bin/tmux`; the only `bin/switch` mentions left are the historical ones inside the superseded addendum).

- [ ] **Step 6: Commit**

```bash
git add -A
git commit -m "docs: delete bin/switch; point docs at unified bin/tmux"
```

---

## Task 6: refactor `web/scripts/e2e-stack.sh` onto the shared core

**Files:**
- Modify: `web/scripts/e2e-stack.sh`

- [ ] **Step 1: Replace `web/scripts/e2e-stack.sh`**

Overwrite it with the version below. It keeps the e2e-specific behaviour (fresh unique table, pinned clock, detached api + PID file, health wait) and delegates docker/wait/seed + the port helper to the shared core:

```bash
#!/usr/bin/env bash
#
# Boots the full xpool stack for the Playwright E2E suite. Delegates infra + seed
# to bin/local-stack (shared with bin/tmux), then starts the API detached with a
# PID file so globalTeardown can stop it. A fresh unique table per run isolates
# runs; bin/local-stack seeds it because a brand-new table is always empty.
#
# Run from anywhere — paths resolve relative to the repo root.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"
source "$REPO_ROOT/bin/lib.sh"

API_PORT=3000
export DYNAMO_ENDPOINT="http://localhost:8000"
export AWS_REGION="${AWS_REGION:-us-east-1}"
export AWS_ACCESS_KEY_ID="${AWS_ACCESS_KEY_ID:-local}"
export AWS_SECRET_ACCESS_KEY="${AWS_SECRET_ACCESS_KEY:-local}"

# A fresh table per run; teardown drops it. (DynamoDB Local is in-memory and the
# container is long-lived, so the table name must be unique.)
export XPOOL_TABLE="xpool-e2e-$(date +%s)"
echo "$XPOOL_TABLE" > "$REPO_ROOT/web/.e2e-table"

# Pin the API clock mid-tournament so the seeded fixture is "live"; tests
# override per-request via X-Dev-Now. Auth: local JWT issuer + HMAC secret.
export XPOOL_NOW="${XPOOL_NOW:-2026-06-20T12:00:00Z}"
export LOCAL_AUTH_ISSUER="${LOCAL_AUTH_ISSUER:-1}"
export INVITE_CODE_SECRET="${INVITE_CODE_SECRET:-test-secret-must-be-32-bytes-long}"

PID_FILE="$REPO_ROOT/web/.e2e-api.pid"
API_LOG="$REPO_ROOT/web/.e2e-api.log"
log() { echo "[e2e-stack] $*"; }
log "using fresh table $XPOOL_TABLE"
log "API clock (XPOOL_NOW) = $XPOOL_NOW"

# ── infra + seed via the shared primitive ────────────────────────────────────
"$REPO_ROOT/bin/local-stack"

# ── stop any stale API on :3000 (Vite is owned by Playwright) ─────────────────
kill_port "$API_PORT"
sleep 1
stale="$(port_pids "$API_PORT")"
# shellcheck disable=SC2086
[ -n "$stale" ] && kill -9 $stale 2>/dev/null || true

# ── build + start the API detached, recording the real PID ───────────────────
log "building the API"
cargo build -q -p api
API_BIN="$(cargo metadata --no-deps --format-version 1 | jq -r .target_directory)/debug/api"
if [ ! -x "$API_BIN" ]; then
  log "ERROR: API binary not found at $API_BIN"
  exit 1
fi
log "starting the API on :$API_PORT ($API_BIN)"
: > "$API_LOG"
"$API_BIN" >>"$API_LOG" 2>&1 &
echo $! > "$PID_FILE"
log "API started (pid $(cat "$PID_FILE"))"

# ── wait for /api/health ─────────────────────────────────────────────────────
log "waiting for GET /api/health"
for i in $(seq 1 60); do
  if curl -fsS "http://localhost:${API_PORT}/api/health" >/dev/null 2>&1; then
    log "API is healthy"
    exit 0
  fi
  if [ "$i" -eq 60 ]; then
    log "ERROR: API did not become healthy in time — last log lines:"
    tail -20 "$API_LOG" || true
    exit 1
  fi
  sleep 1
done
```

- [ ] **Step 2: Verify the refactored bootstrap directly**

```bash
bash web/scripts/e2e-stack.sh
```

Expected: `local-stack` output (docker up, wait, `seeding xpool-e2e-…`), then `building the API`, `API is healthy`, exit 0. Confirm `web/.e2e-table` and `web/.e2e-api.pid` were written. Stop the started api: `kill "$(cat web/.e2e-api.pid)" 2>/dev/null || true`.

- [ ] **Step 3: Verify the full e2e suite still passes (scenario 10)**

Run: `cd web && npm run e2e`
Expected: Playwright boots via `global-setup` → `e2e-stack.sh`, the suite runs green (including `e2e/dev-clock-presets.spec.ts`). This is the authoritative end-to-end check that the refactor didn't regress.

- [ ] **Step 4: Commit**

```bash
git add web/scripts/e2e-stack.sh
git commit -m "refactor(e2e): build e2e-stack on shared bin/local-stack + lib.sh"
```

---

## Task 7: final integration pass — worktree jump + cleanup

**Files:** none (verification + a `.gitignore` check).

- [ ] **Step 1: Verify a worktree jump (scenario 4) — only if you use worktrees**

If a worktree exists under `.claude/worktrees/<name>` (create one with `git worktree add .claude/worktrees/smoke -b smoke` if you want to test), run:

```bash
bin/tmux smoke
tmux list-panes -s -t xpool -F '#{@role} #{@target} #{@branch}'
lsof -ti tcp:3000 -sTCP:LISTEN >/dev/null && echo "api running"
```

Expected: `api`/`web` `@target` now point at `.claude/worktrees/smoke`, `@branch` is `smoke`, and the api is serving against table `xpool-smoke` (seeded by local-stack). Docker/DynamoDB container untouched. Then jump back: `bin/tmux` (from the main checkout) repoints to the main checkout's branch/table.

Clean up if you created the throwaway worktree: `git worktree remove .claude/worktrees/smoke --force; git branch -D smoke`.

- [ ] **Step 2: Verify the temp/smoke tables aren't lingering as a problem**

Run (lists tables in DynamoDB Local):

```bash
AWS_REGION=us-east-1 AWS_ACCESS_KEY_ID=local AWS_SECRET_ACCESS_KEY=local \
  aws dynamodb list-tables --endpoint-url http://localhost:8000 --output text
```

Expected: you see `xpool-<branch>` tables (and any `xpool-e2e-*` from e2e runs). These are in-memory only and vanish on `docker compose down`/restart — no action needed, but drop the throwaway ones if you like with `XPOOL_TABLE=<name> cargo run -q -p xtask -- drop-table`.

- [ ] **Step 3: Confirm `bin/lib.test.sh` still passes (regression guard)**

Run: `bash bin/lib.test.sh`
Expected: `all passed`, exit 0.

- [ ] **Step 4: Final review of the bin/ surface**

Run: `ls -la bin/ && grep -L 'set -euo pipefail' bin/lib.sh bin/local-stack bin/run-api bin/run-web bin/tmux 2>/dev/null`

Expected: `bin/switch` is gone; `lib.sh`, `local-stack`, `run-api`, `run-web`, `tmux`, `lib.test.sh` present and executable. The `grep -L` lists files **without** the strict-mode line — `bin/lib.sh` is expected here (it's sourced, not executed standalone, so it intentionally omits `set -e`); the rest should not appear. If any executable script (local-stack/run-api/run-web/tmux) is listed, add `set -euo pipefail`.

- [ ] **Step 5: Commit any cleanup**

```bash
git add -A
git commit -m "chore(bin): final integration pass for unified dev session" --allow-empty
```

---

## Self-review notes (for the executor)

- **Spec coverage:** decisions 1–12 in the spec map to tasks here — native app/infra-only (no containerisation work needed; verified by Tasks 2/3 running cargo/npm natively), one script + delete switch (Tasks 4–5), cwd-based target (Task 4 resolve block), per-pane reconcile incl. dead-port restart (Task 4 `ensure_server`), conditional install + unshared `CARGO_TARGET_DIR` (Task 3 `run-web` + each checkout's own `cargo run`), self-healing seed (Task 2), shell pane (Task 4), run-api/run-web wrappers (Task 3), shared local-stack + lib.sh + e2e fold-in (Tasks 1/2/6), per-branch tables (Task 1 `table_for`, used in Tasks 2/3/4), minimal env (no `.env` sourcing anywhere — confirm by `grep -rn 'source .*\.env' bin/` returning nothing), deferred profiles (no work).
- **Order rationale:** helpers (1) → primitive (2) → launchers (3) → orchestrator (4) → cleanup/docs (5) → e2e (6) → integration (7). Each task is independently committable and leaves the repo working.
- **If `nc` is unavailable** on the host, `wait_for_port` fails; substitute `curl -s -o /dev/null "http://localhost:$port"` (DynamoDB Local answers any HTTP request) — but the current `bin/tmux` already uses `nc`, so it's present.
