//! Table snapshots for the prod→local data pull (`bin/pull-data` tooling).
//!
//! A [`Snapshot`] is a faithful, transport-friendly capture of a DynamoDB
//! table: the raw `(pk, sk, data, version)` rows exactly as stored. Because
//! every row's `data` is kept as an opaque JSON string, the round-trip is
//! byte-faithful for every row the export does not deliberately rewrite.
//!
//! The one deliberate rewrite is [`anonymize_emails`]: it remaps the e-mail in
//! every `IDENTITY#…` row to `<nick>@dev.invalid`, leaving names, nicks,
//! predictions and all other rows untouched. That keeps real people's addresses
//! off local disk while preserving the full identity→person→player graph so the
//! dev-login endpoint still resolves each account.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use storage::RawItem;

/// The synthetic e-mail domain anonymised addresses land in. `dev.invalid` is
/// the reserved `.invalid` TLD — guaranteed never to resolve or receive mail.
const ANON_DOMAIN: &str = "dev.invalid";

/// A faithful capture of a DynamoDB table: its rows, verbatim.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub struct Snapshot {
    pub items: Vec<RawItem>,
}

impl Snapshot {
    pub fn new(items: Vec<RawItem>) -> Self {
        Self { items }
    }

    /// Rows whose `pk` matches a predicate — a small read helper for callers
    /// (e.g. counting players in a snapshot).
    pub fn rows_where(&self, pred: impl Fn(&str) -> bool) -> impl Iterator<Item = &RawItem> {
        self.items.iter().filter(move |it| pred(&it.pk))
    }
}

/// Remap every e-mail in the snapshot's `IDENTITY#…` rows to
/// `<nick>@dev.invalid`. Pure — returns a new [`Snapshot`]; the input is left
/// untouched. Names, nicks, predictions and every non-identity row pass through
/// byte-for-byte.
pub fn anonymize_emails(snapshot: Snapshot) -> Snapshot {
    let nick_by_person = nick_by_person(&snapshot);
    let items = snapshot
        .items
        .into_iter()
        .map(|item| {
            if item.pk.starts_with("IDENTITY#") {
                anonymize_identity(item, &nick_by_person)
            } else {
                item
            }
        })
        .collect();
    Snapshot::new(items)
}

/// Build `person_id → nick` from the snapshot's Player rows. Players are the
/// only rows that carry a nick, and an Identity links to its Player via the
/// shared `person_id`.
fn nick_by_person(snapshot: &Snapshot) -> HashMap<String, String> {
    snapshot
        .items
        .iter()
        .filter(|it| it.pk.ends_with("#PLAYER"))
        .filter_map(|it| {
            let data: serde_json::Value = serde_json::from_str(&it.data).ok()?;
            let person_id = data.get("person_id")?.as_str()?.to_owned();
            let nick = data.get("nick")?.as_str()?.to_owned();
            Some((person_id, nick))
        })
        .collect()
}

/// Rewrite one `IDENTITY#…` row: replace `verified_email` (and, for an
/// email-provider identity, the `provider_id` and the `pk` that embeds it) with
/// the anonymised address derived from the linked player's nick.
fn anonymize_identity(item: RawItem, nick_by_person: &HashMap<String, String>) -> RawItem {
    let Ok(mut data) = serde_json::from_str::<serde_json::Value>(&item.data) else {
        // Unparseable identity data: leave the row exactly as-is rather than
        // risk corrupting it. (Should not happen for real rows.)
        return item;
    };

    let person_id = data.get("person_id").and_then(|v| v.as_str());
    let new_email = anon_email(person_id, nick_by_person);

    let provider = data
        .get("provider")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_owned();

    // Always remap a present verified_email; leave an absent/null one absent.
    if data
        .get("verified_email")
        .map(|v| !v.is_null())
        .unwrap_or(false)
    {
        data["verified_email"] = serde_json::Value::String(new_email.clone());
    }

    // For an email-provider identity the address is also the provider_id, which
    // is embedded in the pk — rewrite both so the key stays consistent.
    let new_pk = if provider == "email" {
        data["provider_id"] = serde_json::Value::String(new_email.clone());
        format!("IDENTITY#email#{new_email}")
    } else {
        item.pk
    };

    let data = serde_json::to_string(&data).unwrap_or(item.data);
    RawItem {
        pk: new_pk,
        sk: item.sk,
        data,
        version: item.version,
    }
}

