//! Crate interface

use crate::prelude::*;

use crate::core::*;

use std::sync::mpsc::Sender;

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Serialize)]
pub enum Action {
    Vote { voter: PID, ballot: Option<Choice> },
    Reveal { actor: PID },
    Target { actor: PID, target: Choice },
    Mark { actor: PID, target: Choice },
}

// Might expand this to Recv<(Action, Context)> or something
pub struct ActionInput(Receiver<Action>);

impl ActionInput {
    pub fn recv(&self) -> Result<Action> {
        self.0.recv().map_err(|e| Error::MpscRecvActionError(e))
    }
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Serialize)]
pub enum Event {}

pub struct EventOutput(Sender<Event>);

impl EventOutput {
    pub fn send(&self, event: Event) -> Result<()> {
        self.0.send(event).map_err(|e| Error::MpscSendEventError(e));
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, Hash, PartialEq, Serialize)]
pub enum Choice {
    Player(PID),
    Abstain,
}
