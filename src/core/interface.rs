//! Crate interface

use crate::prelude::*;

use super::*;

use std::default;
use std::sync::mpsc::Sender;

#[derive(Debug, Default, Clone, Copy, Eq, Hash, PartialEq, Serialize)]
pub enum Choice {
    Player(PID),
    #[default]
    Abstain,
}

#[derive(Debug, Default, Clone, Copy, Eq, Hash, PartialEq, Serialize)]
pub enum Election {
    Hammer(PID, PID),
    #[default]
    Peace,
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Serialize)]
pub enum Action {
    Vote { voter: PID, ballot: Option<Choice> },
    Reveal { actor: PID },
    Target { actor: PID, target: Choice },
    Mark { actor: PID, target: Choice },
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub enum Event {
    Vote {
        voter: PID,
        ballot: Option<Choice>,
    },
    Election {
        election: Election,
        voters: HashSet<PID>,
    },
    Revenge {
        avenger: PID,
        votee: PID,
    },
    Reveal {
        player: PID,
        role: Role,
    },
    Target {
        actor: PID,
        target: Choice,
    },
    Mark {
        killer: PID,
        mark: Choice,
    },
    Strip {
        stripper: PID,
        target: PID,
    },
    Block {
        blocked: PID,
    },
    Save {
        doctor: PID,
        target: PID,
    },
    Kill {
        killer: PID,
        mark: PID,
    },
    Investigate {
        cop: PID,
        target: PID,
        role: Role,
    },
    Day {
        day: usize,
        players: HashMap<PID, Role>,
    },
    Dusk {
        day: usize,
        avenger: PID,
        voters: HashSet<PID>,
    },
    Refocus {
        holder: PID,
        former_role: Role,
        new_role: Role,
    },
}

#[derive(Debug, Clone)]
pub struct EventOutput(Sender<Event>);

impl EventOutput {
    pub fn send(&self, event: Event) -> Result<()> {
        self.0.send(event).map_err(|e| Error::MpscSendEventError(e));
        Ok(())
    }
}
