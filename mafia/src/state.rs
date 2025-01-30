mod action;
mod night_action;
pub mod phase;
pub mod players;
mod update;
mod util;

use crate::{prelude::*, rolegen::RoleGen};

pub type EventTx = mpsc::UnboundedSender<Event>;
pub type StatusTx = watch::Sender<State>;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    day: u32,
    phase: Phase,
    players: Players,
    rules: Rules,
    #[serde(skip)]
    tx: Option<EventTx>,
    // #[serde(skip)]
    // action_rx: mpsc::Receiver<(Action, oneshot::Sender<Result<(), Error>>)>,
}

impl State {
    pub fn new(players: impl IntoIterator<Item = impl Into<Pid>>, rules: Rules) -> Self {
        Self::with_tx(players, rules, None)
    }
    pub fn with_tx(
        players: impl IntoIterator<Item = impl Into<Pid>>,
        rules: Rules,
        tx: Option<EventTx>,
    ) -> Self {
        let registry = rules.rolegen_config.generate_roles(players);
        Self { day: 0, phase: Phase::Init, players: Players::from_registry(registry), rules, tx }
    }
    pub fn is_started(&self) -> bool {
        !matches!(self.phase, Phase::Init)
    }

    pub fn start(&mut self) {
        self.tx(Event::Start { players: self.players.alive(), rules: self.rules.clone() });
        let n = self.players.n();
        if n % 2 == 1 {
            self.day(HashMap::new());
        } else {
            self.night();
        }
    }

    fn tx(&self, event: Event) {
        if let Some(event_tx) = &self.tx {
            let _ = event_tx.send(event);
        }
    }

    pub fn players(&self) -> &Players {
        &self.players
    }
}
