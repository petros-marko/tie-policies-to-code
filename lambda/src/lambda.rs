use aws_config::{BehaviorVersion, Region};
use aws_sdk_iam::Client as AwsIamClient;
use aws_sdk_lambda::Client as AwsLambdaClient;
use aws_sdk_lambda::config::Builder as LambdaBuilder;
use aws_sdk_lambda::types::{FunctionCode, Runtime};
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::config::SharedCredentialsProvider;

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
        handler: &str,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let role_arn = self.create_or_get_lambda_role(role_name).await?;

        let code_bytes = std::fs::read(zipped_code_path)?;
        let code = FunctionCode::builder()
            .zip_file(code_bytes.clone().into())
            .build();

        let create_function_result = self
            .lambda_client
            .create_function()
            .function_name(function_name)
            .role(role_arn)
            .handler(handler)
            .runtime(Runtime::Python39)
            .code(code)
            .send()
            .await?;

        let function_arn = create_function_result.function_arn().unwrap_or_default();

        Ok(function_arn.to_string())
    }

    async fn create_or_get_lambda_role(
        &self,
        role_name: &str,
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

