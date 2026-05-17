# xpool — Scenario Catalog

The **authoritative, test-enabling** inventory of every xPool behaviour. Each
scenario has a stable ID, a Given/When/Then shape, and a `Tests` field linking
the real Rust/Playwright tests that cover it. This catalog **supersedes
`REWRITE_USE_CASES.md` §3** (the prose use cases); that document keeps its
narrative journeys (§1–2) and rules summary (§5) for onboarding.

It is built from two sources: the rewrite specs (`DATA_MODEL.md`, `SCORING.md`,
`API.md`, `FWC26_RULES.md`) **and** an archeology pass over the legacy
`archive/` app — so every legacy behaviour has an explicit verdict, including
the ones deliberately dropped.

Related: [`REWRITE_USE_CASES.md`](./REWRITE_USE_CASES.md) (journeys),
[`DATA_MODEL.md`](./DATA_MODEL.md), [`SCORING.md`](./SCORING.md),
[`API.md`](./API.md).

---

## How to read a scenario

```
### AREA-NN — Short title
Status: keep · Actor: Player · Screen: My Tips
Given  <precondition>
When   <action>
Then   <observable outcome>
Tests: <test name> (crate)  ·  web/e2e/<spec>
Note:  <optional clarification, rationale, or open point>
```

**Status legend** — every legacy behaviour gets one verdict:

| Status | Meaning |
|---|---|
| `keep` | Carried forward from legacy unchanged in intent. |
| `changed` | Kept, but the mechanism or rule was deliberately modified — the change is noted. |
| `dropped` | Legacy had this; the rewrite deliberately will **not**. Kept here as a recorded decision, with rationale. |
| `new` | Did not exist in legacy (Pools, the Identity/Person/Player split). |
| `future` | Decided in principle, mechanism/surface not yet built; `Tests` is a placeholder. |

`Tests: —` means **no test exists yet** — a coverage gap to close.

**Capability areas:** Public Browsing (`BROWSE`), Auth & Identity (`AUTH`),
Predictions & Locking (`PRED`), Scoring & Scoreboard (`SCORE`), Pools (`POOL`),
Tournament Setup & Results (`ADMIN`).

---

## Design decisions baked into this catalog

Settled in the grilling session that produced this document:

- **Auth**: Auth0 managed IdP — passwordless email, Google, email+password.
  `Identity → Person → Player`; the **Person layer owns identity-linking**
  (explicit confirmation, never silent). Auth mechanism wiring is `future`;
  the **behavioural contract** below is decided.
- **Invite-only**: no open self-signup. Two entry paths — a direct referral, or
  self-join with a custom-pool join code.
- **No admin role, no web admin features.** "Admin" = whoever can authenticate
  as the **result-user** (a `Player` like any other, with an `Identity`).
  Tournament setup is `xtask`.
- **Pools** are scoreboard-scoping only, members-only visibility, joined via a
  shareable join code.
- **Dropped**: MOTD banner, Facebook login, the legacy `LocalUserGroup`
  password gate, the bespoke `authcode` magic-link token.

---

## 1. Public Browsing — `BROWSE`

Everything a not-logged-in **Visitor** can do. Public pages: Home, Schedule,
Scoreboard, Perfect, Today, Rules.

### BROWSE-01 — Visitor views the schedule
Status: keep · Actor: Visitor · Screen: Schedule
Given  a tournament is imported.
When   the visitor opens Schedule.
Then   the full fixture list — kickoff times, venues, navigable by stage — is
       shown, read-only, no login required.
Tests: `loads_fwc26_with_expected_counts` (xtask) · `tournament_query_is_public` (api) · web/e2e/visitor-smoke.spec.ts · web/e2e/schedule-order.spec.ts

### BROWSE-02 — Visitor views the scoreboard
Status: keep · Actor: Visitor · Screen: Scoreboard
Given  results have been entered.
When   the visitor opens Scoreboard.
Then   a ranked leaderboard of total points is shown — overall and per
       stage, each stage labelled with its multiplier. Public.
Tests: `scoreboard_query_reflects_recompute` (api) · web/e2e/admin-scoreboard.spec.ts
Note:  The global ("everyone") scoreboard is public; **custom-pool scoreboards
       are not** — see POOL-06.

### BROWSE-03 — Visitor views the perfect-predictions list
Status: keep · Actor: Visitor · Screen: Perfect
Given  at least one player scored a maximum (4-point) match prediction.
When   the visitor opens Perfect.
Then   the celebratory list of players with perfect predictions is shown.
       Public.
