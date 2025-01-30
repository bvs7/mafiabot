// use broadcast::error::RecvError;
// use rand::thread_rng;

use crate::prelude::*;

mod app_state;
pub use app_state::AppState;
use tokio::task::JoinHandle;
mod parse;

struct App {
    app_state: Arc<AppState>,
    message_rx: broadcast::Receiver<Data>,
    subscriber_handle: JoinHandle<()>,
}

impl App {
    pub async fn new() -> Self {
        let app_state = Arc::new(AppState::new());
        let (subscriber_handle, message_rx) = PushWebSocketServer::create();
        Self { app_state, subscriber_handle, message_rx }
    }
}

// enum AppRequest {
//     CreateGame { users: Vec<UserId>, rules: Rules, resp: oneshot::Sender<Result<GameId, Error>> },
//     CreateGroup { members: Vec<(UserId, String)>, resp: oneshot::Sender<Result<GroupId, Error>> },
// }

// #[derive(Debug)]
// pub struct LobbyInfo {
//     pub lobby_id: GroupId,
//     pub chat: GroupMeGroup,
//     pub rules: Rules,
// }

// #[derive(Debug, Default)]
// pub struct AppStatus {
//     pub games: HashMap<GameId, GameInfo>,
//     pub groups: HashMap<GroupId, GroupMeGroup>,
//     pub lobbies: HashMap<GroupId, LobbyInfo>,
//     pub focuses: HashMap<UserId, GameId>,
// }

// impl AppStatus {
//     pub fn new() -> Self {
//         Self::default()
//     }
// }
