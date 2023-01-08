use std::sync::mpsc::Receiver;

mod interface;

use super::core::*;
use super::discord::*;
use interface::{Command, CommandKind};
use rand::seq::SliceRandom;

pub struct Controller {
    rx: Receiver<Command>,
    game: Option<Game<UserID>>,
    rx_events: Receiver<Event<UserID>>,
    init_msg: Option<MessageID>,
}

fn get_lobby() -> ChannelID {
    0
}

impl Controller {
    fn controller_thread(mut self) {
        loop {
            let cmd = match self.rx.recv() {
                Ok(cmd) => cmd,
                Err(err) => continue,
            };

            // Handle command.

            match cmd.kind {
                CommandKind::Init => {
                    self.init_msg = Some(send_to_channel(
                        get_lobby(),
                        "React with *️⃣ to join game".to_string(),
                    ));
                }
                CommandKind::Start => {
                    if let Some(msgID) = self.init_msg {
                        let users = get_reacts_to_message(get_lobby(), msgID, "*️⃣".to_string());
                        let roleset = full_roleset();
                        let mut roles = get_roles(users.len(), 0.4, &roleset);
                        let (players, contracts) = get_players(users, roles.clone());
                        let (tx, rx) = std::sync::mpsc::channel::<Event<UserID>>();
                        self.game = Some(Game::new(players, contracts, Comm::new(&tx)));
                    }
                }
                CommandKind::Vote { text, mentions } => {
                    if let Some(game) = &mut self.game {
                        let voter = cmd.sender;
                        let mut ballot = None;
                        match mentions.len() {
                            0 => {
                                let words = text.split(" ").collect::<Vec<&str>>();
                                if words.len() >= 2 && words[1] == "nokill" {
                                    ballot = Some(Choice::Abstain);
                                }
                            }
                            1 => {
                                let target = mentions[0];
                                ballot = Some(Choice::Player(target));
                            }
                            _ => {}
                        }
                        if let Err(err) = game.handle(Action::Vote { voter, ballot }) {
                            send_to_channel(cmd.channel, format!("{:?}", err));
                        }
                    }
                }
                CommandKind::Reveal => {
                    if let Some(game) = &mut self.game {
                        let celeb = cmd.sender;
                        if let Err(err) = game.handle(Action::Reveal { celeb }) {
                            send_to_channel(cmd.channel, format!("{:?}", err));
                        }
                    }
                }
                CommandKind::Target { text } => {
                    if let Some(game) = &mut self.game {
                        let actor = cmd.sender;
                        let words = text.split(" ").collect::<Vec<&str>>();
                        let target = if words.len() >= 2 {
                            // TODO: Parse target option...
                            Some(0)
                        } else {
                            None
                        };
                    }
                }
                _ => {}
            }
        }
    }
}
