# SES deadline reminder emails

Status: deferred build — design settled (2026-06-27 grill); execution-ready plan at docs/superpowers/plans/2026-06-27-cluster-backend-infra.md (awaiting go-ahead)

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

## Resolved decisions (2026-06-27 grill — STATUS: deferred for build, design settled)

> **NOTE:** This cluster is **deferred** from the Wave-1 build (Peter's call at the
> design-review gate) but its design is now fully settled. Build after the page clusters.

**Predictions are PER-PLAYER and GLOBAL — pools do NOT factor into reminders.**
A player has a single prediction set for the tournament; pools are only competition
groupings. So reminders are keyed to the *player*, never the pool. There is **no pool
dimension** in targeting or dedup. **Only ever send when the player actually has
something pending** (no empty emails).

- **Trigger model — BOTH:**
  1. **1h last-call** (automated) — ~1h before a group/match deadline (group deadline =
     earliest kickoff in its subtree; each knockout match is its own one-match group),
     email the player if they're still unpredicted/unlocked for that group/match.
     Dedup by `(person, group, "1h")`. Driven by the hourly EventBridge tick scanning
     for deadlines ~1h out.
  2. **Daily matchday digest** (automated) — fires once daily at **00:00
     America/Los_Angeles** ("midnight PST"; PDT/UTC-7 during the June–July tournament),
     via an EventBridge Scheduler rule with `timezone = America/Los_Angeles`. Emails the
     player about that LA-day's groups/matches they still have pending. Dedup by
     `(person, matchday-date)`.
  3. **Manual admin send** — an admin-only mutation that runs the same per-player pending
     sweep on demand (so the path is testable on dev). Same targeting as auto:
     incomplete/unlocked players only. (The 24h-ahead nudge is dropped.)
- **Recipients:** send to ALL verified emails of each targeted person; persons with no
  verified email are skipped, skipped count surfaced to the admin.
- **Targeting:** only players with INCOMPLETE/UNLOCKED predictions for the relevant
  group/match (not zero-only, not everyone, not pool-scoped).
- **Email content:** name the pending group(s)/match(es), the deadline, and a **deep link
  to the relevant My Tips section** (`/mytips/<round>#<group.id>` — reuses the
  knockout-subgroup-anchors deep-link work). No empty emails.
- **Language:** **bilingual — EN then HU stacked** in one email (no per-person language
  data needed). Templates live with the other i18n copy.
- **Opt-out / bounce:** NONE this round — no unsubscribe; SES bounce/complaint handling
  is future work.
- Cluster: `cluster/backend-infra` (Wave 1 surface, **deferred build**). Touches `crates`
  mail, `infrastructure/*.tf` (EventBridge/Lambda), and adds an admin mutation in the
  GraphQL mutation root.
