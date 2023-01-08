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
    night_action_channels: Vec<ChannelID>,
    rx_events: Receiver<Event<UserID>>,
}
// TODO: Implement handling !vote, !target, !mark, !reveal commands here

impl GameData {
    fn handle(&mut self, cmd: &Command) -> Result<(), ()> {
        todo!();
    }
}

pub struct Lobby {
    channel: ChannelID,
    init_msg: Option<MessageID>,
}
// TODO: Implement handling !init and !start commands here
// !start command returns a GameData struct, which contains the Game, and the channels for the game.

impl Lobby {
    fn handle(&mut self, cmd: &Command) -> Result<(), ()> {
        todo!();
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
                CommandKind::Init | CommandKind::Start => self.lobby.handle(&cmd),
                CommandKind::Vote { .. }
                | CommandKind::Target { .. }
                | CommandKind::Mark { .. }
                | CommandKind::Reveal => {
                    if let Some(game_data) = &mut self.game_data {
                        game_data.handle(&cmd)
                    } else {
                        Err(())
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

            if let Err(_err) = result {
                todo!();
            }
        }
    }
}
