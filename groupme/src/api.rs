use std::sync::OnceLock;

use crate::prelude::*;

mod types;
pub use types::*;
mod handler;
use handler::ApiHandler;

const BASE_API_URI: &str = "https://api.groupme.com/v3";

static HANDLER: OnceLock<ApiHandler> = OnceLock::new();
fn handler() -> &'static ApiHandler {
    HANDLER.get_or_init(|| ApiHandler::new().expect("Cannot continue without API Handler"))
}

static TOKEN: OnceLock<String> = OnceLock::new();
fn token() -> &'static str {
    TOKEN.get_or_init(|| {
        std::env::var("GROUPME_TOKEN").expect("Cannot continue without GROUPME_TOKEN")
    })
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

pub async fn add_members(
    group_id: &GroupId,
    members: Vec<impl Into<Member> + std::fmt::Debug>,
) -> Result<Vec<Member>, Error> {
    let handler = handler();
    handler.add_members(group_id, members).await
}

pub async fn create_group(name: &str, share: bool) -> Result<GroupId, Error> {
    let handler = handler();
    handler.create_group(name, share).await
}

pub async fn delete_group(group_id: &GroupId) -> Result<(), Error> {
    let handler = handler();
    handler.delete_group(group_id).await
}

pub async fn send_group_message(group_id: &GroupId, text: &str) -> Result<MessageId, Error> {
    let handler = handler();
    handler.send_group_message(group_id, text).await
}

pub async fn get_group(group_id: &GroupId) -> Result<GroupResp, Error> {
    let handler = handler();
    handler.get_group(group_id).await
}

pub async fn send_dm(user_id: UserId, text: &str) -> Result<MessageId, Error> {
    let handler = handler();
    handler.send_dm(user_id, text).await
}

pub async fn get_group_message(
    group_id: &GroupId,
    msg_id: &MessageId,
) -> Result<MessageResp, Error> {
    let handler = handler();
    handler.get_group_message(group_id, msg_id).await
}

pub async fn like_group_message(group_id: &GroupId, msg_id: &MessageId) -> Result<(), Error> {
    let handler = handler();
    handler.like_group_message(group_id, msg_id).await
}

pub async fn like_dm_message(user_id: &UserId, msg_id: &MessageId) -> Result<(), Error> {
    let handler = handler();
    handler.like_dm_message(user_id, msg_id).await
}

pub async fn get_user_id() -> Result<UserId, Error> {
    let handler = handler();
    handler.get_user_id().await
}
