mod action;
mod night_action;
pub mod phase;
pub mod players;
pub mod status;
mod update;
mod util;

use crate::{prelude::*, rolegen::RoleGen};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State<E> {
    day: u32,
    phase: Phase,
    pub players: Players,
    rules: Rules,
    #[serde(skip)]
    pub event_tx: Option<mpsc::UnboundedSender<Event>>,
    #[serde(skip)]
    event_handler: E,
    // #[serde(skip)]
    // action_rx: mpsc::Receiver<(Action, oneshot::Sender<Result<(), Error>>)>,
}

impl<E> State<E> {
    pub fn new(
        players: impl IntoIterator<Item = impl Into<Pid>>,
        rules: Rules,
        event_handler: E,
    ) -> Self {
        let registry = rules.rolegen_config.generate_roles(players);
        Self {
            day: 0,
            phase: Phase::Init,
            players: Players::from_registry(registry),
            rules,
            event_tx: None,
            event_handler,
        }
    }
    pub fn tx(&mut self, event: Event)
    where
        E: EventHandler,
    {
        self.event_handler.handle(event);
    }
}

pub trait EventHandler: std::fmt::Debug {
    fn handle(&mut self, event: Event);
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct EventLogger {
    events: Vec<Event>,
}

impl EventHandler for EventLogger {
    fn handle(&mut self, event: Event) {
        self.events.push(event);
    }
}
