use crate::data_model::{
    CreationResult, DeletionResult, Friendship, FriendshipStatus, Message, Profile,
    UpdateProfileRequest, UpdateResult, UsersFriendsOrIdentical,
};
use aws_sdk_dynamodb::error::SdkError;
use aws_sdk_dynamodb::operation::update_item::UpdateItemError;
use aws_sdk_dynamodb::types::{AttributeValue, ReturnValue};
use aws_sdk_dynamodb::{Client, operation::put_item::PutItemError};
use chrono::prelude::Utc;
use itertools::Itertools;
use serde_dynamo::aws_sdk_dynamodb_1;
use std::collections::HashMap;

pub(crate) fn conversation_id(user_id1: &str, user_id2: &str) -> String {
    let (min_id, max_id) = if user_id1 < user_id2 {
        (user_id1, user_id2)
    } else {
        (user_id2, user_id1)
    };
    format!("CONVERSATION#{}#{}", min_id, max_id)
}

pub(crate) async fn send_message(
    client: &Client,
    table_name: &str,
    user_a: &str,
    user_b: &str,
    text: &str,
) -> Result<CreationResult, Box<dyn std::error::Error + Send + Sync>> {
    let convo_id = conversation_id(user_a, user_b);
    let msg_id = format!("MSG#{}", Utc::now());
    client
        .put_item()
        .table_name(table_name)
        .item("PK", AttributeValue::S(convo_id))
        .item("SK", AttributeValue::S(msg_id))
        .item("sender_id", AttributeValue::S(user_a.to_string()))
        .item(
            "content",
            AttributeValue::M(HashMap::from([(
                "text".to_string(),
                AttributeValue::S(text.to_string()),
            )])),
        )
        .send()
        .await?;
    Ok(CreationResult::Success)
}

pub(crate) async fn get_conversation(
    client: &Client,
    table_name: &str,
    user_a: &str,
    user_b: &str,
) -> Result<Vec<Message>, Box<dyn std::error::Error + Send + Sync>> {
    let convo_id = conversation_id(user_a, user_b);
    // Query DynamoDB
    let resp = client
        .query()
        .table_name(table_name)
        .key_condition_expression("PK = :pk AND begins_with(SK, :msg)")
        .expression_attribute_values(":pk", AttributeValue::S(convo_id))
        .expression_attribute_values(":msg", AttributeValue::S("MSG#".to_string()))
        .scan_index_forward(true) // oldest → newest
        .send()
        .await?;

    // Convert items to Vec<Message>
    let items: Vec<HashMap<String, AttributeValue>> = resp.items().to_vec();
    let messages: Vec<Message> = aws_sdk_dynamodb_1::from_items(items)?;

    Ok(messages)
}

pub(crate) async fn get_latest_message(
    client: &Client,
    table_name: &str,
    user_a: &str,
    user_b: &str,
) -> Result<Option<Message>, Box<dyn std::error::Error + Send + Sync>> {
    let convo_id = conversation_id(user_a, user_b);
    let resp = client
        .query()
        .table_name(table_name)
        .key_condition_expression("PK = :pk AND begins_with(SK, :msg)")
        .expression_attribute_values(":pk", AttributeValue::S(convo_id))
        .expression_attribute_values(":msg", AttributeValue::S("MSG#".to_string()))
        .scan_index_forward(false)
        .limit(1)
        .send()
        .await?;
    let message = match resp.items().first() {
        Some(item) => aws_sdk_dynamodb_1::from_item(item.clone())?,
        None => None,
    };
    Ok(message)
}

fn user_id(id: &str) -> String {
    format!("USER#{id}")
}

pub(crate) async fn users_are_friends_or_identical(
    client: &Client,
    table_name: &str,
    user_a: &str,
    user_b: &str,
) -> Result<UsersFriendsOrIdentical, Box<dyn std::error::Error + Send + Sync>> {
    if user_a == user_b {
        Ok(UsersFriendsOrIdentical::Identical)
    } else {
        let resp = client
            .query()
            .table_name(table_name)
            .key_condition_expression("PK = :pk AND SK = :sk")
            .expression_attribute_values(":pk", AttributeValue::S(user_id(user_a)))
            .expression_attribute_values(":sk", AttributeValue::S(format!("FRIEND#{user_b}")))
            .send()
            .await?;
        let mut items = resp.items().to_vec();
        if items.is_empty() {
            Ok(UsersFriendsOrIdentical::Unrelated)
        } else {
            let friendship: Friendship = aws_sdk_dynamodb_1::from_item(items.pop().unwrap())?;
            Ok(match friendship.status {
                FriendshipStatus::Accepted => UsersFriendsOrIdentical::Friends,
                FriendshipStatus::Pending => UsersFriendsOrIdentical::Unrelated,
            })
        }
    }
}

