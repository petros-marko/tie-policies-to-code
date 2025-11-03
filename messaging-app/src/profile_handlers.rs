use crate::auth::AuthUser;
use crate::data_model::{
    AppState, CreationResult, Profile, UpdateProfileRequest, UpdateResult, UsersFriendsOrIdentical,
};
use crate::util;
use axum::{
    self, Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;

pub(crate) async fn create_profile(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
    AuthUser {claims}: AuthUser,
    Json(profile): Json<Profile>,
) -> impl IntoResponse {
    // This should be pulled out of the request
    // let my_user_id = "123";
    if claims.sub.strip_prefix("auth0|").unwrap() == user_id {
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

pub(crate) async fn update_profile(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
    AuthUser {claims}: AuthUser,
    Json(update_profile_request): Json<UpdateProfileRequest>,
) -> impl IntoResponse {
    // This should be pulled out of the request
    // let my_user_id = "123";
    if claims.sub.strip_prefix("auth0|").unwrap() == user_id {
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

pub(crate) async fn get_profile(
    State(state): State<Arc<AppState>>,
    Path(user_id): Path<String>,
    AuthUser {claims}: AuthUser,
) -> impl IntoResponse {
    // This should be pulled out of the request
    // let my_user_id = "123";
    match util::users_are_friends_or_identical(
        &state.db,
        &state.user_table_name,
        &claims.sub.strip_prefix("auth0|").unwrap(),
        &user_id,
    )
    .await
    {
        Ok(UsersFriendsOrIdentical::Friends) | Ok(UsersFriendsOrIdentical::Identical) => {
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
        Ok(UsersFriendsOrIdentical::Unrelated) => (
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
