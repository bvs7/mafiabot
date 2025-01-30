use std::{env, path::PathBuf};
use tokio::task::JoinHandle;
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
pub type EventRx = mpsc::UnboundedReceiver<Event>;
type StatusRx = watch::Receiver<State>;

#[derive(Debug)]
pub struct Game<P> {
    id: GameId,
    run_handle: JoinHandle<State>,
    state_rx: StatusRx,
    action_tx: ActionTx<P>,
}

// So the question is... can we have both shared refs to Game handler, and mutable for action recv?

impl<P> Game<P> {
    pub fn new(players: impl IntoIterator<Item = impl Into<Pid>>, rules: Rules) -> (Self, EventRx)
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
    ) -> (Self, EventRx)
    where
        P: Into<Pid> + Copy + Send + 'static,
    {
        let (action_tx, action_rx) = mpsc::channel(16);
        let (event_tx, event_rx) = mpsc::unbounded_channel();
        let state = State::with_tx(players, rules, Some(event_tx));
        let (state_tx, state_rx) = watch::channel(state.clone());
        let run_handle = tokio::spawn(Self::run(state, action_rx, state_tx));
        let game = Self { id, run_handle, state_rx, action_tx };
        (game, event_rx)
    }

    pub fn id(&self) -> GameId {
        self.id
    }

    fn dur_until(time: Option<DateTime<Local>>) -> Duration {
        let time = time.map(|t| (t - Local::now()).to_std());
        match time {
            Some(Ok(dur)) => dur,
            Some(Err(_)) => Duration::ZERO,
            None => Duration::MAX,
        }
    }

    pub async fn run(mut state: State, mut action_rx: ActionRx<P>, state_tx: StatusTx) -> State
    where
        P: Into<Pid> + Copy,
    {
        if !state.is_started() {
            state.start();
        }
        let mut alarm_time = state.update();
        state_tx.send(state.clone()).expect("Game should not drop state_rx");
        loop {
            match tokio::time::timeout(Self::dur_until(alarm_time), action_rx.recv()).await {
                Ok(Some((action, resp))) => {
                    // Handle action
                    let result = state.validate_action(action).map(|va| state.perform_action(va));
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
            alarm_time = state.update();
            state_tx.send(state.clone()).expect("Game should not drop state_rx");
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

    pub async fn stop(self) -> State {
        drop(self.action_tx);
        match self.run_handle.await {
            Ok(state) => state,
            Err(err) => {
                error!("Game run_handle couldn't join: {}", err);
                self.state_rx.borrow().clone()
            }
        }
    }
}
