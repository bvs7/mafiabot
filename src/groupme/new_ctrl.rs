// Another attempt at making a controller

use std::{
    collections::{HashMap, HashSet},
    time::Duration,
};

use chrono::Local;
use reqwest::Client;

use crate::engine::{state::id::Gid, sync_state::State};

use super::{
    api::{self, GroupId},
    BRIAN_UID,
};

type MessageId = String;
type UserId = u64;

mod parse {
    use serde::Deserialize;
    use serde_json::Value as JsonValue;

    use crate::{
        engine::state::id::{Choice, Gid, RawBallot, RawChoice},
        groupme::api::GroupId,
    };

    use super::{GameHolder, Lobby, MessageId, UserId};

    #[derive(Debug, Clone, Deserialize)]
    #[serde(from = "String")]
    enum InteractionType {
        GroupMsg,
        DirectMsg,
        Unknown(String),
    }
    impl From<String> for InteractionType {
        fn from(s: String) -> Self {
            match s.as_str() {
                "line.create" => InteractionType::GroupMsg,
                "direct_message.create" => InteractionType::DirectMsg,
                _ => InteractionType::Unknown(s),
            }
        }
    }

    #[derive(Debug, Clone, Deserialize)]
    #[serde(tag = "type")]
    enum Attachment {
        #[serde(rename = "mentions")]
        Mentions { user_ids: Vec<UserId> },
        #[serde(other)]
        Unknown,
    }

    // TODO: make UserId a newtypes

    #[derive(Debug, Clone, Deserialize)]
    struct Subject {
        attachments: Vec<Attachment>,
        #[serde(alias = "chat_id")]
        group_id: GroupId,
        created_at: u64,
        id: String,
        name: String,
        sender_id: String,
        text: String,
        user_id: String,
    }

    #[derive(Debug, Clone, Deserialize)]
    struct Interaction {
        #[serde(rename = "type")]
        type_: InteractionType,
        subject: Subject,
    }

    #[derive(Debug, Clone)]
    enum Cmd {
        Lobby(UserId, LobbyCmd),
        Game(Gid, UserId, GameCmd),
    }

    #[derive(Debug, Clone)]
    enum LobbyCmd {
        Start(usize, usize),
        Status(Option<Gid>),
    }

    #[derive(Debug, Clone)]
    enum GameCmd {
        Vote(RawBallot),
        Reveal,
        Target(RawChoice),
        Scheme(RawChoice),
        Status,
    }

    #[derive(Debug, Clone)]
    struct Command {
        cmd: Cmd,
        response: Response,
    }

    #[derive(Debug, Clone)]
    enum Response {
        Group(MessageId, GroupId),
        DM(MessageId, UserId),
    }

    impl Lobby {
        fn parse_cmd(&self, response: JsonValue) -> Option<Command> {
            let inter: Interaction = serde_json::from_value(response).ok()?;
            use InteractionType::*;

            // Parse the command
            let user_id: u64 = inter.subject.user_id.parse().ok()?;
            let mut chars = inter.subject.text.chars();
            let c1 = chars.next()?;
            if c1 != '/' {
                return None;
            }
            let text: String = chars.collect();
            let mut words = text.split_whitespace();
            let mut cmd = None;
            if matches!(inter.type_, GroupMsg) {
                let group_id = &inter.subject.group_id;
                if group_id == &self.lobby_chat_id {
                    cmd = self
                        .parse_lobby_cmd(&inter, &mut words)
                        .map(|cmd| Cmd::Lobby(user_id, cmd));
                } else {
                    for (gid, game) in self.games.iter() {
                        if group_id == &game.main_chat_id {
                            cmd = game
                                .parse_main_chat_cmd(&inter, user_id, &mut words)
                                .map(|cmd| Cmd::Game(*gid, user_id, cmd));
                            break;
                        } else if group_id == &game.mafia_chat_id {
                            cmd = game
                                .parse_mafia_chat_cmd(&inter, &mut words)
                                .map(|cmd| Cmd::Game(*gid, user_id, cmd));
                            break;
                        }
                    }
                }
            }

            if cmd.is_none() && matches!(inter.type_, DirectMsg) {
                let gid = self.player_focus.get(&user_id);
                if let Some(gid) = gid {
                    if let Some(game) = self.games.get(gid) {
                        cmd = game
                            .parse_dm_game_cmd(&inter, *gid, &mut words)
                            .map(|cmd| Cmd::Game(*gid, user_id, cmd));
                    }
                }
            }
            let response = match inter.type_ {
                GroupMsg => Response::Group(inter.subject.id, inter.subject.group_id),
                DirectMsg => Response::DM(inter.subject.id, user_id),
                Unknown(_) => return None,
            };
            cmd.map(|cmd| Command { cmd, response })
        }

