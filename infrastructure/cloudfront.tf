locals {
  # The Function URL is https://<id>.lambda-url.<region>.on.aws/ — CloudFront
  # custom origins take a bare hostname.
  lambda_origin_host = replace(
    replace(aws_lambda_function_url.api.function_url, "https://", ""),
    "/", ""
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
    s3_spa = {
      description      = "OAC for the xpool SPA bucket"
      origin_type      = "s3"
      signing_behavior = "always"
      signing_protocol = "sigv4"
    }
  }

  origin = {
    s3_spa = {
      domain_name           = module.spa_bucket.s3_bucket_bucket_regional_domain_name
      origin_access_control = "s3_spa"
    }
    api_lambda = {
      domain_name = local.lambda_origin_host
      custom_origin_config = {
        http_port              = 80
        https_port             = 443
        origin_protocol_policy = "https-only"
        origin_ssl_protocols   = ["TLSv1.2"]
      }
    }
  }

  default_root_object = "index.html"

  default_cache_behavior = {
    target_origin_id       = "s3_spa"
    viewer_protocol_policy = "redirect-to-https"
    allowed_methods        = ["GET", "HEAD", "OPTIONS"]
    cached_methods         = ["GET", "HEAD"]
    use_forwarded_values   = false
    cache_policy_id        = "658327ea-f89d-4fab-a63d-7e88639e58f6"
  }

  ordered_cache_behavior = [
    {
      path_pattern           = "/api/*"
      target_origin_id       = "api_lambda"
      viewer_protocol_policy = "redirect-to-https"
      allowed_methods        = ["GET", "HEAD", "OPTIONS", "PUT", "POST", "PATCH", "DELETE"]
      cached_methods         = ["GET", "HEAD"]
      use_forwarded_values     = false
      cache_policy_id          = "4135ea2d-6df8-44a3-9df3-4b5a84be39ad"
      origin_request_policy_id = "b689b0a8-53d0-40ab-baf2-68738e2966ac"
    }
  ]

  custom_error_response = [
    { error_code = 403, response_code = 200, response_page_path = "/index.html" },
    { error_code = 404, response_code = 200, response_page_path = "/index.html" },
  ]

  viewer_certificate = {
    acm_certificate_arn      = module.acm.acm_certificate_arn
    ssl_support_method       = "sni-only"
    minimum_protocol_version = "TLSv1.2_2021"
  }
}
