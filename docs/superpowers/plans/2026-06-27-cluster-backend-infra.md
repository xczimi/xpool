# cluster/backend-infra Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Ship the Wave-1 `cluster/backend-infra` work: SES-backed deadline-reminder emails (an admin on-demand mutation + two automated EventBridge triggers) and a `bin/local-dev --fresh` flag that loads the cached prod snapshot into the current branch's table.

**Architecture:** A new pure-and-I/O-light `crates/mail` crate owns email: a `MailSender` trait (SES in prod via `aws-sdk-sesv2`, SMTP→MailHog locally via `lettre`, plus a `CapturingSender`/`NullSender` for tests), pure recipient-selection + window + dedup-key functions, bilingual EN/HU templates, and a `sweep` orchestrator that resolves pending players **globally** from `storage::Repository`, dedups via new reminder-marker rows, and sends. **Predictions are per-player and global** — a player has one prediction set for the whole tournament and pools are only competition groupings, so reminders carry no pool dimension (targeting and dedup are keyed by person, never by pool). Two trigger entrypoints invoke the same per-player sweep: an admin GraphQL mutation (on-demand, for dev testing) and a scheduled Lambda (`crates/api/src/bin/reminder.rs`) driven by two EventBridge schedules — an hourly last-call rule and a daily LA-midnight matchday-digest schedule. The clock is always injected (`now` param), never `Utc::now()` inside logic, so every path is deterministically testable. `bin/local-dev --fresh` reuses the existing snapshot tooling, targeting `xpool-<branch>` directly.

**Tech Stack:** Rust workspace (axum + async-graphql, aws-sdk-sesv2, aws-config, lettre, chrono, chrono-tz), DynamoDB single-table storage, Terraform (terraform-aws-modules/lambda, EventBridge Rules + EventBridge Scheduler), bash `bin/` tooling, MailHog for local mail capture.

> **Build status: Phase A ready to build; Phase B gated.** Grilled and revised
> 2026-06-27 — see **"## Revisions (post-grill 2026-06-27)"** immediately below;
> those entries OVERRIDE the task bodies. Phase A (the mail crate, storage
> markers, admin mutation, xtask runner, MailHog) is execution-ready and merges
> with zero unattended sends. Phase B (the scheduled Lambda + EventBridge
> Terraform) is written but must NOT be `apply`-ed until the activation gate is met.

---

## Revisions (post-grill 2026-06-27) — READ FIRST; these override the tasks below

A grilling pass validated the two assumptions the original draft never checked,
and replaced the most aggressive defaults. Apply these on top of the task bodies.

**Verified facts (were unstated assumptions):**
- **Recipients exist.** ~41 real persons have a non-null `Identity.verified_email`
  in prod (22 google + 20 email providers), inferred from the anonymized
  `snapshots/prod-snapshot.json` — `anonymize_emails` rewrites addresses to
  `<nick>@dev.invalid` but **preserves presence/null-ness**, so 42/42 non-null in
  the snapshot ⇒ 42/42 real emails in prod.
- **SES production access is confirmed** for the `xczimi.com` sending identity
  (out of sandbox — can send to arbitrary recipients).

**R1 — Phasing + activation gate.**
- **Phase A (merge now, NO unattended sends):** Tasks 1–8 plus the *xtask runner*
  half of Task 9 (the `send-reminders` subcommand). This yields the mail crate,
  reminder-dedup markers, the admin `sendDeadlineReminders` mutation, the local
  MailHog path, and the local xtask runner.
- **Phase B (write now, DO NOT apply):** the scheduled Lambda entrypoint
  (`crates/api/src/bin/reminder.rs`, the rest of Task 9) and all of Task 10
  (`infrastructure/reminder.tf`, `bin/deploy-reminder`).
- **Gate:** apply Phase B only after the admin mutation has been exercised
  against prod and real deliveries/bounces observed.
- **Operational note:** running the admin mutation against prod **writes real
  dedup markers**. Re-testing the same window therefore needs a fresh window
  (`X-Dev-Now`) or the markers cleared — otherwise the cron later sees them and
  (correctly) skips.

**R2 — Last-call window becomes slot + slack (replaces the 1-hour window).**
The `last_call_due` *body* is unchanged; only the lead constant + its doc change
(Task 3, Step 3):

```rust
/// Last-call lead = trigger interval (30 min) + jitter slack (10 min) = 40 min.
/// Each 30-min tick sends for deadlines in `(now, now + 40min]`; consecutive
/// windows overlap ~10 min to absorb EventBridge jitter, and the per-(person,
/// group) dedup marker stops the overlap double-sending. The window is
/// continuous, so a deadline's minute-of-hour is irrelevant — `:30` kickoffs are
/// covered without any tick-phase alignment.
pub const LAST_CALL_LEAD: Duration = Duration::minutes(40);
```

**Test ripple — REQUIRED (else the suite fails).** Any test whose `now` sits
41–60 min before its deadline must move to within 40 min:
- Task 3 `last_call_due_only_within_the_final_hour` → rename `..._final_window`;
  new cases for an 18:00 deadline: `17:00` (60m) = false, `17:15` (45m) = false,
  `17:25` (35m) = true, `17:20` (exactly 40m) = true, `18:00`/`18:30` = false.
- Task 3 `groups_due_last_call_…`: change `now` `17:10` → `17:30`.
- Task 7 sweep tests (`last_call_sends_once_and_dedups`): change the two ticks
  `17:10`/`17:40` → `17:30`/`17:50`.
- Task 8 mutation test (`…sends_to_incomplete_member`): change `now` `17:10` → `17:30`.

**R3 — 30-minute ticks.** In Task 10, `reminder_last_call_schedule` default
`rate(1 hour)` → `rate(30 minutes)`. (Fixture: 99/104 FWC26 kickoffs are `:00`,
5 are `:30`; slot+slack + 30-min ticks cover all and keep "last call" ≤40 min out.)

**R4 — Manual opt-out (no SES→SNS infra).** Task 4: append a manual opt-out line
to BOTH bodies (last-call + digest), bilingual — e.g. EN "To stop these
reminders, just reply to this email." / HU "Ha nem kérsz több emlékeztetőt,
válaszolj erre az emailre." Add a template-test assertion for it. Plus an
admin-side exclude: simplest v1 form is an env/config list of `person_id`s
skipped inside `pending_players`/the digest loop (a stored per-person preference
can come later).

**R5 — Digest timezone, documented.** Task 10
`aws_scheduler_schedule.reminder_digest`: keep `America/Los_Angeles`, but add a
comment — midnight LA sits a few hours before the earliest NA kickoff, so the
digest always lands before that day's deadlines regardless of recipient TZ.

**R6 — No bounce/complaint plumbing in v1.** SES→SNS bounce/complaint handling
stays future work (addresses are real+verified; audience ~41). Revisit if anyone
complains or bounces appear.

---

## Design decisions (read before starting)

1. **Mail library — `aws-sdk-sesv2` (prod) + `lettre` SMTP (local), trait-selected.** The API Lambda's IAM role already grants `ses:SendEmail`/`ses:SendRawEmail` (`infrastructure/lambda.tf:82-86`), so the SES *API* path needs **zero new credentials**. SES SMTP would require minting/storing IAM SMTP credentials (new infra) — rejected. Locally we capture mail in MailHog (`docker-compose.yml`, SMTP `:1025`) via `lettre`. Selection mirrors the existing `ReportedResultSource` env-pick pattern in `crates/api/src/lib.rs` (`StubLiveSource` / `SportsDbSource` / `NullSource`). Tests inject a `CapturingSender` directly — never the network.

2. **Scheduled checker — a second Lambda entrypoint in `crates/api`, not a new lambda crate.** `crates/api/src/bin/reminder.rs` (gated `required-features = ["lambda"]`) wraps `lambda_runtime::service_fn` and calls the shared `mail::sweep` orchestrator, reusing the existing repo/domain wiring. The EventBridge schedules pass a `{"mode": "..."}` payload so one Lambda serves both triggers. The identical sweep is runnable locally via a new `xtask send-reminders --mode ...` subcommand against MailHog — this is how the scheduled path is tested without deploying. The default `cargo build` skips the bin (required-features), so the local API build is unaffected.

3. **Two automated triggers (no 24h nudge), both per-player and global:**
   - **Last-call (hourly):** ~1h before a group/match deadline, remind every player (globally, across all players — not per pool) still incomplete/unlocked for that group. Deadline = `Tournament::deadline(group_id)` (earliest kickoff in subtree). Dedup key = `(person, group, "1h")` — **no pool**. Driven by an hourly `aws_cloudwatch_event_rule`.
   - **Matchday digest (daily, LA-midnight):** at `00:00 America/Los_Angeles`, one email per **person** listing that LA-calendar-day's groups they are incomplete/unlocked on. Dedup key = `(person, matchday-date)` — **no pool**. Driven by a separate `aws_scheduler_schedule` with `schedule_expression_timezone = "America/Los_Angeles"` (named TZ, DST-aware — never a hard-coded UTC offset).

4. **Recipients / targeting / content / opt-out:**
   - Predictions are **per-player and global**; pools do **not** factor into reminders at all. The recipient set is computed globally over players — "players with incomplete/unlocked predictions for the relevant group/match" — never per pool.
   - **Only send when the player actually has something pending** — never an empty email (the digest skips a person whose day has no pending groups).
   - Send to **all** verified emails of each targeted person (skip persons with none; surface the skipped count to the admin).
   - Email **content** names the pending group(s)/match(es) + the deadline + a deep link to the relevant My Tips section. The deep link is `<origin>/mytips/<group.id>#<group.id>` — the `/mytips/:groupId` route resolves a leaf group id to the right round+group (`web/src/lib/groupRoute.ts`), and `#<group.id>` is the stable scroll anchor from the knockout-subgroup-anchors work (anchor ids use `group.id`). `origin` comes from `XPOOL_PUBLIC_ORIGIN` (default `http://localhost:5173`), the same env the invite links use.
   - **No opt-out** this round; SES bounce/complaint handling is explicitly future work.

5. **Dedup persistence — new reminder-marker rows on the existing single table.** Two new `Repository` methods (`put_reminder_marker` / `reminder_marker_exists`). Keys are pure functions (unit-tested). This is an additive change to the trait; both adapters (`InMemoryRepository`, `DynamoRepository`) implement it.

6. **`bin/local-dev --fresh` is opt-in, non-destructive, local-only.** It loads the newest snapshot under `snapshots/` into `xpool-<branch>` (the table the session actually reads), fixing the master-vs-branch mismatch. It does NOT pull from AWS (that stays `bin/pull-data`) and does NOT blank dev auth/clock.

---

## File structure

