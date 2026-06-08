//! Pure invite-code helpers — alphabet, suffix encoding, lenient parsing, and
//! cosmetic pool-prefix slugs (`DESIGN.md` Phase 0 decisions). No I/O: the
//! entropy draw and table lookups belong to the application layer; this module
//! only encodes bytes and parses strings.
//!
//! Format: `PREFIX-SUFFIX` (e.g. `SOUTH7K-AD9XK3P7QT`). The **suffix** is the
//! globally-unique high-entropy key (10 Crockford-base32 chars ≈ 50 bits); the
//! **prefix** is a cosmetic, validatable pool label. Entry is lenient: the full
//! form, a bare suffix, or a bare prefix all parse, case-insensitively, and
//! pasted URLs are accepted.

/// Crockford base32 alphabet — 32 chars, excluding the ambiguous `I`, `L`, `O`,
/// `U`. `32 == 256 / 8`, so `byte % 32` is an unbiased mapping.
pub const CROCKFORD_ALPHABET: &[u8; 32] = b"0123456789ABCDEFGHJKMNPQRSTVWXYZ";

/// Length (in chars) of a generated invite suffix. 10 × 5 bits ≈ 50 bits.
pub const SUFFIX_LEN: usize = 10;

/// Map random bytes to a Crockford-base32 string, one char per input byte.
/// Each byte is reduced `% 32` (unbiased, since 32 divides 256). Supply
/// [`SUFFIX_LEN`] bytes of OS randomness to mint a suffix.
pub fn encode_suffix(random_bytes: &[u8]) -> String {
    random_bytes
        .iter()
        .map(|b| CROCKFORD_ALPHABET[(*b % 32) as usize] as char)
        .collect()
}

/// A parsed invite-code entry. Resolution (suffix → invite, prefix → owner
/// invite) is the application layer's job; this only structures the input.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CodeInput {
    /// Full `PREFIX-SUFFIX` form. The suffix is the key; the prefix is advisory.
    PrefixAndSuffix { prefix: String, suffix: String },
    /// A single bare token — either a suffix (key) or a pool prefix. The caller
    /// resolves which by trying a suffix lookup first, then a prefix lookup.
    Bare(String),
}

/// Light cleanup for typed input: take the final path segment (so a pasted URL
/// works), uppercase, and strip whitespace. Dashes are preserved (they separate
/// prefix from suffix). No ambiguous-char mapping here — that would corrupt a
/// cosmetic prefix containing a real `O`/`I`/`L`; see [`normalize_suffix`].
fn clean(raw: &str) -> String {
    let token = raw.trim().rsplit('/').next().unwrap_or("").trim();
    token
        .chars()
        .filter(|c| !c.is_whitespace())
        .map(|c| c.to_ascii_uppercase())
        .collect()
}

/// Canonicalize a **suffix** token (the key) for lookup: uppercase, drop
/// non-alphanumerics, and apply Crockford leniency — `O → 0`, `I/L → 1`. The
/// suffix alphabet excludes `O`/`I`/`L`/`U`, so a genuine suffix never contains
/// them; mapping is purely to forgive a human who typed the look-alike.
pub fn normalize_suffix(suffix: &str) -> String {
    suffix
        .chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| match c.to_ascii_uppercase() {
            'O' => '0',
            'I' | 'L' => '1',
            other => other,
        })
        .collect()
}

/// Parse a typed or pasted invite code. Returns `None` for blank input.
/// Splits on the **last** `-` (the suffix carries no dash), so a hyphenated
/// prefix still resolves. Apply [`normalize_suffix`] to the suffix before a
/// table lookup; the prefix is left as-is (cosmetic, case-folded only).
pub fn parse_code(raw: &str) -> Option<CodeInput> {
    let normalized = clean(raw);
    if normalized.is_empty() {
        return None;
    }
    match normalized.rsplit_once('-') {
        Some((prefix, suffix)) if !prefix.is_empty() && !suffix.is_empty() => {
            Some(CodeInput::PrefixAndSuffix {
                prefix: prefix.to_owned(),
                suffix: suffix.to_owned(),
            })
        }
        // A leading/trailing-only dash collapses to a bare token.
        _ => Some(CodeInput::Bare(normalized.replace('-', ""))),
    }
}

/// Build a cosmetic pool-prefix label from a name: uppercase, keep ASCII
/// alphanumerics, truncate to `max_len`. Uniqueness (and any disambiguator) is
/// the application layer's concern — this is the pure slug part only.
pub fn slugify(name: &str, max_len: usize) -> String {
    name.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_uppercase())
        .take(max_len)
        .collect()
}
