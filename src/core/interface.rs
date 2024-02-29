use crate::base::{Choice, ID};
use crate::core::{PhaseKind, State};
use crate::roles::{Role, RoleKind, Team};

use serde::Deserialize;
use std::collections::HashMap;
// use std::sync::mpsc::{SendError, Sender};
use serde_json;
use tokio::sync::{mpsc, oneshot};

pub type CommandTx<PID> = mpsc::Sender<Command<PID>>;
pub type CommandRx<PID> = mpsc::Receiver<Command<PID>>;
pub type EventTx<PID> = mpsc::Sender<Event<PID>>;
pub type EventRx<PID> = mpsc::Receiver<Event<PID>>;

pub fn command_channel<PID: ID>() -> (mpsc::Sender<Command<PID>>, mpsc::Receiver<Command<PID>>) {
    mpsc::channel(100)
}

pub fn event_channel<PID: ID>() -> (mpsc::Sender<Event<PID>>, mpsc::Receiver<Event<PID>>) {
    mpsc::channel(100)
}

#[derive(Debug)]
pub struct Interface<PID: ID> {
    pub event_tx: EventTx<PID>,
    pub event_rx: Option<EventRx<PID>>,
    pub cmd_tx: CommandTx<PID>,
    pub cmd_rx: CommandRx<PID>,
}

impl<PID: ID> Interface<PID> {
    pub async fn new() -> Self {
        let (event_tx, event_rx) = event_channel();
        let (command_tx, command_rx) = command_channel();
        Self {
            event_tx,
            event_rx: Some(event_rx),
            cmd_tx: command_tx,
            cmd_rx: command_rx,
        }
    }

    pub async fn take_channels(self) -> Option<(Self, EventRx<PID>, CommandTx<PID>)> {
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
}

impl<PID: ID> Default for Interface<PID> {
    fn default() -> Self {
        let (event_tx, event_rx) = event_channel();
        let (command_tx, command_rx) = command_channel();
        Self {
            event_tx,
            event_rx: Some(event_rx),
            cmd_tx: command_tx,
            cmd_rx: command_rx,
        }
    }
}

pub type ActionResponder<PID> = oneshot::Sender<Result<(), CoreError<PID>>>;
pub type StatusResponder<PID> = oneshot::Sender<Result<State<PID>, CoreError<PID>>>;
pub type SerializeResponder = oneshot::Sender<Result<String, serde_json::Error>>;

// Responses are either () for Action or status for Status?
#[derive(Debug)]
pub enum Command<PID: ID> {
    Action(Action<PID>, ActionResponder<PID>),
    Status(StatusResponder<PID>),
    Serialize(SerializeResponder),
    Close,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Action<PID: ID> {
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
pub enum Event<PID: ID> {
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
pub enum CoreError<PID: ID> {
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
