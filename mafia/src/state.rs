pub mod action;
pub mod night_action;
pub mod phase;
pub mod players;
mod update;
mod util;

use std::path::Path;

use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWrite, AsyncWriteExt};
use tracing::event;

use crate::rules;
use crate::{prelude::*, rolegen::RoleGen};

pub type EventTx = mpsc::UnboundedSender<Event2>;

#[derive(Debug)]
pub struct StateProc {
    rules: Rules,
    event_tx: EventTx,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    day: u32,
    phase: Phase,
    players: Players,
    // #[serde(skip)]
    // action_rx: mpsc::Receiver<(Action, oneshot::Sender<Result<(), Error>>)>,
}

impl State {
    pub fn new(players: impl IntoIterator<Item = impl Into<Pid>>, rolegen: impl RoleGen) -> Self {
        let registry = rolegen.generate_roles(players);
        Self { day: 0, phase: Phase::Init, players: Players::from_registry(registry) }
    }
    pub fn is_started(&self) -> bool {
        !matches!(self.phase, Phase::Init)
    }
    pub fn is_ended(&self) -> bool {
        matches!(self.phase, Phase::End { .. })
    }

    pub fn players(&self) -> &Players {
        &self.players
    }
    pub fn start(&mut self, tx: &EventTx) {
        let _ = tx.send(Event2::Start { players: self.players.alive() });
        let n = self.players.n();
        if n % 2 == 1 {
            self.day(HashMap::new(), tx);
        } else {
            self.night(tx);
        }
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
