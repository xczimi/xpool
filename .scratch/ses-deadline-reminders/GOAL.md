# Ship SES deadline reminders (Phase B) — `/goal` orchestration prompt

Phase A is **done, merged, and pushed** (`master` @ `9d636a4`): the `crates/mail`
crate, reminder-dedup markers, the two admin mutations, the local MailHog path,
and the `xtask send-reminders` runner all exist and are green. What remains is
**Phase B** — the automated heartbeat — split into file-disjoint clusters built in
parallel by a team-leader loop.

The loop is now cleared to **deploy to `dev` only** (`AWS_PROFILE=xczimi`) and
exercise the path there — but under a hard **no-real-emails** safety gate (every
recipient must be a `@dev.invalid` address from the anonymized snapshot before any
send fires). **Prod stays your call** (the R1 activation gate): the loop stops at
the completion bar with a prod runbook and does **not** deploy prod or merge to
master.

The design is already settled (the 2026-06-27 grill → R1–R7 in the plan). **No new
grilling is needed** — this is a build, not a design.

> The `/goal` prompt below is kept **under 4000 chars** (the goal-condition limit).
> It leans on the plan/handoff for detail rather than restating it — the loop reads
> those first.

## Clusters (worktree + branch per cluster — built in parallel)

| Cluster | Branch / worktree | Scope | Files (disjoint) |
|---|---|---|---|
| `reminder-lambda` | `cluster/reminder-lambda` | Task 9 — scheduled Lambda entrypoint | `crates/api/Cargo.toml` (`[[bin]] reminder`), `crates/api/src/bin/reminder.rs` |
| `reminder-infra` | `cluster/reminder-infra` | Task 10 — TF + deploy script (apply R3/R4a/R5) | `infrastructure/reminder.tf`, `bin/deploy-reminder` |
| `reminder-copy` *(optional polish)* | `cluster/reminder-copy` | Polish bilingual EN/HU copy vs `.specs/LEGACY_I18N.md` | `crates/mail/src/templates.rs` |

The three surfaces don't overlap (`crates/api` vs `infrastructure/`+`bin/` vs
`crates/mail/src/templates.rs`), so they build concurrently with no collision.
Only `Cargo.lock` is shared (cluster `reminder-lambda` may bump it) — reconcile on
combine. **The xtask half of Task 9 is already merged** — `reminder-lambda` is only
the Lambda bin. `reminder-copy` is optional and does **not** block the completion bar.

The dynamic-port unlock (e2e suite allocates API/Vite ports per run via
`web/e2e/run-e2e.mjs`, plus per-branch `xpool-<branch>` tables) means any
verification stack stands up per-worktree without colliding on `:3000`/`:5173` —
so the clusters verify in parallel, not just author in parallel.

---

## The `/goal` prompt

