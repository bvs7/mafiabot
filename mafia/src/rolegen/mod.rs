use crate::prelude::*;

mod standard;
pub use standard::StandardRoleGen;

use rand::{seq::SliceRandom, Rng};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum GenRole {
    Role(RoleKind),
    GuardCharged(usize),
    AgentCharged(usize),
}

impl GenRole {
    pub fn role(self, players: &Vec<Pid>) -> Role {
        match self {
            GenRole::Role(role) => role.into(),
            GenRole::GuardCharged(idx) => Role::GUARD(players[idx]),
            GenRole::AgentCharged(idx) => Role::AGENT(players[idx]),
        }
    }
}

pub trait RoleGen {
    type RNG: Rng;
    fn generate_roles(n: usize, rules: &Rules, rng: &mut Self::RNG) -> Vec<GenRole>;
}

pub fn assign_roles(
    users: Vec<impl Into<Pid>>,
    roles: Vec<GenRole>,
    rng: &mut impl Rng,
) -> Vec<(Pid, Role)> {
    let mut users = users.into_iter().map(Into::into).collect::<Vec<_>>();
    users.shuffle(rng);
    let u2 = users.clone();
    users.into_iter().zip(roles.into_iter().map(|r| r.role(&u2))).collect()
}
