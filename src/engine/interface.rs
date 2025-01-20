use std::collections::HashMap;

use super::{
    state::{
        phase::PhaseKind,
        players::Context,
        role::{Role, RoleKind, Team},
        rules::Rules,
        Choice,
    },
    GameId, PlayerId,
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
#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    Start {
        id: GameId,
        players: Vec<(PlayerId, Role)>,
        rules: Rules,
        counts: HashMap<Team, usize>, // TODO: make Team generic?
    },
    Day {
        day: u32,
        counts: HashMap<Team, usize>, // TODO: make Team generic
    },
    Night {
        day: u32,
        counts: HashMap<Team, usize>, // TODO: make Team generic
    },
    Vote {
        voter: PlayerId,
        ballot: Option<(Choice, usize)>,
        former: Option<(Choice, usize)>,
    },
    Reveal {
        celeb: PlayerId,
    },
    Election {
        choice: Choice,
        hammer: PlayerId,
        voters: Vec<PlayerId>,
    },
    Dawn, // Potentially note those who failed to do night actions
    Eliminate {
        player: PlayerId,
        role: RoleKind,
        context: Context,
    },
    Target {
        actor: PlayerId,
        choice: Choice,
    },
    Scheme {
        killer: PlayerId,
        mark: Choice,
    },
    Block {
        blocked: PlayerId,
        blockers: Vec<PlayerId>,
    },
    Save {
        saved: PlayerId,
        saviors: Vec<PlayerId>,
    },
    Kill {
        killer: PlayerId,
        mark: PlayerId,
    },
    Investigate {
        cop: PlayerId,
        target: PlayerId,
        appears_mafia: bool,
    },
    End {
        winner: Team,
    },
    Debug,
}

pub type ActionMsg = (Action, oneshot::Sender<Result<(), Error>>);
pub type ActionRx = mpsc::Receiver<ActionMsg>;
pub type ActionTx = mpsc::Sender<ActionMsg>;
pub type EventRx = broadcast::Receiver<Event>;
pub type EventTx = broadcast::Sender<Event>;
