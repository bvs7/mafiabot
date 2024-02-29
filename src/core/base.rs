use std::fmt::{Debug, Display};
use std::hash::Hash;

use serde::{Deserialize, Serialize};

pub trait ID:
    Eq + Hash + Copy + Debug + Display + Default + Sync + Send + Serialize + 'static
{
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Choice<PID> {
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
