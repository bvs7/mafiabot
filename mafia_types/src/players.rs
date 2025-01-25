use crate::prelude::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub enum PlayerState {
    Dead,
    #[serde(untagged)]
    Alive(Role),
}

impl From<Role> for PlayerState {
    fn from(role: Role) -> Self {
        PlayerState::Alive(role)
    }
}

impl TryFrom<PlayerState> for Role {
    type Error = Error;
    fn try_from(pstate: PlayerState) -> Result<Self, Self::Error> {
        match pstate {
            PlayerState::Alive(role) => Ok(role),
            PlayerState::Dead => Err(Error::DeadPlayer),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Context {}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Players {
    map: HashMap<Pid, PlayerState>,
    log: HashMap<Pid, Vec<(PlayerState, Context)>>,
}

// What operations do we want to perform?
// Act like this is a hashmap to a roles, but store updates in log.
// Get

impl Players {
    pub fn from_registry<'a>(registry: impl IntoIterator<Item = (impl Into<Pid>, Role)>) -> Self {
        let map: HashMap<Pid, PlayerState> =
            registry.into_iter().map(|(p, r)| (p.into(), r.into())).collect();
        let log = HashMap::new();
        Self { map, log }
    }

    pub fn validate(&self, pid: impl Into<Pid> + Copy) -> Result<Pid, Error> {
        match self.map.get(&pid.into()) {
            Some(PlayerState::Alive(_)) => Ok(pid.into()),
            Some(PlayerState::Dead) => Err(Error::DeadPlayer),
            None => Err(Error::InvalidPlayer { pid: pid.into().into() }),
        }
    }

    pub fn validate_choice(&self, choice: Option<impl Into<Pid> + Copy>) -> Result<Choice, Error> {
        choice.map(|pid| self.validate(pid)).transpose()
    }
    pub fn validate_ballot(
        &self,
        ballot: Option<Option<impl Into<Pid> + Copy>>,
    ) -> Result<Ballot, Error> {
        ballot.map(|choice| self.validate_choice(choice)).transpose()
    }

    pub fn get_role(&self, pid: Pid) -> Result<Role, Error> {
        match self.map.get(&pid) {
            Some(PlayerState::Alive(role)) => Ok(*role),
            Some(PlayerState::Dead) => Err(Error::DeadPlayer),
            None => Err(Error::InvalidPlayer { pid: pid.into() }),
        }
    }

    // Always sort the list of players by pid to ensure consistent ordering.
    pub fn alive(&self) -> Vec<(Pid, Role)> {
        let mut alive: Vec<(Pid, Role)> = self
            .map
            .iter()
            .filter_map(|(p, ps)| Role::try_from(*ps).ok().map(|r| (*p, r)))
            .collect();
        alive.sort_by_key(|(p, _)| *p);
        alive
    }
    pub fn n(&self) -> usize {
        self.alive().len()
    }
    pub fn refocus(&mut self, pid: Pid, role: Role, context: Context) {
        let old_role: Role = self
            .map
            .insert(pid, role.into())
            .expect("refocus pid should be valid")
            .try_into()
            .expect("refocus pid should be alive");
        self.log.entry(pid).or_default().push((old_role.into(), context));
    }

    pub fn eliminate(&mut self, pid: Pid, context: Context) -> Role {
        let old_role: Role = self
            .map
            .insert(pid, PlayerState::Dead)
            .expect("eliminate pid should be valid")
            .try_into()
            .expect("eliminate pid should be alive");
        self.log.entry(pid).or_default().push((old_role.into(), context));
        old_role
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
