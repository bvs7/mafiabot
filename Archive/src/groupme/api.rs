use http::header::CONTENT_TYPE;
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, to_string, Map as JsonMap, Value as JsonValue};
use std::sync::OnceLock;
use uuid::Uuid;

use super::util::{get_token, json_access, GroupId, MessageId, UnexpectedJsonError, UserId};

const BASE_API_URI: &str = "https://api.groupme.com/v3";

static CLIENT: OnceLock<Client> = OnceLock::new();

mod error {
    use serde::de::Unexpected;

    use crate::groupme::util::UnexpectedJsonError;

    pub enum Error {
        UnexpectedJsonError(UnexpectedJsonError),
        SerdeJsonError(serde_json::Error),
        ReqwestError(reqwest::Error),
        TokenError(std::env::VarError),
        OtherError(String),
    }
    impl From<UnexpectedJsonError> for Error {
        fn from(e: UnexpectedJsonError) -> Self {
            Self::UnexpectedJsonError(e)
        }
    }
    impl From<serde_json::Error> for Error {
        fn from(e: serde_json::Error) -> Self {
            Self::SerdeJsonError(e)
        }
    }
    impl From<reqwest::Error> for Error {
        fn from(e: reqwest::Error) -> Self {
            Self::ReqwestError(e)
        }
    }
    impl From<std::env::VarError> for Error {
        fn from(e: std::env::VarError) -> Self {
            Self::TokenError(e)
        }
    }
}

use error::Error;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    nickname: String,
    user_id: UserId,
    /// Membership id, used to kick
    #[serde(skip_serializing, rename = "id")]
    membership_id: Option<String>,
}

impl From<(String, UserId)> for Member {
    fn from((nickname, user_id): (String, UserId)) -> Self {
        Self { nickname, user_id, membership_id: None }
    }
}

pub fn client() -> &'static Client {
    &CLIENT.get_or_init(|| Client::new())
}

#[tracing::instrument]
pub async fn add_members(
    group_id: &GroupId,
    members: Vec<impl Into<Member> + std::fmt::Debug>,
) -> Result<Vec<Member>, Error> {
    let uri = format!("{BASE_API_URI}/groups/{group_id}/members/add");
    let members: Vec<Member> = members.into_iter().map(Into::into).collect();
    let members = json!({
        "members": members
    });
    let body = serde_json::to_string(&members)?;
    tracing::debug!(%body);
    let resp = client()
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
    let resp = client()
        .get(uri)
        .query(&[("token", get_token()?)])
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;
    let value: JsonValue = serde_json::from_str(&resp)?;
    let members: Vec<Member> = json_access(&value, "response.members")?;
    tracing::debug!(?members);
    Ok(members)
}

#[tracing::instrument]
pub async fn create_group(name: &str, share: bool) -> Result<GroupId, Error> {
    let uri = format!("{BASE_API_URI}/groups");
    let body = json!({"name": name, "share": share});
    let body = serde_json::to_string(&body)?;
    let resp = client()
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
    let id: GroupId = json_access::<String>(&value, "response.id")?.into();
    Ok(id)
}

#[tracing::instrument]
pub async fn delete_group(group_id: &GroupId) -> Result<(), Error> {
    let uri = format!("{BASE_API_URI}/groups/{group_id}/destroy");
    let resp =
        client().post(uri).query(&[("token", get_token()?)]).send().await?.error_for_status()?;
    tracing::debug!(?resp);
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
enum Attachment {
    #[serde(rename = "mentions")]
    Mentions { user_ids: Vec<UserId> },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename = "message")]
struct MessageReq {
    text: String,
    source_guid: String,
    attachments: Vec<Attachment>,
}

impl MessageReq {
    fn new(text: String) -> Self {
        Self { text, attachments: Vec::new(), source_guid: Uuid::new_v4().to_string() }
    }
}

#[tracing::instrument]
pub async fn send_group_message(group_id: &GroupId, text: &str) -> Result<MessageId, Error> {
    let uri = format!("{BASE_API_URI}/groups/{group_id}/messages");
    let uuid = Uuid::new_v4();
    let body = MessageReq::new(text.to_owned());
    let body = serde_json::to_string(&body)?;
    let resp = client()
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
    Ok(json_access(&value, "response.message.id")?)
}

#[derive(Debug, Clone, Deserialize)]
struct GroupResp {
    name: String,
    id: Option<GroupId>,
    members: Vec<Member>,
    share_url: Option<String>,
}

#[tracing::instrument]
pub async fn get_group(group_id: &GroupId) -> Result<GroupResp, Error> {
    let uri = format!("{BASE_API_URI}/groups/{group_id}");
    let resp = client()
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

#[derive(Debug, Clone, Serialize)]
#[serde(rename = "direct_message")]
struct DirectMsg {
    recipient_id: UserId,
    text: String,
    source_guid: String,
}

impl DirectMsg {
    fn new(recipient_id: UserId, text: String) -> Self {
        Self { recipient_id, text, source_guid: Uuid::new_v4().to_string() }
    }
}

#[tracing::instrument]
pub async fn send_dm(user_id: UserId, text: &str) -> Result<MessageId, Error> {
    let uri = format!("{BASE_API_URI}/direct_messages");
    let body = DirectMsg::new(user_id, text.to_owned());
    let body = serde_json::to_string(&body)?;
    let resp = client()
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
    Ok(json_access(&value, "response.message.id")?)
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename = "message")]
pub struct MessageResp {
    text: String,
    attachments: Vec<Attachment>,
    source_guid: String,
    id: MessageId,
    user_id: UserId,
    group_id: GroupId,
    favorited_by: Vec<UserId>,
}

#[tracing::instrument]
pub async fn get_group_message(
    group_id: &GroupId,
    msg_id: &MessageId,
) -> Result<MessageResp, Error> {
    let uri = format!("{BASE_API_URI}/groups/{group_id}/messages");
    let resp = client()
        .get(uri)
        .query(&[
            ("token", get_token()?),
            ("after_id", msg_id.prev()),
            ("before_id", msg_id.next()),
            ("limit", "1".to_owned()),
        ])
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    tracing::debug!(?resp);
    let value: JsonValue = serde_json::from_str(&resp)?;
    let message: MessageResp = json_access(&value, "response.messages.0")?;
    if msg_id != &message.id {
        return Err(Error::OtherError(format!(
            "Expected message id {}, got {}",
            msg_id, message.id
        )));
    }
    Ok(message)
}

#[tracing::instrument]
pub async fn like_message(conv_id: &str, msg_id: &MessageId) -> Result<(), Error> {
    let uri = format!("{BASE_API_URI}/messages/{conv_id}/{msg_id}/like");
    let resp =
        client().post(uri).query(&[("token", get_token()?)]).send().await?.error_for_status()?;
    tracing::debug!(?resp);
    Ok(())
}

pub async fn get_user_id() -> Result<UserId, Error> {
    let uri = format!("{BASE_API_URI}/users/me");
    let resp = client()
        .get(uri)
        .query(&[("token", get_token()?)])
        .send()
        .await?
        .error_for_status()?
        .text()
        .await?;

    let value: JsonValue = serde_json::from_str(&resp)?;
    let user_id: UserId = json_access(&value, "response.id")?;
    tracing::info!("Got User Id: {user_id:?}");
    Ok(user_id)
}