pub(crate) async fn create_profile(
    client: &Client,
    table_name: &str,
    user: &str,
    profile: Profile,
) -> Result<CreationResult, Box<dyn std::error::Error + Send + Sync>> {
    let res = client
        .put_item()
        .table_name(table_name)
        .item("PK", AttributeValue::S(user_id(user)))
        .item("SK", AttributeValue::S("PROFILE".to_string()))
        .item("full_name", AttributeValue::S(profile.full_name))
        .item("email", AttributeValue::S(profile.email))
        .condition_expression("attribute_not_exists(PK)")
        .send()
        .await;
    if let Err(SdkError::ServiceError(err)) = &res
        && let PutItemError::ConditionalCheckFailedException(_ce) = err.err()
    {
        Ok(CreationResult::Conflict)
    } else {
        let _ = res?;
        Ok(CreationResult::Success)
    }
}

pub(crate) async fn update_profile(
    client: &Client,
    table_name: &str,
    user: &str,
    update_profile_request: UpdateProfileRequest,
) -> Result<UpdateResult<Profile>, Box<dyn std::error::Error + Send + Sync>> {
    let mut parts = vec![];
    let mut expr_attr_values = HashMap::new();
    if let Some(name) = update_profile_request.full_name {
        expr_attr_values.insert(":n".to_string(), AttributeValue::S(name));
        parts.push("full_name = :n");
    }
    if let Some(email) = update_profile_request.email {
        expr_attr_values.insert(":e".to_string(), AttributeValue::S(email));
        parts.push("email = :e");
    }
    if parts.is_empty() {
        return Ok(UpdateResult::EmptyUpdate);
    }
    let update_expr = format!("SET {}", parts.iter().format(", "));
    let resp = client
        .update_item()
        .table_name(table_name)
        .key("PK", AttributeValue::S(user_id(user)))
        .key("SK", AttributeValue::S("PROFILE".to_string()))
        .update_expression(update_expr)
        .set_expression_attribute_values(Some(expr_attr_values))
        .condition_expression("attribute_exists(PK)")
        .return_values(ReturnValue::AllNew)
        .send()
        .await;
    if let Err(SdkError::ServiceError(err)) = &resp
        && let UpdateItemError::ConditionalCheckFailedException(_ce) = err.err()
    {
        Ok(UpdateResult::NotFound)
    } else {
        let updated = resp?.attributes.unwrap_or_default();
        let new_profile = aws_sdk_dynamodb_1::from_item(updated)?;
        Ok(UpdateResult::Success(new_profile))
    }
}

pub(crate) async fn get_profile(
    client: &Client,
    table_name: &str,
    user: &str,
) -> Result<Option<Profile>, Box<dyn std::error::Error + Send + Sync>> {
    let resp = client
        .query()
        .table_name(table_name)
        .key_condition_expression("PK = :pk AND SK = :sk")
        .expression_attribute_values(":pk", AttributeValue::S(user_id(user)))
        .expression_attribute_values(":sk", AttributeValue::S("PROFILE".to_string()))
        .send()
        .await?;
    let profile: Option<Profile> = match resp.items().first() {
        Some(item) => Some(aws_sdk_dynamodb_1::from_item(item.clone())?),
        None => None,
    };
    Ok(profile)
}

async fn get_friendship(
    client: &Client,
    table_name: &str,
    user_a: &str,
    user_b: &str,
) -> Result<Option<Friendship>, Box<dyn std::error::Error + Send + Sync>> {
    let item = client
        .get_item()
        .table_name(table_name)
        .key("PK", AttributeValue::S(user_id(user_a)))
        .key("SK", AttributeValue::S(format!("FRIEND#{user_b}")))
        .send()
        .await?
        .item;
    let friendship: Option<Friendship> = match item {
        Some(item) => aws_sdk_dynamodb_1::from_item(item)?,
        None => None,
    };
    Ok(friendship)
}

