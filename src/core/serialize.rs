use crate::prelude::*;

use super::*;

#[derive(Debug, Clone)]
pub enum SerRole<T> {
    Role(Role_<T>),
    Hist(Vec<Role_<T>>),
}

impl Serialize for SerRole<String> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        match self {
            SerRole::Role(role) => role.serialize(serializer),
            SerRole::Hist(history) => history.serialize(serializer),
        }
    }
}

impl From<RoleHist> for SerRole<String> {
    fn from(rh: RoleHist) -> Self {
        if rh.history.is_empty() {
            SerRole::Role(rh.role)
        } else {
            let mut all_roles = rh.history.clone();
            all_roles.push(rh.role);
            SerRole::Hist(all_roles)
        }
    }
}

impl From<SerRole<String>> for RoleHist {
    fn from(sr: SerRole<String>) -> Self {
        match sr {
            SerRole::Role(role) => RoleHist {
                role,
                history: Vec::new(),
            },
            SerRole::Hist(history) => {
                let role = history.last().unwrap().clone();
                let history = history[..history.len() - 1].to_vec();
                RoleHist { role, history }
            }
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct SerPlayers(HashMap<String, SerRole<String>>);

impl From<(Players, &Names)> for SerPlayers {
    fn from((players, names): (Players, &Names)) -> Self {
        let mut map = HashMap::new();
        for (pid, rh) in players.0 {
            let name = names.get(&pid).unwrap().clone();
            map.insert(name, SerRole::from(rh));
        }
        SerPlayers(map)
    }
}

impl From<(SerPlayers, &Names)> for Players {
    fn from((ser_players, names): (SerPlayers, &Names)) -> Self {
        let mut map = HashMap::new();
        for (name, sr) in ser_players.0 {
            let pid = names.iter().find(|(_, n)| *n == &name).unwrap().0;
            map.insert(pid.clone(), RoleHist::from(sr));
        }
        Players(map)
    }
}
