use crate::prelude::*;

fn uuid() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum Attachment {
    #[serde(rename = "mentions")]
    Mentions { user_ids: Vec<UserId> },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Serialize)]
pub enum Payload {
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
    pub fn new_message(text: String) -> Self {
        Self::Message { text, attachments: Vec::new(), source_guid: uuid() }
    }
    pub fn new_dm(user_id: &UserId, text: String) -> Self {
        Self::DirectMsg {
            text,
            source_guid: uuid(),
            recipient_id: *user_id,
            attachments: Vec::new(),
        }
    }
}
#[derive(Debug, Clone, Deserialize)]
pub struct GroupResp {
    pub name: String,
    pub id: Option<GroupId>,
    pub members: Vec<Member>,
}
#[derive(Debug, Clone, Deserialize)]
pub struct MessageResp {
    text: String,
    attachments: Vec<Attachment>,
    source_guid: String,
    pub id: MessageId,
    user_id: UserId,
    group_id: GroupId,
    pub favorited_by: Vec<UserId>,
}
