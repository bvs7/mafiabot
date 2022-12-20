use serde::{Deserialize, Serialize};
use std::{
    fmt::Debug,
    sync::mpsc::{Receiver, Sender},
};

use super::player::{Actor, Ballot, Pidx, Player, RawPID, Target, Winner};
use super::Phase;

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
        ballot: Option<Ballot<Pidx>>,
        former: Option<Ballot<Pidx>>,
        threshold: usize,
        count: usize,
    },
    Elect {
        ballot: Ballot<Pidx>,
    },
    Night,
    Action {
        actor: Actor<Pidx>,
        target: Option<Target<Pidx>>,
    },
    Dawn,
    Strip,
    Save,
    Investigate,
    Kill,
    Eliminate {
        player: Pidx,
    },
    Win,
    End,
    InvalidCommand,
}

#[derive(Debug)]
pub struct Comm<U: RawPID, S: Source> {
    pub rx: Receiver<Request<U, S>>,
    pub tx: Sender<Response<U, S>>,
    pub src: S,
}

impl<U: RawPID, S: Source> Comm<U, S> {
    pub fn new(rx: Receiver<Request<U, S>>, tx: Sender<Response<U, S>>) -> Self {
        Self {
            rx,
            tx,
            src: S::default(),
        }
    }

    pub fn rx(&mut self) -> Command<U> {
        loop {
            let req = self.rx.recv();
            match req {
                Err(err) => {
                    println!("Error: {:?}", err);
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
        println!("Sending event: {:?}", event);
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
