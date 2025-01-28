mod state_impl;

pub mod phase;
pub mod players;
pub mod status;
mod util;

use crate::prelude::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    day: u32,
    phase: Phase,
    pub players: Players,
    rules: Rules,
    #[serde(skip)]
    pub event_tx: Option<mpsc::UnboundedSender<Event>>,
    // #[serde(skip)]
    // action_rx: mpsc::Receiver<(Action, oneshot::Sender<Result<(), Error>>)>,
}

impl State {
    pub fn new(registry: impl IntoIterator<Item = (impl Into<Pid>, Role)>, rules: Rules) -> Self {
        Self {
            day: 0,
            phase: Phase::Init,
            players: Players::from_registry(registry),
            rules,
            event_tx: None,
        }
    }

    pub fn tx(&mut self, event: Event) {
        if let Some(event_tx) = &self.event_tx {
            let _ = event_tx.send(event);
        }
    }
}
