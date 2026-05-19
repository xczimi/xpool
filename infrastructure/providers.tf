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
