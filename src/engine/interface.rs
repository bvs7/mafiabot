use std::collections::HashMap;

use super::state::{
    id::{Choice, Gid, Pid},
    phase::PhaseKind,
    players::Context,
    role::{Role, RoleKind, Team},
    rules::Rules,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, oneshot};

#[derive(Debug, Clone)]
pub enum Error {
    InvalidPhase {
        expected: PhaseKind,
        actual: PhaseKind,
    },
    InvalidPlayer {
        pid: u64,
    },
    DeadPlayer {
        pid: u64,
    },
    ExpectedTargetingRole {
        actual: RoleKind,
    },
    ExpectedSchemingRole {
        actual: RoleKind,
    },
    ExpectedCeleb {
        actual: RoleKind,
    },
    IneffectiveVote,
}

impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Error::InvalidPhase { expected, actual } => {
                write!(f, "Invalid Phase. Expected {expected} but got {actual}")
            }
            Error::InvalidPlayer { pid } => write!(f, "Invalid player id: {pid}"),
            Error::DeadPlayer { pid } => write!(f, "Player is dead: {pid}"),
            Error::ExpectedTargetingRole { actual } => {
                write!(f, "Expected targing role, got {actual}")
            }
            Error::ExpectedSchemingRole { actual } => {
                write!(f, "Expected scheming role, got {actual}")
            }
            Error::ExpectedCeleb { actual } => {
                write!(f, "Expected celeb, got {actual}")
            }
            Error::IneffectiveVote => {
                write!(f, "This vote would not have any effect")
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Deserialize)]
pub enum Action {
    Start,
    Vote {
        voter: u64,
        ballot: Option<Option<u64>>,
    },
    Target {
        actor: u64,
        choice: Option<u64>,
    },
    Scheme {
        killer: u64,
        mark: Option<u64>,
    },
    Reveal {
        actor: u64,
    },
}

impl Action {
    pub fn actor(&self) -> Option<u64> {
        use Action::*;
        match self {
            Vote { voter: actor, .. }
            | Target { actor, .. }
            | Scheme { killer: actor, .. }
            | Reveal { actor } => Some(*actor),
            _ => None,
        }
    }

    pub fn other(&self) -> Option<u64> {
        use Action::*;
        match self {
            Vote {
                ballot: Some(Some(other)),
                ..
            }
            | Target {
                choice: Some(other),
                ..
            }
            | Scheme {
                mark: Some(other), ..
            } => Some(*other),
            _ => None,
        }
    }
}

// We probably need two generics for start roles and known roles here?
// Maybe even a third for reveal on death...
// Alternatively, have one Rules trait of some sort with associated types!
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Event {
    Start {
        players: Vec<(Pid, Role)>,
        rules: Rules,
    },
    Day {
        day: u32,
        counts: HashMap<Team, usize>, // TODO: make Team generic
    },
    Night {
        day: u32,
        counts: HashMap<Team, usize>, // TODO: make Team generic
    },
    Eclipse {
        avenger: Pid,
        hammer: Pid,
        guilty: Vec<Pid>,
    },
    Vengeance {
        avenger: Pid,
        victim: Pid,
    },
    Vote {
        voter: Pid,
        ballot: Option<(Choice, usize)>,
        former: Option<(Choice, usize)>,
    },
    Reveal {
        celeb: Pid,
    },
    Election {
        choice: Choice,
        hammer: Pid,
        voters: Vec<Pid>,
    },
    Dawn, // Potentially note those who failed to do night actions
    Eliminate {
        player: Pid,
        role: RoleKind,
        context: Context,
    },
    Target {
        actor: Pid,
        choice: Choice,
    },
    Scheme {
        killer: Pid,
        mark: Choice,
    },
    Block {
        blocked: Pid,
        blockers: Vec<Pid>,
    },
    Save {
        saved: Pid,
        saviors: Vec<Pid>,
    },
    NoKill,
    Kill {
        killer: Pid,
        mark: Pid,
    },
    Investigate {
        cop: Pid,
        target: Pid,
        appears_mafia: bool,
    },
    Milk {
        milky: Pid,
        target: Pid,
    },
    End {
        winner: Team,
    },
    Debug,
}

pub type ActionResponder = oneshot::Sender<Result<(), Error>>;
pub type ActionMsg = (Action, ActionResponder);
pub type ActionRx = mpsc::Receiver<ActionMsg>;
pub type ActionTx = mpsc::Sender<ActionMsg>;
pub type EventRx = broadcast::Receiver<Event>;
pub type EventTx = broadcast::Sender<Event>;
