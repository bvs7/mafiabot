/// Handle GroupMe API calls
use anyhow::{bail, Result};
use http::header::CONTENT_TYPE;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, to_string, Map as JsonMap, Value as JsonValue};
use uuid::Uuid;

use super::util::{get_token, json_access, UnexpectedJsonError};

pub type UserId = u64;

pub type GroupId = u64;

pub type MessageId = String;

const BASE_API_URI: &str = "https://api.groupme.com/v3";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    nickname: String,
    user_id: UserId,
}

#[tracing::instrument(skip(client))]
pub async fn add_members(
    client: &Client,
    group_id: &str,
    members: Vec<(String, UserId)>,
) -> Result<()> {
    let uri = format!("{BASE_API_URI}/groups/{group_id}/members/add");
    let mut members_vec = Vec::new();
    for (nickname, user_id) in members {
        let user_id_str = user_id.to_string();
        members_vec.push(json!(
            {
                "nickname":nickname,
                "user_id": user_id_str
            }
        ));
    }
    let members = json!({
        "members": members_vec
    });
    let body = serde_json::to_string(&members)?;
    tracing::debug!(%body);
    let resp = client
        .post(uri)
        .header(CONTENT_TYPE, "application/json")
        .query(&[("token", get_token()?)])
        .body(body)
        .send()
        .await?
        // .error_for_status()?
        .text()
        .await?;
    tracing::debug!(%resp);
    let value: JsonValue = serde_json::from_str(&resp)?;
    let result_id: String = json_access(&value, "response.results_id")?;
    let uri = format!("{BASE_API_URI}/groups/{group_id}/members/results/{result_id}");
    let resp = client
        .get(uri)
        .query(&[("token", get_token()?)])
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    tracing::debug!(?resp);
    Ok(())
}

#[tracing::instrument(skip(client))]
pub async fn create_group(client: &Client, name: &str) -> Result<GroupId> {
    let uri = format!("{BASE_API_URI}/groups");
    let body = json!({"name": name});
    let body = serde_json::to_string(&body)?;
    let resp = client
        .post(uri)
        .header(CONTENT_TYPE, "application/json")
        .query(&[("token", get_token()?)])
        .body(body)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    tracing::debug!(?resp);
    let value: JsonValue = serde_json::from_str(&resp)?;
    let id: String = json_access(&value, "response.id")?;
    let id: GroupId = id.parse()?;
    Ok(id)
}

#[tracing::instrument(skip(client))]
pub async fn delete_group(client: &Client, group_id: GroupId) -> Result<()> {
    let uri = format!("{BASE_API_URI}/groups/{group_id}/destroy");
    let resp = client
        .post(uri)
        .query(&[("token", get_token()?)])
        .send()
        .await?
        .error_for_status()?;
    tracing::debug!(?resp);
    Ok(())
}

#[tracing::instrument(skip(client))]
pub async fn send_group_message(client: &Client, group_id: &str, text: &str) -> Result<MessageId> {
    let uri = format!("{BASE_API_URI}/groups/{group_id}/messages");
    let uuid = Uuid::new_v4();
    let body = json!({
        "message": {
            "source_guid": uuid.to_string(),
            "text" : text,
        }
    });
    let body = serde_json::to_string(&body)?;
    let resp = client
        .post(uri)
        .header(CONTENT_TYPE, "application/json")
        .query(&[("token", get_token()?)])
        .body(body)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    tracing::debug!(?resp);
    let value: JsonValue = serde_json::from_str(&resp)?;
    let id: MessageId = json_access(&value, "response.message.id")?;

    Ok(id)
}

#[tracing::instrument(skip(client))]
pub async fn get_group(client: &Client, group_id: &str) -> Result<JsonValue> {
    let uri = format!("{BASE_API_URI}/groups/{group_id}");
    let resp = client
        .get(uri)
        .query(&[("token", get_token()?)])
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    tracing::debug!(?resp);
    Ok(serde_json::from_str(&resp)?)
}

#[tracing::instrument(skip(client))]
pub async fn send_dm(client: &Client, user_id: UserId, text: &str) -> Result<MessageId> {
    let uri = format!("{BASE_API_URI}/direct_messages");
    let uuid = Uuid::new_v4();
    let body = json!({
        "direct_message": {
            "source_guid": uuid.to_string(),
            "recipient_id" : user_id,
            "text" : text,
        }
    });
    let body = serde_json::to_string(&body)?;
    let resp = client
        .post(uri)
        .header(CONTENT_TYPE, "application/json")
        .query(&[("token", get_token()?)])
        .body(body)
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    tracing::debug!(?resp);
    let value: JsonValue = serde_json::from_str(&resp)?;
    let id: MessageId = json_access(&value, "response.direct_message.id")?;

    Ok(id)
}

#[tracing::instrument(skip(client))]
pub async fn get_group_message_likes(
    client: &Client,
    group_id: GroupId,
    msg_id: &MessageId,
) -> Result<Vec<UserId>> {
    let uri = format!("{BASE_API_URI}/groups/{group_id}/messages");
    let prev_msg_id = (msg_id.parse::<u64>()? - 1).to_string();
    let resp = client
        .get(uri)
        .query(&[
            ("token", get_token()?),
            ("after_id", prev_msg_id),
            ("limit", "1".to_owned()),
        ])
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    tracing::debug!(?resp);
    let value: JsonValue = serde_json::from_str(&resp)?;
    let id: String = json_access(&value, "response.messages.0.id")?;
    if msg_id != &id {
        bail!("Got the wrong message?? msg_id={msg_id}, found={id}");
    }
    let user_strs: Vec<String> = json_access(&value, "response.messages.0.favorited_by")?;
    let users: Vec<u64> = user_strs.into_iter().map(|u| u.parse().unwrap()).collect();
    Ok(users)
}

#[tracing::instrument(skip(client))]
pub async fn like_message(client: &Client, conv_id: &str, msg_id: &MessageId) -> Result<()> {
    let uri = format!("{BASE_API_URI}/messages/{conv_id}/{msg_id}/like");
    let resp = client
        .post(uri)
        .query(&[("token", get_token()?)])
        .send()
        .await?
        .error_for_status()?;
    tracing::debug!(?resp);
    Ok(())
}

pub async fn get_user_id(client: &Client) -> Result<UserId> {
    let uri = format!("{BASE_API_URI}/users/me");
    let resp = client
        .get(uri)
        .query(&[("token", get_token()?)])
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let value: JsonValue = serde_json::from_str(&resp)?;
    let user_id_str: String = json_access(&value, "response.id")?;
    let user_id = user_id_str.parse().unwrap();
    tracing::info!("Got User Id: {user_id}");
    Ok(user_id)
}
