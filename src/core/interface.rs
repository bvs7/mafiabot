use crate::core::base::{Choice, ID};
use crate::core::PhaseKind;
use crate::roles::{Role, RoleKind, Team};

use std::collections::HashMap;
// use std::sync::mpsc::{SendError, Sender};
use tokio::sync::{mpsc, oneshot};

pub type Resp<PID, T> = Result<T, CoreError<PID>>;
pub type RespInput<PID, T> = oneshot::Sender<Resp<PID, T>>;
pub type RespOutput<PID, T> = oneshot::Receiver<Resp<PID, T>>;

pub type ActionInput<PID, T> = mpsc::Sender<(Action<PID>, RespInput<PID, T>)>;
pub type ActionOutput<PID, T> = mpsc::Receiver<(Action<PID>, RespInput<PID, T>)>;

pub type EventOutput<PID> = mpsc::Sender<Event<PID>>;

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
    Close, // TODO: split this into a separate command. Have ActionCommand(Action) and Close as separate commands.
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
    Election {
        choice: Choice<PID>,
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
