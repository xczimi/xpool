# 27 — deadline boundary comparator is inconsistent (`>=` vs `>`)

Status: ready-for-agent
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
