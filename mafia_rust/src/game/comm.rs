use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    sync::mpsc::{Receiver, Sender},
};

use super::{
    player::{Choice, Pidx, Player, RawPID, Role, Team, Winner},
    PhaseKind,
};

// A generic way to store the game?
#[derive(Default, Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum SaveStrategy {
    #[default]
    Never,
    PerPhase(String),
    PerChange(String),
}

// Eventually this will require a way to respond?
pub trait Source: Debug + Clone + Default + Send {}

/// Has details about where the command came from
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Request<U: RawPID, S: Source> {
    pub cmd: Command<U>,
    pub src: S,
    // Implementation specifics
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CommandKind {
    Vote,
    Retract,
    Reveal,
    Target,
    Mark,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Command<U: RawPID> {
    Vote { voter: U, ballot: Choice<U> },
    Retract { voter: U },
    Reveal { celeb: U },
    Target { actor: U, target: Choice<U> },
    Mark { killer: U, mark: Choice<U> },
}
impl<U: RawPID> Command<U> {
    pub fn kind(&self) -> CommandKind {
        match self {
            Command::Vote { .. } => CommandKind::Vote,
            Command::Retract { .. } => CommandKind::Retract,
            Command::Reveal { .. } => CommandKind::Reveal,
            Command::Target { .. } => CommandKind::Target,
            Command::Mark { .. } => CommandKind::Mark,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Response<U: RawPID, S: Source> {
    pub event: Event<U>,
    pub src: S,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Event<U: RawPID> {
    Init,
    Start {
        players: Vec<Player<U>>,
        phase: PhaseKind,
    },
    Day {
        day_no: usize,
    },
    Vote {
        voter: Pidx,
        ballot: Choice<Pidx>,
        former: Option<Choice<Pidx>>,
        threshold: usize,
        count: usize,
    },
    Retract {
        voter: Pidx,
        former: Option<Choice<Pidx>>,
    },
    Reveal {
        celeb: Pidx,
    },
    Election {
        electors: Vec<Pidx>,
        ballot: Choice<Pidx>,
    },
    Night {
        night_no: usize,
    },
    Target {
        actor: Pidx,
        target: Choice<Pidx>,
    },
    Mark {
        killer: Pidx,
        mark: Choice<Pidx>,
    },
    Dawn,
    Strip {
        stripper: Pidx,
        blocked: Pidx,
    },
    Block {
        blocked: Pidx,
    },
    Save {
        doctor: Pidx,
        saved: Pidx,
    },
    Investigate {
        cop: Pidx,
        suspect: Pidx,
        team: Team,
    },
    Kill {
        killer: Pidx,
        mark: Pidx,
    },
    NoKill,
    Eliminate {
        player: Pidx,
    },
    Halt,
    End {
        winner: Winner,
    },
}

#[derive(Debug)]
pub struct Comm<U: RawPID, S: Source> {
    pub rx: Receiver<Request<U, S>>,
    pub tx: Sender<Response<U, S>>,
    pub src: S,
    pub save: SaveStrategy,
}

impl<U: RawPID, S: Source> Comm<U, S> {
    pub fn new(rx: Receiver<Request<U, S>>, tx: Sender<Response<U, S>>) -> Self {
        Self {
            rx,
            tx,
            src: S::default(),
            save: SaveStrategy::default(),
        }
    }

    pub fn rx(&mut self) -> Command<U> {
        loop {
            let req = self.rx.recv();
            match req {
                Err(err) => {
                    println!("Recv Error in Comm!: {:?}", err);
                    continue;
                }
                Ok(req) => {
                    self.src = req.src.clone();
                    return req.cmd;
                }
            }
        }
    }
    pub fn tx(&self, event: Event<U>) {
        println!("Game sending event: {:?}", event);
        let resp = Response {
            event,
            src: self.src.clone(),
        };
        if let Err(e) = self.tx.send(resp) {
            println!("Error: {:?}", e);
        }
    }
}
