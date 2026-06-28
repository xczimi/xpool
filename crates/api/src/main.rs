//! Local axum server entrypoint (`API.md` §1).
//!
//! By default this runs a plain HTTP server on `127.0.0.1:3000`. Building with
//! `--features lambda` swaps the entrypoint to `lambda_http::run`, wrapping the
//! same axum router for AWS Lambda behind API Gateway.

use std::sync::Arc;
use storage::{DynamoRepository, Repository};

/// Build the repository and the axum app shared by both entrypoints.
async fn app() -> anyhow::Result<axum::Router> {
    let repo = DynamoRepository::from_env().await?;
    // The deployed table is OpenTofu-managed (see infrastructure/dynamodb.tf).
    // Only the local/dev server self-creates it against DynamoDB Local.
    #[cfg(not(feature = "lambda"))]
    repo.ensure_table().await?;
    let repo: Arc<dyn Repository> = Arc::new(repo);
    // CLOUDFRONT_SECRET present (e.g. running on Lambda where tofu wires it
    // in) → the X-CloudFront-Secret header is required on every request.
    // Absent (typical `cargo run -p api`) → the middleware is not attached
    // and the local stack stays open the same as it always was.
    let cloudfront_secret = api::cloudfront_auth::read_secret_from_env();
    let mail = mail::build_sender_from_env().await?;
    Ok(api::build_app(repo, true, cloudfront_secret, mail))
}

#[cfg(not(feature = "lambda"))]
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // In dev/test builds, load `.env` from the workspace root before anything
    // else reads env vars. Silently no-ops when the file doesn't exist (common
    // in containers / CI). Lambda builds (--features lambda,
    // --no-default-features) skip this — env vars come from the
    // Terraform-managed Lambda env.
    #[cfg(feature = "dev_auth")]
    let _ = dotenvy::dotenv();

    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let app = app().await?;
    // Dev and the Playwright e2e stack run the same binary on different ports
    // (dev :3000, e2e :3001) so they can coexist — see `api::listen`.
    let addr = api::listen::listen_addr(std::env::var("XPOOL_PORT").ok().as_deref());
    let listener = tokio::net::TcpListener::bind(&addr).await?;
    tracing::info!("xpool api listening on http://{addr}");
    axum::serve(listener, app).await?;
    Ok(())
}

#[cfg(feature = "lambda")]
#[tokio::main]
async fn main() -> Result<(), lambda_http::Error> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .json()
        .init();

    let app = app()
        .await
        .map_err(|e| lambda_http::Error::from(e.to_string()))?;
    lambda_http::run(app).await
}
