#!/usr/bin/env bash
# Shared PURE helpers for the xpool dev-session scripts. Sourcing this file has
# NO side effects (no env mutation, no docker/tmux/network calls) — it only
# defines functions. Sourced by bin/local-stack, bin/run-api, bin/run-web,
# bin/local-dev, and web/scripts/e2e-stack.sh.

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

# Newest *.json snapshot under a directory (by mtime), or empty if none.
# Pure: reads the filesystem only; no network, no mutation.
latest_snapshot() {  # <snapshots-dir>
  local dir="$1"
  [ -d "$dir" ] || return 0
  # Snapshot names are controlled (alphanumeric, *.json from bin/pull-data) and
  # `ls -t` sorts by mtime — the portable choice (BSD/macOS `find` lacks -printf).
  # shellcheck disable=SC2012
  ls -t "$dir"/*.json 2>/dev/null | head -n1
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
    nc -zw1 localhost "$port" 2>/dev/null && return 0
    sleep 0.5
  done
  echo "lib.sh: timed out waiting for :$port" >&2
  return 1
}
