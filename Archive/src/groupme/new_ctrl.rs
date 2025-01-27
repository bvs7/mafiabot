// Another attempt at making a controller

use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex},
    time::Duration,
};

use chrono::{format, DateTime, Local};
use parse::{Cmd, Command, GameCmd, LobbyCmd, Response};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use tokio::sync::{broadcast::error::SendError, mpsc};

use crate::engine::{
    interface::Event,
    state::{
        id::{Gid, Pid},
        role::Role,
        rules::Rules,
    },
    sync_state::{State, State_},
};

use super::{
    api::{self, GroupId},
    BRIAN_UID,
};

mod parse {
    use serde::Deserialize;
    use serde_json::Value as JsonValue;

    use crate::{
        engine::state::id::{Choice, Gid, RawBallot, RawChoice},
        groupme::{
            api::{self, GroupId},
            subscriber::PushMessage,
        },
    };

    use super::{Controller, Game, Lobby, MessageId, UserId};

    #[derive(Debug, Clone)]
    pub enum Cmd {
        Lobby(GroupId, UserId, LobbyCmd),
        Game(Gid, UserId, GameCmd),
    }

    #[derive(Debug, Clone)]
    pub enum LobbyCmd {
        Start(usize, usize),
        Status(Option<Gid>),
    }

    // TODO: don't let this be raw
    #[derive(Debug, Clone)]
    pub enum GameCmd {
        Vote(RawBallot),
        Reveal,
        Target(RawChoice),
        Scheme(RawChoice),
        Status,
    }

    #[derive(Debug, Clone)]
    pub struct Command {
        pub cmd: Cmd,
        pub response: Response,
    }

    #[derive(Debug, Clone)]
    pub enum Response {
        Group(MessageId, GroupId),
        DM(MessageId, UserId),
    }

    impl Response {
        pub fn msg_id(&self) -> &MessageId {
            match self {
                Response::Group(msg_id, _) => msg_id,
                Response::DM(msg_id, _) => msg_id,
            }
        }
        pub async fn like(&self) {
            match self {
                Response::Group(msg_id, group_id) => {
                    let _ = api::like_message(group_id, msg_id).await.unwrap();
                }
                Response::DM(_, user_id) => {
                    // Skip for now
                }
            }
        }
        pub async fn respond(&self, text: &str) -> anyhow::Result<MessageId> {
            match self {
                Response::Group(_, group_id) => api::send_group_message(group_id, text).await,
                Response::DM(_, user_id) => api::send_dm(*user_id, text).await,
            }
        }
    }

    impl Controller {
        fn handle_input(&mut self, msg: PushMessage) {
            let Some(data) = msg.data() else {
                return;
            };
            let Some(command) = self.parse_input(data.clone()) else {
                return;
            };

            self.handle_command(command);
        }

        fn parse_input(&self, data: JsonValue) -> Option<Command> {
            let interaction: Interaction = match serde_json::from_value(data) {
                Ok(interaction) => interaction,
                Err(err) => {
                    tracing::warn!("Error parsing interaction: {err}");
                    return None;
                }
            };
            match interaction.type_ {
                InteractionType::GroupMsg => {
                    let cmd = self.parse_group_cmd(&interaction.subject)?;
                    Some(Command {
                        cmd,
                        response: Response::Group(
                            interaction.subject.id,
                            interaction.subject.group_id,
                        ),
                    })
                }
                InteractionType::DirectMsg => {
                    let cmd = self.parse_dm_cmd(&interaction.subject)?;
                    Some(Command {
                        cmd,
                        response: Response::DM(interaction.subject.id, interaction.subject.user_id),
                    })
                }
                InteractionType::Unknown(s) => {
                    tracing::warn!("Unknown interaction type: {s}");
                    None
                }
            }
        }

        fn parse_group_cmd(&self, s: &Subject) -> Option<Cmd> {
            // Parse the command
            let user_id = s.user_id;
            let group_id = &s.group_id;
            let text = &s.text;
            let c1 = text.chars().next()?;
            if c1 != '/' {
                return None;
            }
            let words: Vec<_> = text.split_whitespace().collect();
            let mut cmd = None;

            if let Some(lobby) = self.lobbies.get(group_id) {
                let c = lobby.parse_lobby_cmd(&mut words.clone());
                if let Some(c) = c {
                    cmd = Some(Cmd::Lobby(group_id.to_string(), user_id, c));
                }
            }

            for (gid, game) in self.games.iter() {
                if &game.main_chat.group_id == group_id {
                    if let Some(c) = game.parse_main_chat_cmd(&s, user_id, &words) {
                        cmd = Some(Cmd::Game(*gid, user_id, c));
                        break;
                    }
                } else if &game.mafia_chat.group_id == group_id {
                    if let Some(c) = game.parse_mafia_chat_cmd(&words) {
                        cmd = Some(Cmd::Game(*gid, user_id, c));
                        break;
                    }
                }
            }

            // TODO: parse app command?

            cmd
        }

