//! `verified_claims → Identity → Person → Player` resolution. Filled in
//! by Task 13.

use crate::auth::CurrentPlayer;
use crate::auth::jwt::VerifiedClaims;
use storage::Repository;

#[allow(dead_code)]
pub async fn resolve_player(_repo: &dyn Repository, _claims: VerifiedClaims) -> CurrentPlayer {
    unimplemented!("Task 13")
}
