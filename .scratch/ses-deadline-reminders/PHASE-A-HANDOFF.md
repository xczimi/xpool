# SES Deadline Reminders — Phase A handoff

**Status (2026-06-28):** Phase A **complete, validated, merged to `master`, and pushed** (`origin/master` @ `14593c8`). Phase B (scheduled Lambda + EventBridge Terraform) intentionally **not built**.

Authoritative spec: [`docs/superpowers/plans/2026-06-27-cluster-backend-infra.md`](../../docs/superpowers/plans/2026-06-27-cluster-backend-infra.md) — read its **"Revisions (post-grill 2026-06-27)"** section (R1–R7); those override the original task bodies.

---

## What was done

### 1. Plan grilled & revised (the "execution-ready in haste" fix)
The plan had been marked execution-ready without checking its load-bearing assumptions. Grilled to decisions (R1–R7 in the plan, pushed to master):
- **Verified:** ~41 real recipients exist in prod (anonymized snapshot preserves null-ness); SES has **production access** (confirmed).
- **R1 — phased go-live:** Phase A (mail crate + admin mutations + MailHog) merges with zero unattended sends; Phase B (Lambda + Terraform) is gated behind exercising the admin mutation on prod first.
- **R2 — last-call window:** 40-min slot+slack (`(now, now+40min]`), driven by 30-min ticks, dedup per `(person, group)`. Robust to EventBridge jitter; covers `:30` kickoffs (99/104 fixture kickoffs are `:00`, 5 are `:30`).
- **R4 — manual opt-out:** a bilingual "reply to stop" line in both bodies + an admin-side exclude. No SES→SNS bounce/complaint infra in v1.
- **R4a — From/Reply-To:** From = `pool@xczimi.com` (reuses Auth0's monitored sender, DKIM-aligned); Reply-To = `MAIL_REPLY_TO` env or From.
- **R5 — digest timing:** 00:00 `America/Los_Angeles` (documented rationale: lands before the earliest NA kickoff).
- **R7 — two admin mutations** (`sendLastCallReminders` / `sendMatchdayDigest`), not one enum-switched mutation. `mail::ReminderMode` retained for the xtask runner + future Lambda.

### 2. Phase A built (8 tasks, subagent-driven-development)
Each task: implementer → spec review → code-quality review → fix loop. Tasks 4/5/6 fanned out across 3 isolated worktrees and merged back.
- **`crates/mail`** (new): `MailSender` seam (SES / SMTP→MailHog / Null + Capturing adapters), pure selection/window/dedup-keys, bilingual EN/HU plaintext templates + opt-out, sweep orchestrator (global per-player, dedup-after-send, no empty digests). 25 unit tests.
- **`crates/storage`:** reminder-dedup marker rows (`<t>#REMINDER`) in both adapters.
- **`crates/api`:** mail seam threaded through `build_app`/schema (NullSender default; real sender only in `main.rs`); the two admin mutations (admin-gated, request-clock).
- **`crates/xtask`:** `send-reminders --mode last-call|digest` local runner.

### 3. Validated locally against MailHog
Ran the sweep against the branch table → 19 emails captured in MailHog, **0 real sends** (all `@dev.invalid`). Confirmed multi-email fan-out on real data and the bilingual deep link `/mytips/<group>#<group>` (path resolves the group, `#` is the scroll anchor).

### 4. i18n bug found & fixed during validation (`bcf6034`)
Emails lacked a UTF-8 charset declaration — **would have broken Hungarian in prod too**, not just MailHog:
- **SMTP/lettre:** bare `.body(String)` omitted `Content-Type` → now `SinglePart::plain` (`text/plain; charset=utf-8` + `MIME-Version`).
- **SES:** `Content` defaults to ASCII → now `.charset("UTF-8")` on subject + body.
Verified: headers now `Content-Type: text/plain; charset=utf-8`; MailHog renders Hungarian correctly (user-confirmed).

Also shipped earlier this session (separate branch, already on master `4cfa946`): chart-stretch + best-thirds-order UI fixes + `bin/local-dev --fresh`.

---

## What's left for a new session

1. **Deploy + exercise on prod (the Phase-B gate).** `bin/deploy [dev|prod]`, then run `sendLastCallReminders` / `sendMatchdayDigest` against prod (admin = result-user) and watch real deliveries/bounces. Note: the admin mutation respects `X-Dev-Now`, and exercising it writes real dedup markers.
2. **Build Phase B** per the plan's Task 9 (Lambda half) + Task 10:
   - `crates/api/src/bin/reminder.rs` (scheduled Lambda entrypoint, `required-features = ["lambda"]`).
   - `infrastructure/reminder.tf` (Lambda + hourly→**30-min** last-call rule + daily LA-midnight digest schedule) + `bin/deploy-reminder`.
   - **Open decision:** one Lambda with a `{mode}` payload (current plan, leaner) vs. two Lambdas (per-mode logs/metrics/IAM). Both consume the same shared `mail::run_*_sweep`.
   - Apply R3: the last-call EventBridge rule is `rate(30 minutes)`.
3. **Refine email copy.** Currently placeholder-grade by choice (plaintext, bilingual stacked). Polish wording (align with `.specs/LEGACY_I18N.md`) when ready.

---

## Pointers
- Plan + revisions: `docs/superpowers/plans/2026-06-27-cluster-backend-infra.md`
- PRD: `.scratch/ses-deadline-reminders/PRD.md`
- Memory (loaded at session start): `ses-reminders-phase-a-status`, `ses-reminders-golive-decisions`, `ses-reminders-prod-recipients`, `email-utf8-charset-both-transports`
- **Local validation recipe** (zero real sends — MailHog catches all):
  ```sh
  docker compose up -d                                  # DynamoDB Local :8000 + MailHog :1025/:8025
  bin/xtask import tournaments/fwc26.json && bin/xtask seed
  XPOOL_NOW=2026-06-11T18:30:00Z MAIL_TRANSPORT=smtp bin/xtask send-reminders --mode last-call
  XPOOL_NOW=2026-06-11T07:00:00Z MAIL_TRANSPORT=smtp bin/xtask send-reminders --mode digest
  # inspect: http://localhost:8025  (re-run in the same window => dedup; use a fresh XPOOL_TABLE/window to resend)
  ```
- Worktree `.claude/worktrees/ses-reminders-phase-a` (branch `feat/ses-reminders-phase-a`) still exists — now fully merged into master, so it's safe to `git worktree remove` + `git branch -d` whenever, or reuse for Phase B.
