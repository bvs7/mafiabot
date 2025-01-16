use anyhow::{self, bail, Context, Result};
use futures_util::{SinkExt, StreamExt};
use http::HeaderMap;
use reqwest::Client;
use reqwest_websocket::{CloseCode, Message, RequestBuilderExt, WebSocket};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::collections::HashMap;
use std::env;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio::sync::{broadcast, mpsc};
use tokio::task::JoinHandle;
use tracing::Level;

use crate::engine::game::Game;
use crate::engine::interface::Action;

const MODERATOR_UID: &str = "43040067";

const LOBBY_CHAT_ID: &str = "25833774";
const MAIN_CHAT_ID: &str = "105362524";
const MAFIA_CHAT_ID: &str = "105362533";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(from = "String", into = "String")]
pub struct PushId(u64);

impl From<PushId> for String {
    fn from(value: PushId) -> Self {
        format!("{:x}", value.0)
    }
}

impl From<String> for PushId {
    fn from(value: String) -> Self {
        Self(u64::from_str_radix(value.as_str(), 16).unwrap_or_else(|e| {
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
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<JsonValue>,
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
    fn _connection_type(mut self, connection_type: &str) -> Self {
        self.connection_type = Some(String::from(connection_type));
        self
    }
}

fn handshake_msg(id: &mut PushId) -> PushMessage {
    PushMessage::new(id, Channel::Handshake)
        .version("1.0")
        .supported_connection_types(&["websocket"])
}

fn subscribe_user_msg(id: &mut PushId, client_id: &str, user_id: &str) -> PushMessage {
    let sub = format!("/user/{user_id}");
    PushMessage::new(id, Channel::Subscribe)
        .client_id(&client_id)
        .subscription(&sub)
        .ext(Ext::now().unwrap())
}

fn _subscribe_group_msg(id: &mut PushId, client_id: &str, group_id: &str) -> PushMessage {
    let sub = format!("/group/{group_id}");
    PushMessage::new(id, Channel::Subscribe)
        .client_id(&client_id)
        .subscription(&sub)
        .ext(Ext::now().unwrap())
}

pub async fn get_websocket(client: &Client, uri: &str) -> Result<WebSocket> {
    Ok(client
        .get(uri)
        .upgrade()
        .send()
        .await?
        .into_websocket()
        .await?)
}

type FromWebSocketTx = broadcast::Sender<PushMessage>;
type FromWebSocketRx = broadcast::Receiver<PushMessage>;
type ToWebSocketTx = mpsc::Sender<PushMessage>;
type ToWebSocketRx = mpsc::Receiver<PushMessage>;

async fn handle_ws_msg(
    input: Option<Result<Message, reqwest_websocket::Error>>,
    ws: &mut WebSocket,
    tx: &FromWebSocketTx,
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
                tx.send(msg)?;
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

async fn websocket_handler(
    mut ws: WebSocket,
    tx: FromWebSocketTx,
    mut rx: ToWebSocketRx,
) -> Result<()> {
    while tokio::select! {
        input = ws.next() => handle_ws_msg(input, &mut ws, &tx).await?,
        output = rx.recv() => forward_rx_to_ws(output, &mut ws).await?,
    } {}
    ws.close(CloseCode::Normal, None).await?;
    Ok(())
}

async fn handshaker_and_subscriber(
    tx: ToWebSocketTx,
    mut rx: FromWebSocketRx,
    mut id: PushId,
) -> Result<()> {
    let mut subscribed = false;
    loop {
        let req = handshake_msg(&mut id);
        tx.send(req).await?;

        // Wait for handshake success
        let client_id = loop {
            let mut n = 0;
            let msg = rx.recv().await?;
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
            let req = subscribe_user_msg(&mut id, &client_id, MODERATOR_UID);
            tx.send(req).await?;
            subscribed = true;
        }
        // Sleep 45 min then get a new client_id
        tokio::time::sleep(Duration::from_secs(60 * 45)).await;
    }
    Ok(())
}

async fn msg_tracer(mut rx: FromWebSocketRx) {
    loop {
        match rx.recv().await {
            Ok(msg) => tracing::info!("Received {msg:#?}"),
            Err(broadcast::error::RecvError::Closed) => break,
            Err(broadcast::error::RecvError::Lagged(n)) => tracing::warn!("Missed {n} messages"),
        }
    }
}

async fn start_websocket_handler(
    uri: &str,
    client: Client,
) -> Result<(FromWebSocketTx, ToWebSocketTx, JoinHandle<()>)> {
    let from_websocket_tx = broadcast::Sender::new(100);
    let (to_websocket_tx, to_websocket_rx) = mpsc::channel(100);

    let h1 = tokio::spawn(handshaker_and_subscriber(
        to_websocket_tx.clone(),
        from_websocket_tx.subscribe(),
        PushId(0),
    ));

    let ws = get_websocket(&client, uri).await?;

    let h2 = tokio::spawn(websocket_handler(
        ws,
        from_websocket_tx.clone(),
        to_websocket_rx,
    ));

    let h = tokio::spawn(async move {
        let (r1, r2) = tokio::join!(h1, h2,);
        match r1 {
            Ok(Ok(())) => {}
            Err(err) => tracing::error!(?err),
            Ok(Err(err)) => tracing::error!(?err),
        }
        match r2 {
            Ok(Ok(())) => {}
            Err(err) => tracing::error!(?err),
            Ok(Err(err)) => tracing::error!(?err),
        }
    });

    Ok((from_websocket_tx, to_websocket_tx, h))
}

pub async fn run() {
    let uri = "https://push.groupme.com/faye";
    let client = Client::new();

    let mut ctrl = Controller {
        games: HashMap::new(),
        game_chats: HashMap::new(),
        targeter_games: HashMap::new(),
    };

    ctrl.targeter_games.insert(21642197, 1);
    ctrl.game_chats
        .insert(MAIN_CHAT_ID.parse().unwrap(), (GameChatKind::Main, 1));

    let Ok((from_tx, to_tx, h)) = start_websocket_handler(uri, client).await else {
        return;
    };

    let h2 = tokio::spawn(msg_tracer(from_tx.subscribe()));
    let h3 = tokio::spawn(async move {
        let mut ctrl = ctrl;
        ctrl.process_msg_data(from_tx.subscribe()).await;
    });

    let _ = tokio::join!(h, h2, h3);
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

/* What kinds of commands or interactions can we have?
// Command
// - Lobby
// - Game
// - DM
// Action
// - Game
// - DM
*/

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case", tag = "type")]
enum Attachment {
    Mentions {
        user_ids: Vec<String>,
        loci: Vec<Vec<usize>>,
    },
    #[serde(other)]
    Unknown,
}

#[derive(Debug, Clone, Deserialize)]
struct Subject {
    attachments: Vec<Attachment>,
    created_at: u64,
    group_id: Option<String>,
    chat_id: Option<String>,
    #[serde(rename = "id")]
    message_id: String,
    sender_id: String,
    sender_type: String,
    text: String,
    user_id: String,
}

#[derive(Debug, Clone, Deserialize)]
struct MsgData {
    received_at: u64,
    subject: Subject,
    #[serde(rename = "type")]
    type_: String,
}

#[derive(Debug, Clone)]
enum GameCmd {
    Status,
    Help,
    Rules,
}

#[derive(Debug, Clone)]
enum LobbyCmd {
    Start { minutes: u32, min_players: usize },
    Status,
    Help,
}

#[derive(Debug, Clone)]
enum DMCmd {
    Status,
    Help,
}

#[derive(Debug, Clone)]
enum Source {
    Group(u64),
    DM,
}

#[derive(Debug, Clone)]
enum AppCmd {
    Other(String),
}

#[derive(Debug, Clone)]
enum Cmd {
    GameAction {
        game_id: u64,
        action: Action,
    },
    GameCmd {
        game_id: u64,
        cmd: GameCmd,
    },
    DMCmd {
        user: u64,
        cmd: DMCmd,
    },
    LobbyCmd {
        lobby: u64,
        cmd: LobbyCmd,
    },
    AppCmd {
        user: u64,
        source: Source,
        cmd: AppCmd,
    },
}

// We have a list of games with their mafia and main chats, right?

#[derive(Debug, Clone)]
enum GameChatKind {
    Main,
    Mafia,
}

#[derive(Debug)]
struct GameHolder {
    game: Game,
    living: Vec<u64>,
}

#[derive(Debug)]
struct Controller {
    games: HashMap<u64, GameHolder>,               // game_id -> game
    game_chats: HashMap<u64, (GameChatKind, u64)>, // chat_id -> (kind, game_id)
    targeter_games: HashMap<u64, u64>,             // user_id -> game_id
}

impl Controller {
    fn msg_data_to_command(&self, msg_data: MsgData) -> Option<Cmd> {
        let mut text = msg_data.subject.text.clone();
        let first_char = text.chars().nth(0)?;
        if first_char != '/' {
            return None; // No command specifier
        }
        let mut words = text.split(' ');
        let first = words.next()?;

        let user: u64 = msg_data.subject.user_id.parse().ok()?;

        if &msg_data.type_ == "direct_message.create" {
            if first == "/help" {
                return Some(Cmd::DMCmd {
                    user,
                    cmd: DMCmd::Help,
                });
            } else if first == "/status" {
                return Some(Cmd::DMCmd {
                    user,
                    cmd: DMCmd::Status,
                });
            } else if first == "/target" {
                let actor = user;
                let mut choice: Option<usize> = None;
                if let Some(next) = words.next() {
                    choice = next.parse().ok();
                }
                let game_id = *self.targeter_games.get(&actor)?;
                let choice = if let Some(choice_n) = choice {
                    let living = &self.games.get(&game_id)?.living;
                    let choice: u64 = *living.get(choice_n)?;
                    Some(choice)
                } else {
                    None
                };

                return Some(Cmd::GameAction {
                    game_id,
                    action: Action::Target { actor, choice },
                });
            } else {
                return Some(Cmd::AppCmd {
                    user,
                    source: Source::DM,
                    cmd: AppCmd::Other(first.to_owned()),
                });
            }
        }

        if &msg_data.type_ == "line.create" {
            let group_id = &msg_data.subject.group_id?;
            if group_id == LOBBY_CHAT_ID {
                let lobby = LOBBY_CHAT_ID.parse().ok()?;
                if first == "/status" {
                    return Some(Cmd::LobbyCmd {
                        lobby,
                        cmd: LobbyCmd::Status,
                    });
                } else if first == "/help" {
                    return Some(Cmd::LobbyCmd {
                        lobby,
                        cmd: LobbyCmd::Help,
                    });
                } else if first == "/start" {
                    let mut minutes: u32 = 10;
                    let mut min_players: usize = 7;
                    if let Some(m) = words.next() {
                        if let Ok(m) = m.parse() {
                            minutes = m;
                        }
                    }
                    if let Some(m) = words.next() {
                        if let Ok(m) = m.parse() {
                            min_players = m;
                        }
                    }
                    return Some(Cmd::LobbyCmd {
                        lobby,
                        cmd: LobbyCmd::Start {
                            minutes,
                            min_players,
                        },
                    });
                }
            }

            let g_id: u64 = group_id.parse().ok()?;
            // Look for an associated game
            if let Some((kind, game)) = self.game_chats.get(&g_id) {
                let game_id = game.clone();
                if matches!(kind, GameChatKind::Main) {
                    // Check for a game action
                    if first == "/vote" {
                        let voter = user;
                        // Check for a mention next
                        let mut ballot = None;
                        if let Some(mut mention_user_ids) = msg_data
                            .subject
                            .attachments
                            .iter()
                            .filter_map(|a| match a {
                                Attachment::Mentions { user_ids: u, .. } => Some(u),
                                _ => None,
                            })
                            .cloned()
                            .nth(0)
                        {
                            let choice_str = mention_user_ids.pop();
                            ballot = choice_str.map(|s| s.parse::<u64>().ok());
                        }
                        if ballot.is_none() {
                            if let Some("nokill") = words.next() {
                                ballot = Some(None);
                            }
                        }
                        return Some(Cmd::GameAction {
                            game_id,
                            action: Action::Vote { voter, ballot },
                        });
                    } else if first == "/status" {
                        return Some(Cmd::GameCmd {
                            game_id,
                            cmd: GameCmd::Status,
                        });
                    } else if first == "/help" {
                        return Some(Cmd::GameCmd {
                            game_id,
                            cmd: GameCmd::Help,
                        });
                    } else if first == "/rules" {
                        return Some(Cmd::GameCmd {
                            game_id,
                            cmd: GameCmd::Rules,
                        });
                    }
                } else if matches!(kind, GameChatKind::Mafia) {
                    if first == "/target" {
                        let killer = user;
                        let mut choice = None;
                        if let Some(next) = words.next() {
                            choice = next.parse().ok();
                        }
                        return Some(Cmd::GameAction {
                            game_id,
                            action: Action::Scheme {
                                killer,
                                mark: choice,
                            },
                        });
                    }
                }
            }
            return Some(Cmd::AppCmd {
                user,
                source: Source::Group(g_id),
                cmd: AppCmd::Other(first.to_owned()),
            });
        }
        None
    }

    async fn process_msg_data(&mut self, mut rx: FromWebSocketRx) {
        loop {
            match rx.recv().await {
                Ok(msg) => {
                    if let Some(data) = msg.data {
                        // Try to deserialize to msg data
                        match serde_json::from_value::<MsgData>(data) {
                            Ok(msg_data) => {
                                tracing::info!("Message Data: {msg_data:#?}");
                                if let Some(cmd) = self.msg_data_to_command(msg_data) {
                                    tracing::info!("Cmd: {cmd:#?}");
                                }
                            }
                            Err(err) => tracing::error!("Error parsing Message Data: {err:?}"),
                        }
                    }
                }
                Err(broadcast::error::RecvError::Closed) => break,
                Err(broadcast::error::RecvError::Lagged(_)) => {}
            }
        }
    }
}

/*
TODO:
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

/* Message "Check" in Main Chat from Brian
Json: Array [
    Object {
        "channel": String("/user/43040067"),
        "clientId": String("fayeF34IPC2IHND7HULV5WAZW2EZNKBMKVO"),
        "data": Object {
            "alert": String("Brian \"Testing\" Scaramella: Test"),
            "received_at": Number(1737048635000),
            "subject": Object {
                "attachments": Array [],
                "avatar_url": String("https://i.groupme.com/200x205.jpeg.3475af00b96f4d1d8a21a9822ddd5e3a"),
                "created_at": Number(1737048634),
                "deleted_at": Null,
                "deletion_actor": Null,
                "group_id": String("105362524"),
                "id": String("173704863488783901"),
                "location": Object {
                    "lat": String(""),
                    "lng": String(""),
                    "name": Null,
                },
                "name": String("Brian \"Testing\" Scaramella"),
                "parent_id": Null,
                "picture_url": Null,
                "pinned_at": Null,
                "pinned_by": Null,
                "sender_id": String("21642197"),
                "sender_type": String("user"),
                "source_guid": String("android-bbfe3445-d65e-4352-a613-19df30dede79"),
                "system": Bool(false),
                "text": String("Test"),
                "updated_at": Null,
                "user_id": String("21642197"),
            },
            "type": String("line.create"),
        },
        "id": String("16122cff"),
    },
]
*/

/* Message "/vote @Brian" in Main Chat from Moderator
Json: Array [
    Object {
        "channel": String("/user/43040067"),
        "clientId": String("fayeH7MREDZPQTEG2VHXK2UEDRPO6UR2QP2"),
        "data": Object {
            "alert": String("MODERATOR: /vote @Brian \"Testing\" Scaramella "),
            "received_at": Number(1737048690000),
            "subject": Object {
                "attachments": Array [
                    Object {
                        "loci": Array [
                            Array [
                                Number(6),
                                Number(27),
                            ],
                        ],
                        "type": String("mentions"),
                        "user_ids": Array [
                            Number(21642197),
                        ],
                    },
                ],
                "avatar_url": String("https://i.groupme.com/1920x1080.jpeg.247b8c490afd429f88e239a4914a436d"),
                "created_at": Number(1737048690),
                "deleted_at": Null,
                "deletion_actor": Null,
                "group_id": String("105362524"),
                "id": String("173704869054202031"),
                "location": Object {
                    "lat": String(""),
                    "lng": String(""),
                    "name": Null,
                },
                "name": String("MODERATOR"),
                "parent_id": Null,
                "picture_url": Null,
                "pinned_at": Null,
                "pinned_by": Null,
                "sender_id": String("43040067"),
                "sender_type": String("user"),
                "source_guid": String("ba9a45dff754aa1bcb060a1eb7760135"),
                "system": Bool(false),
                "text": String("/vote @Brian \"Testing\" Scaramella "),
                "updated_at": Null,
                "user_id": String("43040067"),
            },
            "type": String("line.create"),
        },
        "id": String("17d46b97"),
    },
] */
/* DM from Brian to Moderator "Test DM"
Json: Array [
    Object {
        "channel": String("/user/43040067"),
        "clientId": String("fayeLJBD7D2WAWVUDKBKUQEWQ3N4POYHRWH"),
        "data": Object {
            "alert": String("Brian Scaramella: Test DM"),
            "received_at": Number(1737048767000),
            "subject": Object {
                "attachments": Array [],
                "avatar_url": String("https://i.groupme.com/200x205.jpeg.3475af00b96f4d1d8a21a9822ddd5e3a"),
                "chat_id": String("21642197+43040067"),
                "created_at": Number(1737048766),
                "favorited_by": Array [],
                "id": String("173704876681069898"),
                "location": Object {
                    "lat": String(""),
                    "lng": String(""),
                    "name": Null,
                },
                "name": String("Brian Scaramella"),
                "picture_url": Null,
                "recipient_id": String("43040067"),
                "sender_id": String("21642197"),
                "sender_type": String("user"),
                "source_guid": String("android-c6edbdaa-a2c0-4ac9-a37b-975de0b73a8b"),
                "text": String("Test DM"),
                "user_id": String("21642197"),
            },
            "type": String("direct_message.create"),
        },
        "id": String("16124d04"),
    },
] */
