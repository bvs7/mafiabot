use serde::Serialize;
use std::fmt::{Debug, Display};

use super::Players;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
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

pub trait RawPID: Debug + Display + Clone + Copy + PartialEq + Eq + Send + Serialize {}

pub type Pidx = usize;
impl RawPID for Pidx {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize /*Deserialize*/)]
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
    pub fn to_p<U: RawPID>(&self, players: &Players<U>) -> Option<Player<U>> {
        match self {
            Choice::Player(p) => Some(players[*p].clone()),
            Choice::Abstain => None,
        }
    }
}

pub type Action<U> = (U, Choice<U>);
pub type Election = (Vec<Pidx>, Choice<Pidx>);

pub type Votes = Vec<Action<Pidx>>;
pub type Targets = Vec<(Pidx, Choice<Pidx>, Role)>;
pub type Scheme = Option<Action<Pidx>>;
