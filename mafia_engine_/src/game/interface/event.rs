use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use tokio::sync::broadcast;

use crate::engine::game::*;

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum Cause {
    Election,
    Kill,
    Vengeance,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub struct Context {
    pub day: u32,
    pub cause: Cause,
}

impl Context {
    pub fn new(day: u32, cause: Cause) -> Self {
        Self { day, cause }
    }
}

// We probably need two generics for start roles and known roles here?
// Maybe even a third for reveal on death...
// Alternatively, have one Rules trait of some sort with associated types!
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub enum Event {
    Start {
        players: Vec<(Pid, Role)>,
        rules: Rules,
    },
    Day {
        day: u32,
        counts: HashMap<Team, usize>, // TODO: make Team generic
    },
    Night {
        day: u32,
        counts: HashMap<Team, usize>, // TODO: make Team generic
    },
    Eclipse {
        avenger: Pid,
        hammer: Pid,
        guilty: Vec<Pid>,
    },
    Vengeance {
        avenger: Pid,
        victim: Pid,
    },
    Vote {
        voter: Pid,
        ballot: Option<(Choice, usize)>,
        former: Option<(Choice, usize)>,
    },
    Reveal {
        celeb: Pid,
    },
    Election {
        choice: Choice,
        hammer: Pid,
        voters: Vec<Pid>,
    },
    Dawn, // Potentially note those who failed to do night actions
    Eliminate {
        player: Pid,
        role: RoleKind,
        context: Context,
    },
    Target {
        actor: Pid,
        choice: Choice,
    },
    Scheme {
        killer: Pid,
        mark: Choice,
    },
    Block {
        blocked: Pid,
        blockers: Vec<Pid>,
    },
    Save {
        saved: Pid,
        saviors: Vec<Pid>,
    },
    NoKill,
    Kill {
        killer: Pid,
        mark: Pid,
    },
    Investigate {
        cop: Pid,
        target: Pid,
        appears_mafia: bool,
    },
    Milk {
        milky: Pid,
        target: Pid,
    },
    End {
        winner: Team,
    },
    Debug,
}

pub type EventRx = broadcast::Receiver<Event>;
pub type EventTx = broadcast::Sender<Event>;
