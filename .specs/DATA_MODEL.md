# xpool — Data Model

The agreed domain and storage model for the rewrite, settled in a design
review. Where this conflicts with [`REWRITE_IMPLEMENTATION.md`](./REWRITE_IMPLEMENTATION.md)
§1 (entities) or §4 (anti-patterns), **this document wins** — see
[§11](#11-corrections-to-earlier-specs).

Related: [`REWRITE_USE_CASES.md`](./REWRITE_USE_CASES.md) (behaviour),
[`GAME_RULES.md`](./GAME_RULES.md) (scoring), [`FWC26_RULES.md`](./FWC26_RULES.md)
(competition rules).

---

## 1. Principles

- The **domain model is single-tournament** — no `tournamentId` is threaded
  through domain types or the scoring engine.
- The **datastore is multi-tournament** — each tournament is an isolated
  key namespace, so the next tournament needs no reset/migration.
- The data model stays **generic and tournament-shape-agnostic**;
  tournament-specific logic (e.g. FWC26 bracket resolution) lives in
  application code, not in the model.
- Prefer **coarse-grained items** and **derived/computed** values over stored
  derived state.

## 2. Tournament scoping

A single config value, `CURRENT_TOURNAMENT_ID` (e.g. `fwc26`), selects the
active tournament. The `Repository` adapter is scoped to it and prefixes all
tournament-zone keys; **domain code never sees a tournament id**. Switching
tournaments = new namespace + flip the config. There is no multi-tournament UX.

## 3. Entities

```
Identity ──N:1──▶ Person ──1:N──▶ Player ──┬─▶ MatchPrediction (per SingleGame)
 (global)        (global)      (per-tournament) └─▶ StandingsPrediction (per GroupGame)

Tournament tree:  GroupGame ──┬─▶ child GroupGames   (internal node)
                              └─▶ SingleGames        (leaf group)
SingleGame ──▶ home/away: nullable Team + slot description
Pool ──▶ owner: Player,  members: [Player]
```

| Entity | Zone | Represents |
|---|---|---|
| **Identity** | global | A login credential (Google sub, email+password, magic-link). N per Person. |
| **Person** | global | The actual human. Persists across tournaments. Has 1..N Identities. |
| **Player** | per-tournament | A Person's participation in one tournament: profile (nick, full name), referrer, predictions. One per `(Person, tournament)`. |
| **Team** | per-tournament | A national team: name, short code, flag, external id. |
| **GroupGame** | per-tournament | A node in the tournament tree (internal group or leaf group). |
| **SingleGame** | per-tournament | One match: kickoff, venue, home/away team slots. |
| **MatchPrediction** | per-tournament | One Player's score prediction for one SingleGame. |
| **StandingsPrediction** | per-tournament | One Player's predicted team ordering for one GroupGame. |
| **Pool** | per-tournament | A named subset of Players sharing a scoreboard. |

## 4. Tournament structure — recursive tree

A tournament is a **recursive `GroupGame` tree**. A node holds either child
`GroupGame`s (internal) or `SingleGame`s (leaf group). The tree is kept generic
and shape-agnostic so it serves any tournament format without model changes.

**Each knockout match is wrapped in its own one-match group.** This is
intentional, not legacy debt — see [§10](#10-design-rationale).

A node carries:
- **Lock granularity** — *lock-together* (group stage: all predictions lock as a
  unit) or *lock-per-match* (knockout).
- **Standings prediction** — whether the node carries a `StandingsPrediction`.
- **Deadline** — derived: the earliest kickoff in the node's subtree. Uniform
  across group stage (first of 6) and knockout (the one match).
- **Multiplier** — explicit per node: GroupStage ×1, R32 ×2, R16 ×3, QF ×4,
  SF ×5, ThirdPlace/Final ×6. (Fixes the `GAME_RULES.md` multiplier bug.)

## 5. Predictions, results, scoring

- **`MatchPrediction`**: `homeScore`, `awayScore`, `locked`.
- **`StandingsPrediction`**: an ordering of the node's teams + `locked`. For a
  group-stage group, the ordering is the predicted final table; for a knockout
  one-match group, the 2-team ordering **is the extra-time/penalty advancer**.
- **Official results are the predictions of a "result user"** — a distinguished
  `Player` whose `MatchPrediction`s/`StandingsPrediction`s are the official
  outcomes. Scoring is symmetric: `score(predictionsA, predictionsB)`.
  - The official scoreboard scores every real Player against the result user.
  - Because scoring is symmetric, any Player can be used as the baseline —
    enabling player-vs-player relative scoring and "what-if" rescoring.
  - Cost accepted: queries listing "real players" must exclude the result user.
- The scoring engine is decided in [`SCORING.md`](./SCORING.md) — pure
  functions, `score(prediction, result, config)`.

## 6. Knockout placeholders

A `SingleGame`'s `home`/`away` are **nullable `Team` references**, each paired
with a **slot `description`** string (e.g. `"3ABCDF"`, `"Winner SF 1"`). The
data model does not interpret these. Resolving descriptions to concrete teams
as results land is **FWC26-specific application code** (using `FWC26_RULES.md`
§4 and the Annexe C lookup §5) — kept out of the generic model.

## 7. Lock state machine

- `locked` is a boolean **per prediction** (`MatchPrediction` and
  `StandingsPrediction`). There is no separate group-level lock state — a
  "lock whole group" action is a bulk operation; "group is locked" is derived.
- `locked` = the player's **explicit, early, irreversible** lock.
- **Effective-locked** (what scoring and visibility use) is derived:
  `locked OR (now > node deadline AND prediction is complete)`.
  Implemented as a **pure function** — no scheduled job, the stored `locked`
  is never auto-mutated.
- **"Complete" is per-prediction, not per-group.** A `MatchPrediction` always
  carries both `u8` scores, so it is *always* complete; it auto-counts after
  the deadline. A `StandingsPrediction` is complete when its `ordering` is
  non-empty. There is no group-level "all matches predicted" requirement on
  this auto-count path: a player who predicted only *some* games of a
  `LockTogether` group still has each filled game auto-count after the
  deadline, while the unpredicted games (no `MatchPrediction` at all) score 0.
  An "incomplete draft scores 0" therefore means an *absent or empty*
  prediction, not a partially-filled group.
- **Visibility** (UC-9): a prediction is visible to others when
  `effective-locked OR match kicked off`.

## 8. Pools

A `Pool` is **scoreboard scoping only**: `(id, name, owner: Player,
members: [Player])`. Players make **one global prediction set**; a Player's
score is computed once. A pool scoreboard ranks that same score among the
pool's members. The "everyone" scoreboard is the implicit root pool.
Predictions, results, and the scoring engine are untouched by pools.

Membership is **explicit lists** (no rule-based pools). A Player creates a pool
and owns it. **Pool membership population/management is a separate concern**,
out of scope for this model — `Pool` just holds the member list.

## 9. Storage layout

Single DynamoDB table, on-demand. Two key zones:

**Global zone** (no tournament prefix):

| Item | Key | Holds |
|---|---|---|
| Person | `PERSON#<id>` | the human; linked Identity ids |
| Identity | `IDENTITY#<provider>#<providerId>` | credential; `person_id` |

**Per-tournament zone** (prefixed `<tournamentId>#`):

| Item | Key | Holds |
|---|---|---|
| Player | `<t>#PLAYER` / `<playerId>` | profile **+ all** MatchPredictions + StandingsPredictions; `person_id`; `referrer`; `version` |
| Tournament | `<t>#TOURNAMENT` | the GroupGame tree, SingleGames, Teams |
| Pool | `<t>#POOL` / `<poolId>` | name, owner, member ids |
| Scoreboard | `<t>#SCOREBOARD` | materialized `playerId → {stage → score}`; recomputed wholesale on result change (see [`SCORING.md`](./SCORING.md) §8) |

Notes:
- **Coarse-grained**: a Player's whole prediction set lives on the one Player
  item (bounded — 104 matches + 12 standings, a few KB). A player's page = one
  `GetItem`; all players = one `Query` of the `<t>#PLAYER` partition.
- The **result user is a Player item** like any other.
- A prediction save is a read-modify-write of the whole Player item, guarded by
  a **`version` attribute + conditional write** (optimistic concurrency).
- Persistence is a single **DynamoDB adapter** behind a `Repository` trait;
  the trait also has an in-memory fake for unit tests. Local dev / integration
  tests run against **DynamoDB Local**.

## 10. Design rationale

Decisions that override the existing specs or are non-obvious:

- **Recursive tree** (vs a flat FWC26 model): shape-agnostic — the next
  tournament is a data change, not a model change.
- **One-match-group wrapper for knockout** (`REWRITE_IMPLEMENTATION.md` calls it
  "awkward" — that critique is incomplete): the wrapper gives every knockout
  match a `StandingsPrediction`, whose 2-team ordering cleanly encodes the
  extra-time/penalty advancer — for predictions *and* results. It also makes the
  deadline rule uniform (per group node). It is a unification that pays off.
- **Result user** (vs a first-class `MatchResult`): keeps one prediction type
  and a symmetric `score(A, B)`, which buys player-vs-player relative scoring
  and "what-if" rescoring for free. The cost (filtering the synthetic user from
  player listings) is accepted.
- **Single-tournament domain / multi-tournament storage**: the tournament id is
  a storage namespace, never a domain field.

## 11. Corrections to earlier specs

- `REWRITE_IMPLEMENTATION.md` §1 entity table is **superseded** by §3 here
  (notably: `Identity`/`Person`/`Player` split; `Pool`).
- `REWRITE_IMPLEMENTATION.md` §4's anti-pattern table **conflates intentional
  design with real debt**. The one-match-group wrapper and the result-user are
  *intentional design* (§10), not debt. Genuine debt remains: the `everything()`
  256-item cache, hardcoded secrets, `NOW` at module load, non-expiring magic
  links, no input validation, Python 2.

## 12. Open / deferred

- **Auth mechanism** — deferred (Auth0 vs app-managed). Phase 1 uses a dev stub
  behind a single auth seam: the edge resolves an `Identity` → `Person` →
  `(Person, current tournament)` → `Player`.
- **Pool membership management** — separate concern, not modelled here.
- **Minor / not yet grilled**: `Team` fields, score-value bounds/validation.
- **Motd** (site-wide banner) was **dropped** — see `SCENARIOS.md` ADMIN-08.
