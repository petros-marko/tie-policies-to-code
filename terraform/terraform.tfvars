# Multiple Lambda Functions Configuration

# Shared configuration
account_id        = "000000000000"
api_name          = "my-new-api-terraform"
s3_bucket_name    = "my-code-bucket-terraform-new"

# Multiple Lambda functions with individual policies
lambda_functions = [
  {
    name        = "my-test-func-terraform"
    code_path   = "./bootstrap.zip"
    s3_key      = "my-test-func"
    api_path    = "my_test_func"
    http_method = "GET"
    policy_document = "./policies/user-service-policy.json"
  }
]
