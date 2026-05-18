# 05 — invite has no authorization or self-target guard

Status: done
Severity: HIGH
Area: crates/api

## Problem

`invite` (`crates/api/src/gql/mutation.rs:313-323`) lets any authenticated
player set *any* existing player's `referrer` to themselves — repeatedly,
overwriting a prior referrer, and including targeting themselves.

## Expected

- Reject self-invite.
- Reject re-assigning a `referrer` once one is set (an invitee can only be
  referred once).
- Confirm whether any further authorization is needed (e.g. invitee must be a
  newly-created / un-onboarded player).

## Acceptance

- API tests: self-invite rejected; second invite of an already-referred player
  rejected.
- `cargo test -p api` green.

## Comments

Fixed in `crates/api/src/gql/mutation.rs`: `invite` now rejects self-targeting and
rejects re-referring a player whose `referrer` is already set (immutable update,
no mutation). Tests added to `graphql.rs`.
