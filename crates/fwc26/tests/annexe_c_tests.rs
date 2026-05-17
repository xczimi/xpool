//! Tests for the Annexe C lookup table (FWC26_RULES.md §5).

use fwc26::{annexe_c, ANNEXE_C};
use std::collections::{BTreeSet, HashSet};

/// The 495 rows must all be present.
#[test]
fn test_annexe_c_row_count() {
    assert_eq!(
        ANNEXE_C.len(),
        495,
        "Annexe C must have exactly 495 rows (12 choose 8 = 495)"
    );
}

/// Every key (qualifying set) must be exactly 8 distinct letters from A-L.
#[test]
fn test_annexe_c_keys_are_valid_8_subsets() {
    for (i, (key, _)) in ANNEXE_C.iter().enumerate() {
        let key_chars: Vec<char> = key.iter().map(|&b| b as char).collect();

        // Must be 8 distinct letters
        let unique: HashSet<char> = key_chars.iter().copied().collect();
        assert_eq!(
            unique.len(),
            8,
            "Row {}: key must have 8 distinct letters, got {:?}",
            i + 1,
            key_chars
        );

        // All letters must be in A-L
        for &c in &key_chars {
            assert!(
                ('A'..='L').contains(&c),
                "Row {}: invalid group letter '{}' in key",
                i + 1,
                c
            );
        }

        // Must be sorted
        let mut sorted = key_chars.clone();
        sorted.sort();
        let sorted_bytes: Vec<u8> = sorted.iter().map(|&c| c as u8).collect();
        assert_eq!(
            &sorted_bytes[..],
            key,
            "Row {}: key must be sorted alphabetically",
            i + 1
        );
    }
}

/// All 495 keys must be distinct.
#[test]
fn test_annexe_c_keys_are_unique() {
    let keys: HashSet<[u8; 8]> = ANNEXE_C.iter().map(|(k, _)| *k).collect();
    assert_eq!(
        keys.len(),
        495,
        "All 495 Annexe C keys must be distinct; found {} unique keys",
        keys.len()
    );
}

