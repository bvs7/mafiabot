use crate::prelude::*;
use std::env;

use mafia::Pid;

pub const MODERATOR_UID: UserId = UserId(43040067);
pub const BRIAN_UID: UserId = UserId(21642197);

pub const LOBBY_CHAT_ID: GroupId = GroupId(25833774);
pub const MAIN_CHAT_ID: GroupId = GroupId(105362524);
pub const MAFIA_CHAT_ID: GroupId = GroupId(105362533);
pub const TEST_LOBBY_CHAT_ID: GroupId = GroupId(105412553);

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub struct UserId(pub u64);

impl Into<Pid> for UserId {
    fn into(self) -> Pid {
        Pid::from(self.0)
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
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

pub fn get_token() -> Result<String, env::VarError> {
    env::var("GROUPME_TOKEN")
}

#[derive(Debug)]
pub struct UnexpectedJsonError {
    value: JsonValue,
    path: String,
    source: Option<serde_json::Error>,
}
impl std::error::Error for UnexpectedJsonError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        self.source.as_ref().map(|e| e as &(dyn std::error::Error + 'static))
    }
}
impl std::fmt::Display for UnexpectedJsonError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Unexpected JSON error")
    }
}

pub fn json_access<T: serde::de::DeserializeOwned>(
    value: &JsonValue,
    path: &str,
) -> Result<T, UnexpectedJsonError> {
    let keys = path.split(".");
    let mut v = value;
    for key in keys {
        // If we have a key, first ensure v is indexable
        match v {
            JsonValue::Object(_) | JsonValue::Array(_) | JsonValue::String(_) => {}
            _ => {
                return Err(UnexpectedJsonError {
                    value: value.clone(),
                    path: path.to_string(),
                    source: None,
                })
            }
        }
        match key.parse::<usize>() {
            Ok(k) => v = &v[k],
            Err(_) => v = &v[key],
        }
    }
    let t: T = match serde_json::from_value(v.clone()) {
        Ok(t) => t,
        Err(err) => {
            return Err(UnexpectedJsonError {
                value: value.clone(),
                path: path.to_string(),
                source: Some(err),
            })
        }
    };
    return Ok(t);
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::prelude::*;

    #[test]
    fn test_json_access() -> Result<(), UnexpectedJsonError> {
        let value = json!({
            "outer" : {
                "inner" : {
                    "array" : [
                        "0",
                        "1",
                        "2",
                        "3",
                    ],
                    "number" : 100,
                    "string" : "string",
                    "bool": true,
                    "null": null,
                }
            }
        });

        let outer: JsonValue = json_access(&value, "outer")?;
        let inner: JsonValue = json_access(&value, "outer.inner")?;
        let inner_: JsonValue = json_access(&outer, "inner")?;
        assert!(inner == inner_);

        assert!(json_access::<u64>(&value, "badrefs").is_err());

        let array: Vec<String> = json_access(&value, "outer.inner.array")?;

        let array_element: String = json_access(&value, "outer.inner.array.1")?;

        assert!(json_access::<String>(&value, "outer.inner.array.100").is_err());

        let number: u32 = json_access(&value, "outer.inner.number")?;
        assert_eq!(number, 100);
        let string: String = json_access(&value, "outer.inner.string")?;
        assert_eq!(string, "string".to_owned());
        let boolean: bool = json_access(&value, "outer.inner.bool")?;
        assert_eq!(boolean, true);

        assert!(json_access::<()>(&value, "outer.inner.null").is_ok());
        Ok(())
    }

    #[test]
    fn test_get_token() {
        assert!(get_token().is_ok())
    }
}
