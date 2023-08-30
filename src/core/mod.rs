use serde::Serialize;
use std::sync::mpsc::Sender;

pub mod interface;
mod phase;

pub use phase::{Phase, PhaseKind};

use self::interface::{Event, EventOutput};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum Role {
    TOWN,
    COP,
    DOCTOR,
    CELEB,
    MILLER,
    MASON,
    MAFIA,
    GODFATHER,
    STRIPPER,
    GOON,
    IDIOT,
    SURVIVOR,
    GUARD,
    AGENT,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum Team {
    Town,
    Mafia,
    Rogue,
}

impl Role {
    pub fn team(&self) -> Team {
        match self {
            Role::TOWN | Role::COP | Role::DOCTOR | Role::CELEB => Team::Town,
            Role::MILLER | Role::MASON => Team::Town,
            Role::MAFIA | Role::GODFATHER | Role::GOON | Role::STRIPPER => Team::Mafia,
            Role::IDIOT | Role::SURVIVOR | Role::GUARD | Role::AGENT => Team::Rogue,
        }
    }
    pub fn investigate(&self) -> Team {
        match self {
            Role::GODFATHER => Team::Town,
            Role::MILLER => Team::Mafia,
            _ => self.team(),
        }
    }

    pub fn investigate_mafia(&self) -> bool {
        match self {
            Role::GODFATHER => false,
            Role::MILLER => true,
            _ => self.team() == Team::Mafia,
        }
    }

    pub fn targeting(&self) -> bool {
        matches!(self, Role::COP | Role::DOCTOR | Role::STRIPPER)
    }

    pub fn marking(&self) -> bool {
        self.team() == Team::Mafia && self != &Role::GOON
    }
}

pub type PID = u64;
pub type PIDs = Vec<PID>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Player {
    pub user_id: PID,
    pub role: Role,
}

pub type Players = Vec<Player>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Task {
    Protect(PID),
    Assassinate(PID),
    Elect(PID), // bool denotes success?
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct Contract {
    holder: PID,
    task: Task,
    success: bool,
}

type Contracts = Vec<Contract>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum GenRole {
    Role(Role),
    ContractRole(Role, usize),
}

pub type RoleGen = Vec<GenRole>;

pub struct GameRules {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Choice {
    /// Possible selections for a vote, target, or mark, used for external interface
    Player(PID),
    Abstain,
    None,
}

impl From<Option<PID>> for Choice {
    fn from(pid: Option<PID>) -> Self {
        match pid {
            Some(pid) => Choice::Player(pid),
            None => Choice::Abstain,
        }
    }
}
