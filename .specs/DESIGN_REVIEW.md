# xpool — Design Review: Peter's Decisions & Rationale

A record of the design-review (grilling) session that produced the `.specs/`
set. Written to capture **Peter's positions, opinions, and reasoning** — in
particular the calls that *overrode* an initial recommendation, since those
carry the intent that the other spec docs only state as outcomes.

Peter has built and run xpool across ~3 tournaments (2010, 2012, 2014), each
hand-coded in under a week. He owns the original design deeply — that context
underpins much of what follows.

---

## 1. Peter's design philosophy

Recurring principles, in his own emphasis:

- **Generic, shape-agnostic data model; tournament-specific logic in code.**
  The domain model must serve any tournament format unchanged. FWC26 specifics
  (bracket resolution, Annexe C) belong in an `fwc26` code module, never baked
  into the model. This principle decided several questions on its own.
- **The legacy design was largely intentional, not debt.** The recursive tree,
  the one-match-group knockout wrapper, and the "result user" were *deliberate*
  choices that earned their place. The rewrite specs' "anti-pattern" framing of
  them was too harsh and conflated intentional design with real debt.
- **Build to learn and enjoy it.** Engineering choices weigh learning value and
  fun, not only expediency.
- **Cheap at rest, runnable locally.** Near-$0 idle cost and fully-offline local
  development are first-class, non-negotiable constraints.
- **Simplicity by default — but flexibility can outrank it.** Where frontend
  iteration freedom was at stake, Peter accepted more machinery.
- **Capture decisions; don't let docs lie.** Lock decisions into `.specs/`
  before moving on; banner or retire stale docs; never treat an un-ratified
  draft as settled.

## 2. Decisions — and where Peter overrode a recommendation

`[override]` marks a call made against the initial recommendation.

### Stack & language

- **Backend = Rust.** `[override]` — recommendation was Python (faster scoring
  port, better cold start). Peter: *"I want to learn and have fun."* Learning
  value outranks expediency.
- DynamoDB on-demand, React + Vite — confirmed early.

### Data model

- **Single-tournament *domain*, multi-tournament *storage*.** The domain model
  carries no tournament id; the datastore namespaces each tournament by a key
  prefix so the next tournament needs no reset. Peter's refinement — he wanted
  no datastore wipe between tournaments without polluting the domain model.
- **Recursive `GroupGame` tree kept.** `[override]` — recommendation was a flat
  FWC26-specific model. Peter: the recursive tree is reusable across tournament
  structures with little tweaking, and it keeps knockout games from being
  "overly special."
- **One-match-group knockout wrapper kept.** `[override]` — recommendation was
  to drop it. Peter: the wrapper lets the knockout advancer (extra-time /
  penalties) be modelled cleanly as the one-match group's `StandingsPrediction`
  — for predictions *and* results. A unification that pays off, not a hack.
- **Official results = a "result user".** `[override]` — recommendation was a
  first-class `MatchResult`. Peter calls this his "personal quirk": it keeps the
  model simple (one prediction type, symmetric scoring) and enables comparing
  any player against any other as a baseline — relative player-vs-player scores
  and "what-if" scenario evaluation.
- **Knockout placeholders = nullable team refs + a description string.**
  `[override]` — recommendation was a tagged sum type. Peter: keep the data
  model generic; bracket-resolution logic is tournament-specific code, not a
  model concern. A source-code file for tournament implementation detail is
  fine.
- **Lock = per-prediction `locked`; auto-lock complete drafts at the deadline.**
  Peter softened the legacy's harsh "forgot to lock → 0": be lenient as long as
  a complete prediction was entered.
- **Pools** — Peter's curve ball: independent scoreboards for subsets of players
  (friends, colleagues, referrals). Resolved as **scoreboard scoping only** —
  one global prediction set, a pool just filters the ranking. Explicit
  membership; **membership population/management is a separate concern**.
- **Identity → Person → Player.** Peter corrected a conflation: `Identity` is a
  *login credential* (global), `Person` is *the actual human* (global), `Player`
  is *tournament-scoped participation*. Login identity is not tournament-scoped;
  the player record is.

### Scoring

- 4-goal rule kept, threshold ≥4 — and Peter noted the threshold must stay
  *easy to change before launch*.
- **`ScoringConfig` = source-code constants, not a stored admin entity.**
  Peter's nuance: *"configurable could mean a lot of things"* — easy-to-edit
  constants tuned before launch are enough; no stored config entity or admin UI.
- Tie-breaker ladder uses only score-derivable criteria; `draw_order` is the
  manual prediction for everything else (drawing of lots, disciplinary cards,
  FIFA coefficient). Peter framed `draw_order`'s exact purpose.
- Per-round multipliers — explicit, admin-tunable before launch, not locked.
- Materialized `<t>#SCOREBOARD`. Peter corrected a sloppy claim: loading all the
  data is the real cost, not the scoring arithmetic.
- 90-minute rule kept; "perfect" = scored the maximum.

### Tournament import

- Hand-curated, git-committed `fwc26.json`; FotMob ICS used once as a scaffold.
- CLI / seed binary import.
- **Knockout resolution = automatic on result-lock.** `[override]` —
  recommendation was an operator-invoked action. Peter chose automatic,
  wholesale, self-correcting resolution.

### API & frontend

- **GraphQL.** `[override]` — recommendation was typed RPC (simpler). Peter
  wants a robust data model + API so the frontend can *"play around on frontend
  features"* without backend round-trips; he judged typed RPC as locking in
  what the frontend may do. Resolvers to be kept minimal.
- Group-level form editing (single `submitGroup` mutation, optimistic UI).
- Smart polling — only when results are actually possible (after a game ends).

### Deployment

- Lambda Function URL (no API Gateway).
- **Three environments: local + dev + prod.** `[override]` — recommendation was
  local + prod only. Peter wants a deployed `dev` to validate cloud aspects
  (deployment, IAM, AWS wiring).
- GitHub Actions CI/CD; auto-deploy `dev`, gated prod.
- **OpenTofu.** `[override]` — recommendation was AWS CDK. Peter rejected the
  "same language as the React SPA" argument as *"a very very weak argument,"*
  asked for an SST vs Terraform/OpenTofu comparison, and chose OpenTofu — open
  licensing, transparency, stability, and reuse of open-source modules.

## 3. How Peter ran the review (working-style notes)

- **Timeline is his concern, not the assistant's.** Stated twice, firmly — he
  has shipped xpool by hand in under a week, three times; capacity is his to
  judge. Estimates and scope-cutting from the assistant were out of scope.
- **Insisted on dependency order.** Stopped a jump to storage design with *"why
  are we picking storage type if we haven't agreed on the data model yet?"*
- **Rejects weak arguments explicitly** — e.g. the "same language as the SPA"
  point. Wants reasoning that holds up.
- **Wants well-defined options, not mush** — *"'REST' is just too ambiguous,
  present better options."* Asked for proper comparisons with background before
  choosing (GraphQL vs RPC; SST vs OpenTofu).
- **Capture before proceeding.** Repeatedly chose to lock decisions into `.specs/`
  before grilling the next area, and had stale docs bannered or retired so the
  review never built on something false.
- **Does not treat the assistant's drafts as settled** — flagged that
  `REWRITE_ARCHITECTURE.md` was an un-ratified Plan-agent draft, not a decision.

---

*This document is the "why" companion to the spec set. The other `.specs/`
docs state the decisions; this one preserves the intent and the overrides.*
