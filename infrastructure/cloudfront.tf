# Custom Origin Request Policy for the api_lambda cache behavior.
#
# Originally narrowed to dodge the Lambda OAC SigV4 body-signing bug. We've
# since dropped OAC (the bug had no clean workaround) and the URL is public.
# The narrow header set still makes sense — the API doesn't need viewer
# cookies, and forwarding fewer headers is cheaper. Kept as a custom policy
# in case we ever revisit.
resource "aws_cloudfront_origin_request_policy" "api_lambda" {
  name    = "xpool-${var.environment}-api-lambda"
  comment = "Minimal header set forwarded to the api_lambda Function URL origin."

  cookies_config {
    cookie_behavior = "none"
  }

  query_strings_config {
    query_string_behavior = "all"
  }

  headers_config {
    header_behavior = "whitelist"
    headers {
      items = ["content-type", "accept", "origin"]
    }
  }
}

locals {
  # The Function URL is https://<id>.lambda-url.<region>.on.aws/ — CloudFront
  # custom origins take a bare hostname.
  lambda_origin_host = trimsuffix(
    trimprefix(aws_lambda_function_url.api.function_url, "https://"),
    "/"
  )
}

module "cloudfront" {
  source  = "terraform-aws-modules/cloudfront/aws"
  version = "~> 4.0"

  aliases             = [var.domain_name]
  comment             = "xpool ${var.environment}"
  enabled             = true
  is_ipv6_enabled     = true
  price_class         = "PriceClass_100"
  wait_for_deployment = false

  create_origin_access_control = true
  origin_access_control = {
    # OAC name (the map key) must be account-global-unique, so it is
    # env-namespaced — dev and prod each own their own OAC. A bare "s3_spa"
    # collided across environments (409 OriginAccessControlAlreadyExists).
    "xpool-${var.environment}-spa" = {
      description      = "OAC for the xpool SPA bucket"
      origin_type      = "s3"
      signing_behavior = "always"
      signing_protocol = "sigv4"
    }
    # api_lambda OAC removed — see comment on the Function URL in lambda.tf.
    # Body-signing bug for POST requests had no clean workaround inside
    # CloudFront config; we moved request authorization into the Lambda code
    # path via a shared secret header (custom_header below).
  }

  origin = {
    s3_spa = {
      domain_name           = module.spa_bucket.s3_bucket_bucket_regional_domain_name
      origin_access_control = "xpool-${var.environment}-spa"
    }
    api_lambda = {
      domain_name = local.lambda_origin_host
      custom_origin_config = {
        http_port              = 80
        https_port             = 443
        origin_protocol_policy = "https-only"
        origin_ssl_protocols   = ["TLSv1.2"]
      }
      # CloudFront injects this on every origin request. The Lambda's
      # `cloudfront_auth` middleware (crates/api/src/cloudfront_auth.rs)
      # rejects requests without it — that's what keeps the public Function
      # URL from being usefully callable outside our distribution.
      #
      # NB: the module's key is `custom_header` (singular) — the plural
      # spelling is silently ignored without a plan diff because tofu
      # doesn't validate unknown keys in dynamic-map inputs.
      custom_header = [
        {
          name  = "X-CloudFront-Secret"
          value = random_password.cloudfront_secret.result
        },
      ]
    }
  }

  default_root_object = "index.html"

  default_cache_behavior = {
    target_origin_id       = "s3_spa"
    viewer_protocol_policy = "redirect-to-https"
    allowed_methods        = ["GET", "HEAD", "OPTIONS"]
    cached_methods         = ["GET", "HEAD"]
    use_forwarded_values   = false
    cache_policy_id        = "658327ea-f89d-4fab-a63d-7e88639e58f6" # AWS managed: CachingOptimized
  }

  ordered_cache_behavior = [
    {
      path_pattern             = "/api/*"
      target_origin_id         = "api_lambda"
      viewer_protocol_policy   = "redirect-to-https"
      allowed_methods          = ["GET", "HEAD", "OPTIONS", "PUT", "POST", "PATCH", "DELETE"]
      cached_methods           = ["GET", "HEAD"]
      use_forwarded_values     = false
      cache_policy_id          = "4135ea2d-6df8-44a3-9df3-4b5a84be39ad" # AWS managed: CachingDisabled
      origin_request_policy_id = aws_cloudfront_origin_request_policy.api_lambda.id

      # compress = false: CloudFront must not re-encode the response body, since
      # Lambda OAC computes the SHA256 of the request body for SigV4 and any
      # transformation between edge-signing and origin-receipt breaks the hash.
      # (The same concern is why we use a tight header whitelist below instead
      # of the managed AllViewerExceptHostHeader policy.)
      compress = false
    }
  ]

  # SPA fallback. The SPA bucket has block-public-acls = true, so S3 returns
  # 403 (not 404) for any key that doesn't exist. To make client-side routing
  # work when someone types /games directly, rewrite both 403 and 404 from
  # origin to /index.html. The trade-off: 403/404 from the api_lambda origin
  # ALSO get rewritten and viewers see /index.html instead of the API's
  # actual error response — keep this in mind when debugging API errors via
  # the viewer; check CloudWatch logs instead.
  custom_error_response = [
    { error_code = 403, response_code = 200, response_page_path = "/index.html", error_caching_min_ttl = 0 },
    { error_code = 404, response_code = 200, response_page_path = "/index.html", error_caching_min_ttl = 0 },
  ]

  viewer_certificate = {
    acm_certificate_arn      = module.acm.acm_certificate_arn
    ssl_support_method       = "sni-only"
    minimum_protocol_version = "TLSv1.2_2021"
  }
}
