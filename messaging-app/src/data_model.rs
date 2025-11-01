use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize)]
pub struct Message {
    pub sender_id: String,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum FriendshipStatus {
    Accepted,
    Pending,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Friendship {
    pub status: FriendshipStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Profile {
    pub full_name: String,
    pub email: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UpdateProfileRequest {
    pub full_name: Option<String>,
    pub email: Option<String>,
}

pub enum CreationResult {
    Success,
    Conflict,
}

pub enum DeletionResult {
    Success,
    NotFound,
}

pub enum UpdateResult<T: Serialize + for<'a> Deserialize<'a>> {
    Success(T),
    EmptyUpdate,
    NotFound,
}
