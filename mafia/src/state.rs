mod action;
mod night_action;
pub mod phase;
pub mod players;
mod update;
mod util;

use crate::{prelude::*, rolegen::RoleGen};

pub type EventTx = broadcast::Sender<Event>;
pub type StatusTx = watch::Sender<State>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    day: u32,
    phase: Phase,
    players: Players,
    rules: Rules,
    // #[serde(skip)]
    // action_rx: mpsc::Receiver<(Action, oneshot::Sender<Result<(), Error>>)>,
}

impl State {
    pub fn new(players: impl IntoIterator<Item = impl Into<Pid>>, rules: Rules) -> Self {
        let registry = rules.rolegen_config.generate_roles(players);
        Self { day: 0, phase: Phase::Init, players: Players::from_registry(registry), rules }
    }

    pub fn is_started(&self) -> bool {
        !matches!(self.phase, Phase::Init)
    }

    pub fn start(&mut self, tx: &EventTx) {
        tx.send(Event::Start { players: self.players.alive(), rules: self.rules.clone() });
        let n = self.players.n();
        if n % 2 == 1 {
            self.day(HashMap::new(), tx);
        } else {
            self.night(tx);
        }
    }
}
