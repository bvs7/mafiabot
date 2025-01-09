use std::collections::HashMap;

use super::{
    role::Role,
    state::{CountKey, PhaseKind, Rules},
    RoleKind,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, oneshot};

#[derive(Debug)]
pub enum Error {
    InvalidPhase {
        expected: PhaseKind,
        actual: PhaseKind,
    },
    InvalidActor {
        pid: u64,
    },
    InvalidOther {
        pid: u64,
    },
    ExpectedTargetingRole {
        actual: RoleKind,
    },
    ExpectedSchemingRole {
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
            Error::InvalidActor { pid } => write!(f, "Invalid actor player id: {pid}"),
            Error::InvalidOther { pid } => write!(f, "Invalid other player id: {pid}"),
            Error::ExpectedTargetingRole { actual } => {
                write!(f, "Expected targing role, got {actual}")
            }
            Error::ExpectedSchemingRole { actual } => {
                write!(f, "Expected scheming role, got {actual}")
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    Start {
        id: u64,
        players: HashMap<u64, Role>,
        rules: Rules,
        counts: HashMap<CountKey, u32>,
    },
    Day {
        day: u32,
        counts: HashMap<CountKey, u32>, // Use to get thresholds
    },
    Night {
        day: u32,
        counts: HashMap<CountKey, u32>,
    },
    Vote {
        voter: u64,
        ballot: Option<Option<u64>>,
        former: Option<Option<u64>>,
    },
    CheckElection {
        choice: Option<u64>,
        choice_count: usize,
    },
    Election {
        candidate: Option<u64>, // Choice
        hammer: u64,
        voters: Vec<u64>,
    },
    Dawn, // Potentially note those who failed to do night actions
    Debug,
}

pub type ActionMsg = (Action, oneshot::Sender<Result<(), Error>>);
pub type ActionRx = mpsc::Receiver<ActionMsg>;
pub type ActionTx = mpsc::Sender<ActionMsg>;
pub type EventRx = broadcast::Receiver<Event>;
pub type EventTx = broadcast::Sender<Event>;