Tests: `is_perfect_true_for_4_points`, `is_perfect_false_for_less_than_4`, `is_perfect_via_four_goal_rule_counts` (domain)

### BROWSE-04 — Visitor checks Today / Fresh
Status: keep · Actor: Visitor · Screen: Today
Given  matches exist within ±2 days of now.
When   the visitor opens Today.
Then   a flat list of up to **12** matches within the ±2-day window is shown
       with results; prediction/points columns are empty (not logged in).
Tests: —
Note:  Legacy window: `time` ≥ now−2d AND ≤ now+2d, `fetch(12)`
       (`archive/control.py:381`). Carried forward verbatim.

### BROWSE-05 — Visitor cannot reach player-only screens
Status: keep · Actor: Visitor · Screen: My Tips / All Tips
Given  the visitor is not logged in.
When   they request My Tips or All Tips.
Then   access is refused; they are prompted to log in. Betting and all-tips
       screens require a `Player`.
Tests: `me_requires_authentication`, `submit_group_requires_authentication` (api) · web/e2e/auth.spec.ts

### BROWSE-06 — Visitor reads the rules
Status: keep · Actor: Visitor · Screen: Rules
Given  any state.
When   the visitor opens Rules.
Then   the scoring and rules-of-play explanation is shown. Public, static.
Tests: —

---

## 2. Auth & Identity — `AUTH`

The credential layer is **Auth0** (passwordless email, Google, email+password);
mechanism wiring is `future`. The **behavioural contract** — what login,
invitation, claiming, linking, and profile editing must *do* — is decided.
`Identity → Person → Player` (`DATA_MODEL.md` §3).

### AUTH-01 — Dev-stub resolves the current player
Status: keep · Actor: Player · Screen: (all)
Given  the API is running in Phase-1 dev-stub mode.
When   a request carries an `X-Dev-Player` header naming a seeded player.
Then   the edge resolves that `Player` into the GraphQL context as
       `CurrentPlayer::Authenticated`; resolvers never re-authenticate.
Tests: `me_returns_player_when_authenticated` (api) · web/e2e/auth.spec.ts
Note:  This is the **shippable auth today**. AUTH-02..12 are `future` — the
       contract the real Auth0 integration must satisfy.

### AUTH-02 — Visitor with no header is unauthenticated
Status: keep · Actor: Visitor · Screen: (all)
Given  a request with no `X-Dev-Player` header.
When   it reaches the API.
Then   `CurrentPlayer::Visitor` is placed in context; player-only resolvers
       return an auth error.
Tests: `me_requires_authentication` (api)

### AUTH-03 — Login via passwordless email
Status: future · Actor: Player · Screen: Login
Given  a `Person` exists with a verified email identity.
When   they request a magic link and click it.
Then   Auth0 verifies email ownership; the app resolves
       `Identity → Person → Player` for the current tournament and the player
       is logged in. **No password is stored anywhere.**
Tests: —
Note:  Passwordless email unifies legacy login, the legacy "email+password"
       case, and the magic-link referral into one primitive.

### AUTH-04 — Login via Google
Status: future · Actor: Player · Screen: Login
Given  a `Person` has a linked Google `Identity`.
When   they sign in with Google through Auth0.
Then   the Auth0 `sub` matches the `Identity`, resolving to their `Player`.
Tests: —

### AUTH-05 — Login via email + password
Status: future · Actor: Player · Screen: Login
Given  a `Person` has an Auth0-hosted password identity.
When   they sign in with email + password.
Then   Auth0 verifies the (properly hashed, Auth0-managed) credential and the
       app resolves their `Player`.
Tests: —
Note:  changed from legacy — legacy stored passwords **in plaintext**
       (`archive/model.py:68`). Auth0 owns hashing, reset, lockout.

### AUTH-06 — Authenticated but unclaimed user is a logged-in visitor
Status: new · Actor: Visitor · Screen: (all)
Given  someone authenticates via Auth0 but **no `Person`/`Player`** exists for
       their verified email (xPool is invite-only).
When   they browse the site.
Then   they see public pages and a "you need an invitation to play" state;
       they **cannot** predict or open player-only screens.
Tests: —
Note:  New scenario forced by the invite-only + Auth0 combination — Auth0 makes
       authenticating trivial, but authentication ≠ participation.

### AUTH-07 — Player invites a friend (direct referral)
Status: changed · Actor: Player · Screen: Invite
Given  a logged-in player opens Invite.
When   they enter a friend's email, nick, and full name.
Then   a **pending** `Person` + `Player` is **eagerly created** keyed by that
       email, with `referrer` set to the inviter; a branded invitation email
       is sent.
