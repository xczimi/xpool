# xpool — Implementation Details for a Rewrite

> ⚠️ **PARTIALLY SUPERSEDED.** §1 (domain model) and §4 (anti-patterns) are
> superseded by [`DATA_MODEL.md`](./DATA_MODEL.md) — see its §11. In particular,
> §4's anti-pattern table **conflates intentional design** (the one-match-group
> knockout wrapper, the result-user) **with real debt**. §2 (scoring engine) and
> §3 (data ingestion) remain current.

The **technical** half of the rewrite specification: the domain model, data
ingestion, the legacy stack and its anti-patterns, and a suggested shape for
the new system. For *what the app does* — actors, journeys, use cases, screens
— see [`REWRITE_USE_CASES.md`](./REWRITE_USE_CASES.md). For the scoring math,
see [`GAME_RULES.md`](./GAME_RULES.md).

The guiding principle: **keep the domain and behavior, discard the platform.**
The legacy app is Google App Engine, Python 2.7, `webapp2`, Django 1.2
templates, the schemaless GAE datastore, and `memcache` — none of that should
be reproduced.

---

## 1. Domain model

### Tournament structure — a tree

A tournament is a **tree of game nodes**:

- A **Group node** (`GroupGame`) has a name and an optional parent. It contains
  either child Group nodes *or* a set of matches — internal nodes vs. leaf
  groups.
- A **Match** (`SingleGame`) is one fixture: home team, away team, kickoff
  time, location, and the leaf group it belongs to.
- Typical shape:
  ```
  Tournament (root)
  ├── Group Stage
  │   ├── Group A   → 6 matches
  │   ├── Group B   → 6 matches
  │   └── ...
  └── Knockout Stage
      ├── Round of 16 → each match wrapped in its own one-match group
      ├── Quarterfinals
      ├── Semifinals
      └── Final
  ```
- In the knockout stage **each match is its own group**, because betting
  deadlines and the standings bonus are computed per group.
- A node's **start time** is the earliest kickoff in its subtree. A node's
  **depth/level** drives the scoring multiplier (see `GAME_RULES.md`).

### Entities

Legacy names in parentheses.

| Entity | Represents | Key fields |
|--------|-----------|------------|
| **Player** (`LocalUser`) | A participant | email (unique, login id), nickname, full name, password (optional), `authcode` (magic-link token), `referrer` (who invited them), `active` flag |
| **Team** | A national team | name, short code (e.g. `bra`), flag image, external id |
| **GroupGame** | A node in the tournament tree | name, parent ref |
| **SingleGame** | A match | kickoff time, location, home team, away team, parent group, external match id |
| **MatchPrediction** (`Result`) | One player's score prediction for one match | player, match, homeScore, awayScore, `locked` |
| **StandingsPrediction** (`GroupResult`) | One player's predicted finishing order for a group | player, group, `draw_order` (manual tiebreak), `locked` |
| **Motd** | A site-wide banner message | text |

### Constraints to enforce explicitly

The legacy app enforced these only in application code (or not at all). The
rewrite should make them schema-level constraints:

- `(player, match)` unique — one prediction per player per match.
- `(player, group)` unique — one standings prediction per player per group.
- `email` unique per player.
- Score values bounded and validated (legacy never validated; allowed e.g. 15).

### Modeling fix: official results

The legacy app stores official match results as the *predictions of a fake
"result user"* (a `LocalUser` with email `fifa@fifa.com` / `uefa@uefa.com`).
Every scoring call compares a player against this synthetic account. This is a
hack. The rewrite should make the official result a **first-class field on the
Match**, or a dedicated `MatchResult` entity — not a user.

---

## 2. Scoring engine

Full math and known bugs are in [`GAME_RULES.md`](./GAME_RULES.md). Engineering
notes for the rewrite:

- Implement scoring as **pure, well-tested functions**: per-match points,
  per-group standings-order bonus, stage multiplier, and a recursive total.
- The legacy `GroupResult.get_ranks()` — which computes a group's standings and
  applies tie-breaking — is the worst code in the app. Its own comment calls it
  "one ugly beast": nested `reduce`/`filter`/`lambda` chains with O(n⁴)
  behavior on deep ties. Rewrite it as a clear standings function:
  1. Aggregate each team's W/D/L and goals from the player's predicted scores.
  2. Rank by points (3/1/0).
  3. Break ties using only head-to-head games among the tied teams (points →
     goal difference → goals scored).
  4. Break remaining ties with the player's manual `draw_order`.
- The legacy scoreboard is recomputed by walking the whole tournament tree on
  **every request** — O(players × matches). Instead, **materialize** group
  standings and scoreboard totals, and recompute them on an event: when an
  admin locks an official result.

### Known scoring bugs — decide before rewriting

(Detailed in `GAME_RULES.md`.)

1. The away-team "4-goal rule" branch checks the *home* score field twice — the
   away high-scoring fallback is broken.
