# The SES domain identity is managed in a separate account-wide repo.
# Referenced here only to scope the Lambda's send permission to its ARN.
data "aws_ses_domain_identity" "sending" {
  domain = var.ses_domain
}
