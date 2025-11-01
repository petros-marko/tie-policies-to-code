use aws_config::{BehaviorVersion, Region};
use aws_sdk_dynamodb::{
    Client,
    config::{Credentials, SharedCredentialsProvider},
};
use axum::{
    self, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, post, put},
};
use std::sync::Arc;

use crate::data_model::{
    CreationResult, DeletionResult, Profile, UpdateProfileRequest, UpdateResult,
};
mod data_model;
mod util;

struct AppState {
    db: Client,
    user_table_name: String,
    message_table_name: String,
}

async fn get_conversation_handler(
    Path(user_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // This should be pulled out of the request
    let my_user_id = "123";
    let messages =
        util::get_conversation(&state.db, &state.message_table_name, my_user_id, &user_id).await;
    match messages {
        Ok(messages) => Json(messages).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error fetching conversation: {}", err),
        )
            .into_response(),
    }
}

async fn create_profile(
    Path(user_id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(profile): Json<Profile>,
) -> impl IntoResponse {
    // This should be pulled out of the request
    let my_user_id = "123";
    if my_user_id == user_id {
        let result =
            util::create_profile(&state.db, &state.user_table_name, &user_id, profile).await;
        match result {
            Ok(CreationResult::Success) => (
                StatusCode::CREATED,
                Json(serde_json::json!({ "status" : "ok" })),
            )
                .into_response(),
            Ok(CreationResult::Conflict) => (
                StatusCode::CONFLICT,
                Json(serde_json::json!({ "error" : "User already exists"})),
            )
                .into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error creating profile: {err}"),
            )
                .into_response(),
        }
    } else {
        (
            StatusCode::UNAUTHORIZED,
            "Users can only create profiles for themselves",
        )
            .into_response()
    }
}

async fn update_profile(
    Path(user_id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(update_profile_request): Json<UpdateProfileRequest>,
) -> impl IntoResponse {
    // This should be pulled out of the request
    let my_user_id = "123";
    if my_user_id == user_id {
        match util::update_profile(
            &state.db,
            &state.user_table_name,
            &user_id,
            update_profile_request,
        )
        .await
        {
            Ok(UpdateResult::Success(profile)) => (StatusCode::OK, Json(profile)).into_response(),
            Ok(UpdateResult::EmptyUpdate) => {
                (StatusCode::BAD_REQUEST, "No fields to update provided").into_response()
            }
            Ok(UpdateResult::NotFound) => (
                StatusCode::NOT_FOUND,
                format!("Profile for user {user_id} not found"),
            )
                .into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error updating profile: {err}"),
            )
                .into_response(),
        }
    } else {
        (
            StatusCode::UNAUTHORIZED,
            "Users can only update their own profiles",
        )
            .into_response()
    }
}

async fn get_profile(
    Path(user_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // This should be pulled out of the request
    let my_user_id = "123";
    match util::users_are_friends_or_identical(
        &state.db,
        &state.user_table_name,
        my_user_id,
        &user_id,
    )
    .await
    {
        Ok(true) => {
            let profile = util::get_profile(&state.db, &state.user_table_name, &user_id).await;
            match profile {
                Ok(profile) => match profile {
                    Some(profile) => Json(profile).into_response(),
                    None => (
                        StatusCode::NOT_FOUND,
                        format!("Profile for user {user_id} not found"),
                    )
                        .into_response(),
                },
                Err(err) => (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    format!("Error retrieving user profile: {}", err),
                )
                    .into_response(),
            }
        }
        Ok(false) => (
            StatusCode::UNAUTHORIZED,
            "Users can only get the profile information of themselves and their friends",
        )
            .into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error determining user friendship: {}", err),
        )
            .into_response(),
    }
}

async fn create_friendship(
    Path(user_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // This should be pulled out of the request
    let my_user_id = "123";
    match util::create_friendship(&state.db, &state.user_table_name, my_user_id, &user_id).await {
        Ok(CreationResult::Success) => (
            StatusCode::CREATED,
            Json(serde_json::json!({ "status" : "ok" })),
        )
            .into_response(),
        Ok(CreationResult::Conflict) => (
            StatusCode::CONFLICT,
            Json(serde_json::json!({ "error" : "Friend request already exists"})),
        )
            .into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error creating friend request: {err}"),
        )
            .into_response(),
    }
}

async fn accept_friendship(
    Path(user_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // This should be pulled out of the request
    let my_user_id = "123";
    match util::accept_friendship(&state.db, &state.user_table_name, my_user_id, &user_id).await {
        Ok(UpdateResult::Success(friendship)) => (StatusCode::OK, Json(friendship)).into_response(),
        Ok(UpdateResult::EmptyUpdate) => (
            StatusCode::BAD_REQUEST,
            format!("You are already friends with {user_id}"),
        )
            .into_response(),
        Ok(UpdateResult::NotFound) => (
            StatusCode::NOT_FOUND,
            format!("Friend request from user {user_id} not found"),
        )
            .into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error accepting friend request: {err}"),
        )
            .into_response(),
    }
}

async fn delete_friendship(
    Path(user_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // This should be pulled out of the request
    let my_user_id = "123";
    match util::delete_friendship(&state.db, &state.user_table_name, my_user_id, &user_id).await {
        Ok(DeletionResult::Success) => (
            StatusCode::NO_CONTENT,
            Json(serde_json::json!({ "status" : "deleted" })),
        )
            .into_response(),
        Ok(DeletionResult::NotFound) => (
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error" : "Friendship not found" })),
        )
            .into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error deleting friendship: {err}"),
        )
            .into_response(),
    }
}

#[tokio::main]
async fn main() {
    let creds = Credentials::new("test", "test", None, None, "test");
    let creds_provider = SharedCredentialsProvider::new(creds);
    let config = aws_config::SdkConfig::builder()
        .behavior_version(BehaviorVersion::latest())
        .endpoint_url("http://localhost:4566")
        .credentials_provider(creds_provider)
        .region(Region::new("us-east-1"))
        .build();
    let db = Client::new(&config);
    let state = Arc::new(AppState {
        db,
        user_table_name: "Users".to_string(),
        message_table_name: "Messages".to_string(),
    });
    // build our application with a single route
    let app = Router::new()
        .route("/", get(|| async { "Hello, World!" }))
        .route("/profile/{user_id}", post(create_profile))
        .route("/profile/{user_id}", put(update_profile))
        .route("/profile/{user_id}", get(get_profile))
        .route("/friendship/{user_id}", post(create_friendship))
        .route("/friendship/{user_id}", delete(delete_friendship))
        .route("/friendship/{user_id}/accept", post(accept_friendship))
        .route(
            "/conversation_with/{user_id}",
            get(get_conversation_handler),
        )
        .with_state(state);

    // run our app with hyper, listening globally on port 3000
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}
