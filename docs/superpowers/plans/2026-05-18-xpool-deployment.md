# xpool Deployment Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Deploy xpool to AWS as two parameterized environments (`dev` at `pool-dev.xczimi.com`, `prod` at `pool.xczimi.com`) using OpenTofu, per [`.specs/DEPLOYMENT.md`](../../../.specs/DEPLOYMENT.md).

**Architecture:** One CloudFront distribution per environment with two origins — a private S3 bucket (SPA static assets, via OAC) and a Lambda Function URL (`/api/*` → the Rust axum GraphQL app). DynamoDB single-table backs the app; SSM holds the one secret; SES sends mail. `dev` and `prod` are one OpenTofu configuration instantiated twice, with separate state files and zero shared resources.

**Tech Stack:** Rust (axum, async-graphql, `lambda_http`), `cargo-lambda`, OpenTofu + `terraform-aws-modules`, AWS (CloudFront, S3, Lambda, DynamoDB, ACM, Route53, SSM, SES), React + Vite SPA.

**Spec:** [`.specs/DEPLOYMENT.md`](../../../.specs/DEPLOYMENT.md). Read it first. This plan deliberately implements a **subset/variant** — see "Scope & spec deviations" below.

---

## Scope & spec deviations

Decisions confirmed with the project owner that differ from or refine `DEPLOYMENT.md`:

