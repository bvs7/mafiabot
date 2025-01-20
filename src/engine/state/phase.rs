use super::{
    night_action::{NightAct, NightAction},
    role::Team,
    Ballot, Choice, Error, PlayerId, Players,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, thread::JoinHandle};
use tokio::task::AbortHandle;

pub type Votes = HashMap<PlayerId, Option<PlayerId>>; // voter -> ballot
pub type Blocks = HashMap<PlayerId, Vec<PlayerId>>; // blocked -> blockers
pub type Targets = HashMap<PlayerId, Option<PlayerId>>; // actor -> target
pub type Scheme = (PlayerId, Option<PlayerId>); // killer -> mark

#[derive(Debug, Clone, Default, Serialize, Deserialize, EnumKind)]
#[enum_kind(PhaseKind, derive(Serialize, Deserialize))]
pub enum Phase {
    #[default]
    Init,
    Day {
        votes: Votes,
        blocks: Blocks,
        #[serde(skip)]
        pend_elect: Option<(Choice, AbortHandle)>,
    },
    Night {
        targets: Targets, // actor -> target
        scheme: Option<Scheme>,
        #[serde(skip)]
        pend_dawn: Option<AbortHandle>,
    },
    End {
        winner: Team,
    },
}

impl Phase {
    pub fn kind(&self) -> PhaseKind {
        PhaseKind::from(self)
    }

    pub fn expected<T>(&self, expected: PhaseKind) -> Result<T, Error> {
        Err(Error::InvalidPhase {
            expected,
            actual: self.kind(),
        })
    }

    pub fn vote(&mut self, voter: PlayerId, ballot: Ballot) -> Result<Ballot, Error> {
        let Self::Day { votes, .. } = self else {
            return self.expected(PhaseKind::Day);
        };
        let former = votes.get(&voter).copied();
        if former == ballot {
            return Err(Error::IneffectiveVote);
        }
        if let Some(choice) = ballot {
            votes.insert(voter, choice);
        }
        Ok(former)
    }
    pub fn vote_list(&self) -> Result<HashMap<Choice, Vec<PlayerId>>, Error> {
        let Self::Day { votes, .. } = self else {
            return self.expected(PhaseKind::Day);
        };
        let mut map: HashMap<_, Vec<PlayerId>> = HashMap::new();
        for (voter, choice) in votes {
            map.entry(*choice).or_default().push(*voter);
        }
        Ok(map)
    }

    pub fn target(&mut self, actor: PlayerId, choice: Choice) -> Result<Option<Choice>, Error> {
        let Self::Night { targets, .. } = self else {
            return self.expected(PhaseKind::Night);
        };
        Ok(targets.insert(actor, choice))
    }

    pub fn scheme(
        &mut self,
        killer: PlayerId,
        mark: Choice,
    ) -> Result<Option<(PlayerId, Choice)>, Error> {
        let Self::Night { scheme, .. } = self else {
            return self.expected(PhaseKind::Night);
        };
        Ok(scheme.replace((killer, mark)))
    }

    pub fn to_night_actions(&self, players: &Players) -> impl Iterator<Item = NightAction> {
        let Phase::Night {
            targets, scheme, ..
        } = self
        else {
            panic!("To night actions during not night");
        };
        let mut night_actions: Vec<NightAction> = targets
            .into_iter()
            .flat_map(|(a, t)| t.map(|t| NightAction::from_target(players.get(*a), *a, t)))
            .collect();
        let s = scheme
            .map(|(actor, m)| {
                m.map(|target| NightAction {
                    act: NightAct::Kill,
                    actor,
                    target,
                })
            })
            .flatten();
        night_actions.extend(s);
        night_actions.sort();
        night_actions.into_iter().rev()
    }
}

impl Drop for Phase {
    /// If the phase is dropped, ensure any leftover timers are aborted
    fn drop(&mut self) {
        match self {
            Phase::Day {
                pend_elect: Some((_, h)),
                ..
            }
            | Phase::Night {
                pend_dawn: Some(h), ..
            } => h.abort(),
            _ => {}
        }
    }
}

impl std::fmt::Display for PhaseKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
