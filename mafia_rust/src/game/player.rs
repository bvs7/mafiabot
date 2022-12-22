use serde::Serialize;
use std::fmt::{Debug, Display};

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

#[derive(Debug, Clone, PartialEq, Eq, Serialize /*Deserialize*/)]
pub struct Player<U: RawPID> {
    pub raw_pid: U,
    pub name: String,
    pub role: Role,
}

impl<U: RawPID> Player<U> {
    pub fn new(raw_pid: U, name: &str, role: Role) -> Self {
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
impl Display for Winner {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
pub enum Target<U: RawPID> {
    Player(U),
    Abstain,
    Blocked,
}

impl<U: RawPID> Target<U> {
    pub fn is_player(&self) -> Option<U> {
        if let Target::Player(p) = self {
            Some(*p)
        } else {
            None
        }
    }
}

pub type Action<U> = (U, Target<U>);
pub type Election = (Vec<Pidx>, Target<Pidx>);

pub type Votes = Vec<Action<Pidx>>;
pub type Targets = Vec<Action<Pidx>>;
pub type Scheme = Option<Action<Pidx>>;

pub type Night<'a> = (usize, &'a mut Targets, &'a mut Scheme);
