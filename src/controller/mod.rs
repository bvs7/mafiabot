// Controller.

/*
A unique controller will exist for each guild the bot is active in.

The controller is responsible for:
- Maintaining Game Cores
- Routing Core Actions to the appropriate Game Core
- Handling other App Actions
- Handling Game Core Events

Other App Actions:

- Create Lobby (either in new channel or in current channel)
- Close Lobby
- Create new game
  - Create game thread in lobby channel
  - Create Game message with current players
  - Listen for reacts to join game? Or just press button
- Start a game
  - Rolegen
  - Create game channel + threads
- Rules Import/Export/View/Setting

Each game has an associated game initializer, right?


Game Initializer:
- A thread in the lobby.
- Has a "join game"/"watch game" message and a status message

Each Controller is associated with a different Guild.
Then we have the top level Server.
The server can set up controllers when the bot is added to new guilds.

The server therefore knows how to tell if an event comes from a guild for a specific controller.
The controller can then do some processing itself.

The Controller has a central EventHandler, which handles discord events the server passes to it.

*/
#![allow(dead_code, unused_imports, unused_variables)]

use crate::core::base::{Choice, ID};
use crate::core::interface::{CommandTx, EventRx, Interface};

use std::collections::HashMap;

use serenity;
use serenity::all::{CommandDataOptionValue, User};
use serenity::async_trait;
use serenity::client::Context;
use serenity::http::Http;
use serenity::model::id::{ChannelId, GuildId, MessageId, UserId};
use serenity::model::{application, channel, gateway, id};
use std::sync::Arc;

use serenity::prelude::*;

impl ID for UserId {}

pub type GameId = u64;

pub struct GameInitializer {
    pub thread_id: ChannelId,
    pub join_msg_id: MessageId,
    pub status_msg_id: MessageId,
}

pub struct GameData {
    pub game_id: GameId,
    pub cmd_tx: CommandTx<UserId>,
    pub event_rx: EventRx<UserId>,
    pub initializer_thread_id: ChannelId,
    pub main_channel_id: ChannelId,
    pub mafia_thread_id: ChannelId,
    pub targeting_threads: HashMap<ChannelId, UserId>,
    pub reveal_threads: HashMap<ChannelId, UserId>,
}

pub struct Lobby {
    pub channel_id: ChannelId,
    pub game_initializer: Option<GameInitializer>,
}

pub struct Controller {
    pub guild_id: GuildId,
    pub lobbies: HashMap<ChannelId, Lobby>,
    pub games: HashMap<GameId, GameData>,
}

pub enum ButtonAction {
    JoinGame(ChannelId, UserId),
    WatchGame(ChannelId, UserId),
    StartGame(ChannelId, UserId),
    Reveal(GameId, UserId),
}

impl Controller {
    pub fn new(guild_id: GuildId, http: Arc<Http>) -> Self {
        Self {
            guild_id,
            lobbies: HashMap::new(),
            games: HashMap::new(),
        }
    }

    pub fn check_button(
        &self,
        button_id: String,
        msg_id: MessageId,
        channel_id: ChannelId,
        user_id: UserId,
    ) -> Option<ButtonAction> {
        // A button press can be one of:
        // - Join Game/watch game
        // - Start Game
        // - Reveal in a reveal thread
        // Check games
        for (game_id, game_data) in &self.games {
            for (thread_id, reveal_user_id) in &game_data.reveal_threads {
                if *thread_id == channel_id {
                    assert_eq!(
                        reveal_user_id, &user_id,
                        "User tried to reveal in a thread they don't own?!"
                    );
                    return Some(ButtonAction::Reveal(*game_id, user_id));
                }
            }
        }

        for (lobby_channel_id, lobby) in &self.lobbies {
            if let Some(game_initializer) = &lobby.game_initializer {
                if game_initializer.join_msg_id == msg_id {
                    match button_id.as_str() {
                        "join_game" => {
                            return Some(ButtonAction::JoinGame(lobby.channel_id, user_id))
                        }
                        "watch_game" => {
                            return Some(ButtonAction::WatchGame(lobby.channel_id, user_id))
                        }
                        "start_game" => {
                            return Some(ButtonAction::StartGame(lobby.channel_id, user_id))
                        }
                        _ => {}
                    }
                }
            }
        }

        None
    }

