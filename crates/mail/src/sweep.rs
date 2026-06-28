//! The reminder sweep: resolve pending players GLOBALLY from the repository,
//! dedup, and send. Predictions are per-player and global — pools do NOT factor
//! in. Two modes — hourly last-call and daily LA-matchday digest. Clock is
//! injected; the SPA origin (for deep links) comes from `XPOOL_PUBLIC_ORIGIN`.

use crate::select::{
    dedup_key_digest, dedup_key_last_call, groups_due_last_call, la_date, matchday_groups,
    needs_reminder, pending_players,
};
use crate::sender::{Email, MailSender};
use crate::templates::{
    render_digest, render_last_call, DigestContext, DigestItem, LastCallContext,
};
use anyhow::Context as _;
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
    /// Reminder candidates counted before email resolution/dedup. **The unit
    /// differs by sweep:** `run_last_call_sweep` counts per (player, due group)
    /// — a player pending in two groups counts twice — whereas
    /// `run_digest_sweep` counts per unique player (one digest per person/day).
    /// Callers combining summaries via `Add` should not read this as a distinct
    /// head count across modes.
    pub recipients: usize,
    /// Emails actually sent.
    pub sent: usize,
    /// Persons skipped because they have no verified email.
    pub skipped_no_email: usize,
    /// Persons skipped because a dedup marker already existed.
    pub deduped: usize,
}

impl std::ops::Add for ReminderSummary {
    type Output = Self;
    fn add(self, o: Self) -> Self {
        Self {
            recipients: self.recipients + o.recipients,
            sent: self.sent + o.sent,
            skipped_no_email: self.skipped_no_email + o.skipped_no_email,
            deduped: self.deduped + o.deduped,
        }
    }
}

/// All verified emails attached to a person (possibly several; possibly none).
async fn verified_emails(repo: &dyn Repository, person_id: &str) -> anyhow::Result<Vec<String>> {
    let ids = repo
        .find_identities_by_person(person_id)
        .await
        .with_context(|| format!("resolving identities for {person_id}"))?;
    Ok(ids.into_iter().filter_map(|i| i.verified_email).collect())
}

/// The SPA origin for absolute deep links (same env as invite links).
fn public_origin() -> String {
    std::env::var("XPOOL_PUBLIC_ORIGIN").unwrap_or_else(|_| "http://localhost:5173".to_owned())
}

/// Hourly last-call sweep, GLOBAL over all players: for each group ~40min from
/// locking, email every incomplete/unlocked player once (dedup by person|group|1h).
/// `summary.recipients` is counted per (player, due group) here.
///
/// Fail-fast: a `mail.send()` (or repo) error aborts the rest of the batch.
/// This is intended for transport-level SES failures — a later retry sweep
/// resumes safely because a marker is written only for recipients already sent.
pub async fn run_last_call_sweep(
    repo: &dyn Repository,
    mail: &dyn MailSender,
    now: DateTime<Utc>,
) -> anyhow::Result<ReminderSummary> {
    let tournament = repo
        .get_tournament()
        .await
        .context("loading tournament")?
        .ok_or_else(|| anyhow::anyhow!("no tournament loaded"))?;
    let players = repo.list_players().await.context("listing players")?;
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
            if repo
                .reminder_marker_exists(&key)
                .await
                .context("checking reminder marker")?
            {
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
            .await
            .context("sending last-call email")?;
            repo.put_reminder_marker(&key)
                .await
                .context("writing reminder marker")?;
            summary.sent += 1;
        }
    }
    Ok(summary)
}

