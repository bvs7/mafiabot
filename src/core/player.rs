use serde::Serialize;
use std::fmt::{Debug, Display};

use super::roles::{Role, Team};

pub trait RawPID: Debug + Display + Clone + Copy + PartialEq + Eq + Send + Serialize {}

pub type Pidx = usize;
impl RawPID for Pidx {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize /*Deserialize*/)]
pub struct Player<U: RawPID> {
    pub raw_pid: U,
    pub role: Role,
}

impl<U: RawPID> Player<U> {
    pub fn new(raw_pid: U, role: Role) -> Self {
        Self { raw_pid, role }
    }
}
impl<U: RawPID> Display for Player<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.raw_pid)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
pub enum Winner {
    Team(Team),
    Player(Pidx),
    None,
}
impl Display for Winner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
pub enum Choice<U: RawPID> {
    Player(U),
    Abstain,
}

impl<U: RawPID> Choice<U> {
    pub fn is_player(&self) -> Option<U> {
        if let Choice::Player(p) = self {
            Some(*p)
        } else {
            None
        }
    }
    pub fn as_opt(&self) -> Option<U> {
        if let Choice::Player(p) = self {
            Some(*p)
        } else {
            None
        }
    }
}
impl Into<Option<Pidx>> for Choice<Pidx> {
    fn into(self) -> Option<Pidx> {
        if let Choice::Player(p) = self {
            Some(p)
        } else {
            None
        }
    }
}
impl Choice<Pidx> {
    pub fn to_p<U: RawPID>(&self, players: &Vec<Player<U>>) -> Option<Player<U>> {
        match self {
            Choice::Player(p) => Some(players[*p].clone()),
            Choice::Abstain => None,
        }
    }
}

// pub type Action<U> = (U, Choice<U>);
// pub type Election = (Vec<Pidx>, Choice<Pidx>);
