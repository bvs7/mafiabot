use crate::prelude::*;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub struct PushId(u64);

impl From<PushId> for String {
    fn from(value: PushId) -> Self {
        let mut x = value.0;
        let mut s = String::new();
        while x > 0 {
            let c = char::from_u32(((x % 36) + 48) as u32).unwrap();
            x = x / 36;
            s.insert(0, c);
        }
        format!("{:x}", value.0)
    }
}

impl From<String> for PushId {
    fn from(value: String) -> Self {
        Self(u64::from_str_radix(value.as_str(), 36).unwrap_or_else(|e| {
            tracing::error!("Error parsing pushid: {value}, {e}");
            0
        }))
    }
}

impl PushId {
    pub fn new() -> Self {
        Self(0)
    }
    pub fn inc_and_clone(&mut self) -> Self {
        self.0 += 1;
        Self(self.0)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Channel {
    #[default]
    #[serde(rename = "/meta/handshake")]
    Handshake,
    #[serde(rename = "/meta/subscribe")]
    Subscribe,
    #[serde(rename = "/meta/connect")]
    Connect,
    #[serde(rename = "/user/43040067")]
    Moderator,
    #[serde(untagged)]
    Unknown(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Advice {
    reconnect: String,
    interval: u32,
    timeout: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Ext {
    access_token: String,
    timestamp: u64,
}

impl Ext {
    pub fn now() -> Result<Self, env::VarError> {
        Ok(Self { access_token: get_token()?, timestamp: chrono::Utc::now().timestamp() as u64 })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushMessage {
    pub id: PushId,
    pub channel: Channel,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub supported_connection_types: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub subscription: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub advice: Option<Advice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub successful: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", skip_deserializing)]
    pub ext: Option<Ext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_type: Option<String>,
    #[serde(skip_serializing)]
    pub data: Option<Data>,
}

impl PushMessage {
    pub fn new(id: &mut PushId, channel: Channel) -> Self {
        Self { id: id.inc_and_clone(), channel, ..Self::default() }
    }

    pub fn data(&self) -> &Option<Data> {
        &self.data
    }

    pub fn version(mut self, version: &str) -> Self {
        self.version = Some(String::from(version));
        self
    }
    pub fn supported_connection_types(mut self, value: &[&str]) -> Self {
        let v = value.into_iter().map(|s| String::from(*s)).collect();
        self.supported_connection_types = Some(v);
        self
    }
    pub fn subscription(mut self, subscription: &str) -> Self {
        self.subscription = Some(String::from(subscription));
        self
    }
    pub fn client_id(mut self, client_id: &str) -> Self {
        self.client_id = Some(String::from(client_id));
        self
    }
    pub fn ext(mut self, ext: Ext) -> Self {
        self.ext = Some(ext);
        self
    }
    pub fn connection_type(mut self, connection_type: &str) -> Self {
        self.connection_type = Some(String::from(connection_type));
        self
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", content = "subject")]
pub enum Data {
    #[serde(rename = "line.create")]
    GroupMsg {
        attachments: Vec<Attachment>,
        group_id: GroupId,
        created_at: u64,
        id: MessageId,
        name: String,
        text: String,
        user_id: UserId,
    },
    #[serde(rename = "direct_message.create")]
    DirectMsg {
        attachments: Vec<Attachment>,
        created_at: u64,
        id: MessageId,
        name: String,
        text: String,
        user_id: UserId,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
pub enum Attachment {
    #[serde(rename = "mentions")]
    Mentions { user_ids: Vec<u64> },
    #[serde(other)]
    Unknown,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    #[tracing_test::traced_test]
    fn test_channels() {
        let channels = vec![
            super::Channel::Handshake,
            super::Channel::Subscribe,
            super::Channel::Connect,
            super::Channel::Unknown("/user/45234".to_string()),
        ];

        for channel in channels {
            let channel_str = serde_json::to_string(&channel).unwrap();
            debug!("{:?} => {:?}", channel, channel_str);
        }

        let channel_strs = vec![
            "\"/meta/handshake\"",
            "\"/meta/subscribe\"",
            "\"/meta/connect\"",
            "\"/user/45234\"",
        ];

        for channel_str in channel_strs {
            let channel: Channel = serde_json::from_str(channel_str).unwrap();
            debug!("{:?} => {:?}", channel_str, channel);
        }
    }

    fn test1() -> JsonValue {
        json!([
            {
                "channel": "/user/43040067",
                "clientId": "fayeF34IPC2IHND7HULV5WAZW2EZNKBMKVO",
                "data": {
                    "alert": "Brian \"Testing\" Scaramella: Test",
                    "received_at": 1737048635000_u128,
                    "subject": {
                        "attachments": [],
                        "avatar_url": "https://i.groupme.com/200x205.jpeg.3475af00b96f4d1d8a21a9822ddd5e3a",
                        "created_at": 1737048634,
                        "deleted_at": null,
                        "deletion_actor": null,
                        "group_id": "105362524",
                        "id": "173704863488783901",
                        "location": {
                            "lat": "",
                            "lng": "",
                            "name": null,
                        },
                        "name": "Brian \"Testing\" Scaramella",
                        "parent_id": null,
                        "picture_url": null,
                        "pinned_at": null,
                        "pinned_by": null,
                        "sender_id": "21642197",
                        "sender_type": "user",
                        "source_guid": "android-bbfe3445-d65e-4352-a613-19df30dede79",
                        "system": false,
                        "text": "Test",
                        "updated_at": null,
                        "user_id": "21642197",
                    },
                    "type": "line.create",
                },
                "id": "16122cff",
            },
        ])
    }

    fn test2() -> JsonValue {
        json!([{
            "channel": "/user/43040067",
            "clientId": "fayeF34IPC2IHND7HULV5WAZW2EZNKBMKVO",
            "data":  {
                "received_at": 1737048690000_u128,
                "subject":  {
                    "attachments":  [{
                        "loci":  [ [
                                6,
                                27,
                            ],],
                        "type": "mentions",
                        "user_ids":  [
                            21642197,
                        ],
                    },],
                    "created_at": 1737048690,
                    "group_id": "105362524",
                    "id": "173704869054202031",
                    "name": "MODERATOR",
                    "text": "/vote @Brian \"Testing\" Scaramella ",
                    "user_id": "43040067",
                },
                "type": "line.create",
            },
            "id": "16122cff",
        },])
    }

    fn test3() -> JsonValue {
        json!([{
            "channel": "/user/43040067",
            "clientId": "fayeLJBD7D2WAWVUDKBKUQEWQ3N4POYHRWH",
            "data":  {
                "alert": "Brian Scaramella: Test DM",
                "received_at": 1737048767000_u128,
                "subject":  {
                    "attachments":  [],
                    "avatar_url": "https://i.groupme.com/200x205.jpeg.3475af00b96f4d1d8a21a9822ddd5e3a",
                    "chat_id": "21642197+43040067",
                    "created_at": 1737048766,
                    "favorited_by":  [],
                    "id": "173704876681069898",
                    "location":  {
                        "lat": "",
                        "lng": "",
                        "name": null,
                    },
                    "name": "Brian Scaramella",
                    "picture_url": null,
                    "recipient_id": "43040067",
                    "sender_id": "21642197",
                    "sender_type": "user",
                    "source_guid": "android-c6edbdaa-a2c0-4ac9-a37b-975de0b73a8b",
                    "text": "Test DM",
                    "user_id": "21642197",
                },
                "type": "direct_message.create",
            },
            "id": "16124d04",
        },])
    }

    #[test]
    #[tracing_test::traced_test]
    fn test_msg() {
        let msg = test1();
        let msgs: Vec<PushMessage> = serde_json::from_value(msg).unwrap();
        for msg in msgs {
            info!("{:#?}", msg);
        }

        let msg = test2();
        let msgs: Vec<PushMessage> = serde_json::from_value(msg).unwrap();
        for msg in msgs {
            info!("{:#?}", msg);
        }

        let msg = test3();
        let msgs: Vec<PushMessage> = serde_json::from_value(msg).unwrap();
        for msg in msgs {
            info!("{:#?}", msg);
        }
    }
}
