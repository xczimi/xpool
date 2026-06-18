#!/usr/bin/env bash
#
# Boots an ISOLATED xpool stack for the Playwright E2E suite, on its OWN ports
# (API :3001, DynamoDB :8001) so it never collides with — or tears down — a
# running dev session (API :3000, DynamoDB :8000, Vite :5173). Vite (:5174) is
# owned by Playwright's webServer. Delegates infra + seed to bin/local-stack
# (shared with bin/local-dev), then starts the API detached with a PID file so
# globalTeardown can stop it. A fresh unique table per run isolates runs;
# bin/local-stack seeds it because a brand-new table is always empty.
#
# Run from anywhere — paths resolve relative to the repo root.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"
source "$REPO_ROOT/bin/lib.sh"

# Dedicated e2e ports — distinct from the dev stack so the two coexist.
API_PORT=3001
export XPOOL_PORT="$API_PORT"
export DYNAMO_ENDPOINT="http://localhost:8001"
export AWS_REGION="${AWS_REGION:-us-east-1}"
export AWS_ACCESS_KEY_ID="${AWS_ACCESS_KEY_ID:-local}"
export AWS_SECRET_ACCESS_KEY="${AWS_SECRET_ACCESS_KEY:-local}"

# Bring up the isolated e2e DynamoDB (:8001) — its own container, behind the
# `e2e` compose profile — plus the shared mailhog. bin/local-stack reads these.
export XPOOL_COMPOSE_PROFILE=e2e
export XPOOL_COMPOSE_SERVICES="dynamodb-e2e mailhog"

# Invite links the API builds must point at the e2e web port, not dev's :5173.
export XPOOL_PUBLIC_ORIGIN="${XPOOL_PUBLIC_ORIGIN:-http://localhost:5174}"

# A fresh table per run; teardown drops it. (DynamoDB Local is in-memory and the
# container is long-lived, so the table name must be unique.)
export XPOOL_TABLE="xpool-e2e-$(date +%s)"
echo "$XPOOL_TABLE" > "$REPO_ROOT/web/.e2e-table"

# Pin the API clock mid-tournament so the seeded fixture is "live"; tests
# override per-request via X-Dev-Now. Auth: local JWT issuer + HMAC secret.
export XPOOL_NOW="${XPOOL_NOW:-2026-06-20T12:00:00Z}"
export LOCAL_AUTH_ISSUER="${LOCAL_AUTH_ISSUER:-1}"
export INVITE_CODE_SECRET="${INVITE_CODE_SECRET:-test-secret-must-be-32-bytes-long}"

# Keep e2e hermetic: force the SportsDB source to NullSource (no live calls).
export THESPORTSDB_API_KEY=""

PID_FILE="$REPO_ROOT/web/.e2e-api.pid"
API_LOG="$REPO_ROOT/web/.e2e-api.log"
log() { echo "[e2e-stack] $*"; }
log "using fresh table $XPOOL_TABLE"
log "API clock (XPOOL_NOW) = $XPOOL_NOW"

# ── infra + seed via the shared primitive ────────────────────────────────────
"$REPO_ROOT/bin/local-stack"

# ── stop any stale *e2e* API on :3001 only — never touch dev's :3000. Vite is
#    owned by Playwright. ───────────────────────────────────────────────────────
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
