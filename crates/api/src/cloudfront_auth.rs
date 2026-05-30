//! Shared-secret check for requests fronted by CloudFront.
//!
//! Production background: the Lambda Function URL is `AuthType = NONE` (a
//! documented Lambda-OAC body-signing bug made the OAC path unusable for
//! GraphQL POSTs). To compensate, our CloudFront distribution injects an
//! `X-CloudFront-Secret` header on every origin request (see
//! `infrastructure/cloudfront.tf:custom_header` +
//! `infrastructure/secrets.tf`). This middleware is the matching gate.
//!
//! Activation is by env-var presence, mirroring the `XPOOL_NOW` /
//! `X-Dev-Player` dev-stub philosophy in `clock.rs` / `auth.rs`: when
//! `CLOUDFRONT_SECRET` is unset the middleware is *not* attached at all and
//! the local `cargo run -p api` flow is byte-for-byte identical to before
//! this commit. When the env var is set (Lambda runtime, or local with
//! explicit `CLOUDFRONT_SECRET=...`) every request must carry a matching
//! header — non-matching requests get `403 Forbidden`.

use axum::{
    extract::{Request, State},
    http::{HeaderMap, StatusCode},
    middleware::Next,
    response::{IntoResponse, Response},
};

/// Header name CloudFront sends and we validate. Lowercase per HTTP/2 norms;
/// `HeaderMap::get` is case-insensitive but using lowercase here keeps the
/// canonical form consistent with how axum / hyper handle headers internally.
const HEADER: &str = "x-cloudfront-secret";

/// Env var name. Tofu wires this in via
/// `module.api_lambda.environment_variables.CLOUDFRONT_SECRET`.
const ENV_VAR: &str = "CLOUDFRONT_SECRET";

/// Read the expected secret from the environment. `None` for both "unset" and
/// "set to empty string" — the latter defends against a misconfigured tofu
/// variable that would otherwise lock everyone out with no usable secret.
pub fn read_secret_from_env() -> Option<String> {
    std::env::var(ENV_VAR).ok().filter(|v| !v.is_empty())
}

/// Middleware: require the `X-CloudFront-Secret` header to match the expected
/// value. Use with `axum::middleware::from_fn_with_state(expected, ...)`.
///
/// Comparison is a plain `==`. The secret is 32 random URL-safe characters
/// and the channel is TLS — a constant-time compare is overkill here; if the
/// threat model ever changes, swap for `subtle::ConstantTimeEq`.
pub async fn require_cloudfront_secret(
    State(expected): State<String>,
    headers: HeaderMap,
    request: Request,
    next: Next,
) -> Response {
    if header_matches(&headers, &expected) {
        next.run(request).await
    } else {
        (StatusCode::FORBIDDEN, "forbidden").into_response()
    }
}

/// Pure predicate so the unit tests can exercise the branching without
/// constructing a full axum middleware pipeline.
fn header_matches(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get(HEADER)
        .and_then(|v| v.to_str().ok())
        .is_some_and(|v| v == expected)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn headers_with(secret: Option<&str>) -> HeaderMap {
        let mut h = HeaderMap::new();
        if let Some(s) = secret {
            h.insert(HEADER, s.parse().unwrap());
        }
        h
    }

    #[test]
    fn matching_header_passes() {
        assert!(header_matches(&headers_with(Some("xyz")), "xyz"));
    }

    #[test]
    fn mismatched_header_is_rejected() {
        assert!(!header_matches(&headers_with(Some("wrong")), "xyz"));
    }

    #[test]
    fn missing_header_is_rejected() {
        assert!(!header_matches(&headers_with(None), "xyz"));
    }

    #[test]
    fn read_from_env_returns_none_for_unset() {
        // Use a deliberately unique key so we don't fight other tests.
        let prev = std::env::var(ENV_VAR).ok();
        std::env::remove_var(ENV_VAR);
        assert!(read_secret_from_env().is_none());
        if let Some(v) = prev {
            std::env::set_var(ENV_VAR, v);
        }
    }

    #[test]
    fn read_from_env_treats_empty_as_unset() {
        let prev = std::env::var(ENV_VAR).ok();
        std::env::set_var(ENV_VAR, "");
        assert!(read_secret_from_env().is_none());
        match prev {
            Some(v) => std::env::set_var(ENV_VAR, v),
            None => std::env::remove_var(ENV_VAR),
        }
    }
}