        fn parse_dm_cmd(&self, s: &Subject) -> Option<Cmd> {
            let user_id = s.user_id;
            let group_id = &s.group_id;
            let text = &s.text;
            let c1 = text.chars().next()?;
            if c1 != '/' {
                return None;
            }
            let words: Vec<_> = text.split_whitespace().collect();
            let mut cmd = None;

            if let Some(gid) = self.player_focus.get(&user_id) {
                let game = self.games.get(gid)?;
                if let Some(c) = game.parse_dm_game_cmd(&words) {
                    cmd = Some(Cmd::Game(*gid, user_id, c));
                }
            }
            // TODO: parse app command
            cmd
        }
    }
    impl Lobby {
        fn parse_lobby_cmd(&self, words: &Vec<&str>) -> Option<LobbyCmd> {
            let mut words = words.iter();
            let first = *words.next()?;
            match first {
                "start" => {
                    let minutes = words.next()?.parse().ok()?;
                    let min_players = words.next()?.parse().ok()?;
                    Some(LobbyCmd::Start(minutes, min_players))
                }
                "status" => {
                    let gid = words
                        .next()
                        .map(|gid_str| gid_str.parse::<u64>().ok().map(Gid::from))
                        .flatten()
                        .map(|gid| self.games.contains(&gid).then_some(gid))
                        .flatten();
                    Some(LobbyCmd::Status(gid))
                }
                _ => None,
            }
        }
    }

    impl Game {
        fn parse_target(&self, target_idx: &str) -> Option<RawChoice> {
            let idx = target_idx.chars().next()?.to_ascii_uppercase();
            if !idx.is_ascii_alphabetic() {
                return None;
            }
            let ascii_idx = (idx as u8) - b'A';
            let rstate = self.state.lock().unwrap();
            let choice = rstate.get_target(ascii_idx as usize).ok()?.map(u64::from);
            drop(rstate);

            Some(choice)
        }
        fn parse_main_chat_cmd(
            &self,
            s: &Subject,
            uid: UserId,
            words: &Vec<&str>,
        ) -> Option<GameCmd> {
            let mut words = words.iter();
            let first = *words.next()?;
            match first {
                "vote" => {
                    let next = words.next();
                    if let Some(&"nokill") = next {
                        return Some(GameCmd::Vote(Some(None)));
                    } else if let Some(&"me") = next {
                        return Some(GameCmd::Vote(Some(Some(uid.0))));
                    } else {
                        for attachment in s.attachments.iter() {
                            if let Attachment::Mentions { user_ids } = attachment {
                                if user_ids.len() >= 1 {
                                    let uid = user_ids[0];
                                    return Some(GameCmd::Vote(Some(Some(uid.0))));
                                }
                            }
                        }
                    }
                    None
                }
                "unvote" => Some(GameCmd::Vote(None)),
                "status" => Some(GameCmd::Status),
                _ => None,
            }
        }

        fn parse_mafia_chat_cmd(&self, words: &Vec<&str>) -> Option<GameCmd> {
            let mut words = words.iter();
            let first = *words.next()?;
            match first {
                "target" => {
                    let target_idx = *words.next()?;
                    let choice = self.parse_target(target_idx)?;
                    Some(GameCmd::Scheme(choice))
                }
                _ => None,
            }
        }

        fn parse_dm_game_cmd(&self, words: &Vec<&str>) -> Option<GameCmd> {
            let mut words = words.iter();
            let first = *words.next()?;
            match first {
                "reveal" => Some(GameCmd::Reveal),
                "target" => {
                    let target_idx = *words.next()?;
                    let choice = self.parse_target(target_idx)?;
                    Some(GameCmd::Target(choice))
                }
                _ => None,
            }
        }
    }
    #[cfg(test)]
    mod tests {
        use std::collections::HashMap;

        use serde_json::json;
        use tracing_subscriber::registry;
        use tracing_test::traced_test;

        use crate::{
            engine::{
                state::{role::Role, rules::Rules},
                sync_state::State_,
            },
            groupme::{BRIAN_UID, LOBBY_CHAT_ID, TEST_LOBBY_CHAT_ID},
        };

        use super::*;

        // fn state_3() -> State_ {
        //     let registry = vec![(1, Role::TOWN), (2, Role::TOWN), (3, Role::MAFIA)];
        //     State_::new(registry, Rules::default())
        // }

