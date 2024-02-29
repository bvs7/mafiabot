use std::fmt::Debug;
use std::hash::Hash;

use serde::{Deserialize, Serialize};

pub trait ID: Eq + Hash + Copy + Debug + Default + Sync + Send + Serialize + 'static {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Choice<PID: ID> {
    Player(PID),
    Abstain,
}

impl<PID: ID> Into<Option<PID>> for Choice<PID> {
    fn into(self) -> Option<PID> {
        match self {
            Choice::Player(pid) => Some(pid),
            Choice::Abstain => None,
        }
    }
}
