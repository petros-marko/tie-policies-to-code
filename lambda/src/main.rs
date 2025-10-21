mod api;
mod lambda;

use std::path::Path;
use std::{env, io};

use crate::lambda::LambdaClient;

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    let mut args = env::args().skip(1); // Skip program name

    let file_path = match args.next() {
        Some(path) => path,
        None => {
            eprintln!("Usage: your_program <path_to_file> <name_of_func>");
            std::process::exit(1);
        }
    };

    let function_name = match args.next() {
        Some(path) => path,
        None => {
            eprintln!("Usage: your_program <path_to_file> <name_of_func>");
            std::process::exit(1);
        }
    };

    let zip_path = Path::new(&file_path);
    let handler = "lambda_function.lambda_handler";
    let account_id = "000000000000";

    // Create AWS clients
    let rest_api = api::RestApiGateway::new().await.expect("aws api gateway");
    let lambda_client = LambdaClient::new().await.expect("aws lambda client");

    // Deploy/update Lambda function
    let function_arn = lambda_client
        .deploy_fn("test-rolename", &function_name, zip_path, handler)
        .await
        .expect("deploy lambda");

    // Create endpoint
    rest_api
        .create_endpoint("test_func", "GET", &function_arn)
        .await
        .expect("create endpoint");

    // Add Lambda permission for API Gateway to invoke
    lambda_client
        .add_lambda_permission_to_gateway(&function_name, rest_api.api_id(), account_id)
        .await
        .expect("add lambda permission");

    rest_api
        .print_api_endpoints()
        .await
        .expect("print endpoints");

    // test that it worked
    rest_api
        .http_get("test_func")
        .await
        .expect("test invoke endpoint");

    Ok(())
}

