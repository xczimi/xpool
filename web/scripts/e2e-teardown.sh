#!/usr/bin/env bash
#
# Stops the API process started by e2e-stack.sh, then drops the ephemeral
# DynamoDB table for this run. Docker is left running on purpose — DynamoDB
# Local is in-memory and cheap, and reusing it speeds up repeated runs.
# Run `docker compose down` manually to stop it.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"

# THIS run's dynamic API port (set by global-teardown; falls back to :3001 for a
# bare `playwright test`). Per-run state files are namespaced by it, so teardown
# only ever stops THIS run's API and drops THIS run's table — never a concurrent
# run's. DynamoDB (:8001) is shared and isolated by the unique table name; dev
# (:3000 / :8000) is NEVER touched.
API_PORT="${XPOOL_E2E_API_PORT:-3001}"
DYNAMO_PORT=8001
PID_FILE="$REPO_ROOT/web/.e2e-api.${API_PORT}.pid"
export DYNAMO_ENDPOINT="http://localhost:${DYNAMO_PORT}"
export AWS_REGION="${AWS_REGION:-us-east-1}"
export AWS_ACCESS_KEY_ID="${AWS_ACCESS_KEY_ID:-local}"
export AWS_SECRET_ACCESS_KEY="${AWS_SECRET_ACCESS_KEY:-local}"

log() { echo "[e2e-teardown] $*"; }

if [ -f "$PID_FILE" ]; then
  PID="$(cat "$PID_FILE")"
  if [ -n "$PID" ] && kill -0 "$PID" 2>/dev/null; then
    log "stopping API (pid $PID)"
    kill "$PID" 2>/dev/null || true
    sleep 1
    kill -9 "$PID" 2>/dev/null || true
  fi
  rm -f "$PID_FILE"
fi

# Belt and braces: anything still bound to THIS run's e2e API port only.
PIDS="$(lsof -ti tcp:${API_PORT} 2>/dev/null || true)"
if [ -n "$PIDS" ]; then
  log "killing leftover process(es) on :${API_PORT} — $PIDS"
  # shellcheck disable=SC2086
  kill -9 $PIDS 2>/dev/null || true
fi

TABLE_FILE="$REPO_ROOT/web/.e2e-table.${API_PORT}"
if [ -f "$TABLE_FILE" ]; then
  XPOOL_TABLE="$(cat "$TABLE_FILE")"
  export XPOOL_TABLE
  log "dropping table $XPOOL_TABLE"
  (cd "$REPO_ROOT" && cargo run -q -p xtask -- drop-table) || true
  rm -f "$TABLE_FILE"
fi

# Drop this run's API log too, so namespaced files don't accumulate.
rm -f "$REPO_ROOT/web/.e2e-api.${API_PORT}.log"

log "done (docker left running — 'docker compose down' to stop it)"
