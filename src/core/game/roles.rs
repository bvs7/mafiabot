use std::fmt::Display;

use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize /*Deserialize*/)]
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

impl Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::TOWN => write!(f, "TOWN"),
            Role::COP => write!(f, "COP"),
            Role::DOCTOR => write!(f, "DOCTOR"),
            Role::CELEB => write!(f, "CELEB"),
            Role::MILLER => write!(f, "MILLER"),
            Role::MASON => write!(f, "MASON"),
            Role::MAFIA => write!(f, "MAFIA"),
            Role::GODFATHER => write!(f, "GODFATHER"),
            Role::STRIPPER => write!(f, "STRIPPER"),
            Role::GOON => write!(f, "GOON"),
            Role::IDIOT => write!(f, "IDIOT"),
            Role::SURVIVOR => write!(f, "SURVIVOR"),
            Role::GUARD => write!(f, "GUARD"),
            Role::AGENT => write!(f, "AGENT"),
        }
    }
}

impl Display for Team {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Team::Town => write!(f, "Town Aligned"),
            Team::Mafia => write!(f, "Mafia Aligned"),
            Team::Rogue => write!(f, "Rogue (Unaligned)"),
        }
    }
}

impl Role {
    pub fn description(&self) -> &'static str {
        match self {
            Self::TOWN => "Figure out who the Mafia are and kill them!",
            Self::COP => "You can investigate a player each night to see if they are Mafia or not.",
            Self::DOCTOR => "You can save a player each night from being killed by the Mafia.",
            Self::CELEB => "During the Day, you can reveal yourself publicly as CELEB.",
            Self::MILLER => "But if a COP investigates you, they see you as Mafia Aligned!",
            Self::MASON => "You can talk to other Masons during the night.",
            Self::MAFIA => {
                "Conspire during the night with your fellow Mafia and mark a player to be killed!"
            }
            Self::GODFATHER => "But if a COP investigates you, they see you as Not Mafia Aligned!",
            Self::STRIPPER => "You can visit a player at night to block their action!",
            Self::GOON => "But you cannot mark a player to be killed during the Night!",
            Self::IDIOT | Self::SURVIVOR | Self::GUARD | Self::AGENT => {
                "You have been given a contract. Try to fulfill it!"
            }
        }
    }
}
