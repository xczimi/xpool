module "spa_bucket" {
  source  = "terraform-aws-modules/s3-bucket/aws"
  version = "~> 4.1"

  bucket = "xpool-spa-${var.environment}-${data.aws_caller_identity.current.account_id}"

  block_public_acls       = true
  block_public_policy     = true
  ignore_public_acls      = true
  restrict_public_buckets = true

  versioning = {
    enabled = false
  }
}

data "aws_iam_policy_document" "spa_bucket_oac" {
  statement {
    actions   = ["s3:GetObject"]
    resources = ["${module.spa_bucket.s3_bucket_arn}/*"]

    principals {
      type        = "Service"
      identifiers = ["cloudfront.amazonaws.com"]
    }

    condition {
      test     = "StringEquals"
      variable = "AWS:SourceArn"
      values   = [module.cloudfront.cloudfront_distribution_arn]
    }
  }
}

resource "aws_s3_bucket_policy" "spa_bucket_oac" {
  bucket = module.spa_bucket.s3_bucket_id
  policy = data.aws_iam_policy_document.spa_bucket_oac.json
}
