use std::{env, path::PathBuf};
use tracing::event;

use crate::{prelude::*, state};

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

type ActionMsg<P> = (Action<P>, oneshot::Sender<Result<(), Error>>);
type ActionTx<P> = mpsc::Sender<ActionMsg<P>>;
type ActionRx<P> = mpsc::Receiver<ActionMsg<P>>;
type EventRx = broadcast::Receiver<Event>;
type StatusRx = watch::Receiver<State>;

pub struct Game<P> {
    id: GameId,
    state_rx: StatusRx,
    action_tx: ActionTx<P>,
    event_rx: EventRx,
}

// So the question is... can we have both shared refs to Game handler, and mutable for action recv?

impl<P> Game<P> {
    pub fn new(players: impl IntoIterator<Item = impl Into<Pid>>, rules: Rules) -> Arc<Self>
    where
        P: Into<Pid> + Copy + Send + 'static,
    {
        let id = GameId::new().unwrap_or_default();
        Self::with_id(id, players, rules)
    }

    pub fn with_id(
        id: GameId,
        players: impl IntoIterator<Item = impl Into<Pid>>,
        rules: Rules,
    ) -> Arc<Self>
    where
        P: Into<Pid> + Copy + Send + 'static,
    {
        let (action_tx, action_rx) = mpsc::channel(100);
        let (event_tx, event_rx) = broadcast::channel(100);
        let state = State::new(players, rules);
        let (state_tx, state_rx) = watch::channel(state.clone());
        let game = Self { id, state_rx, action_tx, event_rx };
        let game = Arc::new(game);
        let g = game.clone();
        tokio::spawn(async move { g.run(state, action_rx, event_tx, state_tx).await });
        game
    }

    fn dur_until(time: Option<DateTime<Local>>) -> Duration {
        let time = time.map(|t| (t - Local::now()).to_std());
        match time {
            Some(Ok(dur)) => dur,
            Some(Err(_)) => Duration::ZERO,
            None => Duration::MAX,
        }
    }

    pub async fn run(
        &self,
        mut state: State,
        mut action_rx: ActionRx<P>,
        event_tx: EventTx,
        state_tx: StatusTx,
    ) -> State
    where
        P: Into<Pid> + Copy,
    {
        if !state.is_started() {
            state.start(&event_tx);
        }
        state.update(&event_tx, &state_tx);
        let _ = state_tx.send(state.clone());
        let mut alarm_time = None;
        loop {
            match tokio::time::timeout(Self::dur_until(alarm_time), action_rx.recv()).await {
                Ok(Some((action, resp))) => {
                    // Handle action
                    let result =
                        state.validate_action(action).map(|va| state.perform_action(va, &event_tx));
                    let _ = resp.send(result);
                }
                Ok(None) => {
                    // Action queue closed
                    break;
                }
                Err(_) => {
                    // Timeout
                }
            }
            alarm_time = state.update(&event_tx, &state_tx);
        }
        state
    }

    pub async fn send_action(&self, action: Action<P>) -> Result<(), Error>
    where
        P: Into<Pid>,
    {
        let (tx, rx) = oneshot::channel();
        let _ = self.action_tx.send((action, tx)).await;
        rx.await.expect("Action handler should not drop tx")
    }

    pub fn get_state(&self) -> State {
        self.state_rx.borrow().clone()
    }
}
