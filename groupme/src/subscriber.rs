use std::time::Duration;

use crate::prelude::*;

mod push_msg;
use push_msg::{Advice, Channel, Ext, PushId, PushMessage};
pub use push_msg::{Attachment, Data};

use futures_util::{SinkExt, StreamExt};
use reqwest::Client;
use reqwest_websocket::{Message, RequestBuilderExt, WebSocket};
use tokio::task::JoinHandle;

const WEBSOCKET_URI: &str = "https://push.groupme.com/faye";

type FromWebSocketTx = broadcast::Sender<Data>;
pub type FromWebSocket = broadcast::Receiver<Data>;
pub type ToWebSocket = mpsc::Sender<PushMessage>;
type ToWebSocketRx = mpsc::Receiver<PushMessage>;

pub struct PushWebSocketServer {
    to_tx: ToWebSocket,
    to_rx: Option<ToWebSocketRx>,
    from_tx: FromWebSocketTx,
    id: PushId,
}

// TODO: clean up this interface. Hopefully we could have some kind of
// new() -> Stream<Data> or something

impl PushWebSocketServer {
    pub fn new() -> Self {
        let (to_tx, to_rx) = mpsc::channel(32);
        let from_tx = broadcast::Sender::new(32);
        let to_rx = Some(to_rx);

        return Self { to_tx, to_rx, from_tx, id: PushId::new() };
    }

    /// Initialize then spawn a run task
    pub fn start(&mut self) -> Result<JoinHandle<()>, ()> {
        // Take to_rx, pass in relevant parts...
        let to_rx = self.to_rx.take().ok_or(())?;
        let to_tx = self.get_tx();
        let from_tx = self.from_tx.clone();
        Ok(tokio::spawn(Self::start_(to_rx, to_tx, from_tx)))
    }

    async fn start_(mut to_rx: ToWebSocketRx, to_tx: ToWebSocket, from_tx: FromWebSocketTx) {
        let (meta_tx, meta_rx) = mpsc::channel(5);

        let h1 = tokio::spawn(Self::handshaker_and_subscriber(to_tx, meta_rx));

        let client = reqwest::Client::new();
        let mut ws = match client.get(WEBSOCKET_URI).upgrade().send().await {
            Ok(upgrade_resp) => match upgrade_resp.into_websocket().await {
                Ok(ws) => ws,
                Err(err) => {
                    error!("Failed to upgrade websocket: {err}");
                    return;
                }
            },
            Err(err) => {
                error!("Failed to get upgrade response: {err}");
                return;
            }
        };

        Self::run(&mut to_rx, &from_tx, meta_tx, &mut ws).await;
        h1.abort();
    }

    #[tracing::instrument(skip_all)]
    pub async fn run(
        to_rx: &mut ToWebSocketRx,
        from_tx: &FromWebSocketTx,
        meta_tx: mpsc::Sender<PushMessage>,
        ws: &mut WebSocket,
    ) {
        loop {
            tokio::select! {
                input = ws.next() => {
                    if let Some(Ok(input)) = input {
                        debug!("Got from websocket: {input:?}");
                        Self::handle_ws_msg(input, ws, from_tx, &meta_tx).await;
                    } else {
                        warn!("Websocket input closed or error, exiting");
                        break;
                    }
                }
                output = to_rx.recv() =>{
                    if let Some(output) = output {
                        debug!("Sending to websocket: {output:?}");
                        let _ = Self::forward_rx_to_ws(output, ws).await;
                    } else {
                        warn!("Websocket output channel closed");
                        break
                    }

                }
            }
        }
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
        mut from_meta: mpsc::Receiver<PushMessage>,
    ) {
        let to_tx = &to_tx;
        let from_meta = &mut from_meta;
        let mut id = PushId::new();
        let mut subscribed = false;
        loop {
            debug!("Sending handshake request");
            let req = PushMessage::new(&mut id, Channel::Handshake)
                .version("1.0")
                .supported_connection_types(&["websocket"]);
            to_tx.send(req).await.unwrap();

            // Wait for handshake success
            let client_id = loop {
                let mut n = 0;
                let msg = from_meta.recv().await.unwrap();
                n += 1;
                if n > 3 {
                    error!("Failed to get handshake response in 3 messages");
                    return;
                }
                if matches!(msg.channel, Channel::Handshake) {
                    tracing::debug!("Got handshake response: {msg:?}");
                    break msg.client_id.unwrap();
                }
            };
            if !subscribed {
                debug!("Subscribing to moderator channel");
                // Subscribe
                let sub = format!("/user/{MODERATOR_UID}");
                let req = PushMessage::new(&mut id, Channel::Subscribe)
                    .client_id(&client_id)
                    .subscription(&sub)
                    .ext(Ext::now().unwrap());
                to_tx.send(req).await.unwrap();
                subscribed = true;
            }
            // Sleep 45 min then get a new client_id
            tokio::time::sleep(Duration::from_secs(60 * 45)).await;
        }
    }

    async fn handle_ws_msg(
        input: Message,
        ws: &mut WebSocket,
        from_tx: &FromWebSocketTx,
        meta_tx: &mpsc::Sender<PushMessage>,
    ) {
        match input {
            Message::Close { code, reason } => {
                warn!("Websocket got Close Msg, closing: {:?}, {}", code, reason);
                let _ = ws.close().await;
            }
            Message::Ping(i) => {
                tracing::debug!("Got Ping: {i:?}");
                let _ = ws.send(Message::Pong(i)).await;
            }
            Message::Text(text) => {
                debug!("Got Push Message text: {text}");
                let msgs: Vec<PushMessage> = match serde_json::from_str(text.as_str()) {
                    Ok(msgs) => msgs,
                    Err(err) => {
                        error!("Failed to parse websocket message: err:{err}, {text}");
                        return;
                    }
                };
                for msg in msgs {
                    if let Channel::Moderator = msg.channel {
                        let _ = from_tx.send(msg.data.unwrap());
                    } else {
                        let _ = meta_tx.send(msg).await;
                    }
                }
            }
            other => {
                error!("Websocket got unknown Message: {other:?}");
            }
        }
    }

    async fn forward_rx_to_ws(
        output: PushMessage,
        ws: &mut WebSocket,
    ) -> Result<bool, reqwest_websocket::Error> {
        let msg = Message::Text(serde_json::to_string(&output).unwrap());
        let result = ws.send(msg).await;
        if let Err(err) = result {
            warn!("Websocket send error: {err}");
            return Ok(false);
        }
        Ok(true)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[tracing_test::traced_test]
    async fn basic() {
        info!("Starting test");
        let mut server = PushWebSocketServer::new();

        let h1 = server.start().unwrap();

        let mut from_rx = server.get_rx();

        while let Ok(Ok(msg)) = tokio::time::timeout(Duration::from_secs(10), from_rx.recv()).await
        {
            info!("{:?}", msg);
        }
    }
}
