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
