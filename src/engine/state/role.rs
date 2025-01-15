use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumKind, Serialize, Deserialize)]
#[enum_kind(RoleKind, derive(Hash, Serialize, Deserialize))]
pub enum Role {
    TOWN,
    COP,
    DOCTOR,
    CELEB,
    MAFIA,
}

impl Role {
    pub fn is_targeting(&self) -> bool {
        use Role::*;
        match self {
            COP | DOCTOR => true,
            TOWN | CELEB | MAFIA => false,
        }
    }
    pub fn is_scheming(&self) -> bool {
        use Role::*;
        match self {
            MAFIA => true,
            TOWN | COP | DOCTOR | CELEB => false,
        }
    }
    pub fn is_mafia(&self) -> bool {
        self.team() == Team::Mafia
    }
    pub fn team(&self) -> Team {
        Team::from(*self)
    }
    pub fn kind(&self) -> RoleKind {
        RoleKind::from(self)
    }
}

impl PartialEq<Role> for RoleKind {
    fn eq(&self, role: &Role) -> bool {
        let role_kind: RoleKind = role.into();
        self == &role_kind
    }
}

impl std::fmt::Display for RoleKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Team {
    Town,
    Mafia,
    Rogue,
}

impl From<Role> for Team {
    fn from(role: Role) -> Self {
        use Role::*;
        match role {
            TOWN | COP | DOCTOR | CELEB => Self::Town,
            MAFIA => Self::Mafia,
        }
    }
}

impl std::fmt::Display for Team {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
