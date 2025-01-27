// /*
// How to structure commands?

// */
// use std::collections::HashMap;

// use tokio::sync::{
//     broadcast::{self, error::RecvError}, watch, Mutex
// };

// use crate::engine::{interface::{ActionTx, Event, EventRx}, state::{role::Role, PlayerId}, Game};

// use super::api::{self, GroupId, UserId};

// #[derive(Debug, Clone)]
// enum GameCmd {
//     Status,
//     Help,
//     Rules,
// }

// #[derive(Debug, Clone)]
// enum LobbyCmd {
//     Start { minutes: u32, min_players: usize },
//     Status,
//     Help,
// }

// #[derive(Debug, Clone)]
// enum DMCmd {
//     Status,
//     Help,
// }

// #[derive(Debug, Clone)]
// enum Source {
//     Group(u64),
//     DM,
// }

// #[derive(Debug, Clone)]
// enum AppCmd {
//     Other(String),
// }

// #[derive(Debug, Clone)]
// enum Cmd {
//     GameAction {
//         game_id: u64,
//         action: Action,
//     },
//     GameCmd {
//         game_id: u64,
//         cmd: GameCmd,
//     },
//     DMCmd {
//         user: u64,
//         cmd: DMCmd,
//     },
//     LobbyCmd {
//         lobby: u64,
//         cmd: LobbyCmd,
//     },
//     AppCmd {
//         user: u64,
//         source: Source,
//         cmd: AppCmd,
//     },
// }

// // We have a list of games with their mafia and main chats, right?

// #[derive(Debug, Clone)]
// enum GameChatKind {
//     Main,
//     Mafia,
// }

// #[derive(Debug)]
// struct GameContext {
//     game: Game,
//     main_chat: GroupId,
//     mafia_chat: GroupId,
//     names: watch::Receiver<HashMap<PlayerId, String>>,
//     living: Vec<PlayerId>,
// }

// #[derive(Debug)]
// struct Controller {
//     game: Mutex<Option<GameHolder>>,
//     names: watch::Receiver<HashMap<u64, String>
//     games: HashMap<u64, GameHolder>,               // game_id -> game
//     game_chats: HashMap<u64, (GameChatKind, u64)>, // chat_id -> (kind, game_id)
//     targeter_games: HashMap<u64, u64>,             // user_id -> game_id
// }

// impl Controller {
//     fn send_role_assignment(
//         self: &Arc<Self>,
//         client: &Client,
//         user_id: UserId,
//         role: Role,
//     ) -> Result<String> {
//         let mut extra = "";
//         if let Role::GUARD(charge) | Role::AGENT(charge) = &role {
//             let name = self.get_name(charge)?;
//             extra = &format!("Your charge is {name}.");
//         }
//         let kind = role.kind();
//         let team = role.team();
//         let msg = format!("Your role is {kind}. You are {team} Aligned. {extra}\n(use /help roles or /help teams for help)");
//         Ok(msg.to_owned())
//     }

//     // Event handler needs to know nicknames!

//     #[tracing::instrument(skip_all)]
//     async fn event_handler(self: Arc<Self>, rx: broadcast::Receiver<Event>) {
//         let client = Client::new();
//         loop {
//             match rx.recv().await {
//                 Err(RecvError::Lagged(n)) => tracing::warn!("Lagged {n}"),
//                 Err(RecvError::Closed) => break,
//                 Ok(event) => {
//                     match Event {
//                         Event::Start {
//                             id,
//                             players,
//                             rules,
//                             counts,
//                         } => {
//                             // Send out individual roles
//                             for (pid, role) in &players {
//                                 let msg = self.send_role_assignment(&client, pid, role)?;
//                                 api::send_dm(&client, pid, &msg)?;
//                             }
//                             let a = self.game;
//                             api::send_group_message(&client, "", text);
//                         }
//                         Event::Vote {
//                             voter,
//                             ballot,
//                             former,
//                         } => {}
//                         _ => unimplemented!(),
//                     }
//                 }
//             }
//         }
//     }
// }

// impl Controller {
//     fn msg_data_to_command(&self, msg_data: MsgData) -> Option<Cmd> {
//         let mut text = msg_data.subject.text.clone();
//         let first_char = text.chars().nth(0)?;
//         if first_char != '/' {
//             return None; // No command specifier
//         }
//         let mut words = text.split(' ');
//         let first = words.next()?;

//         let user: u64 = msg_data.subject.user_id.parse().ok()?;

