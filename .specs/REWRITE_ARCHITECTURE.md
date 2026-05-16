# xpool — Serverless AWS Rewrite: Architecture Plan

## Context

The legacy **xpool** soccer score-prediction pool (Python 2.7 / Google App Engine)
is archived under `archive/`. Three spec docs define the rewrite target:
[`REWRITE_USE_CASES.md`](./REWRITE_USE_CASES.md) (behaviour),
[`REWRITE_IMPLEMENTATION.md`](./REWRITE_IMPLEMENTATION.md) (domain model &
anti-patterns), [`GAME_RULES.md`](./GAME_RULES.md) (scoring math). No new code
exists yet.

The pool runs only every 2–4 years for a tournament, has dozens of players, and
sits **idle most of its life**. The driving requirement: **near-$0 cost at rest**,
a static frontend on S3 + CloudFront, a stateless serverless backend, and the
ability to **run almost the entire stack locally** with no AWS dependency.

This plan picks a concrete stack and a phased build order. It does not write
application code — it establishes the architecture and project skeleton.

---

## Cost-at-rest target

| Component | Choice | Idle cost |
|---|---|---|
| Frontend | S3 (private bucket) + CloudFront + OAC | ~$0 (free tier) |
| API | **Lambda + Function URL** (no API Gateway) | $0 (free tier) |
| Database | **DynamoDB on-demand** | $0 — no provisioned capacity |
| Auth | App-managed (JWT + DynamoDB), no Cognito | $0 |
| Email | SES (sandbox/verified domain) | $0 |
| Custom domain | Route53 hosted zone (optional) | ~$0.50/mo — only non-$0 item |

**Explicitly avoided** (all charge while idle): RDS, Aurora Serverless v2
(~$43/mo floor), NAT Gateway, ALB, provisioned DynamoDB, API Gateway.

DynamoDB on-demand is the only AWS database that genuinely bills $0 at rest —
this drives a NoSQL single-table design. The dataset is tiny and access patterns
are simple, so this is a comfortable fit.

---

## Recommended stack

### Backend language — **Rust** (recommended; Python is the fallback)

The choice between Rust and Python turns on the usage pattern: with *"no users
around"* the Lambda is **almost always cold**, so nearly every real visit pays a
cold-start.

- **Rust** (`axum` + `lambda_http`, built with `cargo-lambda`, `provided.al2023`
  runtime): cold start ~15–30 ms. The scoring engine — flagged by the specs as
  the highest-risk code — benefits from Rust's type system and exhaustive tests.
  Cost: slower iteration; the ~100-line scoring engine is *rewritten*, not ported.
- **Python** (`FastAPI` + `Mangum`): cold start ~400–800 ms — noticeable on an
  app that is cold on nearly every visit. Upside: `archive/pool.py` ports almost
  directly; faster iteration; Pydantic validation.

**Recommendation: Rust.** The cold-start gap is the single biggest UX lever in an
otherwise-idle app, and the scoring engine is small enough that a fresh,
well-typed implementation is low-cost and lower-risk than a literal port. If
development velocity is the priority, Python remains viable with the identical
architecture below — only the `backend/` internals change.

### Other choices

- **Database:** DynamoDB on-demand, single-table design.
- **IaC + local dev:** AWS CDK (TypeScript) with a *portable* backend app — the
  same `axum` app runs as a plain local HTTP server **and** wraps into Lambda.
- **Frontend:** React + Vite SPA, `react-i18next` for English/Hungarian.

### Auth — app-managed, not Cognito

Cognito is $0 at rest but **has no good local emulator** (LocalStack Pro only),
which conflicts with the run-everything-locally goal. Instead: JWT sessions,
Argon2 password hashing, Google OIDC token verification, and expiring magic-link
tokens — all stored in the DynamoDB table. This keeps the full auth flow
runnable offline and matches the spec's emphasis on magic links and explicit
identity linking.

---

## Project structure

```
xpool/
  archive/            legacy app — untouched
  *.md                spec docs
  tournaments/        declarative tournament definitions (JSON) — no scraping
  backend/            Rust workspace
    crates/
      domain/         entities + scoring engine — pure, no AWS, fully unit-tested
      api/            axum router, handlers, DynamoDB repository
      lambda/         thin bin: lambda_http wrapper around the axum app
      server/         thin bin: runs the axum app as a local HTTP server
  frontend/           React + Vite SPA
  infra/              AWS CDK app (TypeScript)
  docker-compose.yml  DynamoDB Local + MailHog for offline dev
```

Key separation: `domain/` is platform-free and holds the scoring/standings
engine; `api/` adds HTTP + persistence; `lambda/` and `server/` are thin entry
points so prod and local run **identical** application code.

---

## Architecture

**Request flow (prod):** CloudFront with two origins —
- `/*` → private S3 bucket (SPA assets) via Origin Access Control
- `/api/*` → Lambda Function URL (auth `NONE`; the app verifies its own JWTs)