        // #[test]
        // #[traced_test]
        // fn parsing_cmds() {
        //     let mut lobby = Lobby::new(TEST_LOBBY_CHAT_ID.to_string());
        //     let game = GameHolder {
        //         game_id: Gid::from(1),
        //         game: state_3(),
        //         main_chat_id: "main".to_string(),
        //         mafia_chat_id: "mafia".to_string(),
        //         player_list: vec![1, 2, 3],
        //         names: HashMap::new(),
        //     };

        //     lobby.games.insert(Gid::from(1), game);
        //     lobby.player_focus.insert(BRIAN_UID, Gid::from(1));

        //     let msg1 = json!({
        //         "subject":  {
        //             "attachments": [],
        //             "created_at": 1737048634,
        //             "group_id": TEST_LOBBY_CHAT_ID,
        //             "id": "173704863488783901",
        //             "name": "Brian \"Testing\" Scaramella",
        //             "sender_id": "21642197",
        //             "text": "/start 5 3",
        //             "user_id": "21642197",
        //         },
        //         "type": "line.create",
        //     });
        //     let cmd = lobby.parse_cmd(msg1);
        //     assert!(matches!(
        //         cmd,
        //         Some(Command {
        //             cmd: Cmd::Lobby(_, LobbyCmd::Start(5, 3)),
        //             ..
        //         })
        //     ));

        //     let msg2 = json!({
        //         "subject":  {
        //             "attachments": [],
        //             "created_at": 1737048634,
        //             "group_id": TEST_LOBBY_CHAT_ID,
        //             "id": "173704863488783901",
        //             "name": "Brian \"Testing\" Scaramella",
        //             "sender_id": "21642197",
        //             "text": "/status",
        //             "user_id": "21642197",
        //         },
        //         "type": "line.create",
        //     });
        //     let cmd = lobby.parse_cmd(msg2);
        //     assert!(matches!(
        //         cmd,
        //         Some(Command {
        //             cmd: Cmd::Lobby(_, LobbyCmd::Status(None)),
        //             ..
        //         })
        //     ));

        //     let msg3 = json!({
        //         "subject":  {
        //             "attachments": [],
        //             "created_at": 1737048634,
        //             "group_id": "main",
        //             "id": "173704863488783901",
        //             "name": "Brian \"Testing\" Scaramella",
        //             "sender_id": "21642197",
        //             "text": "/status",
        //             "user_id": "21642197",
        //         },
        //         "type": "line.create",
        //     });
        //     let cmd = lobby.parse_cmd(msg3);
        //     assert!(matches!(
        //         cmd,
        //         Some(Command {
        //             cmd: Cmd::Game(_, _, GameCmd::Status),
        //             ..
        //         })
        //     ));

        //     let msg4 = json!({
        //         "subject":  {
        //             "attachments": [],
        //             "created_at": 1737048634,
        //             "group_id": "main",
        //             "id": "173704863488783901",
        //             "name": "Brian \"Testing\" Scaramella",
        //             "sender_id": "21642197",
        //             "text": "/vote me",
        //             "user_id": "21642197",
        //         },
        //         "type": "line.create",
        //     });
        //     let cmd = lobby.parse_cmd(msg4);
        //     assert!(matches!(
        //         cmd,
        //         Some(Command {
        //             cmd: Cmd::Game(_, _, GameCmd::Vote(_)),
        //             ..
        //         })
        //     ));

        //     let msg5 = json!({
        //         "subject":  {
        //             "attachments": [],
        //             "created_at": 1737048634,
        //             "group_id": "mafia",
        //             "id": "173704863488783901",
        //             "name": "Brian \"Testing\" Scaramella",
        //             "sender_id": "21642197",
        //             "text": "/target B",
        //             "user_id": "21642197",
        //         },
        //         "type": "line.create",
        //     });
        //     let cmd = lobby.parse_cmd(msg5);
        //     assert!(matches!(
        //         cmd,
        //         Some(Command {
        //             cmd: Cmd::Game(_, _, GameCmd::Scheme(_)),
        //             ..
        //         })
        //     ));

        //     let msg6 = json!({
        //         "subject":  {
        //             "attachments": [],
        //             "created_at": 1737048634,
        //             "group_id": "mafia",
        //             "id": "173704863488783901",
        //             "name": "Brian \"Testing\" Scaramella",
        //             "sender_id": "21642197",
        //             "text": "/target B",
        //             "user_id": "21642197",
        //         },
        //         "type": "direct_message.create",
        //     });
        //     let cmd = lobby.parse_cmd(msg6);
        //     assert!(matches!(
        //         cmd,
        //         Some(Command {
        //             cmd: Cmd::Game(_, _, GameCmd::Target(_)),
        //             ..
        //         })
        //     ));
        // }
    }
}

