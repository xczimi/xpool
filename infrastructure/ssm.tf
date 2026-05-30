resource "aws_ssm_parameter" "thesportsdb_key" {
  name        = "/xpool/${var.environment}/thesportsdb-api-key"
  description = "TheSportsDB premium API key. Forward-provisioned per DEPLOYMENT.md §3/§7 (the Lambda role is granted SSM read); no code consumes it yet."
  type        = "SecureString"
  value       = "PLACEHOLDER-set-out-of-band"

  lifecycle {
    ignore_changes = [value]
  }
}
