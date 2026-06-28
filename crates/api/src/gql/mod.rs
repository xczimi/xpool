//! The GraphQL layer — a thin adapter (`API.md` §3).
//!
//! Coarse load → expose graph → glue to the pure domain. Types are
//! `SimpleObject` mirrors of `domain` structs; resolvers carry no domain
//! logic.

pub mod inputs;
pub mod mutation;
pub mod query;
pub mod types;

use async_graphql::{EmptySubscription, Schema};
use mutation::MutationRoot;
use query::QueryRoot;
use std::sync::Arc;
use storage::Repository;

use crate::reported::ReportedResultSource;

/// The xpool GraphQL schema type.
pub type XpoolSchema = Schema<QueryRoot, MutationRoot, EmptySubscription>;

/// Build the schema, injecting the repository, reported-results source, and a
/// no-op mail sender. The per-request `CurrentPlayer` is added per request in
/// the router. Use [`build_schema_with_mail`] to inject a real/test sender.
pub fn build_schema(
    repo: Arc<dyn Repository>,
    reported: Arc<dyn ReportedResultSource>,
) -> XpoolSchema {
    build_schema_with_mail(repo, reported, Arc::new(mail::NullSender))
}

/// Build the schema with an explicit [`mail::MailSender`] in schema data — the
/// production path (real sender) and the admin-mutation tests (capturing sender).
pub fn build_schema_with_mail(
    repo: Arc<dyn Repository>,
    reported: Arc<dyn ReportedResultSource>,
    mail: Arc<dyn mail::MailSender>,
) -> XpoolSchema {
    Schema::build(QueryRoot, MutationRoot, EmptySubscription)
        .data(repo)
        .data(reported)
        .data(mail)
        // Simple query-depth cap (API.md §2). Must stay above the standard
        // GraphQL introspection query (`__schema → types → fields → args →
        // type → ofType ×7`, depth ~13) or GraphiQL cannot load the schema.
        .limit_depth(20)
        .finish()
}

/// The id of the result-user player (the referral-graph root), or empty string
/// if none is configured. Used to gate pool creation (`may_create_pool`).
pub(crate) async fn result_user_id(repo: &dyn Repository) -> async_graphql::Result<String> {
    Ok(repo
        .list_players()
        .await?
        .into_iter()
        .find(|p| p.is_result_user)
        .map(|p| p.id)
        .unwrap_or_default())
}
