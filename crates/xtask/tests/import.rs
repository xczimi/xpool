//! Integration test for the tournament importer against the committed
//! `tournaments/fwc26.json`.

use domain::{GroupChildren, Round};
use std::path::PathBuf;
use storage::{InMemoryRepository, Repository};

fn fwc26_path() -> PathBuf {
    // CARGO_MANIFEST_DIR is crates/xtask; the tournaments dir is two levels up.
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tournaments/fwc26.json")
}

#[test]
fn loads_fwc26_with_expected_counts() {
    let t = xtask::load_tournament(&fwc26_path()).expect("fwc26.json should load and validate");

    assert_eq!(t.games.len(), 104, "104 games");
    assert_eq!(t.teams.len(), 48, "48 teams");

    let group_stage_leaves = t
        .groups
        .values()
        .filter(|g| g.round == Round::GroupStage && matches!(g.children, GroupChildren::Games(_)))
        .count();
    assert_eq!(group_stage_leaves, 12, "12 group-stage groups");

    // Each group-stage leaf has 6 games.
    for g in t.groups.values() {
        if g.round == Round::GroupStage {
            if let GroupChildren::Games(ids) = &g.children {
                assert_eq!(ids.len(), 6, "group {} has 6 games", g.id);
            }
        }
    }

    // Root resolves.
    assert!(t.groups.contains_key(&t.root));
}

#[tokio::test]
async fn import_is_idempotent() {
    let repo = InMemoryRepository::new();
    let t = xtask::load_tournament(&fwc26_path()).unwrap();

    repo.put_tournament(&t).await.unwrap();
    repo.put_tournament(&t).await.unwrap();

    let stored = repo.get_tournament().await.unwrap().unwrap();
    assert_eq!(stored.games.len(), 104);
}

#[tokio::test]
async fn seed_is_idempotent() {
    let repo = InMemoryRepository::new();
    xtask::seed::seed(&repo).await.unwrap();
    xtask::seed::seed(&repo).await.unwrap();

    let players = repo.list_players().await.unwrap();
    assert_eq!(players.len(), 7, "1 result user + 6 demo players");
    assert_eq!(players.iter().filter(|p| p.is_result_user).count(), 1);

    let pools = repo.list_pools().await.unwrap();
    assert_eq!(pools.len(), 1);
}
