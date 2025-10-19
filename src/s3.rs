use std::path::PathBuf;

use aws_sdk_s3::Client as S3Client;
use aws_sdk_s3::config::{Credentials, SharedCredentialsProvider, Builder};
use aws_config::{BehaviorVersion, Region};
use policy_macros;

pub async fn get_s3_client() -> S3Client {
    let creds = Credentials::new("test", "test", None, None, "test");
    let creds_provider = SharedCredentialsProvider::new(creds);
    let config = aws_config::SdkConfig::builder()
        .behavior_version(BehaviorVersion::latest())
        .endpoint_url("http://localhost:4566")
        .credentials_provider(creds_provider)
        .region(Region::new("us-east-1"))
        .build();
    let s3_config = Builder::from(&config).force_path_style(true).build();
    S3Client::from_conf(s3_config)
}

pub async fn create_bucket(
    client: &aws_sdk_s3::Client,
    bucket_name: &str,
) -> Result<Option<aws_sdk_s3::operation::create_bucket::CreateBucketOutput>, aws_sdk_s3::Error> {
    let create = client
        .create_bucket()
        .bucket(bucket_name)
        .send()
        .await;

    // its okay if BucketAlreadyExists or BucketAlreadyOwnedByYou, just return
    create.map(Some).or_else(|err| {
        if err
            .as_service_error()
            .map(|se| se.is_bucket_already_exists() || se.is_bucket_already_owned_by_you())
            == Some(true)
        {
            Ok(None)
        } else {
            Err(aws_sdk_s3::Error::from(err))
        }
    })
}

#[policy_macros::policy_attr(allow post mybucket)]
pub async fn upload_object(
    client: &aws_sdk_s3::Client,
    bucket_name: &str,
    file_name: &str,
    key: &str,
) -> Result<aws_sdk_s3::operation::put_object::PutObjectOutput, aws_sdk_s3::Error> {
    let body = aws_sdk_s3::primitives::ByteStream::from_path(std::path::Path::new(file_name)).await;
    client
        .put_object()
        .bucket(bucket_name)
        .key(key)
        .body(body.unwrap())
        .send()
        .await
        .map_err(aws_sdk_s3::Error::from)
}

#[policy_macros::policy_attr(allow get mybucket)]
pub async fn download_object(
    client: &aws_sdk_s3::Client,
    bucket_name: &str,
    key: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    let object = client
        .get_object()
        .bucket(bucket_name)
        .key(key)
        .send()
        .await
        .map_err(aws_sdk_s3::Error::from)?;

    // write file to ./output
    tokio::fs::create_dir_all("./s3_output").await?;
    let output_path = PathBuf::from("./s3_output").join(key);
    let mut file = tokio::fs::File::create(&output_path).await?;
    let mut body = object.body.into_async_read();
    tokio::io::copy(&mut body, &mut file).await?;

    println!("Downloaded to: {}", output_path.display());

    return Ok(())
}

pub async fn list_objects(client: &aws_sdk_s3::Client, bucket: &str) -> Result<(), aws_sdk_s3::Error> {
    let mut response = client
        .list_objects_v2()
        .bucket(bucket.to_owned())
        .max_keys(10) // In this example, go 10 at a time.
        .into_paginator()
        .send();

    while let Some(result) = response.next().await {
        match result {
            Ok(output) => {
                for object in output.contents() {
                    println!(" - {}", object.key().unwrap_or("Unknown"));
            }
            }
            Err(err) => {
                eprintln!("{err:?}")
            }
        }
    }

    Ok(())
}