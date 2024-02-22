use crate::base::{Choice, ID};
use crate::error::CoreError;
use crate::roles::{Role, Team};

use std::sync::mpsc::Sender;

pub type EventOutput<PID> = Sender<Event<PID>>;

#[derive(Debug)]
pub enum Event<PID: ID> {
    Vote {
        voter: PID,
        ballot: Option<Choice<PID>>,
        former_ballot: Option<Choice<PID>>,
    },
    Reveal {
        player: PID,
        role: Role,
    },
    Target {
        actor: PID,
        target: Choice<PID>,
    },
    Scheme {
        actor: PID,
        mark: Choice<PID>,
    },
    Eliminate {
        player: PID,
        role: Role,
    },
    Election {
        choice: Choice<PID>,
        voters: Vec<PID>,
    },
    Block {
        actor: PID,
        target: PID,
    },
    EvidentBlock {
        blocked: PID,
        blockers: Vec<PID>,
    },
    Save {
        actor: PID,
        target: PID,
    },
    EvidentSave {
        savior: PID,
        mark: PID,
    },
    Investigate {
        actor: PID,
        target: PID,
    },
    Kill {
        killer: PID,
        mark: PID,
    },
    NoNightKill,
    Day {
        day_no: u32,
    },
    Dawn,
}

pub fn send<PID: ID>(
    event_output: &EventOutput<PID>,
    event: Event<PID>,
) -> Result<(), CoreError<PID>> {
    event_output
        .send(event)
        .map_err(|err| CoreError::EventSendError(err))
}
