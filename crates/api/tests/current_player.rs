use api::auth::{CurrentPlayer, VerifiedIdentity};
use async_graphql::{Context, EmptyMutation, EmptySubscription, Object, Schema};

struct Q;
#[Object]
impl Q {
    async fn require_player_id(&self, ctx: &Context<'_>) -> async_graphql::Result<String> {
        Ok(CurrentPlayer::require(ctx)?.id.clone())
    }
}

#[tokio::test]
async fn unclaimed_is_not_a_player() {
    let schema = Schema::new(Q, EmptyMutation, EmptySubscription);
    let viewer = CurrentPlayer::AuthenticatedUnclaimed(VerifiedIdentity {
        connection: "email".into(),
        sub: "auth0|abc".into(),
        verified_email: Some("x@example.com".into()),
        verified_phone: None,
    });
    let res = schema
        .execute(async_graphql::Request::new("{ requirePlayerId }").data(viewer))
        .await;
    assert!(!res.errors.is_empty(), "unclaimed must fail require()");
}
