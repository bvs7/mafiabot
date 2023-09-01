use std::{collections::HashSet, ops::RangeBounds};

use kinded::Kinded;
use serde::Serialize;

use super::CoreError;
use CoreError::PlayerNotFound;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub struct PID(u64);

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct PIDs(HashSet<PID>);

impl PIDs {
    pub fn check(&self, pid: &PID) -> Result<(), CoreError> {
        self.0
            .contains(pid)
            .then(|| ())
            .ok_or(PlayerNotFound(pid.clone()))
    }
}

/// Generic Role Implementation
#[derive(Debug, Clone, Copy, Eq, Serialize, Kinded)]
#[kinded(derive(Serialize), kind = RoleKind)]
pub enum Role_<T> {
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
    GUARD(T),
    AGENT(T),
}

impl RoleKind {
    pub fn team(&self) -> Team {
        match self {
            RoleKind::TOWN | RoleKind::COP | RoleKind::DOCTOR | RoleKind::CELEB => Team::Town,
            RoleKind::MILLER | RoleKind::MASON => Team::Town,
            RoleKind::MAFIA | RoleKind::GODFATHER | RoleKind::GOON | RoleKind::STRIPPER => {
                Team::Mafia
            }
            RoleKind::IDIOT | RoleKind::SURVIVOR | RoleKind::GUARD | RoleKind::AGENT => Team::Rogue,
        }
    }

    pub fn investigate(&self) -> Team {
        match self {
            RoleKind::GODFATHER => Team::Town,
            RoleKind::MILLER => Team::Mafia,
            _ => self.team(),
        }
    }

    pub fn investigate_mafia(&self) -> bool {
        match self {
            RoleKind::GODFATHER => false,
            RoleKind::MILLER => true,
            _ => self.team() == Team::Mafia,
        }
    }

    pub fn targeting(&self) -> bool {
        matches!(
            self,
            RoleKind::COP | RoleKind::DOCTOR | RoleKind::STRIPPER | RoleKind::GUARD
        )
    }

    pub fn marking(&self) -> bool {
        self.team() == Team::Mafia && self != &RoleKind::GOON
    }
}

impl<T> Role_<T> {
    pub fn team(&self) -> Team {
        self.kind().team()
    }
    pub fn investigate(&self) -> Team {
        self.kind().investigate()
    }
    pub fn investigate_mafia(&self) -> bool {
        self.kind().investigate_mafia()
    }
    pub fn targeting(&self) -> bool {
        self.kind().targeting()
    }
    pub fn marking(&self) -> bool {
        self.kind().marking()
    }
}

impl<T> PartialEq for Role_<T>
where
    T: PartialEq,
{
    fn eq(&self, other: &Self) -> bool {
        self.kind() == other.kind()
    }
}

/// Role used by State, includes contract assignments
pub type Role = Role_<PID>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize)]
pub enum Team {
    Town,
    Mafia,
    Rogue,
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize)]
pub enum Task {
    Protect(PID),
    Assassinate(PID),
    Elect(PID), // bool denotes success?
}

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq, Serialize)]
pub struct Contract {
    holder: PID,
    task: Task,
    success: bool,
}
