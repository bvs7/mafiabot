use std::collections::HashSet;
use std::sync::mpsc::Receiver;

mod commands;
mod eventhandler;

pub use self::commands::{GameCommand, LobbyCommand};

use super::core::*;
use super::discord::*;
pub use commands::Command;

fn create_game(users: &HashSet<UserID>) -> (Game, Receiver<Event>) {
    let roles = get_roles(users.len(), 0.4, &full_roleset());
    let users = users.into_iter().map(|u| *u).collect();
    let (players, contracts) = get_players(users, roles);
    let (tx, rx) = std::sync::mpsc::channel();
    let game = Game::new(0, players, contracts, Comm { tx });
    (game, rx)
}

pub struct Controller {
    rx: Receiver<Command>,
    game_state: GameState,
    lobby: LobbyController,
}

#[derive(Debug, Clone, Copy)]
pub struct GameChannels {
    pub main: ChannelID,
    pub mafia: ChannelID,
}

#[derive(Debug)]
pub struct LobbyController {
    category: ChannelID,
    channel: ChannelID,
    guild: GuildID,
}

#[derive(Debug)]
pub struct GameController {
    game: Game,
    channels: GameChannels,
    event_queue: Receiver<Event>,
}

#[derive(Debug)]
pub enum GameState {
    Game(GameController),
    Init {
        channels: GameChannels,
        users: HashSet<UserID>,
    },
    None,
}

impl LobbyController {
    fn handle(
        &mut self,
        cmd: LobbyCommand,
        game_state: &mut GameState,
    ) -> Result<(), DiscordError> {
        match cmd {
            LobbyCommand::Init => match game_state {
                GameState::None => {
                    let channels = create_game_channels(self.category)?;
                    *game_state = GameState::Init {
                        channels,
                        users: HashSet::new(),
                    };
                }
                _ => todo!("Game can't be initialized"),
            },

            LobbyCommand::Join(user) => match game_state {
                GameState::Init { channels, users } => {
                    users.insert(user);
                    add_users_to_channel(channels.main, vec![user])?;
                }
                GameState::Game(game_controller) => {
                    todo!("Add user to game.");
                }
                _ => todo!("No game to join"),
            },
            LobbyCommand::Leave(user) => match game_state {
                GameState::Init { channels, users } => {
                    users.remove(&user);
                    remove_users_from_channel(channels.main, vec![user])?;
                }
                GameState::Game(game_controller) => {
                    todo!("Remove user from game if either they are dead or it has ended.");
                }
                _ => todo!("No game to leave?"),
            },
            LobbyCommand::Start => match game_state {
                GameState::Init { channels, users } => {
                    let (game, event_queue) = create_game(users);
                    *game_state = GameState::Game(GameController {
                        game,
                        channels: channels.clone(),
                        event_queue,
                    });
                }
                _ => todo!("Game can't be started"),
            },
            LobbyCommand::Close => match game_state {
                GameState::Init { channels, users } => {
                    delete_game_channels(channels.clone())?;
                    *game_state = GameState::None;
                }
                GameState::Game(game_controller) => {
                    todo!("Close game if it is ended");
                }
                _ => todo!("No game to close"),
            },
        }
        Ok(())
    }
}

impl GameController {
    fn handle(&mut self, act: Action) -> Result<(), ()> {
        let result = self.game.handle(act);

        if let Err(_err) = result {
            todo!();
        }

        // Check event_queue
        loop {
            let event = match self.event_queue.try_recv() {
                Ok(event) => event,
                Err(err) => break,
            };
            todo!("Handle event: {:?}", event);
        }
        Ok(())
    }
}

impl Controller {
    fn new(guild: GuildID, rx: Receiver<Command>) -> Self {
        let (category, channel) = get_lobby_channels(guild).expect("TODO");
        let lobby = LobbyController {
            category,
            channel,
            guild,
        };
        // TODO: Recover a game from save>
        let game_state = GameState::None;
        Self {
            rx,
            game_state,
            lobby,
        }
    }

    fn start(self) -> std::thread::JoinHandle<()> {
        std::thread::spawn(move || self.controller_thread())
    }

    // TODO: Use tokio recv?

    fn controller_thread(mut self) {
        loop {
            let cmd = match self.rx.recv() {
                Ok(cmd) => cmd,
                Err(err) => continue,
            };

            // Handle command.
            let result = match cmd {
                Command::Lobby(cmd) => {
                    self.lobby.handle(cmd, &mut self.game_state);
                    if let GameState::None = self.game_state {
                        create_game_channels(self.lobby.category).map(|channels| {
                            self.game_state = GameState::Init {
                                channels,
                                users: HashSet::new(),
                            };
                        })
                    } else {
                        todo!("Game can't be initialized");
                    }
                }
                Command::Game(act) => match &mut self.game_state {
                    GameState::Game(game_controller) => game_controller.handle(act),
                    _ => todo!("No game to handle action"),
                },
            };

            if let Err(err) = result {
                todo!("Handle error: {:?}", err);
            }
        }
    }
}
