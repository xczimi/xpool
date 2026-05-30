output "cloudfront_domain" {
  description = "The CloudFront distribution domain name."
  value       = module.cloudfront.cloudfront_distribution_domain_name
}

output "site_url" {
  description = "The public URL for this environment."
  value       = "https://${var.domain_name}"
}

output "lambda_function_url" {
  description = "The raw Lambda Function URL (origin for /api/*)."
  value       = aws_lambda_function_url.api.function_url
}

output "spa_bucket" {
  description = "The S3 bucket name for SPA assets."
  value       = module.spa_bucket.s3_bucket_id
}

output "dynamodb_table" {
  description = "The DynamoDB table name."
  value       = module.dynamodb.dynamodb_table_id
}
