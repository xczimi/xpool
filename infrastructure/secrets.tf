# Shared secret CloudFront injects on every request to the api_lambda origin
# and the Lambda's `cloudfront_auth` middleware requires on every request.
#
# Why: Lambda OAC + POST has a documented body-signing bug we can't work
# around inside CloudFront config. We dropped OAC and made the Function URL
# public; this header is the poor-man's auth that ensures only requests
# flowing through *our* CloudFront distribution actually do anything.
#
# `random_password` is stateful — the value persists in tofu state across
# applies and only rotates if the resource is destroyed. State is in S3 with
# encryption + `use_lockfile = true`, so the secret is at rest there.
#
# Visibility: anyone with `tofu state show` access (i.e., write access to the
# state bucket) can read it. For a dev environment with one operator that's
# acceptable; for prod we'd want to rotate periodically.
resource "random_password" "cloudfront_secret" {
  length  = 32
  special = false
}
