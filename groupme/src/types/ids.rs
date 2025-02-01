use crate::prelude::*;

pub const MODERATOR_UID: UserId = UserId(43040067);
pub const BRIAN_UID: UserId = UserId(21642197);

pub const LOBBY_CHAT_ID: GroupId = GroupId(25833774);
pub const MAIN_CHAT_ID: GroupId = GroupId(105362524);
pub const MAFIA_CHAT_ID: GroupId = GroupId(105362533);
pub const TEST_LOBBY_CHAT_ID: GroupId = GroupId(105412553);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub struct UserId(pub u64);

impl From<UserId> for u64 {
    fn from(u: UserId) -> Self {
        u.0
    }
}
impl From<u64> for UserId {
    fn from(u: u64) -> Self {
        Self(u)
    }
}
impl From<String> for UserId {
    fn from(s: String) -> Self {
        Self(s.parse().unwrap())
    }
}
impl From<UserId> for String {
    fn from(u: UserId) -> Self {
        u.0.to_string()
    }
}
impl std::fmt::Display for UserId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub struct GroupId(pub u64);

impl From<String> for GroupId {
    fn from(s: String) -> Self {
        Self(s.parse().unwrap())
    }
}
impl From<GroupId> for String {
    fn from(g: GroupId) -> Self {
        g.0.to_string()
    }
}
impl std::fmt::Display for GroupId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub struct MessageId(pub u128);

impl From<String> for MessageId {
    fn from(s: String) -> Self {
        Self(s.parse().unwrap())
    }
}
impl From<MessageId> for String {
    fn from(m: MessageId) -> Self {
        m.0.to_string()
    }
}
impl std::fmt::Display for MessageId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl MessageId {
    pub fn prev(&self) -> String {
        (self.0 - 1).to_string()
    }

    pub fn next(&self) -> String {
        (self.0 + 1).to_string()
    }
}
