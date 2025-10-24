use std::fs;
use std::path::Path;

use aws_config::{BehaviorVersion, Region};
use aws_sdk_iam::Client as AwsIamClient;
use aws_sdk_lambda::Client as AwsLambdaClient;
use aws_sdk_lambda::config::Builder as LambdaBuilder;
use aws_sdk_lambda::types::{Environment, FunctionCode, Runtime};
use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::config::{Builder, Credentials, SharedCredentialsProvider};

pub struct LambdaClient {
    lambda_client: aws_sdk_lambda::Client,
    iam_client: aws_sdk_iam::Client,
}

impl LambdaClient {
    pub async fn new() -> Result<Self, aws_sdk_lambda::Error> {
        // testing creds that work with my localstack
        let creds = Credentials::new("test", "test", None, None, "test");
        let creds_provider = SharedCredentialsProvider::new(creds);
        let config = aws_config::SdkConfig::builder()
            .behavior_version(BehaviorVersion::latest())
            .endpoint_url("http://localhost:4566")
            .credentials_provider(creds_provider)
            .region(Region::new("us-east-1"))
            .build();

        // creating lambda + iam clients
        let lambda_config = LambdaBuilder::from(&config).build();
        let lambda_client = AwsLambdaClient::from_conf(lambda_config);
        let iam_client = AwsIamClient::new(&config);

        Ok(Self {
            lambda_client,
            iam_client,
        })
    }

    pub async fn deploy_fn(
        &self,
        role_name: &str,
        function_name: &str,
        zipped_code_path: &std::path::Path,
        policy_path: &std::path::Path,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let role_arn = self.create_or_get_lambda_role(role_name, policy_path).await?;

        let creds = Credentials::new("test", "test", None, None, "test");
        let creds_provider = SharedCredentialsProvider::new(creds);
        let config = aws_config::SdkConfig::builder()
            .behavior_version(BehaviorVersion::latest())
            .endpoint_url("http://localhost:4566")
            .credentials_provider(creds_provider)
            .region(Region::new("us-east-1"))
            .build();
        let s3_config = Builder::from(&config).force_path_style(true).build();
        let s3client = S3Client::from_conf(s3_config);

        // put code in s3, because lambda will error if code file too big
        let body =
            aws_sdk_s3::primitives::ByteStream::from_path(std::path::Path::new(zipped_code_path))
                .await;
        let s3_put_result = s3client
            .put_object()
            .bucket("mhanlon-test")
            .key("bootstrap2")
            .body(body.unwrap())
            .send()
            .await
            .map_err(aws_sdk_s3::Error::from);
        let _ = match s3_put_result {
            Err(err) => return Err(err.into()),
            Ok(object) => object,
        };

        let code = FunctionCode::builder()
            .s3_bucket("mhanlon-test")
            .s3_key("bootstrap2")
            .build();

        let create_function_result = self
            .lambda_client
            .create_function()
            .function_name(function_name)
            .role(role_arn)
            .handler("bootstrap")
            .timeout(30)
            .runtime(Runtime::Providedal2023)
            .code(code)
            .environment(
                Environment::builder()
                    .variables("AWS_LAMBDA_LOG_LEVEL", "DEBUG")
                    .build(),
            )
            .send()
            .await;

        let function_arn = match create_function_result {
            Ok(res) => res.function_arn().map(|s| s.to_string()),
            Err(err) => {
                let service_err = err.into_service_error();
                if !service_err.is_resource_conflict_exception() {
                    return Err(service_err.into());
                }
                println!("Function with given name already deployed, just updating code");
                self.lambda_client
                    .update_function_code()
                    .function_name(function_name)
                    .send()
                    .await?
                    .function_arn()
                    .and_then(|r| Some(r.to_string()))
            }
        };

        function_arn.ok_or_else(|| "Failed to get function ARN".into())
    }

    async fn create_or_get_lambda_role(
        &self,
        role_name: &str,
        policy_path: &Path
    ) -> Result<String, Box<dyn std::error::Error>> {
        let assume_role_policy = serde_json::json!({
            "Version": "2012-10-17",
            "Statement": [{
                "Effect": "Allow",
                "Principal": {
                    "Service": "lambda.amazonaws.com"
                },
                "Action": "sts:AssumeRole"
            }]
        });

        let policy_path_data = fs::read_to_string(policy_path)?;

        let create_role_result = self
            .iam_client
            .create_role()
            .role_name(role_name)
            .assume_role_policy_document(assume_role_policy.to_string())
            .send()
            .await;

        let role_arn = match create_role_result {
            Ok(res) => {
                // if created role successfully, attach execution policy and get arn
                let policy_arn = "arn:aws:iam::aws:policy/service-role/AWSLambdaBasicExecutionRole";
                self.iam_client
                    .attach_role_policy()
                    .role_name(role_name)
                    .policy_arn(policy_arn)
                    .send()
                    .await?;
                // also attach specific user defined policy
                println!("putting role policy");
                self.iam_client
                    .put_role_policy()
                    .role_name(role_name)
                    .policy_name("S3AccessPolicy")
                    .policy_document(policy_path_data)
                    .send()
                    .await?;
                res.role()
                    .and_then(|r| Some(r.arn()))
                    .map(|s| s.to_string())
            }
            Err(err) => {
                // otherwise, if failed bc role already existed, just return the arn from that
                let service_err = err.into_service_error();
                if !service_err.is_entity_already_exists_exception() {
                    return Err(service_err.into());
                }
                let existing_role = self
                    .iam_client
                    .get_role()
                    .role_name(role_name)
                    .send()
                    .await?;
                existing_role
                    .role()
                    .and_then(|r| Some(r.arn()))
                    .map(|s| s.to_string())
            }
        };

        Ok(role_arn.expect("role arn"))
    }

    // allow api gateway to call lambda
    pub async fn add_lambda_permission_to_gateway(
        &self,
        function_name: &str,
        api_id: &str,
        account_id: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let source_arn = format!(
            "arn:aws:execute-api:{}:{}:{}/*/*",
            "us-east-1", account_id, api_id
        );

        let _add_permission_result = self
            .lambda_client
            .add_permission()
            .function_name(function_name)
            .statement_id(random_string::generate(10, "1234567890"))
            .action("lambda:InvokeFunction")
            .principal("apigateway.amazonaws.com")
            .source_arn(source_arn)
            .send()
            .await?;

        Ok(())
    }
}