        fn parse_lobby_cmd(
            &self,
            i: &Interaction,
            words: &mut dyn Iterator<Item = &str>,
        ) -> Option<LobbyCmd> {
            let first = words.next()?;
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
                        .map(|gid| self.games.contains_key(&gid).then_some(gid))
                        .flatten();
                    Some(LobbyCmd::Status(gid))
                }
                _ => None,
            }
        }
    }

    impl GameHolder {
        fn parse_target(&self, target_idx: &str) -> Option<RawChoice> {
            let idx = target_idx.chars().next()?.to_ascii_uppercase();
            if !idx.is_ascii_alphabetic() {
                return None;
            }
            let ascii_idx = (idx as u8) - b'A';
            if ascii_idx == self.player_list.len() as u8 {
                return Some(None);
            }
            let pid = self.player_list.get(ascii_idx as usize)?;

            Some(Some(*pid))
        }
        fn parse_main_chat_cmd(
            &self,
            inter: &Interaction,
            user_id: UserId,
            words: &mut dyn Iterator<Item = &str>,
        ) -> Option<GameCmd> {
            let first = words.next()?;
            match first {
                "vote" => {
                    let next = words.next();
                    if let Some("nokill") = next {
                        return Some(GameCmd::Vote(Some(None)));
                    } else if let Some("me") = next {
                        return Some(GameCmd::Vote(Some(Some(user_id))));
                    } else {
                        for attachment in inter.subject.attachments.iter() {
                            if let Attachment::Mentions { user_ids } = attachment {
                                if user_ids.len() >= 1 {
                                    let pid = user_ids[0];
                                    return Some(GameCmd::Vote(Some(Some(pid))));
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

        fn parse_mafia_chat_cmd(
            &self,
            inter: &Interaction,
            words: &mut dyn Iterator<Item = &str>,
        ) -> Option<GameCmd> {
            let first = words.next()?;
            match first {
                "target" => {
                    let target_idx = words.next()?;
                    let choice = self.parse_target(target_idx)?;
                    Some(GameCmd::Scheme(choice))
                }
                _ => None,
            }
        }

        fn parse_dm_game_cmd(
            &self,
            i: &Interaction,
            gid: Gid,
            words: &mut dyn Iterator<Item = &str>,
        ) -> Option<GameCmd> {
            let first = words.next()?;
            match first {
                "reveal" => Some(GameCmd::Reveal),
                "target" => {
                    let target_idx = words.next()?;
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
                sync_state::State,
            },
            groupme::{BRIAN_UID, LOBBY_CHAT_ID, TEST_LOBBY_CHAT_ID},
        };

        use super::*;

        fn state_3() -> State {
            let registry = vec![(1, Role::TOWN), (2, Role::TOWN), (3, Role::MAFIA)];
            State::new(registry, Rules::default())
        }

        #[test]
        #[traced_test]
        fn parsing_cmds() {
            let mut lobby = Lobby::new(TEST_LOBBY_CHAT_ID.to_string());
            let game = GameHolder {
                game_id: Gid::from(1),
                game: state_3(),
                main_chat_id: "main".to_string(),
                mafia_chat_id: "mafia".to_string(),
                player_list: vec![1, 2, 3],
                names: HashMap::new(),
            };

            lobby.games.insert(Gid::from(1), game);
            lobby.player_focus.insert(BRIAN_UID, Gid::from(1));

            let msg1 = json!({
                "subject":  {
                    "attachments": [],
                    "created_at": 1737048634,
                    "group_id": TEST_LOBBY_CHAT_ID,
                    "id": "173704863488783901",
                    "name": "Brian \"Testing\" Scaramella",
                    "sender_id": "21642197",
                    "text": "/start 5 3",
                    "user_id": "21642197",
                },
                "type": "line.create",
            });
            let cmd = lobby.parse_cmd(msg1);
            assert!(matches!(
                cmd,
                Some(Command {
                    cmd: Cmd::Lobby(_, LobbyCmd::Start(5, 3)),
                    ..
                })
            ));

            let msg2 = json!({
                "subject":  {
                    "attachments": [],
                    "created_at": 1737048634,
                    "group_id": TEST_LOBBY_CHAT_ID,
                    "id": "173704863488783901",
                    "name": "Brian \"Testing\" Scaramella",
                    "sender_id": "21642197",
                    "text": "/status",
                    "user_id": "21642197",
                },
                "type": "line.create",
            });
            let cmd = lobby.parse_cmd(msg2);
            assert!(matches!(
                cmd,
                Some(Command {
                    cmd: Cmd::Lobby(_, LobbyCmd::Status(None)),
                    ..
                })
            ));

            let msg3 = json!({
                "subject":  {
                    "attachments": [],
                    "created_at": 1737048634,
                    "group_id": "main",
                    "id": "173704863488783901",
                    "name": "Brian \"Testing\" Scaramella",
                    "sender_id": "21642197",
                    "text": "/status",
                    "user_id": "21642197",
                },
                "type": "line.create",
            });
            let cmd = lobby.parse_cmd(msg3);
            assert!(matches!(
                cmd,
                Some(Command {
                    cmd: Cmd::Game(_, _, GameCmd::Status),
                    ..
                })
            ));

            let msg4 = json!({
                "subject":  {
                    "attachments": [],
                    "created_at": 1737048634,
                    "group_id": "main",
                    "id": "173704863488783901",
                    "name": "Brian \"Testing\" Scaramella",
                    "sender_id": "21642197",
                    "text": "/vote me",
                    "user_id": "21642197",
                },
                "type": "line.create",
            });
            let cmd = lobby.parse_cmd(msg4);
            assert!(matches!(
                cmd,
                Some(Command {
                    cmd: Cmd::Game(_, _, GameCmd::Vote(_)),
                    ..
                })
            ));

            let msg5 = json!({
                "subject":  {
                    "attachments": [],
                    "created_at": 1737048634,
                    "group_id": "mafia",
                    "id": "173704863488783901",
                    "name": "Brian \"Testing\" Scaramella",
                    "sender_id": "21642197",
                    "text": "/target B",
                    "user_id": "21642197",
                },
                "type": "line.create",
            });
            let cmd = lobby.parse_cmd(msg5);
            assert!(matches!(
                cmd,
                Some(Command {
                    cmd: Cmd::Game(_, _, GameCmd::Scheme(_)),
                    ..
                })
            ));

            let msg6 = json!({
                "subject":  {
                    "attachments": [],
                    "created_at": 1737048634,
                    "group_id": "mafia",
                    "id": "173704863488783901",
                    "name": "Brian \"Testing\" Scaramella",
                    "sender_id": "21642197",
                    "text": "/target B",
                    "user_id": "21642197",
                },
                "type": "direct_message.create",
            });
            let cmd = lobby.parse_cmd(msg6);
            assert!(matches!(
                cmd,
                Some(Command {
                    cmd: Cmd::Game(_, _, GameCmd::Target(_)),
                    ..
                })
            ));
        }
    }
}

enum LobbyCmd {
    StartGame(usize, usize),
    Status(Option<Gid>),
    Help(String),
}

enum GameCmd {
    Vote(Option<u64>),
    Reveal,
    Target(Option<u64>),
    Scheme(Option<u64>),
    Status,
    Help(String),
}

struct Lobby {
    lobby_chat_id: String,
    games: HashMap<Gid, GameHolder>,
    player_focus: HashMap<UserId, Gid>,
    names: HashMap<UserId, String>,
    admins: HashSet<UserId>,
}

struct GameHolder {
    game_id: Gid,
    game: State,
    main_chat_id: String,
    mafia_chat_id: String,
    player_list: Vec<UserId>,
    names: HashMap<UserId, String>,
}

impl Lobby {
    fn new(lobby_chat_id: String) -> Self {
        Self {
            lobby_chat_id,
            games: HashMap::new(),
            player_focus: HashMap::new(),
            names: HashMap::new(),
            admins: [BRIAN_UID].into_iter().collect(),
            // start_msg: None,
        }
    }

    async fn start_msg(self, minutes: usize, min_players: usize) {
        let msg = format!(
            "Starting a new game in {} minutes, if {} or more players join.",
            minutes, min_players
        );
        let client = Client::new();
        let msg_id = api::send_group_message(&client, &self.lobby_chat_id, &msg)
            .await
            .unwrap();
        let time = Local::now() + Duration::from_secs(60 * minutes as u64);
        tokio::spawn(async move {
            tokio::time::sleep(Duration::from_secs(60 * minutes as u64));
            // self.try_start_game(msg_id, min_players).await;
        });
        // self.start_msg = Some((msg_id, time));
        // TODO: start a timer to recognize when this finishes
    }

    async fn try_start_game(&mut self, msg_id: MessageId, min_players: usize) {
        let client = Client::new();
        let users = api::get_group_message_likes(&client, &self.lobby_chat_id, &msg_id)
            .await
            .unwrap();
        if users.len() >= min_players {
            // self.start_game(users).await;
        } else {
            let msg = "Not enough players to start a game";
            api::send_group_message(&client, &self.lobby_chat_id, msg)
                .await
                .unwrap();
        }
    }
}
