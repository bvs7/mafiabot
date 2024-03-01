use crate::base::{Choice, ID};
use crate::core::{Core, PhaseKind, State};
use crate::roles::{Role, RoleKind, Team};
use crate::rules::Rules;

use serde::{Deserialize, Serialize};
use serde_json;
use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::num::ParseIntError;
use std::str::FromStr;
use tokio::sync::{mpsc, oneshot};

pub type CommandTx<PID> = mpsc::Sender<Command<PID>>;
pub type CommandRx<PID> = mpsc::Receiver<Command<PID>>;
pub type EventTx<PID> = mpsc::Sender<Event<PID>>;
pub type EventRx<PID> = mpsc::Receiver<Event<PID>>;

#[derive(Debug)]
pub struct Interface<PID: Eq + Hash> {
    pub event_tx: EventTx<PID>,
    pub event_rx: Option<EventRx<PID>>,
    pub cmd_tx: CommandTx<PID>,
    pub cmd_rx: CommandRx<PID>,
}

impl<PID: Debug + Eq + Hash> Interface<PID> {
    pub fn new() -> Self {
        let (event_tx, event_rx) = mpsc::channel(100);
        let (command_tx, command_rx) = mpsc::channel(100);
        Self {
            event_tx,
            event_rx: Some(event_rx),
            cmd_tx: command_tx,
            cmd_rx: command_rx,
        }
    }

    pub fn new_with_channels() -> (Self, EventRx<PID>, CommandTx<PID>) {
        Self::new().take_channels().unwrap()
    }

    pub fn take_channels(self) -> Option<(Self, EventRx<PID>, CommandTx<PID>)> {
        let event_rx = self.event_rx?;
        let inter = Self {
            event_tx: self.event_tx,
            event_rx: None,
            cmd_tx: self.cmd_tx.clone(),
            cmd_rx: self.cmd_rx,
        };
        Some((inter, event_rx, self.cmd_tx))
    }

    pub async fn send(&self, event: Event<PID>) -> Result<(), mpsc::error::SendError<Event<PID>>> {
        self.event_tx.send(event).await
    }

    // TODO: handle comms errors?
    pub async fn send_action(
        cmd_tx: &CommandTx<PID>,
        action: Action<PID>,
    ) -> Result<(), CoreError<PID>> {
        let (tx, rx) = oneshot::channel();
        cmd_tx
            .send(Command::Action(action, tx))
            .await
            .expect("Failed to send action: ");
        let resp = rx.await.unwrap();
        resp
    }

    pub async fn send_status(cmd_tx: &CommandTx<PID>) -> Result<State<PID>, CoreError<PID>> {
        let (tx, rx) = oneshot::channel();
        cmd_tx.send(Command::State(tx)).await.unwrap();
        let resp = rx.await.unwrap();
        resp
    }

    pub async fn send_rules(cmd_tx: &CommandTx<PID>) -> Result<Rules, CoreError<PID>> {
        let (tx, rx) = oneshot::channel();
        cmd_tx.send(Command::Rules(tx)).await.unwrap();
        let resp = rx.await.unwrap();
        resp
    }

    pub async fn send_serialize(
        cmd_tx: &CommandTx<PID>,
    ) -> Result<SerializedGame, SerializeGameError> {
        let (tx, rx) = oneshot::channel();
        cmd_tx.send(Command::Serialize(tx)).await.unwrap();
        rx.await.expect("Response channel error: {:?}")
    }

    pub async fn send_close(cmd_tx: &CommandTx<PID>) {
        cmd_tx.send(Command::Close).await.unwrap();
    }
}

impl<PID: Debug + Eq + Hash> Default for Interface<PID> {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug)]
pub struct SerializedGame {
    pub game_id: String, // Stringified id?
    pub state: String,   // json string
    pub rules: String,   // toml string
}

impl<PID: ID, GID: Display> TryFrom<&Core<PID, GID>> for SerializedGame {
    type Error = SerializeGameError;

    fn try_from(core: &Core<PID, GID>) -> Result<Self, Self::Error> {
        let id = core.game_id.to_string();
        let state = serde_json::to_string(&core.state)?;
        let rules = toml::to_string(&core.rules)?;
        Ok(SerializedGame {
            game_id: id,
            state,
            rules,
        })
    }
}

impl<'a, PID: Debug + Eq + Hash + Deserialize<'a>, GID: FromStr<Err = ParseIntError>>
    TryInto<Core<PID, GID>> for &'a SerializedGame
{
    type Error = DeserializeGameError;

    fn try_into(self) -> Result<Core<PID, GID>, Self::Error> {
        let game_id: GID = GID::from_str(self.game_id.as_str())?;
        let state = serde_json::from_str(self.state.as_str())?;
        let rules = toml::from_str(&self.rules)?;
        Ok(Core {
            game_id,
            state,
            rules,
            inter: Interface::new(),
        })
    }
}

