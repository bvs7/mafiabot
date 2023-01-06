use std::sync::mpsc::Receiver;

mod interface;

use super::core::Game;
use super::discord::UserID;
use interface::{Command, CommandKind};

pub struct Controller {
    rx: Receiver<Command>,
    game: Option<Game<UserID>>,
}

impl Controller {
    fn controller_thread(self) {
        loop {
            let cmd = match self.rx.recv() {
                Ok(cmd) => cmd,
                Err(err) => continue,
            };

            todo!();
        }
    }
}
