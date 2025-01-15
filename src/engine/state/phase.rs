use super::role::Team;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Default, Serialize, Deserialize, EnumKind)]
#[enum_kind(PhaseKind, derive(Serialize, Deserialize))]
pub enum Phase {
    #[default]
    Init,
    Day {
        votes: HashMap<u64, Option<u64>>, // voter -> ballot
        blocks: HashMap<u64, Vec<u64>>,   // blocked -> blockers
    },
    Night {
        targets: HashMap<u64, Option<u64>>, // actor -> target
        scheme: Option<(u64, Option<u64>)>, // killer -> mark
    },
    End {
        winner: Team,
    },
}

impl Phase {
    pub fn new(kind: PhaseKind) -> Self {
        match kind {
            PhaseKind::Init => Phase::Init,
            PhaseKind::Day => Phase::Day {
                votes: HashMap::new(),
                blocks: HashMap::new(),
            },
            PhaseKind::Night => Phase::Night {
                targets: HashMap::new(),
                scheme: None,
            },
            PhaseKind::End => Phase::End { winner: Team::Town },
        }
    }
    pub fn kind(&self) -> PhaseKind {
        PhaseKind::from(self)
    }
}

impl std::fmt::Display for PhaseKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}
