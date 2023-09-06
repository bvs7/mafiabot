//! Crate core

use crate::prelude::*;

pub mod game;
pub mod interface;

use game::{ActionResult, Game, Phase, PhaseKind};
use interface::{Action, Choice, Election, Event, EventOutput};

use std::{
    collections::{HashMap, HashSet},
    ops::Deref,
    sync::{
        mpsc::{Receiver, Sender},
        Arc, Mutex,
    },
};

/* #region Role Types */

pub type PID = u64;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Kinded, Serialize)]
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

impl<T> Role_<T> {
    fn to<U, F>(&self, f: F) -> Role_<U>
    where
        F: FnMut(T) -> U,
    {
        match self {
            Self::GUARD(charge) => Role_::GUARD(f(*charge)),
            Self::AGENT(charge) => Role_::AGENT(f(*charge)),
            Self::IDIOT(b) => Role_::IDIOT(*b),
            _ => todo!(),
        }
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

impl Role {
    pub fn refocus(&self, player: PID, proxy: PID) -> Role {
        match (self, player == proxy) {
            (Role::GUARD(_), false) => Role::AGENT(proxy),
            (Role::GUARD(_), true) => Role::IDIOT(false),
            (Role::AGENT(_), false) => Role::GUARD(proxy),
            (Role::AGENT(_), true) => Role::SURVIVOR,
            _ => unimplemented!("Cannot refocus {:?}", self),
        }
    }
}

/* #endregion Role Impl */

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]

pub struct Rules {}
