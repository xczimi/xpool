# Shared backend settings are literals here; only the per-environment `key`
# is supplied at init: tofu init -backend-config=env/<env>.backend.hcl
terraform {
  backend "s3" {
    bucket       = "xczimi-terraform-state"
    region       = "ca-central-1"
    use_lockfile = true
  }
}
