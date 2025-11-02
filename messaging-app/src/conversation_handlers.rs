use crate::data_model::{AppState, MessageContent, UsersFriendsOrIdentical};
use crate::util;
use axum::{
    self, Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use std::sync::Arc;

pub(crate) async fn send_message(
    Path(user_id): Path<String>,
    State(state): State<Arc<AppState>>,
    Json(message_content): Json<MessageContent>,
) -> impl IntoResponse {
    // This should be pulled out of the request
    let my_user_id = "123";
    let friends_or_identical = util::users_are_friends_or_identical(
        &state.db,
        &state.user_table_name,
        my_user_id,
        &user_id,
    )
    .await;
    if let Err(err) = friends_or_identical {
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error while checking if users are friends: {err}"),
        )
            .into_response();
    }
    match friends_or_identical.unwrap() {
        UsersFriendsOrIdentical::Identical => {
            (StatusCode::BAD_REQUEST, "Cannot send message to self").into_response()
        }
        UsersFriendsOrIdentical::Friends => match util::send_message(
            &state.db,
            &state.message_table_name,
            my_user_id,
            &user_id,
            &message_content.text,
        )
        .await
        {
            Ok(_) => (
                StatusCode::CREATED,
                Json(serde_json::json!({ "status": "ok" })),
            )
                .into_response(),
            Err(err) => (
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Error while sending message: {err}"),
            )
                .into_response(),
        },
        UsersFriendsOrIdentical::Unrelated => (
            StatusCode::UNAUTHORIZED,
            "Users can only send messages to their friends",
        )
            .into_response(),
    }
}

pub(crate) async fn get_conversation(
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

pub(crate) async fn get_latest_message(
    Path(user_id): Path<String>,
    State(state): State<Arc<AppState>>,
) -> impl IntoResponse {
    // This should be pulled out of the request
    let my_user_id = "123";
    let message =
        util::get_latest_message(&state.db, &state.message_table_name, my_user_id, &user_id).await;
    match message {
        Ok(Some(message)) => (StatusCode::OK, Json(message)).into_response(),
        Ok(None) => (
            StatusCode::NOT_FOUND,
            format!("You have no messages with {user_id}"),
        )
            .into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Error fetching the latest message with {user_id}: {err}"),
        )
            .into_response(),
    }
}
