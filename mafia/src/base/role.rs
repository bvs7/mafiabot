use crate::prelude::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, EnumKind, Serialize, Deserialize)]
#[enum_kind(RoleKind, derive(Hash, Serialize, Deserialize))]
pub enum Role {
    TOWN,
    COP,
    DOCTOR,
    CELEB,
    MILKY,
    MILLER,
    MAFIA,
    STRIPPER,
    GODFATHER,
    GOON,
    IDIOT,
    SURVIVOR,
    GUARD(Pid),
    AGENT(Pid),
}

impl Role {
    pub fn is_targeting(&self) -> bool {
        use Role::*;
        match self {
            COP | DOCTOR | MILKY | STRIPPER => true,
            TOWN | CELEB | MILLER | MAFIA | GODFATHER | GOON | IDIOT | SURVIVOR | GUARD(_)
            | AGENT(_) => false,
        }
    }
    pub fn is_scheming(&self) -> bool {
        use Role::*;
        match self {
            MAFIA | STRIPPER | GODFATHER => true,
            TOWN | COP | DOCTOR | CELEB | MILLER | MILKY | GOON | IDIOT | SURVIVOR | GUARD(_)
            | AGENT(_) => false,
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
    pub fn appears_mafia(&self) -> bool {
        match self {
            Role::GODFATHER => false,
            Role::MILLER => true,
            _ => self.team() == Team::Mafia,
        }
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
