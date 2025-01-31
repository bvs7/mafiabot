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
Thoughts about parsing. Parsing commands can all be read only.

How to structure this?

RwLock on the Hashmap of these things seems awkward. Unless we use the Appstate more as an address
book for sending messages. If anything, then, it might be best to make the RwLocks accessed by some
kind of getter. Then the main app controller can be the only thing that ever gets write access.

Messages:
- To the Controller:
    - Create Group
    - Destroy Group
    - Create Game
    - End Game
    - Create Lobby
    - End Lobby

- To the Game:
    - Action
    - Status?

- To the Lobby:
    - Start
    - Status

One problem with the RwLock structure is that we can run into deadlocks. We need all reads to be
non-blocking, including if they were to wait on something that requires a write lock.
Alternatively, the reads could all copy out immediately...

What kinds of read accesses do we need?
- Get group id of a lobby, main chat, or mafia chat...
- Get the focus Game Id of a player.
- Get the names of members of a group.
- Send an action to a game.
- Get the status of a game.
- Update the start_message of a lobby.

And then for write accesses...
- Just adding new groups or games or lobbies...

So first note: everything needs to be doable with just shared references.
Second, adding new groups or games needs to be careful about blocking when writing...

So, LobbyHandler

We could imagine, similar to GameHandler, everything LobbyHandler does can be done from a shared ref.

What does a lobby need to do?

- Send a start message.
    - Need to write the `start_msg` field of lobby. This could be a watch. Then the lobby task coul
    wake up when it is written
- Start a game
    - Need to collect users, send a start game request to the controller
    - Can this be done without holding the lobby read lock? Seems like no.
    - In that case, will there ever be a case where something holds a games read lock then wants to
    write lobbies? No. In fact, we can assume a hierarchy, Lobby, then game, then group, for the
    locks.
    - So, we need to start the game, but then the followup of "add the game to our list of games"
    can happen later, right? How would that work? Maybe there is another Hashmap??

I don't like all of these locks.
How could we do this with pure message passing?
Messages with responses could be awkward? Might cause deadlocks?

First off, routing. We need to be able to route messages to all of their various destinations.
- Controller needs to talk to everything
- Lobbies need to talk to their games (and their group)
- Games need to talk to their lobby and their group.
- Groups just handle input messages.

So we could imagine actors for each of these. And each actor could or could not be async.
Could Groups just be owned? When do we need to get names? Games need names, Lobbies need names to
start games. Yeah, let's just have groups be owned.

For now, let's outline all of the input messages we will need for each actor type

Controller:
- Create Game (resp: GameId)
- Destroy Game
- Create Lobby
- Destroy Lobby

Lobby:
- Create Start Message
- Status (resp: Status)
- Game Ended

Game:
- Action (resp: Result)
- Status (resp: Status)
- GetTarget (resp: UserId)


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
    let games = app_state.games.read().await;
    for game in games.values() {
        if game.main_chat_id == group_id {
            game.parse_main_chat_cmd(user_id, words, attachments, app_state).await;
            return;
        } else if game.mafia_id == group_id {
            let id = game.game_id;
            parse_mafia_chat_cmd(id, user_id, text, attachments, app_state).await;
            drop(r_app_status);
            return;
        }
    }
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

async fn parse_dm_cmd(user_id: UserId, text: String, app_status: &Arc<AppState>) -> bool {
    todo!()
}
