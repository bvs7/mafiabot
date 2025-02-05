use crate::prelude::*;

// mod standard;
// pub use standard::StandardRoleGen;

mod draw;
pub use draw::{DrawRoleGen, DrawRoleGenConfig};

mod debug;
pub use debug::DebugRoleGenConfig;

use rand::{rngs::ThreadRng, seq::SliceRandom, Rng};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum RoleGenConfig {
    Draw(DrawRoleGenConfig),
    Debug(DebugRoleGenConfig),
}

impl Default for RoleGenConfig {
    fn default() -> Self {
        Self::Draw(Default::default())
    }
}

pub trait RoleGen {
    fn generate_roles(&self, players: impl IntoIterator<Item = impl Into<Pid>>)
        -> Vec<(Pid, Role)>;
}

impl RoleGen for RoleGenConfig {
    fn generate_roles(
        &self,
        players: impl IntoIterator<Item = impl Into<Pid>>,
    ) -> Vec<(Pid, Role)> {
        let pids = players.into_iter().map(Into::into).collect::<Vec<_>>();
        match self {
            Self::Draw(config) => DrawRoleGen::generate(config, pids),
            Self::Debug(config) => config.generate_roles(pids),
        }
    }
}

// pub fn assign_roles(
//     users: Vec<impl Into<Pid>>,
//     roles: Vec<Role>,
//     rng: &mut impl Rng,
// ) -> Vec<(Pid, Role)> {
//     let mut users = users.into_iter().map(Into::into).collect::<Vec<_>>();
//     users.shuffle(rng);
//     let u2 = users.clone();
//     users.into_iter().zip(roles.into_iter().map(|r| r.role(&u2))).collect()
// }
