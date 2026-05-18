# 24 — enterResult accepts `advancer` but never persists it

Status: done
Severity: HIGH
Area: crates/api

## Problem

`enter_result` (`crates/api/src/gql/mutation.rs:423-475`) takes an
`advancer: Option<String>` argument — its doc comment says it "is the team id
that progresses on a knockout draw" — but the parameter is
`#[allow(unused_variables)]` and never used.

Issue 10's `fwc26` fix (`determine_winner_loser`) resolves a drawn-knockout
advancer from the result user's `StandingsPrediction.ordering[0]` for the
one-match knockout group. But `enter_result` never writes that standings
prediction. So a drawn knockout result entered through the API has no
standings prediction → `determine_winner_loser` falls back to the home team.

**The issue-10 fix is correct in `fwc26` unit tests but unreachable via the
real write path** — the fwc26 tests hand-build the standings prediction.

## Expected

When `enter_result` is called with `advancer: Some(team_id)`, persist a
`StandingsPrediction` on the result user for the game's one-match knockout
group: `ordering = [advancer, other_team]`. `enter_result` will need to load
the tournament to find the game's `group_id` and the two team ids.

Reject an `advancer` that is not one of the match's two teams.

## Acceptance

- API test: a drawn knockout result entered with `advancer` set resolves the
  bracket to that team (not the home team) after recompute.
- `cargo test -p api` green.

## Comments

`enter_result` now loads the tournament when `advancer` is `Some`, finds the
game's `group_id` and its two resolved team ids, rejects an advancer not in the
match, and persists a `StandingsPrediction` (`ordering = [advancer, other]`,
`draw_order = []`, `locked` mirroring the result) on the result user. The
`#[allow(unused_variables)]` is gone. New API tests
(`enter_result_advancer_resolves_a_drawn_knockout_to_that_team`,
`enter_result_rejects_an_advancer_not_in_the_match`) use a new
`seeded_repo_with_knockout` fixture (one-match R32 + R16 groups) and confirm a
drawn knockout advances the chosen team downstream. 64 api tests green.
