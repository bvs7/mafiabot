use serde::{Deserialize, Serialize};
use std::{
    fmt::{Debug, Display},
    sync::mpsc::{Receiver, Sender},
};

use super::phase::*;
use super::player::*;

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
    Vote { voter: U, ballot: Option<Choice<U>> },
    Reveal { celeb: U },
    Target { actor: U, target: Choice<U> },
    Mark { killer: U, mark: Choice<U> },
}
impl<U: RawPID> Command<U> {
    pub fn kind(&self) -> CommandKind {
        match self {
            Command::Vote { .. } => CommandKind::Vote,
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
        voter: Player<U>,
        ballot: Option<Player<U>>,
        former: Option<Option<Player<U>>>,
        threshold: usize,
        count: usize,
    },
    Retract {
        voter: Player<U>,
        former: Option<Option<Player<U>>>,
    },
    Reveal {
        celeb: Player<U>,
    },
    Election {
        electors: Vec<Player<U>>,
        ballot: Option<Player<U>>,
    },
    Night {
        night_no: usize,
    },
    Target {
        actor: Player<U>,
        target: Option<Player<U>>,
    },
    Mark {
        killer: Player<U>,
        mark: Option<Player<U>>,
    },
    Dawn,
    Strip {
        stripper: Player<U>,
        blocked: Player<U>,
    },
    Block {
        blocked: Player<U>,
    },
    Save {
        doctor: Player<U>,
        saved: Player<U>,
    },
    Investigate {
        cop: Player<U>,
        suspect: Player<U>,
        role: Role,
    },
    Kill {
        killer: Player<U>,
        mark: Player<U>,
    },
    NoKill,
    Eliminate {
        player: Player<U>,
    },
    End {
        winner: Winner,
    },
}

impl<U: RawPID> Display for Event<U> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Event::Init => write!(f, "Init"),
            Event::Start { players, phase } => write!(f, "Start: {:?} {:?}", players, phase),
            Event::Day { day_no } => write!(f, "Day: {}", day_no),
            Event::Vote {
                voter,
                ballot,
                former,
                threshold,
                count,
            } => write!(
                f,
                "Vote: {:?} {:?} {:?} {} {}",
                voter, ballot, former, threshold, count
            ),
            Event::Retract { voter, former } => write!(f, "Retract: {:?} {:?}", voter, former),
            Event::Reveal { celeb } => write!(f, "Reveal: {:?}", celeb),
            Event::Election { electors, ballot } => {
                write!(f, "Election: {:?} {:?}", electors, ballot)
            }
            Event::Night { night_no } => write!(f, "Night: {}", night_no),
            Event::Target { actor, target } => write!(f, "Target: {:?} {:?}", actor, target),
            Event::Mark { killer, mark } => write!(f, "Mark: {:?} {:?}", killer, mark),
            Event::Dawn => write!(f, "Dawn"),
            Event::Strip { stripper, blocked } => write!(f, "Strip: {:?} {:?}", stripper, blocked),
            Event::Block { blocked } => write!(f, "Block: {:?}", blocked),
            Event::Save { doctor, saved } => write!(f, "Save: {:?} {:?}", doctor, saved),
            Event::Investigate { cop, suspect, role } => {
                write!(f, "Investigate: {:?} {:?} {:?}", cop, suspect, role)
            }
            Event::Kill { killer, mark } => write!(f, "Kill: {:?} {:?}", killer, mark),
            Event::NoKill => write!(f, "NoKill"),
            Event::Eliminate { player } => write!(f, "Eliminate: {:?}", player),
            Event::End { winner } => write!(f, "End: {:?}", winner),
        }
    }
}

pub enum EventKind {
    Init,
    Start,
    Day,
    Vote,
    Retract,
    Reveal,
    Election,
    Night,
    Target,
    Mark,
    Dawn,
    Strip,
    Block,
    Save,
    Investigate,
    Kill,
    NoKill,
    Eliminate,
    End,
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
        // println!("Game sending event: {}", event.to_string());
        let resp = Response {
            event,
            src: self.src.clone(),
        };
        if let Err(e) = self.tx.send(resp) {
            println!("Error: {:?}", e);
        }
    }
}

pub trait EventHandler<U: RawPID, S: Source> {
    fn handle(&mut self, event: Event<U>, src: S);
}

pub struct DisplayEventHandler {}

impl DisplayEventHandler {
    pub fn new() -> Self {
        Self {}
    }
}

impl<U: RawPID, S: Source> EventHandler<U, S> for DisplayEventHandler {
    fn handle(&mut self, event: Event<U>, _: S) {
        match event {
            Event::Init => println!("Game initialized"),
            Event::Start { players, phase } => {
                println!(
                    "Game started in phase {} with players: {}",
                    phase,
                    players
                        .iter()
                        .map(|p| p.to_string())
                        .collect::<Vec<_>>()
                        .join(", ")
                )
            }
            Event::Day { day_no } => println!("Day {}", day_no),
            Event::Vote {
                voter,
                ballot,
                former,
                threshold,
                count,
            } => println!(
                "Vote: {:?} {:?} {:?} {} {}",
                voter, ballot, former, threshold, count
            ),
            Event::Retract { voter, former } => println!("Retract: {:?} {:?}", voter, former),
            Event::Reveal { celeb } => println!("Reveal: {:?}", celeb),
            Event::Election { electors, ballot } => {
                println!("Election: {:?} {:?}", electors, ballot)
            }
            Event::Night { night_no } => println!("Night {}", night_no),
            Event::Target { actor, target } => {
                println!(
                    "Target: {} {}",
                    actor,
                    target.map(|t| t.to_string()).unwrap_or("None".to_string())
                )
            }
            Event::Mark { killer, mark } => println!(
                "Mark: {} {}",
                killer,
                mark.map(|t| t.to_string()).unwrap_or("None".to_string())
            ),
            Event::Dawn => println!("Dawn"),
            Event::Strip { stripper, blocked } => println!("Strip: {:?} {:?}", stripper, blocked),
            Event::Block { blocked } => println!("Block: {:?}", blocked),
            Event::Save { doctor, saved } => println!("Save: {:?} {:?}", doctor, saved),
            Event::Investigate { cop, suspect, role } => {
                println!("Investigate: {:?} {:?} {:?}", cop, suspect, role)
            }
            Event::Kill { killer, mark } => println!("Kill: {:?} {:?}", killer, mark),
            Event::NoKill => println!("NoKill"),
            Event::Eliminate { player } => println!("Eliminate: {:?}", player),
            Event::End { winner } => println!("End: {:?}", winner),
        }
    }
}