    pub fn check_menu(
        &self,
        menu_id: String,
        msg_id: MessageId,
        channel_id: ChannelId,
    ) -> Option<(GameId, UserId)> {
        // Check Games
        for (game_id, game_data) in &self.games {
            for (thread_id, targeting_user_id) in &game_data.targeting_threads {
                if *thread_id == channel_id {
                    return Some((*game_id, *targeting_user_id));
                }
            }
        }
        None
    }

    pub async fn create_mafia_command(
        guild_id: GuildId,
        cache_http: impl CacheHttp,
    ) -> Result<serenity::model::application::Command, serenity::Error> {
        use serenity::builder::{CreateCommand, CreateCommandOption};
        use serenity::model::application::{CommandOptionType, CommandType};

        let mafia_command = CreateCommand::new("mafia")
            .kind(CommandType::ChatInput)
            .description("Mafia Game Commands")
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommandGroup,
                    "lobby",
                    "Mafia Lobby Commands",
                )
                .add_sub_option(CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "create",
                    "Create a new channel as a lobby",
                ))
                .add_sub_option(CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "open",
                    "Open a lobby in this channel",
                ))
                .add_sub_option(CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "close",
                    "Close the lobby in this channel",
                )),
            )
            .add_option(
                CreateCommandOption::new(
                    CommandOptionType::SubCommandGroup,
                    "game",
                    "Mafia Game Commands",
                )
                .add_sub_option(CreateCommandOption::new(
                    CommandOptionType::SubCommand,
                    "create",
                    "Start the game in a lobby channel",
                )),
            );

        guild_id.create_command(cache_http, mafia_command).await
    }
}

// Events that take place in this controller's guild are routed here?
#[async_trait]
impl serenity::client::EventHandler for Controller {
    async fn ready(&self, ctx: Context, ready: gateway::Ready) {
        println!("{} is connected!", ready.user.name);
    }

    async fn interaction_create(&self, ctx: Context, interaction: application::Interaction) {
        use application::ComponentInteractionDataKind::{Button, UserSelect};
        use application::Interaction::{Command, Component, Ping};
        // Things this could be:
        match &interaction {
            // - A slash command
            Command(command) => {
                let data = &command.data;
                if data.name == "mafia" {
                    let option = &data.options.first().expect("No subcommandgroup?");
                    if option.name == "lobby" {
                    } else if option.name == "game" {
                    }

                    // TODO: figure out how subcommands are structured and destructure them

                    //     - lobby create/open/close
                    //     - game create
                } else if data.name == "vote" {
                    //     - nokill
                    //     - retract
                    //     - for
                }
            }
            // - A button press
            Component(component) if matches!(&component.data.kind, Button) => {
                // Get the channel from the button press
                let channel_id = component.channel_id;
                let msg_channel_id = component.message.channel_id;
                assert!(
                    channel_id == msg_channel_id,
                    "Select menu came from different channel??"
                );
                let msg_id = component.message.id;
                let button_id = component.data.custom_id.clone();
                let user_id = component
                    .member
                    .as_ref()
                    .expect("Button press should have member")
                    .user
                    .id
                    .clone();
                match self.check_button(button_id, msg_id, msg_channel_id, user_id) {
                    Some(ButtonAction::JoinGame(lobby_channel_id, user_id)) => {
                        // Join the game
                    }
                    Some(ButtonAction::WatchGame(lobby_channel_id, user_id)) => {
                        // Watch the game
                    }
                    Some(ButtonAction::StartGame(lobby_channel_id, user_id)) => {
                        // Start the game
                    }
                    Some(ButtonAction::Reveal(game_id, user_id)) => {
                        // Reveal in the game
                    }
                    None => {
                        todo!("Button press not recognized. Handle this error")
                    }
                }
                // TODO: Check if these channel ids are the same?
                // Check where this message came from
            }
            // - A select menu
            Component(component) if matches!(component.data.kind, UserSelect { .. }) => {
                let UserSelect { values } = component.data.kind.clone() else {
                    panic!("Select menu should have values!")
                };
                let channel_id = component.channel_id;
                let msg_channel_id = component.message.channel_id;
                assert!(
                    channel_id == msg_channel_id,
                    "Select menu came from different channel??"
                );
                let msg_id = component.message.id;
                let menu_id = component.data.custom_id.clone();
                let actor_id = component
                    .member
                    .as_ref()
                    .expect("Button press should have member")
                    .user
                    .id
                    .clone();
                let target_id = values
                    .first()
                    .expect("Select menu should have values")
                    .clone();
                match self.check_menu(menu_id, msg_id, msg_channel_id) {
                    Some((game_id, user_id)) => {
                        // Target the user
                    }
                    None => {
                        todo!("Select menu not recognized. Handle this error")
                    }
                }
            }
            Ping(_) => {
                // This is a ping
            }
            _ => {} // Autocomplete and Modal Interactions?
        }
    }
}

