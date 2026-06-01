variable "environment" {
  description = "Deployment environment name (dev | prod)."
  type        = string
}

variable "domain_name" {
  description = "Public hostname for this environment."
  type        = string
}

variable "route53_zone_name" {
  description = "The Route53 hosted zone the domain lives under."
  type        = string
}

variable "current_tournament_id" {
  description = "Tournament namespace passed to the app as CURRENT_TOURNAMENT_ID."
  type        = string
  default     = "fwc26"
}

variable "lambda_package_path" {
  description = "Path to the cargo-lambda zip artifact, relative to infrastructure/."
  type        = string
  default     = "../target/lambda/api/bootstrap.zip"
}

variable "ses_domain" {
  description = "SES sending domain, looked up via a data source; the identity is managed in a separate account-wide repo."
  type        = string
  default     = "xczimi.com"
}

variable "auth0_domain" {
  description = "Auth0 tenant domain (e.g. xpool.us.auth0.com). Empty disables the Auth0 issuer in the API's trust list. Populate per-env in tfvars once the Auth0 tenant exists (see docs/runbooks/auth0-setup.md)."
  type        = string
  default     = ""
}

variable "auth0_audience" {
  description = "Auth0 API audience identifier the SPA requests and the API validates."
  type        = string
  default     = "xpool-api"
}
