pub mod action;
pub mod error;
pub mod event;

use std::fmt::{Debug, Display};
use std::sync::mpsc::Sender;

use super::*;

type EventOutput<U> = Sender<Event<U>>;

#[derive(Debug)]
pub struct Comm<U: RawPID> {
    pub tx: EventOutput<U>,
}

impl<U: RawPID> Comm<U> {
    pub fn new(tx: &EventOutput<U>) -> Self {
        Self { tx: tx.to_owned() }
    }

    pub fn tx(&self, event: Event<U>) {
        if let Err(e) = self.tx.send(event) {
            // TODO: Handle this better?
            // Do we need Complete propogation in Game.handle()?
            // It would be difficult to add propogation to every function
            // in Phase, Game, etc?
            println!("Error: {:?}", e);
        }
    }
}

pub trait EventHandler<U: RawPID> {
    fn handle(&mut self, event: Event<U>);
}
