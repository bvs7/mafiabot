use crate::prelude::*;

use std::{env, path::PathBuf};
use tokio::task::JoinHandle;

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

// type ActionMsg<P> = (Action<P>, oneshot::Sender<Result<(), Error>>);
// pub type ActionTx<P> = mpsc::Sender<ActionMsg<P>>;
// type ActionRx<P> = mpsc::Receiver<ActionMsg<P>>;
pub type EventRx = mpsc::UnboundedReceiver<Event2>;
type StatusTx = watch::Sender<State>;
pub type StatusRx = watch::Receiver<State>;

// #[derive(Debug, Clone)]
// pub struct Game {
//     id: GameId,
//     state: State,
// }

// impl Game {
//     pub fn new(players: impl IntoIterator<Item = impl Into<Pid>>, rules: Rules) -> Self {
//         let id = GameId::new().unwrap_or_default();
//         let state = State::new(players, rules);
//         Self { id, state }
//     }

//     pub fn start<P: Into<Pid> + Copy + Send + 'static>(
//         self,
//     ) -> (JoinHandle<Game>, ActionTx<P>, StatusRx, EventRx) {
//         let (action_tx, action_rx) = mpsc::channel(16);
//         let (state_tx, state_rx) = watch::channel(self.state.clone());
//         let (event_tx, event_rx) = mpsc::unbounded_channel();
//         let h = tokio::spawn(self.run(action_rx, state_tx, event_tx));
//         (h, action_tx, state_rx, event_rx)
//     }

//     pub async fn run<P: Into<Pid> + Copy>(
//         mut self,
//         mut action_rx: ActionRx<P>,
//         state_tx: StatusTx,
//         event_tx: EventTx,
//     ) -> Self {
//         self.state.event_tx = Some(event_tx);
//         if !self.state.is_started() {
//             self.state.start();
//         }
//         let mut time = self.state.update();
//         let _ = state_tx.send(self.state.clone());
//         loop {
//             let dur = Self::dur_until(time);
//             match tokio::time::timeout(dur, action_rx.recv()).await {
//                 Ok(Some((action, resp))) => {
//                     let result =
//                         self.state.validate_action(action).map(|va| self.state.perform_action(va));

//                     // resp.send(result).unwrap();
//                 }
//                 Ok(None) => {
//                     break;
//                 }
//                 Err(_) => {
//                     //timeout
//                 }
//             }
//             time = self.state.update();
//             let _ = state_tx.send(self.state.clone());
//         }
//         self
//     }

//     pub fn id(&self) -> GameId {
//         self.id
//     }

//     pub fn players(&self) -> HashMap<Pid, Role> {
//         self.state.players().alive().into_iter().collect()
//     }

//     fn dur_until(time: Option<DateTime<Local>>) -> Duration {
//         let time = time.map(|t| (t - Local::now()).to_std());
//         match time {
//             Some(Ok(dur)) => dur,
//             Some(Err(_)) => Duration::ZERO,
//             None => Duration::MAX,
//         }
//     }
// }
