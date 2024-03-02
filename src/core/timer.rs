// A timer implementation.

use crate::base::ID;
use crate::core::Action;

use chrono;
use serde::{Deserialize, Serialize};
use std::fmt::Debug;
use std::hash::Hash;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Timer<PID: Eq + Hash> {
    pub end_time: chrono::DateTime<chrono::Local>,
    pub data: Action<PID>,
}

impl<PID: ID> Timer<PID> {
    // Or just return an action?
    pub async fn check(&self) -> Option<Action<PID>> {
        let now = chrono::Local::now();
        if now > self.end_time {
            Some(self.data.clone())
        } else {
            None
        }
    }
}
