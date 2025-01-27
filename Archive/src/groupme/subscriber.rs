/// A websocket client that subscribes to the "MODERATOR" user
use anyhow::{self, bail, Context, Result};
use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use reqwest_websocket::{Message, RequestBuilderExt, WebSocket};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;

use super::util::{get_token, GroupId, MessageId, UserId, MODERATOR_UID};

const WEBSOCKET_URI: &str = "https://push.groupme.com/faye";

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
            timestamp: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs(),
        })
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PushMessage {
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
    #[serde(skip_serializing)]
    data: Option<Data>,
}

impl PushMessage {
    fn new(id: &mut PushId, channel: Channel) -> Self {
        Self { id: id.inc_and_clone(), channel, ..Self::default() }
    }

    pub fn data(&self) -> &Option<Data> {
        &self.data
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
    fn _connection_type(mut self, connection_type: &str) -> Self {
        self.connection_type = Some(String::from(connection_type));
        self
    }
}

#[derive(Debug)]
pub enum Error {
    AlreadyStarted,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::AlreadyStarted => write!(f, "server already started"),
        }
    }
}

impl std::error::Error for Error {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        None
    }
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
enum Attachment {
    #[serde(rename = "mentions")]
    Mentions { user_ids: Vec<UserId> },
    #[serde(other)]
    Unknown,
}

// TODO: make UserId a newtypes

#[derive(Debug, Clone, Deserialize)]
struct Subject {
    attachments: Vec<Attachment>,
    #[serde(alias = "chat_id")]
    group_id: GroupId,
    created_at: u64,
    id: MessageId,
    name: String,
    text: String,
    user_id: UserId,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", content = "subject")]
enum Data {
    #[serde(rename = "line.create")]
    GroupMsg { subject: Subject, received_at: u128 },
    #[serde(rename = "direct_message.create")]
    DirectMsg { subject: Subject, received_at: u128 },
    #[serde(other)]
    Unknown,
}

type FromWebSocketTx = broadcast::Sender<PushMessage>;
pub type FromWebSocket = broadcast::Receiver<PushMessage>;
pub type ToWebSocket = mpsc::Sender<PushMessage>;
type ToWebSocketRx = mpsc::Receiver<PushMessage>;

pub struct PushWebSocketServer {
    to_tx: ToWebSocket,
    to_rx: Option<ToWebSocketRx>,
    from_tx: FromWebSocketTx,
    id: PushId,
}

impl PushWebSocketServer {
    pub fn new() -> Self {
        let (to_tx, to_rx) = mpsc::channel(100);
        let from_tx = broadcast::Sender::new(100);
        let to_rx = Some(to_rx);

        return Self { to_tx, to_rx, from_tx, id: PushId(0) };
    }

    /// Initialize then spawn a run task
    pub fn start(&mut self) -> Result<JoinHandle<Result<()>>, Error> {
        // Take to_rx, pass in relevant parts...
        let to_rx = self.to_rx.take().ok_or(Error::AlreadyStarted)?;
        let to_tx = self.get_tx();
        let from_tx = self.from_tx.clone();
        Ok(tokio::spawn(Self::start_(to_rx, to_tx, from_tx)))
    }

    async fn start_(
        mut to_rx: ToWebSocketRx,
        to_tx: ToWebSocket,
        from_tx: FromWebSocketTx,
    ) -> Result<()> {
        let h1 = tokio::spawn(Self::handshaker_and_subscriber(to_tx, from_tx.subscribe()));

        let client = Client::new();
        let ws = &mut client.get(WEBSOCKET_URI).upgrade().send().await?.into_websocket().await?;

        Self::run(&mut to_rx, &from_tx, ws).await?;
        h1.abort();
        Ok(())
    }

    #[tracing::instrument(skip_all)]
    pub async fn run(
        to_rx: &mut ToWebSocketRx,
        from_tx: &FromWebSocketTx,
        ws: &mut WebSocket,
    ) -> Result<()> {
        while tokio::select! {
            input = ws.next() => Self::handle_ws_msg(input,ws, from_tx).await?,
            output = to_rx.recv() => Self::forward_rx_to_ws(output,ws).await?,
        } {}

        Ok(())
    }

    pub fn get_rx(&self) -> FromWebSocket {
        self.from_tx.subscribe()
    }

    pub fn get_tx(&self) -> ToWebSocket {
        self.to_tx.clone()
    }

    #[tracing::instrument(skip_all)]
    async fn handshaker_and_subscriber(
        to_tx: ToWebSocket,
        mut from_rx: FromWebSocket,
    ) -> Result<()> {
        let to_tx = &to_tx;
        let from_rx = &mut from_rx;
        let id = &mut PushId(0);
        let mut subscribed = false;
        loop {
            let req = PushMessage::new(id, Channel::Handshake)
                .version("1.0")
                .supported_connection_types(&["websocket"]);
            to_tx.send(req).await?;

            // Wait for handshake success
            let client_id = loop {
                let mut n = 0;
                let msg = from_rx.recv().await?;
                n += 1;
                if n > 3 {
                    bail!("Failed to get handshake resp!");
                }
                if matches!(msg.channel, Channel::Handshake) {
                    break msg.client_id.with_context(|| "Did not get client id")?;
                }
            };
            if !subscribed {
                // Subscribe
                let sub = format!("/user/{MODERATOR_UID}");
                let req = PushMessage::new(id, Channel::Subscribe)
                    .client_id(&client_id)
                    .subscription(&sub)
                    .ext(Ext::now().unwrap());
                to_tx.send(req).await?;
                subscribed = true;
            }
            // Sleep 45 min then get a new client_id
            tokio::time::sleep(Duration::from_secs(60 * 45)).await;
        }
    }

