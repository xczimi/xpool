# 27 — deadline boundary comparator is inconsistent (`>=` vs `>`)

Status: done
Severity: MEDIUM
Area: crates/api / crates/domain

## Problem

`submit_group` rejects edits at `now >= deadline`
(`crates/api/src/gql/mutation.rs:176`), but `effective_locked`
(`crates/domain/src/scoring.rs:91`) and `deadline_passed`
(`crates/api/src/timeflags.rs:21`) use strict `now > deadline`.

At the exact deadline instant the write path rejects an edit while scoring and
the SPA's `deadlinePassed` flag report the deadline as *not yet* passed — an
inconsistent boundary.

## Expected

Pick one comparator and use it everywhere. `> deadline` (deadline instant is
still open) or `>= deadline` (deadline instant is closed) — decide and align
`submit_group`, `effective_locked`, and `deadline_passed`.

## Acceptance

- All three sites use the same comparator.
- A test pins the boundary behaviour at exactly `now == deadline`.
- `cargo test --workspace` green.

## Comments

Aligned `submit_group` to strict `>` (the deadline instant is still open),
matching `effective_locked` (crates/domain/src/scoring.rs, already `>`) and
`deadline_passed` (crates/api/src/timeflags.rs, already `>`) — no domain edit
needed. New API test `submit_group_allowed_at_exactly_the_deadline_instant`
pins the boundary: a submit at `now == deadline` is allowed, one nanosecond
past is rejected. `cargo test -p api`/`-p domain` green.