Single CloudFront distribution = same-origin frontend/API, no CORS, one cache.

**Database — single DynamoDB table**, entities keyed by `PK`/`SK`:

| Entity | PK | SK | Notes |
|---|---|---|---|
| Player | `PLAYER#<id>` | `PROFILE` | GSI on email for login |
| Tournament tree | `TOURN#<id>` | `NODE#<path>` | adjacency via path/parent |
| Match | `TOURN#<id>` | `MATCH#<id>` | official result as first-class fields |
| MatchPrediction | `PLAYER#<id>` | `PRED#<matchId>` | enforces (player,match) uniqueness |
| StandingsPrediction | `PLAYER#<id>` | `STAND#<groupId>` | enforces (player,group) uniqueness |
| Materialized scoreboard | `TOURN#<id>` | `SCOREBOARD` | recomputed on result lock |
| Magic-link / session tokens | `TOKEN#<hash>` | `META` | TTL attribute for expiry |

DynamoDB native TTL expires magic-link and session tokens automatically.

**Scoring engine** (`domain/` crate) — pure functions, rebuilt from
`GAME_RULES.md`, **fixing the three documented legacy bugs** (broken away-score
4-goal branch, `>4` vs `>=4` threshold, stale rules text). Replaces the O(n⁴)
`GroupResult.get_ranks()` with a clear standings + tie-break function. The
scoreboard is **materialized** and recomputed when an admin locks a result —
never per request.

**Tournament import:** declarative JSON files in `tournaments/`, applied via an
admin endpoint / CLI. No HTML scraping; the `archive/xpath/` library is dropped.

---

## Local development story

Everything runs offline — no AWS account needed:

- `docker compose up` → DynamoDB Local + MailHog (email capture).
- `cargo run -p server` → axum API on `localhost`, talking to DynamoDB Local.
- `npm run dev` in `frontend/` → Vite dev server, proxying `/api/*` to the
  local axum server.
- The scoring engine is exercised purely via `cargo test` in `domain/`.

Deploy path: `cargo lambda build --release` → `cdk deploy`.

---

## Phased build order

1. **Skeleton & infra** — workspace layout, `docker-compose.yml`, a minimal CDK
   stack (S3 + CloudFront + OAC, empty Lambda, DynamoDB table). Verify
   `cdk deploy` stands up an idle stack at ~$0.
2. **Domain + scoring engine** — `domain/` crate: entities, standings/tie-break,
   per-match & per-group scoring, multipliers. TDD; fix the three known bugs.
   This is the highest-risk, highest-value code — build and test it first.
3. **Persistence + API** — DynamoDB repository, `axum` routes for the read-only
   public pages (schedule, scoreboard, perfect, today).
4. **Auth** — JWT sessions, password (Argon2), Google OIDC, magic-link referral
   with explicit identity linking; SES email (MailHog locally).
5. **Prediction loop** — My Tips / All Tips: draft→locked state machine,
   hidden-until-locked visibility, deadline enforcement, standings prediction.
6. **Admin** — tournament import from JSON, official result entry (triggers
   scoreboard recompute), team/fixture edits, banner.
7. **Frontend** — React + Vite SPA for all screens; i18n EN/HU; wire to API.
8. **Ship** — `cargo lambda build` + `cdk deploy`; smoke-test the deployed app.

---

## Critical files (to be created)

- `backend/crates/domain/` — entities + scoring engine (port reference:
  `archive/pool.py`, `archive/model.py`; rules: `GAME_RULES.md`).
- `backend/crates/api/` — axum router + DynamoDB repository.
- `backend/crates/lambda/` & `backend/crates/server/` — thin entry points.
- `infra/` — CDK stack (S3, CloudFront+OAC, Lambda+Function URL, DynamoDB, SES).
- `frontend/` — React + Vite SPA.
- `tournaments/*.json` — declarative tournament definitions.
- `docker-compose.yml` — DynamoDB Local + MailHog.

## Verification

- **Scoring engine:** `cargo test -p domain` — exhaustive cases for per-match
  points, standings tie-breaks, multipliers; explicit regression tests asserting
  the three legacy bugs are fixed.
- **Local end-to-end:** `docker compose up`, run `server` + Vite, import a
  `tournaments/*.json`, walk Journey A (invite → magic-link login → predict →
  lock → admin enters result → scoreboard updates). Confirm MailHog received the
  invite email and hidden-until-locked visibility holds.
- **Deployed smoke test:** after `cdk deploy`, load the CloudFront URL, confirm
  SPA serves from S3, `/api/*` reaches Lambda, and a cold request returns
  promptly (Rust target: well under 100 ms cold).
- **Cost check:** with the stack deployed and idle, confirm AWS Cost Explorer
  shows ~$0/day (only the optional Route53 hosted zone if a domain is attached).
