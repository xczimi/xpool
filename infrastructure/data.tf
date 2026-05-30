data "aws_route53_zone" "primary" {
  name         = var.route53_zone_name
  private_zone = false
}

data "aws_caller_identity" "current" {}
