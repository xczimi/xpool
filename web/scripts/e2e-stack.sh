#!/usr/bin/env bash
#
# Boots the full xpool stack for the Playwright E2E suite, end to end:
#
#   1. kill any stale API (:3000) process (Vite is owned by Playwright)
#   2. docker compose up -d  (DynamoDB Local + MailHog)
#   3. wait for DynamoDB Local to accept connections
#   4. xtask import tournaments/fwc26.json  +  xtask seed
#   5. build and start the API (cargo run -p api) in the background
#   6. wait for GET /api/health
#
# The API is started detached; its PID is written to web/.e2e-api.pid so the
# Playwright globalTeardown can stop it. Docker is left running (cheap, and
# DynamoDB Local is in-memory anyway).
#
# Run from anywhere — paths are resolved relative to the repo root.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"

API_PORT=3000
DYNAMO_PORT=8000
export DYNAMO_ENDPOINT="http://localhost:${DYNAMO_PORT}"
# DynamoRepository::from_env needs a region + credentials; DynamoDB Local
# ignores the values but the AWS SDK still requires them to be present.
export AWS_REGION="${AWS_REGION:-us-east-1}"
export AWS_ACCESS_KEY_ID="${AWS_ACCESS_KEY_ID:-local}"
export AWS_SECRET_ACCESS_KEY="${AWS_SECRET_ACCESS_KEY:-local}"

# A fresh table per run — isolates this run from every previous run.
# DynamoDB Local is in-memory and the container is long-lived, so the table
# name must be unique; teardown drops it.
export XPOOL_TABLE="xpool-e2e-$(date +%s)"
TABLE_FILE="$REPO_ROOT/web/.e2e-table"
echo "$XPOOL_TABLE" > "$TABLE_FILE"

# Default the API clock to mid-tournament so the seeded fixture is "live".
# Individual tests override per-request via the dev clock (X-Dev-Now).
export XPOOL_NOW="${XPOOL_NOW:-2026-06-20T12:00:00Z}"

PID_FILE="$REPO_ROOT/web/.e2e-api.pid"
API_LOG="$REPO_ROOT/web/.e2e-api.log"

log() { echo "[e2e-stack] $*"; }
log "using fresh table $XPOOL_TABLE"
log "API clock (XPOOL_NOW) = $XPOOL_NOW"

# ── 1. kill stale processes ──────────────────────────────────────────────────
# Only the API (:3000) is killed here. The Vite dev server (:5173) is owned by
# Playwright's `webServer` (with `reuseExistingServer`), which Playwright may
# already have started concurrently with this globalSetup — killing :5173 here
# would kill Playwright's own server.
kill_port() {
  local port="$1"
  local pids
  pids="$(lsof -ti tcp:"$port" 2>/dev/null || true)"
  if [ -n "$pids" ]; then
    log "killing stale process(es) on :$port — $pids"
    # shellcheck disable=SC2086
    kill $pids 2>/dev/null || true
    sleep 1
    # shellcheck disable=SC2086
    kill -9 $pids 2>/dev/null || true
  fi
}
kill_port "$API_PORT"

# ── 2. docker compose up ─────────────────────────────────────────────────────
log "starting docker compose (DynamoDB Local + MailHog)"
docker compose up -d

# ── 3. wait for DynamoDB Local ───────────────────────────────────────────────
# DynamoDB Local answers a bare GET with HTTP 400 — any HTTP response at all
# means the port is accepting connections, so check connectivity, not status.
log "waiting for DynamoDB Local on :$DYNAMO_PORT"
for i in $(seq 1 60); do
  if curl -s -o /dev/null "http://localhost:${DYNAMO_PORT}"; then
    log "DynamoDB Local is up"
    break
  fi
  if [ "$i" -eq 60 ]; then
    log "ERROR: DynamoDB Local did not come up in time"
    exit 1
  fi
  sleep 1
done

# ── 4. import + seed ─────────────────────────────────────────────────────────
log "importing tournament + seeding demo data"
cargo run -q -p xtask -- import tournaments/fwc26.json
cargo run -q -p xtask -- seed

# ── 5. build + start the API ─────────────────────────────────────────────────
# Build first, then run the produced binary directly — that way the recorded
# PID is the actual server process, not a `cargo run` wrapper, so teardown can
# stop it cleanly.
log "building the API"
cargo build -q -p api
API_BIN="$(cargo metadata --no-deps --format-version 1 \
  | jq -r .target_directory)/debug/api"
if [ ! -x "$API_BIN" ]; then
  log "ERROR: API binary not found at $API_BIN"
  exit 1
fi
log "starting the API on :$API_PORT ($API_BIN)"
: > "$API_LOG"
"$API_BIN" >>"$API_LOG" 2>&1 &
echo $! > "$PID_FILE"
log "API started (pid $(cat "$PID_FILE"), pid file: $PID_FILE)"

# ── 6. wait for /api/health ──────────────────────────────────────────────────
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
