use std::{env, future::Future, path::PathBuf, sync::Arc};

use async_trait::async_trait;
use tokio::{
    sync::{watch, RwLock},
    time::error::Elapsed,
};

use crate::prelude::*;

use super::action;

#[derive(thiserror::Error, Debug)]
pub enum GameIdError {
    #[error("Failed to read game_id file: {0}")]
    ReadError(#[from] std::io::Error),
    #[error("Failed to parse game_id file: {0}")]
    ParseError(#[from] std::num::ParseIntError),
    #[error("Failed to write game_id file")]
    WriteError,
}

#[derive(Debug, Clone, Copy, Default, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(into = "u64", from = "u64")]
pub struct GameId(u64);

impl GameId {
    const DEFAULT_DIR: &'static str = "mafia";

    pub fn new() -> Result<Self, GameIdError> {
        // First try env variable to get mafia directory
        let dir = match env::var("MAFIA_DIR").map(PathBuf::from) {
            Ok(dir) => dir,
            Err(err) => {
                warn!("Failed to get MAFIA_DIR: {}", err);
                PathBuf::from(Self::DEFAULT_DIR)
            }
        };
        let fname = dir.join("game_id");
        let id: u64 = std::fs::read_to_string(&fname)?.parse()?;
        // Write the file with id + 1
        std::fs::write(&fname, (id + 1).to_string()).unwrap_or_else(|_| {
            error!("Failed to write game_id file");
        });

        Ok(Self(id))
    }
}

impl From<GameId> for u64 {
    fn from(value: GameId) -> Self {
        value.0
    }
}
impl From<u64> for GameId {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for GameId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct Game {
    id: GameId,
    state: State,
}

impl Game {
    pub async fn create<P, E, A>(
        players: impl IntoIterator<Item = impl Into<Pid>>,
        rules: Rules,
        mut action_handler: A,
        mut event_handler: E,
    ) where
        P: Into<Pid> + Copy,
        A: ActionHandler<P>,
        E: EventHandler + Send + 'static,
    {
        let (event_tx, event_rx) = mpsc::unbounded_channel();

        let roles: Vec<Role> = Vec::new(); // Generate roles from rules!
        let registry = players.into_iter().zip(roles).collect::<Vec<_>>();
        let mut state = State::new(registry, rules);
        state.event_tx = Some(event_tx);

        let game = Self { id: GameId::new().unwrap_or_default(), state };

        action_handler.init(&game);
        event_handler.init(&game);

        tokio::spawn(Game::event_handler(event_rx, event_handler));

        game.action_handler(action_handler).await;
    }

    #[instrument(skip_all)]
    pub async fn event_handler(
        mut event_rx: mpsc::UnboundedReceiver<Event>,
        mut handler: impl EventHandler,
    ) {
        loop {
            match event_rx.recv().await {
                Some(event) => handler.handle_event(event).await,
                None => {
                    info!("Event channel closed");
                    break;
                }
            }
        }
    }

    #[instrument(skip_all)]
    pub async fn action_handler<P, A>(mut self, mut handler: A)
    where
        P: Into<Pid> + Copy,
        A: ActionHandler<P>,
    {
        loop {
            let timeout = self.state.update();
            handler.update_status(&self.state).await;
            let dur = match timeout.map(|t| (t - Local::now()).to_std()) {
                Some(Ok(dur)) => dur,           // Wait for timeout
                Some(Err(e)) => Duration::ZERO, // Time already lapsed
                None => Duration::MAX,          // No timeout to wait for
            };

            let action = match tokio::time::timeout(dur, handler.recv_action()).await {
                Err(Elapsed { .. }) => continue,
                Ok(None) => break,
                Ok(Some(action)) => action,
            };

            let result = self.state.validate_action(action);
            match result {
                Err(err) => handler.resp_action(Err(err)).await,
                Ok(action) => {
                    handler.resp_action(Ok(())).await;
                    self.state.perform_action(action);
                }
            }
        }
    }
}

// type RespContext = oneshot::Sender<Result<(), Error>>;

// struct Statuses {
//     games: HashMap<GameId, Status>,
//     lobbies: HashMap<u64, HashMap<u64, String>>,
// }

// struct GroupMeGroup {
//     id: u64,
//     names: HashMap<u64, String>,
// }

// struct BasicActionHandler {
//     game_id: GameId,
//     main_chat_id: Option<u64>,
//     mafia_chat_id: Option<u64>,
//     app_comms: Arc<RwLock<AppComms>>,
//     action_rx: mpsc::Receiver<(Action<u64>, RespContext)>,
//     resp: Option<RespContext>,
//     status_tx: watch::Sender<Statuses>,
// }

// #[async_trait]
// impl ActionHandler<u64> for BasicActionHandler {
//     fn init(&mut self, game: &Game) {
//         // Create Main Chat and Mafia Chat...
//     }

//     async fn recv_action(&mut self) -> Option<Action<u64>> {
//         self.action_rx.recv().await.map(|(action, ctx)| {
//             self.resp = Some(ctx);
//             action
//         })
//     }

//     async fn resp_action(&mut self, result: Result<(), Error>) {
//         if let Some(ctx) = self.resp.take() {
//             let _ = ctx.send(result);
//         }
//     }

//     async fn update_status(&mut self, state: &State) {
//         let names = self
//             .app_comms
//             .read()
//             .await
//             .groups
//             .get(&self.main_chat_id.unwrap())
//             .unwrap()
//             .names
//             .clone();
//         let status = state.status(names);
//     }
// }

// struct BasicEventHandler {
//     game_id: GameId,
//     app_comms: Arc<RwLock<AppComms>>,
//     event_rx: mpsc::UnboundedSender<Event>,
// }

// #[async_trait]
// impl EventHandler for BasicEventHandler {
//     fn init(&mut self, game: &Game) {
//         // let (event_tx, event_rx) = mpsc::unbounded_channel();
//         // game.event_tx = Some(event_tx);
//         // self.event_rx = event_rx;
//     }

//     async fn handle_event(&mut self, event: Event) {
//         let _ = self.event_rx.send(event);
//     }
// }

// struct AppComms {
//     groups: HashMap<u64, GroupMeGroup>, // GroupId -> GroupMeGroup
//     users: HashMap<u64, u64>,           // Pid -> UserId
// }
