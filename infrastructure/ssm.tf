resource "aws_ssm_parameter" "thesportsdb_key" {
  name        = "/xpool/${var.environment}/thesportsdb-api-key"
  description = "TheSportsDB premium API key (consumed by xtask import, not the runtime Lambda)."
  type        = "SecureString"
  value       = "PLACEHOLDER-set-out-of-band"

  lifecycle {
    ignore_changes = [value]
  }
}
