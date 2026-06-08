//! Pure invite-code helpers: the Crockford-base32 alphabet, suffix encoding,
//! and lenient parsing of typed/pasted codes (`DESIGN.md` Phase 0 decisions).

use domain::invite::{self, CodeInput, CROCKFORD_ALPHABET, SUFFIX_LEN};

// ── alphabet ──────────────────────────────────────────────────────────────────

#[test]
fn alphabet_is_32_unique_unambiguous_chars() {
    assert_eq!(CROCKFORD_ALPHABET.len(), 32);
    let s: String = CROCKFORD_ALPHABET.iter().map(|b| *b as char).collect();
    // No ambiguous characters: I, L, O, U are excluded.
    for bad in ['I', 'L', 'O', 'U'] {
        assert!(!s.contains(bad), "alphabet must exclude {bad}");
    }
    // All unique.
    let mut chars: Vec<char> = s.chars().collect();
    chars.sort_unstable();
    chars.dedup();
    assert_eq!(chars.len(), 32, "alphabet must be 32 distinct chars");
}

// ── encode_suffix ───────────────────────────────────────────────────────────────

#[test]
fn encode_suffix_maps_each_byte_into_the_alphabet() {
    // byte % 32: 0→'0', 1→'1', 31→'Z', 32→'0', 33→'1'
    let s = invite::encode_suffix(&[0, 1, 31, 32, 33]);
    assert_eq!(s, "01Z01");
}

#[test]
fn encode_suffix_only_produces_alphabet_chars() {
    let bytes: Vec<u8> = (0..=255).collect();
    let s = invite::encode_suffix(&bytes);
    let alphabet: String = CROCKFORD_ALPHABET.iter().map(|b| *b as char).collect();
    assert!(
        s.chars().all(|c| alphabet.contains(c)),
        "every output char must be in the alphabet"
    );
}

#[test]
fn encode_suffix_length_matches_input_byte_count() {
    let bytes = [7u8; SUFFIX_LEN];
    assert_eq!(invite::encode_suffix(&bytes).len(), SUFFIX_LEN);
}

// ── parse_code ──────────────────────────────────────────────────────────────────

#[test]
fn parse_full_form_splits_prefix_and_suffix() {
    assert_eq!(
        invite::parse_code("SOUTH7K-AD9XK3P7QT"),
        Some(CodeInput::PrefixAndSuffix {
            prefix: "SOUTH7K".to_owned(),
            suffix: "AD9XK3P7QT".to_owned(),
        })
    );
}

#[test]
fn parse_bare_suffix_is_bare() {
    assert_eq!(
        invite::parse_code("AD9XK3P7QT"),
        Some(CodeInput::Bare("AD9XK3P7QT".to_owned()))
    );
}

#[test]
fn parse_bare_prefix_is_bare() {
    assert_eq!(
        invite::parse_code("SOUTH7K"),
        Some(CodeInput::Bare("SOUTH7K".to_owned()))
    );
}

#[test]
fn parse_extracts_code_from_a_pasted_url() {
    assert_eq!(
        invite::parse_code("https://xpool.example/invite/SOUTH7K-AD9XK3P7QT"),
        Some(CodeInput::PrefixAndSuffix {
            prefix: "SOUTH7K".to_owned(),
            suffix: "AD9XK3P7QT".to_owned(),
        })
    );
}

#[test]
fn parse_is_case_insensitive_and_tolerates_whitespace() {
    assert_eq!(
        invite::parse_code("  ad9xk3p7qt  "),
        Some(CodeInput::Bare("AD9XK3P7QT".to_owned()))
    );
}

#[test]
fn parse_preserves_a_real_o_in_the_prefix() {
    // The prefix is cosmetic and may legitimately contain O/I/L — parse must
    // NOT Crockford-map it (that mapping is suffix-only).
    assert_eq!(
        invite::parse_code("SOUTH7K-AD9XK3P7QT"),
        Some(CodeInput::PrefixAndSuffix {
            prefix: "SOUTH7K".to_owned(),
            suffix: "AD9XK3P7QT".to_owned(),
        })
    );
}

#[test]
fn normalize_suffix_maps_ambiguous_characters_crockford_style() {
    // A user types O for 0 and I/L for 1 in the key; leniency maps them back.
    assert_eq!(invite::normalize_suffix("adoxi3p7qt"), "AD0X13P7QT");
    assert_eq!(invite::normalize_suffix("AD9XK3P7QT"), "AD9XK3P7QT");
}

#[test]
fn parse_empty_or_blank_is_none() {
    assert_eq!(invite::parse_code("   "), None);
    assert_eq!(invite::parse_code(""), None);
}

// ── slugify (cosmetic pool prefix) ──────────────────────────────────────────────

#[test]
fn slugify_uppercases_strips_nonalnum_and_truncates() {
    assert_eq!(invite::slugify("South Siders!", 5), "SOUTH");
}

#[test]
fn slugify_keeps_short_names_intact() {
    assert_eq!(invite::slugify("Work", 5), "WORK");
}
