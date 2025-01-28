use crate::prelude::*;

mod standard;
pub use standard::StandardRoleGen;

use rand::Rng;

pub trait RoleGen {
    type RNG: Rng;
    fn generate_roles(n: usize, rules: &Rules, rng: Self::RNG) -> Vec<Role>;
}
