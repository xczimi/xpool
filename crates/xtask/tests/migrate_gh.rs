//! Tests for the one-off FWC26 Group G/H standings-prediction migration
//! (`xtask fix-groups-gh`). The group-stage labels for G and H were swapped in
//! the fixture; correcting the tournament leaves any standings prediction
//! keyed by the old letter pointing at the wrong team-set. This migration
//! relabels those predictions, driven by their team content so it is
//! idempotent and order-independent.

use domain::{Player, StandingsPrediction};
use std::path::PathBuf;
use storage::{InMemoryRepository, Repository};
use xtask::migrate_gh::{self, classify_standings, GhDecision};

fn fwc26_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tournaments/fwc26.json")
}

fn teamset(ids: &[&str]) -> std::collections::BTreeSet<String> {
    ids.iter().map(|s| s.to_string()).collect()
}

fn sp(group_id: &str, ordering: &[&str]) -> StandingsPrediction {
    StandingsPrediction {
        group_id: group_id.to_string(),
        ordering: ordering.iter().map(|s| s.to_string()).collect(),
        draw_order: vec![],
        locked: false,
    }
}

fn player(id: &str, standings: Vec<StandingsPrediction>) -> Player {
    Player {
        id: id.to_string(),
        person_id: format!("person-{id}"),
        nick: id.to_string(),
        full_name: id.to_string(),
        referrer: None,
        is_result_user: false,
        version: 0,
        match_predictions: vec![],
        standings_predictions: standings,
    }
}

/// Seed the InMemoryRepository with the corrected tournament so the migration's
/// pre-flight guard passes.
async fn repo_with_corrected_tournament() -> InMemoryRepository {
    let repo = InMemoryRepository::new();
    let t = xtask::load_tournament(&fwc26_path()).unwrap();
    repo.put_tournament(&t).await.unwrap();
    repo
}

// ── pure classifier ─────────────────────────────────────────────────────────

#[test]
fn classify_relabels_when_team_set_belongs_to_the_other_group() {
    // H teams stored under "G" -> should become "H".
    assert_eq!(
        classify_standings("G", &teamset(&["ESP", "CPV", "KSA", "URU"])),
        GhDecision::Relabel { to: "H" }
    );
    // G teams stored under "H" -> should become "G".
    assert_eq!(
        classify_standings("H", &teamset(&["BEL", "EGY", "IRN", "NZL"])),
        GhDecision::Relabel { to: "G" }
    );
}

#[test]
fn classify_leaves_consistent_predictions() {
    assert_eq!(
        classify_standings("G", &teamset(&["BEL", "EGY", "IRN", "NZL"])),
        GhDecision::Consistent
    );
    assert_eq!(
        classify_standings("H", &teamset(&["ESP", "CPV", "KSA", "URU"])),
        GhDecision::Consistent
    );
}

#[test]
fn classify_ignores_other_groups() {
    assert_eq!(
        classify_standings("A", &teamset(&["MEX", "RSA"])),
        GhDecision::OutOfScope
    );
    assert_eq!(
        classify_standings("KO-M82", &teamset(&["ESP"])),
        GhDecision::OutOfScope
    );
}

#[test]
fn classify_flags_ambiguous_team_sets() {
    // Empty ordering -> nothing to go on.
    assert!(matches!(
        classify_standings("G", &teamset(&[])),
        GhDecision::Ambiguous(_)
    ));
    // A team in neither G nor H.
    assert!(matches!(
        classify_standings("H", &teamset(&["MEX"])),
        GhDecision::Ambiguous(_)
    ));
}

// ── run() over a repository ──────────────────────────────────────────────────

#[tokio::test]
async fn dry_run_reports_without_writing() {
    let repo = repo_with_corrected_tournament().await;
    repo.put_player(&player(
        "p1",
        vec![
            sp("G", &["ESP", "CPV", "KSA", "URU"]), // mislabeled
            sp("H", &["BEL", "EGY", "IRN", "NZL"]), // mislabeled
        ],
    ))
    .await
    .unwrap();

    let report = migrate_gh::run(&repo, false).await.unwrap();
    assert_eq!(report.relabels.len(), 2);
    assert_eq!(report.players_written, 0, "dry-run writes nothing");

    // Repository is unchanged.
    let p = repo.get_player("p1").await.unwrap().unwrap();
    assert_eq!(p.standings_prediction("G").unwrap().ordering[0], "ESP");
}

#[tokio::test]
async fn apply_swaps_labels_and_is_idempotent() {
    let repo = repo_with_corrected_tournament().await;
    repo.put_player(&player(
        "p1",
        vec![
            sp("G", &["ESP", "CPV", "KSA", "URU"]),
            sp("H", &["BEL", "EGY", "IRN", "NZL"]),
        ],
    ))
    .await
    .unwrap();

    let report = migrate_gh::run(&repo, true).await.unwrap();
    assert_eq!(report.relabels.len(), 2);
    assert_eq!(report.players_written, 1);

    let p = repo.get_player("p1").await.unwrap().unwrap();
    // The ESP/CPV/KSA/URU ordering now lives under H; BEL/... under G.
    assert_eq!(
        p.standings_prediction("H").unwrap().ordering,
        vec!["ESP", "CPV", "KSA", "URU"]
    );
    assert_eq!(
        p.standings_prediction("G").unwrap().ordering,
        vec!["BEL", "EGY", "IRN", "NZL"]
    );

    // Re-running changes nothing.
    let again = migrate_gh::run(&repo, true).await.unwrap();
    assert_eq!(
        again.relabels.len(),
        0,
        "idempotent: second apply is a no-op"
    );
    assert_eq!(again.players_written, 0);
}

#[tokio::test]
async fn leaves_consistent_and_out_of_scope_untouched() {
    let repo = repo_with_corrected_tournament().await;
    repo.put_player(&player(
        "p1",
        vec![
            sp("G", &["BEL", "EGY", "IRN", "NZL"]), // already correct
            sp("A", &["MEX", "RSA"]),               // not G/H
        ],
    ))
    .await
    .unwrap();

    let report = migrate_gh::run(&repo, true).await.unwrap();
    assert_eq!(report.relabels.len(), 0);
    assert_eq!(report.ambiguous.len(), 0);
    assert_eq!(report.players_written, 0);
}

#[tokio::test]
async fn ambiguous_predictions_are_reported_not_changed() {
    let repo = repo_with_corrected_tournament().await;
    repo.put_player(&player("p1", vec![sp("G", &[]), sp("H", &["MEX"])]))
        .await
        .unwrap();

    let report = migrate_gh::run(&repo, true).await.unwrap();
    assert_eq!(report.relabels.len(), 0);
    assert_eq!(report.ambiguous.len(), 2);
    assert_eq!(
        report.players_written, 0,
        "ambiguous predictions are never auto-changed"
    );
}

#[tokio::test]
async fn guard_bails_when_tournament_missing() {
    let repo = InMemoryRepository::new(); // no tournament imported
    let err = migrate_gh::run(&repo, false).await.unwrap_err();
    assert!(
        err.to_string().contains("import"),
        "guard should tell the operator to import first, got: {err}"
    );
}