1. **CI/CD deferred.** `DEPLOYMENT.md` §6 (GitHub Actions, GitHub OIDC IAM role) is **out of scope**. Deploys are run manually by the owner with the `xczimi` AWS profile. No `.github/workflows/`, no OIDC provider/role.
2. **Stack in `ca-central-1`.** All regional resources (DynamoDB, Lambda, S3, SSM) deploy to `ca-central-1`. CloudFront's ACM certificate *must* live in `us-east-1`, so a second `aws` provider aliased `us_east_1` is declared solely for the certificate (Task 10). CloudFront and Route53 are global.
3. **SES is external — referenced, not managed.** The SES domain identity for `xczimi.com` is owned by a separate account-wide repository (so it is shared by every app in the account, with production access handled there). This plan does **not** create or import it — it reads the identity through a `data` source purely to scope the Lambda's `ses:SendEmail` permission to its ARN. **Prerequisite:** that repo must have verified `xczimi.com` in `ca-central-1` before this stack can be `plan`/`apply`-ed. Separately, **no email-sending code exists in any crate today** (MailHog is in `docker-compose` but unused); wiring the app to actually send mail is an **unbuilt feature outside this plan** that intersects the auth workstream.
4. **TheSportsDB SSM parameter is provisioned but unconsumed.** `DEPLOYMENT.md` §7 says the Lambda reads the TheSportsDB key from SSM. No crate reads it today — `xtask import` is the only TheSportsDB consumer and it runs locally. The plan creates the SSM `SecureString` and grants the role `ssm:GetParameter` per spec, but the runtime wiring is a deliberate no-op until code needs it.
5. **`ensure_table()` skipped in `lambda` mode** (Task 2). The app currently self-creates the DynamoDB table on startup; the table is OpenTofu-managed and long-lived per spec, so the Lambda must not. This keeps the execution role data-plane-only (no `dynamodb:CreateTable`).
6. **SES production access is out of scope.** Whether the `xczimi.com` identity is in SES sandbox or production mode is owned by the separate account-wide SES repo (deviation #3), not this plan.

---

## File structure

All new infrastructure code lives under `infrastructure/` at the repo root:

```
infrastructure/
  versions.tf        # required_version, required_providers
  providers.tf       # the aws provider
  backend.tf         # empty s3 backend block (partial config)
  variables.tf       # environment, domain_name, current_tournament_id, route53_zone_name
  data.tf            # the Route53 hosted-zone data source
  dynamodb.tf        # the single table
  ssm.tf             # the TheSportsDB SecureString parameter
  ses.tf             # data lookup of the externally-managed SES identity
  lambda.tf          # the api Lambda function, Function URL, execution role
  s3.tf              # the private SPA-assets bucket
  acm.tf             # the DNS-validated certificate
  cloudfront.tf      # the distribution (two origins)
  route53.tf         # the alias record -> CloudFront
  outputs.tf         # cloudfront domain, function url, bucket id, table name
  env/
    dev.backend.hcl  # backend key for dev state
    dev.tfvars       # dev variable values
    prod.backend.hcl # backend key for prod state
    prod.tfvars      # prod variable values
```

Plus a 3-line change to `crates/api/src/main.rs` (Task 2) and a deploy runbook appended to this plan's repo docs (Task 16).

One Rust workspace file is touched; the rest is new IaC. Each task produces a self-contained commit.

---

## Phase 1 — Application readiness

### Task 1: Install `cargo-lambda` and build the Lambda artifact

**Files:** none (tooling + build verification).

- [ ] **Step 1: Install `cargo-lambda`**

Run: `cargo install cargo-lambda --locked`
Expected: finishes with `Installed package cargo-lambda ...`. Verify: `cargo lambda --version` prints a version.

- [ ] **Step 2: Build the Lambda artifact (arm64, zipped)**

Run from the repo root:
```bash
cargo lambda build -p api --release --arm64 --features lambda --output-format zip
```
Expected: SUCCESS. Produces `target/lambda/api/bootstrap.zip`.
Note: if the `api` crate's binary name is not `api`, the path is `target/lambda/<binary-name>/bootstrap.zip` — confirm with `ls target/lambda/`.

- [ ] **Step 3: Verify the artifact**

Run: `unzip -l target/lambda/api/bootstrap.zip`
Expected: the archive contains a single file named `bootstrap`.

- [ ] **Step 4: Commit**

No source changes — nothing to commit. Record the artifact path for Task 8: `target/lambda/api/bootstrap.zip`.

---

### Task 2: Skip `ensure_table()` in `lambda` mode

The DynamoDB table is OpenTofu-managed (Task 5) and long-lived. The Lambda must not create it — otherwise the execution role would need `dynamodb:CreateTable`.

**Files:**
- Modify: `crates/api/src/main.rs` (the `app()` function, ~lines 11-15)

- [ ] **Step 1: Read the current `app()` function**

Run: `sed -n '10,16p' crates/api/src/main.rs`
Expected: shows `app()` calling `repo.ensure_table().await?;` unconditionally.

- [ ] **Step 2: Gate the `ensure_table()` call on the non-lambda build**

Change the body of `app()` so the call is compiled out under `--features lambda`:
```rust
async fn app() -> anyhow::Result<axum::Router> {
    let repo = DynamoRepository::from_env().await?;
    // The deployed table is OpenTofu-managed (see infrastructure/dynamodb.tf).
    // Only the local/dev server self-creates it against DynamoDB Local.
    #[cfg(not(feature = "lambda"))]
    repo.ensure_table().await?;
    let repo: Arc<dyn Repository> = Arc::new(repo);
    Ok(api::build_app(repo, true))
}
```

- [ ] **Step 3: Verify the local build still calls it**

Run: `cargo build -p api`
Expected: SUCCESS, no `unused` warnings for `ensure_table`.

- [ ] **Step 4: Verify the lambda build compiles with the call removed**

Run: `cargo build -p api --features lambda`
Expected: SUCCESS.

- [ ] **Step 5: Run the workspace tests to confirm nothing regressed**

Run: `cargo test --workspace`
Expected: PASS (DynamoDB integration tests skip without `DYNAMO_TEST=1` — that is fine).

- [ ] **Step 6: Commit**

```bash
git add crates/api/src/main.rs
git commit -m "feat(api): skip ensure_table in lambda mode (table is IaC-managed)"
```

---

## Phase 2 — OpenTofu foundation

### Task 3: Scaffold the `infrastructure/` configuration

**Files:**
- Create: `infrastructure/versions.tf`, `providers.tf`, `backend.tf`, `variables.tf`, `data.tf`, `outputs.tf`
- Create: `infrastructure/env/dev.backend.hcl`, `dev.tfvars`, `prod.backend.hcl`, `prod.tfvars`

- [ ] **Step 1: Confirm the state bucket region**

Run: `aws s3api get-bucket-location --bucket xczimi-terraform-state --profile xczimi`
Expected: `ca-central-1`. This is the *state bucket's* region (the `region` field in the backend HCL below), independent of where resources deploy. If the bucket reports a different region, substitute it in both backend files.

- [ ] **Step 2: Write `infrastructure/versions.tf`**

```hcl
terraform {
  required_version = ">= 1.10"

  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.70"
    }
  }
}
```

- [ ] **Step 3: Write `infrastructure/providers.tf`**

```hcl
provider "aws" {
  region = "ca-central-1"

  default_tags {
    tags = {
      Project     = "xpool"
      Environment = var.environment
      ManagedBy   = "opentofu"
    }
  }
}

# CloudFront requires its ACM certificate in us-east-1.
provider "aws" {
  alias  = "us_east_1"
  region = "us-east-1"

  default_tags {
    tags = {
      Project     = "xpool"
      Environment = var.environment
      ManagedBy   = "opentofu"
    }
  }
}
```

- [ ] **Step 4: Write `infrastructure/backend.tf`**

```hcl
# Partial config — the key differs per environment.
# Initialise with: tofu init -backend-config=env/<env>.backend.hcl
terraform {
  backend "s3" {}
}
```

- [ ] **Step 5: Write `infrastructure/variables.tf`**

```hcl
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
  description = "Domain to verify as an SES sending identity."
  type        = string
  default     = "xczimi.com"
}
```

- [ ] **Step 6: Write `infrastructure/data.tf`**

```hcl
data "aws_route53_zone" "primary" {
  name         = var.route53_zone_name
  private_zone = false
}
```

- [ ] **Step 7: Write `infrastructure/outputs.tf`**

```hcl
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
```

- [ ] **Step 8: Write `infrastructure/env/dev.backend.hcl`**

```hcl
bucket       = "xczimi-terraform-state"
key          = "xpool/infrastructure/dev/terraform.tfstate"
region       = "ca-central-1"
use_lockfile = true
```

- [ ] **Step 9: Write `infrastructure/env/prod.backend.hcl`**

```hcl
bucket       = "xczimi-terraform-state"
key          = "xpool/infrastructure/prod/terraform.tfstate"
region       = "ca-central-1"
use_lockfile = true
```

- [ ] **Step 10: Write `infrastructure/env/dev.tfvars`**

```hcl
environment           = "dev"
domain_name           = "pool-dev.xczimi.com"
route53_zone_name     = "xczimi.com"
current_tournament_id = "fwc26"
```

- [ ] **Step 11: Write `infrastructure/env/prod.tfvars`**

```hcl
environment           = "prod"
domain_name           = "pool.xczimi.com"
route53_zone_name     = "xczimi.com"
current_tournament_id = "fwc26"
```

- [ ] **Step 12: Commit**

```bash
git add infrastructure/
git commit -m "chore(infra): scaffold OpenTofu config and per-env settings"
```

---

### Task 4: Initialise OpenTofu for `dev`

**Files:** none (creates `.terraform/` — must be gitignored).

- [ ] **Step 1: Add OpenTofu working files to `.gitignore`**

Append to the repo-root `.gitignore`:
```
# OpenTofu
infrastructure/.terraform/
infrastructure/.terraform.lock.hcl
*.tfstate
*.tfstate.*
```
Note: keep `.terraform.lock.hcl` ignored only if you do not want to pin provider hashes; for a solo manual-deploy project this is acceptable.

- [ ] **Step 2: Initialise the dev backend**

Run from `infrastructure/`:
```bash
AWS_PROFILE=xczimi tofu init -backend-config=env/dev.backend.hcl
```
Expected: `OpenTofu has been successfully initialized!`. The `aws` provider downloads.

- [ ] **Step 3: Validate the configuration**

Run: `tofu validate`
Expected: `Success! The configuration is valid.` (Resources do not exist yet — that is fine; `validate` is static.)

- [ ] **Step 4: Commit**

```bash
git add .gitignore
git commit -m "chore(infra): gitignore OpenTofu working files"
```

---

## Phase 3 — Resource definitions

Each task in this phase writes one `.tf` file and ends with `tofu validate`. **No `tofu apply` runs until Phase 4** — the resources are interdependent, so a full `tofu plan` is deferred to Task 13.

### Task 5: DynamoDB table

**Files:**
- Create: `infrastructure/dynamodb.tf`

- [ ] **Step 1: Write `infrastructure/dynamodb.tf`**

The table schema must match `crates/storage/src/dynamo.rs`: hash key `pk` (S), range key `sk` (S), on-demand billing.

```hcl
module "dynamodb" {
  source  = "terraform-aws-modules/dynamodb-table/aws"
  version = "~> 4.0"

  name         = "xpool-${var.environment}"
  hash_key     = "pk"
  range_key    = "sk"
  billing_mode = "PAY_PER_REQUEST"

  attributes = [
    { name = "pk", type = "S" },
    { name = "sk", type = "S" },
  ]

  ttl_enabled        = true
  ttl_attribute_name = "ttl"

  deletion_protection_enabled = true
}
```

- [ ] **Step 2: Validate**

Run: `tofu validate`
Expected: `Success!`. If the module reports an unknown argument, check its version docs and adjust — then re-validate.

- [ ] **Step 3: Commit**

```bash
git add infrastructure/dynamodb.tf
git commit -m "feat(infra): DynamoDB single table"
```

---

### Task 6: SSM parameter for the TheSportsDB key

**Files:**
- Create: `infrastructure/ssm.tf`

- [ ] **Step 1: Write `infrastructure/ssm.tf`**

Created with a placeholder value; the real key is set out-of-band (Step 3) and `ignore_changes` keeps OpenTofu from reverting it.

```hcl
resource "aws_ssm_parameter" "thesportsdb_key" {
  name        = "/xpool/${var.environment}/thesportsdb-api-key"
  description = "TheSportsDB premium API key (consumed by xtask import, not the runtime Lambda)."
  type        = "SecureString"
  value       = "PLACEHOLDER-set-out-of-band"

  lifecycle {
    ignore_changes = [value]
  }
}
```

- [ ] **Step 2: Validate**

Run: `tofu validate`
Expected: `Success!`

- [ ] **Step 3: Note for the runbook**

After the first apply, set the real value once:
```bash
AWS_PROFILE=xczimi aws ssm put-parameter \
  --name /xpool/dev/thesportsdb-api-key --type SecureString \
  --value '<REAL-KEY>' --overwrite --region ca-central-1
```

- [ ] **Step 4: Commit**

```bash
git add infrastructure/ssm.tf
git commit -m "feat(infra): SSM SecureString for the TheSportsDB key"
```

---

### Task 7: SES identity reference (data source)

The SES domain identity for `xczimi.com` is managed in a separate account-wide
repository (deviation #3). This task only *references* it, to scope the Lambda's
`ses:SendEmail` permission (Task 8).

**Files:**
- Create: `infrastructure/ses.tf`

**Prerequisite:** the external SES repo must have verified `xczimi.com` in
`ca-central-1`. If it has not, `tofu plan` (Task 13) fails resolving this data
source — apply the SES repo first, or temporarily widen the Task 8 `ses`
statement to `resources = ["*"]` and defer this task.

- [ ] **Step 1: Write `infrastructure/ses.tf`**

```hcl
# The SES domain identity is managed in a separate account-wide repo.
# Referenced here only to scope the Lambda's send permission to its ARN.
data "aws_ses_domain_identity" "sending" {
  domain = var.ses_domain
}
```

- [ ] **Step 2: Validate**

Run: `tofu validate`
Expected: `Success!` (`validate` is static and does not hit AWS — the identity's existence is resolved at `plan` time, Task 13).

- [ ] **Step 3: Commit**

```bash
git add infrastructure/ses.tf
git commit -m "feat(infra): reference the externally-managed SES identity"
```

---

### Task 8: Lambda function, Function URL, and execution role

**Files:**
- Create: `infrastructure/lambda.tf`

- [ ] **Step 1: Write `infrastructure/lambda.tf`**

Uses the community `lambda` module for the function + role, and a raw `aws_lambda_function_url` (auth `NONE`, per spec §1).

```hcl
module "api_lambda" {
  source  = "terraform-aws-modules/lambda/aws"
  version = "~> 7.0"

  function_name = "xpool-api-${var.environment}"
  description   = "xpool GraphQL API (axum + async-graphql)."

  handler       = "bootstrap"
  runtime       = "provided.al2023"
  architectures = ["arm64"]

  create_package         = false
  local_existing_package = var.lambda_package_path

  timeout     = 10
  memory_size = 256

  environment_variables = {
    XPOOL_TABLE           = module.dynamodb.dynamodb_table_id
    CURRENT_TOURNAMENT_ID = var.current_tournament_id
    RUST_LOG              = "info"
  }

  attach_policy_statements = true
  policy_statements = {
    dynamodb = {
      effect    = "Allow"
      actions   = [
        "dynamodb:GetItem",
        "dynamodb:PutItem",
        "dynamodb:UpdateItem",
        "dynamodb:DeleteItem",
        "dynamodb:Query",
        "dynamodb:Scan",
        "dynamodb:BatchGetItem",
        "dynamodb:BatchWriteItem",
        "dynamodb:DescribeTable",
      ]
      resources = [module.dynamodb.dynamodb_table_arn]
    }
    ssm = {
      effect    = "Allow"
      actions   = ["ssm:GetParameter"]
      resources = [aws_ssm_parameter.thesportsdb_key.arn]
    }
    ses = {
      effect    = "Allow"
      actions   = ["ses:SendEmail", "ses:SendRawEmail"]
      resources = [data.aws_ses_domain_identity.sending.arn]
    }
  }

  cloudwatch_logs_retention_in_days = 14
}

resource "aws_lambda_function_url" "api" {
  function_name      = module.api_lambda.lambda_function_name
  authorization_type = "NONE"
}
```

- [ ] **Step 2: Validate**

Run: `tofu validate`
Expected: `Success!`. If the `lambda` module rejects `attach_policy_statements`/`policy_statements`, confirm the argument names against the `~> 7.0` module docs and adjust.

- [ ] **Step 3: Commit**

```bash
git add infrastructure/lambda.tf
git commit -m "feat(infra): api Lambda, Function URL, execution role"
```

---

### Task 9: S3 bucket for SPA assets

**Files:**
- Create: `infrastructure/s3.tf`

- [ ] **Step 1: Write `infrastructure/s3.tf`**

Private bucket; the bucket policy granting CloudFront OAC read access is attached in Task 11 (it needs the distribution ARN). For now, create the bucket fully locked down.

```hcl
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

data "aws_caller_identity" "current" {}
```

- [ ] **Step 2: Validate**

Run: `tofu validate`
Expected: `Success!`

- [ ] **Step 3: Commit**

```bash
git add infrastructure/s3.tf
git commit -m "feat(infra): private S3 bucket for SPA assets"
```

---

### Task 10: ACM certificate

**Files:**
- Create: `infrastructure/acm.tf`

- [ ] **Step 1: Write `infrastructure/acm.tf`**

DNS-validated certificate for the environment hostname, created in `us-east-1` via the aliased provider — CloudFront requires its certificate there.

```hcl
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
```

- [ ] **Step 2: Validate**

Run: `tofu validate`
Expected: `Success!`

- [ ] **Step 3: Commit**

```bash
git add infrastructure/acm.tf
git commit -m "feat(infra): DNS-validated ACM certificate"
```

---

### Task 11: CloudFront distribution

**Files:**
- Create: `infrastructure/cloudfront.tf`

- [ ] **Step 1: Write `infrastructure/cloudfront.tf`**

Two origins: the S3 bucket (via a CloudFront-managed Origin Access Control) and the Lambda Function URL (as a custom origin). Default behavior serves the SPA from S3; an ordered behavior routes `/api/*` to the Lambda. The module also emits the S3 bucket policy that grants OAC read.

```hcl
locals {
  # The Function URL is https://<id>.lambda-url.<region>.on.aws/ — CloudFront
  # custom origins take a bare hostname.
  lambda_origin_host = replace(
    replace(aws_lambda_function_url.api.function_url, "https://", ""),
    "/", ""
  )
}

module "cloudfront" {
  source  = "terraform-aws-modules/cloudfront/aws"
  version = "~> 3.4"

  aliases             = [var.domain_name]
  comment             = "xpool ${var.environment}"
  enabled             = true
  is_ipv6_enabled     = true
  price_class         = "PriceClass_100"
  wait_for_deployment = false

  create_origin_access_control = true
  origin_access_control = {
    s3_spa = {
      description      = "OAC for the xpool SPA bucket"
      origin_type      = "s3"
      signing_behavior = "always"
      signing_protocol = "sigv4"
    }
  }

  origin = {
    s3_spa = {
      domain_name           = module.spa_bucket.s3_bucket_bucket_regional_domain_name
      origin_access_control = "s3_spa"
    }
    api_lambda = {
      domain_name = local.lambda_origin_host
      custom_origin_config = {
        http_port              = 80
        https_port             = 443
        origin_protocol_policy = "https-only"
        origin_ssl_protocols   = ["TLSv1.2"]
      }
    }
  }

  default_root_object = "index.html"

  default_cache_behavior = {
    target_origin_id       = "s3_spa"
    viewer_protocol_policy = "redirect-to-https"
    allowed_methods        = ["GET", "HEAD", "OPTIONS"]
    cached_methods         = ["GET", "HEAD"]
    use_forwarded_values   = false
    # AWS managed policy: CachingOptimized
    cache_policy_id        = "658327ea-f89d-4fab-a63d-7e88639e58f6"
  }

  ordered_cache_behavior = [
    {
      path_pattern           = "/api/*"
      target_origin_id       = "api_lambda"
      viewer_protocol_policy = "redirect-to-https"
      allowed_methods        = ["GET", "HEAD", "OPTIONS", "PUT", "POST", "PATCH", "DELETE"]
      cached_methods         = ["GET", "HEAD"]
      use_forwarded_values   = false
      # AWS managed policies: CachingDisabled + AllViewerExceptHostHeader
      cache_policy_id          = "4135ea2d-6df8-44a3-9df3-4b5a84be39ad"
      origin_request_policy_id = "b689b0a8-53d0-40ab-baf2-68738e2966ac"
    }
  ]

  # SPA client-side routing: serve index.html for unknown paths.
  custom_error_response = [
    { error_code = 403, response_code = 200, response_page_path = "/index.html" },
    { error_code = 404, response_code = 200, response_page_path = "/index.html" },
  ]

  viewer_certificate = {
    acm_certificate_arn      = module.acm.acm_certificate_arn
    ssl_support_method       = "sni-only"
    minimum_protocol_version = "TLSv1.2_2021"
  }
}
```

- [ ] **Step 2: Add the S3 bucket policy granting CloudFront OAC read**

Append to `infrastructure/s3.tf` (it needs the distribution ARN, available now):
```hcl
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
```

- [ ] **Step 3: Validate**

Run: `tofu validate`
Expected: `Success!`. If the cloudfront module rejects an argument (origin/behavior schemas vary by module version), reconcile against the `~> 3.4` module docs and adjust — then re-validate.

- [ ] **Step 4: Commit**

```bash
git add infrastructure/cloudfront.tf infrastructure/s3.tf
git commit -m "feat(infra): CloudFront distribution with S3 + Lambda origins"
```

---

### Task 12: Route53 alias record

**Files:**
- Create: `infrastructure/route53.tf`

- [ ] **Step 1: Write `infrastructure/route53.tf`**

```hcl
resource "aws_route53_record" "site" {
  zone_id = data.aws_route53_zone.primary.zone_id
  name    = var.domain_name
  type    = "A"

  alias {
    name                   = module.cloudfront.cloudfront_distribution_domain_name
    zone_id                = module.cloudfront.cloudfront_distribution_hosted_zone_id
    evaluate_target_health = false
  }
}

resource "aws_route53_record" "site_ipv6" {
  zone_id = data.aws_route53_zone.primary.zone_id
  name    = var.domain_name
  type    = "AAAA"

  alias {
    name                   = module.cloudfront.cloudfront_distribution_domain_name
    zone_id                = module.cloudfront.cloudfront_distribution_hosted_zone_id
    evaluate_target_health = false
  }
}
```

- [ ] **Step 2: Validate**

Run: `tofu validate`
Expected: `Success!`

- [ ] **Step 3: Commit**

```bash
git add infrastructure/route53.tf
git commit -m "feat(infra): Route53 alias records to CloudFront"
```

---

## Phase 4 — Deploy and verify

### Task 13: Apply the `dev` stack

**Files:** none (creates AWS resources + `terraform.tfstate` in S3).

- [ ] **Step 1: Re-confirm the Lambda artifact exists**

Run: `ls -la target/lambda/api/bootstrap.zip`
Expected: the file exists (rebuild via Task 1 Step 2 if not).

- [ ] **Step 2: Plan the full dev stack**

Run from `infrastructure/`:
```bash
AWS_PROFILE=xczimi tofu plan -var-file=env/dev.tfvars -out=dev.tfplan
```
Expected: a plan creating ~all resources, **0 to destroy**. Review the resource list. ACM DNS validation and CloudFront creation are the slow parts.

- [ ] **Step 3: Apply**

Run: `AWS_PROFILE=xczimi tofu apply dev.tfplan`
Expected: `Apply complete!`. CloudFront + ACM validation can take 5-15 minutes. Note the outputs (`cloudfront_domain`, `lambda_function_url`, `spa_bucket`, `dynamodb_table`).

- [ ] **Step 4: Set the real TheSportsDB key** (see Task 6 Step 3)

- [ ] **Step 5: Commit** — no source change; `dev.tfplan` must not be committed (add `infrastructure/*.tfplan` to `.gitignore` if not already covered).

---

### Task 14: Build and deploy the SPA

**Files:** none.

- [ ] **Step 1: Build the SPA**

Run from `web/`:
```bash
npm ci
npm run build
```
Expected: produces `web/dist/` containing `index.html` and `assets/`.

- [ ] **Step 2: Sync to S3**

Run (substitute the bucket name from Task 13's `spa_bucket` output):
```bash
AWS_PROFILE=xczimi aws s3 sync web/dist/ "s3://<spa_bucket>/" --delete --region ca-central-1
```
Expected: uploads all built assets.

- [ ] **Step 3: Invalidate the CloudFront cache**

```bash
AWS_PROFILE=xczimi aws cloudfront create-invalidation \
  --distribution-id <distribution-id> --paths "/*"
```
Find `<distribution-id>` via: `tofu output` then `aws cloudfront list-distributions`, or add a `cloudfront_distribution_id` output to `outputs.tf`.

- [ ] **Step 4: Add the distribution-id output** (quality-of-life for future deploys)

Append to `infrastructure/outputs.tf`:
```hcl
output "cloudfront_distribution_id" {
  description = "CloudFront distribution id (for cache invalidation)."
  value       = module.cloudfront.cloudfront_distribution_id
}
```
Run `tofu apply -var-file=env/dev.tfvars` to register the new output (no infra change). Commit:
```bash
git add infrastructure/outputs.tf
git commit -m "chore(infra): expose CloudFront distribution id"
```

---

### Task 15: Seed the `dev` DynamoDB table

**Files:** none.

- [ ] **Step 1: Import the tournament data into the dev table**

Run (no `DYNAMO_ENDPOINT` — this targets real AWS; `XPOOL_TABLE` selects the dev table):
```bash
AWS_PROFILE=xczimi XPOOL_TABLE=xpool-dev AWS_REGION=ca-central-1 \
  cargo run -p xtask -- import fwc26.json
```
Expected: import succeeds. (Confirm the JSON path — it is `fwc26.json` at the repo root, or `tournaments/fwc26.json`; adjust to whichever exists.)

- [ ] **Step 2: Seed demo data**

```bash
AWS_PROFILE=xczimi XPOOL_TABLE=xpool-dev AWS_REGION=ca-central-1 \
  cargo run -p xtask -- seed
```
Expected: creates the result-user, demo players, and a pool.

- [ ] **Step 3: Verify the table has items**

```bash
AWS_PROFILE=xczimi aws dynamodb scan --table-name xpool-dev \
  --select COUNT --region ca-central-1
```
Expected: a non-zero `Count`.

---

### Task 16: Smoke-test `pool-dev.xczimi.com`

**Files:**
- Create: a "Deploy runbook" section appended to `.specs/DEPLOYMENT.md` (the spec's home for deployment ops) — or, if the doc-creation hook objects, add it as a `## 9. Deploy runbook` section inside the existing `DEPLOYMENT.md`.

- [ ] **Step 1: Verify the GraphQL API responds**

```bash
curl -sS https://pool-dev.xczimi.com/api/graphql \
  -H 'content-type: application/json' \
  -d '{"query":"{ __typename }"}'
```
Expected: `{"data":{"__typename":"Query"}}` (HTTP 200).

- [ ] **Step 2: Verify the SPA loads**

```bash
curl -sS -o /dev/null -w '%{http_code}\n' https://pool-dev.xczimi.com/
```
Expected: `200`. Open the URL in a browser — the SPA renders and a query against `/api/graphql` succeeds (check the network tab).

- [ ] **Step 3: Verify SPA deep-link routing**

```bash
curl -sS -o /dev/null -w '%{http_code}\n' https://pool-dev.xczimi.com/some/spa/route
```
Expected: `200` (the 403/404 → `index.html` custom error response works).

- [ ] **Step 4: Document the deploy runbook**

Add a `## 9. Deploy runbook` section to `.specs/DEPLOYMENT.md` capturing the exact commands from Tasks 13-15: `tofu init`/`plan`/`apply`, the SPA build + `s3 sync` + invalidation, and the `xtask` seed commands. Commit:
```bash
git add .specs/DEPLOYMENT.md
git commit -m "docs: add the deploy runbook to DEPLOYMENT.md"
```

---

### Task 17: Promote to `prod`

**Files:** none (re-uses the configuration with prod settings).

- [ ] **Step 1: Initialise the prod backend**

Run from `infrastructure/`:
```bash
AWS_PROFILE=xczimi tofu init -reconfigure -backend-config=env/prod.backend.hcl
```
Expected: re-initialised against the prod state key.

- [ ] **Step 2: Plan and apply prod**

```bash
AWS_PROFILE=xczimi tofu plan -var-file=env/prod.tfvars -out=prod.tfplan
AWS_PROFILE=xczimi tofu apply prod.tfplan
```
Expected: creates the prod stack at `pool.xczimi.com`. Both environments reference the same external SES identity (Task 7); nothing SES-related is created.

- [ ] **Step 3: Deploy the SPA and seed** — repeat Tasks 14 and 15 with `XPOOL_TABLE=xpool-prod` and the prod bucket / distribution id.

- [ ] **Step 4: Smoke-test `pool.xczimi.com`** — repeat Task 16 steps 1-3 against the prod hostname.

---

## Self-review notes

- **Spec coverage:** Topology (§1) — Tasks 8-12. Environments (§2) — dev in Phase 4, prod in Task 17. AWS resources (§3) — S3 (9), CloudFront (11), Lambda (8), DynamoDB (5), SSM (6), IAM (8); SES referenced via a data source, not managed here (7). Local dev (§4) — unchanged, not in scope. OpenTofu + modules + remote state + locking (§5) — Tasks 3-4. CI/CD (§6) — **deliberately deferred** (see Scope & spec deviations #1). Secrets/config (§7) — SSM (6), Lambda env vars (8). Cost posture (§8) — on-demand DynamoDB, `PriceClass_100`, Function URL (no API Gateway), all preserved.
- **Known execution risks:** `terraform-aws-modules` argument schemas vary by version — every Phase 3 task ends with `tofu validate` and Task 13 gates on a reviewed `tofu plan` to catch drift before any resource is created. The CloudFront module (Task 11) is the most schema-sensitive.
- **Cross-workstream handoff:** the parallel auth workstream needs only the prod/dev hostnames (now fixed: `pool.xczimi.com`, `pool-dev.xczimi.com`) for its IdP callback URLs. The "where is auth enforced" question is already answered by spec §1 — in-app, no gateway authorizer.
