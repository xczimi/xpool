# 17 — updateProfile does not validate nick / full_name

Status: done
Severity: MEDIUM
Area: crates/api

## Problem

`updateProfile` (`crates/api/src/gql/mutation.rs:287-309`) accepts empty,
whitespace-only, or arbitrarily long `nick` / `full_name`. `nick` is shown
across the app (scoreboard, tips) — there is no length / non-empty (and
possibly uniqueness) validation.

## Expected

Validate `nick` and `full_name`: non-empty after trim, reasonable max length.
Decide whether `nick` must be unique.

## Acceptance

- API test: empty / whitespace / oversized values are rejected.
- `cargo test -p api` green.

## Comments

Fixed in `crates/api/src/gql/mutation.rs`: added `validate_profile_field`;
`update_profile` now rejects empty/whitespace-only and oversized `nick`
(max 40) / `full_name` (max 120) and stores the trimmed value. Tests added to
`graphql.rs`.
