use crate::prelude::*;

use reqwest::header::CONTENT_TYPE;
use std::{env::VarError, sync::Arc, time::Duration};
use tokio::{sync::Notify, task::JoinHandle, time::Interval};

use super::{types::*, Error, BASE_API_URI};

#[derive(Debug)]
pub struct ApiHandler {
    notify: Arc<Notify>,
    token: String,
    client: reqwest::Client,
    notifier: JoinHandle<()>,
}

impl ApiHandler {
    pub fn new() -> Result<Self, VarError> {
        let notify = Arc::new(Notify::new());
        let token = get_token()?.to_owned();
        let client = reqwest::Client::new();
        let interval = tokio::time::interval(Duration::from_millis(100));
        let n = notify.clone();
        let notifier = tokio::spawn(Self::notifier(interval, n));
        Ok(Self { notify, token, client, notifier })
    }

    async fn notifier(mut interval: Interval, notify: Arc<Notify>) {
        interval.tick().await;
        notify.notify_one();
    }

    async fn client(&self) -> &reqwest::Client {
        self.notify.notified().await;
        &self.client
    }

    async fn get(&self, uri: &str, queries: &[(&str, &str)]) -> Result<String, Error> {
        match self
            .client()
            .await
            .get(uri)
            .query(&[("token", &self.token)])
            .query(queries)
            .send()
            .await?
        {
            resp if resp.status().is_success() => Ok(resp.text().await?),
            resp => {
                let status = resp.status();
                let text = resp.text().await?;
                Err(Error::OtherError(format!(
                    "GET request to {uri} failed with status {status}: {text}"
                )))
            }
        }
    }

    async fn post(&self, uri: &str, body: String) -> Result<String, Error> {
        match self
            .client()
            .await
            .post(uri)
            .header(CONTENT_TYPE, "application/json")
            .query(&[("token", &self.token)])
            .body(body)
            .send()
            .await?
        {
            resp if resp.status().is_success() => Ok(resp.text().await?),
            resp => {
                let status = resp.status();
                let text = resp.text().await?;
                Err(Error::OtherError(format!(
                    "POST request to {uri} failed with status {status}: {text}"
                )))
            }
        }
    }

    #[tracing::instrument(skip(self))]
    pub async fn add_members(
        &self,
        group_id: &GroupId,
        members: Vec<impl Into<Member> + std::fmt::Debug>,
    ) -> Result<Vec<Member>, Error> {
        let uri = format!("{BASE_API_URI}/groups/{group_id}/members/add");
        let members: Vec<Member> = members.into_iter().map(Into::into).collect();
        let members = json!({
            "members": members
        });
        let body = serde_json::to_string(&members)?;
        let client = self.client().await;
        let resp = self.post(&uri, body).await?;
        let value: JsonValue = serde_json::from_str(&resp)?;
        let result_id: String = json_access(&value, "response.results_id")?;
        let uri = format!("{BASE_API_URI}/groups/{group_id}/members/results/{result_id}");
        let resp = self.get(&uri, &[]).await?;
        let value: JsonValue = serde_json::from_str(&resp)?;
        let members: Vec<Member> = json_access(&value, "response.members")?;
        debug!(?members);
        Ok(members)
    }

    #[tracing::instrument(skip(self))]
    pub async fn create_group(&self, name: &str, share: bool) -> Result<GroupId, Error> {
        let uri = format!("{BASE_API_URI}/groups");
        let body = json!({"name": name, "share": share});
        let body = serde_json::to_string(&body)?;
        let resp = self.post(&uri, body).await?.to_owned();
        debug!(?resp);
        let value: JsonValue = serde_json::from_str(&resp)?;
        let id: GroupId = json_access::<String>(&value, "response.id")?.into();
        Ok(id)
    }

    #[tracing::instrument(skip(self))]
    pub async fn delete_group(&self, group_id: &GroupId) -> Result<(), Error> {
        let uri = format!("{BASE_API_URI}/groups/{group_id}/destroy");
        let resp = self.post(&uri, "".to_owned()).await?;
        debug!(?resp);
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn send_group_message(
        &self,
        group_id: &GroupId,
        text: &str,
    ) -> Result<MessageId, Error> {
        let uri = format!("{BASE_API_URI}/groups/{group_id}/messages");
        let body = Payload::new_message(text.to_owned());
        let body = serde_json::to_string(&body)?;
        debug!("Sending MessageReq as: {body}");
        let resp = self.post(&uri, body).await?;
        debug!(?resp);
        let value: JsonValue = serde_json::from_str(&resp)?;
        Ok(json_access(&value, "response.message.id")?)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_group(&self, group_id: &GroupId) -> Result<GroupResp, Error> {
        let uri = format!("{BASE_API_URI}/groups/{group_id}");
        let resp = self.get(&uri, &[]).await?;
        debug!(?resp);
        let value: JsonValue = serde_json::from_str(&resp)?;
        Ok(serde_json::from_value(json_access(&value, "response")?)?)
    }

    #[tracing::instrument(skip(self))]
    pub async fn send_dm(&self, user_id: UserId, text: &str) -> Result<MessageId, Error> {
        let uri = format!("{BASE_API_URI}/direct_messages");
        let body = Payload::new_dm(&user_id, text.to_string());
        let body = serde_json::to_string(&body)?;
        let resp = self.post(&uri, body).await?;
        debug!(?resp);
        let value: JsonValue = serde_json::from_str(&resp)?;
        Ok(json_access(&value, "response.direct_message.id")?)
    }

    #[tracing::instrument(skip(self))]
    pub async fn get_group_message(
        &self,
        group_id: &GroupId,
        msg_id: &MessageId,
    ) -> Result<MessageResp, Error> {
        let uri = format!("{BASE_API_URI}/groups/{group_id}/messages");
        let resp = self
            .get(
                &uri,
                &[("after_id", &msg_id.prev()), ("before_id", &msg_id.next()), ("limit", "1")],
            )
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

    #[tracing::instrument(skip(self))]
    pub async fn like_group_message(
        &self,
        group_id: &GroupId,
        msg_id: &MessageId,
    ) -> Result<(), Error> {
        let uri = format!("{BASE_API_URI}/messages/{group_id}/{msg_id}/like");
        let resp = self.post(&uri, "".to_owned()).await?;
        debug!(?resp);
        Ok(())
    }

    #[tracing::instrument(skip(self))]
    pub async fn like_dm_message(&self, user_id: &UserId, msg_id: &MessageId) -> Result<(), Error> {
        let conv_id = if user_id < &MODERATOR_UID {
            format!("{user_id}+{MODERATOR_UID}")
        } else {
            format!("{MODERATOR_UID}+{user_id}")
        };
        let uri = format!("{BASE_API_URI}/messages/{conv_id}/{msg_id}/like");
        let resp = self.post(&uri, "".to_owned()).await?;
        debug!(?resp);
        Ok(())
    }

    pub async fn get_user_id(&self) -> Result<UserId, Error> {
        let uri = format!("{BASE_API_URI}/users/me");
        let resp = self.post(&uri, "".to_owned()).await?;

        let value: JsonValue = serde_json::from_str(&resp)?;
        let user_id: UserId = json_access(&value, "response.id")?;
        tracing::info!("Got User Id: {user_id:?}");
        Ok(user_id)
    }
}
