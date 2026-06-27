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
3. **No templating / i18n** — reminders must respect the en/hu i18n
   (`web/src/i18n/strings.ts`, `.specs/LEGACY_I18N.md`).
4. **No spec** — nothing in `.specs/` covers notifications.

## Open design questions (for brainstorming)

- **Trigger model:** manual admin-send vs. automated scheduled reminders (or both)?
- **Recipient selection:** people with multiple verified emails — pick which?
  People with none — skip silently or surface to admin?
- **Targeting:** everyone in the pool, or only players with incomplete/unlocked
  predictions for the soon-to-lock group?
- **Cadence / dedup:** how far ahead to remind, and how to avoid re-sending the
  same reminder on every schedule tick.
- **Unsubscribe / opt-out** and SES bounce/complaint handling.

## Next step

Run `superpowers:brainstorming` on the trigger model + recipient edge cases
before any implementation.