2. The threshold is `> 4` (≥5 goals) but the rules text says "at least 4" (≥4).
3. The player-facing rules page text is stale (still says "2012 UEFA Euro").

---

## 3. Data ingestion (needs full redesign)

**How the legacy app got fixture data:** it **scraped cached HTML** from
FIFA.com / UEFA.com (stored in `fifa/` and `uefa/`) using a *bundled pure-Python
XPath engine* (`xpath/`). Per-tournament classes (`fifa2010.py`, `fifa2014.py`,
`uefa2012.py`) hardcoded which HTML files to parse, regex patterns for flag
filenames, date formats, and URL prefixes.

**Why it's fragile:**
- Any site redesign breaks the XPath selectors.
- Date parsing and flag-filename parsing are brittle regexes.
- FIFA 2010 and 2014 use different HTML, hence separate files.
- Adding a tournament means copying and editing a whole class file.
- Knockout placeholders (`1A`, `W53`, `L62`) are parsed but **never resolved
  into the model** — the KO bracket is scaffolded but unpopulated until manual
  admin edits.

**Official results were entered manually** by the admin — there was no live
results feed.

**Rewrite recommendation:**
- Define a **declarative tournament definition format** (JSON/YAML): teams,
  groups, fixtures, kickoff times, venues, and the knockout bracket with
  explicit references ("winner of match 53").
- Import tournaments from that file, or from an official sports-data API — not
  by scraping HTML. Drop the bundled `xpath/` library entirely.
- Add a job (or admin action) that **resolves knockout placeholders** to real
  teams once group results are final.
- Optionally add a results feed to reduce manual entry, with validation.

---

## 4. Legacy stack & anti-patterns — what NOT to carry over

| Legacy approach | Problem | Rewrite |
|-----------------|---------|---------|
| Official results stored as predictions of a fake "result user" (`fifa@fifa.com`) | Brittle singleton; conflates data | First-class result field/entity on Match |
| `everything()` — load *all* teams/games into one cached dict | Hard 256-item cap, silent truncation, manual invalidation | Real DB queries + indexes |
| 3-tier hand-rolled caching (`cached` / `perm_cached` / `perm_cached_class`) | Manual, error-prone invalidation | DB query caching / materialized standings |
| Sessions stored only in memcache | A memcache flush logs everyone out | Proper session store |
| Scoreboard recomputed by recursive tree walk on every request | O(players × matches) per request | Precompute/cache standings; recompute on result entry |
| `GroupResult.get_ranks()` — the "ugly beast", O(n⁴) tie handling | Unmaintainable | Clear, tested standings + tie-break function |
| Hardcoded secrets — Facebook app secret in `control.py` source | Credential leak | Env vars / secret manager; rotate the leaked secret |
| `NOW = datetime.utcnow()` captured once at module load | Deadlines go stale until process restart | Evaluate "now" per request |
| Hardcoded `from:` email, no send error handling | Fragile delivery | Configurable mail service with error handling |
| Magic-link token never expires, echoed into a flash message | Security: token leak, no revocation | Expiring token, never logged |
| Auto-linking identities by email with no consent | Account-takeover risk | Explicit confirmation |
| No input validation anywhere | XSS / bad data | Validate & escape all input |
| Python 2 only (`print` statements, `basestring`, `urllib2`) | Dead platform | Modern language/runtime |
| GAE-specific APIs (`users`, `memcache`, `mail`, `db`) | Deprecated, unportable | Standard framework equivalents |

---

## 5. Suggested rewrite shape (non-prescriptive)

- **Backend:** any modern framework with a relational DB. The domain is
  naturally relational — the legacy schemaless datastore added no value.
- **Tournament tree:** an adjacency-list `GroupGame` table (`parent_id`) or a
  nested-set, plus a `SingleGame` table.
- **Standings & scoring:** pure, well-tested functions. Materialize group
  standings and scoreboard totals; recompute on result entry, not per request.
- **Auth:** OAuth/OIDC for Google; email+password with proper hashing; expiring
  magic links for referrals; explicit confirmation when linking identities.
- **Frontend:** a modern SPA or server-rendered app; make the betting screen
  interactive while preserving the draft→locked state machine and the
  hidden-until-locked visibility rule.
- **Tournament import:** a declarative definition file; no scraping.
- **i18n:** keep English + Hungarian; structure for more.
- **Testing:** the scoring engine and standings/tie-break logic are the
  highest-value, highest-risk areas — cover them with a thorough test suite
  before porting anything else.

---

## 6. File reference (legacy)

| Area | Files |
|------|-------|
| Routing | `main.py`, `app.yaml` |
| Request handlers / auth | `control.py`, `facebook.py`, `settings.py` |
| Domain model | `model.py` |
| Scoring engine | `pool.py` |
| Tournament setup / scraping | `fifa*.py`, `uefa*.py`, `xpath/`, `fifa/`, `uefa/` |
| Templates | `view/*.html` |
| i18n | `locale/en`, `locale/hu` |
| Datastore indexes | `index.yaml` |
