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
