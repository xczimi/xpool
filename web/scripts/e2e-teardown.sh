#!/usr/bin/env bash
#
# Stops the API process started by e2e-stack.sh, then drops the ephemeral
# DynamoDB table for this run. Docker is left running on purpose — DynamoDB
# Local is in-memory and cheap, and reusing it speeds up repeated runs.
# Run `docker compose down` manually to stop it.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
PID_FILE="$REPO_ROOT/web/.e2e-api.pid"

DYNAMO_PORT=8000
export DYNAMO_ENDPOINT="http://localhost:${DYNAMO_PORT}"
export AWS_REGION="${AWS_REGION:-us-east-1}"
export AWS_ACCESS_KEY_ID="${AWS_ACCESS_KEY_ID:-local}"
export AWS_SECRET_ACCESS_KEY="${AWS_SECRET_ACCESS_KEY:-local}"

# cargo is not on PATH — mise provides the Rust toolchain.
cargo() { mise exec -- cargo "$@"; }

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

# Belt and braces: anything still bound to :3000.
PIDS="$(lsof -ti tcp:3000 2>/dev/null || true)"
if [ -n "$PIDS" ]; then
  log "killing leftover process(es) on :3000 — $PIDS"
  # shellcheck disable=SC2086
  kill -9 $PIDS 2>/dev/null || true
fi

TABLE_FILE="$REPO_ROOT/web/.e2e-table"
if [ -f "$TABLE_FILE" ]; then
  XPOOL_TABLE="$(cat "$TABLE_FILE")"
  export XPOOL_TABLE
  log "dropping table $XPOOL_TABLE"
  (cd "$REPO_ROOT" && cargo run -q -p xtask -- drop-table) || true
  rm -f "$TABLE_FILE"
fi

log "done (docker left running — 'docker compose down' to stop it)"
