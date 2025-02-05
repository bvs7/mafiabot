use crate::prelude::*;

use super::night_action::{Act, NightAct};

// pub type Votes = HashMap<Pid, Option<Pid>>; // voter -> ballot
pub type Votes = Vec<(Pid, Option<Pid>)>; // voter -> ballot
pub type Blocks = HashMap<Pid, Vec<Pid>>; // blocked -> blockers

pub type Targets = Vec<(Pid, Option<Pid>)>; // actor -> target

// pub type Targets = Vec<Target>;
// pub type Scheme = (Pid, Option<Pid>); // killer -> mark

#[derive(Debug, Clone, Default, Serialize, Deserialize, EnumKind)]
#[enum_kind(PhaseKind, derive(Serialize, Deserialize))]
pub enum Phase {
    #[default]
    Init,
    Day {
        votes: Votes,
        blocks: Blocks,
        #[serde(skip_serializing_if = "Option::is_none")]
        elect: Option<(Choice, Pid, DateTime<Local>)>,
    },
    Night {
        targets: Targets, // actor -> target
        #[serde(skip_serializing_if = "Option::is_none")]
        dawn: Option<DateTime<Local>>,
    },
    Eclipse {
        avenger: Pid,
        hammer: Pid,
        guilty: Vec<Pid>,
        #[serde(skip_serializing_if = "Option::is_none")]
        vengeance: Option<Pid>,
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
        Err(Error::InvalidPhase { expected, actual: self.kind() })
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

impl Default for PhaseKind {
    fn default() -> Self {
        PhaseKind::Init
    }
}