//         if &msg_data.type_ == "direct_message.create" {
//             if first == "/help" {
//                 return Some(Cmd::DMCmd {
//                     user,
//                     cmd: DMCmd::Help,
//                 });
//             } else if first == "/status" {
//                 return Some(Cmd::DMCmd {
//                     user,
//                     cmd: DMCmd::Status,
//                 });
//             } else if first == "/target" {
//                 let actor = user;
//                 let mut choice: Option<usize> = None;
//                 if let Some(next) = words.next() {
//                     choice = next.parse().ok();
//                 }
//                 let game_id = *self.targeter_games.get(&actor)?;
//                 let choice = if let Some(choice_n) = choice {
//                     let living = &self.games.get(&game_id)?.living;
//                     let choice: u64 = *living.get(choice_n)?;
//                     Some(choice)
//                 } else {
//                     None
//                 };

//                 return Some(Cmd::GameAction {
//                     game_id,
//                     action: Action::Target { actor, choice },
//                 });
//             } else {
//                 return Some(Cmd::AppCmd {
//                     user,
//                     source: Source::DM,
//                     cmd: AppCmd::Other(first.to_owned()),
//                 });
//             }
//         }

//         if &msg_data.type_ == "line.create" {
//             let group_id = &msg_data.subject.group_id?;
//             if group_id == LOBBY_CHAT_ID {
//                 let lobby = LOBBY_CHAT_ID.parse().ok()?;
//                 if first == "/status" {
//                     return Some(Cmd::LobbyCmd {
//                         lobby,
//                         cmd: LobbyCmd::Status,
//                     });
//                 } else if first == "/help" {
//                     return Some(Cmd::LobbyCmd {
//                         lobby,
//                         cmd: LobbyCmd::Help,
//                     });
//                 } else if first == "/start" {
//                     let mut minutes: u32 = 10;
//                     let mut min_players: usize = 7;
//                     if let Some(m) = words.next() {
//                         if let Ok(m) = m.parse() {
//                             minutes = m;
//                         }
//                     }
//                     if let Some(m) = words.next() {
//                         if let Ok(m) = m.parse() {
//                             min_players = m;
//                         }
//                     }
//                     return Some(Cmd::LobbyCmd {
//                         lobby,
//                         cmd: LobbyCmd::Start {
//                             minutes,
//                             min_players,
//                         },
//                     });
//                 }
//             }

//             let g_id: u64 = group_id.parse().ok()?;
//             // Look for an associated game
//             if let Some((kind, game)) = self.game_chats.get(&g_id) {
//                 let game_id = game.clone();
//                 if matches!(kind, GameChatKind::Main) {
//                     // Check for a game action
//                     if first == "/vote" {
//                         let voter = user;
//                         // Check for a mention next
//                         let mut ballot = None;
//                         if let Some(mut mention_user_ids) = msg_data
//                             .subject
//                             .attachments
//                             .iter()
//                             .filter_map(|a| match a {
//                                 Attachment::Mentions { user_ids: u, .. } => Some(u),
//                                 _ => None,
//                             })
//                             .cloned()
//                             .nth(0)
//                         {
//                             let choice_str = mention_user_ids.pop();
//                             ballot = choice_str.map(|s| s.parse::<u64>().ok());
//                         }
//                         if ballot.is_none() {
//                             if let Some("nokill") = words.next() {
//                                 ballot = Some(None);
//                             }
//                         }
//                         return Some(Cmd::GameAction {
//                             game_id,
//                             action: Action::Vote { voter, ballot },
//                         });
//                     } else if first == "/status" {
//                         return Some(Cmd::GameCmd {
//                             game_id,
//                             cmd: GameCmd::Status,
//                         });
//                     } else if first == "/help" {
//                         return Some(Cmd::GameCmd {
//                             game_id,
//                             cmd: GameCmd::Help,
//                         });
//                     } else if first == "/rules" {
//                         return Some(Cmd::GameCmd {
//                             game_id,
//                             cmd: GameCmd::Rules,
//                         });
//                     }
//                 } else if matches!(kind, GameChatKind::Mafia) {
//                     if first == "/target" {
//                         let killer = user;
//                         let mut choice = None;
//                         if let Some(next) = words.next() {
//                             choice = next.parse().ok();
//                         }
//                         return Some(Cmd::GameAction {
//                             game_id,
//                             action: Action::Scheme {
//                                 killer,
//                                 mark: choice,
//                             },
//                         });
//                     }
//                 }
//             }
//             return Some(Cmd::AppCmd {
//                 user,
//                 source: Source::Group(g_id),
//                 cmd: AppCmd::Other(first.to_owned()),
//             });
//         }
//         None
//     }
// }