/// Daily matchday digest, GLOBAL over all players: one email per person listing
/// the LA-day's groups they are still incomplete on (dedup by person|LA-date).
/// Never sends an empty email — a person with nothing pending is skipped silently.
/// `summary.recipients` is counted per unique player here (one digest per person).
///
/// Fail-fast: a `mail.send()` (or repo) error aborts the rest of the batch.
/// This is intended for transport-level SES failures — a later retry sweep
/// resumes safely because a marker is written only for recipients already sent.
pub async fn run_digest_sweep(
    repo: &dyn Repository,
    mail: &dyn MailSender,
    now: DateTime<Utc>,
) -> anyhow::Result<ReminderSummary> {
    let tournament = repo
        .get_tournament()
        .await
        .context("loading tournament")?
        .ok_or_else(|| anyhow::anyhow!("no tournament loaded"))?;
    let players = repo.list_players().await.context("listing players")?;
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
        if repo
            .reminder_marker_exists(&key)
            .await
            .context("checking reminder marker")?
        {
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
        .await
        .context("sending digest email")?;
        repo.put_reminder_marker(&key)
            .await
            .context("writing reminder marker")?;
        summary.sent += 1;
    }
    Ok(summary)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::sender::CapturingSender;
    use chrono::{TimeZone, Utc};
    use domain::{
        GroupChildren, GroupGame, Identity, LockMode, Person, Player, Round, SingleGame, TeamSlot,
        Tournament,
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
            home: TeamSlot {
                team_id: Some("X".into()),
                description: "x".into(),
            },
            away: TeamSlot {
                team_id: Some("Y".into()),
                description: "y".into(),
            },
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
        repo.put_person(&Person {
            id: "person-alice".into(),
            identity_ids: vec!["id-a".into()],
        })
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
        repo.put_person(&Person {
            id: "person-bob".into(),
            identity_ids: vec!["id-b".into()],
        })
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

        // Tick 30min before the deadline -> in the 40-min last-call window (R2).
        let now = at(2026, 6, 20, 17, 30);
        let s1 = run_last_call_sweep(&repo, &mail, now).await.unwrap();
        assert_eq!(s1.recipients, 2, "alice + bob both pending for group A");
        assert_eq!(s1.sent, 1, "only alice (has email, incomplete) is sent");
        assert_eq!(s1.skipped_no_email, 1, "bob has no verified email");
        assert_eq!(mail.sent().len(), 1);
        assert_eq!(mail.sent()[0].to, vec!["alice@dev.invalid".to_string()]);
        // The email carries the My Tips deep link for the pending group.
        assert!(mail.sent()[0].body_text.contains("/mytips/A#A"));

        // A second tick in the same window must NOT re-send (dedup).
        let s2 = run_last_call_sweep(&repo, &mail, at(2026, 6, 20, 17, 50))
            .await
            .unwrap();
        assert_eq!(s2.recipients, 2, "both still candidates on the retick");
        assert_eq!(s2.sent, 0);
        assert_eq!(s2.deduped, 1, "alice already has a marker");
        assert_eq!(s2.skipped_no_email, 1, "bob still has no verified email");
        assert_eq!(mail.sent().len(), 1, "still just the one email");
    }

    #[tokio::test]
    async fn last_call_silent_outside_window() {
        let repo = setup(at(2026, 6, 20, 18, 0)).await;
        let mail = CapturingSender::new();
        let s = run_last_call_sweep(&repo, &mail, at(2026, 6, 20, 12, 0))
            .await
            .unwrap();
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
        let s2 = run_digest_sweep(&repo, &mail, at(2026, 6, 20, 7, 30))
            .await
            .unwrap();
        assert_eq!(s2.sent, 0);
        assert_eq!(s2.deduped, 1);
        assert_eq!(mail.sent().len(), 1);
    }

    #[tokio::test]
    async fn last_call_fans_out_to_all_of_a_persons_emails() {
        let repo = setup(at(2026, 6, 20, 18, 0)).await;
        // Give alice a SECOND identity with a different verified email. The
        // sweep must fan out BOTH addresses into one `Email.to`, not send twice.
        repo.put_identity(&Identity {
            id: "id-a2".into(),
            provider: "email".into(),
            provider_id: "e-alice".into(),
            person_id: "person-alice".into(),
            verified_email: Some("alice.alt@dev.invalid".into()),
        })
        .await
        .unwrap();

        let mail = CapturingSender::new();
        let s = run_last_call_sweep(&repo, &mail, at(2026, 6, 20, 17, 30))
            .await
            .unwrap();

        assert_eq!(s.sent, 1, "one email to alice, not one per address");
        assert_eq!(mail.sent().len(), 1, "a single send, not two");
        let mut to = mail.sent()[0].to.clone();
        to.sort(); // identity iteration order is unspecified
        assert_eq!(
            to,
            vec![
                "alice.alt@dev.invalid".to_string(),
                "alice@dev.invalid".to_string()
            ]
        );
    }

    #[tokio::test]
    async fn digest_excludes_the_result_user() {
        // Deadline 2026-06-21 05:00 UTC == 2026-06-20 22:00 LA.
        let repo = setup(at(2026, 6, 21, 5, 0)).await;
        // A result user with a verified email but no predictions: must be
        // skipped entirely (not counted, not sent) by the digest sweep.
        let mut ru = player("ru");
        ru.is_result_user = true;
        repo.put_player(&ru).await.unwrap();
        repo.put_person(&Person {
            id: "person-ru".into(),
            identity_ids: vec!["id-ru".into()],
        })
        .await
        .unwrap();
        repo.put_identity(&Identity {
            id: "id-ru".into(),
            provider: "google".into(),
            provider_id: "g-ru".into(),
            person_id: "person-ru".into(),
            verified_email: Some("ru@dev.invalid".into()),
        })
        .await
        .unwrap();

        let mail = CapturingSender::new();
        let s = run_digest_sweep(&repo, &mail, at(2026, 6, 20, 7, 0))
            .await
            .unwrap();

        // alice + bob are recipients; the result user is excluded (else 3).
        assert_eq!(s.recipients, 2, "result user is not counted as a recipient");
        assert_eq!(s.sent, 1, "only alice has a verified email");
        assert_eq!(s.skipped_no_email, 1, "bob has no verified email");
        assert_eq!(mail.sent().len(), 1);
        assert!(
            mail.sent()
                .iter()
                .all(|e| !e.to.contains(&"ru@dev.invalid".to_string())),
            "the result user must never be emailed"
        );
    }
}
