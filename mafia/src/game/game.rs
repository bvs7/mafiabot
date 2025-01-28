use std::{env, future::Future, path::PathBuf, sync::Arc};

use async_trait::async_trait;
use tokio::{
    sync::{watch, RwLock},
    time::error::Elapsed,
};

use crate::{prelude::*, state};

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

// Ok, let's think. How is a game created?
// 1. We have a list of players and a set of rules.
// 2. Generate and assign roles...
// 3. Create game chats and add members...
// 4. Hook up event handler and action handler...
// 5. Start the game.

// For Action Handler and Event Handler...
// We want a universal state...
// That holds watch::Receiver<Status> for games... as well as Game chat ids?
// We should pass in a ref to the game when creating ActionHandler and EventHandler...
// So we need to be able to create a game, then pass to handlers, then start handlers

pub struct Game {
    id: GameId,
    state: State,
}

impl Game {
    // Do Rolegen before here.
    pub fn new(registry: impl IntoIterator<Item = (impl Into<Pid>, Role)>, rules: Rules) -> Self {
        let state = State::new(registry, rules);
        Self { id: GameId::new().unwrap_or_default(), state }
    }

    pub fn id(&self) -> GameId {
        self.id
    }

    pub async fn run<P: Into<Pid> + Copy + 'static, E, A>(
        mut self,
        action_handler: A,
        event_handler: E,
    ) where
        A: ActionHandler<PID = P> + Send + 'static,
        E: EventHandler + Send + 'static,
    {
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        self.state.event_tx = Some(event_tx);

        tokio::spawn(Game::event_handler(event_rx, event_handler));
        self.action_handler(action_handler).await;
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
    pub async fn action_handler<P: Into<Pid> + Copy>(
        mut self,
        mut handler: impl ActionHandler<PID = P>,
    ) {
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
