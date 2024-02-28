use crate::base::{Choice, ID};
use crate::core::{PhaseKind, State};
use crate::roles::{Role, RoleKind, Team};

use std::collections::HashMap;
// use std::sync::mpsc::{SendError, Sender};
use tokio::sync::{mpsc, oneshot};

pub type ActionResponder<PID> = oneshot::Sender<Result<(), CoreError<PID>>>;
pub type StatusResponder<PID> = oneshot::Sender<Result<State<PID>, CoreError<PID>>>;

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

// Responses are wither () for Action or status for Status?
#[derive(Debug)]
pub enum Command<PID: ID> {
    Action(Action<PID>, ActionResponder<PID>),
    Status(StatusResponder<PID>),
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
    Start,
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
