use crate::prelude::*;

#[derive(
    Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, EnumKind, Serialize, Deserialize,
)]
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

impl From<RoleKind> for Role {
    fn from(kind: RoleKind) -> Self {
        use Role::*;
        match kind {
            RoleKind::TOWN => TOWN,
            RoleKind::COP => COP,
            RoleKind::DOCTOR => DOCTOR,
            RoleKind::CELEB => CELEB,
            RoleKind::MILKY => MILKY,
            RoleKind::MILLER => MILLER,
            RoleKind::MAFIA => MAFIA,
            RoleKind::STRIPPER => STRIPPER,
            RoleKind::GODFATHER => GODFATHER,
            RoleKind::GOON => GOON,
            RoleKind::IDIOT => IDIOT,
            RoleKind::SURVIVOR => SURVIVOR,
            RoleKind::GUARD => GUARD(Pid::new()),
            RoleKind::AGENT => AGENT(Pid::new()),
        }
    }
}

pub const ALL_ROLES: [RoleKind; 14] = [
    RoleKind::TOWN,
    RoleKind::COP,
    RoleKind::DOCTOR,
    RoleKind::CELEB,
    RoleKind::MILKY,
    RoleKind::MILLER,
    RoleKind::MAFIA,
    RoleKind::STRIPPER,
    RoleKind::GODFATHER,
    RoleKind::GOON,
    RoleKind::IDIOT,
    RoleKind::SURVIVOR,
    RoleKind::GUARD,
    RoleKind::AGENT,
];

impl std::fmt::Display for Role {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Role::TOWN => write!(f, "TOWN"),
            Role::COP => write!(f, "COP"),
            Role::DOCTOR => write!(f, "DOCTOR"),
            Role::CELEB => write!(f, "CELEB"),
            Role::MILKY => write!(f, "MILKY"),
            Role::MILLER => write!(f, "MILLER"),
            Role::MAFIA => write!(f, "MAFIA"),
            Role::STRIPPER => write!(f, "STRIPPER"),
            Role::GODFATHER => write!(f, "GODFATHER"),
            Role::GOON => write!(f, "GOON"),
            Role::IDIOT => write!(f, "IDIOT"),
            Role::SURVIVOR => write!(f, "SURVIVOR"),
            Role::GUARD(_) => write!(f, "GUARD"),
            Role::AGENT(_) => write!(f, "AGENT"),
        }
    }
}

impl RoleKind {
    pub fn team(&self) -> Team {
        Team::from(*self)
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
