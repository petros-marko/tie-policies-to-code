use aws_config::{BehaviorVersion, Region};
use aws_sdk_s3::{config::{Builder, Credentials, SharedCredentialsProvider}, Client as S3Client};
use lambda_http::{Body, Error, Request, RequestExt, Response};

pub(crate) async fn function_handler(event: Request) -> Result<Response<Body>, Error> {
    let who = event
        .query_string_parameters_ref()
        .and_then(|params| params.first("name"))
        .unwrap_or("world");

    // Grab test file contents from an S3 bucket
    let config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .load()
        .await;
    let client = S3Client::new(&config);
    let file = client
        .get_object()
        .bucket("mhanlon-test")
        .key("testfile")
        .send()
        .await
        .map_err(aws_sdk_s3::Error::from)?;
    let data = file.body.collect().await?;
    let data_str = String::from_utf8(data.into_bytes().to_vec())?;

    let message = format!(
        "Hello {who}, this is an AWS Lambda HTTP request. File contents: {}",
        data_str
    );

    let resp = Response::builder()
        .status(200)
        .header("content-type", "text/html")
        .body(message.into())
        .map_err(Box::new)?;
    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lambda_http::{Request, RequestExt};
    use std::collections::HashMap;

    #[tokio::test]
    async fn test_generic_http_handler() {
        let request = Request::default();

        let response = function_handler(request).await.unwrap();
        assert_eq!(response.status(), 200);

        let body_bytes = response.body().to_vec();
        let body_string = String::from_utf8(body_bytes).unwrap();

        assert_eq!(
            body_string,
            "Hello world, this is an AWS Lambda HTTP request"
        );
    }

    #[tokio::test]
    async fn test_http_handler_with_query_string() {
        let mut query_string_parameters: HashMap<String, String> = HashMap::new();
        query_string_parameters.insert("name".into(), "new-lambda-project".into());

        let request = Request::default().with_query_string_parameters(query_string_parameters);

        let response = function_handler(request).await.unwrap();
        assert_eq!(response.status(), 200);

        let body_bytes = response.body().to_vec();
        let body_string = String::from_utf8(body_bytes).unwrap();

        assert_eq!(
            body_string,
            "Hello new-lambda-project, this is an AWS Lambda HTTP request"
        );
    }
}