    fn _subscribe_group_msg(id: &mut PushId, client_id: &str, group_id: &str) -> PushMessage {
        let sub = format!("/group/{group_id}");
        PushMessage::new(id, Channel::Subscribe)
            .client_id(&client_id)
            .subscription(&sub)
            .ext(Ext::now().unwrap())
    }

    async fn handle_ws_msg(
        input: Option<Result<Message, reqwest_websocket::Error>>,
        ws: &mut WebSocket,
        from_tx: &FromWebSocketTx,
    ) -> Result<bool> {
        let Some(result) = input else {
            return Ok(true);
        };
        let msg = result?;
        match msg {
            Message::Close { code, reason } => {
                tracing::warn!(msg = "Websocket got Close Msg, closing", ?code, reason);
                ws.close().await?;
                return Ok(false);
            }
            Message::Ping(i) => {
                tracing::debug!("Got Ping: {i:?}");
                ws.send(Message::Pong(i)).await?;
            }
            Message::Text(text) => {
                let msgs: Vec<PushMessage> = serde_json::from_str(text.as_str())?;
                for msg in msgs {
                    from_tx.send(msg)?;
                }
            }
            other => {
                tracing::error!("Websocket got unknown Message: {other:?}");
            }
        }
        Ok(true)
    }

    async fn forward_rx_to_ws(output: Option<PushMessage>, ws: &mut WebSocket) -> Result<bool> {
        if let Some(msg) = output {
            let msg = Message::Text(serde_json::to_string(&msg)?);
            ws.send(msg).await?;
        } else {
            return Ok(false);
        }
        Ok(true)
    }

    #[tracing::instrument(skip_all)]
    async fn msg_tracer(mut rx: FromWebSocket) {
        loop {
            match rx.recv().await {
                Ok(msg) => tracing::info!("Received {msg:#?}"),
                Err(broadcast::error::RecvError::Closed) => break,
                Err(broadcast::error::RecvError::Lagged(n)) => {
                    tracing::warn!("Missed {n} messages")
                }
            }
        }
    }
}
#[cfg(test)]
mod test {

    use super::*;
    use serde_json::{json, Value as JsonValue};

    fn test1() -> JsonValue {
        json!([
            {
                "channel": "/user/43040067",
                "clientId": "fayeF34IPC2IHND7HULV5WAZW2EZNKBMKVO",
                "data": {
                    "alert": "Brian \"Testing\" Scaramella: Test",
                    "received_at": 1737048635000,
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
            "data":  {
                "received_at": 1737048690000,
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
        },])
    }

    fn test3() -> JsonValue {
        json!([{
            "channel": "/user/43040067",
            "clientId": "fayeLJBD7D2WAWVUDKBKUQEWQ3N4POYHRWH",
            "data":  {
                "alert": "Brian Scaramella: Test DM",
                "received_at": 1737048767000,
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
    fn parse_group_msg1() {
        let value = test1();
        let msgs = serde_json::from_value::<Vec<PushMessage>>(value).unwrap();
        let data: Data = msgs.get(0).expect("Single msg").data().clone().unwrap();
        match data {
            Data::GroupMsg { subject, received_at } => {
                assert_eq!(subject.group_id, GroupId(105362524));
                assert_eq!(subject.user_id, UserId(21642197));
                assert_eq!(subject.id, MessageId(173704863488783901));
                assert_eq!(subject.text, "Test");
                assert_eq!(subject.name, "Brian \"Testing\" Scaramella");
                assert_eq!(subject.attachments.len(), 0);
            }
            _ => panic!("Wrong data type"),
        }
    }

    #[test]
    fn parse_group_msg2() {
        let value = test2();
        let msgs = serde_json::from_value::<Vec<PushMessage>>(value).unwrap();
        let data: Data = msgs.get(0).expect("Single msg").data().clone().unwrap();
        match data {
            Data::GroupMsg { subject, received_at } => {
                assert_eq!(subject.group_id, GroupId(105362524));
                assert_eq!(subject.user_id, UserId(43040067));
                assert_eq!(subject.id, MessageId(173704869054202031));
                assert_eq!(subject.text, "/vote @Brian \"Testing\" Scaramella ");
                assert_eq!(subject.name, "MODERATOR");
                assert_eq!(subject.attachments.len(), 1);
                match subject.attachments.get(0).expect("Single attachment") {
                    Attachment::Mentions { user_ids } => {
                        assert_eq!(user_ids.len(), 1);
                        assert_eq!(user_ids.get(0).expect("Single user_id"), &UserId(21642197));
                    }
                    _ => panic!("Wrong attachment type"),
                }
            }
            _ => panic!("Wrong data type"),
        }
    }

    #[test]
    fn parse_direct_msg() {
        let value = test3();
        let msgs = serde_json::from_value::<Vec<PushMessage>>(value).unwrap();
        let data: Data = msgs.get(0).expect("Single msg").data().clone().unwrap();
        match data {
            Data::DirectMsg { subject, received_at } => {
                assert_eq!(subject.user_id, UserId(21642197));
                assert_eq!(subject.id, MessageId(173704876681069898));
                assert_eq!(subject.text, "Test DM");
                assert_eq!(subject.name, "Brian Scaramella");
                assert_eq!(subject.attachments.len(), 0);
            }
            _ => panic!("Wrong data type"),
        }
    }
}
