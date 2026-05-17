#!/usr/bin/env bash
#
# Stops the API process started by e2e-stack.sh. Docker is left running on
# purpose — DynamoDB Local is in-memory and cheap, and reusing it speeds up
# repeated runs. Run `docker compose down` manually to stop it.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
PID_FILE="$SCRIPT_DIR/../.e2e-api.pid"

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

log "done (docker left running — 'docker compose down' to stop it)"
