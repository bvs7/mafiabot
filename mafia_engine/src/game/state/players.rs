use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::Index};

use crate::engine::game::*;

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
        Self { pstate: PlayerState::Alive(*role), log: Vec::new() }
    }
    fn update(&mut self, pstate: PlayerState, context: impl Into<Context>) -> PlayerState {
        self.log.push((self.pstate, context.into()));
        std::mem::replace(&mut self.pstate, pstate)
    }
    fn as_role(&self) -> Option<Role> {
        match &self.pstate {
            PlayerState::Alive(role) => Some(*role),
            PlayerState::Dead => None,
        }
    }
    fn is_alive(&self) -> bool {
        matches!(self.pstate, PlayerState::Alive(_))
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Players {
    map: HashMap<Pid, PlayerLog>,
    list: Vec<Pid>,
}

// What operations do we want to perform?
// Act like this is a hashmap to a roles, but store updates in log.
// Get

impl Players {
    pub fn from_registry<'a>(registry: impl IntoIterator<Item = (impl Into<Pid>, Role)>) -> Self {
        let map: HashMap<Pid, PlayerLog> =
            registry.into_iter().map(|(p, r)| (p.into(), PlayerLog::from_start_role(&r))).collect();
        let list: Vec<Pid> = map.keys().copied().collect();
        Self { map, list }
    }

    pub fn validate(&self, pid: u64) -> Result<Pid, Error> {
        match self.map.get(&pid.into()) {
            Some(PlayerLog { pstate: PlayerState::Alive(_), .. }) => Ok(pid.into()),
            Some(PlayerLog { pstate: PlayerState::Dead, .. }) => Err(Error::DeadPlayer { pid }),
            None => Err(Error::InvalidPlayer { pid }),
        }
    }

    pub fn validate_choice(&self, choice: RawChoice) -> Result<Choice, Error> {
        choice.map(|pid| self.validate(pid)).transpose()
    }
    pub fn validate_ballot(&self, ballot: RawBallot) -> Result<Ballot, Error> {
        ballot.map(|choice| self.validate_choice(choice)).transpose()
    }

    pub fn alive(&self) -> Vec<(Pid, Role)> {
        self.map.iter().filter_map(|(pid, plog)| Some((*pid, plog.as_role()?))).collect()
    }
    pub fn n(&self) -> usize {
        self.alive().len()
    }
    pub fn get(&self, pid: Pid) -> Role {
        let Some(plog) = self.map.get(&pid) else {
            panic!("PlayerId should be valid");
        };
        let Some(role) = plog.as_role() else { panic!("Player should be alive...") };
        role
    }
    pub fn is_alive(&self, pid: Pid) -> bool {
        self.map.get(&pid).map_or(false, |plog| plog.is_alive())
    }

    pub fn list(&self) -> Vec<Pid> {
        self.list.clone()
    }

    pub fn get_target(&self, idx: usize) -> Result<Option<Pid>, Error> {
        if idx > self.list.len() {
            return Err(Error::InvalidTarget { idx: idx });
        }
        Ok(self.list.get(idx).copied())
    }
    pub fn refocus(&mut self, pid: &Pid, role: Role, context: Context) -> PlayerState {
        self.map
            .get_mut(pid)
            .filter(|p| p.is_alive())
            .expect("refocus pid should be valid and alive")
            .update(PlayerState::Alive(role), context)
    }

    pub fn eliminate(&mut self, pid: &Pid, context: Context) -> PlayerState {
        let idx = self.list.iter().position(|p| p == pid).expect("eliminate pid should be valid");
        self.list.remove(idx);
        self.map
            .get_mut(pid)
            .filter(|p| p.is_alive())
            .expect("eliminate pid should be valid and alive")
            .update(PlayerState::Dead, context)
    }
    pub fn counts<C, F>(&self, f: F) -> HashMap<C, usize>
    where
        C: std::hash::Hash + Eq,
        F: Fn(Role) -> C,
    {
        let mut counts = HashMap::new();
        for (_, role) in self.alive() {
            *counts.entry(f(role)).or_insert(0) += 1;
        }
        counts
    }
}
