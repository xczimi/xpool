//! Guard tests: pin the hardcoded `BEST_THIRD_SLOTS` map (FWC26_RULES.md §4)
//! to the FWC26 fixture so a fixture drift fails loudly instead of silently
//! resolving a best-third R32 slot to `None`.

use fwc26::BEST_THIRD_SLOTS;
use std::path::PathBuf;

/// Read `tournaments/fwc26.json` from the repo root.
fn read_fixture() -> String {
    // CARGO_MANIFEST_DIR is `<repo>/crates/fwc26`; the fixture is at the root.
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.pop();
    path.pop();
    path.push("tournaments");
    path.push("fwc26.json");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {e}", path.display()))
}

/// Every key in `BEST_THIRD_SLOTS` must correspond to a real `3XXXXX` slot
/// description in the imported FWC26 fixture.
#[test]
fn test_best_third_slots_match_fixture() {
    let fixture = read_fixture();

    for (groups_str, winner) in BEST_THIRD_SLOTS {
        let needle = format!("\"3{groups_str}\"");
        assert!(
            fixture.contains(&needle),
            "BEST_THIRD_SLOTS entry (\"{groups_str}\" → '{winner}') has no matching \
             slot description {needle} in tournaments/fwc26.json — fixture drift: \
             update the hardcoded map in crates/fwc26/src/lib.rs"
        );
    }
}

/// The map must have exactly 8 entries — one per `THIRD_FACING_WINNERS` column.
#[test]
fn test_best_third_slots_count() {
    assert_eq!(
        BEST_THIRD_SLOTS.len(),
        8,
        "exactly 8 group winners face a 3rd-placed team in R32"
    );

    // All 8 winner letters must be distinct.
    let mut winners: Vec<char> = BEST_THIRD_SLOTS.iter().map(|(_, w)| *w).collect();
    winners.sort_unstable();
    winners.dedup();
    assert_eq!(winners.len(), 8, "winner groups must be distinct");
}
