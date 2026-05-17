# xpool — Deployment & Infrastructure

The **decided** deployment design, settled in design review. Authoritative —
this replaces the earlier un-ratified `REWRITE_ARCHITECTURE.md` draft.

See [`API.md`](./API.md) (the app being deployed), [`DATA_MODEL.md`](./DATA_MODEL.md)
(storage).

---

## 1. Topology

```
                     CloudFront distribution
                     /                       \
          /*  → S3 bucket (private)     /api/* → Lambda Function URL
              SPA static assets               Rust axum GraphQL app
                  ▲ OAC                              │  (lambda_http)
                                                     ▼
                                          DynamoDB table (on-demand)
                                                     ▲
                                          SSM Parameter Store (secrets)
```

One CloudFront distribution, two origins. No API Gateway (Lambda Function URL,
auth `NONE` — the app authenticates itself). No NAT/ALB/RDS.

## 2. Environments

Three: **local**, **dev**, **prod**.

| Env | What | Purpose |
|---|---|---|
| local | `docker compose` + the portable axum server + Vite | day-to-day development, fully offline |
| dev | a deployed AWS stack, on a subdomain | validate cloud aspects — IAM, CloudFront, Function URL, OAC, the deploy itself |
| prod | a deployed AWS stack, the real domain | production |

`dev` and `prod` are **one parameterized OpenTofu configuration**, instantiated
per environment — each with its **own** S3 bucket, CloudFront distribution,
Lambda, and DynamoDB table. No shared resources, no shared data.

## 3. AWS resources (per deployed environment)

- **S3 bucket** — private; the SPA's static assets. Reached only via CloudFront OAC.
- **CloudFront distribution** — origin `/*` → S3, origin `/api/*` → the Lambda
  Function URL; ACM certificate; custom domain.
- **Lambda function** — the Rust axum app, `provided.al2023` runtime, built with
  `cargo-lambda`; a **Function URL** (auth `NONE`).
- **DynamoDB table** — single table, on-demand, TTL attribute; the global +
  per-tournament key zones of [`DATA_MODEL.md`](./DATA_MODEL.md) §9. Long-lived
  (multi-tournament namespacing — never torn down).
- **SSM Parameter Store** — the TheSportsDB key as a `SecureString`.
- **IAM** — a least-privilege Lambda execution role (DynamoDB + SSM read); a
  GitHub OIDC role for CI.

## 4. Local development

Fully offline — no AWS account needed:

- `docker compose up` → **DynamoDB Local** + **MailHog** (email capture).
- `cargo run` the portable axum server → the GraphQL API on `localhost`.
- `npm run dev` → Vite dev server, proxying `/api` to the local server.
- Seed: `cargo run -p xtask -- import fwc26.json` against DynamoDB Local.

The same DynamoDB adapter runs against DynamoDB Local and real DynamoDB — only
the endpoint differs.

## 5. Infrastructure as code — OpenTofu

- **OpenTofu** (open-source, Terraform-compatible) — HCL.
- Community **`terraform-aws-modules`** for the boilerplate-heavy pieces
  (`s3-bucket`, `cloudfront`, `lambda`, `dynamodb-table`, `acm`); raw resources
  for the trivial bits — the stack is small, don't over-modularize.
- **Remote state:** an S3 backend (S3-native lockfile — no separate lock table);
  one state file per environment.
- **Build/deploy separation:** the Rust Lambda is built *outside* OpenTofu with
  `cargo-lambda`; OpenTofu ships the resulting artifact. The SPA is built with
  Vite and synced to S3.

## 6. CI/CD — GitHub Actions

- **On PR / push:** `cargo test`, `cargo clippy`, frontend build + tests.
  Validation only — feeds branch-protection checks.
- **On merge to `master`:** build the Rust Lambda + SPA → `tofu apply` to
  **dev** → S3 sync → CloudFront invalidation.
- **prod:** a **gated, deliberate promotion** — `workflow_dispatch` or a git
  tag / Release. Never automatic per-merge.
- **AWS auth:** GitHub **OIDC** → a scoped per-environment IAM role. No
  long-lived AWS keys in GitHub secrets.

## 7. Secrets & config

- **Secret** — the TheSportsDB premium key: SSM Parameter Store `SecureString`,
  read by the Lambda. Never in OpenTofu code or git.
- **Non-secret per-env config** — `CURRENT_TOURNAMENT_ID`, the domain: Lambda
  **env vars**, set by OpenTofu per environment.

## 8. Cost posture

All three environments cost **~$0 at rest** — the founding constraint:

- S3, CloudFront, Lambda — within free tier when idle.
- DynamoDB on-demand — $0 with no traffic.
- Lambda Function URL, SSM standard parameters — free.
- Only non-zero item: a Route53 hosted zone (~$0.50/mo) per custom domain.

Deliberately avoided — anything that bills while idle: API Gateway, NAT
Gateway, ALB, RDS, Aurora Serverless v2, provisioned DynamoDB.
