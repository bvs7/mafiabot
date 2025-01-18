use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::role::Role;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PlayerState {
    Dead,
    #[serde(untagged)]
    Alive(Role),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PlayerLog {
    pstate: PlayerState, // None means dead
    #[serde(skip_serializing_if = "Vec::is_empty")]
    #[serde(default)]
    log: Vec<(PlayerState, Context)>,
}

impl PlayerLog {
    fn from_start_role(role: &Role) -> Self {
        Self {
            pstate: PlayerState::Alive(*role),
            log: Vec::new(),
        }
    }
    fn update(&mut self, pstate: PlayerState, context: impl Into<Context>) {
        self.log.push((self.pstate, context.into()));
        self.pstate = pstate;
    }
    fn as_role(&self) -> Option<&Role> {
        match &self.pstate {
            PlayerState::Alive(role) => Some(role),
            PlayerState::Dead => None,
        }
    }
    fn is_alive(&self) -> bool {
        matches!(self.pstate, PlayerState::Alive(_))
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum Cause {
    Election,
    Kill,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct Context {
    pub day: u32,
    pub cause: Cause,
}

#[derive(Debug, Default, Serialize, Deserialize)]
pub struct Players(HashMap<u64, PlayerLog>);

// What operations do we want to perform?
// Act like this is a hashmap to a roles, but store updates in log.
// Get

impl Players {
    pub fn from_registry<'a>(registry: impl IntoIterator<Item = &'a (u64, Role)>) -> Self {
        Self(
            registry
                .into_iter()
                .map(|(p, r)| (*p, PlayerLog::from_start_role(r)))
                .collect(),
        )
    }

    pub fn alive(&self) -> Vec<(u64, Role)> {
        self.0
            .iter()
            .filter_map(|(pid, plog)| Some((*pid, *plog.as_role()?)))
            .collect()
    }
    pub fn get(&self, pid: &u64) -> Option<&Role> {
        self.0.get(pid)?.as_role()
    }
    pub fn refocus(&mut self, pid: &u64, role: Role, context: impl Into<Context>) {
        self.0
            .get_mut(pid)
            .filter(|p| p.is_alive())
            .expect("refocus pid should be valid and alive")
            .update(PlayerState::Alive(role), context)
    }
    pub fn eliminate(&mut self, pid: &u64, context: impl Into<Context>) {
        self.0
            .get_mut(pid)
            .filter(|p| p.is_alive())
            .expect("eliminate pid should be valid and alive")
            .update(PlayerState::Dead, context)
    }
}