#[derive(Debug)]
pub enum SerializeGameError {
    JsonError(serde_json::Error),
    TomlError(toml::ser::Error),
}

impl From<serde_json::Error> for SerializeGameError {
    fn from(e: serde_json::Error) -> Self {
        SerializeGameError::JsonError(e)
    }
}

impl From<toml::ser::Error> for SerializeGameError {
    fn from(e: toml::ser::Error) -> Self {
        SerializeGameError::TomlError(e)
    }
}

pub enum DeserializeGameError {
    ParseIntError(std::num::ParseIntError),
    JsonError(serde_json::Error),
    TomlError(toml::de::Error),
}

impl From<std::num::ParseIntError> for DeserializeGameError {
    fn from(e: std::num::ParseIntError) -> Self {
        DeserializeGameError::ParseIntError(e)
    }
}

impl From<serde_json::Error> for DeserializeGameError {
    fn from(e: serde_json::Error) -> Self {
        DeserializeGameError::JsonError(e)
    }
}

impl From<toml::de::Error> for DeserializeGameError {
    fn from(e: toml::de::Error) -> Self {
        DeserializeGameError::TomlError(e)
    }
}

pub type ActionResponder<PID> = oneshot::Sender<Result<(), CoreError<PID>>>;
pub type StateResponder<PID> = oneshot::Sender<Result<State<PID>, CoreError<PID>>>;
pub type RulesResponser<PID> = oneshot::Sender<Result<Rules, CoreError<PID>>>;
pub type SerializeResponder = oneshot::Sender<Result<SerializedGame, SerializeGameError>>;

// Responses are either () for Action or status for Status?
#[derive(Debug)]
pub enum Command<PID: Eq + Hash> {
    Action(Action<PID>, ActionResponder<PID>),
    State(StateResponder<PID>),
    Rules(RulesResponser<PID>),
    Serialize(SerializeResponder),
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum Action<PID: Eq + Hash> {
    Start,
    Vote { voter: PID, choice: Choice<PID> },
    Unvote { voter: PID },
    Reveal { player: PID },
    Target { actor: PID, target: Choice<PID> },
    Scheme { actor: PID, mark: Choice<PID> },
    Avenge { avenger: PID, victim: Choice<PID> },
    Elect { candidate: Choice<PID>, hammer: PID },
    Dawn,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event<PID: Eq + Hash> {
    Start {
        players: HashMap<PID, Role<PID>>,
    },
    Vote {
        voter: PID,
        ballot: Option<Choice<PID>>,
        former_ballot: Option<Choice<PID>>,
    },
    Reveal {
        player: PID,
        role: Role<PID>,
    },
    Target {
        actor: PID,
        target: Choice<PID>,
    },
    Scheme {
        actor: PID,
        mark: Choice<PID>,
    },
    Avenge {
        avenger: PID,
        target: PID,
    },
    Eliminate {
        player: PID,
        role: Role<PID>,
    },
    Refocus {
        player: PID,
        role: Role<PID>,
        former_role: Role<PID>,
    },
    ElectionImminent {
        candidate: Choice<PID>,
        hammer: PID,
    },
    ElectionAverted,
    Election {
        candidate: Choice<PID>,
        hammer: PID,
        voters: Vec<PID>,
    },
    Block {
        actor: PID,
        target: PID,
    },
    EvidentBlock {
        blocked: PID,
        blockers: Vec<PID>,
    },
    Save {
        actor: PID,
        target: PID,
    },
    EvidentSave {
        savior: PID,
        mark: PID,
    },
    Investigate {
        actor: PID,
        target: PID,
    },
    Kill {
        killer: PID,
        mark: PID,
    },
    NoNightKill,
    Day {
        day_no: u32,
    },
    Night {
        day_no: u32,
    },
    Eclipse {
        avenger: PID,
        hammer: PID,
        options: Vec<PID>,
    },
    Dawn,
    End {
        winner: Team,
        alive: Vec<PID>,
        role_history: HashMap<PID, Vec<Role<PID>>>,
    },
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CoreError<PID: Eq + Hash> {
    InvalidPhase {
        actual: PhaseKind,
        expected: PhaseKind,
    },
    InvalidPlayer {
        player: PID,
    },
    ExpectedTargetingRole {
        role: RoleKind,
    },
    ExpectedSchemingRole {
        role: RoleKind,
    },
    ExpectedCeleb {
        actual: RoleKind,
    },
    ExpectedElection {
        candidate: Choice<PID>,
    },
    ExpectedPlayer {
        actual: PID,
        expected: PID,
    },
    StripperOverload {
        actor: PID,
    },
    InvalidOption {
        actual: PID,
        options: Vec<PID>,
    },
    EventSendError(mpsc::error::SendError<Event<PID>>),
    Close,
}

impl<PID: ID> From<mpsc::error::SendError<Event<PID>>> for CoreError<PID> {
    fn from(e: mpsc::error::SendError<Event<PID>>) -> Self {
        CoreError::EventSendError(e)
    }
}
