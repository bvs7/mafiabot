use serde::Serialize;
use std::fmt::{Debug, Display};
use std::sync::mpsc::Sender;

use kinded::Kinded;

use super::game::{
    Contract, Contracts, PIDs, Phase, PhaseKind, Role, RoleGen, RoleGens, RoleKind, State,
    StateKind, Team, PID,
};

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
        votee: Option<Option<PID>>,
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
        mark: PID,
    },
    Saved {
        mark: PID,
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

#[derive(Debug)]
pub struct EventOutput {
    sender: Sender<Event>,
}

impl EventOutput {
    pub fn send(&self, event: Event) {
        self.sender
            .send(event)
            .expect("Failed to send event {event}");
    }
}

#[derive(Debug)]
pub enum InvalidActionError {
    InvalidPhase {
        expected: PhaseKind,
        found: PhaseKind,
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
    InvalidState {
        expected: StateKind,
        found: StateKind,
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
            _ => write!(f, "Unknown Error"),
        }
    }
}

impl std::error::Error for InvalidActionError {}
