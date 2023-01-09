use std::collections::HashMap;
use std::sync::mpsc::Receiver;

mod interface;

use super::core::*;
use super::discord::*;
use interface::{Command, CommandKind};
use rand::seq::SliceRandom;

/*
Notes for controller implementation.

Things the Controller needs to do:
- Route Commands to the Game
    - These are !vote, !target, !mark, !reveal commands
    - Parse them into Actions, and handle them with the Game.handle() method
    - Handle an error by sending a message to the channel the command was sent in.
- Route Events to Discord
    - Create and EventHandler (and probably a standalond thread). Read Events from the Game's event queue, and send the appropriate messages to Discord.
- Create and hold Game
    - Use !init and !start commands, and the data associated with them, to create a Game.
    - After the game ends, destroy the GameData? The channels, and the Game itself?
        - For now, maybe have a !close command that destroys the Game? Later, have a timer that destroys the Game after a certain amount of inactivity?


Notes:
- Maybe find lobby by searching for a channel with "lobby" in the name?
*/

pub struct GameData {
    game: Game<UserID>,
    main_channel: ChannelID,
    mafia_channel: ChannelID,
    user_channels: HashMap<UserID, ChannelID>,
    rx_events: Receiver<Event<UserID>>,
}
// TODO: Implement handling !vote, !target, !mark, !reveal commands here

impl GameData {
    fn handle(&mut self, cmd: &Command) {
        let result = match &cmd.kind {
            CommandKind::Vote { text, mentions } => {
                let mut ballot: Option<Choice<UserID>> = None;
                if mentions.len() >= 1 {
                    ballot = Some(Choice::Player(mentions[0]));
                } else if check_text_for_nokill_vote(&text) {
                    ballot = Some(Choice::Abstain);
                }
                self.game.handle(Action::Vote {
                    voter: cmd.sender,
                    ballot,
                })
            }
            CommandKind::Reveal => self.game.handle(Action::Reveal { celeb: cmd.sender }),
            CommandKind::Target { text } => {
                if let Some(idx) = check_text_for_target_Pidx(text) {
                    let target = Choice::Player(idx);
                    self.game.handle(Action::Target {
                        actor: cmd.sender,
                        target,
                    })
                } else {
                    Err(InvalidActionError::InvalidTargetText { text: text.clone() })
                }
            }
            CommandKind::Mark { text } => {
                if let Some(idx) = check_text_for_target_Pidx(text) {
                    let mark = Choice::Player(idx);
                    self.game.handle(Action::Mark {
                        killer: cmd.sender,
                        mark,
                    })
                } else {
                    Err(InvalidActionError::InvalidTargetText { text: text.clone() })
                }
            }
            _ => Ok(()),
        };
        if let Err(err) = result {
            send_to_channel(cmd.channel, err.to_string());
        }
    }
}

fn check_text_for_nokill_vote(text: &String) -> bool {
    let words: Vec<&str> = text.split(" ").collect();
    if words.len() >= 2 {
        words[1] == "nokill"
    } else {
        false
    }
}

fn check_text_for_target_Pidx(text: &String) -> Option<Pidx> {
    let words: Vec<&str> = text.split(" ").collect();
    if words.len() >= 2 {
        let target = words[1];
        if target.len() > 1 {
            None
        } else {
            let target: u32 = target.chars().nth(0).unwrap().into();
            let a: u32 = 'A'.into();
            Some((target - a) as usize)
        }
    } else {
        None
    }
}

trait GenChannels {
    fn gen_channels(&self) -> (ChannelID, ChannelID, HashMap<UserID, ChannelID>);
}

impl GenChannels for Game<UserID> {
    fn gen_channels(&self) -> (ChannelID, ChannelID, HashMap<UserID, ChannelID>) {
        // TODO: create a category?
        let main_channel = create_channel("main".to_string());
        let mafia_channel = create_channel("mafia".to_string());
        let mut user_channels = HashMap::new();
        for p in self.players {
            user_channels.insert(p.raw_pid, create_single_user_channel(p.raw_pid));
        }
        (main_channel, mafia_channel, user_channels)
    }
}

pub struct Lobby {
    channel: ChannelID,
    init_msg: Option<MessageID>,
}
// TODO: Implement handling !init and !start commands here
// !start command returns a GameData struct, which contains the Game, and the channels for the game.

impl Lobby {
    fn handle(&mut self, cmd: &Command) -> Option<GameData> {
        let result = match &cmd.kind {
            CommandKind::Init => {
                // Send init message
                self.init_msg = Some(send_to_channel(
                    self.channel,
                    "*️⃣ React to this message to join next game".to_string(),
                ));
                Ok(None)
            }
            CommandKind::Status => {
                // Get init message
                if let Some(msg) = self.init_msg {
                    let users = get_reacts_to_message(self.channel, msg, "*️⃣".to_string());
                    let roleset = full_roleset();
                    let roles = get_roles(users.len(), 0.4, &roleset);
                    let (players, contracts) = get_players(users, roles);
                    let (tx, rx_events) = std::sync::mpsc::channel();
                    let night_actors: Vec<UserID> = Vec::new();
                    let game = Game::new(0, players, contracts, Comm::new(&tx));
                    let (main_channel, mafia_channel, user_channels) = game.gen_channels();
                    let game_data = GameData {
                        game,
                        main_channel,
                        mafia_channel,
                        user_channels,
                        rx_events,
                    };
                    Ok(Some(game_data))
                } else {
                    Ok(None)
                }
            }
            _ => Err(()),
        };

        match result {
            Ok(g) => g,
            Err(err) => {
                send_to_channel(cmd.channel, "Failed...".to_string());
                None
            }
        }
    }
}

pub struct Controller {
    rx: Receiver<Command>,
    game_data: Option<GameData>,
    lobby: Lobby,
}

impl Controller {
    fn start() {
        let (tx, rx) = std::sync::mpsc::channel();
        let lobby: Lobby = Lobby {
            channel: 0 as ChannelID, // TODO! Find lobby channel.
            init_msg: None,
        };
        // TODO: Recover a game from save?
        let controller = Controller {
            rx,
            game_data: None,
            lobby,
        };
        std::thread::spawn(move || controller.controller_thread());
    }

    fn controller_thread(mut self) {
        loop {
            let cmd = match self.rx.recv() {
                Ok(cmd) => cmd,
                Err(err) => continue,
            };

            // Handle command.

            let result = match cmd.kind {
                CommandKind::Init | CommandKind::Start => {
                    if let Some(game_data) = self.lobby.handle(&cmd) {
                        self.game_data = Some(game_data);
                    }
                }
                CommandKind::Vote { .. }
                | CommandKind::Target { .. }
                | CommandKind::Mark { .. }
                | CommandKind::Reveal => {
                    if let Some(game_data) = &mut self.game_data {
                        game_data.handle(&cmd);
                    }
                }
                CommandKind::Status => {
                    if let Some(game_data) = &mut self.game_data {
                        todo!();
                    } else {
                        todo!();
                    }
                }
            };
        }
    }
}
