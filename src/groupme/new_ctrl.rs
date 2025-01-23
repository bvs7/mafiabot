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

// What commands do we have?
// Lobby commands
// Game commands
// DM commands

/*
When we parse a command, what do we want?
We will have a response item. From a reference to that,
we will parse a command.

What is in a command:
A destination? Like the lobby controller, or a game controller?
For example:
- Start in lobby goes to lobby ctrl
- Status in lobby goes to lobby ctrl
- Status in a game goes to a game ctrl
- Vote in game goes to a game ctrl
- Reveal in DM goes to a game ctrl
- target in DM goes to a game ctrl
- Help can go to lobby, game, or full app

So we could say we have:
- Command
- and Response context

Although commands to different destinations have different forms, right?
Except maybe help... But help can be a special case.
So let's fold Destination into Command.

We also need a response context.
- Messge Id
- Group or Chat Id it came from


What would be some examples of what we want?
- Start in lobby: create the start message
- admin_start in lobby: start a game
- Status in lobby: show statuses of any games
- Help in lobby: show help message based on following text

- Vote in game: call vote on game. Based on the result, respond to the message...
- Reveal in DM: call reveal on game

The response holds...
- What kind of message it is
- The sender
- The group or chat
- The text of the message


Example of an input...
"data": Object {
    "alert": String("Brian \"Testing\" Scaramella: Test"),
    "received_at": Number(1737048635000),
    "subject": Object {
        "attachments": Array [],
        "avatar_url": String("https://i.groupme.com/200x205.jpeg.3475af00b96f4d1d8a21a9822ddd5e3a"),
        "created_at": Number(1737048634),
        "deleted_at": Null,
        "deletion_actor": Null,
        "group_id": String("105362524"),
        "id": String("173704863488783901"),
        "location": Object {
            "lat": String(""),
            "lng": String(""),
            "name": Null,
        },
        "name": String("Brian \"Testing\" Scaramella"),
        "parent_id": Null,
        "picture_url": Null,
        "pinned_at": Null,
        "pinned_by": Null,
        "sender_id": String("21642197"),
        "sender_type": String("user"),
        "source_guid": String("android-bbfe3445-d65e-4352-a613-19df30dede79"),
        "system": Bool(false),
        "text": String("Test"),
        "updated_at": Null,
        "user_id": String("21642197"),
    },
    "type": String("line.create"),
},

*/
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

    #[derive(Debug, Clone, Deserialize)]
    struct Subject {
        attachments: Vec<Attachment>,
        #[serde(alias = "chat_id")]
        group_id: GroupId,
        created_at: u64,
        id: String,
        name: String,
        sender_id: UserId,
        text: String,
        user_id: UserId,
    }

    #[derive(Debug, Clone, Deserialize)]
    struct Interaction {
        #[serde(rename = "type")]
        type_: InteractionType,
        subject: Subject,
    }

    enum Cmd {
        Lobby(UserId, LobbyCmd),
        Game(Gid, UserId, GameCmd),
    }

    enum LobbyCmd {
        Start(usize, usize),
        Status(Option<Gid>),
    }

    enum GameCmd {
        Vote(RawBallot),
        Reveal,
        Target(RawChoice),
        Scheme(RawChoice),
        Status,
    }

    struct Command {
        cmd: Cmd,
        response: Response,
    }

    enum Response {
        Group(MessageId, GroupId),
        DM(MessageId, UserId),
    }

    impl Lobby {
        fn parse_cmd(&self, response: JsonValue) -> Option<Command> {
            let inter: Interaction = serde_json::from_value(response).ok()?;
            use InteractionType::*;

            // Parse the command
            let user_id = inter.subject.user_id;
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
                                .parse_main_chat_cmd(&inter, &mut words)
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
                let gid = self.player_focus.get(&inter.subject.sender_id);
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
                DirectMsg => Response::DM(inter.subject.id, inter.subject.sender_id),
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
                    let gid = words.next()?.parse::<u64>().ok().map(Gid::from);
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
            words: &mut dyn Iterator<Item = &str>,
        ) -> Option<GameCmd> {
            let first = words.next()?;
            match first {
                "vote" => {
                    let next = words.next();
                    if let Some("nokill") = next {
                        return Some(GameCmd::Vote(Some(None)));
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