/// The values (third-group assignments) must each be 8 distinct letters from A-L
/// and must exactly cover the key letters.
#[test]
fn test_annexe_c_values_are_permutations_of_keys() {
    for (i, (key, thirds)) in ANNEXE_C.iter().enumerate() {
        let key_set: HashSet<u8> = key.iter().copied().collect();
        let thirds_set: HashSet<u8> = thirds.iter().copied().collect();

        assert_eq!(
            thirds_set.len(),
            8,
            "Row {}: thirds must have 8 distinct letters",
            i + 1
        );
        assert_eq!(
            key_set,
            thirds_set,
            "Row {}: thirds letters must be exactly the key letters",
            i + 1
        );

        for &b in thirds.iter() {
            let c = b as char;
            assert!(
                ('A'..='L').contains(&c),
                "Row {}: invalid group letter '{}' in thirds",
                i + 1,
                c
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Specific row lookups from the spec
// ---------------------------------------------------------------------------

fn btree(letters: &[char]) -> BTreeSet<char> {
    letters.iter().copied().collect()
}

/// Option 1: qualifying = {E,F,G,H,I,J,K,L}
/// Expected: 1A→3E, 1B→3J, 1D→3I, 1E→3F, 1G→3H, 1I→3G, 1K→3L, 1L→3K
#[test]
fn test_annexe_c_option_1() {
    let key = btree(&['E', 'F', 'G', 'H', 'I', 'J', 'K', 'L']);
    let map = annexe_c(&key).expect("Option 1 must be found");
    assert_eq!(map[&'A'], 'E', "1A→3E");
    assert_eq!(map[&'B'], 'J', "1B→3J");
    assert_eq!(map[&'D'], 'I', "1D→3I");
    assert_eq!(map[&'E'], 'F', "1E→3F");
    assert_eq!(map[&'G'], 'H', "1G→3H");
    assert_eq!(map[&'I'], 'G', "1I→3G");
    assert_eq!(map[&'K'], 'L', "1K→3L");
    assert_eq!(map[&'L'], 'K', "1L→3K");
}

/// Option 7: qualifying = {D,E,F,G,H,I,K,L}
/// Expected: 1A→3E, 1B→3G, 1D→3I, 1E→3D, 1G→3H, 1I→3F, 1K→3L, 1L→3K
#[test]
fn test_annexe_c_option_7() {
    let key = btree(&['D', 'E', 'F', 'G', 'H', 'I', 'K', 'L']);
    let map = annexe_c(&key).expect("Option 7 must be found");
    assert_eq!(map[&'A'], 'E', "1A→3E");
    assert_eq!(map[&'B'], 'G', "1B→3G");
    assert_eq!(map[&'D'], 'I', "1D→3I");
    assert_eq!(map[&'E'], 'D', "1E→3D");
    assert_eq!(map[&'G'], 'H', "1G→3H");
    assert_eq!(map[&'I'], 'F', "1I→3F");
    assert_eq!(map[&'K'], 'L', "1K→3L");
    assert_eq!(map[&'L'], 'K', "1L→3K");
}

/// Option 495 (last row): qualifying = {A,B,C,D,F,G,H} ... let me check
/// From spec: | 495 | 3H | 3G | 3B | 3C | 3A | 3F | 3D | 3E |
/// The thirds are H,G,B,C,A,F,D,E → qualifying set = {A,B,C,D,E,F,G,H}
#[test]
fn test_annexe_c_option_495() {
    let key = btree(&['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H']);
    let map = annexe_c(&key).expect("Option 495 must be found");
    assert_eq!(map[&'A'], 'H', "1A→3H");
    assert_eq!(map[&'B'], 'G', "1B→3G");
    assert_eq!(map[&'D'], 'B', "1D→3B");
    assert_eq!(map[&'E'], 'C', "1E→3C");
    assert_eq!(map[&'G'], 'A', "1G→3A");
    assert_eq!(map[&'I'], 'F', "1I→3F");
    assert_eq!(map[&'K'], 'D', "1K→3D");
    assert_eq!(map[&'L'], 'E', "1L→3E");
}

/// Option 46: qualifying includes B → {B,F,G,H,I,J,K,L}
/// From spec: | 46 | 3H | 3J | 3B | 3F | 3I | 3G | 3L | 3K |
#[test]
fn test_annexe_c_option_46() {
    let key = btree(&['B', 'F', 'G', 'H', 'I', 'J', 'K', 'L']);
    let map = annexe_c(&key).expect("Option 46 must be found");
    assert_eq!(map[&'A'], 'H', "1A→3H");
    assert_eq!(map[&'B'], 'J', "1B→3J");
    assert_eq!(map[&'D'], 'B', "1D→3B");
    assert_eq!(map[&'E'], 'F', "1E→3F");
    assert_eq!(map[&'G'], 'I', "1G→3I");
    assert_eq!(map[&'I'], 'G', "1I→3G");
    assert_eq!(map[&'K'], 'L', "1K→3L");
    assert_eq!(map[&'L'], 'K', "1L→3K");
}

/// A set of fewer than 8 groups returns None.
#[test]
fn test_annexe_c_invalid_set_size() {
    let key = btree(&['A', 'B', 'C', 'D', 'E', 'F', 'G']);
    assert!(annexe_c(&key).is_none(), "7-element set must return None");
}

/// A set of 9 groups returns None (even if individually valid).
#[test]
fn test_annexe_c_oversized_set() {
    let key = btree(&['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H', 'I']);
    assert!(annexe_c(&key).is_none(), "9-element set must return None");
}

/// A set with an invalid letter returns None.
#[test]
fn test_annexe_c_invalid_letter() {
    // 'Z' is not a valid group letter (only A-L are valid)
    // BTreeSet won't include 'Z' but the lookup will simply miss it
    let mut key: BTreeSet<char> = btree(&['A', 'B', 'C', 'D', 'E', 'F', 'G']);
    key.insert('Z');
    // This has 8 elements but 'Z' is not valid, so no row will match
    assert!(
        annexe_c(&key).is_none(),
        "Set with invalid letter must return None"
    );
}

/// Result map has exactly 8 entries with the correct winner keys.
#[test]
fn test_annexe_c_result_map_keys() {
    let key = btree(&['A', 'B', 'C', 'D', 'E', 'F', 'G', 'H']);
    let map = annexe_c(&key).expect("Must be found");
    let expected_winners: HashSet<char> = ['A', 'B', 'D', 'E', 'G', 'I', 'K', 'L']
        .iter()
        .copied()
        .collect();
    let actual_winners: HashSet<char> = map.keys().copied().collect();
    assert_eq!(
        actual_winners, expected_winners,
        "Result map must have keys A,B,D,E,G,I,K,L"
    );
}
