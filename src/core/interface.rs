//! Crate interface

use crate::prelude::*;

use crate::core::*;

use std::default;
use std::sync::mpsc::Sender;

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
        actor: PID,
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
    Block {
        blocked: PID,
    },
    Dusk {
        avenger: PID,
        voters: HashSet<PID>,
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

#[derive(Debug, Default, Clone, Copy, Eq, Hash, PartialEq, Serialize)]
pub enum Choice {
    Player(PID),
    #[default]
    Abstain,
}
