//! Crate core

use crate::prelude::*;

pub mod interface;

use interface::{Action, Choice, Event, EventOutput};

use std::{
    collections::{HashMap, HashSet},
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
};

/* #region Role Types */

pub type PID = u64;

#[derive(Debug, Clone, Copy, Eq, Hash, Kinded, Serialize)]
#[kinded(kind=RoleKind)]
pub enum Role_<T> {
    TOWN,
    COP,
    DOCTOR,
    CELEB,
    MILLER,
    MAFIA,
    GODFATHER,
    STRIPPER,
    GOON,
    IDIOT(bool),
    SURVIVOR,
    GUARD(T),
    AGENT(T),
}

pub type Role = Role_<PID>;

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Serialize)]
pub enum Team {
    Town,
    Mafia,
    Rogue,
}

/* #endregion Types */

/* #region  Role Impl */
impl<T> PartialEq for Role_<T> {
    fn eq(&self, other: &Self) -> bool {
        self.kind() == other.kind()
    }
}

impl RoleKind {
    pub fn team(&self) -> Team {
        use RoleKind as RK;
        match self {
            RK::TOWN | RK::COP | RK::DOCTOR | RK::CELEB | RK::MILLER => Team::Town,
            RK::MAFIA | RK::GODFATHER | RK::STRIPPER | RK::GOON => Team::Mafia,
            RK::IDIOT | RK::SURVIVOR | RK::GUARD | RK::AGENT => Team::Rogue,
        }
    }
    pub fn investigate(&self) -> RoleKind {
        match self {
            RoleKind::GODFATHER => RoleKind::TOWN,
            RoleKind::MILLER => RoleKind::MAFIA,
            _ => *self,
        }
    }
    pub fn targeting(&self) -> bool {
        matches!(self, RoleKind::COP | RoleKind::DOCTOR | RoleKind::STRIPPER)
    }
}

impl<T> Role_<T> {
    pub fn team(&self) -> Team {
        self.kind().team()
    }
    pub fn investigate(&self) -> RoleKind {
        self.kind().investigate()
    }
    pub fn targeting(&self) -> bool {
        self.kind().targeting()
    }
}

/* #endregion Role Impl */

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub enum Phase {
    Day {
        votes: HashMap<PID, Choice>,
        blocks: HashSet<PID>,
    },
    Night {
        targets: HashMap<PID, Choice>,
        mark: Option<(PID, Choice)>,
    },
    Dusk {
        avenger: PID,
        voters: HashSet<PID>,
    },
}

pub struct Rules {}

pub struct Game {
    id: u64,
    players: HashMap<PID, Role>,
    phase: Phase,
    role_history: HashMap<PID, Vec<Role>>,
    rules: Rules,
    event_output: EventOutput,
}

// TODO: figure out how responses will work, etc...

pub struct GameContext {
    game: Arc<Mutex<Game>>,
    actions: Receiver<Action>,
}
