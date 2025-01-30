use broadcast::error::RecvError;
use groupme::subscriber::{Attachment, Data, PushWebSocketServer};

use crate::prelude::*;

#[derive(Debug, Clone)]
pub enum Cmd {
    Lobby(GroupId, UserId, LobbyCmd),
    Game(GameId, UserId, GameCmd),
}

#[derive(Debug, Clone)]
pub enum LobbyCmd {
    Start(usize, usize),
    Status(Option<GameId>),
}

// TODO: don't let this be raw
#[derive(Debug, Clone)]
pub enum GameCmd {
    Vote(Option<Option<UserId>>),
    Reveal,
    Target(Option<UserId>),
    Scheme(Option<UserId>),
    Status,
}

#[derive(Debug, Clone)]
pub enum Response {
    Group(MessageId, GroupId),
    DM(MessageId, UserId),
}

/*
Command parsing...
If possible, handle parsing in game handler and in lobby handler.
So test if a message applies to various games or lobbies, then forward them.
Have a return type of parsers that represents if the command was handled or not.
If not, then check for a total app command.

*/

async fn message_handler(mut message_rx: broadcast::Receiver<Data>, app_state: Arc<AppState>) {
    loop {
        match message_rx.recv().await {
            Ok(data) => {
                parse_cmd(data, &app_state);
            }
            Err(RecvError::Lagged(n)) => {
                warn!("Lagged: {n}");
                continue;
            }
            Err(RecvError::Closed) => {
                break;
            }
        }
    }
}

enum ParseResult {
    Handled,
    NotHandled,
}

async fn parse_cmd(cmd: Data, app_state: &Arc<AppState>) {
    let text = cmd.text();
    let mut chars = text.chars();
    let Some('\\') = chars.next() else {
        return;
    };
    let text: String = chars.collect();
    // TODO: make this longer if needed
    let words = text.split_whitespace().take(5).map(|s| s.to_owned()).collect::<Vec<String>>();
    match cmd {
        Data::GroupMsg { attachments, group_id, id, name, user_id, .. } => {
            // TODO: Update name of user in app_status group?
            parse_group_cmd(group_id, user_id, words, attachments, app_state).await;
        }
        Data::DirectMsg { attachments, id, name, text, user_id, .. } => {
            parse_dm_cmd(user_id, text, app_state).await;
        }
        Data::Unknown => {}
    }
}

async fn parse_group_cmd(
    group_id: GroupId,
    user_id: UserId,
    words: Vec<String>,
    attachments: Vec<Attachment>,
    app_state: &Arc<AppState>,
) {
    let r_lobbies = app_state.lobbies.read().await;
    if let Some(lobby) = r_lobbies.get(&group_id) {
        lobby.parse_cmd(user_id, words, attachments, app_state).await;
        return;
    }
    // Not a lobby, check if it's a game

    // for game in r_app_status.games.values() {
    //     if game.main_id == group_id {
    //         let id = game.game_id;
    //         drop(r_app_status);
    //         parse_main_chat_cmd(id, user_id, text, attachments, app_state).await;
    //         return;
    //     } else if game.mafia_id == group_id {
    //         let id = game.game_id;
    //         parse_mafia_chat_cmd(id, user_id, text, attachments, app_state).await;
    //         drop(r_app_status);
    //         return;
    //     }
    // }
}

// async fn parse_main_chat_cmd(
//     game_id: GameId,
//     user_id: UserId,
//     text: String,
//     attachments: Vec<Attachment>,
//     app_status: &Arc<AppStatus>,
// ) -> Option<(Cmd, Response)> {
//     let words = text.split_whitespace().collect::<Vec<&str>>();
//     if words.len() == 0 {
//         return None;
//     }
//     let cmd = words[0];
//     if cmd == "/admin_start" {}
//     return None;
// }

// async fn parse_mafia_chat_cmd(
//     game_id: GameId,
//     user_id: UserId,
//     text: String,
//     attachments: Vec<Attachment>,
//     app_status: Arc<RwLock<AppStatus>>,
// ) -> Option<(Cmd, Response)> {
//     todo!()
// }

// async fn parse_dm_cmd(
//     user_id: UserId,
//     text: String,
//     app_status: Arc<RwLock<AppStatus>>,
// ) -> Option<(Cmd, Response)> {
//     todo!()
// }
