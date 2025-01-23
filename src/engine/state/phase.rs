use crate::engine::sync_state::night_action::NightAct;

use super::{role::Team, Ballot, Choice, Error, Pid, Players};
use chrono::{DateTime, Local};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, thread::JoinHandle};
use tokio::task::AbortHandle;

pub type Votes = HashMap<Pid, Option<Pid>>; // voter -> ballot
pub type Blocks = HashMap<Pid, Vec<Pid>>; // blocked -> blockers
pub type Targets = HashMap<Pid, Option<Pid>>; // actor -> target
pub type Scheme = (Pid, Option<Pid>); // killer -> mark

#[derive(Debug, Clone, Default, Serialize, Deserialize, EnumKind)]
#[enum_kind(PhaseKind, derive(Serialize, Deserialize))]
pub enum Phase {
    #[default]
    Init,
    Day {
        votes: Votes,
        blocks: Blocks,
        #[serde(skip_serializing_if = "Option::is_none")]
        pend_elect: Option<(Choice, Pid, DateTime<Local>)>,
    },
    Night {
        targets: Targets, // actor -> target
        scheme: Option<Scheme>,
        #[serde(skip_serializing_if = "Option::is_none")]
        pend_dawn: Option<DateTime<Local>>,
    },
    Eclipse {
        avenger: Pid,
        hammer: Pid,
        guilty: Vec<Pid>,
        vote: Option<Pid>,
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

    pub fn eclipse_vote(&mut self, voter: Pid, ballot: Ballot) -> Result<(), Error> {
        let Self::Eclipse {
            avenger,
            hammer,
            guilty,
            vote,
        } = self
        else {
            return self.expected(PhaseKind::Eclipse);
        };
        if voter != *avenger {
            return Err(Error::IneffectiveVote);
        }
        let Some(choice) = ballot else {
            return Err(Error::IneffectiveVote);
        };
        let Some(victim) = choice else {
            return Err(Error::IneffectiveVote);
        };
        if victim == *avenger {
            return Err(Error::IneffectiveVote);
        }
        if !guilty.contains(&victim) {
            return Err(Error::IneffectiveVote);
        }
        *vote = Some(victim);
        Ok(())
    }

    pub fn vote(&mut self, voter: Pid, ballot: Ballot) -> Result<Ballot, Error> {
        let Self::Day { votes, .. } = self else {
            return self.expected(PhaseKind::Day);
        };
        if votes.get(&voter) == ballot.as_ref() {
            return Err(Error::IneffectiveVote);
        }
        let former = if let Some(choice) = ballot {
            votes.insert(voter, choice)
        } else {
            votes.remove(&voter)
        };
        Ok(former)
    }
    pub fn vote_list(&self) -> Result<HashMap<Choice, Vec<Pid>>, Error> {
        let Self::Day { votes, .. } = self else {
            return self.expected(PhaseKind::Day);
        };
        let mut map: HashMap<_, Vec<Pid>> = HashMap::new();
        for (voter, choice) in votes {
            map.entry(*choice).or_default().push(*voter);
        }
        Ok(map)
    }

    // TODO: Stripper must pick one of target/scheme
    pub fn target(&mut self, actor: Pid, choice: Choice) -> Result<Option<Choice>, Error> {
        let Self::Night { targets, .. } = self else {
            return self.expected(PhaseKind::Night);
        };
        Ok(targets.insert(actor, choice))
    }

    // TODO: Stripper must pick one of target/scheme
    pub fn scheme(&mut self, killer: Pid, mark: Choice) -> Result<Option<(Pid, Choice)>, Error> {
        let Self::Night { scheme, .. } = self else {
            return self.expected(PhaseKind::Night);
        };
        Ok(scheme.replace((killer, mark)))
    }
}

impl Drop for Phase {
    /// If the phase is dropped, ensure any leftover timers are aborted
    fn drop(&mut self) {
        match self {
            // Phase::Day {
            //     pend_elect: Some((_, h)),
            //     ..
            // }
            // | Phase::Night {
            //     pend_dawn: Some(h), ..
            // } => h.abort(),
            _ => {}
        }
    }
}

impl std::fmt::Display for PhaseKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PhaseKind::Init => write!(f, "Init"),
            PhaseKind::Day => write!(f, "Day"),
            PhaseKind::Night => write!(f, "Night"),
            PhaseKind::Eclipse => write!(f, "Eclipse"),
            PhaseKind::End => write!(f, "End"),
        }
    }
}
