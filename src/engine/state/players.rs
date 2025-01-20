use std::collections::HashMap;

use serde::{Deserialize, Serialize};

use super::{role::Role, Ballot, Choice, Error, PlayerId, RawBallot, RawChoice};

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

impl Context {
    pub fn new(day: u32, cause: Cause) -> Self {
        Self { day, cause }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Players(HashMap<PlayerId, PlayerLog>);

// What operations do we want to perform?
// Act like this is a hashmap to a roles, but store updates in log.
// Get

impl Players {
    pub fn from_registry<'a>(
        registry: impl IntoIterator<Item = (impl Into<PlayerId>, Role)>,
    ) -> Self {
        Self(
            registry
                .into_iter()
                .map(|(p, r)| (p.into(), PlayerLog::from_start_role(&r)))
                .collect(),
        )
    }

    pub fn validate(&self, pid: u64) -> Result<PlayerId, Error> {
        match self.0.get(&pid.into()) {
            Some(PlayerLog {
                pstate: PlayerState::Alive(_),
                ..
            }) => Ok(pid.into()),
            Some(PlayerLog {
                pstate: PlayerState::Dead,
                ..
            }) => Err(Error::DeadPlayer { pid }),
            None => Err(Error::InvalidPlayer { pid }),
        }
    }

    pub fn validate_choice(&self, choice: RawChoice) -> Result<Choice, Error> {
        choice.map(|pid| self.validate(pid)).transpose()
    }
    pub fn validate_ballot(&self, ballot: RawBallot) -> Result<Ballot, Error> {
        ballot
            .map(|choice| self.validate_choice(choice))
            .transpose()
    }

    pub fn alive(&self) -> Vec<(PlayerId, Role)> {
        self.0
            .iter()
            .filter_map(|(pid, plog)| Some((*pid, plog.as_role()?)))
            .collect()
    }
    pub fn get(&self, pid: PlayerId) -> Role {
        let Some(plog) = self.0.get(&pid) else {
            panic!("PlayerId should be valid");
        };
        let Some(role) = plog.as_role() else {
            panic!("Player should be alive...")
        };
        role
    }
    pub fn refocus(&mut self, pid: &PlayerId, role: Role, context: Context) -> PlayerState {
        self.0
            .get_mut(pid)
            .filter(|p| p.is_alive())
            .expect("refocus pid should be valid and alive")
            .update(PlayerState::Alive(role), context)
    }
    pub fn eliminate(&mut self, pid: &PlayerId, context: Context) -> PlayerState {
        self.0
            .get_mut(pid)
            .filter(|p| p.is_alive())
            .expect("eliminate pid should be valid and alive")
            .update(PlayerState::Dead, context)
    }
}
