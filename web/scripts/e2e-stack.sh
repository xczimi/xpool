#!/usr/bin/env bash
#
# Boots an ISOLATED xpool stack for the Playwright E2E suite. The API port is
# DYNAMIC (XPOOL_E2E_API_PORT, decided per run by e2e/run-e2e.mjs; falls back to
# :3001) and the Vite web port is dynamic too (XPOOL_E2E_WEB_PORT, fallback
# :5174). DynamoDB stays on the shared isolated :8001 container. This never
# collides with — or tears down — a running dev session (API :3000, DynamoDB
# :8000, Vite :5173). Per-run state files (PID/log/table) are namespaced by the
# API port so multiple e2e runs can run concurrently without clobbering each
# other. Delegates infra + seed to bin/local-stack (shared with bin/local-dev),
# then starts the API detached with a PID file so globalTeardown can stop it.
# A fresh unique table per run isolates runs; bin/local-stack seeds it because a
# brand-new table is always empty.
#
# Run from anywhere — paths resolve relative to the repo root.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
REPO_ROOT="$(cd "$SCRIPT_DIR/../.." && pwd)"
cd "$REPO_ROOT"
source "$REPO_ROOT/bin/lib.sh"

# Per-run dynamic e2e ports — distinct from the dev stack so the two coexist.
API_PORT="${XPOOL_E2E_API_PORT:-3001}"
WEB_PORT="${XPOOL_E2E_WEB_PORT:-5174}"
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
export XPOOL_PUBLIC_ORIGIN="${XPOOL_PUBLIC_ORIGIN:-http://localhost:${WEB_PORT}}"

# A fresh table per run; teardown drops it. (DynamoDB Local is in-memory and the
# container is long-lived, so the table name must be unique.) The pid ($$) +
# $RANDOM suffix keeps it unique even for runs that start in the same second.
export XPOOL_TABLE="xpool-e2e-$(date +%s)-$$-${RANDOM}"

# Per-run state files, namespaced by the (dynamic) API port so concurrent runs
# never clobber each other's pid/log/table bookkeeping.
TABLE_FILE="$REPO_ROOT/web/.e2e-table.${API_PORT}"
echo "$XPOOL_TABLE" > "$TABLE_FILE"

# Pin the API clock mid-tournament so the seeded fixture is "live"; tests
# override per-request via X-Dev-Now. Auth: local JWT issuer + HMAC secret.
export XPOOL_NOW="${XPOOL_NOW:-2026-06-20T12:00:00Z}"
export LOCAL_AUTH_ISSUER="${LOCAL_AUTH_ISSUER:-1}"
export INVITE_CODE_SECRET="${INVITE_CODE_SECRET:-test-secret-must-be-32-bytes-long}"

# Keep e2e hermetic: force the SportsDB source to NullSource (no live calls).
export THESPORTSDB_API_KEY=""

# Deterministic live score for the live-scoring e2e (StubLiveSource). The stub is
# keyed by external id and maps onto M8's REAL SportsDB id (2461105) — we do NOT
# bake a test sentinel into the shipped tournament fixture (that would cost M8 its
# real live-score lookup in production). This makes the scoreboard "Max" column and
# the match-page live overlay appear when the dev clock is inside M8's live window.
# The stub wins over THESPORTSDB only when this var is set, and only maps this one
# id → M8 — all other games get nothing from it, so other specs that assert "no
# live score" under their groups stay green.
export XPOOL_LIVE_SCORES="${XPOOL_LIVE_SCORES:-2461105=1:0:2H}"

PID_FILE="$REPO_ROOT/web/.e2e-api.${API_PORT}.pid"
API_LOG="$REPO_ROOT/web/.e2e-api.${API_PORT}.log"
log() { echo "[e2e-stack] $*"; }
log "using fresh table $XPOOL_TABLE"
log "API clock (XPOOL_NOW) = $XPOOL_NOW"

# ── infra + seed via the shared primitive ────────────────────────────────────
"$REPO_ROOT/bin/local-stack"

# ── stop any stale process on THIS run's dynamic API port only — never touch
#    dev's :3000. Vite is owned by Playwright. ─────────────────────────────────
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
