use crate::{data_model::AppState, util::create_dynamo_table};
use aws_config::{BehaviorVersion, Region};
use aws_sdk_dynamodb::{
    Client,
    config::{Credentials, SharedCredentialsProvider},
};
use axum::{
    self, Router,
    routing::{delete, get, post, put},
};
use std::sync::Arc;
mod conversation_handlers;
mod data_model;
mod friendship_handlers;
mod profile_handlers;
mod util;
mod auth;

#[tokio::main]
async fn main() {
    // create dynamodb client
    let creds = Credentials::new("test", "test", None, None, "test");
    let creds_provider = SharedCredentialsProvider::new(creds);
    let config = aws_config::SdkConfig::builder()
        .behavior_version(BehaviorVersion::latest())
        .endpoint_url("http://localhost:4566")
        .credentials_provider(creds_provider)
        .region(Region::new("us-east-1"))
        .build();
    let db = Client::new(&config);

    // create dynamodb tables
    create_dynamo_table(db.clone(), String::from("Users")).await.unwrap();
    create_dynamo_table(db.clone(), String::from("Messages")).await.unwrap();

    // create app state with db and token map
    let jwks = auth::fetch_jwks().await.unwrap();

    let state = Arc::new(AppState {
        db,
        user_table_name: "Users".to_string(),
        message_table_name: "Messages".to_string(),
        jwks: Arc::new(jwks),
    });

    // build application routes
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/profile/{user_id}", post(profile_handlers::create_profile))
        .route("/profile/{user_id}", put(profile_handlers::update_profile))
        .route("/profile/{user_id}", get(profile_handlers::get_profile))
        .route(
            "/friendship/{user_id}",
            post(friendship_handlers::create_friendship),
        )
        .route(
            "/friendship/{user_id}",
            delete(friendship_handlers::delete_friendship),
        )
        .route(
            "/friendship/{user_id}/accept",
            post(friendship_handlers::accept_friendship),
        )
        .route(
            "/conversation_with/{user_id}",
            post(conversation_handlers::send_message),
        )
        .route(
            "/conversation_with/{user_id}",
            get(conversation_handlers::get_conversation),
        )
        .route(
            "/conversation_with/{user_id}/last",
            get(conversation_handlers::get_latest_message),
        )
        .with_state(state);

    // run app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
