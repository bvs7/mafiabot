use std::collections::HashMap;

use serde::Serialize;

use super::{Role, RoleKind, Role_, Team, PID};

pub type GenRole = Role_<RoleKind>;

/// A list of generated roles
pub type RoleGen = Vec<GenRole>;

/// Roles Assigned to Players
pub type RoleAssign = HashMap<PID, Role>;
pub type RoleHistory = HashMap<PID, Vec<Role>>;
