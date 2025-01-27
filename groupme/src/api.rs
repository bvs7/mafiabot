use std::sync::OnceLock;

use reqwest::header::CONTENT_TYPE;

use crate::prelude::*;

const BASE_API_URI: &str = "https://api.groupme.com/v3";

static CLIENT: OnceLock<reqwest::Client> = OnceLock::new();
fn client() -> &'static reqwest::Client {
    &CLIENT.get_or_init(|| reqwest::Client::new())
}

fn uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("Unexpected JSON error")]
    UnexpectedJsonError(#[from] UnexpectedJsonError),
    #[error("Serde JSON error")]
    SerdeJsonError(#[from] serde_json::Error),
    #[error("Reqwest error")]
    ReqwestError(#[from] reqwest::Error),
    #[error("Token error")]
    TokenError(#[from] std::env::VarError),
    #[error("Unknown error {0}")]
    OtherError(String),
}

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
    debug!(?members);
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
    debug!(?resp);
    let value: JsonValue = serde_json::from_str(&resp)?;
    let id: GroupId = json_access::<String>(&value, "response.id")?.into();
    Ok(id)
}

#[tracing::instrument]
pub async fn delete_group(group_id: &GroupId) -> Result<(), Error> {
    let uri = format!("{BASE_API_URI}/groups/{group_id}/destroy");
    let resp =
        client().post(uri).query(&[("token", get_token()?)]).send().await?.error_for_status()?;
    debug!(?resp);
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
enum Payload {
    #[serde(rename = "message")]
    Message { text: String, source_guid: String, attachments: Vec<Attachment> },
    #[serde(rename = "direct_message")]
    DirectMsg {
        text: String,
        source_guid: String,
        recipient_id: UserId,
        attachments: Vec<Attachment>,
    },
}

impl Payload {
    fn new_message(text: String) -> Self {
        Self::Message { text, attachments: Vec::new(), source_guid: uuid() }
    }
    fn new_dm(user_id: &UserId, text: String) -> Self {
        Self::DirectMsg {
            text,
            source_guid: uuid(),
            recipient_id: *user_id,
            attachments: Vec::new(),
        }
    }
}

#[tracing::instrument]
pub async fn send_group_message(group_id: &GroupId, text: &str) -> Result<MessageId, Error> {
    let uri = format!("{BASE_API_URI}/groups/{group_id}/messages");
    let body = Payload::new_message(text.to_owned());
    let body = serde_json::to_string(&body)?;
    debug!("Sending MessageReq as: {body}");
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

    debug!(?resp);
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
    debug!(?resp);
    Ok(serde_json::from_str(&resp)?)
}

#[tracing::instrument]
pub async fn send_dm(user_id: UserId, text: &str) -> Result<MessageId, Error> {
    let uri = format!("{BASE_API_URI}/direct_messages");
    let body = Payload::new_dm(&user_id, text.to_string());
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

    debug!(?resp);
    let value: JsonValue = serde_json::from_str(&resp)?;
    Ok(json_access(&value, "response.direct_message.id")?)
}

#[derive(Debug, Clone, Deserialize)]
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

    debug!(?resp);
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
pub async fn like_group_message(group_id: &GroupId, msg_id: &MessageId) -> Result<(), Error> {
    let uri = format!("{BASE_API_URI}/messages/{group_id}/{msg_id}/like");
    let resp =
        client().post(uri).query(&[("token", get_token()?)]).send().await?.error_for_status()?;
    debug!(?resp);
    Ok(())
}

#[tracing::instrument]
pub async fn like_dm_message(user_id: &UserId, msg_id: &MessageId) -> Result<(), Error> {
    let conv_id = if user_id < &MODERATOR_UID {
        format!("{user_id}+{MODERATOR_UID}")
    } else {
        format!("{MODERATOR_UID}+{user_id}")
    };
    let uri = format!("{BASE_API_URI}/messages/{conv_id}/{msg_id}/like");
    let resp =
        client().post(uri).query(&[("token", get_token()?)]).send().await?.error_for_status()?;
    debug!(?resp);
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

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn basic() {
        let text = "Test message";

        let msg_id = send_group_message(&TEST_LOBBY_CHAT_ID, &text).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        like_group_message(&TEST_LOBBY_CHAT_ID, &msg_id).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        let msg_id = send_dm(BRIAN_UID, &text).await.unwrap();
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
        like_dm_message(&BRIAN_UID, &msg_id).await.unwrap();
    }
}
