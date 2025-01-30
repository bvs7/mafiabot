use crate::prelude::*;

mod standard;
pub use standard::StandardRoleGen;

mod draw;
pub use draw::DrawRoleGen;

use rand::{rngs::ThreadRng, seq::SliceRandom, Rng};

pub trait RoleGen {
    fn generate_roles(&mut self, users: Vec<impl Into<Pid>>, rules: &Rules) -> Vec<(Pid, Role)>;
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
