# Sandbox (`sbx`) deployment — no-Auth0 + dev-clock playground

Status: needs-triage
Area: deployment / api / web

## Idea

Introduce a **third deployed environment, `sbx`**, that runs the dev-stub auth
(no Auth0) and the dev clock — the friction-free playground — so that **`dev`
can stay a faithful Auth0 mirror of prod**. Today the only deployed env besides
prod is `dev`, which uses real Auth0; there is nowhere to switch players freely
or time-travel without running the full local stack.

## Motivation

The dev-clock and no-Auth0 affordances do **not** belong on `dev` (that should
mirror prod, so the real auth + invite + deploy pipeline can be exercised). They
belong on a separate sandbox:

| Env | Auth | Clock | Data | Purpose |
|-----|------|-------|------|---------|
| prod | Auth0, invite-only | real | real | the live pool |
| dev  | Auth0 (mirror of prod) | real | seeded | test the real auth/invite/deploy pipeline |
| **sbx** 🆕 | **dev-stub (no Auth0)** | **dev clock** | demo | the friction-free playground |

On `sbx` the invite **bootstrap chicken-and-egg dissolves**: you dev-login as
any seeded player — including the result-user/admin — the seeded demo pool
already exists, and you can still exercise the real invite flow between demo
players for fun. Pair the dev clock with the [[scenario-test-data-generator]] to
inspect a whole tournament at any date.

## What it takes (mostly config + one small API gate)

The infra is already fully `var.environment`-parameterised (table
`xpool-<env>`, lambda `xpool-api-<env>`, domain from tfvars), so a third env is
cheap:

1. `infrastructure/env/sbx.tfvars` + `sbx.backend.hcl` (state key). Domain e.g.
   `pool-sbx.xczimi.com`; `auth0_domain = ""` (empty disables the Auth0 issuer
   in `TrustList::from_env`); `dev_affordances = "1"`. ACM cert + Route53 record
   provision automatically from `domain_name`.
2. Extend the `dev|prod)` allow-list to `dev|prod|sbx)` in `bin/deploy-infra`,
   `bin/deploy-api`, `bin/deploy-spa` (+ its `ALIAS`), `bin/deploy-data`.
3. **One API gate flag — `DEV_AFFORDANCES`** — that mounts the dev-login route
   (today gated by `LOCAL_AUTH_ISSUER`, `router.rs`) **and** lets
   `clock.rs::resolve_now` honour `X-Dev-Now` / `XPOOL_NOW`. Set only in
   `sbx.tfvars`.
4. SPA built in dev-auth mode for sbx (blank `VITE_AUTH0_*` → the dev auth bar
   appears — the same mechanism the e2e suite uses).

## Prod-safety bonus (worth doing regardless of the env decision)

`clock.rs::resolve_now` currently honours `X-Dev-Now` / `XPOOL_NOW`
**unconditionally on every env, including prod** — a latent hole the
code-comment itself flags. Introducing `DEV_AFFORDANCES` to turn the clock seam
*on* for sbx is the same change that turns it *off* for dev + prod.

## Open questions (env model not yet decided)

- **Adopt three envs (this PRD) vs. bolt affordances onto `dev` behind a flag?**
  Three envs keeps `dev` a true prod mirror; one-env-with-flag is fewer moving
  parts but blurs dev's role. **Undecided** — owner is still weighing it.
- One flag (`DEV_AFFORDANCES`) for both clock + dev-login, or separate toggles?
- Does sbx get its own Auth0-free login UI, or just the existing dev auth bar?
- Cost/upkeep of a third domain + cert + lambda + table — acceptable?

## Related

- Supersedes the *target* of [[dev-deploy-clock-and-auth]] (affordances move off
  `dev` onto `sbx`) — pending the env-model decision above.
- [[invite-only-hardening]] — prod tightens; sbx deliberately keeps the bypass.
- [[scenario-test-data-generator]] — the dev clock + demo data make sbx the
  place to eyeball scenarios across dates.

## Context

Filed 2026-06-07 while debugging why `pool@xczimi.com` couldn't act on the
`dev` deployment. Root cause there was unrelated (a missing
`IDENTITY#email#pool@xczimi.com` row + unset `RESULT_USER_EMAIL`, since fixed) —
but it surfaced that the dev-clock/no-Auth0 affordances want their own home.
