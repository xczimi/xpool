# TheSportsDB Reported-Results — Production Release Runbook

**Date:** 2026-06-18
**Feature merged:** PR #15 → `master` (`b543e2c`). Design: `docs/superpowers/specs/2026-06-14-sportsdb-reported-results-design.md`.
**What ships:** assisted official-result entry — the admin (result-user) opens a group in My Tips and the score inputs pre-fill from TheSportsDB (per-event lookup, suggested regardless of finished-status; admin confirms and submits through the unchanged `submitGroup` path).

---

## Why this release is not a plain code deploy

A normal deploy is `bin/deploy <env>` (= `infra api spa`, `data` opt-in). This feature needs **all four steps, including `infra` and `data`**:

- **`infra`** — `bin/deploy-infra` (`tofu apply`) wires the new `data.aws_ssm_parameter.thesportsdb_key` read and injects `THESPORTSDB_API_KEY` into the Lambda env (`infrastructure/lambda.tf`). **Without it the deployed api has no key → `reportedResults` returns `[]`** (graceful, but the feature is inert).
- **`data`** — `bin/deploy-data` re-imports `tournaments/fwc26.json`, landing the **72 group-game `external_id`s** on the live table (players/pools untouched). **Without it the resolver can't map idEvent → game → returns `[]`.**
- **`api` / `spa`** — the new resolver + the SPA pre-fill.

---

## Prerequisites (verify, don't assume)

1. **AWS:** `export AWS_PROFILE=xczimi` (the deploy scripts default to this) · region `ca-central-1` · account `869028341442`. Confirm: `aws sts get-caller-identity --profile xczimi`.
2. **SSM keys already populated** (done 2026-06-14 — verify they decrypt to the premium key, not the placeholder):
   ```sh
   for e in dev prod; do aws ssm get-parameter --profile xczimi --region ca-central-1 \
     --name "/xpool/$e/thesportsdb-api-key" --with-decryption --query Parameter.Value --output text; done
   ```
   Both must print the premium key. (`infrastructure/ssm.tf` has `ignore_changes=[value]`, so `tofu apply` will NOT clobber them.)
3. **On `master` at `b543e2c` or later**, clean tree, `cargo test --workspace` + `cd web && npm run build && npm run lint` green (already verified pre-merge).

---

## Release — dev first, then prod

### 1. Deploy to dev
```sh
AWS_PROFILE=xczimi bin/deploy dev infra api spa data
```
- At the `tofu apply` (infra) step, **review the plan**: expect a new `data.aws_ssm_parameter.thesportsdb_key` and the Lambda gaining the `THESPORTSDB_API_KEY` env var — and **no change to the `aws_ssm_parameter` value** (the `ignore_changes` guard).
- The `data` step does a non-destructive `put_tournament` — it refreshes games/teams (now with `external_id`s) and leaves players/pools alone.

### 2. Verify dev (`pool-dev.xczimi.com`)
- **Lambda env present:**
  ```sh
  aws lambda get-function-configuration --profile xczimi --region ca-central-1 \
    --function-name "$(cd infrastructure && tofu output -raw lambda_function_name 2>/dev/null || echo xpool-dev)" \
    --query 'Environment.Variables.THESPORTSDB_API_KEY' --output text
  ```
  Must be non-empty. (Derive the exact function name from the tofu output / `infrastructure`; `xpool-dev` is the expected pattern.)
- **Mapping landed:** the deployed tournament has 72 `external_id`s (the `data` step's import summary prints `104 games`; the JSON it imported is the committed one with 72 mapped).
- **Functional (the real check):** sign in as the **result-user** (Auth0 admin, `result_user_email`) on `pool-dev.xczimi.com`, open **My Tips → a group with a finished, not-yet-entered match**. Its score should pre-fill. (Pre-fill only appears for finished-and-unentered games — pick a group whose latest match SportsDB has scored but the admin hasn't entered.)
- **No regression:** Home / Schedule / Scoreboard load; a non-admin sees no change (the `reportedResults` query is paused for them).

### 3. Deploy to prod
```sh
AWS_PROFILE=xczimi bin/deploy prod infra api spa data
```
- Two confirmations surface in turn: the infra `tofu apply` approval, and `deploy-data`'s typed `prod` gate. Read both.

### 4. Verify prod (`pool.xczimi.com`)
Repeat the dev verification against prod (Lambda env var, functional pre-fill as the admin, smoke test).

---

## Rollback (low risk — the feature degrades safe)

The feature **never blocks manual entry**: any failure (no key, SportsDB down, unmapped game) makes `reportedResults` return `[]`, and the admin types as before.

- **Code:** redeploy the previous `master` — `bin/deploy <env> api spa` from the prior commit (code is decoupled from infra via `ignore_source_code_hash`, so this is a fast code-only ship).
- **Key/infra:** to disable the integration, blank the env var (or the SSM value) — the api falls back to `NullSource`. The env-var injection is additive; no destructive infra change to undo.
- **Data:** the `external_id` backfill is an additive `Option` field — old code ignores it, so no data rollback is needed even if code is reverted.

---

## Known follow-ups (not blockers)

- **Knockout fixtures (M73–M104) are unmapped** (`external_id` null) — TheSportsDB hasn't published them and our slots are still placeholders. When they appear: re-run `cargo run -p xtask -- reconcile-events` (against a table with current data), backfill the new ids into `tournaments/fwc26.json`, commit, and `bin/deploy <env> data`.
- **Team aliases:** three name aliases (`Turkiye→Turkey`, `Czechia→Czech Republic`, `Bosnia…→Bosnia-Herzegovina`) plus diacritic folding resolve the full group roster. If a live reconcile prints `# Unresolved teams (N>0)`, add the alias in `crates/xtask/src/reconcile.rs` and re-run.
- **No UI status marker** — by decision, the pre-fill shows no source/finality badge; the admin judges finality by watching the match (see the design doc). Don't add one.
- **#2 live-preview** (provisional points during a match) reuses the same `sportsdb` crate + `ReportedResult` type — a separate future spec.
