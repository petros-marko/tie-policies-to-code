use crate::auth::AuthUser;
use crate::data_model::{AppState, CreationResult, DeletionResult, UpdateResult};
use crate::util;
use axum::{
    self, Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;

pub(crate) async fn create_friendship(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
    AuthUser {claims}: AuthUser,
) -> impl IntoResponse {
    // This should be pulled out of the request
    // let my_user_id = "123";
    match util::create_friendship(&state.db, &state.user_table_name, &claims.sub.strip_prefix("auth0|").unwrap(), &user_id).await {
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

pub(crate) async fn accept_friendship(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
    AuthUser {claims}: AuthUser,
) -> impl IntoResponse {
    // This should be pulled out of the request
    // let my_user_id = "123";
    match util::accept_friendship(&state.db, &state.user_table_name, &claims.sub.strip_prefix("auth0|").unwrap(), &user_id).await {
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

pub(crate) async fn delete_friendship(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
    AuthUser {claims}: AuthUser,
) -> impl IntoResponse {
    // This should be pulled out of the request
    // let my_user_id = "123";
    match util::delete_friendship(&state.db, &state.user_table_name, &claims.sub.strip_prefix("auth0|").unwrap(), &user_id).await {
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
