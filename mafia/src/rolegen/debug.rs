use crate::prelude::*;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct DebugRoleGenConfig {
    roles: Vec<Role>,
}

impl DebugRoleGenConfig {
    pub fn new(roles: Vec<Role>) -> Self {
        Self { roles }
    }

    pub fn generate_roles(&self, pids: Vec<Pid>) -> Vec<(Pid, Role)> {
        let roles = self.roles.iter().cloned();
        pids.into_iter().zip(roles).collect()
    }
}
