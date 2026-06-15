# Exclude non-participating players from listings

- **Date:** 2026-06-14
- **Status:** Approved (design) — pending implementation plan
- **Scope:** `crates/domain`, `crates/api` (three GraphQL resolvers). No frontend code change.

## Problem

The All Tips grid (and, to a lesser degree, the Scoreboard and Standings-bonus
grid) shows rows for players who never entered the relevant predictions. Pulling
production data into local made this obvious: of 38 competitors, several saved no
predictions at all (`Twoeazy21`, `Balint`, `Czimi`, `Jess`, `Randy`), and others
saved some but skipped whole groups (e.g. `VanPete` predicted 66/72 games but
none of Group A). Their cells render as the misleading word **"hidden"**, and
their all-zero rows clutter the rankings.

We want these non-participating rows excluded from the listings — without
coupling the API to the UI, and without hiding genuine participants whose tips
are merely time-gated by mutual commitment (pre-kickoff).

## Decision

**Filter on a domain predicate — "did this player enter the relevant
predictions" — computed by pure functions in `domain`, applied by the thin
resolvers.** This is the same category of rule the API already applies to the
result-user (`Player listings must exclude it`), just generalised and lifted out
of inline ad-hoc checks into the pure-logic layer.

Two complementary criteria (both requested):

- **Global (Scoreboard):** drop *non-participants* — players who are not the
  result-user and have **no** match predictions **and no** standings
  predictions across the tournament. A participant who tipped but scored 0 is
  **kept**.
- **Per-group (All Tips, Standings grid):** drop players with **nothing in the
  viewed group** — no match prediction among that group's games (All Tips), or
  no standings prediction for that group's leaves (Standings grid). A player may
  therefore appear on some groups' grids and not others.

### Why not the alternatives

- **Frontend filtering** — duplicates the rule per page, ships data the client
  discards, and would have to proxy on "all cells blank", which wrongly drops a
  participant whose tips are legitimately hidden by mutual commitment before
  kickoff. Rejected.
- **An `inactive` flag on Player** — heavier; needs maintaining a stored flag
  for a property that is fully derivable. YAGNI. Rejected.
- **Filtering in `storage` / at scoreboard materialisation** — `storage` is pure
  persistence and must not embed competition semantics; per-group filtering also
  needs tournament data the repo doesn't hold. Materialisation should stay
  complete (compute everyone) and filtering should happen at read. Rejected.

## Architecture & layering

This repo has no separate stateful service layer; the **pure `domain`/`fwc26`
functions are the logic layer** that the resolvers delegate to (as `scoring.rs`,
Annexe-C, and bracket resolution already do). The rule belongs there — not
inline in the resolvers, and not as a helper in the `api`/`gql` crate.

```
domain  ── pure entity model + scoring + participation (NEW)   ← the rule lives here
  ▲
storage ── Repository (persistence only)
  ▲
api/gql ── resolvers: load roster, call pure selectors, assemble   ← applies the rule
```

The selectors take a roster and (where relevant) the group's ids, and answer a
**domain** question. They know nothing about pages — so the same call returns
the same answer for any client, which is the test that this is domain logic, not
view-coupling.

## Domain layer (new)

New module `crates/domain/src/participation.rs`, pure and unit-tested:

```rust
impl Player {
    /// A competing player who has entered at least one prediction.
    /// False for the result-user and for players with no predictions at all.
    pub fn is_participant(&self) -> bool;
}

/// Competitors for global listings (Scoreboard): participants only.
pub fn participants(players: &[Player]) -> Vec<&Player>;

/// Players with at least one match prediction among `game_ids` (All Tips).
/// Excludes the result-user.
pub fn tippers_in(players: &[Player], game_ids: &[GameId]) -> Vec<&Player>;

/// Players with at least one standings prediction among `group_ids`
/// (Standings-bonus grid). Excludes the result-user.
pub fn standings_tippers(players: &[Player], group_ids: &[GroupId]) -> Vec<&Player>;
```

`is_participant` = `!self.is_result_user && (!self.match_predictions.is_empty()
|| !self.standings_predictions.is_empty())`. The two `*_in` selectors also
exclude the result-user (the grids never list it) and reuse the existing
`Player::match_prediction` / `Player::standings_prediction` accessors.

## Resolver changes (`crates/api/src/gql/query.rs`)

Each resolver replaces its inline `is_result_user` skip with a call to the
matching pure selector; behaviour is otherwise unchanged.

- **`scoreboard`** — build the participant id-set from `participants(&players)`
  and keep only `board.entries` whose player id is in it (in addition to the
  existing pool-membership filter). Drops non-participants' all-zero rows.
- **`tips(group_id)`** — restrict the per-(player, game) loop to
  `tippers_in(&players, &group_game_ids)`. Players with no tip anywhere in the
  group produce no rows. (Partial tippers are kept; cells for games they didn't
  tip within the group still render empty — see Non-goals.)
- **`standings(group_id)`** — restrict to
  `standings_tippers(&players, &leaf_group_ids)` over the group's leaf groups.

## What does NOT change

- **Frontend:** the grids and scoreboard render whatever rows the API returns;
  fewer rows means fewer empty/"hidden" rows. No component change is expected —
  but the variable per-group roster must be confirmed to render cleanly (see
  Testing).
- **Scoreboard materialisation (`recompute.rs`):** still computes every player;
  filtering is read-side only. "Participant" stays derivable, not stored.
- **No view-context argument** is added to any field, and no field's result
  depends on caller identity. Filtering is on domain facts (predictions exist),
  never presentation state (visibility, points).

## Edge cases

- **Participant who scored 0** → kept (criterion is "has predictions", not
  "has points").
- **Standings-only player in a group** (a standings prediction but no match
  prediction there) → dropped from All Tips, kept on the Standings grid.
- **Result-user** → excluded everywhere (folded into all three selectors).
- **Per-group roster varies** by design (the "Both" decision) — a participant
  appears only on the groups they tipped.

## Testing

- **`domain` unit tests:** `is_participant` truth table (result-user, empty,
  match-only, standings-only); `participants` / `tippers_in` /
  `standings_tippers` selection incl. result-user exclusion and empty input.
- **`api` integration tests (`InMemoryRepository`):**
  - `tips(group)` omits a player with no tip in that group; keeps a partial
    tipper; keeps a participant whose tips are still hidden by mutual commitment.
  - `scoreboard` omits a non-participant; keeps a participant who scored 0.
  - `standings(group)` omits a player with no standings prediction for the group.
- **E2E / manual:** All Tips for a group renders without the dropped rows and
  still shows kept players correctly (variable roster).

## Non-goals (separate work)

- **The "hidden" label.** For a *kept* partial tipper, games they didn't tip
  within a group still render as "hidden" rather than "— (no tip)". Fixing that
  label is a frontend concern, tracked separately.
- A "who hasn't predicted" admin view. If ever wanted, that is a *separate,
  explicitly-named* field — never a flag on these.
