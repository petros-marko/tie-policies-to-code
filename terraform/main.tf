terraform {
  required_providers {
    aws = {
      source  = "hashicorp/aws"
      version = "~> 5.0"
    }
  }
}

# Configuration specific for LocalStack
provider "aws" {
  access_key                  = "test"
  secret_key                  = "test"
  region                      = "us-east-1"
  skip_credentials_validation = true
  skip_metadata_api_check     = true
  skip_requesting_account_id  = true

  endpoints {
    apigateway     = "http://localhost:4566"
    iam            = "http://localhost:4566"
    lambda         = "http://localhost:4566"
    s3             = "http://localhost:4566"
  }

  s3_use_path_style = true
}

# Configuration vars that should be defined in terraform.tfvars
variable "account_id" {
  description = "AWS Account ID"
  type        = string
  default     = "000000000000"
}

variable "api_name" {
  description = "API Gateway name"
  type        = string
}

variable "s3_bucket_name" {
  description = "S3 bucket name for Lambda code"
  type        = string
}

variable "lambda_functions" {
  description = "List of Lambda functions to deploy"
  type = list(object({
    name        = string
    code_path   = string
    s3_key      = string
    api_path    = string
    http_method = string
    policy_document = string  # JSON policy document (can be inline JSON or file path)
  }))
}

# S3 bucket for Lambda code
resource "aws_s3_bucket" "lambda_code_bucket" {
  bucket = var.s3_bucket_name
}

# API Gateway REST API
resource "aws_api_gateway_rest_api" "main" {
  name = var.api_name
}

# Upload multiple Lambda code files to S3
resource "aws_s3_object" "lambda_code" {
  for_each = { for func in var.lambda_functions : func.name => func }
  
  bucket = aws_s3_bucket.lambda_code_bucket.id
  key    = each.value.s3_key
  source = each.value.code_path
  
  # to detect code changes
  etag = filemd5(each.value.code_path)
}

# IAM roles for each Lambda function
resource "aws_iam_role" "function_roles" {
  for_each = { for func in var.lambda_functions : func.name => func }
  
  name = "${each.value.name}-role"

  assume_role_policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Action = "sts:AssumeRole"
        Effect = "Allow"
        Principal = {
          Service = "lambda.amazonaws.com"
        }
      }
    ]
  })
}

# Attach basic execution policy to each function role
resource "aws_iam_role_policy_attachment" "function_basic_execution" {
  for_each = { for func in var.lambda_functions : func.name => func }
  
  role       = aws_iam_role.function_roles[each.key].name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
}

# create indiviual custom policies for each function (from the policy file)
resource "aws_iam_role_policy" "function_policies" {
  for_each = { for func in var.lambda_functions : func.name => func }
  
  name = "${each.value.name}-policy"
  role = aws_iam_role.function_roles[each.key].id

  policy = fileexists(each.value.policy_document) ? file(each.value.policy_document) : each.value.policy_document
}

resource "aws_lambda_function" "functions" {
  for_each = { for func in var.lambda_functions : func.name => func }
  
  function_name = each.value.name
  role          = aws_iam_role.function_roles[each.key].arn
  handler       = "bootstrap"
  runtime       = "provided.al2023"
  timeout       = 30

  s3_bucket = var.s3_bucket_name
  s3_key    = each.value.s3_key

  environment {
    variables = {
      AWS_LAMBDA_LOG_LEVEL = "DEBUG"
    }
  }

  depends_on = [
    aws_iam_role_policy_attachment.function_basic_execution,
    aws_iam_role_policy.function_policies,
    aws_s3_object.lambda_code
  ]
}

# API Gateway resources
resource "aws_api_gateway_resource" "function_resources" {
  for_each = { for func in var.lambda_functions : func.name => func }
  
  rest_api_id = aws_api_gateway_rest_api.main.id
  parent_id   = aws_api_gateway_rest_api.main.root_resource_id
  path_part   = each.value.api_path
}

# API Gateway methods
resource "aws_api_gateway_method" "function_methods" {
  for_each = { for func in var.lambda_functions : func.name => func }
  
  rest_api_id   = aws_api_gateway_rest_api.main.id
  resource_id   = aws_api_gateway_resource.function_resources[each.key].id
  http_method   = each.value.http_method
  authorization = "NONE"
}

# API Gateway integrations
resource "aws_api_gateway_integration" "function_integrations" {
  for_each = { for func in var.lambda_functions : func.name => func }
  
  rest_api_id = aws_api_gateway_rest_api.main.id
  resource_id = aws_api_gateway_resource.function_resources[each.key].id
  http_method = aws_api_gateway_method.function_methods[each.key].http_method

  integration_http_method = "POST"
  type                   = "AWS_PROXY"
  uri                    = aws_lambda_function.functions[each.key].invoke_arn
}

# Lambda permissions
resource "aws_lambda_permission" "function_permissions" {
  for_each = { for func in var.lambda_functions : func.name => func }
  
  statement_id  = "AllowExecutionFromAPIGateway-${each.key}"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.functions[each.key].function_name
  principal     = "apigateway.amazonaws.com"
  source_arn    = "${aws_api_gateway_rest_api.main.execution_arn}/*/*"
}

# Update deployment to depend on all integrations
resource "aws_api_gateway_deployment" "main" {
  depends_on = [
    aws_api_gateway_integration.function_integrations
  ]

  rest_api_id = aws_api_gateway_rest_api.main.id
  stage_name  = "$default"
}

# Outputs
output "function_urls" {
  description = "URLs for all Lambda functions"
  value = {
    for func in var.lambda_functions : func.name => 
    "http://localhost:4566/restapis/${aws_api_gateway_rest_api.main.id}/$default/_user_request_/${func.api_path}"
  }
}

output "function_arns" {
  description = "ARNs of all Lambda functions"
  value = {
    for func in var.lambda_functions : func.name => 
    aws_lambda_function.functions[func.name].arn
  }
}

output "api_gateway_id" {
  description = "ID of the API Gateway"
  value       = aws_api_gateway_rest_api.main.id
}
