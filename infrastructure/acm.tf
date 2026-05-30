module "acm" {
  source  = "terraform-aws-modules/acm/aws"
  version = "~> 5.1"

  # CloudFront requires the certificate in us-east-1.
  providers = {
    aws = aws.us_east_1
  }

  domain_name = var.domain_name
  zone_id     = data.aws_route53_zone.primary.zone_id

  validation_method   = "DNS"
  wait_for_validation = true
}