/// The anonymised address for a person: `<nick>@dev.invalid` when the linked
/// player is known, else a stable `anon-<person_id>@dev.invalid` fallback.
fn anon_email(person_id: Option<&str>, nick_by_person: &HashMap<String, String>) -> String {
    match person_id.and_then(|p| nick_by_person.get(p)) {
        Some(nick) => format!("{nick}@{ANON_DOMAIN}"),
        None => format!("anon-{}@{ANON_DOMAIN}", person_id.unwrap_or("unknown")),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn player_row(person_id: &str, nick: &str) -> RawItem {
        RawItem {
            pk: "fwc26#PLAYER".to_owned(),
            sk: format!("player-{nick}"),
            data: serde_json::json!({
                "id": format!("player-{nick}"),
                "person_id": person_id,
                "nick": nick,
                "full_name": "Real Name",
            })
            .to_string(),
            version: Some(3),
        }
    }

    fn identity_row(
        provider: &str,
        provider_id: &str,
        person_id: &str,
        email: Option<&str>,
    ) -> RawItem {
        RawItem {
            pk: format!("IDENTITY#{provider}#{provider_id}"),
            sk: "#".to_owned(),
            data: serde_json::json!({
                "id": format!("identity-{provider_id}"),
                "provider": provider,
                "provider_id": provider_id,
                "person_id": person_id,
                "verified_email": email,
            })
            .to_string(),
            version: None,
        }
    }

    fn data_of(snap: &Snapshot, pk_starts: &str) -> serde_json::Value {
        let row = snap
            .items
            .iter()
            .find(|it| it.pk.starts_with(pk_starts))
            .expect("row present");
        serde_json::from_str(&row.data).unwrap()
    }

    #[test]
    fn email_provider_identity_rewrites_pk_provider_id_and_verified_email() {
        let snap = Snapshot::new(vec![
            player_row("p-ada", "ada"),
            identity_row("email", "ada@real.com", "p-ada", Some("ada@real.com")),
        ]);

        let out = anonymize_emails(snap);

        let id = out
            .items
            .iter()
            .find(|it| it.pk.starts_with("IDENTITY#"))
            .unwrap();
        assert_eq!(id.pk, "IDENTITY#email#ada@dev.invalid");
        let data = data_of(&out, "IDENTITY#");
        assert_eq!(data["provider_id"], "ada@dev.invalid");
        assert_eq!(data["verified_email"], "ada@dev.invalid");
    }

    #[test]
    fn non_email_provider_keeps_pk_and_provider_id_but_rewrites_verified_email() {
        let snap = Snapshot::new(vec![
            player_row("p-grace", "grace"),
            identity_row("auth0", "auth0|abc123", "p-grace", Some("grace@real.com")),
        ]);

        let out = anonymize_emails(snap);

        let id = out
            .items
            .iter()
            .find(|it| it.pk.starts_with("IDENTITY#"))
            .unwrap();
        assert_eq!(id.pk, "IDENTITY#auth0#auth0|abc123", "pk unchanged");
        let data = data_of(&out, "IDENTITY#");
        assert_eq!(data["provider_id"], "auth0|abc123", "opaque sub unchanged");
        assert_eq!(data["verified_email"], "grace@dev.invalid");
    }

    #[test]
    fn null_verified_email_stays_null() {
        let snap = Snapshot::new(vec![
            player_row("p-x", "linus"),
            identity_row("auth0", "auth0|x", "p-x", None),
        ]);

        let out = anonymize_emails(snap);
        let data = data_of(&out, "IDENTITY#");
        assert!(data["verified_email"].is_null());
    }

    #[test]
    fn unknown_person_uses_stable_fallback() {
        // No player row for this person → fallback address.
        let snap = Snapshot::new(vec![identity_row(
            "email",
            "ghost@real.com",
            "p-ghost",
            Some("ghost@real.com"),
        )]);

        let out = anonymize_emails(snap);
        let data = data_of(&out, "IDENTITY#");
        assert_eq!(data["verified_email"], "anon-p-ghost@dev.invalid");
        assert_eq!(out.items[0].pk, "IDENTITY#email#anon-p-ghost@dev.invalid");
    }

    #[test]
    fn non_identity_rows_pass_through_untouched() {
        let player = player_row("p-ada", "ada");
        let pool = RawItem {
            pk: "fwc26#POOL".to_owned(),
            sk: "pool-1".to_owned(),
            data: r#"{"id":"pool-1","name":"My Pool"}"#.to_owned(),
            version: None,
        };
        let snap = Snapshot::new(vec![player.clone(), pool.clone()]);

        let out = anonymize_emails(snap);

        assert!(out.items.contains(&player), "player row byte-identical");
        assert!(out.items.contains(&pool), "pool row byte-identical");
    }
}
