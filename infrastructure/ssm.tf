resource "aws_ssm_parameter" "thesportsdb_key" {
  name        = "/xpool/${var.environment}/thesportsdb-api-key"
  description = "TheSportsDB premium API key. Injected into the Lambda env (THESPORTSDB_API_KEY) at deploy time via the data source in lambda.tf; the api reads it to back the reportedResults query."
  type        = "SecureString"
  value       = "PLACEHOLDER-set-out-of-band"

  lifecycle {
    ignore_changes = [value]
  }
}