async fn friend_request_exists(
    client: &Client,
    table_name: &str,
    user_a: &str,
    user_b: &str,
) -> Result<bool, Box<dyn std::error::Error + Send + Sync>> {
    Ok(get_friendship(client, table_name, user_a, user_b)
        .await?
        .is_some())
}

pub(crate) async fn create_friendship(
    client: &Client,
    table_name: &str,
    user_a: &str,
    user_b: &str,
) -> Result<CreationResult, Box<dyn std::error::Error + Send + Sync>> {
    if user_a == user_b
        || friend_request_exists(client, table_name, user_a, user_b).await?
        || friend_request_exists(client, table_name, user_b, user_a).await?
    {
        return Ok(CreationResult::Conflict);
    }
    let _ = client
        .put_item()
        .table_name(table_name)
        .item("PK", AttributeValue::S(user_id(user_a)))
        .item("SK", AttributeValue::S(format!("FRIEND#{user_b}")))
        .item("status", AttributeValue::S("Pending".to_string()))
        .send()
        .await?;
    Ok(CreationResult::Success)
}

pub(crate) async fn accept_friendship(
    client: &Client,
    table_name: &str,
    user_a: &str,
    user_b: &str,
) -> Result<UpdateResult<Friendship>, Box<dyn std::error::Error + Send + Sync>> {
    if !matches!(
        users_are_friends_or_identical(client, table_name, user_a, user_b).await?,
        UsersFriendsOrIdentical::Unrelated
    ) {
        Ok(UpdateResult::EmptyUpdate)
    } else {
        if friend_request_exists(client, table_name, user_b, user_a).await? {
            client
                .put_item()
                .table_name(table_name)
                .item("PK", AttributeValue::S(user_id(user_a)))
                .item("SK", AttributeValue::S(format!("FRIEND#{user_b}")))
                .item("status", AttributeValue::S("Accepted".to_string()))
                .send()
                .await?;
            let res = client
                .update_item()
                .table_name(table_name)
                .key("PK", AttributeValue::S(user_id(user_b)))
                .key("SK", AttributeValue::S(format!("FRIEND#{user_a}")))
                .update_expression("SET #status = :status")
                .set_expression_attribute_names(Some(HashMap::from([(
                    "#status".to_string(),
                    "status".to_string(),
                )])))
                .set_expression_attribute_values(Some(HashMap::from([(
                    ":status".to_string(),
                    AttributeValue::S("Accepted".to_string()),
                )])))
                .return_values(ReturnValue::AllNew)
                .send()
                .await;
            println!("{:?}", res);
            let friendship: Friendship =
                aws_sdk_dynamodb_1::from_item(res?.attributes.unwrap_or_default())?;
            Ok(UpdateResult::Success(friendship))
        } else {
            Ok(UpdateResult::NotFound)
        }
    }
}

pub(crate) async fn delete_friendship(
    client: &Client,
    table_name: &str,
    user_a: &str,
    user_b: &str,
) -> Result<DeletionResult, Box<dyn std::error::Error + Send + Sync>> {
    if user_a == user_b {
        return Ok(DeletionResult::NotFound);
    }
    let friend_a_b = friend_request_exists(client, table_name, user_a, user_b).await?;
    let friend_b_a = friend_request_exists(client, table_name, user_b, user_a).await?;
    if friend_a_b && friend_b_a {
        // This should happen in a transaction, but eh
        client
            .delete_item()
            .table_name(table_name)
            .key("PK", AttributeValue::S(user_id(user_a)))
            .key("SK", AttributeValue::S(format!("FRIEND#{user_b}")))
            .send()
            .await?;
        client
            .delete_item()
            .table_name(table_name)
            .key("PK", AttributeValue::S(user_id(user_b)))
            .key("SK", AttributeValue::S(format!("FRIEND#{user_a}")))
            .send()
            .await?;
        Ok(DeletionResult::Success)
    } else if friend_a_b {
        client
            .delete_item()
            .table_name(table_name)
            .key("PK", AttributeValue::S(user_id(user_a)))
            .key("SK", AttributeValue::S(format!("FRIEND#{user_b}")))
            .send()
            .await?;
        Ok(DeletionResult::Success)
    } else if friend_b_a {
        client
            .delete_item()
            .table_name(table_name)
            .key("PK", AttributeValue::S(user_id(user_b)))
            .key("SK", AttributeValue::S(format!("FRIEND#{user_a}")))
            .send()
            .await?;
        Ok(DeletionResult::Success)
    } else {
        Ok(DeletionResult::NotFound)
    }
}