/*
Should the lobby be separated from its games?
A lobby should know about its games, as in, know how to reference its games...
... but maybe not necessarily.
Say we had an app that has both lobbies and games. The lobby starting a game
would register that game with the app. In fact the app should tell the lobby
what the gid is.
How would that relationship work?

The app is split into...
- Input server. A handler that listens for messages and serves them accoringly
- A controller, which is a single task that calls updates, creates unique gids, etc.
- Event handlers, which are tasks for each game that listen for events and respond.


Controller. When we create one of these...
- It starts its input server, which is the push server.
- The push server gets messages, and routes them to the correct lobbies and games
    - Using the group id or chat id, it can route to the correct lobby or game
- Then, the controller also has update tasks.
    - The controller itself has a task that gets start requests from lobbies
        - And also monitors games for completion, and deletes them after some amount of time
    - Each lobby has a task which listens for a start timer to finish
    - Each game has a task that calls the update fn when necessary
- Additionally, we have event listeners for the games.


The update functions for each system should be called each time a mutating command was passed in.
They should return a time for when they need to be called again.



*/
// TODO: put this in api? maybe
struct GroupMeGroup {
    group_id: GroupId,
    names: HashMap<UserId, String>,
}

impl GroupMeGroup {
    async fn new(name: &str) -> Self {
        let group_id = api::create_group(name).await.unwrap();
        Self { group_id, names: HashMap::new() }
    }
}

/*
What if we want to get rid of async?
We could have a task for each loop. For games and for lobbies (and for controller)
Then those could just pass messages to each other.



*/

struct Controller {
    lobbies: HashMap<GroupId, Lobby>,
    games: HashMap<Gid, Game>,
    player_focus: HashMap<UserId, Gid>,
}

impl Controller {
    async fn handle_command(&mut self, cmd: Command) {
        match cmd {
            Command { cmd: Cmd::Game(gid, uid, game_cmd), response } => {
                if let Some(game) = self.games.get_mut(&gid) {
                    game.handle_command(uid, game_cmd, response);
                }
            }
            Command { cmd: Cmd::Lobby(lobby_id, uid, lobby_cmd), response } => {
                if let Some(lobby) = self.lobbies.get_mut(&lobby_id) {
                    lobby.handle_command(uid, lobby_cmd, response, &self.games);
                }
            }
        }
    }

    async fn handle_try_start(&mut self, players: Vec<UserId>, group_id: GroupId) {
        let lobby = self.lobbies.get_mut(&group_id).unwrap();
        let gid = Gid::new();
        let roles: Vec<Role> = todo!("Rolegen");
        let registry: Vec<(UserId, Role)> = players.into_iter().zip(roles).collect();
        let state = State::new(registry, Rules::default());
        let game = Game::new(gid, state).await;
        // TODO: create event listener?
        self.games.insert(gid, game);
        lobby.games.insert(gid);
        // TODO: Add players to games
        // TODO: Update names of main chat
    }
}

// Note: for now, Mutex is std::sync::Mutex, which means the Guard can't be held
// over await boundaries... so we need to be careful about that.
// Let's have a
type SendEvent = fn(Event) -> Result<usize, SendError<Event>>;
struct Game {
    game_id: Gid,
    main_chat: GroupMeGroup,
    mafia_chat: GroupMeGroup,
    state: Arc<Mutex<State>>,
    send_event: SendEvent,
}

fn send(event: Event) -> Result<usize, SendError<Event>> {
    // Send the event
    todo!()
}

impl Game {
    async fn new(game_id: Gid, state: State) -> Self {
        let main_chat = GroupMeGroup::new(&format!("Main Chat #{game_id}")).await;
        let mafia_chat = GroupMeGroup::new(&format!("Mafia Chat #{game_id}")).await;

        Self {
            game_id,
            main_chat,
            mafia_chat,
            state: Arc::new(Mutex::new(state)),
            send_event: send,
        }
    }
    fn handle_command(&mut self, uid: UserId, cmd: GameCmd, response: Response) {
        let raw_uid = u64::from(uid);
        let mut rstate = self.state.lock().unwrap();
        let result = match cmd {
            GameCmd::Vote(vote) => rstate.vote(raw_uid, vote),
            GameCmd::Reveal => rstate.reveal(raw_uid),
            GameCmd::Target(target) => rstate.target(raw_uid, target),
            GameCmd::Scheme(scheme) => rstate.scheme(raw_uid, scheme),
            GameCmd::Status => {
                let status = rstate.status(self.main_chat.names.clone());
                let gid = self.game_id;
                let msg = format!("Game {gid} {status}");
                let r = response.clone();
                tokio::spawn(async move { r.respond(&msg).await.unwrap() });
                Ok(())
            }
        };
        drop(rstate);
        match result {
            Ok(()) => {
                let _ = tokio::spawn(async move { response.like().await });
            }
            Err(err) => {
                let _ =
                    tokio::spawn(async move { response.respond(&format!("Error: {err}")).await });
            }
        };
    }
}