**Created:**
- `crates/mail/Cargo.toml` — new crate manifest.
- `crates/mail/src/lib.rs` — module wiring + `now_from_env`.
- `crates/mail/src/sender.rs` — `Email`, `MailSender` trait, `CapturingSender`, `NullSender`.
- `crates/mail/src/select.rs` — pure selection/window/dedup-key functions.
- `crates/mail/src/templates.rs` — bilingual EN/HU last-call + digest renderers + the My Tips deep-link helper.
- `crates/mail/src/transport.rs` — `choose_transport`, `SmtpSender`, `SesSender`, `build_sender_from_env`.
- `crates/mail/src/sweep.rs` — `ReminderMode`, `ReminderSummary`, the two **global** per-player sweeps (no pool dimension).
- `crates/api/src/bin/reminder.rs` — scheduled Lambda entrypoint.
- `infrastructure/reminder.tf` — reminder Lambda + 2 EventBridge schedules + scheduler IAM role + var.
- `bin/deploy-reminder` — build + push the reminder Lambda zip (Peter runs it; not in this plan's scope to execute).

**Modified:**
- `Cargo.toml` (workspace) — add `crates/mail` member + `mail` workspace dep.
- `crates/storage/src/lib.rs` — two `Repository` trait methods.
- `crates/storage/src/memory.rs` — in-memory marker impl.
- `crates/storage/src/dynamo.rs` — DynamoDB marker impl + doc-table row.
- `crates/api/Cargo.toml` — `mail` dep + `[[bin]] reminder`.
- `crates/api/src/lib.rs` — `build_app` gains a `mail` param.
- `crates/api/src/gql/mod.rs` — `build_schema_with_mail`; `build_schema` defaults to `NullSender`.
- `crates/api/src/gql/mutation.rs` — `sendDeadlineReminders(mode)` mutation (no pool arg) + `ReminderReport` + test wiring.
- `crates/api/src/main.rs` — build the real sender; pass to `build_app`.
- `crates/api/tests/common/mod.rs`, `crates/api/tests/cloudfront_auth.rs` — pass `NullSender` to `build_app`.
- `crates/xtask/Cargo.toml`, `crates/xtask/src/main.rs` — `send-reminders` subcommand.
- `bin/lib.sh` — `latest_snapshot` helper.
- `bin/lib.test.sh` — test for `latest_snapshot`.
- `bin/local-dev` — `--fresh` flag.

---

## Task 1: `bin/local-dev --fresh` (feature: local-dev-fresh-snapshot)

Pure-bash, no Rust compile risk — do it first as a self-contained warm-up.

**Files:**
- Modify: `bin/lib.sh`
- Test: `bin/lib.test.sh`
- Modify: `bin/local-dev`

- [ ] **Step 1: Write the failing test for `latest_snapshot`**

Append to `bin/lib.test.sh` immediately before the final summary line (`[ "$fails" -eq 0 ] && ...`):

```bash
# latest_snapshot prints the newest *.json under a snapshots dir (by mtime),
# empty when the dir has no snapshots.
snapdir="$(mktemp -d)"
check "empty dir -> empty" "" "$(latest_snapshot "$snapdir")"
: > "$snapdir/prod-snapshot.json"
sleep 1
: > "$snapdir/dev-snapshot.json"
check "newest wins" "$snapdir/dev-snapshot.json" "$(latest_snapshot "$snapdir")"
check "missing dir -> empty" "" "$(latest_snapshot "$snapdir/nope")"
rm -rf "$snapdir"
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `bash bin/lib.test.sh`
Expected: FAIL — `latest_snapshot: command not found` (or the new checks fail).

- [ ] **Step 3: Add the `latest_snapshot` helper to `bin/lib.sh`**

Append after the `table_for` function (before `port_pids`):

```bash
# Newest *.json snapshot under a directory (by mtime), or empty if none.
# Pure: reads the filesystem only; no network, no mutation.
latest_snapshot() {  # <snapshots-dir>
  local dir="$1"
  [ -d "$dir" ] || return 0
  ls -t "$dir"/*.json 2>/dev/null | head -n1
}
```

- [ ] **Step 4: Run the test to verify it passes**

Run: `bash bin/lib.test.sh`
Expected: `all passed` (exit 0).

- [ ] **Step 5: Wire `--fresh` into `bin/local-dev`**

In `bin/local-dev`, extend the arg parser loop (currently handling `--reseed`):

```bash
# --- parse args: an optional worktree name/dir + flags --------------------
RESEED=""
FRESH=0
ARG=""
for a in "$@"; do
  case "$a" in
    --reseed) RESEED="--reseed" ;;
    --fresh)  FRESH=1 ;;
    *)        ARG="$a" ;;
  esac
done
```

Then, immediately after the infra/data block (after the line that runs
`bin/local-stack`), add the fresh-load step. Replace this existing block:

```bash
# --- 1. infra + data (self-healing) -----------------------------------------
( cd "$TARGET" && XPOOL_TABLE="$TABLE" "$BIN/local-stack" ${RESEED:+--reseed} )
```

with:

```bash
# --- 1. infra + data (self-healing) -----------------------------------------
( cd "$TARGET" && XPOOL_TABLE="$TABLE" "$BIN/local-stack" ${RESEED:+--reseed} )

# --- 1b. optional: load the latest cached snapshot into the branch table -----
# Opt-in (--fresh), non-destructive, local-only. Loads snapshots/<newest>.json
# into THIS branch's table (xpool-<branch>) — fixing the master-vs-branch
# mismatch (pull-data seeds xpool-master). Pulling a fresh snapshot from AWS
# stays a separate explicit step (bin/pull-data); --fresh only reuses on-disk.
if [ "$FRESH" = 1 ]; then
  SNAP="$(latest_snapshot "$PROJECT/snapshots")"
  if [ -z "$SNAP" ]; then
    echo "bin/local-dev: --fresh: no snapshot under $PROJECT/snapshots (run bin/pull-data first)" >&2
    exit 1
  fi
  echo "==> --fresh: loading $SNAP into $TABLE"
  ( cd "$TARGET" \
      && XPOOL_TABLE="$TABLE" DYNAMO_ENDPOINT="${DYNAMO_ENDPOINT:-http://localhost:8000}" \
         cargo run -q -p xtask -- load "$SNAP" )
fi
```

Also update the usage comment block near the top of `bin/local-dev` to add:

```bash
#   bin/local-dev [...] --fresh   load newest snapshots/*.json into the branch table
```

- [ ] **Step 6: Verify the flag is wired (smoke + shellcheck)**

Run: `bash -n bin/local-dev && shellcheck bin/local-dev bin/lib.sh`
Expected: no syntax errors; shellcheck clean (or only pre-existing warnings).

Documented manual verification (Peter / executor runs once, requires docker up):
```
docker compose up -d
bin/pull-data prod            # if no snapshots/*.json exists yet
bin/local-dev --fresh         # loads snapshot into xpool-<branch>
AWS_REGION=us-east-1 AWS_ACCESS_KEY_ID=local AWS_SECRET_ACCESS_KEY=local \
  aws dynamodb scan --table-name "xpool-$(git rev-parse --abbrev-ref HEAD | tr / -)" \
  --select COUNT --endpoint-url http://localhost:8000 --query Count --output text
```
Expected: a count larger than the bare demo seed (prod rows loaded).

- [ ] **Step 7: Commit**

```bash
git add bin/lib.sh bin/lib.test.sh bin/local-dev
git commit -m "feat(bin): local-dev --fresh loads cached snapshot into branch table"
```

---

## Task 2: `crates/mail` crate skeleton + `MailSender` seam

**Files:**
- Modify: `Cargo.toml` (workspace root)
- Create: `crates/mail/Cargo.toml`
- Create: `crates/mail/src/lib.rs`
- Create: `crates/mail/src/sender.rs`

- [ ] **Step 1: Add the crate to the workspace**

In the root `Cargo.toml`, add `"crates/mail"` to `members`:

```toml
members = ["crates/domain", "crates/fwc26", "crates/storage", "crates/api", "crates/xtask", "crates/sportsdb", "crates/mail"]
```

And under `[workspace.dependencies]`, add (next to the existing `domain`/`storage` lines):

```toml
mail = { path = "crates/mail" }
```

- [ ] **Step 2: Create `crates/mail/Cargo.toml`**

```toml
[package]
name = "mail"
version.workspace = true
edition.workspace = true
license.workspace = true

[lib]
name = "mail"
path = "src/lib.rs"

[dependencies]
domain.workspace = true
storage.workspace = true
chrono.workspace = true
chrono-tz = "0.10"
anyhow.workspace = true
async-trait.workspace = true
tokio.workspace = true
tracing.workspace = true
lettre = { version = "0.11", default-features = false, features = ["builder", "tokio1", "tokio1-rustls-tls", "smtp-transport"] }
aws-config = { version = "1", features = ["behavior-version-latest"] }
aws-sdk-sesv2 = "1"

[dev-dependencies]
serde_json = "1"
```

- [ ] **Step 3: Create `crates/mail/src/lib.rs`**

```rust
//! xpool mail crate — email sending + deadline-reminder selection/orchestration.
//!
//! Pure selection/window/dedup logic (`select`), bilingual templates
//! (`templates`), a `MailSender` seam with SES/SMTP/null adapters (`sender`,
//! `transport`), and the `sweep` orchestrator that ties them to
//! `storage::Repository`. The clock is always injected — logic never calls
//! `Utc::now()` itself (see `.specs/TESTING.md` §3.2).

pub mod select;
pub mod sender;
pub mod sweep;
pub mod templates;
pub mod transport;

pub use sender::{CapturingSender, Email, MailSender, NullSender};
pub use sweep::{run_digest_sweep, run_last_call_sweep, ReminderMode, ReminderSummary};
pub use transport::build_sender_from_env;

use chrono::{DateTime, Utc};

/// `now` for non-HTTP entrypoints (the scheduled Lambda, the xtask runner):
/// the `XPOOL_NOW` env override, else the real clock. Mirrors the HTTP clock
/// seam (`api::clock`) for a context with no request headers.
pub fn now_from_env() -> DateTime<Utc> {
    std::env::var("XPOOL_NOW")
        .ok()
        .and_then(|s| {
            DateTime::parse_from_rfc3339(s.trim())
                .ok()
                .map(|d| d.with_timezone(&Utc))
        })
        .unwrap_or_else(Utc::now)
}
```

- [ ] **Step 4: Write the failing test + the `sender.rs` module**

Create `crates/mail/src/sender.rs`:

```rust
//! The `MailSender` seam and its test/dev adapters.

use async_trait::async_trait;
use std::sync::{Arc, Mutex};

/// A composed plaintext email. `to` may carry several verified addresses for a
/// single person — SES/SMTP fan out to all of them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Email {
    pub to: Vec<String>,
    pub subject: String,
    pub body_text: String,
}

/// Abstraction over the mail transport. The production adapters live in
/// `transport`; tests use [`CapturingSender`].
#[async_trait]
pub trait MailSender: Send + Sync {
    async fn send(&self, email: &Email) -> anyhow::Result<()>;
}

/// Discards everything. The default injected into the schema so tests and the
/// e2e stack never touch a real transport, and dev runs without MailHog stay
/// quiet rather than crashing.
pub struct NullSender;

#[async_trait]
impl MailSender for NullSender {
    async fn send(&self, email: &Email) -> anyhow::Result<()> {
        tracing::debug!(to = ?email.to, subject = %email.subject, "NullSender: dropping email");
        Ok(())
    }
}

/// Records every sent email in memory for assertions. Cheap to clone — clones
/// share one buffer.
#[derive(Clone, Default)]
pub struct CapturingSender {
    sent: Arc<Mutex<Vec<Email>>>,
}

impl CapturingSender {
    pub fn new() -> Self {
        Self::default()
    }
    /// A snapshot of every captured email, in send order.
    pub fn sent(&self) -> Vec<Email> {
        self.sent.lock().unwrap().clone()
    }
}

#[async_trait]
impl MailSender for CapturingSender {
    async fn send(&self, email: &Email) -> anyhow::Result<()> {
        self.sent.lock().unwrap().push(email.clone());
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn capturing_sender_records_each_send() {
        let sender = CapturingSender::new();
        let email = Email {
            to: vec!["a@dev.invalid".into()],
            subject: "hi".into(),
            body_text: "body".into(),
        };
        sender.send(&email).await.unwrap();
        sender.send(&email).await.unwrap();
        assert_eq!(sender.sent().len(), 2);
        assert_eq!(sender.sent()[0], email);
    }

    #[tokio::test]
    async fn null_sender_is_a_noop() {
        NullSender
            .send(&Email {
                to: vec![],
                subject: String::new(),
                body_text: String::new(),
            })
            .await
            .unwrap();
    }
}
```

The other modules (`select`, `templates`, `transport`, `sweep`) are declared in
`lib.rs` but created in later tasks. To keep this task compiling on its own,
temporarily comment out their `pub mod` lines and re-exports in `lib.rs`
(re-enable each as you create it). Equivalently, create empty stub files now:

```bash
printf '//! created in Task 3\n' > crates/mail/src/select.rs
printf '//! created in Task 4\n' > crates/mail/src/templates.rs
printf '//! created in Task 5\n' > crates/mail/src/transport.rs
printf '//! created in Task 7\n' > crates/mail/src/sweep.rs
```

and comment the `pub use` lines in `lib.rs` that reference not-yet-written items
(`build_sender_from_env`, `run_*_sweep`, `ReminderMode`, `ReminderSummary`) until
their tasks land. Re-enable them as each task completes.

- [ ] **Step 5: Run the test to verify it passes**

Run: `cargo test -p mail sender`
Expected: PASS (2 tests).

- [ ] **Step 6: Lint + format**

Run: `cargo clippy -p mail -- -D warnings && cargo fmt`
Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add Cargo.toml Cargo.lock crates/mail
git commit -m "feat(mail): new crate with MailSender seam (capturing/null adapters)"
```

---

## Task 3: pure selection, windows & dedup keys (`select.rs`)

These are the must-unit-test pure functions: last-call window, LA-timezone matchday match, recipient targeting, and both dedup-key shapes.

**Files:**
- Create/replace: `crates/mail/src/select.rs`
- Test: inline `#[cfg(test)]` in the same file.

- [ ] **Step 1: Write the failing tests**

Replace `crates/mail/src/select.rs` with the module below (implementation + tests
together; write the tests first mentally, but the file is one unit). Start by
pasting only the `#[cfg(test)]` block and a bare `pub fn` skeleton, run it red,
then fill the bodies — or paste the whole file and run it green. The tests:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};
    use domain::{
        GroupChildren, GroupGame, LockMode, MatchPrediction, Player, Round, SingleGame, TeamSlot,
        Tournament,
    };
    use std::collections::HashMap;

    fn at(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap()
    }

    fn player(id: &str, preds: Vec<MatchPrediction>) -> Player {
        Player {
            id: id.into(),
            person_id: format!("person-{id}"),
            nick: id.into(),
            full_name: id.into(),
            referrer: None,
            is_result_user: false,
            version: 0,
            match_predictions: preds,
            standings_predictions: vec![],
        }
    }

    fn pred(game_id: &str, locked: bool) -> MatchPrediction {
        MatchPrediction {
            game_id: game_id.into(),
            home_score: 1,
            away_score: 0,
            locked,
        }
    }

    // ── last_call_due: a 1h window, hourly tick catches it exactly once ──────
    #[test]
    fn last_call_due_only_within_the_final_hour() {
        let deadline = at(2026, 6, 20, 18, 0);
        assert!(!last_call_due(at(2026, 6, 20, 16, 0), deadline)); // 2h out
        assert!(last_call_due(at(2026, 6, 20, 17, 5), deadline)); // ~55m out
        assert!(last_call_due(at(2026, 6, 20, 17, 0), deadline)); // exactly 1h out
        assert!(!last_call_due(at(2026, 6, 20, 18, 0), deadline)); // at deadline (strict <)
        assert!(!last_call_due(at(2026, 6, 20, 18, 30), deadline)); // past
    }

    // ── matchday digest: LA-day match, DST-aware (June = PDT = UTC-7) ────────
    #[test]
    fn matchday_uses_la_calendar_day_not_utc() {
        // 2026-06-21 05:00 UTC == 2026-06-20 22:00 America/Los_Angeles (PDT).
        let deadline = at(2026, 6, 21, 5, 0);
        // Digest tick at LA-midnight 2026-06-20 (== 2026-06-20 07:00 UTC).
        let tick = at(2026, 6, 20, 7, 0);
        assert!(is_matchday_group(deadline, tick));
        // A deadline on the next LA day must NOT match this tick.
        let next_day = at(2026, 6, 22, 5, 0); // 2026-06-21 22:00 LA
        assert!(!is_matchday_group(next_day, tick));
    }

    #[test]
    fn la_date_of_tick() {
        // 2026-06-20 07:00 UTC is 2026-06-20 00:00 LA.
        assert_eq!(la_date(at(2026, 6, 20, 7, 0)).to_string(), "2026-06-20");
    }

    // ── needs_reminder: missing OR unlocked => true ─────────────────────────
    #[test]
    fn needs_reminder_truth_table() {
        let game_ids = vec!["M1".to_string(), "M2".to_string()];
        // both locked -> no reminder
        assert!(!needs_reminder(
            &player("a", vec![pred("M1", true), pred("M2", true)]),
            &game_ids
        ));
        // one unlocked -> reminder
        assert!(needs_reminder(
            &player("b", vec![pred("M1", true), pred("M2", false)]),
            &game_ids
        ));
        // one missing -> reminder
        assert!(needs_reminder(
            &player("c", vec![pred("M1", true)]),
            &game_ids
        ));
        // none -> reminder
        assert!(needs_reminder(&player("d", vec![]), &game_ids));
    }

    fn leaf_group(id: &str, kickoff: chrono::DateTime<Utc>) -> (Tournament, String) {
        let game = SingleGame {
            id: format!("{id}-g"),
            kickoff,
            venue: None,
            group_id: id.into(),
            home: TeamSlot { team_id: Some("X".into()), description: "x".into() },
            away: TeamSlot { team_id: Some("Y".into()), description: "y".into() },
            external_id: None,
        };
        let group = GroupGame {
            id: id.into(),
            name: format!("Group {id}"),
            parent: None,
            round: Round::GroupStage,
            lock_mode: LockMode::LockTogether,
            carries_standings: true,
            children: GroupChildren::Games(vec![game.id.clone()]),
        };
        let t = Tournament {
            root: id.into(),
            groups: HashMap::from([(id.to_string(), group)]),
            games: HashMap::from([(game.id.clone(), game)]),
            teams: HashMap::new(),
        };
        (t, id.to_string())
    }

    #[test]
    fn groups_due_last_call_picks_leaf_groups_in_the_window() {
        let (t, gid) = leaf_group("A", at(2026, 6, 20, 18, 0));
        let due = groups_due_last_call(&t, at(2026, 6, 20, 17, 10));
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].group_id, gid);
        // Outside the window -> nothing.
        assert!(groups_due_last_call(&t, at(2026, 6, 20, 12, 0)).is_empty());
    }

    #[test]
    fn matchday_groups_picks_groups_on_the_la_day() {
        let (t, gid) = leaf_group("A", at(2026, 6, 21, 5, 0)); // 2026-06-20 LA
        let due = matchday_groups(&t, at(2026, 6, 20, 7, 0)); // LA-midnight 06-20
        assert_eq!(due.len(), 1);
        assert_eq!(due[0].group_id, gid);
        assert!(matchday_groups(&t, at(2026, 6, 21, 7, 0)).is_empty()); // wrong day
    }

    #[test]
    fn pending_players_excludes_locked_and_result_user() {
        // Targeting is GLOBAL over all players — no pool dimension.
        let game_ids = vec!["A-g".to_string()];
        let needs = player("needs", vec![]); // no prediction -> pending
        let done = player("done", vec![pred("A-g", true)]); // locked -> not pending
        let mut ru = player("ru", vec![]);
        ru.is_result_user = true; // result user -> excluded
        let all = vec![needs, done, ru];
        let got: Vec<&str> = pending_players(&all, &game_ids).iter().map(|p| p.id.as_str()).collect();
        assert_eq!(got, vec!["needs"]);
    }

    #[test]
    fn dedup_keys_are_stable_and_distinct_with_no_pool() {
        // Per-player keys — pool plays no part.
        assert_eq!(dedup_key_last_call("person-x", "A"), "person-x|A|1h");
        let d = chrono::NaiveDate::from_ymd_opt(2026, 6, 20).unwrap();
        assert_eq!(dedup_key_digest("person-x", d), "person-x|2026-06-20");
        assert_ne!(dedup_key_last_call("person-x", "A"), dedup_key_digest("person-x", d));
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p mail select`
Expected: FAIL — functions not defined.

- [ ] **Step 3: Write the implementation (prepend above the test module)**

```rust
//! Pure deadline-reminder selection, windows, and dedup keys. No I/O, no clock
//! reads — `now` is always passed in (`.specs/TESTING.md` §3.2).

use chrono::{DateTime, Duration, NaiveDate, Utc};
use chrono_tz::America::Los_Angeles;
use domain::{GameId, GroupChildren, GroupId, Player, Tournament};

/// The last-call lead time. The hourly EventBridge tick + this 1h window means
/// each deadline is caught by exactly one tick (the one 0–60 min before it).
pub const LAST_CALL_LEAD: Duration = Duration::hours(1);

/// A group/match whose deadline makes it a reminder candidate, with its
/// computed deadline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DueGroup {
    pub group_id: GroupId,
    pub deadline: DateTime<Utc>,
}

/// True when `deadline` is within the last hour before it (and not yet passed).
/// Strict `<` at the deadline mirrors the submit gate (`API.md` Issue 27).
pub fn last_call_due(now: DateTime<Utc>, deadline: DateTime<Utc>) -> bool {
    now < deadline && deadline <= now + LAST_CALL_LEAD
}

/// The America/Los_Angeles calendar date of an instant (DST-aware via chrono-tz).
pub fn la_date(now: DateTime<Utc>) -> NaiveDate {
    now.with_timezone(&Los_Angeles).date_naive()
}

/// True when `deadline` falls on the same LA calendar day as `now`.
pub fn is_matchday_group(deadline: DateTime<Utc>, now: DateTime<Utc>) -> bool {
    la_date(deadline) == la_date(now)
}

/// Leaf groups (those that directly hold games) — the lockable units. Parent
/// nodes share a child's earliest kickoff, so restricting to leaves avoids
/// double-counting. Group-stage groups and one-match knockout groups are both
/// leaves, so the treatment is uniform.
fn leaf_groups(t: &Tournament) -> impl Iterator<Item = (&GroupId, DateTime<Utc>)> {
    t.groups.values().filter_map(move |g| {
        if matches!(g.children, GroupChildren::Games(_)) {
            t.deadline(&g.id).map(|d| (&g.id, d))
        } else {
            None
        }
    })
}

/// Leaf groups whose deadline is within the last-call window at `now`.
pub fn groups_due_last_call(t: &Tournament, now: DateTime<Utc>) -> Vec<DueGroup> {
    let mut out: Vec<DueGroup> = leaf_groups(t)
        .filter(|(_, d)| last_call_due(now, *d))
        .map(|(id, d)| DueGroup { group_id: id.clone(), deadline: d })
        .collect();
    out.sort_by(|a, b| a.group_id.cmp(&b.group_id));
    out
}

/// Leaf groups whose deadline falls on the LA calendar day of `now`.
pub fn matchday_groups(t: &Tournament, now: DateTime<Utc>) -> Vec<DueGroup> {
    let mut out: Vec<DueGroup> = leaf_groups(t)
        .filter(|(_, d)| is_matchday_group(*d, now))
        .map(|(id, d)| DueGroup { group_id: id.clone(), deadline: d })
        .collect();
    out.sort_by(|a, b| a.group_id.cmp(&b.group_id));
    out
}

/// A player needs a reminder for a group when any of its games lacks a *locked*
/// prediction — i.e. missing OR unlocked (incomplete). Within a reminder window
/// `now < deadline`, so effective-lock equals the stored `locked` flag.
pub fn needs_reminder(player: &Player, game_ids: &[GameId]) -> bool {
    game_ids.iter().any(|gid| match player.match_prediction(gid) {
        None => true,
        Some(mp) => !mp.locked,
    })
}

/// Players (globally — predictions are per-player, pools don't matter) who
/// should be reminded for a group's games: not the result user, and
/// `needs_reminder`. Returns references into the input slice, in input order.
pub fn pending_players<'a>(players: &'a [Player], game_ids: &[GameId]) -> Vec<&'a Player> {
    players
        .iter()
        .filter(|p| !p.is_result_user && needs_reminder(p, game_ids))
        .collect()
}

/// Dedup key for the hourly last-call nudge: one per (person, group). No pool —
/// predictions are per-player and global.
pub fn dedup_key_last_call(person_id: &str, group_id: &str) -> String {
    format!("{person_id}|{group_id}|1h")
}

/// Dedup key for the daily matchday digest: one per (person, LA-day). No pool.
pub fn dedup_key_digest(person_id: &str, day: NaiveDate) -> String {
    format!("{person_id}|{day}")
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p mail select`
Expected: PASS (all selection tests).

- [ ] **Step 5: Lint + format**

Run: `cargo clippy -p mail -- -D warnings && cargo fmt`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add crates/mail/src/select.rs
git commit -m "feat(mail): pure last-call + LA-matchday selection and dedup keys"
```

---

## Task 4: bilingual EN/HU templates (`templates.rs`)

**Files:**
- Create/replace: `crates/mail/src/templates.rs`

- [ ] **Step 1: Write the failing tests**

Paste this test module (it pins the bilingual content + interpolation):

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Utc};

    #[test]
    fn mytips_link_targets_the_group_route_and_anchor() {
        // /mytips/<group.id>#<group.id> — leaf group id resolves round+group,
        // the hash is the stable anchor (knockout-subgroup-anchors).
        assert_eq!(
            mytips_link("https://pool.xczimi.com", "M76"),
            "https://pool.xczimi.com/mytips/M76#M76"
        );
    }

    #[test]
    fn last_call_is_bilingual_with_deadline_and_deep_link() {
        let r = render_last_call(&LastCallContext {
            group_name: "Group A".into(),
            group_id: "A".into(),
            deadline: Utc.with_ymd_and_hms(2026, 6, 20, 18, 0, 0).unwrap(),
            origin: "https://pool.xczimi.com".into(),
        });
        assert!(r.subject.contains("Group A"));
        assert!(r.body_text.contains("deadline")); // EN
        assert!(r.body_text.contains("határidő")); // HU
        assert!(r.body_text.contains("2026-06-20 18:00 UTC"));
        assert!(r.body_text.contains("https://pool.xczimi.com/mytips/A#A")); // deep link
    }

    #[test]
    fn digest_lists_every_group_in_both_languages_with_links() {
        let r = render_digest(&DigestContext {
            day: chrono::NaiveDate::from_ymd_opt(2026, 6, 20).unwrap(),
            origin: "https://pool.xczimi.com".into(),
            groups: vec![
                DigestItem {
                    group_name: "Group A".into(),
                    group_id: "A".into(),
                    deadline: Utc.with_ymd_and_hms(2026, 6, 20, 18, 0, 0).unwrap(),
                },
                DigestItem {
                    group_name: "Group B".into(),
                    group_id: "B".into(),
                    deadline: Utc.with_ymd_and_hms(2026, 6, 20, 21, 0, 0).unwrap(),
                },
            ],
        });
        assert!(r.subject.contains("2026-06-20"));
        assert!(r.body_text.contains("Group A"));
        assert!(r.body_text.contains("Group B"));
        assert!(r.body_text.contains("/mytips/A#A"));
        assert!(r.body_text.contains("/mytips/B#B"));
        assert!(r.body_text.contains("Today's matches")); // EN
        assert!(r.body_text.contains("Mai meccsek")); // HU
    }
}
```

- [ ] **Step 2: Run the tests to verify they fail**

Run: `cargo test -p mail templates`
Expected: FAIL — items not defined.

- [ ] **Step 3: Write the implementation (prepend above the tests)**

```rust
//! Bilingual (EN + HU) reminder email templates. No per-person language
//! preference exists yet, so every email carries both languages (minimal this
//! round). Wording tracks `.specs/LEGACY_I18N.md`.

use chrono::{DateTime, NaiveDate, Utc};

/// A rendered email: subject + plaintext body.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RenderedReminder {
    pub subject: String,
    pub body_text: String,
}

/// Context for the hourly last-call nudge. No pool — predictions are per-player.
pub struct LastCallContext {
    pub group_name: String,
    /// The leaf group/match id — used for the My Tips deep link + anchor.
    pub group_id: String,
    pub deadline: DateTime<Utc>,
    /// SPA origin for absolute deep links (`XPOOL_PUBLIC_ORIGIN`).
    pub origin: String,
}

/// One line of the daily digest.
pub struct DigestItem {
    pub group_name: String,
    pub group_id: String,
    pub deadline: DateTime<Utc>,
}

/// Context for the daily matchday digest. No pool.
pub struct DigestContext {
    pub day: NaiveDate,
    pub origin: String,
    pub groups: Vec<DigestItem>,
}

fn fmt_deadline(d: DateTime<Utc>) -> String {
    d.format("%Y-%m-%d %H:%M UTC").to_string()
}

/// Deep link into the My Tips page for a group. The `/mytips/:groupId` route
/// resolves a leaf group id to the right round+group (`web/src/lib/groupRoute.ts`);
/// `#<group.id>` is the stable scroll anchor (knockout-subgroup-anchors).
pub fn mytips_link(origin: &str, group_id: &str) -> String {
    format!("{origin}/mytips/{group_id}#{group_id}")
}

/// The last-call (≈1h before deadline) email.
pub fn render_last_call(ctx: &LastCallContext) -> RenderedReminder {
    let when = fmt_deadline(ctx.deadline);
    let link = mytips_link(&ctx.origin, &ctx.group_id);
    let subject = format!(
        "Last call: {group} predictions close soon / Utolsó hívás: {group}",
        group = ctx.group_name
    );
    let body_text = format!(
        "EN\n\
         The prediction deadline for {group} is at {when}. \
         You still have unlocked or missing predictions — finish them here:\n\
         {link}\n\
         \n\
         HU\n\
         A(z) {group} tippelési határidő: {when}. \
         Még van zárolatlan vagy hiányzó tipped — itt fejezd be:\n\
         {link}\n",
        group = ctx.group_name,
        when = when,
        link = link,
    );
    RenderedReminder { subject, body_text }
}

/// The daily matchday digest email.
pub fn render_digest(ctx: &DigestContext) -> RenderedReminder {
    let subject = format!(
        "Today's matches ({day}) — predictions to finish / Mai meccsek ({day})",
        day = ctx.day
    );
    let lines: String = ctx
        .groups
        .iter()
        .map(|g| {
            format!(
                "  - {} ({})\n    {}\n",
                g.group_name,
                fmt_deadline(g.deadline),
                mytips_link(&ctx.origin, &g.group_id)
            )
        })
        .collect();
    let body_text = format!(
        "EN\n\
         Today's matches ({day}) you still have unlocked or missing predictions for:\n\
         {lines}\n\
         HU\n\
         Mai meccsek ({day}), amikhez még van zárolatlan vagy hiányzó tipped:\n\
         {lines}",
        day = ctx.day,
        lines = lines,
    );
    RenderedReminder { subject, body_text }
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p mail templates`
Expected: PASS.

- [ ] **Step 5: Lint + format**

Run: `cargo clippy -p mail -- -D warnings && cargo fmt`
Expected: clean.

- [ ] **Step 6: Commit**

```bash
git add crates/mail/src/templates.rs
git commit -m "feat(mail): bilingual EN/HU last-call + digest templates"
```

---

## Task 5: transport adapters (`transport.rs`)

**Files:**
- Create/replace: `crates/mail/src/transport.rs`

The SES/SMTP senders do network I/O and are not unit-tested against the network;
only the pure `choose_transport` selector is tested. They must compile cleanly.

- [ ] **Step 1: Write the failing test for `choose_transport`**

Paste this test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn explicit_transport_wins() {
        assert_eq!(choose_transport(Some("ses"), Some("http://localhost:8000")), TransportKind::Ses);
        assert_eq!(choose_transport(Some("smtp"), None), TransportKind::Smtp);
        assert_eq!(choose_transport(Some("null"), None), TransportKind::Null);
    }

    #[test]
    fn local_dynamo_endpoint_defaults_to_smtp() {
        assert_eq!(choose_transport(None, Some("http://localhost:8000")), TransportKind::Smtp);
    }

    #[test]
    fn no_hints_defaults_to_ses() {
        assert_eq!(choose_transport(None, None), TransportKind::Ses);
        assert_eq!(choose_transport(None, Some("")), TransportKind::Ses);
    }
}
```

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test -p mail transport`
Expected: FAIL — `choose_transport` not defined.

- [ ] **Step 3: Write the implementation (prepend above the tests)**

```rust
//! Concrete `MailSender` adapters and env-based selection.
//!
//! - [`SesSender`] (prod): `aws-sdk-sesv2`, reusing the Lambda role's
//!   `ses:SendEmail` grant — no new credentials.
//! - [`SmtpSender`] (local): `lettre` SMTP, pointed at MailHog (`:1025`).
//! - Selection mirrors `api`'s `ReportedResultSource` env-pick.

use crate::sender::{Email, MailSender};
use anyhow::Context as _;
use async_trait::async_trait;
use aws_sdk_sesv2::types::{Body, Content, Destination, EmailContent, Message as SesMessage};
use lettre::{
    message::Mailbox, AsyncSmtpTransport, AsyncTransport, Message, Tokio1Executor,
};
use std::sync::Arc;

/// Which transport to construct.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TransportKind {
    Smtp,
    Ses,
    Null,
}

/// Pure selector: explicit `MAIL_TRANSPORT` wins; otherwise a local DynamoDB
/// endpoint implies local SMTP (MailHog), else SES.
pub fn choose_transport(mail_transport: Option<&str>, dynamo_endpoint: Option<&str>) -> TransportKind {
    match mail_transport.map(str::trim) {
        Some("ses") => TransportKind::Ses,
        Some("smtp") => TransportKind::Smtp,
        Some("null") => TransportKind::Null,
        _ => {
            let local = dynamo_endpoint.map(str::trim).is_some_and(|e| !e.is_empty());
            if local {
                TransportKind::Smtp
            } else {
                TransportKind::Ses
            }
        }
    }
}

/// The verified `From:` address. SES requires it to be on the verified domain
/// (`var.ses_domain`, `xczimi.com`).
fn from_address() -> String {
    std::env::var("MAIL_FROM").unwrap_or_else(|_| "xpool@xczimi.com".to_owned())
}

/// `lettre` SMTP sender (local MailHog by default; plaintext, no TLS).
pub struct SmtpSender {
    transport: AsyncSmtpTransport<Tokio1Executor>,
    from: String,
}

impl SmtpSender {
    pub fn from_env() -> anyhow::Result<Self> {
        let host = std::env::var("SMTP_HOST").unwrap_or_else(|_| "localhost".to_owned());
        let port: u16 = std::env::var("SMTP_PORT")
            .ok()
            .and_then(|p| p.parse().ok())
            .unwrap_or(1025);
        // builder_dangerous = plaintext (MailHog speaks no TLS).
        let transport = AsyncSmtpTransport::<Tokio1Executor>::builder_dangerous(&host)
            .port(port)
            .build();
        Ok(Self { transport, from: from_address() })
    }
}

#[async_trait]
impl MailSender for SmtpSender {
    async fn send(&self, email: &Email) -> anyhow::Result<()> {
        let from: Mailbox = self.from.parse().context("parsing MAIL_FROM")?;
        let mut builder = Message::builder().from(from).subject(&email.subject);
        for addr in &email.to {
            let mbox: Mailbox = addr.parse().with_context(|| format!("parsing to-address {addr}"))?;
            builder = builder.to(mbox);
        }
        let msg = builder
            .body(email.body_text.clone())
            .context("building SMTP message")?;
        self.transport.send(msg).await.context("SMTP send")?;
        Ok(())
    }
}

/// `aws-sdk-sesv2` sender (prod). Uses the ambient AWS credentials/role.
pub struct SesSender {
    client: aws_sdk_sesv2::Client,
    from: String,
}

impl SesSender {
    pub async fn from_env() -> anyhow::Result<Self> {
        let config = aws_config::load_defaults(aws_config::BehaviorVersion::latest()).await;
        Ok(Self {
            client: aws_sdk_sesv2::Client::new(&config),
            from: from_address(),
        })
    }
}

#[async_trait]
impl MailSender for SesSender {
    async fn send(&self, email: &Email) -> anyhow::Result<()> {
        let dest = Destination::builder()
            .set_to_addresses(Some(email.to.clone()))
            .build();
        let subject = Content::builder()
            .data(&email.subject)
            .build()
            .context("SES subject content")?;
        let text = Content::builder()
            .data(&email.body_text)
            .build()
            .context("SES body content")?;
        let body = Body::builder().text(text).build();
        let msg = SesMessage::builder().subject(subject).body(body).build();
        let content = EmailContent::builder().simple(msg).build();
        self.client
            .send_email()
            .from_email_address(&self.from)
            .destination(dest)
            .content(content)
            .send()
            .await
            .context("SES send_email")?;
        Ok(())
    }
}

/// Build the sender chosen by the environment.
pub async fn build_sender_from_env() -> anyhow::Result<Arc<dyn MailSender>> {
    let mail_transport = std::env::var("MAIL_TRANSPORT").ok();
    let dynamo_endpoint = std::env::var("DYNAMO_ENDPOINT").ok();
    match choose_transport(mail_transport.as_deref(), dynamo_endpoint.as_deref()) {
        TransportKind::Smtp => Ok(Arc::new(SmtpSender::from_env()?)),
        TransportKind::Ses => Ok(Arc::new(SesSender::from_env().await?)),
        TransportKind::Null => Ok(Arc::new(crate::sender::NullSender)),
    }
}
```

- [ ] **Step 4: Run the test + compile the whole crate**

Run: `cargo test -p mail transport`
Expected: PASS (3 tests).
Run: `cargo build -p mail`
Expected: builds clean (SES/SMTP adapters compile).

- [ ] **Step 5: Re-enable the `build_sender_from_env` re-export in `lib.rs`**

Ensure `crates/mail/src/lib.rs` has `pub use transport::build_sender_from_env;`
un-commented (from the Task 2 stub note).

- [ ] **Step 6: Lint + format**

Run: `cargo clippy -p mail -- -D warnings && cargo fmt`
Expected: clean.

- [ ] **Step 7: Commit**

```bash
git add crates/mail/src/transport.rs crates/mail/src/lib.rs Cargo.lock
git commit -m "feat(mail): SES + SMTP transports with env-based selection"
```

---

## Task 6: reminder-marker rows in storage

**Files:**
- Modify: `crates/storage/src/lib.rs` (trait)
- Modify: `crates/storage/src/memory.rs`
- Modify: `crates/storage/src/dynamo.rs`

- [ ] **Step 1: Write the failing test (in-memory)**

Append to `crates/storage/src/memory.rs` (add a `#[cfg(test)]` module at the end
of the file, or extend an existing one):

```rust
#[cfg(test)]
mod reminder_marker_tests {
    use super::*;

    #[tokio::test]
    async fn marker_absent_then_present_and_idempotent() {
        let repo = InMemoryRepository::new();
        let key = "person-x|A|1h"; // per-person key shape (no pool)
        assert!(!repo.reminder_marker_exists(key).await.unwrap());
        repo.put_reminder_marker(key).await.unwrap();
        assert!(repo.reminder_marker_exists(key).await.unwrap());
        // idempotent
        repo.put_reminder_marker(key).await.unwrap();
        assert!(repo.reminder_marker_exists(key).await.unwrap());
        // distinct key unaffected
        assert!(!repo.reminder_marker_exists("other").await.unwrap());
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p storage reminder_marker`
Expected: FAIL — methods not on `Repository`.

- [ ] **Step 3: Add the trait methods**

In `crates/storage/src/lib.rs`, inside the `Repository` trait, after the
`find_identities_by_person` method, add:

```rust
    // ── Reminder dedup markers ───────────────────────────────────────────────
    //
    // Idempotent set of "this reminder was already sent" keys (see
    // `mail::select` for the key shapes). Lets the hourly/daily reminder sweep
    // run repeatedly without re-sending. Keys are opaque strings.

    /// Record a reminder-dedup marker. Idempotent (re-storing is a no-op).
    async fn put_reminder_marker(&self, key: &str) -> anyhow::Result<()>;
    /// Whether a reminder-dedup marker exists.
    async fn reminder_marker_exists(&self, key: &str) -> anyhow::Result<bool>;
```

- [ ] **Step 4: Implement in `InMemoryRepository`**

In `crates/storage/src/memory.rs`, add the field to `Inner` (after `persons`):

```rust
    /// Reminder-dedup marker keys (see `mail::select`).
    reminder_markers: std::collections::HashSet<String>,
```

(The `use std::collections::HashMap;` import is already present; `HashSet` is
referenced fully-qualified above, so no import change is needed.)

Then add the method impls inside `impl Repository for InMemoryRepository` (after
`find_identities_by_person`):

```rust
    async fn put_reminder_marker(&self, key: &str) -> anyhow::Result<()> {
        let mut inner = self.inner.lock().unwrap();
        inner.reminder_markers.insert(key.to_owned());
        Ok(())
    }

    async fn reminder_marker_exists(&self, key: &str) -> anyhow::Result<bool> {
        let inner = self.inner.lock().unwrap();
        Ok(inner.reminder_markers.contains(key))
    }
```

- [ ] **Step 5: Implement in `DynamoRepository`**

In `crates/storage/src/dynamo.rs`, add to the key-table doc comment a row:

```rust
//! | ReminderMarker | `<t>#REMINDER` | `<key>` | `true` |
```

Then add the method impls inside `impl Repository for DynamoRepository` (place
them next to the other methods; they use the existing `get_item` /
`put_item_simple` helpers and `self.t()`):

```rust
    async fn put_reminder_marker(&self, key: &str) -> anyhow::Result<()> {
        let pk = format!("{}#REMINDER", self.t());
        self.put_item_simple(&pk, key, &true).await
    }

    async fn reminder_marker_exists(&self, key: &str) -> anyhow::Result<bool> {
        let pk = format!("{}#REMINDER", self.t());
        Ok(self.get_item::<bool>(&pk, key).await?.is_some())
    }
```

- [ ] **Step 6: Run the in-memory test to verify it passes**

Run: `cargo test -p storage reminder_marker`
Expected: PASS.

- [ ] **Step 7: Verify the whole workspace still compiles (trait impls complete)**

Run: `cargo build --workspace`
Expected: builds — both adapters implement the new methods. (DynamoDB integration
tests stay gated behind `DYNAMO_TEST=1`; the Dynamo marker path is exercised via
the local stack in Task 9's manual verification.)

- [ ] **Step 8: Lint + format + commit**

```bash
cargo clippy -p storage -- -D warnings && cargo fmt
git add crates/storage/src/lib.rs crates/storage/src/memory.rs crates/storage/src/dynamo.rs
git commit -m "feat(storage): reminder-dedup marker rows (both adapters)"
```

---

## Task 7: sweep orchestration (`sweep.rs`)

Ties pure selection + repo email resolution + dedup markers + an injected
`MailSender`. End-to-end-in-memory tests prove both triggers fire once and dedup.

**Files:**
- Create/replace: `crates/mail/src/sweep.rs`

- [ ] **Step 1: Write the failing tests**

Paste this test module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use crate::sender::CapturingSender;
    use chrono::{TimeZone, Utc};
    use domain::{
        GroupChildren, GroupGame, Identity, LockMode, Person, Player, Round, SingleGame,
        TeamSlot, Tournament,
    };
    use std::collections::HashMap;
    use storage::{InMemoryRepository, Repository};

    fn at(y: i32, mo: u32, d: u32, h: u32, mi: u32) -> chrono::DateTime<Utc> {
        Utc.with_ymd_and_hms(y, mo, d, h, mi, 0).unwrap()
    }

    // One leaf group "A" with one game kicking off at `kickoff`.
    fn tournament(kickoff: chrono::DateTime<Utc>) -> Tournament {
        let game = SingleGame {
            id: "A-g".into(),
            kickoff,
            venue: None,
            group_id: "A".into(),
            home: TeamSlot { team_id: Some("X".into()), description: "x".into() },
            away: TeamSlot { team_id: Some("Y".into()), description: "y".into() },
            external_id: None,
        };
        let group = GroupGame {
            id: "A".into(),
            name: "Group A".into(),
            parent: None,
            round: Round::GroupStage,
            lock_mode: LockMode::LockTogether,
            carries_standings: true,
            children: GroupChildren::Games(vec!["A-g".into()]),
        };
        Tournament {
            root: "A".into(),
            groups: HashMap::from([("A".to_string(), group)]),
            games: HashMap::from([("A-g".to_string(), game)]),
            teams: HashMap::new(),
        }
    }

    fn player(id: &str) -> Player {
        Player {
            id: id.into(),
            person_id: format!("person-{id}"),
            nick: id.into(),
            full_name: id.into(),
            referrer: None,
            is_result_user: false,
            version: 0,
            match_predictions: vec![],
            standings_predictions: vec![],
        }
    }

    // Two players exist GLOBALLY (no pool): alice (has a verified email, no
    // predictions -> a target) and bob (no verified email -> skipped_no_email).
    async fn setup(kickoff: chrono::DateTime<Utc>) -> InMemoryRepository {
        let repo = InMemoryRepository::new();
        repo.put_tournament(&tournament(kickoff)).await.unwrap();

        let alice = player("alice");
        repo.put_player(&alice).await.unwrap();
        repo.put_person(&Person { id: "person-alice".into(), identity_ids: vec!["id-a".into()] })
            .await
            .unwrap();
        repo.put_identity(&Identity {
            id: "id-a".into(),
            provider: "google".into(),
            provider_id: "g-alice".into(),
            person_id: "person-alice".into(),
            verified_email: Some("alice@dev.invalid".into()),
        })
        .await
        .unwrap();

        let bob = player("bob");
        repo.put_player(&bob).await.unwrap();
        repo.put_person(&Person { id: "person-bob".into(), identity_ids: vec!["id-b".into()] })
            .await
            .unwrap();
        repo.put_identity(&Identity {
            id: "id-b".into(),
            provider: "google".into(),
            provider_id: "g-bob".into(),
            person_id: "person-bob".into(),
            verified_email: None,
        })
        .await
        .unwrap();

        repo
    }

    #[tokio::test]
    async fn last_call_sends_once_and_dedups() {
        let kickoff = at(2026, 6, 20, 18, 0);
        let repo = setup(kickoff).await;
        let mail = CapturingSender::new();

        // Tick ~50min before the deadline -> in the last-call window.
        let now = at(2026, 6, 20, 17, 10);
        let s1 = run_last_call_sweep(&repo, &mail, now).await.unwrap();
        assert_eq!(s1.sent, 1, "only alice (has email, incomplete) is sent");
        assert_eq!(s1.skipped_no_email, 1, "bob has no verified email");
        assert_eq!(mail.sent().len(), 1);
        assert_eq!(mail.sent()[0].to, vec!["alice@dev.invalid".to_string()]);
        // The email carries the My Tips deep link for the pending group.
        assert!(mail.sent()[0].body_text.contains("/mytips/A#A"));

        // A second tick in the same window must NOT re-send (dedup).
        let s2 = run_last_call_sweep(&repo, &mail, at(2026, 6, 20, 17, 40)).await.unwrap();
        assert_eq!(s2.sent, 0);
        assert_eq!(s2.deduped, 1);
        assert_eq!(mail.sent().len(), 1, "still just the one email");
    }

    #[tokio::test]
    async fn last_call_silent_outside_window() {
        let repo = setup(at(2026, 6, 20, 18, 0)).await;
        let mail = CapturingSender::new();
        let s = run_last_call_sweep(&repo, &mail, at(2026, 6, 20, 12, 0)).await.unwrap();
        assert_eq!(s.sent, 0);
        assert!(mail.sent().is_empty());
    }

    #[tokio::test]
    async fn digest_sends_once_per_la_day_and_dedups() {
        // Deadline 2026-06-21 05:00 UTC == 2026-06-20 22:00 LA.
        let repo = setup(at(2026, 6, 21, 5, 0)).await;
        let mail = CapturingSender::new();

        // Digest tick at LA-midnight 2026-06-20 (07:00 UTC).
        let now = at(2026, 6, 20, 7, 0);
        let s1 = run_digest_sweep(&repo, &mail, now).await.unwrap();
        assert_eq!(s1.sent, 1);
        assert_eq!(s1.skipped_no_email, 1);
        assert!(mail.sent()[0].subject.contains("2026-06-20"));

        // Same LA day, later tick -> deduped.
        let s2 = run_digest_sweep(&repo, &mail, at(2026, 6, 20, 7, 30)).await.unwrap();
        assert_eq!(s2.sent, 0);
        assert_eq!(s2.deduped, 1);
        assert_eq!(mail.sent().len(), 1);
    }
}
```

- [ ] **Step 2: Run to verify it fails**

Run: `cargo test -p mail sweep`
Expected: FAIL — sweep items not defined.

- [ ] **Step 3: Write the implementation (prepend above the tests)**

```rust
//! The reminder sweep: resolve pending players GLOBALLY from the repository,
//! dedup, and send. Predictions are per-player and global — pools do NOT factor
//! in. Two modes — hourly last-call and daily LA-matchday digest. Clock is
//! injected; the SPA origin (for deep links) comes from `XPOOL_PUBLIC_ORIGIN`.

use crate::select::{
    dedup_key_digest, dedup_key_last_call, groups_due_last_call, la_date, matchday_groups,
    needs_reminder, pending_players,
};
use crate::sender::{Email, MailSender};
use crate::templates::{render_digest, render_last_call, DigestContext, DigestItem, LastCallContext};
use chrono::{DateTime, Utc};
use storage::Repository;

/// Which reminder trigger to run. The scheduled Lambda picks this from the
/// EventBridge payload; the admin mutation and xtask pass it explicitly.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReminderMode {
    LastCall,
    Digest,
}

impl ReminderMode {
    /// Parse the EventBridge / CLI string form.
    pub fn parse(s: &str) -> Option<Self> {
        match s.trim().to_ascii_lowercase().as_str() {
            "last_call" | "last-call" | "lastcall" => Some(Self::LastCall),
            "digest" | "matchday" => Some(Self::Digest),
            _ => None,
        }
    }
}

/// Counts surfaced to the admin / logged by the Lambda.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct ReminderSummary {
    /// Persons selected as needing a reminder (before email resolution/dedup).
    pub recipients: usize,
    /// Emails actually sent.
    pub sent: usize,
    /// Persons skipped because they have no verified email.
    pub skipped_no_email: usize,
    /// Persons skipped because a dedup marker already existed.
    pub deduped: usize,
}

impl ReminderSummary {
    fn add(self, o: ReminderSummary) -> ReminderSummary {
        ReminderSummary {
            recipients: self.recipients + o.recipients,
            sent: self.sent + o.sent,
            skipped_no_email: self.skipped_no_email + o.skipped_no_email,
            deduped: self.deduped + o.deduped,
        }
    }
}

/// All verified emails attached to a person (possibly several; possibly none).
async fn verified_emails(repo: &dyn Repository, person_id: &str) -> anyhow::Result<Vec<String>> {
    let ids = repo.find_identities_by_person(person_id).await?;
    Ok(ids.into_iter().filter_map(|i| i.verified_email).collect())
}

/// The SPA origin for absolute deep links (same env as invite links).
fn public_origin() -> String {
    std::env::var("XPOOL_PUBLIC_ORIGIN").unwrap_or_else(|_| "http://localhost:5173".to_owned())
}

/// Hourly last-call sweep, GLOBAL over all players: for each group ~1h from
/// locking, email every incomplete/unlocked player once (dedup by person|group|1h).
pub async fn run_last_call_sweep(
    repo: &dyn Repository,
    mail: &dyn MailSender,
    now: DateTime<Utc>,
) -> anyhow::Result<ReminderSummary> {
    let tournament = repo
        .get_tournament()
        .await?
        .ok_or_else(|| anyhow::anyhow!("no tournament loaded"))?;
    let players = repo.list_players().await?;
    let origin = public_origin();
    let mut summary = ReminderSummary::default();

    for due in groups_due_last_call(&tournament, now) {
        let game_ids: Vec<String> = tournament
            .games_in(&due.group_id)
            .iter()
            .map(|g| g.id.clone())
            .collect();
        let group_name = tournament
            .groups
            .get(&due.group_id)
            .map(|g| g.name.clone())
            .unwrap_or_default();

        for player in pending_players(&players, &game_ids) {
            summary.recipients += 1;
            let key = dedup_key_last_call(&player.person_id, &due.group_id);
            if repo.reminder_marker_exists(&key).await? {
                summary.deduped += 1;
                continue;
            }
            let emails = verified_emails(repo, &player.person_id).await?;
            if emails.is_empty() {
                summary.skipped_no_email += 1;
                continue;
            }
            let rendered = render_last_call(&LastCallContext {
                group_name: group_name.clone(),
                group_id: due.group_id.clone(),
                deadline: due.deadline,
                origin: origin.clone(),
            });
            mail.send(&Email {
                to: emails,
                subject: rendered.subject,
                body_text: rendered.body_text,
            })
            .await?;
            repo.put_reminder_marker(&key).await?;
            summary.sent += 1;
        }
    }
    Ok(summary)
}

/// Daily matchday digest, GLOBAL over all players: one email per person listing
/// the LA-day's groups they are still incomplete on (dedup by person|LA-date).
/// Never sends an empty email — a person with nothing pending is skipped silently.
pub async fn run_digest_sweep(
    repo: &dyn Repository,
    mail: &dyn MailSender,
    now: DateTime<Utc>,
) -> anyhow::Result<ReminderSummary> {
    let tournament = repo
        .get_tournament()
        .await?
        .ok_or_else(|| anyhow::anyhow!("no tournament loaded"))?;
    let players = repo.list_players().await?;
    let origin = public_origin();
    let day = la_date(now);
    let due_groups = matchday_groups(&tournament, now);
    let mut summary = ReminderSummary::default();

    for player in &players {
        if player.is_result_user {
            continue;
        }
        // Collect the day's groups this player is still incomplete on.
        let mut items: Vec<DigestItem> = Vec::new();
        for due in &due_groups {
            let game_ids: Vec<String> = tournament
                .games_in(&due.group_id)
                .iter()
                .map(|g| g.id.clone())
                .collect();
            if needs_reminder(player, &game_ids) {
                let group_name = tournament
                    .groups
                    .get(&due.group_id)
                    .map(|g| g.name.clone())
                    .unwrap_or_default();
                items.push(DigestItem {
                    group_name,
                    group_id: due.group_id.clone(),
                    deadline: due.deadline,
                });
            }
        }
        if items.is_empty() {
            continue; // never an empty email
        }
        summary.recipients += 1;
        let key = dedup_key_digest(&player.person_id, day);
        if repo.reminder_marker_exists(&key).await? {
            summary.deduped += 1;
            continue;
        }
        let emails = verified_emails(repo, &player.person_id).await?;
        if emails.is_empty() {
            summary.skipped_no_email += 1;
            continue;
        }
        let rendered = render_digest(&DigestContext {
            day,
            origin: origin.clone(),
            groups: items,
        });
        mail.send(&Email {
            to: emails,
            subject: rendered.subject,
            body_text: rendered.body_text,
        })
        .await?;
        repo.put_reminder_marker(&key).await?;
        summary.sent += 1;
    }
    Ok(summary)
}
```

- [ ] **Step 4: Run the tests to verify they pass**

Run: `cargo test -p mail sweep`
Expected: PASS (last_call sends-once+dedup, silent-outside-window, digest sends-once+dedup).

- [ ] **Step 5: Re-enable the `lib.rs` re-exports**

Ensure `crates/mail/src/lib.rs` has these un-commented:

```rust
pub use sweep::{run_digest_sweep, run_last_call_sweep, ReminderMode, ReminderSummary};
```

- [ ] **Step 6: Lint + format + full crate test**

Run: `cargo test -p mail && cargo clippy -p mail -- -D warnings && cargo fmt`
Expected: all green.

- [ ] **Step 7: Commit**

```bash
git add crates/mail/src/sweep.rs crates/mail/src/lib.rs
git commit -m "feat(mail): reminder sweep (last-call + digest) with dedup + skip counts"
```

---

## Task 8: admin mutation + schema/app mail wiring

**Files:**
- Modify: `crates/api/Cargo.toml`
- Modify: `crates/api/src/gql/mod.rs`
- Modify: `crates/api/src/lib.rs`
- Modify: `crates/api/src/main.rs`
- Modify: `crates/api/src/gql/mutation.rs`
- Modify: `crates/api/tests/common/mod.rs`
- Modify: `crates/api/tests/cloudfront_auth.rs`

- [ ] **Step 1: Add the `mail` dependency to the api crate**

In `crates/api/Cargo.toml`, under `[dependencies]`, after `storage.workspace = true`:

```toml
mail.workspace = true
```

- [ ] **Step 2: Add `build_schema_with_mail`, default `build_schema` to NullSender**

Replace the `build_schema` function in `crates/api/src/gql/mod.rs` with:

```rust
/// Build the schema, injecting the repository, reported-results source, and a
/// no-op mail sender. The per-request `CurrentPlayer` is added per request in
/// the router. Use [`build_schema_with_mail`] to inject a real/test sender.
pub fn build_schema(
    repo: Arc<dyn Repository>,
    reported: Arc<dyn ReportedResultSource>,
) -> XpoolSchema {
    build_schema_with_mail(repo, reported, Arc::new(mail::NullSender))
}

/// Build the schema with an explicit [`mail::MailSender`] in schema data — the
/// production path (real sender) and the admin-mutation tests (capturing sender).
pub fn build_schema_with_mail(
    repo: Arc<dyn Repository>,
    reported: Arc<dyn ReportedResultSource>,
    mail: Arc<dyn mail::MailSender>,
) -> XpoolSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(repo)
        .data(reported)
        .data(mail)
        .limit_depth(20)
        .finish()
}
```

(The existing `use std::sync::Arc;` in that file already covers `Arc`.)

- [ ] **Step 3: Thread `mail` through `build_app`**

In `crates/api/src/lib.rs`, change `build_app` to accept and forward a mail sender:

```rust
pub fn build_app(
    repo: Arc<dyn Repository>,
    cors: bool,
    cloudfront_secret: Option<String>,
    mail: Arc<dyn mail::MailSender>,
) -> axum::Router {
    use crate::reported::{
        CachingSource, NullSource, ReportedResultSource, SportsDbSource, StubLiveSource,
    };
    let reported: Arc<dyn ReportedResultSource> = if let Some(stub) = StubLiveSource::from_env() {
        Arc::new(stub)
    } else if let Some(client) = sportsdb::SportsDb::from_env() {
        Arc::new(CachingSource::new(SportsDbSource(client)))
    } else {
        Arc::new(NullSource)
    };
    let schema = gql::build_schema_with_mail(repo.clone(), reported, mail);
    router::build_router(schema, repo, cors, cloudfront_secret)
}
```

- [ ] **Step 4: Build the real sender in `main.rs`**

In `crates/api/src/main.rs`, update `app()` to build a sender from the
environment and pass it:

```rust
async fn app() -> anyhow::Result<axum::Router> {
    let repo = DynamoRepository::from_env().await?;
    #[cfg(not(feature = "lambda"))]
    repo.ensure_table().await?;
    let repo: Arc<dyn Repository> = Arc::new(repo);
    let cloudfront_secret = api::cloudfront_auth::read_secret_from_env();
    let mail = mail::build_sender_from_env().await?;
    Ok(api::build_app(repo, true, cloudfront_secret, mail))
}
```

- [ ] **Step 5: Update the two `build_app` test call sites**

In `crates/api/tests/common/mod.rs` (the `build_app` call ~line 295):

```rust
    let app = api::build_app(repo.clone(), false, None, std::sync::Arc::new(mail::NullSender));
```

In `crates/api/tests/cloudfront_auth.rs` (~line 23):

```rust
    api::build_app(repo, false, cloudfront_secret.map(String::from), std::sync::Arc::new(mail::NullSender))
```

(Add `mail` to those test crates' deps if needed — they already compile against
`api`, but `mail::NullSender` requires the `mail` crate. The api crate
re-exports nothing here, so add `mail.workspace = true` under `[dev-dependencies]`
in `crates/api/Cargo.toml`.)

So also add to `crates/api/Cargo.toml` `[dev-dependencies]`:

```toml
mail.workspace = true
```

- [ ] **Step 6: Write the failing mutation test**

Append a test module to `crates/api/src/gql/mutation.rs` (next to the existing
`submit_group_tests`):

```rust
#[cfg(test)]
mod send_reminders_tests {
    use crate::auth::CurrentPlayer;
    use chrono::{TimeZone, Utc};
    use domain::{
        GroupChildren, GroupGame, Identity, LockMode, Person, Player, Round, SingleGame,
        TeamSlot, Tournament,
    };
    use mail::CapturingSender;
    use std::sync::Arc;
    use storage::{InMemoryRepository, Repository};

    struct NoSource;
    #[async_trait::async_trait]
    impl crate::reported::ReportedResultSource for NoSource {
        async fn lookup_events(&self, _ids: &[String]) -> anyhow::Result<Vec<sportsdb::Event>> {
            Ok(vec![])
        }
    }

    fn admin() -> Player {
        Player {
            id: "ru".into(),
            person_id: "person-ru".into(),
            nick: "ru".into(),
            full_name: "ru".into(),
            referrer: None,
            is_result_user: true,
            version: 0,
            match_predictions: vec![],
            standings_predictions: vec![],
        }
    }

    #[tokio::test]
    async fn admin_send_deadline_reminders_sends_to_incomplete_member() {
        let repo = InMemoryRepository::new();

        // One leaf group "A" locking ~50min after `now`.
        let game = SingleGame {
            id: "A-g".into(),
            kickoff: Utc.with_ymd_and_hms(2026, 6, 20, 18, 0, 0).unwrap(),
            venue: None,
            group_id: "A".into(),
            home: TeamSlot { team_id: Some("X".into()), description: "x".into() },
            away: TeamSlot { team_id: Some("Y".into()), description: "y".into() },
            external_id: None,
        };
        let group = GroupGame {
            id: "A".into(),
            name: "Group A".into(),
            parent: None,
            round: Round::GroupStage,
            lock_mode: LockMode::LockTogether,
            carries_standings: true,
            children: GroupChildren::Games(vec!["A-g".into()]),
        };
        repo.put_tournament(&Tournament {
            root: "A".into(),
            groups: std::collections::HashMap::from([("A".to_string(), group)]),
            games: std::collections::HashMap::from([("A-g".to_string(), game)]),
            teams: std::collections::HashMap::new(),
        })
        .await
        .unwrap();

        let alice = Player {
            id: "alice".into(),
            person_id: "person-alice".into(),
            nick: "alice".into(),
            full_name: "alice".into(),
            referrer: None,
            is_result_user: false,
            version: 0,
            match_predictions: vec![],
            standings_predictions: vec![],
        };
        repo.put_player(&alice).await.unwrap();
        repo.put_person(&Person { id: "person-alice".into(), identity_ids: vec!["id-a".into()] })
            .await
            .unwrap();
        repo.put_identity(&Identity {
            id: "id-a".into(),
            provider: "google".into(),
            provider_id: "g-alice".into(),
            person_id: "person-alice".into(),
            verified_email: Some("alice@dev.invalid".into()),
        })
        .await
        .unwrap();

        // No pool needed — the sweep is global over players (per-player predictions).
        let mail = CapturingSender::new();
        let repo_arc: Arc<dyn Repository> = Arc::new(repo);
        let source: Arc<dyn crate::reported::ReportedResultSource> = Arc::new(NoSource);
        let mail_arc: Arc<dyn mail::MailSender> = Arc::new(mail.clone());
        let schema = crate::gql::build_schema_with_mail(repo_arc, source, mail_arc);

        let now = Utc.with_ymd_and_hms(2026, 6, 20, 17, 10, 0).unwrap();
        let req = async_graphql::Request::new(
            r#"mutation { sendDeadlineReminders(mode: LAST_CALL) { sent skippedNoEmail recipients deduped } }"#,
        )
        .data(CurrentPlayer::Player(Box::new(admin())))
        .data(crate::clock::RequestNow(now));
        let resp = schema.execute(req).await;
        assert!(resp.errors.is_empty(), "errors: {:?}", resp.errors);
        assert_eq!(mail.sent().len(), 1);
        assert_eq!(mail.sent()[0].to, vec!["alice@dev.invalid".to_string()]);
    }

    #[tokio::test]
    async fn non_admin_is_rejected() {
        let repo = InMemoryRepository::new();
        let repo_arc: Arc<dyn Repository> = Arc::new(repo);
        let source: Arc<dyn crate::reported::ReportedResultSource> = Arc::new(NoSource);
        let mail_arc: Arc<dyn mail::MailSender> = Arc::new(CapturingSender::new());
        let schema = crate::gql::build_schema_with_mail(repo_arc, source, mail_arc);
        let mut nonadmin = admin();
        nonadmin.is_result_user = false;
        let req = async_graphql::Request::new(
            r#"mutation { sendDeadlineReminders(mode: LAST_CALL) { sent } }"#,
        )
        .data(CurrentPlayer::Player(Box::new(nonadmin)))
        .data(crate::clock::RequestNow(Utc::now()));
        let resp = schema.execute(req).await;
        assert!(!resp.errors.is_empty(), "non-admin must be rejected");
    }
}
```

- [ ] **Step 7: Run to verify it fails**

Run: `cargo test -p api send_reminders`
Expected: FAIL — `sendDeadlineReminders` / `ReminderReport` / `ReminderModeArg` undefined.

- [ ] **Step 8: Add the mutation, the GraphQL summary type, and the mode enum**

In `crates/api/src/gql/mutation.rs`, add near the other `SimpleObject` types
(after the `ClaimResult` definition):

```rust
/// GraphQL mirror of [`mail::ReminderSummary`].
#[derive(SimpleObject)]
pub struct ReminderReport {
    pub recipients: i32,
    pub sent: i32,
    pub skipped_no_email: i32,
    pub deduped: i32,
}

impl From<mail::ReminderSummary> for ReminderReport {
    fn from(s: mail::ReminderSummary) -> Self {
        Self {
            recipients: s.recipients as i32,
            sent: s.sent as i32,
            skipped_no_email: s.skipped_no_email as i32,
            deduped: s.deduped as i32,
        }
    }
}

/// Which reminder trigger the admin mutation runs.
#[derive(Clone, Copy, Debug, async_graphql::Enum, PartialEq, Eq)]
pub enum ReminderModeArg {
    LastCall,
    Digest,
}
```

Add the `SimpleObject`/`Enum` imports to the existing `use async_graphql::...`
line at the top of the file:

```rust
use async_graphql::{Context, Enum, Object, SimpleObject};
```

(`Enum` is the derive used above — if the derive is referenced as
`async_graphql::Enum` inline as written, no import change is strictly required;
keep whichever the compiler accepts.)

Then add the mutation method inside `impl MutationRoot` (after `recompute`):

```rust
    /// Admin: run a deadline-reminder sweep on demand (for dev testing). This is
    /// the SAME per-player, global pending sweep the scheduled Lambda runs — there
    /// is no pool argument (predictions are per-player and global). `mode` selects
    /// the hourly last-call sweep or the daily matchday digest. Uses the request
    /// clock, so a dev can drive it via `X-Dev-Now`. Targets only players with
    /// incomplete/unlocked predictions; all their verified emails are used; persons
    /// with none are counted in `skippedNoEmail`; never sends an empty email.
    async fn send_deadline_reminders(
        &self,
        ctx: &Context<'_>,
        mode: ReminderModeArg,
    ) -> async_graphql::Result<ReminderReport> {
        CurrentPlayer::require_admin(ctx)?;
        let repo = repo(ctx);
        let mail = ctx.data_unchecked::<Arc<dyn mail::MailSender>>();
        let now = now(ctx);
        let summary = match mode {
            ReminderModeArg::LastCall => {
                mail::run_last_call_sweep(repo.as_ref(), mail.as_ref(), now).await
            }
            ReminderModeArg::Digest => {
                mail::run_digest_sweep(repo.as_ref(), mail.as_ref(), now).await
            }
        }
        .map_err(|e| {
            tracing::error!("sendDeadlineReminders failed: {e}");
            async_graphql::Error::new("sending reminders failed; please retry")
        })?;
        Ok(ReminderReport::from(summary))
    }
```

- [ ] **Step 9: Run the mutation tests to verify they pass**

Run: `cargo test -p api send_reminders`
Expected: PASS (sends-to-incomplete + non-admin-rejected).

- [ ] **Step 10: Full api test + lint + format**

Run: `cargo test -p api && cargo clippy -p api -- -D warnings && cargo fmt`
Expected: all green (the `build_schema` default keeps every other test compiling
unchanged).

- [ ] **Step 11: Commit**

```bash
git add crates/api/Cargo.toml crates/api/src/gql/mod.rs crates/api/src/lib.rs \
        crates/api/src/main.rs crates/api/src/gql/mutation.rs \
        crates/api/tests/common/mod.rs crates/api/tests/cloudfront_auth.rs Cargo.lock
git commit -m "feat(api): admin sendDeadlineReminders mutation + mail seam in schema"
```

---

## Task 9: scheduled Lambda entrypoint + xtask runner

**Files:**
- Modify: `crates/api/Cargo.toml`
- Create: `crates/api/src/bin/reminder.rs`
- Modify: `crates/xtask/Cargo.toml`
- Modify: `crates/xtask/src/main.rs`

- [ ] **Step 1: Declare the reminder bin (lambda-only)**

In `crates/api/Cargo.toml`, after the existing `[[bin]] name = "api"` block, add:

```toml
[[bin]]
name = "reminder"
path = "src/bin/reminder.rs"
# Only built for the Lambda artifact; the default `cargo build` skips it so the
# local API build is unaffected.
required-features = ["lambda"]
```

- [ ] **Step 2: Write the Lambda entrypoint**

Create `crates/api/src/bin/reminder.rs`:

```rust
//! Scheduled deadline-reminder Lambda (the reminder heartbeat).
//!
//! Built only under `--features lambda` (see `required-features` in Cargo.toml).
//! Two EventBridge schedules invoke it with a constant payload selecting the
//! mode: `{"mode":"last_call"}` (hourly) or `{"mode":"digest"}` (daily, at
//! 00:00 America/Los_Angeles). Both call the shared `mail::sweep` orchestrator.
//!
//! The clock honours `XPOOL_NOW` (so the path is testable on dev) then the real
//! clock — the same precedence as the HTTP clock seam, minus request headers.

use lambda_runtime::{service_fn, LambdaEvent};
use mail::ReminderMode;
use serde_json::Value;
use std::sync::Arc;
use storage::{DynamoRepository, Repository};

async fn handler(event: LambdaEvent<Value>) -> Result<Value, lambda_runtime::Error> {
    let mode_str = event
        .payload
        .get("mode")
        .and_then(Value::as_str)
        .unwrap_or("last_call");
    let mode = ReminderMode::parse(mode_str)
        .ok_or_else(|| lambda_runtime::Error::from(format!("unknown reminder mode: {mode_str}")))?;

    let repo = DynamoRepository::from_env()
        .await
        .map_err(|e| lambda_runtime::Error::from(e.to_string()))?;
    let repo: Arc<dyn Repository> = Arc::new(repo);
    let mail = mail::build_sender_from_env()
        .await
        .map_err(|e| lambda_runtime::Error::from(e.to_string()))?;
    let now = mail::now_from_env();

    let summary = match mode {
        ReminderMode::LastCall => mail::run_last_call_sweep(repo.as_ref(), mail.as_ref(), now).await,
        ReminderMode::Digest => mail::run_digest_sweep(repo.as_ref(), mail.as_ref(), now).await,
    }
    .map_err(|e| lambda_runtime::Error::from(e.to_string()))?;

    tracing::info!(
        mode = ?mode,
        recipients = summary.recipients,
        sent = summary.sent,
        skipped_no_email = summary.skipped_no_email,
        deduped = summary.deduped,
        "reminder sweep complete"
    );
    Ok(serde_json::json!({
        "mode": mode_str,
        "recipients": summary.recipients,
        "sent": summary.sent,
        "skipped_no_email": summary.skipped_no_email,
        "deduped": summary.deduped,
    }))
}

#[tokio::main]
async fn main() -> Result<(), lambda_runtime::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .json()
        .init();
    lambda_runtime::run(service_fn(handler)).await
}
```

(`lambda_runtime`, `tokio`, `tracing`, `tracing-subscriber`, `serde_json`, and
`mail` are already api dependencies; `lambda_runtime` is behind the `lambda`
feature, which this bin requires.)

- [ ] **Step 3: Verify the default build still ignores the bin, and the lambda build compiles it**

Run: `cargo build -p api`
Expected: builds (reminder bin skipped — required-features off).
Run: `cargo build -p api --features lambda --bin reminder`
Expected: the reminder bin compiles. (This is a host-target compile-check; the
deploy uses `cargo lambda build` for arm64 — see Task 10 / `bin/deploy-reminder`.)

- [ ] **Step 4: Add the `send-reminders` xtask subcommand**

In `crates/xtask/Cargo.toml`, under `[dependencies]`, add:

```toml
mail.workspace = true
```

In `crates/xtask/src/main.rs`, add a variant to the `Command` enum (after
`ReconcileEvents`):

```rust
    /// Run a deadline-reminder sweep against the configured table + mail
    /// transport (local: MailHog SMTP). Honours XPOOL_NOW. This is how the
    /// scheduled Lambda path is exercised locally.
    SendReminders {
        /// `last-call` (hourly) or `digest` (daily matchday).
        #[arg(long, default_value = "last-call")]
        mode: String,
    },
```

And add its arm to the `match cli.command { ... }` block (after the
`Command::ReconcileEvents` arm):

```rust
        Command::SendReminders { mode } => {
            let mode = mail::ReminderMode::parse(&mode)
                .ok_or_else(|| anyhow::anyhow!("unknown mode `{mode}` (use last-call|digest)"))?;
            let mail_sender = mail::build_sender_from_env().await?;
            let now = mail::now_from_env();
            let repo: std::sync::Arc<dyn Repository> = std::sync::Arc::new(repo);
            let summary = match mode {
                mail::ReminderMode::LastCall => {
                    mail::run_last_call_sweep(repo.as_ref(), mail_sender.as_ref(), now).await?
                }
                mail::ReminderMode::Digest => {
                    mail::run_digest_sweep(repo.as_ref(), mail_sender.as_ref(), now).await?
                }
            };
            println!(
                "reminders ({mode:?}): {} recipients, {} sent, {} skipped (no email), {} deduped",
                summary.recipients, summary.sent, summary.skipped_no_email, summary.deduped
            );
        }
```

(`use storage::{DynamoRepository, Repository};` is already imported at the top of
`crates/xtask/src/main.rs`; the `repo` binding from `DynamoRepository::from_env()`
is moved into the `Arc` here, which is fine since this match arm is terminal.)

- [ ] **Step 5: Build + lint the workspace**

Run: `cargo build --workspace && cargo clippy --workspace -- -D warnings && cargo fmt`
Expected: clean.

- [ ] **Step 6: Documented manual verification (local, MailHog)**

```
docker compose up -d                       # DynamoDB Local + MailHog
export DYNAMO_ENDPOINT=http://localhost:8000
export XPOOL_TABLE=xpool-master
cargo run -p xtask -- import tournaments/fwc26.json
cargo run -p xtask -- seed
# Pick a deadline ~50min ahead of a faked now and run the last-call sweep:
XPOOL_NOW=<rfc3339 ~50min before some group's earliest kickoff> \
  cargo run -p xtask -- send-reminders --mode last-call
# Open MailHog UI at http://localhost:8025 and confirm the email(s).
# Re-run the same command -> "0 sent, N deduped" (dedup markers persisted).
```

Expected: first run sends; second run dedups; emails visible in MailHog.

- [ ] **Step 7: Commit**

```bash
git add crates/api/Cargo.toml crates/api/src/bin/reminder.rs \
        crates/xtask/Cargo.toml crates/xtask/src/main.rs Cargo.lock
git commit -m "feat(api,xtask): scheduled reminder Lambda + xtask send-reminders runner"
```

---

## Task 10: Terraform — reminder Lambda + two EventBridge schedules

Provision only; **do NOT apply/deploy** (Peter deploys after review). Verify with
`terraform fmt` + `terraform validate`.

**Files:**
- Create: `infrastructure/reminder.tf`
- Create: `bin/deploy-reminder`

- [ ] **Step 1: Write `infrastructure/reminder.tf`**

```hcl
# Scheduled deadline-reminder Lambda (the reminder heartbeat) + its two
# EventBridge triggers. Code is shipped out-of-band by bin/deploy-reminder
# (like the api Lambda); tofu manages the function shell + schedules only.

variable "reminder_lambda_package_path" {
  description = "Path to the reminder cargo-lambda zip artifact, relative to infrastructure/."
  type        = string
  default     = "../target/lambda/reminder/bootstrap.zip"
}

variable "reminder_last_call_schedule" {
  description = "EventBridge rate/cron for the hourly last-call reminder rule."
  type        = string
  default     = "rate(1 hour)"
}

variable "reminder_digest_schedule" {
  description = "EventBridge Scheduler cron for the daily matchday digest."
  type        = string
  default     = "cron(0 0 * * ? *)"
}

variable "reminder_digest_timezone" {
  description = "Named timezone for the daily digest (DST-aware; never a hard-coded offset)."
  type        = string
  default     = "America/Los_Angeles"
}

variable "mail_from" {
  description = "Verified From: address for reminder emails (must be on var.ses_domain)."
  type        = string
  default     = "xpool@xczimi.com"
}

module "reminder_lambda" {
  source  = "terraform-aws-modules/lambda/aws"
  version = "~> 7.0"

  function_name = "xpool-reminder-${var.environment}"
  description   = "xpool deadline-reminder sweep (EventBridge-driven)."

  handler       = "bootstrap"
  runtime       = "provided.al2023"
  architectures = ["arm64"]

  create_package         = false
  local_existing_package = var.reminder_lambda_package_path

  # Code shipped out-of-band by bin/deploy-reminder; ignore zip hash drift.
  ignore_source_code_hash = true

  # The digest sweep scans every pool; give it more headroom than the api.
  timeout     = 60
  memory_size = 256

  environment_variables = {
    XPOOL_TABLE           = module.dynamodb.dynamodb_table_id
    CURRENT_TOURNAMENT_ID = var.current_tournament_id
    RUST_LOG              = "info"
    # No DYNAMO_ENDPOINT and no MAIL_TRANSPORT -> build_sender_from_env picks SES.
    MAIL_FROM = var.mail_from
  }

  attach_policy_statements = true
  policy_statements = {
    dynamodb = {
      effect = "Allow"
      actions = [
        "dynamodb:GetItem", "dynamodb:PutItem", "dynamodb:UpdateItem",
        "dynamodb:Query", "dynamodb:Scan", "dynamodb:BatchGetItem",
        "dynamodb:DescribeTable",
      ]
      resources = [module.dynamodb.dynamodb_table_arn]
    }
    ses = {
      effect    = "Allow"
      actions   = ["ses:SendEmail", "ses:SendRawEmail"]
      resources = [data.aws_ses_domain_identity.sending.arn]
    }
  }

  cloudwatch_logs_retention_in_days = 14
}

# ── Trigger A: hourly last-call (EventBridge Rules) ──────────────────────────
resource "aws_cloudwatch_event_rule" "reminder_last_call" {
  name                = "xpool-reminder-last-call-${var.environment}"
  description         = "Hourly last-call deadline reminder sweep."
  schedule_expression = var.reminder_last_call_schedule
}

resource "aws_cloudwatch_event_target" "reminder_last_call" {
  rule  = aws_cloudwatch_event_rule.reminder_last_call.name
  arn   = module.reminder_lambda.lambda_function_arn
  input = jsonencode({ mode = "last_call" })
}

resource "aws_lambda_permission" "reminder_last_call" {
  statement_id  = "AllowEventBridgeLastCall"
  action        = "lambda:InvokeFunction"
  function_name = module.reminder_lambda.lambda_function_name
  principal     = "events.amazonaws.com"
  source_arn    = aws_cloudwatch_event_rule.reminder_last_call.arn
}

# ── Trigger B: daily matchday digest (EventBridge Scheduler, LA timezone) ────
resource "aws_iam_role" "reminder_scheduler" {
  name = "xpool-reminder-scheduler-${var.environment}"
  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect    = "Allow"
      Principal = { Service = "scheduler.amazonaws.com" }
      Action    = "sts:AssumeRole"
    }]
  })
}

resource "aws_iam_role_policy" "reminder_scheduler_invoke" {
  name = "invoke-reminder-lambda"
  role = aws_iam_role.reminder_scheduler.id
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect   = "Allow"
      Action   = "lambda:InvokeFunction"
      Resource = module.reminder_lambda.lambda_function_arn
    }]
  })
}

resource "aws_scheduler_schedule" "reminder_digest" {
  name = "xpool-reminder-digest-${var.environment}"

  flexible_time_window {
    mode = "OFF"
  }

  # Named timezone -> DST-aware; midnight LA is PDT (UTC-7) during the tournament.
  schedule_expression          = var.reminder_digest_schedule
  schedule_expression_timezone = var.reminder_digest_timezone

  target {
    arn      = module.reminder_lambda.lambda_function_arn
    role_arn = aws_iam_role.reminder_scheduler.arn
    input    = jsonencode({ mode = "digest" })
  }
}
```

- [ ] **Step 2: Write `bin/deploy-reminder`**

```bash
#!/bin/bash
# xpool reminder Lambda: cross-compile + push to ca-central-1.
#
# Usage: bin/deploy-reminder [dev|prod]   # default: dev
#
# Mirrors bin/deploy-api. Code is decoupled from tofu
# (module.reminder_lambda has ignore_source_code_hash = true); this is the only
# thing that ships new reminder-Lambda code.
set -euo pipefail

ENV="${1:-dev}"
case "$ENV" in
    dev|prod) ;;
    *) echo "Usage: $0 [dev|prod]"; exit 2 ;;
esac

PROJECT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$PROJECT"

export AWS_PROFILE="${AWS_PROFILE:-xczimi}"
export AWS_REGION="${AWS_REGION:-ca-central-1}"

echo "==> cargo lambda build reminder (arm64, --features lambda)"
cargo lambda build -p api --bin reminder --release --arm64 --features lambda --output-format zip

ZIP="$PROJECT/target/lambda/reminder/bootstrap.zip"
if [[ ! -f "$ZIP" ]]; then
    echo "build did not produce $ZIP" >&2
    exit 1
fi

SIZE=$(stat -f%z "$ZIP" 2>/dev/null || stat -c%s "$ZIP")
echo "==> aws lambda update-function-code xpool-reminder-${ENV} ($((SIZE / 1024)) KB)"
aws lambda update-function-code \
    --function-name "xpool-reminder-${ENV}" \
    --zip-file "fileb://$ZIP" \
    --no-cli-pager \
    --query '{Version:Version,LastModified:LastModified,CodeSha256:CodeSha256}' \
    --output table
```

Then: `chmod +x bin/deploy-reminder`.

- [ ] **Step 3: Validate the Terraform (no apply)**

Run:
```bash
cd infrastructure && terraform fmt && terraform init -backend=false && terraform validate
```
Expected: `terraform fmt` rewrites nothing new (or only the new file's
formatting); `terraform validate` → `Success! The configuration is valid.`

If `terraform` is unavailable, use `tofu` (the repo uses OpenTofu per
`bin/deploy`): `cd infrastructure && tofu fmt && tofu init -backend=false && tofu validate`.

- [ ] **Step 4: Shellcheck the new script**

Run: `shellcheck bin/deploy-reminder`
Expected: clean.

- [ ] **Step 5: Commit**

```bash
git add infrastructure/reminder.tf bin/deploy-reminder
git commit -m "feat(infra): reminder Lambda + hourly last-call rule + daily LA-midnight digest schedule"
```

---

## Task 11: Final verification + request code review

**Files:** none (verification only).

- [ ] **Step 1: Full workspace build (default features)**

Run: `cargo build --workspace`
Expected: clean.

- [ ] **Step 2: Lambda-feature build of both bins**

Run: `cargo build -p api --features lambda --bins`
Expected: both `api` and `reminder` bins compile under the lambda feature.

- [ ] **Step 3: Full test suite**

Run: `cargo test --workspace`
Expected: all green (DynamoDB integration tests skip without `DYNAMO_TEST=1`).

- [ ] **Step 4: Clippy with warnings as errors**

Run: `cargo clippy --workspace -- -D warnings`
Expected: no warnings.

- [ ] **Step 5: Format check**

Run: `cargo fmt --check`
Expected: no diff.

- [ ] **Step 6: bin tests + shellcheck**

Run: `bash bin/lib.test.sh && shellcheck bin/local-dev bin/lib.sh bin/deploy-reminder`
Expected: `all passed`; shellcheck clean.

- [ ] **Step 7: Terraform validate (no apply)**

Run: `cd infrastructure && terraform fmt -check && terraform validate` (or `tofu`).
Expected: formatted; `Success! The configuration is valid.`

- [ ] **Step 8: Request code review**

REQUIRED SUB-SKILL: Use superpowers:requesting-code-review to verify the work
meets the cluster quality bar before merging. Confirm in the review:
- recipient-selection + dedup-key + both window checks are pure and unit-tested;
- the mail sender is injected/mocked in every test (no network);
- the clock is injected everywhere (`now` param / `XPOOL_NOW`); no `Utc::now()`
  inside reminder logic;
- both EventBridge triggers are present (hourly rule + daily LA-timezone schedule)
  and Terraform validates;
- `bin/local-dev --fresh` loads into `xpool-<branch>`, opt-in and non-destructive.

- [ ] **Step 9: Final commit (if review fixes were needed)**

```bash
git add -A
git commit -m "chore(cluster/backend-infra): address code-review feedback"
```

---

## Self-review notes (author)

- **Spec coverage — ses-deadline-reminders (revised 2026-06-27):**
  - **Per-player, global predictions; no pool dimension** — sweeps iterate
    `list_players()` directly (Task 7); dedup keys are `(person, group, "1h")` and
    `(person, LA-date)` with no pool (Task 3 `dedup_key_last_call`/`dedup_key_digest`);
    targeting is `pending_players` over all players (Task 3).
  - **Admin on-demand mutation** is `sendDeadlineReminders(mode)` — no pool arg —
    running the same global sweep for dev testing (Task 8).
  - Two automated triggers: hourly last-call + daily LA-midnight digest, timezone
    `America/Los_Angeles` via EventBridge Scheduler, no hard-coded offset (Tasks 9, 10).
  - **Only send when pending; never an empty email** — digest skips a person with no
    pending groups (Task 7 `if items.is_empty() { continue }`); last-call only iterates
    `pending_players` (Task 3/7).
  - All-verified-emails recipients + `skipped_no_email` count (Task 7 `verified_emails`).
  - **Email content** names the pending group(s)/match(es) + deadline + a deep link
    `<origin>/mytips/<group.id>#<group.id>` (Task 4 `mytips_link`, tested).
  - Bilingual EN+HU stacked in one body (Task 4); incomplete/unlocked targeting
    (Task 3 `needs_reminder`); dedup markers (Task 6); no opt-out this round (by design);
    testable scheduled path via `XPOOL_NOW` + `xtask send-reminders` (Task 9);
    sender injected/mocked with `CapturingSender` in every unit test (Tasks 2,7,8).
- **Spec coverage — local-dev-fresh-snapshot:** opt-in `--fresh` (Task 1);
  newest cached snapshot under `snapshots/` (Task 1 `latest_snapshot`); loads
  into `xpool-<branch>` (Task 1 uses `$TABLE`); non-destructive/local-only
  (additive `xtask load`, never touches AWS).
- **Type consistency:** `ReminderMode`/`ReminderSummary`/`Email`/`MailSender`/
  `RenderedReminder`/`DueGroup`/`DigestItem`/`DigestContext`/`LastCallContext`/
  `mytips_link`/`pending_players` are defined once and reused; `run_last_call_sweep`
  /`run_digest_sweep` have the same `(repo, mail, now)` signature called from the
  mutation (Task 8), the Lambda (Task 9), and xtask (Task 9); `build_schema_with_mail`,
  `build_app(.., mail)`, and `ctx.data_unchecked::<Arc<dyn mail::MailSender>>()` agree
  on `Arc<dyn mail::MailSender>`; the no-pool dedup-key shapes match between `select.rs`
  and the sweep callers.
- **No placeholders:** every code step contains complete code; commands have
  expected output.
- **Build deferred:** the header records that execution waits on the user's go-ahead.
  The coordinator's design corrections were applied as technical input; they are not
  treated as user approval to start the build.
