use serde::Serialize;
use std::fmt::{Debug, Display};
use std::sync::mpsc::Sender;

use kinded::Kinded;

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Choice {
    Player(PID),
    Abstain,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Kinded, Serialize)]
pub enum Action {
    Vote { voter: PID, ballot: Option<Choice> },
    Reveal { celeb: PID },
    Target { actor: PID, target: Option<Choice> },
    Mark { killer: PID, mark: Option<Choice> },
}

#[derive(Debug, Clone, PartialEq, Kinded, Serialize)]
pub enum Event {
    Init {
        game_id: usize,
    },
    Start {
        role_assign: RoleAssign,
        phase: PhaseKind,
    },
    Day {
        num: usize,
        players: Vec<PID>,
    },
    Vote {
        voter: PID,
        ballot: Option<Choice>,
    },
    Reveal {
        celeb: PID,
    },
    Election {
        electors: PIDs,
        elected: Choice,
    },
    Night {
        num: usize,
        players: PIDs,
    },
    Target {
        actor: PID,
        target: Choice,
    },
    Mark {
        killer: PID,
        mark: Choice,
    },
    Dawn,
    Strip {
        stripper: PID,
        blocked: PID,
    },
    Block {
        blocked: PID,
    },
    Save {
        doctor: PID,
        mark: PID,
    },
    Saved {
        mark: PID,
    },
    Investigate {
        cop: PID,
        suspect: PID,
        role: Role,
    },
    Kill {
        killer: PID,
        mark: PID,
    },
    NoKill,
    Eliminate {
        eliminated: PID,
        role: Role,
    },
    ContractUpdate {
        contract: Contract,
    },
    Refocus {
        new_contract: Contract,
    },
    End {
        winner: Option<Team>,
        contracts: HashSet<Contract>,
    },
}

#[derive(Debug)]
pub struct EventOutput {
    sender: Sender<Event>,
}

impl EventOutput {
    pub fn send(&self, event: Event) {
        self.sender
            .send(event)
            .expect("Failed to send event {event}");
    }
}

#[derive(Debug)]
pub enum CoreError {
    /// Expected PhaseKind, Found PhaseKind
    InvalidPhase(PhaseKind, PhaseKind),
    PlayerNotFound(PID),

    InvalidRole {
        role: Role,
        action: ActionKind,
    },
    NoGame,
    InvalidTargetText {
        text: String,
    },
    InvalidTarget {
        target: PID,
    },
    /// Expected StateKind, Found StateKind
    InvalidState(StateKind, StateKind),
}
impl Display for CoreError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Error with command: ")?;
        match self {
            _ => write!(f, "Unknown Error"),
        }
    }
}

impl std::error::Error for CoreError {}
