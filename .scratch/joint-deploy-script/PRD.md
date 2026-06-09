# Joint `bin/deploy` orchestrator script

Status: needs-triage
Area: bin / infra

## Idea

Add one `bin/deploy [dev|prod]` that runs the four existing deploy steps in the
right order, so a full deploy is a single command instead of four invocations
the operator has to remember and sequence by hand.

## Motivation

Deployment is currently split across four sibling scripts, each `[dev|prod]`:

| Script | Ships | Tooling |
|---|---|---|
| `bin/deploy-infra` | function shell, S3, CloudFront, DynamoDB, Route53, SSM, IAM | `tofu apply` |
| `bin/deploy-api` | Lambda **code** | `cargo lambda build` + `aws lambda update-function-code` |
| `bin/deploy-spa` | SPA bundle | `vite build` + `s3 sync` + CF invalidation |
| `bin/deploy-data` | tournament **data** into the live table | `xtask import` (+ `--seed`/`--bootstrap`) |

The split itself is deliberate and good (code is decoupled from infra via
`ignore_source_code_hash = true`, so shipping code doesn't require a tofu run).
But there's no single entry point, and they share a real ordering dependency:
**infra must exist before code/data can land on it.** Today the operator has to
know that, and run all four with the same `dev`/`prod` arg each time.

A thin orchestrator removes the foot-guns:

- one env arg, passed through to every step (no chance of mixing `dev` infra
  with a `prod` data load)
- the correct order encoded once: `infra → api + spa → data`
- the shared `AWS_PROFILE` / `AWS_REGION` boilerplate lives in one place

## Sketch

- `bin/deploy [dev|prod] [steps...]` — default runs all steps; named steps
  (`infra`, `api`, `spa`, `data`) let you run a subset, e.g.
  `bin/deploy prod api spa` to ship code without touching infra.
- Delegates to the existing scripts — it's an orchestrator, not a rewrite. The
  four keep working standalone.
- Order: `deploy-infra` first (it has the interactive `tofu apply` confirm),
  then `deploy-api` + `deploy-spa` (independent — could even run together),
  then `deploy-data` last.
- Pass the env arg straight through; surface each step's `==>` output so a
  failed step stops the chain (`set -euo pipefail`).

## Open questions

- Should `data` be in the default "deploy everything" set, or opt-in? A bare
  `import` is non-destructive, but most deploys don't need a re-import — maybe
  default to code-only (`infra api spa`) and require `data` explicitly.
- How to handle the prod confirmations: `deploy-infra` (tofu `yes`) and
  `deploy-data` (typed `prod`) each prompt. A full prod run would prompt twice —
  fine, or consolidate into one up-front confirm?
- `--dry-run` pass-through: only `deploy-data` supports it today. Worth a plan
  preview for the whole chain?

## Related

- `.scratch/sbx-deployment/` — deployment work.
- `.scratch/dev-deploy-clock-and-auth/` — dev-stage deploy concerns.