Tests: —
Note:  changed — legacy generated a bespoke `authcode` token
       (`archive/control.py:343`). The rewrite drops the token: security rests
       on Auth0's verified-email login (AUTH-09).

### AUTH-08 — Invite to an existing email is rejected
Status: keep · Actor: Player · Screen: Invite
Given  the entered email already belongs to a `Person`.
When   the player submits the invite.
Then   the invite is rejected with a "this user is already in the system"
       message; no duplicate is created.
Tests: —
Note:  Legacy: `archive/control.py:340`.

### AUTH-09 — Invited friend claims their pending account
Status: changed · Actor: Invited player · Screen: Login → Profile
Given  a pending `Person`/`Player` exists for an invited email.
When   the invitee logs in via Auth0 with that **verified** email for the
       first time.
Then   the login claims and activates the pending account, links
       `Identity → Person → Player`, and lands them on Profile.
Tests: —
Note:  changed — the deep link in the email is convenience only; the verified
       email is the security boundary, replacing the non-expiring `authcode`
       (genuine debt per `DATA_MODEL.md` §11).

### AUTH-10 — Pending players are hidden until claimed
Status: changed · Actor: (system) · Screen: Scoreboard / All Tips
Given  an invited `Player` has been created but never logged in.
When   any player listing, scoreboard, or All Tips grid is rendered.
Then   the pending player is **excluded** until the account is claimed.
Tests: —
Note:  Replaces the legacy `active` boolean (`archive/model.py:71`,
       `LocalUser.actives()`). Activation moves from "first prediction save"
       (`archive/control.py:412`) to "first login claim" (AUTH-09).

### AUTH-11 — Self-join with a custom-pool join code
Status: new · Actor: Visitor → Player · Screen: Join
Given  a visitor holds a valid pool join code (see POOL-02).
When   they authenticate via Auth0 and supply their own nick/full name.
Then   a `Player` is **lazily** created, joined to that pool **and** the
       implicit global pool; `referrer` = the pool's owner.
Tests: —
Note:  The second invite path. Asymmetric with AUTH-07: here the joiner
       supplies their own profile (the owner never typed it).

### AUTH-12 — Existing person joins a pool by code
Status: new · Actor: Player · Screen: Join
Given  the authenticating email already belongs to a `Person`/`Player`.
When   they use a pool join code.
Then   **no new `Player`** is created — they are added to the pool's members
       only.
Tests: —

### AUTH-13 — Linking a second identity requires explicit confirmation
Status: changed · Actor: Player · Screen: Profile
Given  a player logged in via one identity; a login arrives whose verified
       email matches their `Person` via a different provider.
When   the system detects the match.
Then   it **prompts** "link these accounts?" — the `Person` layer links the
       `Identity` rows only on explicit confirmation.
Tests: —
Note:  changed — legacy linked silently by email (`archive/control.py:227`,
       `:242`). UC-2 calls explicit confirmation out specifically.

### AUTH-14 — Player edits their profile
Status: changed · Actor: Player · Screen: Profile
Given  a logged-in player.
When   they change their nick and/or full name.
Then   the `Player` profile is updated.
Tests: —
Note:  changed — legacy profile also edited email and password
       (`archive/control.py:355`). Email is now identity-bound (Auth0);
       password is Auth0-managed. The legacy "change password clears authcode"
       behaviour (`:365`) is obsolete (no authcode, no app-stored password).

### AUTH-15 — Logout
Status: changed · Actor: Player · Screen: (all)
Given  a logged-in player.
When   they log out.
Then   the Auth0 session is ended; they return to the public site as a
       Visitor.
Tests: web/e2e/auth.spec.ts
Note:  changed — legacy used a cookie + memcache session
       (`archive/control.py:93`); the rewrite uses Auth0 sessions.

### AUTH-16 — Facebook login
Status: dropped · Actor: — · Screen: —
Legacy had Facebook login (`archive/facebook.py`, 540 lines; `fb_current_user`
in `control.py:176`). **Dropped.** Rationale: Meta's OAuth app review is
ongoing maintenance for a hobby app; Google social covers the one-click case.
UC-2 already flagged "OAuth, or drop it." A test asserting Facebook is **not**
an offered connection guards the decision.
Tests: —