/*
Commands json?:
json = [
    {
        "name": "mafia",
        "type": 1, // CHAT_INPUT, Slash Command
        "description": "Mafiabot Controller Commands",
        "options": [
            {
                "name": "lobby",
                "description": "Create, Open, or Close a Lobby",
                "type": 2,  // SUB_COMMAND_GROUP
                "options" :[
                    {
                        "name": "create",
                        "description": "Create a new lobby channel, owned by Mafiabot",
                        "type": 1, // SUB_COMMAND
                    },
                    {
                        "name": "open",
                        "description": "Open a lobby in this channel",
                        "type": 1, // SUB_COMMAND
                    },
                    {
                        "name": "close",
                        "description": "Close this lobby channel",
                        "type": 1, // SUB_COMMAND
                    }
                ]
            }
        ]
    },
    {
        "name": "vote",
        "type": 1, // CHAT_INPUT, Slash Command
        "description": "In a game of mafia, a vote to elect a player for death!",
        "options": [
            {
                "name": "nokill",
                "type": 1, // SUB_COMMAND
                "description": "Vote for peace"
            },
            {
                "name": "retract",
                "type": 1, // SUB_COMMAND
                "description": "Retract your vote"
            },
            {
                "name": "for"
                "type": 1, // SUB_COMMAND
                "description": "Vote for a player"
                "options" : [
                    {
                        "name": "player",
                        "type": 6, // USER
                        "description": "Player to die"
                        "required": true
                    }
                ]
            },
        ]
    }
]


sample component interaction:
{
    "version": 1,
    "type": 3, // MESSAGE_COMPONENT
    "token": "token",
    "message": {
        "type": 0, // DEFAULT
        "tts": false,
        "timestamp": "2021-05-19T02:12:51.710000+00:00",
        "pinned": false,
        "mentions": [],
        "mention_roles": [],
        "mention_everyone": false,
        "id": "844397162624450620",
        "flags": 0,
        "embeds": [],
        "edited_timestamp": null,
        "content": "This is a message with components.",
        "components": [
            {
                "type": 1,
                "components": [
                    {
                        "type": 2,
                        "label": "Click me!",
                        "style": 1,
                        "custom_id": "click_one"
                    }
                ]
            }
        ],
        "channel_id": "345626669114982402",
        "author": {
            "username": "Mason",
            "public_flags": 131141,
            "id": "53908232506183680",
            "discriminator": "1337",
            "avatar": "a_d5efa99b3eeaa7dd43acca82f5692432"
        },
        "attachments": []
    },
    "member": {
        "user": {
            "username": "Mason",
            "public_flags": 131141,
            "id": "53908232506183680",
            "discriminator": "1337",
            "avatar": "a_d5efa99b3eeaa7dd43acca82f5692432"
        },
        "roles": [
            "290926798626357999"
        ],
        "premium_since": null,
        "permissions": "17179869183",
        "pending": false,
        "nick": null,
        "mute": false,
        "joined_at": "2017-03-13T19:19:14.040000+00:00",
        "is_pending": false,
        "deaf": false,
        "avatar": null
    },
    "id": "846462639134605312",
    "guild_id": "290926798626357999",
    "data": {
        "custom_id": "click_one",
        "component_type": 2 // BUTTON
    },
    "channel_id": "345626669114982999",
    "application_id": "290926444748734465"
}


*/
