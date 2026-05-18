# 18 — recompute failure after enter_result leaves inconsistent state

Status: ready-for-agent
Severity: MEDIUM
Area: crates/api

## Problem

`enter_result` (`crates/api/src/gql/mutation.rs:355-360`) writes the result-user
player, then runs `recompute`. If `recompute` returns `Err`, the new result is
already persisted but the scoreboard / bracket are stale, and the caller gets
an error suggesting nothing happened. No transaction or compensation.

`recompute` errors (`"no result user found"`, `"no tournament loaded"`) are
also bubbled raw to the client (`crates/api/src/recompute.rs`).

## Expected — decision needed

DynamoDB single-table writes can't span a transaction trivially here. Options:
make `recompute` infallible by construction, recompute *before* committing the
result, or surface a clear "result saved, scoreboard refresh pending" state.
Human decision on the desired consistency model — hence `ready-for-human`.

## Acceptance

- Decision recorded; `enter_result` no longer leaves a silently-stale
  scoreboard, and raw internal errors are not leaked to the client.

## Decision (grilled 2026-05-17)

**Idempotent retry + manual trigger.** `recompute` is a wholesale, idempotent
rebuild — any re-run fully repairs partial state.

- `enter_result` persists the (locked) result, then runs `recompute`. On
  `recompute` error it returns success with a `recomputePending: true` flag
  instead of erroring — the result IS saved.
- Change `enter_result`'s return type from `bool` to a struct carrying
  `recomputePending` (e.g. `ResultEntered { recompute_pending }`).
  **Ripple:** GraphQL schema + the `AdminResults` SPA page must surface the
  pending state.
- Add an admin **`recompute` mutation** (`require_admin`) that re-runs the
  hook on demand / self-heals. Any later `enter_result` also rebuilds wholesale.
- Stop leaking raw internal error strings (`"no result user found"` etc.) to
  the client.

A briefly-stale scoreboard after a failed recompute is accepted — same risk
posture as the issue-02 `unlockResult` decision.
