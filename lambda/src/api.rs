use aws_config::{BehaviorVersion, Region};
use aws_sdk_apigateway::types::IntegrationType;
use aws_sdk_apigateway::Client as ApiGatewayClient;
use aws_sdk_s3::config::Credentials;
use aws_sdk_s3::config::SharedCredentialsProvider;
use aws_sdk_s3::error::BoxError;
use reqwest::Client as HTTPClient;

pub struct RestApiGateway {
    api_client: aws_sdk_apigateway::Client,
    api_id: String,
    root_resource_id: String,
    stage_name: String,
}

impl RestApiGateway {
    pub async fn new() -> Result<Self, BoxError> {
        // testing creds that work with my localstack
        let creds = Credentials::new("test", "test", None, None, "test");
        let creds_provider = SharedCredentialsProvider::new(creds);
        let config = aws_config::SdkConfig::builder()
            .behavior_version(BehaviorVersion::latest())
            .endpoint_url("http://localhost:4566")
            .credentials_provider(creds_provider)
            .region(Region::new("us-east-1"))
            .build();

        // create rest api with api gateway client
        let api_client = ApiGatewayClient::new(&config);
        let result = api_client
            .create_rest_api()
            .name("michcat-api-name-new")
            .send()
            .await?;

        Ok(Self {
            api_client,
            api_id: result.id().unwrap().to_string(),
            root_resource_id: result.root_resource_id().unwrap().to_string(),
            stage_name: "$default".to_string(),
        })
    }

    // creates resource (at 'path'), adds method 'http_method' on resource, and deploys it
    pub async fn create_endpoint(
        &self,
        path: &str,
        http_method: &str,
        function_arn: &str,
    ) -> Result<(), aws_sdk_apigateway::Error> {
        // create new AWS 'Resource'
        let create_resource_result = self
            .api_client
            .create_resource()
            .rest_api_id(self.api_id.clone())
            .parent_id(self.root_resource_id.clone())
            .path_part(path)
            .send()
            .await?;
        let resource_id = create_resource_result.id().unwrap().to_string();

        // put an http method on the resource
        let _ = self
            .api_client
            .put_method()
            .rest_api_id(self.api_id.clone())
            .resource_id(&resource_id)
            .http_method(http_method)
            .authorization_type("NONE")
            .send()
            .await?;

        // integration URI so api can connect to lambda
        let integration_uri = format!(
            "arn:aws:apigateway:us-east-1:lambda:path/2015-03-31/functions/{}/invocations",
            function_arn
        );
        let _ = self
            .api_client
            .put_integration()
            .rest_api_id(self.api_id.clone())
            .resource_id(&resource_id)
            .http_method("GET")
            .integration_http_method("POST")
            .r#type(IntegrationType::AwsProxy)
            .uri(&integration_uri)
            .send()
            .await?;

        // creates deployment resource, which makes api callable
        let _ = self
            .api_client
            .create_deployment()
            .rest_api_id(self.api_id.clone())
            .stage_name(self.stage_name.clone())
            .send()
            .await?;

        Ok(())
    }

    // get 'endpoint'
    pub async fn http_get(&self, endpoint: &str) -> Result<(), Box<dyn std::error::Error>> {
        let client = HTTPClient::new();

        // After creating deployment with stage "dev"
        let url = format!(
            "http://localhost:4566/restapis/{}/$default/_user_request_/{}",
            self.api_id, endpoint
        );

        let response = client
            .get(url)
            .query(&[("name", "tie-policies-to-code")]) 
            .header("Content-Type", "application/json")
            .header("headerName", "headerValue")
            .send()
            .await?;

        println!("Status: {}", response.status());
        println!("Body: {}", response.text().await?);

        // let endpoint_url = format!(
        //     "http://localhost:4566/restapis/{}/{}/_user_request_/{}",
        //     self.api_id, self.stage_name, endpoint
        // );

        // println!("endopint is {}", endpoint_url);

        // let client = reqwest::Client::new();
        // let response = client.get(&endpoint_url).json(&example_json).send().await?;

        // let status = response.status();
        // let body = response.text().await?;

        // println!("Response Status: {}", status);
        // println!("Response Body: {}", body);

        // let payload = fs::read_to_string("example.json")
        // .expect("Failed to read example.json");

        // // Configure AWS SDK to use LocalStack endpoint
        // let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
        // let config = aws_config::from_env()
        //     .region(region_provider)
        //     .endpoint_url("http://localhost:4566")
        //     .load()
        //     .await;

        // let client = Client::new(&config);

        // // Invoke the Lambda function
        // let resp = client
        //     .invoke()
        //     .function_name("michelina4") // Replace with your function name
        //     .payload(Blob::new(payload.as_bytes()))
        //     .send()
        //     .await?;

        // // Handle the response
        // if let Some(payload) = resp.payload() {
        //     let response_str = String::from_utf8_lossy(payload.as_ref());
        //     println!("Response: {}", response_str);
        // }

        // // if let Some(status_code) = resp.status_code() {
        // //     println!("Status code: {}", status_code);
        // // }

        // if let Some(function_error) = resp.function_error() {
        //     eprintln!("Function error: {}", function_error);
        // }

        Ok(())
    }

    // print all endpoints
    pub async fn print_api_endpoints(&self) -> Result<(), Box<dyn std::error::Error>> {
        let resp = self.api_client.get_rest_apis().send().await?;

        for api in resp.items() {
            println!("API name: {}", api.name().unwrap_or_default());

            let api_id = api.id().unwrap_or_default();
            let resources_resp = self
                .api_client
                .get_resources()
                .rest_api_id(api_id)
                .send()
                .await?;

            println!("Resources:");
            for resource in resources_resp.items() {
                println!("Path: {}", resource.path().unwrap_or_default());

                // Get methods for this resource
                if let Some(resource_methods) = resource.resource_methods() {
                    println!("Methods:");
                    for (method, _method_data) in resource_methods {
                        println!("- {}", method);
                    }
                }
            }
            println!()
        }

        Ok(())
    }

    // Getter for api_id
    pub fn api_id(&self) -> &str {
        &self.api_id
    }
}
