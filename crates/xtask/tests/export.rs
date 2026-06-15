//! End-to-end test for the `export` → `load` data pull, against DynamoDB Local.
//!
//! Skipped unless `DYNAMO_TEST=1` (and a reachable `DYNAMO_ENDPOINT`), matching
//! the storage-crate integration-test gate:
//! ```bash
//! docker compose up -d dynamodb
//! DYNAMO_TEST=1 DYNAMO_ENDPOINT=http://localhost:8000 cargo test -p xtask
//! ```

use domain::{Identity, Player};
use storage::{DynamoRepository, Repository};

fn dynamo_enabled() -> bool {
    std::env::var("DYNAMO_TEST").as_deref() == Ok("1")
}

/// A repository on a uniquely-named table — `export` scans the whole table, so
/// each test side needs its own.
async fn unique_repo(suffix: &str) -> DynamoRepository {
    std::env::set_var("XPOOL_TABLE", "xpool-test");
    std::env::set_var("CURRENT_TOURNAMENT_ID", "test");
    let base = DynamoRepository::from_env().await.expect("build repo");
    DynamoRepository {
        table: format!("xpool-export-{suffix}-{}", std::process::id()),
        ..base
    }
}

fn player(nick: &str, person_id: &str) -> Player {
    Player {
        id: format!("player-{nick}"),
        person_id: person_id.to_owned(),
        nick: nick.to_owned(),
        full_name: "Ada Lovelace".to_owned(),
        referrer: None,
        is_result_user: false,
        version: 0,
        match_predictions: vec![],
        standings_predictions: vec![],
    }
}

#[tokio::test]
async fn export_anonymizes_emails_then_load_restores_everything_else() {
    if !dynamo_enabled() {
        return;
    }

    // ── source table: a player + a real-email identity linked by person id ──
    let src = unique_repo("src").await;
    src.ensure_table().await.unwrap();
    src.put_player(&player("ada", "p-ada")).await.unwrap();
    src.put_identity(&Identity {
        id: "identity-ada".to_owned(),
        provider: "email".to_owned(),
        provider_id: "ada@real.example".to_owned(),
        person_id: "p-ada".to_owned(),
        verified_email: Some("ada@real.example".to_owned()),
    })
    .await
    .unwrap();

    // ── export (anonymise) to a temp snapshot file ──
    let dir = tempfile::tempdir().unwrap();
    let file = dir.path().join("snapshot.json");
    let summary = xtask::export::export(&src, &file, true).await.unwrap();
    assert_eq!(summary.players, 1);
    assert_eq!(summary.identities, 1);

    // The real e-mail must not have been written to disk.
    let on_disk = std::fs::read_to_string(&file).unwrap();
    assert!(
        !on_disk.contains("ada@real.example"),
        "real e-mail leaked into the snapshot file"
    );
    assert!(on_disk.contains("ada@dev.invalid"));
    assert!(on_disk.contains("Ada Lovelace"), "real name is preserved");

    // ── load into a fresh table ──
    let dst = unique_repo("dst").await;
    dst.ensure_table().await.unwrap();
    xtask::export::load(&dst, &file).await.unwrap();

    // Identity is keyed under the anonymised address now; the real one is gone.
    assert!(
        dst.get_identity("email", "ada@real.example")
            .await
            .unwrap()
            .is_none(),
        "real-email identity must not exist"
    );
    let anon = dst
        .get_identity("email", "ada@dev.invalid")
        .await
        .unwrap()
        .expect("anonymised identity present");
    assert_eq!(anon.verified_email.as_deref(), Some("ada@dev.invalid"));
    assert_eq!(anon.person_id, "p-ada", "person link preserved");

    // The player round-trips intact — real name kept, version preserved.
    let p = dst.get_player("player-ada").await.unwrap().expect("player");
    assert_eq!(p.full_name, "Ada Lovelace");
    assert_eq!(p.nick, "ada");
    assert_eq!(
        p.version, 1,
        "put_player stored version 1; load preserved it"
    );

    // ── load is idempotent ──
    xtask::export::load(&dst, &file).await.unwrap();
    let p2 = dst.get_player("player-ada").await.unwrap().expect("player");
    assert_eq!(p2.version, 1, "re-load overwrites, no version drift");

    src.delete_table().await.unwrap();
    dst.delete_table().await.unwrap();
}
