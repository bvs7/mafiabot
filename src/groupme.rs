use anyhow::{self, Context, Result};
use futures_util::stream::SplitStream;
use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use reqwest_websocket::{Message, RequestBuilderExt, WebSocket};
use serde::{Deserialize, Serialize};
use std::env;
use std::time::{SystemTime, UNIX_EPOCH};

const MODERATOR_UID: &str = "43040067";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub struct PushId(u64);

impl From<PushId> for String {
    fn from(value: PushId) -> Self {
        value.0.to_string()
    }
}

impl From<String> for PushId {
    fn from(value: String) -> Self {
        Self(value.parse().unwrap_or_default())
    }
}

impl PushId {
    pub fn inc_and_clone(&mut self) -> Self {
        self.0 += 1;
        Self(self.0)
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
enum Channel {
    #[default]
    Handshake,
    Subscribe,
    Connect,
    Unknown(String),
}

impl From<String> for Channel {
    fn from(value: String) -> Self {
        match value.as_str() {
            "/meta/handshake" => Self::Handshake,
            "/meta/subscribe" => Self::Subscribe,
            "/meta/connect" => Self::Connect,
            _ => Self::Unknown(value),
        }
    }
}

impl From<Channel> for String {
    fn from(value: Channel) -> Self {
        match value {
            Channel::Handshake => String::from("/meta/handshake"),
            Channel::Subscribe => String::from("/meta/subscribe"),
            Channel::Connect => String::from("/meta/connect"),
            Channel::Unknown(u) => u,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Advice {
    reconnect: String,
    interval: u32,
    timeout: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Ext {
    access_token: String,
    timestamp: u64,
}

impl Ext {
    fn now() -> Result<Self> {
        Ok(Self {
            access_token: get_token()?,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PushMessage {
    id: PushId,
    channel: Channel,
    #[serde(skip_serializing_if = "Option::is_none")]
    client_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    version: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    supported_connection_types: Option<Vec<String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    subscription: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    advice: Option<Advice>,
    #[serde(skip_serializing_if = "Option::is_none")]
    successful: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none", skip_deserializing)]
    ext: Option<Ext>,
    #[serde(skip_serializing_if = "Option::is_none")]
    connection_type: Option<String>,
}

impl PushMessage {
    fn new(id: &mut PushId, channel: Channel) -> Self {
        Self {
            id: id.inc_and_clone(),
            channel,
            ..Self::default()
        }
    }

    fn version(mut self, version: &str) -> Self {
        self.version = Some(String::from(version));
        self
    }
    fn supported_connection_types(mut self, value: &[&str]) -> Self {
        let v = value.into_iter().map(|s| String::from(*s)).collect();
        self.supported_connection_types = Some(v);
        self
    }
    fn subscription(mut self, subscription: &str) -> Self {
        self.subscription = Some(String::from(subscription));
        self
    }
    fn client_id(mut self, client_id: &str) -> Self {
        self.client_id = Some(String::from(client_id));
        self
    }
    fn ext(mut self, ext: Ext) -> Self {
        self.ext = Some(ext);
        self
    }
    fn connection_type(mut self, connection_type: &str) -> Self {
        self.connection_type = Some(String::from(connection_type));
        self
    }
}

fn handshake_msg(id: &mut PushId) -> PushMessage {
    PushMessage::new(id, Channel::Handshake)
        .version("1.0")
        .supported_connection_types(&["websocket"])
}

fn subscribe_msg(id: &mut PushId, client_id: &str) -> PushMessage {
    let sub = format!("/user/{MODERATOR_UID}");
    PushMessage::new(id, Channel::Subscribe)
        .client_id(&client_id)
        .subscription(&sub)
        .ext(Ext::now().unwrap())
}

pub async fn get_websocket(client: &Client, uri: &str) -> Result<WebSocket> {
    Ok(client
        .get("https://push.groupme.com/faye")
        .upgrade()
        .send()
        .await?
        .into_websocket()
        .await?)
}

pub async fn try_websocket_subscribe() -> Result<()> {
    let uri = "https://push.groupme.com/faye";
    let mut id = PushId(0);
    let client = Client::new();
    let websocket = get_websocket(&client, uri).await?;
    let (mut tx, mut rx) = websocket.split();
    let req = handshake_msg(&mut id);
    tracing::info!("Request: {req:#?}");
    tx.send(Message::Text(serde_json::to_string(&req)?)).await?;

    let result = loop {
        if let Some(message) = rx.next().await {
            let msg = message?;
            tracing::debug!("Got {msg:?}");
            match msg {
                Message::Text(t) => break t,
                _ => {}
            }
        }
    };

    let mut resp: Vec<PushMessage> = serde_json::from_str(&result)?;
    let resp = resp.pop().with_context(|| "No resp")?;
    tracing::info!("Response: {resp:#?}");

    let client_id = resp.client_id.with_context(|| "Missing client_id")?;

    let req = subscribe_msg(&mut id, &client_id);
    tx.send(Message::Text(serde_json::to_string(&req)?)).await?;

    Ok(())
}

fn get_token() -> anyhow::Result<String> {
    env::var("GROUPME_TOKEN").with_context(|| "failed to get groupme token")
}

pub async fn get_user_id() -> anyhow::Result<()> {
    let client = reqwest::Client::new();
    let resp = client
        .get("https://api.groupme.com/v3/users/me")
        .query(&[("token", get_token()?)])
        .send()
        .await?;

    tracing::info!("Resp: {:#?}", resp);

    let body = resp.text().await?;
    let json_body = serde_json::from_str::<serde_json::Value>(&body);

    tracing::info!("Body: {:#?}", json_body);
    Ok(())
}

fn example() {
    let _ = serde_json::json!(
        [
            {
                "channel":"/user/43040067",
                "clientId":"...",
                "data":{
                    "alert":"Brian Scaramella: Test please",
                    "subject": {
                        "attachments":[],
                        "avatary_url": "...",
                        "...": "...",
                        "group_id": "105362524",
                        "name": "Brian Scaramella",
                        "sender_id": "21642197",
                        "sender_type": "user",
                        "text": "Test please",
                        "user_id": "216422197",
                    },
                }
            }
        ]
    );
}

/*
TODO:
- Try subscribing to a specific chat, not a user
- Try subscribing to multiple chats and routing the input
- Create methods to pull out relevant message data
- Add a task that updates the signature every hour
- Use API to add a user to a group, with a name
- Make basic lobby functions (in a more generic file than here)
*/

/*
Message Example:
[

]
"[{\"channel\":\"/user/43040067\",\"clientId\":\"fayeG3DISD3RLD6KOKLG4LJZTFDQD3LWOTW\",\"id\":\"1591d5f8\",\"data\":{\"alert\":\"Brian Scaramella: Test please\",
\"subject\":{\"attachments\":[],\"avatar_url\":\"https://i.groupme.com/200x205.jpeg.3475af00b96f4d1d8a21a9822ddd5e3a\",\"created_at\":1736985478,\"deleted_at\":null,
\"deletion_actor\":null,\"group_id\":\"105362524\",\"id\":\"173698547834459350\",\"location\":{\"lat\":\"\",\"lng\":\"\",\"name\":null},\"name\":\"Brian Scaramella\",
\"parent_id\":null,\"picture_url\":null,\"pinned_at\":null,\"pinned_by\":null,\"sender_id\":\"21642197\",\"sender_type\":\"user\",
\"source_guid\":\"android-ab187b81-37e3-4d2e-b0c1-9636e1dbe998\",\"system\":false,\"text\":\"Test please\",\"updated_at\":null,\"user_id\":\"21642197\"},
\"type\":\"line.create\",\"received_at\":1736985478000}}]"

*/
