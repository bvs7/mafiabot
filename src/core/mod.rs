use serde::Serialize;
use std::sync::mpsc::Sender;

pub mod interface;
mod phase;

use phase::PhaseKind;

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
}

pub type PID = u64;
pub type PIDs = Vec<PID>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Player {
    pub user_id: PID,
    pub role: Role,
}

pub type Players = Vec<Player>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Task {
    Protect(PID),
    Assassinate(PID),
    ElectSelf(bool),
}

type Contract = (PID, Task);
type Contracts = Vec<Contract>;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GenRole {
    Role(Role),
    ContractRole(Role, usize),
}

pub type RoleGen = Vec<GenRole>;
pub struct GameRules {}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Choice {
    Player(PID),
    Abstain,
    None,
}

pub enum Action {
    Vote { voter: PID, ballot: Choice },
    Reveal { celeb: PID },
    Target { actor: PID, target: Choice },
    Mark { killer: PID, mark: Choice },
}

#[derive(Debug, Clone, PartialEq, Eq)]
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
        ballot: Choice,
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
        player: PID,
    },
    Refocus {
        contract: Contract,
    },
    End {
        winner: Option<Team>,
        contracts: Contracts,
    },
}

pub type EventOutput = Sender<Event>;
