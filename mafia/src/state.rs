mod action;
mod night_action;
pub mod phase;
pub mod players;
mod update;
mod util;

use std::path::Path;

use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};

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
    pub event_tx: Option<EventTx>,
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
        Self {
            day: 0,
            phase: Phase::Init,
            players: Players::from_registry(registry),
            rules,
            event_tx: tx,
        }
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
        if let Some(event_tx) = &self.event_tx {
            let _ = event_tx.send(event);
        }
    }

    pub fn players(&self) -> &Players {
        &self.players
    }
}

impl std::fmt::Display for State {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.phase.kind(), self.day)?;
        //TODO: Display players, phase, etc...
        Ok(())
    }
}

pub struct Brief {
    day: u32,
    phase: PhaseKind,
    counts: HashMap<Team, usize>,
}

impl From<State> for Brief {
    fn from(state: State) -> Self {
        let mut counts = HashMap::new();
        for (_, role) in state.players.alive() {
            *counts.entry(role.team()).or_default() += 1;
        }
        Self { day: state.day, phase: state.phase.kind(), counts }
    }
}

impl std::fmt::Display for Brief {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} {}", self.phase, self.day)?;
        if let Some(count) = self.counts.get(&Team::Town) {
            write!(f, " Town:{}", count)?;
        }
        if let Some(count) = self.counts.get(&Team::Mafia) {
            write!(f, " Mafia:{}", count)?;
        }
        if let Some(count) = self.counts.get(&Team::Rogue) {
            write!(f, " Rogue:{}", count)?;
        }
        Ok(())
    }
}

impl State {
    pub async fn save(&self, path: impl AsRef<Path>) -> std::io::Result<()> {
        let mut f = File::create(path.as_ref()).await?;
        let state_str = serde_json::to_vec(self)?;
        f.write_all(&state_str).await
    }

    pub async fn load(path: impl AsRef<Path>) -> std::io::Result<Self> {
        let mut f = File::open(path.as_ref()).await?;
        let mut buf = Vec::new();
        f.read_to_end(&mut buf).await?;
        let state: Self = serde_json::from_slice(&buf)?;
        Ok(state)
    }
}