### AUTH-17 — Legacy `LocalUserGroup` password gate
Status: dropped · Actor: — · Screen: —
Legacy `LocalUserGroup` (`archive/model.py:111`) was a named user-group with a
`root` user and a plaintext **password** — the precursor to Pools. **Dropped.**
Superseded by `Pool` with an opaque, rotatable join code (POOL-02): a join code
is single-purpose and not conflated with an auth credential, and nothing is
stored in plaintext.
Tests: —

---

## 3. Predictions & Locking — `PRED`

The core loop. `MatchPrediction` (`homeScore`, `awayScore`, `locked`) and
`StandingsPrediction` (team ordering, `locked`). Lock state machine and
deadlines per `DATA_MODEL.md` §7.

### PRED-01 — Player saves a group's predictions as a draft
Status: changed · Actor: Player · Screen: My Tips
Given  a player viewing a group before its deadline.
When   they enter scores and choose **Save draft**.
Then   one `submitGroup` mutation persists all the group's matches; the
       predictions are drafts (`locked = false`) — they score 0 and are hidden
       from others.
Tests: `submit_group_saves_draft` (api) · web/e2e/mytips.spec.ts
Note:  changed — legacy did a full-page POST per save (`archive/control.py:417`).
       The rewrite saves a whole group in one coarse mutation (`API.md` §5–6).

### PRED-02 — Player locks a group's predictions
Status: keep · Actor: Player · Screen: My Tips
Given  a player with a complete draft for a group.
When   they choose **Lock**.
Then   `submitGroup` persists with `lock = true`; the predictions are locked,
       irreversible by the player, and become visible to others.
Tests: `submit_group_locks_predictions` (api) · web/e2e/mytips.spec.ts

### PRED-03 — A match can only be locked when both scores are filled
Status: keep · Actor: Player · Screen: My Tips
Given  a match prediction missing a home or away score.
When   the player attempts to lock.
Then   that match is not locked (legacy required `homeScore ≥ 0 AND
       awayScore ≥ 0`, `archive/control.py:452`).
Tests: —

### PRED-04 — Player predicts group standings, with manual tie order
Status: keep · Actor: Player · Screen: My Tips
Given  a group whose node carries a `StandingsPrediction`.
When   the player's predicted scores produce a qualification-affecting tie not
       broken by score-derivable criteria.
Then   the player manually orders the tied teams (`draw_order`), modelling
       ET / penalties / drawing of lots.
Tests: `submit_group_saves_standings`, `submit_group_without_standings_leaves_them_empty` (api)
Note:  See SCORE-09 for the tiebreak ladder.

### PRED-05 — Per-match lock for knockout matches
Status: keep · Actor: Player · Screen: My Tips
Given  a knockout match (its own one-match group, `LockMode::LockPerMatch`).
When   the player locks it.
Then   only that match locks — `submitGroup` with one match flagged
       (`API.md` §5). Other knockout matches are unaffected.
Tests: `submit_group_locks_predictions` (api)

### PRED-06 — Group-stage predictions lock together
Status: keep · Actor: Player · Screen: My Tips
Given  a group-stage group (`LockMode::LockTogether`).
When   the player locks the group.
Then   all predictions in the node lock as a unit.
Tests: `submit_group_locks_predictions` (api)

### PRED-07 — Deadline makes inputs read-only
Status: keep · Actor: Player · Screen: My Tips
Given  a group whose deadline (earliest kickoff in its subtree) has passed.
When   a non-result-user player opens it.
Then   inputs are read-only; the player can no longer edit.
Tests: `tournament_group_carries_subtree_deadline` (api)
Note:  Legacy `editable` rule, `archive/control.py:518`.

### PRED-08 — A complete draft auto-counts after the deadline
Status: keep · Actor: Player · Screen: (scoring)
Given  a player left a **complete** prediction as a draft (never locked).
When   the deadline passes.
Then   the prediction is **effective-locked** —
       `locked OR (now > deadline AND complete)` — and scores normally. The
       stored `locked` flag is never auto-mutated.
Tests: `effective_locked_after_deadline_and_complete`, `effective_locked_when_explicitly_locked`, `effective_locked_truth_table`, `score_tournament_auto_locked_after_deadline` (domain)

### PRED-09 — An incomplete draft scores zero after the deadline
Status: keep · Actor: Player · Screen: (scoring)
Given  a player left an **incomplete** or absent prediction.
When   the deadline passes.
Then   it is not effective-locked; it scores 0.
Tests: `effective_locked_after_deadline_but_incomplete`, `score_tournament_unlocked_prediction_scores_zero` (domain)

