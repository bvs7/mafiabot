use serde::{de, Deserialize, Serialize};
use std::{collections::HashMap, sync::Arc, time::Duration};
use tokio::sync::{broadcast, oneshot, RwLock};
use tracing::{debug, error, info, instrument, trace, warn};

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(into = "u64", from = "u64")]
pub struct Pid(u64);

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

#[derive(Debug, Clone, Copy, Hash, Eq, PartialEq, Serialize, Deserialize)]
#[serde(into = "u64", from = "u64")]
pub struct Gid(u64);

impl Gid {
    pub fn new() -> Self {
        Self(0)
    }
}

impl From<Gid> for u64 {
    fn from(value: Gid) -> Self {
        value.0
    }
}
impl From<u64> for Gid {
    fn from(value: u64) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for Gid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub type Choice = Option<Pid>;
pub type RawChoice = Option<u64>;
pub type Ballot = Option<Choice>;
pub type RawBallot = Option<RawChoice>;
