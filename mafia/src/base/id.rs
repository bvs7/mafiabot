use crate::prelude::*;

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(into = "u64", from = "u64")]
pub struct Pid(pub u64);

impl Pid {
    pub fn new() -> Self {
        Self(0)
    }
}

impl From<Pid> for u64 {
    fn from(value: Pid) -> Self {
        value.0
    }
}
impl From<u64> for Pid {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for Pid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub type Choice = Option<Pid>;
pub type Ballot = Option<Choice>;