### PRED-10 — Before the deadline, an unlocked prediction is not effective-locked
Status: keep · Actor: Player · Screen: (scoring)
Given  a complete draft before its deadline.
When   it is scored or checked for visibility.
Then   it is **not** effective-locked — scores 0, hidden from others.
Tests: `effective_locked_before_deadline_not_locked`, `effective_locked_exactly_at_deadline_not_locked` (domain)
Note:  The deadline boundary is exclusive: *at* the deadline, still not locked.

### PRED-11 — Concurrent saves resolve via optimistic concurrency
Status: new · Actor: Player · Screen: My Tips
Given  the player item carries a `version` attribute.
When   two `submitGroup` writes race.
Then   the conditional write rejects the stale one; the loser retries against
       the fresh item.
Tests: `submit_group_resolves_version_conflict_with_retry` (api) · `player_conflict_returns_error`, `player_update_same_version_succeeds`, `dynamo_player_optimistic_concurrency_conflict`, `dynamo_player_optimistic_concurrency_success` (storage)
Note:  new — legacy had no concurrency control (`put()` last-write-wins).

### PRED-12 — Score input range
Status: changed · Actor: Player · Screen: My Tips
Given  a match prediction input.
When   the player picks a score.
Then   scores 0–9 are accepted (legacy `Result.score_list = range(10)`,
       `archive/model.py:340`).
Tests: —
Note:  changed (effectively) — under the 4-goal rule (SCORE-04) any score
       ≥ `high_scoring_threshold` (4) is equivalent for scoring, so 4..9 form
       one bucket. Exact bounds/validation still flagged open in
       `DATA_MODEL.md` §12.

---

## 4. Scoring & Scoreboard — `SCORE`

The scoring engine is **decided and authoritative in `SCORING.md`** — this
section catalogues the behaviour and links the regression suite; it does not
re-decide anything.

### SCORE-01 — Exact home + away + outcome scores the maximum
Status: keep · Actor: (engine)
Given  a prediction equal to the result on both scores.
When   scored.
Then   `1 + 1 + 2 = 4` points (the per-match maximum).
Tests: `score_match_exact_home_away_and_outcome`, `score_match_max_points` (domain)

### SCORE-02 — Correct outcome only
Status: keep · Actor: (engine)
Given  a prediction with the right W/D/L but wrong scores.
When   scored.
Then   `outcome_point` (2) only.
Tests: `score_match_correct_outcome_only` (domain)

### SCORE-03 — Wrong everything scores zero
Status: keep · Actor: (engine)
Tests: `score_match_wrong_everything` (domain)

### SCORE-04 — The 4-goal rule (per side, symmetric)
Status: changed · Actor: (engine)
Given  a side where both prediction and result are ≥ `high_scoring_threshold`
       (4).
When   scored.
Then   that side earns its exact-score point even if the counts differ.
Tests: `score_match_both_sides_high_scoring`, `score_match_four_goal_rule_only_result_high`, `score_match_four_goal_rule_away_below_threshold_no_match`, `score_match_four_goal_rule_prediction_exactly_4`, `score_match_threshold_3_does_not_trigger` (domain)
Note:  changed — fixes legacy bug #1: the legacy away check read the **home**
       field (`archive/pool.py:9,11`). See SCORE-05.

