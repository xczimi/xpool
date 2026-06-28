# Scheduled deadline-reminder Lambda (the reminder heartbeat) + its two
# EventBridge triggers. Code is shipped out-of-band by bin/deploy-reminder
# (like the api Lambda); tofu manages the function shell + schedules only.
#
# Activation gate (plan R1): the Lambda *shell* is always created, but the two
# automated triggers are gated behind var.reminder_enabled (default false). So a
# routine `bin/deploy [dev|prod]` (which runs `infra` by default) provisions the
# function WITHOUT arming the email heartbeat — apply, then `aws lambda invoke`
# it by hand to validate, and only flip reminder_enabled = true once the R1 gate
# is met. This makes "merge ≠ activate" an enforced invariant, not a promise.

variable "reminder_lambda_package_path" {
  description = "Path to the reminder cargo-lambda zip artifact, relative to infrastructure/."
  type        = string
  default     = "../target/lambda/reminder/bootstrap.zip"
}

variable "reminder_last_call_schedule" {
  description = "EventBridge rate/cron for the last-call reminder rule (every 30 minutes; R2 slot+slack)."
  type        = string
  default     = "rate(30 minutes)"
}

variable "reminder_digest_schedule" {
  description = "EventBridge Scheduler cron for the daily matchday digest."
  type        = string
  default     = "cron(0 0 * * ? *)"
}

variable "reminder_digest_timezone" {
  description = "Named timezone for the daily digest (DST-aware; never a hard-coded offset)."
  type        = string
  default     = "America/Los_Angeles"
}

variable "mail_from" {
  description = "Verified From: address for reminder emails (must be on var.ses_domain). Reuses Auth0's monitored pool@ sender."
  type        = string
  default     = "pool@xczimi.com"
}

variable "reminder_reply_to" {
  description = "Reply-To for reminder emails (the opt-out destination). Empty -> the mail crate falls back to the From address; set to repoint replies without a code change."
  type        = string
  default     = ""
}

variable "reminder_enabled" {
  description = "Arm the two automated EventBridge triggers (R1 activation gate). false (default) provisions only the Lambda shell — no scheduled sends. Set true per-env in tfvars once the R1 gate is met."
  type        = bool
  default     = false
}

module "reminder_lambda" {
  source  = "terraform-aws-modules/lambda/aws"
  version = "~> 7.0"

  function_name = "xpool-reminder-${var.environment}"
  description   = "xpool deadline-reminder sweep (EventBridge-driven)."

  handler       = "bootstrap"
  runtime       = "provided.al2023"
  architectures = ["arm64"]

  create_package         = false
  local_existing_package = var.reminder_lambda_package_path

  # Code shipped out-of-band by bin/deploy-reminder; ignore zip hash drift.
  ignore_source_code_hash = true

  # The digest sweep scans every player; give it more headroom than the api.
  timeout     = 60
  memory_size = 256

  environment_variables = {
    XPOOL_TABLE           = module.dynamodb.dynamodb_table_id
    CURRENT_TOURNAMENT_ID = var.current_tournament_id
    RUST_LOG              = "info"
    # No DYNAMO_ENDPOINT and no MAIL_TRANSPORT -> build_sender_from_env picks SES.
    MAIL_FROM = var.mail_from
    # Empty -> the mail crate falls back to From (see crates/mail transport.rs).
    MAIL_REPLY_TO = var.reminder_reply_to
    # Deep links in reminder emails must point at the deployed SPA, not the
    # localhost default the mail crate uses when this is unset. Mirrors the api
    # Lambda's XPOOL_PUBLIC_ORIGIN.
    XPOOL_PUBLIC_ORIGIN = "https://${var.domain_name}"
  }

  attach_policy_statements = true
  policy_statements = {
    dynamodb = {
      effect = "Allow"
      # The reminder sweep is read + marker-write only: GetItem/Query/Scan to
      # resolve players/identities/tournament, PutItem for dedup markers. No
      # UpdateItem/BatchGetItem (the storage crate never calls them) and no
      # DeleteItem/BatchWriteItem — narrower than the api Lambda by design.
      actions = [
        "dynamodb:GetItem", "dynamodb:PutItem",
        "dynamodb:Query", "dynamodb:Scan",
        "dynamodb:DescribeTable",
      ]
      resources = [module.dynamodb.dynamodb_table_arn]
    }
    ses = {
      effect    = "Allow"
      actions   = ["ses:SendEmail", "ses:SendRawEmail"]
      resources = [data.aws_ses_domain_identity.sending.arn]
    }
  }

  cloudwatch_logs_retention_in_days = 14
}

# ── Trigger A: last-call every 30 minutes (EventBridge Rules) ────────────────
# Gated by var.reminder_enabled (R1): no rule/target/permission until armed.
resource "aws_cloudwatch_event_rule" "reminder_last_call" {
  count               = var.reminder_enabled ? 1 : 0
  name                = "xpool-reminder-last-call-${var.environment}"
  description         = "Last-call deadline reminder sweep (every 30 minutes)."
  schedule_expression = var.reminder_last_call_schedule
}

resource "aws_cloudwatch_event_target" "reminder_last_call" {
  count = var.reminder_enabled ? 1 : 0
  rule  = aws_cloudwatch_event_rule.reminder_last_call[0].name
  arn   = module.reminder_lambda.lambda_function_arn
  input = jsonencode({ mode = "last_call" })
}

resource "aws_lambda_permission" "reminder_last_call" {
  count         = var.reminder_enabled ? 1 : 0
  statement_id  = "AllowEventBridgeLastCall"
  action        = "lambda:InvokeFunction"
  function_name = module.reminder_lambda.lambda_function_name
  principal     = "events.amazonaws.com"
  source_arn    = aws_cloudwatch_event_rule.reminder_last_call[0].arn
}

# ── Trigger B: daily matchday digest (EventBridge Scheduler, LA timezone) ────
# Why 00:00 America/Los_Angeles: midnight LA sits a few hours before the
# earliest North-American kickoff, so the digest always lands before that day's
# deadlines regardless of the recipient's timezone. A named TZ is DST-aware
# (PDT during the tournament) — never a hard-coded UTC offset.
resource "aws_iam_role" "reminder_scheduler" {
  count = var.reminder_enabled ? 1 : 0
  name  = "xpool-reminder-scheduler-${var.environment}"
  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect    = "Allow"
      Principal = { Service = "scheduler.amazonaws.com" }
      Action    = "sts:AssumeRole"
    }]
  })
}

resource "aws_iam_role_policy" "reminder_scheduler_invoke" {
  count = var.reminder_enabled ? 1 : 0
  name  = "invoke-reminder-lambda"
  role  = aws_iam_role.reminder_scheduler[0].id
  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [{
      Effect   = "Allow"
      Action   = "lambda:InvokeFunction"
      Resource = module.reminder_lambda.lambda_function_arn
    }]
  })
}

resource "aws_scheduler_schedule" "reminder_digest" {
  count = var.reminder_enabled ? 1 : 0
  name  = "xpool-reminder-digest-${var.environment}"

  flexible_time_window {
    mode = "OFF"
  }

  schedule_expression          = var.reminder_digest_schedule
  schedule_expression_timezone = var.reminder_digest_timezone

  target {
    arn      = module.reminder_lambda.lambda_function_arn
    role_arn = aws_iam_role.reminder_scheduler[0].arn
    input    = jsonencode({ mode = "digest" })
  }
}
