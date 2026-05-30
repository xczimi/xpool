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

  # Code is deployed out-of-band by bin/deploy-api (aws lambda update-function-code).
  # Tofu manages the function shell (role, env, URL, permissions); it does not
  # track or push the zip's contents. The local zip is still required on first
  # create (it seeds the initial code), but subsequent applies ignore hash drift.
  ignore_source_code_hash = true

  timeout     = 10
  memory_size = 256

  environment_variables = {
    XPOOL_TABLE           = module.dynamodb.dynamodb_table_id
    CURRENT_TOURNAMENT_ID = var.current_tournament_id
    RUST_LOG              = "info"
    # The API requires every request to carry `X-CloudFront-Secret: <this
    # value>` (enforced by crates/api/src/cloudfront_auth.rs). CloudFront
    # injects the header on every origin call via the `custom_header` block
    # in cloudfront.tf, so viewers never see or send the secret directly.
    CLOUDFRONT_SECRET = random_password.cloudfront_secret.result
  }

  attach_policy_statements = true
  policy_statements = {
    dynamodb = {
      effect = "Allow"
      actions = [
        "dynamodb:GetItem", "dynamodb:PutItem", "dynamodb:UpdateItem",
        "dynamodb:DeleteItem", "dynamodb:Query", "dynamodb:Scan",
        "dynamodb:BatchGetItem", "dynamodb:BatchWriteItem", "dynamodb:DescribeTable",
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

# Public-access grants for the Function URL.
#
# Why not Lambda OAC: CloudFront's OAC signs the SHA256 of POST bodies into
# the SigV4 signature, but Function URL recomputes a different hash on
# receipt and rejects the request with InvalidSignatureException. There's no
# clean workaround at the CloudFront config layer. So the URL is `AuthType =
# NONE` and the request-time check lives in the Lambda code (see
# `CLOUDFRONT_SECRET` env above + `crates/api/src/cloudfront_auth.rs`).
#
# Both `InvokeFunctionUrl` AND `InvokeFunction` are granted — the legacy
# auto-generated `FunctionURLAllowPublicAccess` statement only covers the
# former, and Function URL refuses requests without the latter for reasons
# that are not in the public docs.
resource "aws_lambda_permission" "api_function_url_public" {
  statement_id           = "AllowPublicInvokeFunctionUrl"
  function_name          = module.api_lambda.lambda_function_name
  action                 = "lambda:InvokeFunctionUrl"
  principal              = "*"
  function_url_auth_type = "NONE"
}

resource "aws_lambda_permission" "api_invoke_function_public" {
  statement_id  = "AllowPublicInvokeFunction"
  function_name = module.api_lambda.lambda_function_name
  action        = "lambda:InvokeFunction"
  principal     = "*"
  # No function_url_auth_type: that argument injects a
  # `lambda:FunctionUrlAuthType` condition which the AWS API only accepts on
  # `lambda:InvokeFunctionUrl` permissions, not on `lambda:InvokeFunction`.
}
