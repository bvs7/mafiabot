// use std::sync::Arc;

// use serde::{Deserialize, Serialize};

// /*
// Sync lobby has...
// Its state
// - List of games
// - The Lobby Chat
// - Optional Start message and time

// */
// struct Lobby {
//     lobby_chat: GroupMeGroup,
//     start_msg: Arc<Mutex<Option<(MessageId, DateTime<Local>, usize)>>>,
//     games: HashSet<Gid>,
//     try_start: Arc<mpsc::Sender<(Vec<UserId>, GroupId)>>,
//     admins: HashSet<UserId>,
// }

// impl Lobby {
//     fn new(lobby_chat: GroupMeGroup, try_start: mpsc::Sender<(Vec<UserId>, GroupId)>) -> Self {
//         Self {
//             lobby_chat,
//             start_msg: Arc::new(Mutex::new(None)),
//             games: HashSet::new(),
//             try_start: Arc::new(try_start),
//             admins: [BRIAN_UID.into()].into_iter().collect(),
//         }
//     }

//     fn handle_command(
//         &mut self,
//         uid: UserId,
//         cmd: LobbyCmd,
//         response: Response,
//         games: &HashMap<Gid, Game>,
//     ) {
//         match cmd {
//             LobbyCmd::Start(minutes, min_players) => {
//                 tokio::spawn(Self::send_start_message(
//                     self.start_msg.clone(),
//                     response,
//                     minutes,
//                     min_players,
//                 ));
//             }
//             LobbyCmd::Status(Some(gid)) => {
//                 if !self.games.contains(&gid) {
//                     let _ = tokio::spawn(async move {
//                         response
//                             .respond(&format!("Game {} not found for this lobby.", gid))
//                             .await
//                             .unwrap()
//                     });
//                     return;
//                 }
//                 match games.get(&gid) {
//                     Some(game) => {
//                         let state = game.state.lock().unwrap();
//                         let status = state.status(self.lobby_chat.names.clone());
//                         let msg = format!("Game {gid} {status}");
//                         let _ = tokio::spawn(async move { response.respond(&msg).await.unwrap() });
//                     }
//                     None => {
//                         let _ = tokio::spawn(async move {
//                             response.respond(&format!("Game {} not found.", gid)).await.unwrap()
//                         });
//                     }
//                 }
//             }
//             LobbyCmd::Status(None) => {
//                 // Send summary of games
//                 let mut msg = "".to_string();
//                 for gid in self.games.iter() {
//                     let Some(game) = games.get(gid) else {
//                         msg.push_str(&format!("Game {gid} not found.\n"));
//                         continue;
//                     };
//                     let state = game.state.lock().unwrap();
//                     let status = state.status(self.lobby_chat.names.clone());
//                     msg.push_str(&status.brief());
//                 }
//             }
//         }
//     }

//     fn update(&mut self) -> Option<DateTime<Local>> {
//         // Check status of start message
//         let mut start_msg = self.start_msg.lock().unwrap();
//         if let Some((msg_id, time, min_players)) = start_msg.take() {
//             if time < Local::now() {
//                 let _ = tokio::spawn(Self::try_start_game(
//                     msg_id,
//                     min_players,
//                     self.lobby_chat.group_id.clone(),
//                     self.try_start.clone(),
//                 ));
//             } else {
//                 *start_msg = Some((msg_id, time, min_players));
//                 return Some(time);
//             }
//         }
//         None
//     }

//     async fn send_start_message(
//         start_msg: Arc<Mutex<Option<(MessageId, DateTime<Local>, usize)>>>,
//         response: Response,
//         minutes: usize,
//         min_players: usize,
//     ) {
//         let msg_id = response
//             .respond(&format!(
//                 "Game starting in {minutes} minutes if there are at \
//                 least {min_players} players. Like this message to join."
//             ))
//             .await
//             .unwrap();
//         let time = Local::now() + Duration::from_secs(minutes as u64 * 60);
//         let mut start_msg = start_msg.lock().unwrap();
//         *start_msg = Some((msg_id, time, min_players));
//     }

//     async fn try_start_game(
//         msg_id: MessageId,
//         min_players: usize,
//         lobby_id: GroupId,
//         try_start: Arc<mpsc::Sender<(Vec<UserId>, GroupId)>>,
//     ) {
//         let users = api::get_group_message_likes(&lobby_id, &msg_id).await.unwrap();
//         if users.len() >= min_players {
//             let _ = try_start.send((users, lobby_id)).await;
//         } else {
//             api::send_group_message(&lobby_id, "Not enough players to start game.").await.unwrap();
//         }
//     }
// }
