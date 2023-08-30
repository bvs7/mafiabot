use serde::Serialize;
use std::fmt::{Debug, Display};
use std::sync::mpsc::Sender;

use kinded::Kinded;

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Kinded, Serialize)]
pub enum Action {
    Vote { voter: PID, ballot: Option<Choice> },
    Reveal { celeb: PID },
    Target { actor: PID, target: Option<Choice> },
    Mark { killer: PID, mark: Option<Choice> },
}

#[derive(Debug, Clone, PartialEq, Eq, Kinded, Serialize)]
pub enum Event {
    Init {
        game_id: usize,
    },
    Start {
        entrants: PIDs,
        rolegen: RoleGen,
        phase: PhaseKind,
    },
    Day {
        num: usize,
        players: PIDs,
    },
    Vote {
        voter: PID,
        choice: Choice,
        former: Choice,
        threshold: usize,
        count: usize,
    },
    Retract {
        voter: PID,
        former: Choice,
    },
    Reveal {
        celeb: PID,
    },
    Election {
        electors: PIDs,
        elected: Option<PID>,
    },
    Night {
        num: usize,
        players: PIDs,
    },
    Target {
        actor: PID,
        target: Choice,
    },
    Mark {
        killer: PID,
        mark: Choice,
    },
    Dawn,
    Strip {
        stripper: PID,
        blocked: PID,
    },
    Block {
        blocked: PID,
    },
    Save {
        doctor: PID,
        saved: PID,
    },
    Investigate {
        cop: PID,
        suspect: PID,
        role: Role,
    },
    Kill {
        killer: PID,
        mark: PID,
    },
    NoKill,
    Eliminate {
        eliminated: PID,
        role: Role,
    },
    ContractUpdate {
        contract: Contract,
    },
    Refocus {
        new_contract: Contract,
    },
    End {
        winner: Option<Team>,
        contracts: Contracts,
    },
}

pub type EventOutput = Sender<Event>;

#[derive(Debug)]
pub enum InvalidActionError {
    InvalidPhase {
        expected: PhaseKind,
        found: Phase,
    },
    InvalidAction {
        action: ActionKind,
        phase: PhaseKind,
    },
    PlayerNotFound {
        pid: PID,
    },
    InvalidRole {
        role: Role,
        action: ActionKind,
    },
    NoGame,
    InvalidTargetText {
        text: String,
    },
    InvalidTarget {
        target: PID,
    },
}
impl Display for InvalidActionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error with command: ")?;
        match self {
            Self::InvalidPhase { expected, found } => {
                write!(
                    f,
                    "Invalid Phase (expected {:?}, found {:?}",
                    expected, found
                )
            }
            Self::InvalidAction { action, phase } => {
                write!(f, "Invalid Action ({:?}) for Phase ({:?})", action, phase)
            }
            Self::PlayerNotFound { pid } => {
                write!(f, "Player with UserID {:?} not found", pid)
            }
            Self::InvalidRole { role, action } => {
                write!(f, "Invalid Role ({:?}) for Action ({:?})", role, action)
            }
            Self::NoGame => {
                write!(f, "No Game")
            }
            Self::InvalidTargetText { text } => {
                write!(f, "Invalid Target: {}", text)
            }
            Self::InvalidTarget { target } => {
                write!(f, "Invalid Target: {}", target)
            }
        }
    }
}

impl std::error::Error for InvalidActionError {}
