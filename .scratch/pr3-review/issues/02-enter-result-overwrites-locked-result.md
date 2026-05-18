# 02 — enter_result can silently overwrite a locked official result

Status: done
Severity: CRITICAL
Area: crates/api

## Problem

`enter_result` (`crates/api/src/gql/mutation.rs:327-362`) uses `retain` + `push`
to blindly replace the official result for a game, then recomputes. There is no
guard distinguishing first entry from rewriting an already-locked result.
`SCENARIOS.md` (~line 707) cites the legacy rule that locked results are not
silently rewound.

## Expected

Overwriting a locked official result should be a deliberate, explicit path
(e.g. a separate mutation or an explicit `force` flag), not a side effect of
the same mutation used for first entry.

Needs a human decision on the intended admin workflow before implementation —
hence `ready-for-human`.

## Acceptance

- Decision recorded (in `.specs/` or this issue).
- `enter_result` rejects (or explicitly requires opt-in for) overwriting a
  locked result; covered by an API test.

## Decision (grilled 2026-05-17)

Add a dedicated **`unlockResult(game_id)` admin mutation** that flips the
official result's `locked` flag to `false` — a **bare state flip, no
recompute**. The materialised scoreboard is briefly stale between unlock and
re-entry; this is accepted (admin-only, momentary — unlock is followed by
`enter_result` + re-lock in practice).

`enter_result` **rejects** any game whose official result is already `locked`,
with a clear error pointing at `unlockResult`. Correcting a locked result is
therefore a deliberate two-call sequence: `unlockResult` → `enter_result`.
This matches the legacy `editable` rule (`not locked`, ADMIN-06).

Implementation: `unlockResult` is `require_admin`, loads the result user,
clears `locked` on the matching `MatchPrediction`, `put_player`. `enter_result`
checks the existing prediction's `locked` before the retain/push.

## Comments

Implemented in `crates/api/src/gql/mutation.rs`: new `unlockResult(gameId: ID!): Boolean!`
admin mutation (bare `locked = false` flip on the result user, no recompute), and
`enterResult` now rejects any game whose existing official result is `locked` with an
error pointing at `unlockResult`. Unlocked results stay freely correctable. Covered by
`enter_result_rejects_a_locked_result`, `enter_result_allows_correcting_an_unlocked_result`,
`unlock_result_flips_the_locked_flag`, and `unlock_result_requires_admin` in
`crates/api/tests/graphql.rs`.

## Comments — web ripple

`AdminResults.tsx` now shows an **Unlock** button per locked game (calls the new
`unlockResult` mutation, then refetches) so a locked result becomes editable
again — the deliberate `unlockResult` → `enterResult` two-call sequence. Added
`UNLOCK_RESULT_MUTATION` to `queries.ts` and the `unlockResult` / `resultUnlocked`
i18n strings (en + hu).