```
/goal TEAM LEADER: ship SES deadline reminders PHASE B as parallel sub-agent clusters
(Phase A is merged to master; this is the automated heartbeat half). Lean on the
superpowers skills at every phase — don't freelance.

SPEC: docs/superpowers/plans/2026-06-27-cluster-backend-infra.md — read its
"## Revisions (post-grill 2026-06-27)" FIRST (R1–R7 override the task bodies).
Also read .scratch/ses-deadline-reminders/{PRD,PHASE-A-HANDOFF}.md.

CLUSTERS (one git worktree+branch each, PARALLEL, disjoint files):
 1 cluster/reminder-lambda — Task 9: crates/api/src/bin/reminder.rs + the
   `[[bin]] reminder` / required-features=["lambda"] in crates/api/Cargo.toml.
   The xtask send-reminders runner half is ALREADY MERGED — don't rebuild it.
 2 cluster/reminder-infra — Task 10: infrastructure/reminder.tf + bin/deploy-reminder,
   APPLYING revisions over the stale body: R3 last-call rate(1 hour)->rate(30 minutes);
   R4a var.mail_from xpool@->pool@xczimi.com + ADD var.reminder_reply_to (default "")
   wired to MAIL_REPLY_TO on the Lambda (mail reads it); R5 keep America/Los_Angeles
   digest TZ + add the rationale comment.
 3 cluster/reminder-copy (OPTIONAL, non-blocking) — polish EN/HU wording in
   crates/mail/src/templates.rs vs .specs/LEGACY_I18N.md; keep opt-out line + UTF-8
   charset; update template tests. Skip if it would delay 1+2.

DESIGN SETTLED — NO GRILL. Lone open call: ONE Lambda with {mode} payload (plan,
leaner) vs TWO — DEFAULT to one; flag me only on real friction. Don't re-litigate R1–R7.

PHASE 1 BUILD (PARALLEL): superpowers:using-git-worktrees (worktree+branch per
cluster); fan out (dispatching-parallel-agents/subagent-driven-development); each
implements its slice with executing-plans + TDD + systematic-debugging. Per-cluster
bar: lambda — `cargo build -p api` still SKIPS the bin, `--features lambda --bins`
compiles both, clean workspace build/clippy/test/fmt; infra — terraform fmt+validate
(init -backend=false), shellcheck clean, provision only (no apply yet); copy —
`cargo test -p mail` green. Close each with requesting-code-review +
verification-before-completion. GUARDRAIL: never amend/rebase/reset — forward commits only.

PHASE 2 INTEGRATE: combine into ONE branch ses-reminders-phase-b
(finishing-a-development-branch; reconcile Cargo.lock). Re-verify green per the plan's
Task 11 — including a REAL arm64 artifact: `cargo lambda build -p api --bin reminder
--release --arm64 --features lambda --output-format zip` -> target/lambda/reminder/bootstrap.zip.
LOCAL SWEEP (0 real sends, MailHog) per the handoff recipe: bin/local-dev --fresh, then
`MAIL_TRANSPORT=smtp bin/xtask send-reminders --mode last-call|digest` with XPOOL_NOW in
a pending window; inspect :8025 (captured, bilingual, HU decodes, deep links; re-run => deduped).

PHASE 3 DEPLOY TO DEV ONLY (AWS_PROFILE=xczimi): `bin/deploy dev` then `bin/deploy-reminder
dev`. SAFETY GATE before ANY send: scan the dev table — if ANY verified_email is not
@dev.invalid, STOP (reseed dev from the anonymized snapshot so every recipient is
@dev.invalid, or ask me). Only when EVERY recipient is @dev.invalid may you invoke a sweep.
Then exercise sendLastCallReminders / sendMatchdayDigest on dev as result-user (X-Dev-Now
into a pending window); confirm SES accepts and 100% of recipients were @dev.invalid. NO
real emails. Do NOT touch prod.

COMPLETION BAR — then STOP, hand back to me: branch ses-reminders-phase-b (clusters 1+2,
3 if free), all checks green, arm64 artifact built, local MailHog + dev evidence, dev
deployed + exercised with 0 real-email sends. Surface the branch, a per-cluster summary,
the evidence, and the prod runbook (bin/deploy prod -> bin/deploy-reminder prod -> exercise
as result-user). Do NOT merge to master, do NOT deploy prod — my calls. Keep working
across turns until the bar is met.
```

---

## Notes

- **Why Phase B, not the whole feature:** Phase A (mail crate, admin mutations,
  MailHog, xtask runner) is already on `master`. Phase B is the unbuilt automated
  half — the EventBridge-driven Lambda heartbeat. This GOAL ships it, exercised on
  `dev`, for your review before prod.
- **The no-real-emails guarantee:** `@dev.invalid` is a reserved, non-resolving TLD
  (RFC 6761) — no real inbox can receive it. The dev exercise runs against a table
  seeded from the anonymized snapshot (every `verified_email` rewritten to
  `<nick>@dev.invalid` while presence/null-ness is preserved), and the loop **scans
  and asserts** every recipient is `@dev.invalid` *before* any send. So the real SES
  path is exercised end-to-end while delivery physically cannot reach a person. If
  the dev table holds any real address, the loop stops rather than send.
- **Only 2 (3) clusters, not 4:** Phase B is a small, file-disjoint surface — a wider
  fan-out would be artificial. `reminder-lambda` (Rust) and `reminder-infra`
  (Terraform/bash) are the deploy-blockers; `reminder-copy` is optional polish on a
  third disjoint file that runs alongside for free but never blocks.
- **No grill:** the open questions were resolved in the 2026-06-27 grill (R1–R7). The
  lone surviving decision (one Lambda with a `{mode}` payload vs two) is defaulted to
  the plan; the loop flags it only on friction.
- **Apply the Revisions, not the stale Task 10 body:** the Task 10 HCL predates
  R3/R4a/R5 — `rate(1 hour)`, `xpool@xczimi.com`, and a comment-less digest schedule
  are all superseded. The cluster prompt names each.
- **Local review data:** `bin/local-dev --fresh` loads the newest `snapshots/` file
  into `xpool-<branch>`, so the local sweep runs against real-shaped data with 0 real
  sends. Refresh from AWS first with `bin/pull-data prod snapshots/prod-snapshot.json`
  if the snapshot is stale.
- **Prod is still your call:** after you review the dev deploy, ship prod with
  `bin/deploy prod` then `bin/deploy-reminder prod`, then exercise
  `sendLastCallReminders` / `sendMatchdayDigest` on prod as `result-user` and watch
  real deliveries before the schedules ever fire unattended (the R1 gate). The loop
  never touches prod.
