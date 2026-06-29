//! `clear-verified-email <addr>` — the manual reminder opt-out.
//!
//! Deadline reminders target each person's `Identity.verified_email`; a person
//! with none is silently skipped (`mail::sweep` → `skipped_no_email`). There is
//! no opt-out flag, so removing a recipient means **blanking that email** on the
//! identities that carry it.
//!
//! This is safe and durable: login resolves an Identity by `(provider,
//! provider_id)` — Google by `sub`, e-mail by the address itself — not by the
//! `verified_email` field, and `auth::resolution::resolve_player` is read-only
//! for an already-claimed player, so a blanked `verified_email` is not
//! re-populated on the next sign-in and does not break it. (Only edge effect:
//! future invite-*re*-linking by that email — `find_identities_by_verified_email`
//! would no longer see them.)
//!
//! Idempotent; a read-only report unless `--apply`.

use anyhow::Context;
use domain::Identity;
use storage::Repository;

/// One identity whose `verified_email` carries (or carried) the target address.
#[derive(Debug, Clone)]
pub struct Cleared {
    pub identity_id: String,
    pub provider: String,
    pub provider_id: String,
    pub person_id: String,
    /// The player's nick, when the person resolves to one (for a human-readable
    /// report). `None` for an unclaimed identity.
    pub nick: Option<String>,
}

/// Outcome of a clear run.
#[derive(Debug, Default)]
pub struct ClearReport {
    pub email: String,
    pub cleared: Vec<Cleared>,
    pub applied: bool,
}

impl ClearReport {
    pub fn print(&self) {
        let mode = if self.applied {
            "APPLIED"
        } else {
            "DRY RUN (read-only)"
        };
        println!("== clear-verified-email {} — {mode} ==", self.email);
        if self.cleared.is_empty() {
            println!("no identity carries that verified_email — nothing to do");
            return;
        }
        let verb = if self.applied {
            "cleared"
        } else {
            "would clear"
        };
        println!(
            "{verb} verified_email on {} identit(y/ies):",
            self.cleared.len()
        );
        for c in &self.cleared {
            let who = c.nick.as_deref().unwrap_or("(unclaimed)");
            println!(
                "  {} [{}#{}] person={} nick={who}",
                c.identity_id, c.provider, c.provider_id, c.person_id
            );
        }
        if !self.applied {
            println!(
                "re-run with --apply to blank these — those recipients then stop getting reminders"
            );
        }
    }
}

/// Find every identity carrying `email` as its `verified_email` and (when
/// `apply`) blank that field, immutably (`put_identity` of a new value).
pub async fn run<R: Repository>(repo: &R, email: &str, apply: bool) -> anyhow::Result<ClearReport> {
    let hits = repo
        .find_identities_by_verified_email(email)
        .await
        .with_context(|| format!("looking up identities for {email}"))?;

    let mut report = ClearReport {
        email: email.to_string(),
        applied: apply,
        ..Default::default()
    };

    for identity in &hits {
        let nick = repo
            .get_player_by_person(&identity.person_id)
            .await?
            .map(|p| p.nick);
        report.cleared.push(Cleared {
            identity_id: identity.id.clone(),
            provider: identity.provider.clone(),
            provider_id: identity.provider_id.clone(),
            person_id: identity.person_id.clone(),
            nick,
        });
        if apply {
            let updated = Identity {
                verified_email: None,
                ..identity.clone()
            };
            repo.put_identity(&updated).await?;
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use domain::{Identity, Player};
    use storage::InMemoryRepository;

    fn identity(
        id: &str,
        provider: &str,
        provider_id: &str,
        person: &str,
        email: Option<&str>,
    ) -> Identity {
        Identity {
            id: id.into(),
            provider: provider.into(),
            provider_id: provider_id.into(),
            person_id: person.into(),
            verified_email: email.map(str::to_owned),
        }
    }

    fn player(id: &str, person: &str, nick: &str) -> Player {
        Player {
            id: id.into(),
            person_id: person.into(),
            nick: nick.into(),
            full_name: nick.into(),
            referrer: None,
            is_result_user: false,
            version: 0,
            match_predictions: vec![],
            standings_predictions: vec![],
        }
    }

    async fn repo_with(identities: Vec<Identity>, players: Vec<Player>) -> InMemoryRepository {
        let repo = InMemoryRepository::new();
        for i in &identities {
            repo.put_identity(i).await.unwrap();
        }
        for p in &players {
            repo.put_player(p).await.unwrap();
        }
        repo
    }

    #[tokio::test]
    async fn dry_run_reports_but_writes_nothing() {
        let repo = repo_with(
            vec![identity(
                "id-g",
                "google",
                "sub-1",
                "person-1",
                Some("target@x.com"),
            )],
            vec![player("pl-1", "person-1", "ada")],
        )
        .await;

        let report = run(&repo, "target@x.com", false).await.unwrap();

        assert_eq!(report.cleared.len(), 1);
        assert_eq!(report.cleared[0].nick.as_deref(), Some("ada"));
        assert!(!report.applied);
        // Still present — dry run wrote nothing.
        let still = repo
            .find_identities_by_verified_email("target@x.com")
            .await
            .unwrap();
        assert_eq!(still.len(), 1);
    }

    #[tokio::test]
    async fn apply_clears_every_matching_identity_then_idempotent() {
        // One person reachable at the same email via two identities (google + email).
        let repo = repo_with(
            vec![
                identity("id-g", "google", "sub-1", "person-1", Some("target@x.com")),
                identity(
                    "id-e",
                    "email",
                    "target@x.com",
                    "person-1",
                    Some("target@x.com"),
                ),
                // An unrelated identity that must be left alone.
                identity("id-o", "google", "sub-2", "person-2", Some("other@x.com")),
            ],
            vec![player("pl-1", "person-1", "ada")],
        )
        .await;

        let report = run(&repo, "target@x.com", true).await.unwrap();
        assert_eq!(
            report.cleared.len(),
            2,
            "both of person-1's identities matched"
        );

        // The target email now resolves to nobody.
        let gone = repo
            .find_identities_by_verified_email("target@x.com")
            .await
            .unwrap();
        assert!(
            gone.is_empty(),
            "no identity should still carry the address"
        );

        // The login keys (provider, provider_id) survive — only the email field went.
        let g = repo.get_identity("google", "sub-1").await.unwrap().unwrap();
        assert_eq!(g.person_id, "person-1");
        assert!(g.verified_email.is_none());
        let e = repo
            .get_identity("email", "target@x.com")
            .await
            .unwrap()
            .unwrap();
        assert!(e.verified_email.is_none());

        // Unrelated identity untouched.
        let other = repo.get_identity("google", "sub-2").await.unwrap().unwrap();
        assert_eq!(other.verified_email.as_deref(), Some("other@x.com"));

        // Second run is a no-op.
        let report2 = run(&repo, "target@x.com", true).await.unwrap();
        assert!(report2.cleared.is_empty(), "re-run finds nothing to clear");
    }

    #[tokio::test]
    async fn unknown_email_is_a_clean_no_op() {
        let repo = repo_with(
            vec![identity(
                "id-g",
                "google",
                "sub-1",
                "person-1",
                Some("real@x.com"),
            )],
            vec![],
        )
        .await;

        let report = run(&repo, "typo@x.com", true).await.unwrap();
        assert!(report.cleared.is_empty());
        // The real one is untouched.
        let real = repo
            .find_identities_by_verified_email("real@x.com")
            .await
            .unwrap();
        assert_eq!(real.len(), 1);
    }
}
