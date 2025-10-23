mod api;
mod lambda;

use std::path::Path;
use std::{env, io};
use std::thread;
use std::time::Duration;

// next todos:
// need to deploy a iam policy along with the binary
// need to parse our policy structure to iam policy structure

#[tokio::main]
async fn main() -> Result<(), io::Error> {
    let account_id = "000000000000";
    let rolename = "test-rolename";
    let http_method = "GET";

    let mut args = env::args().skip(1); // skip program name

    let filepath = match args.next() {
        Some(f) => f,
        None => {
            eprintln!("Usage: cargo run <path_to_zipped_code> <name_of_func> <path>");
            std::process::exit(1);
        }
    };

    let function_name = match args.next() {
        Some(n) => n,
        None => {
            eprintln!("Usage: cargo run <path_to_zipped_code> <name_of_func> <path>");
            std::process::exit(1);
        }
    };

     let path = match args.next() {
        Some(p) => p,
        None => {
            eprintln!("Usage: cargo run <path_to_zipped_code> <name_of_func> <path>");
            std::process::exit(1);
        }
    };

    // to create a zip of some rust project binary: 'cargo lambda build --output-format=zip' 
    // output is in target/lambda/your_project/bootstrap.zip
    // more info here: https://www.cargo-lambda.info/guide/getting-started.html
    let zip_path = Path::new(&filepath);

    let rest_api = api::RestApiGateway::new().await.expect("aws api gateway");
    let lambda_client = lambda::LambdaClient::new().await.expect("aws lambda client");

    // Deploy Lambda function (or update if function already exists) 
    let function_arn = lambda_client
        .deploy_fn(rolename, &function_name, &zip_path)
        .await
        .expect("deploy lambda");

    // Create endpoint
    rest_api
        .create_endpoint(&path, &http_method, &function_arn)
        .await
        .expect("create endpoint");

    // Add permission for API Gateway to invoke Lambda
    lambda_client
        .add_lambda_permission_to_gateway(&function_name, rest_api.api_id(), account_id)
        .await
        .expect("add lambda permission");

    println!("Sleeping for 2 seconds so function definitely moves out of Pending state");
    thread::sleep(Duration::from_secs(2));
    println!("Finished sleeping");

    // test that it worked
    rest_api
        .http_get(&path)
        .await
        .expect("test invoke endpoint");

    Ok(())
}

