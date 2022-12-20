use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    sync::mpsc::{Receiver, Sender},
};

use super::player::{Actor, Ballot, Election, Pidx, Player, RawPID, Role, Target, Winner};
use super::Phase;

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
pub enum Command<U: RawPID> {
    Vote(U, Ballot<U>),
    Action(Actor<U>, Target<U>),
    End,
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
        phase: Phase,
    },
    Day,
    Vote {
        voter: Pidx,
        ballot: Ballot<Pidx>,
        former: Option<Ballot<Pidx>>,
        threshold: usize,
        count: usize,
    },
    RetractVote {
        voter: Pidx,
        former: Option<Ballot<Pidx>>,
    },
    Election {
        electors: Vec<Pidx>,
        ballot: Ballot<Pidx>,
    },
    Night,
    Action {
        actor: Actor<Pidx>,
        target: Target<Pidx>,
    },
    Dawn,
    Strip {
        stripper: Pidx,
        stripped: Pidx,
    },
    Save {
        doctor: Pidx,
        saved: Pidx,
    },
    Investigate {
        cop: Pidx,
        suspect: Pidx,
        role: Role<U>,
    },
    Kill {
        killer: Pidx,
        victim: Pidx,
    },
    NoKill,
    Eliminate {
        player: Pidx,
    },
    Win {
        winner: Winner,
    },
    End,
    InvalidCommand(String),
}

impl<U: RawPID> Event<U> {
    pub fn is_same_type(&self, other: &Event<U>) -> bool {
        match (self, other) {
            (Event::Init, Event::Init) => true,
            (Event::Start { .. }, Event::Start { .. }) => true,
            (Event::Day, Event::Day) => true,
            (Event::Vote { .. }, Event::Vote { .. }) => true,
            (Event::RetractVote { .. }, Event::RetractVote { .. }) => true,
            (Event::Election { .. }, Event::Election { .. }) => true,
            (Event::Night, Event::Night) => true,
            (Event::Action { .. }, Event::Action { .. }) => true,
            (Event::Dawn, Event::Dawn) => true,
            (Event::Strip { .. }, Event::Strip { .. }) => true,
            (Event::Save { .. }, Event::Save { .. }) => true,
            (Event::Investigate { .. }, Event::Investigate { .. }) => true,
            (Event::Kill { .. }, Event::Kill { .. }) => true,
            (Event::NoKill, Event::NoKill) => true,
            (Event::Eliminate { .. }, Event::Eliminate { .. }) => true,
            (Event::Win { .. }, Event::Win { .. }) => true,
            (Event::End, Event::End) => true,
            (Event::InvalidCommand(_), Event::InvalidCommand(_)) => true,
            _ => false,
        }
    }
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
        // println!("Sending event: {:?}", event);
        let resp = Response {
            event,
            src: self.src.clone(),
        };
        match self.tx.send(resp) {
            Err(err) => println!("Error: {:?}", err),
            Ok(_) => {}
        }
    }
}
