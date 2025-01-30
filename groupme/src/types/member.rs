use crate::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Member {
    pub nickname: String,
    pub user_id: UserId,
    /// Membership id, used to kick
    #[serde(skip_serializing, rename = "id")]
    pub membership_id: Option<String>,
}

impl Member {
    pub fn new(nickname: String, user_id: UserId) -> Self {
        Self { nickname, user_id, membership_id: None }
    }
}

impl From<(String, UserId)> for Member {
    fn from((nickname, user_id): (String, UserId)) -> Self {
        Self { nickname, user_id, membership_id: None }
    }
}
