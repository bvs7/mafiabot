use std::collections::HashMap;

use super::state::{
    phase::PhaseKind,
    players::Context,
    role::{RoleKind, Team},
    rules::Rules,
    Choice,
};
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast, mpsc, oneshot};

#[derive(Debug, Clone)]
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
pub struct Vote {
    pub voter: u64,
    pub ballot: Option<(Choice, usize)>,
    pub former: Option<(Choice, usize)>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Election {
    pub choice: Option<u64>,
    pub hammer: u64,
    pub voters: Vec<u64>,
}

#[derive(Debug, Clone, Hash, PartialEq, Eq, Serialize, Deserialize, EnumKind)]
#[enum_kind(CountKeyKind)]
pub enum CountKey {
    Role(RoleKind),
    Team(Team),
    IsMafia(bool),
    Players,
}

impl std::fmt::Display for CountKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            CountKey::Role(rk) => write!(f, "{}", rk),
            CountKey::Team(t) => write!(f, "{}", t),
            CountKey::IsMafia(m) => {
                if *m {
                    write!(f, "Mafia Aligned")
                } else {
                    write!(f, "Not Mafia Aligned")
                }
            }
            CountKey::Players => write!(f, "Players"),
        }
    }
}

impl From<RoleKind> for CountKey {
    fn from(kind: RoleKind) -> Self {
        CountKey::Role(kind)
    }
}
impl From<Team> for CountKey {
    fn from(team: Team) -> Self {
        CountKey::Team(team)
    }
}
impl From<bool> for CountKey {
    fn from(is_mafia: bool) -> Self {
        CountKey::IsMafia(is_mafia)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Event {
    Start {
        id: u64,
        players: Vec<u64>,
        rules: Rules,
        counts: HashMap<CountKey, usize>,
    },
    Day {
        day: u32,
        counts: HashMap<CountKey, usize>, // Use to get thresholds
    },
    Night {
        day: u32,
        counts: HashMap<CountKey, usize>,
    },
    Vvote(Vote),
    Vote {
        voter: u64,
        ballot: Option<(Option<u64>, usize)>,
        former: Option<(Option<u64>, usize)>,
    },
    Election(Election),
    Dawn, // Potentially note those who failed to do night actions
    Eliminate {
        player: u64,
        context: Context,
    },
    Target {
        actor: u64,
        choice: Choice,
    },
    Scheme {
        killer: u64,
        mark: Choice,
    },
    Debug,
}

pub type ActionMsg = (Action, oneshot::Sender<Result<(), Error>>);
pub type ActionRx = mpsc::Receiver<ActionMsg>;
pub type ActionTx = mpsc::Sender<ActionMsg>;
pub type EventRx = broadcast::Receiver<Event>;
pub type EventTx = broadcast::Sender<Event>;
