# 11 — scoring.rs reintroduces the clock seam (unwrap_or_else(Utc::now))

Status: done
Severity: HIGH
Area: crates/domain

## Problem

`crates/domain/src/scoring.rs:618` does
`t.deadline(group_id).unwrap_or_else(Utc::now)`. When a leaf group has no
resolvable games (empty `Games` list, or all game ids missing from `t.games`),
the deadline silently falls back to wall-clock `Utc::now()`.

The project mandates a server-authoritative, injectable clock (CLAUDE.md
"Server-authoritative clock", `TESTING.md` §3.1 "domain has no clock"). A pure
scoring function calling `Utc::now()` is non-deterministic and breaks the
`XPOOL_NOW` / `X-Dev-Now` model. No test covers the empty-leaf-group path.

## Expected

Do not call `Utc::now()` in `domain`. Either propagate `now` into the scoring
call, or treat a missing deadline explicitly (e.g. "not past" / far future)
without consulting the wall clock.

## Acceptance

- No `Utc::now()` (or equivalent) call remains in `crates/domain`.
- Test covering an empty / unresolvable leaf group.
- `cargo test -p domain` green.

## Comments

Replaced `t.deadline(group_id).unwrap_or_else(Utc::now)` in `score_group_node`
with `unwrap_or(DateTime::<Utc>::MAX_UTC)` — an unresolvable leaf-group deadline
is now treated as "far in the future", so `now > deadline` stays false and an
unscored leaf group is never silently auto-locked. No public signature changed.
Also pinned the `effective_locked_truth_table` unit test to fixed timestamps so
`crates/domain/src` is fully wall-clock-free. Added integration test
`score_tournament_unresolvable_leaf_group_never_auto_locks` covering a leaf group
whose only game id is missing from `t.games`. Note: this path is output-neutral
(empty games → empty rankings → 0 bonus), so the seam was non-deterministic but
harmless; the fix restores purity.
