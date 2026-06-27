# SES deadline reminder emails

Status: needs-triage

## Idea

Email pool participants about upcoming prediction deadlines using AWS SES.
When a group's deadline (earliest kickoff in its subtree) is approaching,
notify the players in that pool who haven't completed/locked their predictions.

## Why this is feasible now

The AWS side is already paved — this is a feature build, not an infra change:

- **SES already provisioned + permitted.** `infrastructure/ses.tf` looks up the
  domain identity (`xczimi.com`, `var.ses_domain`); the API Lambda's IAM role
  already grants `ses:SendEmail` / `ses:SendRawEmail` against it
  (`infrastructure/lambda.tf:82-86`). The deployed API can call SES today.
- **Recipients are enumerable.** `Pool.members` → `get_player` → `get_person`
  → `find_identities_by_person` → `Identity.verified_email`
  (`crates/storage/src/lib.rs`). Emails live on **Identity**, not Player/Person;
  a person can have several verified emails (or none).
- **Deadlines are computable in pure domain.** `Tournament::deadline(group_id)`
  returns the earliest kickoff in a group's subtree
  (`crates/domain/src/model.rs:200`); `effective_locked` / `deadline_passed`
  already exist (`crates/domain/src/scoring.rs:255`).
- **Local mail capture exists.** MailHog (`docker-compose.yml`, SMTP :1025,
  UI :8025) for testing the send path without real SES.

## What's missing (the actual work)

1. **No mail code at all** — no `aws-sdk-ses` / `lettre` dependency, no send
   function. Invites today only generate a shareable link; nothing emails it.
2. **No trigger** — no mutation, endpoint, or scheduled job. Options:
   - admin mutation ("notify pool X") — simplest, manual.
   - **EventBridge schedule → Lambda** for true automated "deadline approaching"
     reminders. The server-authoritative clock (`XPOOL_NOW` / `X-Dev-Now`,
     `crates/api` request `now`) makes a scheduled checker testable.
     - **Concrete first piece (filed separately as an idea):** stand up an
       **hourly EventBridge → Lambda trigger** as the reminder heartbeat. Each
       tick scans for groups whose deadline falls inside the next window and
       enqueues/sends reminders, with dedup so the same reminder isn't re-sent
       every hour. This is the infrastructure half; the send/templating half is
       items 1 & 3 above.
3. **No templating / i18n** — reminders must respect the en/hu i18n
   (`web/src/i18n/strings.ts`, `.specs/LEGACY_I18N.md`).
4. **No spec** — nothing in `.specs/` covers notifications.

## Resolved decisions (2026-06-27 grill)

- **Trigger model — BOTH:** (a) a manual admin-send mutation ("notify pool X"), and
  (b) an automated hourly **EventBridge → Lambda** heartbeat. Peter explicitly wants
  the scheduled path so it can be **tested on dev**.
- **Recipients:** send to all verified emails of each targeted person; persons with
  no verified email are skipped, with the skipped count surfaced to the admin.
- **Targeting:** only players with incomplete/unlocked predictions for the
  soon-to-lock group (not the whole pool).
- **Cadence (2026-06-27, revised by Peter):** TWO triggers (the 24h-ahead nudge is dropped):
  1. **1h last-call** — ~1h before a group/match deadline (the group's deadline = earliest
     kickoff in its subtree; each knockout match is its own one-match group), remind players
     still unpredicted/unlocked for that group/match. Dedup by `(pool, group, person, "1h")`.
     Driven by the hourly EventBridge tick scanning for deadlines ~1h out.
  2. **Daily matchday digest** — fires once daily at **00:00 America/Los_Angeles**
     ("midnight PST"; PDT/UTC-7 during the June–July tournament). Reminds players with
     incomplete/unlocked predictions about that calendar day's matches/groups (deadline
     falling that LA-day). Dedup by `(pool, person, matchday-date)`. Driven by a daily
     EventBridge Scheduler rule with `timezone = America/Los_Angeles`.
- **Recipients (grill):** send to ALL verified emails of each targeted person.
- **Targeting (grill):** only players with INCOMPLETE/UNLOCKED predictions for the
  soon-to-lock group (not zero-only, not everyone).
- **Opt-out / bounce (grill):** NONE this round (small pool) — no unsubscribe system;
  SES bounce/complaint handling noted as future work.
- **i18n:** reminder templates must be EN + HU (`web/src/i18n/strings.ts` /
  `.specs/LEGACY_I18N.md`).
- Cluster: `cluster/backend-infra` (Wave 1). Touches `crates` mail, `infrastructure/*.tf`
  (EventBridge/Lambda), and adds one resolver/mutation in `query.rs`.