### SCORE-05 — Regression: 4-goal rule away check uses the away field
Status: changed · Actor: (engine)
Given  the legacy copy-paste bug where the away 4-goal check read `homeScore`.
When   scoring the away side.
Then   the away check reads the **away** field (`SCORING.md` §10 #1).
Tests: `score_match_regression_away_four_goal_rule_uses_away_field` (domain)

### SCORE-06 — Regression: 4-goal threshold is ≥4, not >4
Status: changed · Actor: (engine)
Given  the legacy `> 4` (≥5) threshold vs the stated ≥4.
When   scoring.
Then   the threshold is `≥ 4` (`high_scoring_threshold`, `SCORING.md` §10 #2).
Tests: `score_match_regression_threshold_is_exactly_4_not_5` (domain)

### SCORE-07 — Draw predictions
Status: keep · Actor: (engine)
Given  a predicted draw.
When   scored against a drawn / exact-draw result.
Then   outcome and exact-score points apply per side.
Tests: `score_match_draw_correct`, `score_match_draw_exact` (domain)

### SCORE-08 — Standings bonus rewards correct pair ordering
Status: keep · Actor: (engine)
Given  a player's predicted standings for a group.
When   scored.
Then   `standings_pair_point` is awarded per team-pair ordered the same as the
       official standings (4-team group → up to 6 pairs; one-match group → 1).
Tests: `standings_bonus_2_team_group_correct`, `standings_bonus_2_team_group_wrong`, `standings_bonus_perfect_match_4_team`, `standings_bonus_reversed_4_team`, `standings_bonus_partial_match`, `standings_bonus_custom_point_value`, `standings_bonus_empty_teams` (domain)

### SCORE-09 — Predicted-standings tiebreak ladder
Status: keep · Actor: (engine)
Given  a player's predicted match scores.
When   the implied group table is computed.
Then   teams rank by: points → head-to-head (points, GD, goals) → all-matches
       GD → all-matches goals → manual `draw_order` (`SCORING.md` §4).
Tests: `rank_group_4_team_by_points`, `rank_group_h2h_points_tiebreak`, `rank_group_h2h_tiebreak`, `rank_group_all_gd_tiebreak`, `rank_group_all_goals_tiebreak`, `rank_group_2_team_decisive`, `rank_group_2_team_decisive_away_wins`, `rank_group_2_team_draw_uses_draw_order` (domain)

### SCORE-10 — The 90-minute rule and the advancer
Status: keep · Actor: (engine)
Given  a knockout match.
When   scored.
Then   the per-match score is judged at **90 minutes**; the **advancer** rides
       on the one-match group's `StandingsPrediction` 2-team ordering — derived
       automatically from a decisive 90-minute score, set explicitly via
       `draw_order` for a predicted draw (`SCORING.md` §5).
Tests: `rank_group_2_team_draw_uses_draw_order` (domain)

### SCORE-11 — Stage multipliers
Status: changed · Actor: (engine)
Given  a node's round.
When   its total is computed.
Then   `(Σ match points + standings bonus) × multiplier[round]`, summed
       recursively — Group ×1, R32 ×2, R16 ×3, QF ×4, SF ×5, ThirdPlace ×5,
       Final ×6.
Tests: `multiplier_table_all_rounds`, `score_tournament_qf_multiplier`, `score_tournament_per_round_breakdown`, `score_tournament_regression_explicit_multiplier_table` (domain)
Note:  changed — fixes legacy bug #3: legacy derived multipliers from
       start-time order, which drifted when the bracket changed
       (`archive/pool.py:26`). Now an explicit table (`SCORING.md` §6, §10 #3).

### SCORE-12 — A "perfect" is a max-scoring prediction
Status: keep · Actor: (engine) · Screen: Perfect
Given  a match prediction.
When   it scores `perfect_threshold` (4) points.
Then   it is a "perfect" — defined as "scored the max", so a perfect reached
       via the 4-goal rule counts (`SCORING.md` §7).
Tests: `is_perfect_true_for_4_points`, `is_perfect_false_for_less_than_4`, `is_perfect_via_four_goal_rule_counts` (domain)

### SCORE-13 — Unlocked predictions and results score zero
Status: keep · Actor: (engine)
Given  a prediction or result that is not effective-locked.
When   scored.
Then   it contributes 0.
Tests: `score_tournament_unlocked_prediction_scores_zero`, `score_tournament_unlocked_result_scores_zero` (domain)

### SCORE-14 — Scoreboard is materialised and recomputed wholesale
Status: changed · Actor: (system) · Screen: Scoreboard
Given  a result is entered/locked (ADMIN-04).
When   the post-result hook fires.
Then   the `<t>#SCOREBOARD` item (`playerId → {stage → score}`) is rebuilt
       wholesale; every scoreboard read is one `GetItem` (`SCORING.md` §8).
Tests: `enter_result_recomputes_scoreboard`, `scoreboard_query_reflects_recompute` (api) · `scoreboard_round_trip`, `dynamo_scoreboard_round_trip` (storage)
Note:  changed — legacy used per-node memcache with an invalidation cascade
       (`archive/pool.py:75`). Rewrite: one item, full rebuild, no cascade.

### SCORE-15 — Player-vs-player / what-if scoring
Status: future · Actor: Player · Screen: (undesigned)
Given  the scoring engine is symmetric — `score(A, B)` works for any baseline.
When   a player rescores against another player's predictions as the baseline.
Then   relative / "what-if" rankings are produced.
Tests: —
Note:  Engine capability exists; the **feature surface is undesigned**
       (`SCORING.md` §11, `API.md` §9). Placeholder — a separate design effort.

---

## 5. Pools — `POOL`

A `Pool` is **scoreboard-scoping only** (`DATA_MODEL.md` §8) — a Player's score
is global and computed once; a pool just ranks that score among its members.
**Entirely new** — absent from the original use cases.

### POOL-01 — Player creates a pool
Status: new · Actor: Player · Screen: Pools
Given  a logged-in player.
When   they create a named pool.
Then   a `Pool` is created with them as **owner and member**; a join code is
       generated.
Tests: `pool_round_trip`, `pool_list_multiple`, `pool_overwrite`, `dynamo_pool_round_trip` (storage)

### POOL-02 — Join a pool with a join code
Status: new · Actor: Player · Screen: Pools
Given  a player holds a valid join code.
When   they join.
Then   they are added to the pool's members.
Tests: —
Note:  The code also bootstraps brand-new players — see AUTH-11/AUTH-12.

### POOL-03 — Owner rotates the join code
Status: new · Actor: Pool owner · Screen: Pools
Given  a pool owner.
When   they rotate the join code.
Then   a new code is issued; the old code no longer admits anyone. Existing
       members are unaffected.
Tests: —

### POOL-04 — Owner removes a member
Status: new · Actor: Pool owner · Screen: Pools
Given  a pool with members.
When   the owner removes a member.
Then   that player is dropped from the member list (their global score is
       untouched).
Tests: —

### POOL-05 — Member leaves a pool
Status: new · Actor: Player · Screen: Pools
Given  a member of a pool they do not own.
When   they leave.
Then   they are removed from the member list.
Tests: —

### POOL-06 — Pool scoreboard and roster are members-only
Status: new · Actor: Player / Visitor · Screen: Scoreboard
Given  a custom pool.
When   a non-member (player or visitor) tries to view its scoreboard or roster.
Then   access is refused — only the owner and members can see it; the pool is
       not discoverable.
Tests: —
Note:  Contrast BROWSE-02: the implicit "everyone" pool scoreboard is public.

### POOL-07 — Pool scoreboard ranks the global score among members
Status: new · Actor: Player · Screen: Scoreboard
Given  a pool and the materialised `<t>#SCOREBOARD`.
When   a member views the pool scoreboard.
Then   it is one `GetItem` of the scoreboard, filtered to pool members, sorted
       — the **same per-player score** as the global board.
Tests: —
Note:  Predictions, results, and the scoring engine are untouched by pools.

### POOL-08 — Owner renames a pool
Status: new · Actor: Pool owner · Screen: Pools
Tests: `pool_overwrite` (storage)

### POOL-09 — Owner deletes a pool
Status: new · Actor: Pool owner · Screen: Pools
Given  a pool owner.
When   they delete the pool.
Then   the pool is removed; members' scores and other pools are untouched.
Tests: —

### POOL-10 — Owner cannot leave without deleting
Status: new · Actor: Pool owner · Screen: Pools
Given  a pool owner is always a member.
When   they try to leave.
Then   they cannot — they must delete the pool. There is **no ownership
       transfer**.
Tests: —

### POOL-11 — A player belongs to many pools
Status: new · Actor: Player · Screen: Pools
Given  a player.
When   they create and/or join several pools.
Then   all are listed; no cap is enforced. The `pools` query returns the
       player's pools.
Tests: `pool_list_multiple` (storage)

### POOL-12 — The result-user is never a pool owner or member
Status: new · Actor: (system) · Screen: Pools
Given  the result-user `Player`.
When   pools are created or membership is computed.
Then   the result-user can never own or belong to a pool — consistent with its
       exclusion from all player listings.
Tests: —

---

## 6. Tournament Setup & Results — `ADMIN`

There is **no admin role and no web admin screens**. Setup is `xtask`; official
results are entered by whoever can authenticate as the **result-user**.

### ADMIN-01 — Import a tournament
Status: changed · Actor: Organizer · Screen: (xtask CLI)
Given  a tournament definition file (e.g. `tournaments/fwc26.json`).
When   `xtask import` is run.
Then   the `GroupGame` tree, `SingleGame`s, and `Team`s are written to the
       tournament's key namespace.
Tests: `loads_fwc26_with_expected_counts`, `import_is_idempotent` (xtask)
Note:  changed — legacy initialised the tree from a web admin action
       (`/admin/fifa`, `archive/control.py:633`). Now a CLI command.

### ADMIN-02 — Import is idempotent
Status: new · Actor: Organizer · Screen: (xtask CLI)
Given  a tournament already imported.
When   `xtask import` is run again.
Then   the result is unchanged — safe to re-run (DynamoDB Local is in-memory;
       re-import is routine).
Tests: `import_is_idempotent` (xtask)

### ADMIN-03 — Seed the result-user and demo players
Status: new · Actor: Organizer · Screen: (xtask CLI)
Given  an imported tournament.
When   `xtask seed` is run.
Then   the result-user, demo players, and a demo pool are created; re-running
       is idempotent.
Tests: `seed_is_idempotent` (xtask)

### ADMIN-04 — Enter an official result (web, as the result-user)
Status: changed · Actor: Result-user · Screen: My Tips
Given  someone authenticated as the result-user `Player`.
When   they enter and lock a match's score through My Tips.
Then   the result-user's `MatchPrediction` *is* the official result; the
       post-result hook recomputes the scoreboard (SCORE-14).
Tests: `enter_result_recomputes_scoreboard`, `enter_result_requires_admin` (api) · web/e2e/admin-scoreboard.spec.ts
Note:  changed — legacy had a dedicated `/admin/result` form
       (`archive/control.py:666`). Now it is just the result-user predicting.
       "Admin" = "can log in as the result-user"; `require_admin` checks
       `is_result_user`.

### ADMIN-05 — Enter an official result (xtask backup path)
Status: new · Actor: Organizer · Screen: (xtask CLI)
Given  an imported, seeded tournament.
When   the organizer enters a result via `xtask`.
Then   the result-user's prediction is set and the scoreboard recomputes — a
       scripting/backup path equivalent to ADMIN-04.
Tests: —

### ADMIN-06 — The result-user can edit results after kickoff
Status: keep · Actor: Result-user · Screen: My Tips
Given  a match whose kickoff has passed.
When   the result-user opens it.
Then   it is **still editable** for the result-user (to enter/correct the
       official score) — non-result-user players are locked out (PRED-07).
Tests: —
Note:  Legacy `editable` rule: `str(user) == result.key() AND now > kickoff AND
       not locked` (`archive/control.py:517`).

### ADMIN-07 — Knockout team slots resolve automatically
Status: changed · Actor: (system) · Screen: Schedule
Given  group results determine knockout participants.
When   results are entered.
Then   `SingleGame` `home`/`away` slot descriptions (`"2A"`, `"3ABCDF"`,
       `"Winner SF 1"`) resolve to concrete teams via the `fwc26` bracket
       resolver and the Annexe C lookup.
Tests: `test_resolve_group_positions`, `test_resolve_third_via_annexe_c`, `test_resolve_knockout_progression_undetermined`, `test_resolve_partial_group_results`, `test_resolve_self_correcting`, `test_resolve_loser_slot_not_yet_known` (fwc26) · `test_best_thirds_*`, `test_annexe_c_*` (fwc26)
Note:  changed — legacy required the admin to manually reassign a match's
       home/away teams (`/admin/game`, `archive/control.py:644`). The rewrite
       resolves them automatically; there is **no manual web override**.

### ADMIN-08 — MOTD banner
Status: dropped · Actor: — · Screen: —
Legacy had a `Motd` site-wide banner (`archive/model.py:461`), set by the admin
(UC-16), with a hardcoded default message (`archive/control.py:254`).
**Dropped entirely** — the feature was not considered worth carrying forward.
No banner, no `Motd` entity. The `Motd` struct, the `get_motd`/`put_motd`
repository methods, the `motd` query, the `setMotd` mutation, the web
`AdminBanner` page, and all their tests have been **removed**.
Tests: — (a test asserting `motd`/`setMotd` are absent from the schema would
       guard the decision)

---

## 7. Cross-cutting — `i18n`

### XCUT-01 — English / Hungarian language switch
Status: keep · Actor: any · Screen: (all)
Given  the site in either language.
When   the user switches language from the header.
Then   all UI strings, labels, column headers, and the invitation email render
       in the chosen language. i18n is first-class
       (`web/src/i18n/strings.ts`, `LEGACY_I18N.md`).
Tests: —
Note:  Legacy stored language in the session (`archive/control.py:120`).

---

## 8. Open / not yet grilled

- **AUTH-03..14** are `future` — the Auth0 integration is decided in principle
  (provider, methods, linking ownership) but not built; their `Tests` are
  empty by design.
- **Score-value bounds / validation** (PRED-12) — `DATA_MODEL.md` §12.
- **`Team` field set** — `DATA_MODEL.md` §12.
- **SCORE-15** what-if surface — `SCORING.md` §11.
- **Coverage gaps** — every `Tests: —` above is a scenario with no automated
  test yet; closing them is the point of this catalog.
