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

variable "role_name" {
  description = "IAM Role name for Lambda function"
  type        = string
}

variable "function_name" {
  description = "Lambda function name"
  type        = string
}

variable "api_name" {
  description = "API Gateway name"
  type        = string
}

variable "api_path" {
  description = "API Gateway path"
  type        = string
}

variable "http_method" {
  description = "HTTP method for the API endpoint"
  type        = string
}

variable "s3_bucket_name" {
  description = "S3 bucket name for Lambda code"
  type        = string
}

variable "s3_key" {
  description = "S3 key for Lambda code"
  type        = string
}

variable "lambda_code_path" {
  description = "Local path to the Lambda function code zip file"
  type        = string
}

# S3 bucket for Lambda code
resource "aws_s3_bucket" "lambda_code_bucket" {
  bucket = var.s3_bucket_name 
}

# Upload Lambda code to S3
resource "aws_s3_object" "lambda_code" {
  bucket = aws_s3_bucket.lambda_code_bucket.id
  key    = var.s3_key
  source = var.lambda_code_path
  
  # This ensures the Lambda function is updated when the code changes
  etag = filemd5(var.lambda_code_path)
  
  # LocalStack specific configuration
  force_destroy = true
}

# IAM Role for Lambda function
resource "aws_iam_role" "lambda_role" {
  name = var.role_name

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

# AWS managed policy for basic Lambda execution
resource "aws_iam_role_policy_attachment" "lambda_basic_execution" {
  role       = aws_iam_role.lambda_role.name
  policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole"
}

# Custom S3 access policy for Lambda function
resource "aws_iam_role_policy" "s3_access_policy" {
  name = "S3AccessPolicy"
  role = aws_iam_role.lambda_role.id

  policy = jsonencode({
    Version = "2012-10-17"
    Statement = [
      {
        Sid    = "ExampleStmt"
        Action = [
          "s3:GetObject"
        ]
        Effect = "Allow"
        Resource = [
          "arn:aws:s3:::${var.s3_bucket_name}/*"
        ]
      }
    ]
  })
}

resource "aws_lambda_function" "main" {
  function_name = var.function_name
  role          = aws_iam_role.lambda_role.arn
  handler       = "bootstrap"
  runtime       = "provided.al2023"
  timeout       = 30

  # Code from S3 
  s3_bucket = var.s3_bucket_name
  s3_key    = var.s3_key

  environment {
    variables = {
      AWS_LAMBDA_LOG_LEVEL = "DEBUG"
    }
  }

  depends_on = [
    aws_iam_role_policy_attachment.lambda_basic_execution,
    aws_iam_role_policy.s3_access_policy,
    aws_s3_object.lambda_code
  ]
}

# API Gateway REST API 
resource "aws_api_gateway_rest_api" "main" {
  name = var.api_name
}

# API Gateway Resource
resource "aws_api_gateway_resource" "main" {
  rest_api_id = aws_api_gateway_rest_api.main.id
  parent_id   = aws_api_gateway_rest_api.main.root_resource_id
  path_part   = var.api_path
}

# API Gateway Method
resource "aws_api_gateway_method" "main" {
  rest_api_id   = aws_api_gateway_rest_api.main.id
  resource_id   = aws_api_gateway_resource.main.id
  http_method   = var.http_method
  authorization = "NONE"
}

# API Gateway Integration
resource "aws_api_gateway_integration" "main" {
  rest_api_id = aws_api_gateway_rest_api.main.id
  resource_id = aws_api_gateway_resource.main.id
  http_method = aws_api_gateway_method.main.http_method

  integration_http_method = "POST"
  type                   = "AWS_PROXY"
  uri                    = aws_lambda_function.main.invoke_arn
}

# API Gateway Deployment
resource "aws_api_gateway_deployment" "main" {
  depends_on = [
    aws_api_gateway_integration.main
  ]

  rest_api_id = aws_api_gateway_rest_api.main.id
  stage_name  = "$default"
}

# Lambda Permission for API Gateway
resource "aws_lambda_permission" "api_gateway" {
  statement_id  = "AllowExecutionFromAPIGateway"
  action        = "lambda:InvokeFunction"
  function_name = aws_lambda_function.main.function_name
  principal     = "apigateway.amazonaws.com"
  # two * are for any stage, and any HTTP method
  source_arn    = "${aws_api_gateway_rest_api.main.execution_arn}/*/*"
}

# Outputs
output "api_gateway_url" {
  description = "URL of the API Gateway"
  value       = "http://localhost:4566/restapis/${aws_api_gateway_rest_api.main.id}/$default/_user_request_/${var.api_path}"
}

output "lambda_function_arn" {
  description = "ARN of the Lambda function"
  value       = aws_lambda_function.main.arn
}

output "api_gateway_id" {
  description = "ID of the API Gateway"
  value       = aws_api_gateway_rest_api.main.id
}
