use std::fmt::Debug;
use std::hash::Hash;

pub trait ID: Eq + Hash + Copy + Debug + Sync + Send + 'static {}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Choice<PID: ID> {
    Player(PID),
    Abstain,
}
