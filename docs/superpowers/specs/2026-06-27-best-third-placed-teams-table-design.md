# Best Third-Placed Teams Table — Phase 1 (display)

**Date:** 2026-06-27
**Status:** Design approved, ready for implementation plan
**Phase 2 (editable tiebreak):** see `.scratch/best-thirds-table/phase-2-editable-tiebreak.md`

## Problem

FWC26 sends the **8 best third-placed teams** (of 12 groups) to the round of 32,
and a 495-row Annexe C lookup decides **which** group winner each qualifying
third faces. All of this is already computed server-side in `crates/fwc26`
(`best_thirds`, `annexe_c`, `resolve_bracket` / `ResolutionContext`), but **none
of it is exposed to the frontend**. The web app only sees raw knockout slot
descriptions like `3ABCDF` with no interpretation.

We want to **show the best-thirds ranking for visibility and transparency**: who
qualifies, on what stats, and how each qualifier's R32 pairing was derived.

## Whose ranking

This is a prediction pool, so the 12 thirds — and which 8 advance — depend on
whose group results you read.

- **My Tips page:** show the **player's predicted** ranking **and** the
  **official result** ranking *side by side* (mirroring how My Tips already
  shows player-vs-official for groups — not a toggle).
- **Schedule page:** show the **official-only** ranking, next to where the
  `3ABCDF`-style knockout slots appear, so anyone can see how those placeholders
  resolve. No per-player comparison.

A player predicts every group game, so their predicted ranking is **always
complete** (Annexe C fully resolvable). The official ranking is **provisional**
until the group stage finishes and fills in as results land.

## Non-goals (Phase 1)

- **Editable last-resort tiebreak** (reordering tied thirds, like the per-group
  `draw_order`). There is **no** player-controllable cross-group tiebreak today:
  when thirds tie on points / goal-difference / goals-for, `best_thirds` falls
  back to **group-letter order (A–L)**, and no domain field stores an
  alternative. Adding one touches the locked `domain/model.rs` contract,
  storage, the resolver, a mutation, and UI. Deferred to Phase 2 — captured in
  `.scratch/best-thirds-table/phase-2-editable-tiebreak.md`. Phase 1 keeps the
  group-letter fallback.
- All Tips page integration (a 12-row thirds table per player is too heavy for
  the multi-player comparison layout; revisit separately if wanted).

## Design

### 1. Backend — expose existing logic, add no new domain logic

Refactor the thirds computation that `ResolutionContext::build` already performs
into a reusable **pure function** in `crates/fwc26`:

```rust
pub struct ThirdPlaceRow {
    pub group: char,                    // 'A'..='L'
    pub team_id: Option<TeamId>,        // None until this group's 3rd is determinable
    pub points: i32,
    pub goal_diff: i32,
    pub goals_for: i32,
    pub qualifies: bool,                // top 8
    pub faces_winner_group: Option<char>, // R32 opponent group-winner, via Annexe C
    pub faces_game: Option<GameId>,       // the R32 match id, via Annexe C
}

/// Ranked best-first. Annexe C fields are set only once the qualifying set of 8
/// is fully determined; provisional/partial otherwise.
pub fn third_place_ranking(t: &Tournament, result: &Player) -> Vec<ThirdPlaceRow>;
```

`resolve_bracket` / `ResolutionContext` must **reuse** this function — no
duplicated ranking logic. Ranking criteria and the group-letter tie fallback are
unchanged from today's `best_thirds`.

**Provisional state:** until each group has a determinable 3rd place, rows still
appear (ranked by current stats) but `qualifies` and the Annexe C fields stay
unset, because the qualifying set isn't yet known.

### 2. GraphQL — one query, `null` = official

```graphql
type ThirdPlaceEntry {
  group: String!
  team: Team           # null until determinable
  points: Int!
  goalDiff: Int!
  goalsFor: Int!
  rank: Int!           # 1..12
  qualifies: Boolean!
  facesWinnerGroup: String  # e.g. "E"
  facesGame: ID             # e.g. the M74 game id
}

type ThirdPlaceRanking {
  entries: [ThirdPlaceEntry!]!
  complete: Boolean!   # all 12 groups' thirds determinable AND qualifying set known
}

# player: null -> official (result-user); a player id -> that player's predicted ranking
thirdPlaceRanking(player: ID): ThirdPlaceRanking!
```

The resolver does **no I/O and no domain logic** (per project convention) — it
loads the coarse items once and calls the pure `third_place_ranking` function.

- **My Tips** issues the query twice with aliases (`mine` + `official`).
- **Schedule** issues it once (`official`, `player: null`).

### 3. Frontend

- New `ThirdPlaceTable` component in its own file (per file-org rules), props
  drive single-column (Schedule, official) vs dual-column (My Tips, predicted +
  official) rendering. Top-8 rows visually highlighted; each qualifier shows its
  pairing label (e.g. "3A → Winner E · M74"). A "provisional — group stage
  incomplete" note when `complete` is false.
- i18n (en/hu) strings in `web/src/i18n/strings.ts`: table headers, the
  "qualifies" marker, the pairing label, and the provisional note.
- New class names need real CSS; verify the rendered page visually, not just
  green checks.

### 4. Testing

- **`crates/fwc26`** unit tests for `third_place_ranking`: ranking order;
  qualifies boundary (8th vs 9th); Annexe C pairing correctness for a known
  qualifying set; provisional/partial state when a group's 3rd isn't yet
  determinable.
- **`crates/api`** resolver test: official (`player: null`) vs a specific
  player; entry shape and `complete` flag.
- **`web`** e2e (boots the live stack): My Tips shows predicted + official
  thirds with top-8 highlighted and pairings; Schedule shows the official
  ranking. Use the dev clock to drive a state where group results exist.

## Files likely touched

- `crates/fwc26/src/lib.rs` — extract/add `third_place_ranking`, reuse in
  `ResolutionContext`.
- `crates/fwc26/tests/` — new tests.
- `crates/api/src/gql/types.rs`, `query.rs` — `ThirdPlaceEntry` /
  `ThirdPlaceRanking` types + `thirdPlaceRanking` query.
- `web/src/graphql/queries.ts`, `types.ts` — query + types.
- `web/src/components/ThirdPlaceTable.tsx` (new) + CSS.
- `web/src/pages/mytips/*`, `web/src/pages/SchedulePage.tsx` — mount it.
- `web/src/i18n/strings.ts` — en/hu strings.
- `web/e2e/` — new spec.
