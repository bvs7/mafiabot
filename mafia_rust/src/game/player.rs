use serde::{Deserialize, Serialize};
use std::fmt::{Debug, Display};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
pub enum Role<U: RawPID> {
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
    GUARD(U),
    AGENT(U),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
pub enum Team {
    Town,
    Mafia,
    Rogue,
}
impl<U: RawPID> Role<U> {
    pub fn team(&self) -> Team {
        match self {
            Role::TOWN | Role::COP | Role::DOCTOR | Role::CELEB => Team::Town,
            Role::MILLER | Role::MASON => Team::Town,
            Role::MAFIA | Role::GODFATHER | Role::GOON | Role::STRIPPER => Team::Mafia,
            Role::IDIOT | Role::SURVIVOR | Role::GUARD(_) | Role::AGENT(_) => Team::Rogue,
        }
    }
    pub fn investigate_mafia(&self) -> bool {
        match self {
            Role::GODFATHER => false,
            Role::MILLER => true,
            _ => self.team() == Team::Mafia,
        }
    }

    pub fn has_night_action(&self) -> bool {
        match self {
            Role::COP | Role::DOCTOR | Role::STRIPPER => true,
            _ => false,
        }
    }
}

pub trait RawPID: Debug + Display + Clone + Copy + PartialEq + Eq + Send + Serialize {}

pub type Pidx = usize;
impl RawPID for Pidx {}

#[derive(Debug, Clone, PartialEq, Eq, Serialize /*Deserialize*/)]
pub struct Player<U: RawPID> {
    pub raw_pid: U,
    pub name: String,
    pub role: Role<U>,
}

impl<U: RawPID> Player<U> {
    pub fn new(raw_pid: U, name: &str, role: Role<U>) -> Self {
        Self {
            raw_pid,
            name: name.to_string(),
            role,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
pub enum Winner {
    Team(Team),
    Player(Pidx),
    None,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
pub enum Ballot<U: RawPID> {
    Player(U),
    Abstain,
    Retract,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
pub struct Election {
    pub electors: Vec<Pidx>,
    pub ballot: Ballot<Pidx>,
}

impl<U: RawPID> Display for Ballot<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Ballot::Player(p) => write!(f, "Player({})", p),
            Ballot::Abstain => write!(f, "Abstain"),
            Ballot::Retract => write!(f, "Retract"),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
pub enum Actor<U: RawPID> {
    Player(U),
    Mafia(U),
}
impl<U: RawPID> Actor<U> {
    pub fn overlaps(&self, other: &Self) -> bool {
        match (self, other) {
            (Actor::Player(p1), Actor::Player(p2)) => p1 == p2,
            (Actor::Mafia(_), Actor::Mafia(_)) => true,
            _ => false,
        }
    }
    pub fn is_player(&self, p: U) -> bool {
        match self {
            Actor::Player(p2) => p == *p2,
            _ => false,
        }
    }
    pub fn is_mafia(&self) -> bool {
        match self {
            Actor::Mafia(_) => true,
            _ => false,
        }
    }
}
impl<U: RawPID> Display for Actor<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Actor::Player(p) => write!(f, "Player({})", p),
            Actor::Mafia(p) => write!(f, "Mafia({})", p),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
pub enum Target<U: RawPID> {
    Player(U),
    NoTarget,
    Blocked,
}

impl<U: RawPID> Target<U> {
    pub fn is_player(&self, p: U) -> bool {
        match self {
            Target::Player(p2) => p == *p2,
            _ => false,
        }
    }
}

pub type Votes = Vec<(Pidx, Ballot<Pidx>)>;
pub type Actions = Vec<(Actor<Pidx>, Target<Pidx>)>;
