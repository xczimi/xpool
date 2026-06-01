//! The §3 login-resolution algorithm.

use crate::auth::jwt::VerifiedClaims;
use crate::auth::{CurrentPlayer, VerifiedIdentity};
use storage::Repository;

/// Returns the `(provider, provider_id)` an Identity row should be keyed
/// at for a given verified claims-set. Returns None when the connection
/// has no usable contact (shouldn't happen with verified claims, but the
/// caller should treat None as "Visitor").
pub fn identity_key_for(claims: &VerifiedClaims) -> Option<(String, String)> {
    match claims.connection.as_str() {
        "email" | "dev" => {
            claims.verified_email.as_ref().map(|e| ("email".to_owned(), e.clone()))
        }
        "sms" => {
            claims.verified_phone.as_ref().map(|p| ("phone".to_owned(), p.clone()))
        }
        "google" => Some(("google".to_owned(), claims.sub.clone())),
        _ => None,
    }
}

/// The full algorithm:
///
/// 1. Look up Identity by (provider, provider_id).
/// 2. Found → Person → Player → return Player.
/// 3. Not found, verified email exists, find_identities_by_verified_email
///    returns hits → AuthenticatedUnclaimed (link path — UI prompts in
///    Phase 6).
/// 4. Not found, no email match → AuthenticatedUnclaimed (claim/join
///    path).
/// 5. No verified contact at all → Visitor.
pub async fn resolve_player(
    repo: &dyn Repository,
    claims: VerifiedClaims,
) -> CurrentPlayer {
    let Some((provider, provider_id)) = identity_key_for(&claims) else {
        return CurrentPlayer::Visitor;
    };

    if let Ok(Some(identity)) = repo.get_identity(&provider, &provider_id).await {
        if let Ok(Some(_person)) = repo.get_person(&identity.person_id).await {
            // Person → Player. By the data model's convention, `Player.id`
            // equals `Person.id` for the current tournament.
            if let Ok(Some(player)) = repo.get_player(&identity.person_id).await {
                return CurrentPlayer::Player(Box::new(player));
            }
        }
    }
    // Fall through to unclaimed — the resolver caller (or claim mutation)
    // handles the link-vs-claim disambiguation against
    // `find_identities_by_verified_email`.
    CurrentPlayer::AuthenticatedUnclaimed(VerifiedIdentity {
        connection: claims.connection,
        sub: claims.sub,
        verified_email: claims.verified_email,
        verified_phone: claims.verified_phone,
    })
}
