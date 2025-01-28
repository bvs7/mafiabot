use crate::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rules {
    pub allowed_roles: HashSet<RoleKind>,
    pub guaranteed_roles: HashMap<RoleKind, usize>,
    pub mislead: u64, // 0 to 100
    pub kink: u64,    // 0 to 100
    pub rogue: u64,   // 0 to 100
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            allowed_roles: ALL_ROLES.iter().copied().collect(),
            guaranteed_roles: [(RoleKind::COP, 1), (RoleKind::DOCTOR, 1)].into_iter().collect(),
            mislead: 33,
            kink: 35,
            rogue: 8,
        }
    }
}
